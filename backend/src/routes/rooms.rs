use axum::{
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use rand::{distributions::Alphanumeric, Rng};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast;

use crate::{
    db::Db,
    rooms::{Player, Room, RoomStatus, RoomStore},
};

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub game_id: i64,
    pub nickname: String,
}

#[derive(Serialize)]
pub struct CreateRoomResponse {
    pub code: String,
    pub game_id: i64,
    pub nickname: String,
}

#[derive(Deserialize)]
pub struct JoinQuery {
    pub nickname: String,
}

pub async fn create_room(
    State(db): State<Db>,
    State(rooms): State<RoomStore>,
    Json(payload): Json<CreateRoomRequest>,
) -> Result<Json<CreateRoomResponse>, (StatusCode, String)> {
    let nickname = payload.nickname.trim().to_string();

    if nickname.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Nickname é obrigatório".into()));
    }

    let (min_players, max_players) = get_game_player_limits(&db, payload.game_id)?;

    let code = generate_room_code();

    let (tx, _) = broadcast::channel(100);

    let room = Room {
        code: code.clone(),
        game_id: payload.game_id,
        status: RoomStatus::Waiting,
        min_players,
        max_players,
        host_player_id: 0,
        next_player_id: 1,
        players: vec![],
        tx,
    };

    {
        let mut rooms = rooms.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Falha ao acessar salas".to_string(),
            )
        })?;

        rooms.insert(code.clone(), room);
    }

    Ok(Json(CreateRoomResponse {
        code,
        game_id: payload.game_id,
        nickname,
    }))
}

pub async fn get_room(
    State(rooms): State<RoomStore>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rooms = rooms.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar salas".to_string(),
        )
    })?;

    let room = rooms
        .get(&code)
        .ok_or((StatusCode::NOT_FOUND, "Sala não encontrada".into()))?;

    Ok(Json(room_snapshot(room)))
}

pub async fn room_ws(
    State(rooms): State<RoomStore>,
    Path(code): Path<String>,
    Query(query): Query<JoinQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_room_socket(socket, rooms, code, query.nickname))
}

async fn handle_room_socket(
    socket: WebSocket,
    rooms: RoomStore,
    code: String,
    nickname: String,
) {
    let nickname = nickname.trim().to_string();

    if nickname.is_empty() {
        return;
    }

    let (player_id, mut rx) = {
        let mut rooms_guard = match rooms.lock() {
            Ok(rooms) => rooms,
            Err(_) => return,
        };

        let Some(room) = rooms_guard.get_mut(&code) else {
            return;
        };

        if room.players.len() >= room.max_players as usize {
            return;
        }

        let player_id = room.next_player_id;
        room.next_player_id += 1;

        let is_host = room.players.is_empty();

        let player = Player {
            id: player_id,
            nickname,
            is_host,
        };

        if is_host {
            room.host_player_id = player_id;
        }

        room.players.push(player);

        let snapshot = room_snapshot(room);
        let _ = room.tx.send(snapshot.to_string());

        (player_id, room.tx.subscribe())
    };

    let (mut sender, mut receiver) = socket.split();

    let initial_snapshot = {
        let rooms_guard = match rooms.lock() {
            Ok(rooms) => rooms,
            Err(_) => return,
        };

        let Some(room) = rooms_guard.get(&code) else {
            return;
        };

        room_snapshot(room).to_string()
    };

    if sender.send(Message::Text(initial_snapshot.into())).await.is_err() {
        remove_player(&rooms, &code, player_id);
        return;
    }

    loop {
        tokio::select! {
            incoming = receiver.next() => {
                let Some(Ok(message)) = incoming else {
                    break;
                };

                match message {
                    Message::Text(text) => {
                        handle_client_message(&rooms, &code, player_id, text.to_string());
                    }
                    Message::Close(_) => {
                        break;
                    }
                    _ => {}
                }
            }

            broadcast_message = rx.recv() => {
                let Ok(text) = broadcast_message else {
                    break;
                };

                if sender.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    remove_player(&rooms, &code, player_id);
}

fn handle_client_message(
    rooms: &RoomStore,
    code: &str,
    player_id: u64,
    text: String,
) {
    let mut rooms_guard = match rooms.lock() {
        Ok(rooms) => rooms,
        Err(_) => return,
    };

    let Some(room) = rooms_guard.get_mut(code) else {
        return;
    };

    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => {
            let _ = room.tx.send(json!({
                "type": "error",
                "message": "Mensagem inválida"
            }).to_string());
            return;
        }
    };

    let message_type = parsed
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    match message_type {
        "start" => {
            if player_id != room.host_player_id {
                let _ = room.tx.send(json!({
                    "type": "error",
                    "message": "Apenas o host pode iniciar a partida"
                }).to_string());
                return;
            }

            if room.players.len() < room.min_players as usize {
                let _ = room.tx.send(json!({
                    "type": "error",
                    "message": "Jogadores insuficientes para iniciar"
                }).to_string());
                return;
            }

            room.status = RoomStatus::Running;

            let _ = room.tx.send(json!({
                "type": "room_started",
                "room": room_snapshot(room)
            }).to_string());
        }

        "keydown" | "keyup" | "paddown" | "padup" => {
            let _ = room.tx.send(json!({
                "type": "input",
                "player_id": player_id,
                "event": parsed
            }).to_string());
        }

        _ => {
            let _ = room.tx.send(json!({
                "type": "echo",
                "player_id": player_id,
                "message": parsed
            }).to_string());
        }
    }
}

fn remove_player(rooms: &RoomStore, code: &str, player_id: u64) {
    let mut rooms_guard = match rooms.lock() {
        Ok(rooms) => rooms,
        Err(_) => return,
    };

    let Some(room) = rooms_guard.get_mut(code) else {
        return;
    };

    room.players.retain(|player| player.id != player_id);

    let _ = room.tx.send(json!({
        "type": "player_left",
        "room": room_snapshot(room)
    }).to_string());

    if room.players.is_empty() {
        rooms_guard.remove(code);
    }
}

fn room_snapshot(room: &Room) -> serde_json::Value {
    json!({
        "type": "room_state",
        "code": room.code,
        "game_id": room.game_id,
        "status": room.status,
        "min_players": room.min_players,
        "max_players": room.max_players,
        "players": room.players,
        "can_start": room.players.len() >= room.min_players as usize
    })
}

fn generate_room_code() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect::<String>()
        .to_uppercase()
}

fn get_game_player_limits(
    db: &Db,
    game_id: i64,
) -> Result<(i64, i64), (StatusCode, String)> {
    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar banco de dados".to_string(),
        )
    })?;

    let result = conn
        .query_row(
            "
            SELECT min_players, max_players
            FROM games
            WHERE id = ?1
            ",
            [game_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    result.ok_or((StatusCode::NOT_FOUND, "Jogo não encontrado".into()))
}
