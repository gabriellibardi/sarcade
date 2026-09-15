use std::{
    collections::HashMap,
    sync::MutexGuard,
    time::{Duration, Instant},
};

use rand::{distributions::Alphanumeric, Rng};
use tokio::sync::broadcast;

use backend::game_runtime::{
    RuntimeCommand, RuntimeHandle, RuntimeInputEvent, RuntimeOutput, RuntimePlayer,
};

use crate::rooms::{
    model::{
        InputEvent, Player, Room, RoomError, RoomSnapshot, RoomStatus, ServerMessage,
    },
    store::RoomStore,
};

const EMPTY_ROOM_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_NICKNAME_LENGTH: usize = 30;

pub struct CreatedRoom {
    pub code: String,
    pub host_token: String,
}

pub struct JoinRoom {
    pub player_id: u64,
    pub is_host: bool,
    pub room: RoomSnapshot,
    pub rx: broadcast::Receiver<ServerMessage>,
}

pub fn create_room(
    store: &RoomStore,
    game_id: i64,
    min_players: i64,
    max_players: i64,
) -> Result<CreatedRoom, RoomError> {
    let mut rooms = lock_rooms(store)?;

    cleanup_empty_expired_rooms(&mut rooms);

    let code = generate_unique_room_code(&rooms);
    let host_token = random_token(32);

    let (tx, _) = broadcast::channel(100);
    let (runtime, runtime_outputs) = RuntimeHandle::reserve(game_id);
    forward_runtime_outputs(runtime_outputs, tx.clone());

    let room = Room {
        code: code.clone(),
        game_id,
        status: RoomStatus::Waiting,
        min_players,
        max_players,
        host_token: host_token.clone(),
        host_player_id: None,
        next_player_id: 1,
        players: Vec::new(),
        tx,
        runtime,
        created_at: Instant::now(),
    };

    rooms.insert(code.clone(), room);

    Ok(CreatedRoom { code, host_token })
}

pub fn get_room_snapshot(
    store: &RoomStore,
    code: &str,
) -> Result<RoomSnapshot, RoomError> {
    let mut rooms = lock_rooms(store)?;

    cleanup_empty_expired_rooms(&mut rooms);

    let room = rooms.get(code).ok_or(RoomError::NotFound)?;

    Ok(room_snapshot(room))
}

pub fn join_room(
    store: &RoomStore,
    code: &str,
    nickname: String,
    host_token: Option<String>,
) -> Result<JoinRoom, RoomError> {
    let nickname = validate_nickname(&nickname)?;

    let mut rooms = lock_rooms(store)?;

    cleanup_empty_expired_rooms(&mut rooms);

    let room = rooms.get_mut(code).ok_or(RoomError::NotFound)?;

    if room.status == RoomStatus::Running {
        return Err(RoomError::AlreadyRunning);
    }

    if room.players.len() >= room.max_players as usize {
        return Err(RoomError::Full);
    }

    if room
        .players
        .iter()
        .any(|player| player.nickname.eq_ignore_ascii_case(&nickname))
    {
        return Err(RoomError::NicknameTaken);
    }

    let is_host = host_token
        .as_deref()
        .is_some_and(|token| token == room.host_token);

    let player_id = room.next_player_id;
    room.next_player_id += 1;

    if is_host {
        for player in &mut room.players {
            player.is_host = false;
        }

        room.host_player_id = Some(player_id);
    }

    let player = Player {
        id: player_id,
        nickname,
        is_host,
    };

    room.players.push(player);

    let rx = room.tx.subscribe();
    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::RoomState {
        room: snapshot.clone(),
    });

    Ok(JoinRoom {
        player_id,
        is_host,
        room: snapshot,
        rx,
    })
}

pub fn validate_nickname(nickname: &str) -> Result<String, RoomError> {
    let nickname = nickname.trim();

    if nickname.is_empty() {
        return Err(RoomError::NicknameRequired);
    }

    if nickname.chars().count() > MAX_NICKNAME_LENGTH {
        return Err(RoomError::NicknameTooLong);
    }

    Ok(nickname.to_string())
}

pub fn leave_room(
    store: &RoomStore,
    code: &str,
    player_id: u64,
) -> Result<(), RoomError> {
    let mut rooms = lock_rooms(store)?;

    let Some(room) = rooms.get_mut(code) else {
        return Ok(());
    };

    let before = room.players.len();

    room.players.retain(|player| player.id != player_id);

    let removed = before != room.players.len();

    if !removed {
        return Ok(());
    }

    if room.host_player_id == Some(player_id) {
        room.host_player_id = None;

        for player in &mut room.players {
            player.is_host = false;
        }
    }

    if room.players.is_empty() {
        let _ = room.runtime.send(RuntimeCommand::Stop);
        rooms.remove(code);
        return Ok(());
    }

    if room.status == RoomStatus::Running {
        let _ = room.runtime.send(RuntimeCommand::PlayerLeft { player_id });
    }

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::PlayerLeft {
        player_id,
        room: snapshot,
    });

    Ok(())
}

pub fn start_room(
    store: &RoomStore,
    code: &str,
    player_id: u64,
) -> Result<(), RoomError> {
    let mut rooms = lock_rooms(store)?;
    let room = rooms.get_mut(code).ok_or(RoomError::NotFound)?;

    ensure_host(room, player_id)?;

    if room.status == RoomStatus::Running {
        return Err(RoomError::AlreadyRunning);
    }

    if room.players.len() < room.min_players as usize {
        return Err(RoomError::InsufficientPlayers);
    }

    room.runtime
        .send(RuntimeCommand::Start {
            game_id: room.game_id,
            players: room
                .players
                .iter()
                .map(|player| RuntimePlayer {
                    id: player.id,
                    nickname: player.nickname.clone(),
                })
                .collect(),
        })
        .map_err(|_| RoomError::RuntimeUnavailable)?;

    room.status = RoomStatus::Running;

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::RoomStarted {
        room: snapshot,
    });

    Ok(())
}

pub fn kick_player(
    store: &RoomStore,
    code: &str,
    requester_id: u64,
    target_id: u64,
) -> Result<(), RoomError> {
    let mut rooms = lock_rooms(store)?;
    let room = rooms.get_mut(code).ok_or(RoomError::NotFound)?;

    ensure_host(room, requester_id)?;

    if requester_id == target_id {
        return Err(RoomError::CannotKickSelf);
    }

    if !room.players.iter().any(|player| player.id == target_id) {
        return Err(RoomError::PlayerNotFound);
    }

    room.players.retain(|player| player.id != target_id);

    if room.status == RoomStatus::Running {
        let _ = room.runtime.send(RuntimeCommand::PlayerLeft {
            player_id: target_id,
        });
    }

    if room.host_player_id == Some(target_id) {
        room.host_player_id = None;
    }

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::PlayerKicked {
        player_id: target_id,
        room: snapshot.clone(),
    });

    let _ = room.tx.send(ServerMessage::RoomState {
        room: snapshot,
    });

    Ok(())
}

pub fn promote_host(
    store: &RoomStore,
    code: &str,
    requester_id: u64,
    target_id: u64,
) -> Result<(), RoomError> {
    let mut rooms = lock_rooms(store)?;
    let room = rooms.get_mut(code).ok_or(RoomError::NotFound)?;

    ensure_host(room, requester_id)?;

    if !room.players.iter().any(|player| player.id == target_id) {
        return Err(RoomError::PlayerNotFound);
    }

    room.host_player_id = Some(target_id);

    for player in &mut room.players {
        player.is_host = player.id == target_id;
    }

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::HostChanged {
        host_player_id: target_id,
        room: snapshot.clone(),
    });

    let _ = room.tx.send(ServerMessage::RoomState {
        room: snapshot,
    });

    Ok(())
}

pub fn broadcast_input(
    store: &RoomStore,
    code: &str,
    player_id: u64,
    event: InputEvent,
) -> Result<(), RoomError> {
    let rooms = lock_rooms(store)?;
    let room = rooms.get(code).ok_or(RoomError::NotFound)?;

    if !room.players.iter().any(|player| player.id == player_id) {
        return Err(RoomError::PlayerNotFound);
    }

    if room.status != RoomStatus::Running {
        return Err(RoomError::NotRunning);
    }

    room.runtime
        .send(RuntimeCommand::Input {
            player_id,
            event: runtime_input_event(event),
        })
        .map_err(|_| RoomError::RuntimeUnavailable)?;

    Ok(())
}

fn runtime_input_event(event: InputEvent) -> RuntimeInputEvent {
    match event {
        InputEvent::KeyDown { key } => RuntimeInputEvent::KeyDown { key },
        InputEvent::KeyUp { key } => RuntimeInputEvent::KeyUp { key },
        InputEvent::PadDown { input } => RuntimeInputEvent::PadDown { input },
        InputEvent::PadUp { input } => RuntimeInputEvent::PadUp { input },
    }
}

fn forward_runtime_outputs(
    mut outputs: tokio::sync::mpsc::Receiver<RuntimeOutput>,
    tx: broadcast::Sender<ServerMessage>,
) {
    tokio::spawn(async move {
        while let Some(output) = outputs.recv().await {
            let message = match output {
                RuntimeOutput::Frame { player_id, svg } => match String::from_utf8(svg) {
                    Ok(svg) if svg.trim_start().starts_with("<svg") => {
                        ServerMessage::Frame { player_id, svg }
                    }
                    Ok(_) => ServerMessage::Error {
                        message: "Runtime respondeu um frame que não é SVG".into(),
                    },
                    Err(_) => ServerMessage::Error {
                        message: "Runtime respondeu um SVG com UTF-8 inválido".into(),
                    },
                },
                RuntimeOutput::Failed(error) => {
                    tracing::error!(%error, "runtime da partida falhou");
                    ServerMessage::Error {
                        message: "Runtime da partida foi encerrado".into(),
                    }
                }
            };

            let _ = tx.send(message);
        }
    });
}

fn ensure_host(room: &Room, player_id: u64) -> Result<(), RoomError> {
    if room.host_player_id == Some(player_id) {
        Ok(())
    } else {
        Err(RoomError::NotHost)
    }
}

fn room_snapshot(room: &Room) -> RoomSnapshot {
    RoomSnapshot {
        code: room.code.clone(),
        game_id: room.game_id,
        status: room.status.clone(),
        min_players: room.min_players,
        max_players: room.max_players,
        players: room.players.clone(),
        can_start: room.host_player_id.is_some()
            && room.status == RoomStatus::Waiting
            && room.players.len() >= room.min_players as usize,
    }
}

fn lock_rooms<'a>(
    store: &'a RoomStore,
) -> Result<MutexGuard<'a, HashMap<String, Room>>, RoomError> {
    store.lock().map_err(|_| RoomError::Internal)
}

fn cleanup_empty_expired_rooms(rooms: &mut HashMap<String, Room>) {
    let now = Instant::now();

    rooms.retain(|_, room| {
        !room.players.is_empty()
            || now.duration_since(room.created_at) < EMPTY_ROOM_TTL
    });
}

fn generate_unique_room_code(rooms: &HashMap<String, Room>) -> String {
    loop {
        let code = random_token(6).to_uppercase();

        if !rooms.contains_key(&code) {
            return code;
        }
    }
}

fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::validate_nickname;
    use crate::rooms::RoomError;

    #[test]
    fn nickname_is_trimmed_and_limited_on_the_server() {
        assert_eq!(validate_nickname("  Jogador  ").unwrap(), "Jogador");
        assert!(matches!(
            validate_nickname("   "),
            Err(RoomError::NicknameRequired)
        ));
        assert!(validate_nickname(&"á".repeat(30)).is_ok());
        assert!(matches!(
            validate_nickname(&"á".repeat(31)),
            Err(RoomError::NicknameTooLong)
        ));
    }
}
