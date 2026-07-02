use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::{
    db::Db,
    rooms::{service, RoomError, RoomSnapshot, RoomStore},
};

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub game_id: i64,
    pub nickname: String,
}

#[derive(Serialize)]
pub struct CreateRoomResponse {
    pub code: String,
    pub host_token: String,
    pub game_id: i64,
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

    let created = service::create_room(
        &rooms,
        payload.game_id,
        min_players,
        max_players,
    )
    .map_err(room_error_to_http)?;

    Ok(Json(CreateRoomResponse {
        code: created.code,
        host_token: created.host_token,
        game_id: payload.game_id,
        nickname,
    }))
}

pub async fn get_room(
    State(rooms): State<RoomStore>,
    Path(code): Path<String>,
) -> Result<Json<RoomSnapshot>, (StatusCode, String)> {
    let room = service::get_room_snapshot(&rooms, &code)
        .map_err(room_error_to_http)?;

    Ok(Json(room))
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

fn room_error_to_http(error: RoomError) -> (StatusCode, String) {
    let status = match error {
        RoomError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        RoomError::NotFound => StatusCode::NOT_FOUND,
        RoomError::NotHost => StatusCode::FORBIDDEN,
        _ => StatusCode::BAD_REQUEST,
    };

    (status, error.message().to_string())
}
