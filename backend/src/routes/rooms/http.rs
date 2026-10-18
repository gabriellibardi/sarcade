use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use backend::game_runtime::{
    MAX_PLAYERS_PER_GAME, RuntimeSpec,
    version::{SGLEAM_BUILD_ID, SGLEAM_GIT_COMMIT, SINTERACT_PROTOCOL_VERSION},
};

use crate::{
    auth::current_user,
    db::Db,
    rooms::{RoomError, RoomSnapshot, RoomStore, service},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub game_id: i64,
    pub nickname: String,
}

#[derive(Deserialize)]
pub struct CreateRevisionRoomRequest {
    pub nickname: String,
}

#[derive(Serialize)]
pub struct CreateRoomResponse {
    pub code: String,
    pub host_token: String,
    pub game_id: i64,
    pub nickname: String,
    pub invite_token: Option<String>,
}

#[derive(Deserialize)]
pub struct RoomAccessQuery {
    pub invite_token: Option<String>,
    pub player_token: Option<String>,
}

pub async fn create_room(
    State(state): State<AppState>,
    Json(payload): Json<CreateRoomRequest>,
) -> Result<Json<CreateRoomResponse>, (StatusCode, String)> {
    let nickname = service::validate_nickname(&payload.nickname).map_err(room_error_to_http)?;

    let spec = get_runtime_spec(&state.db, payload.game_id)?;
    let executable = executable_for(&state, &spec)?;
    let runtime_permit = reserve_runtime_process(&state)?;
    let created = service::create_room(&state.rooms, spec, executable, runtime_permit)
        .map_err(room_error_to_http)?;

    Ok(Json(CreateRoomResponse {
        code: created.code,
        host_token: created.host_token,
        game_id: payload.game_id,
        nickname,
        invite_token: created.invite_token,
    }))
}

pub async fn create_revision_room(
    State(state): State<AppState>,
    Path(revision_id): Path<i64>,
    jar: CookieJar,
    Json(payload): Json<CreateRevisionRoomRequest>,
) -> Result<Json<CreateRoomResponse>, (StatusCode, String)> {
    let user = current_user(&state.db, &jar)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Faça login para testar uma revisão".to_string(),
    ))?;
    let nickname = service::validate_nickname(&payload.nickname).map_err(room_error_to_http)?;
    let spec = get_revision_runtime_spec(&state.db, revision_id, user.id, user.is_admin)?;
    let executable = executable_for(&state, &spec)?;
    let runtime_permit = reserve_runtime_process(&state)?;
    let game_id = spec.game_id();
    let created = service::create_preview_room(&state.rooms, spec, executable, runtime_permit)
        .map_err(room_error_to_http)?;

    Ok(Json(CreateRoomResponse {
        code: created.code,
        host_token: created.host_token,
        game_id,
        nickname,
        invite_token: created.invite_token,
    }))
}

fn reserve_runtime_process(
    state: &AppState,
) -> Result<tokio::sync::OwnedSemaphorePermit, (StatusCode, String)> {
    state.reserve_runtime_process().map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "O servidor atingiu o limite de partidas simultâneas; tente novamente em instantes"
                .into(),
        )
    })
}

fn executable_for(state: &AppState, spec: &RuntimeSpec) -> Result<PathBuf, (StatusCode, String)> {
    if spec.runtime() != "sgleam"
        || spec.runtime_build_id() != SGLEAM_BUILD_ID
        || spec.git_commit() != Some(SGLEAM_GIT_COMMIT)
        || spec.validate_protocol(SINTERACT_PROTOCOL_VERSION).is_err()
    {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "Este build de runtime não é compatível com o executor instalado".into(),
        ));
    }
    if spec.max_players() > i64::from(MAX_PLAYERS_PER_GAME) {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "Este jogo excede o limite de participantes da plataforma".into(),
        ));
    }

    state
        .sgleam_executable
        .as_ref()
        .map(|executable| executable.path().to_path_buf())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "Runtime indisponível para criar a sala".into(),
        ))
}

pub async fn get_room(
    State(rooms): State<RoomStore>,
    Path(code): Path<String>,
    Query(query): Query<RoomAccessQuery>,
) -> Result<Json<RoomSnapshot>, (StatusCode, String)> {
    let room = service::get_room_snapshot_with_access(
        &rooms,
        &code,
        query.invite_token.as_deref(),
        query.player_token.as_deref(),
    )
    .map_err(room_error_to_http)?;

    Ok(Json(room))
}

fn get_revision_runtime_spec(
    db: &Db,
    revision_id: i64,
    user_id: i64,
    is_admin: bool,
) -> Result<RuntimeSpec, (StatusCode, String)> {
    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar banco de dados".to_string(),
        )
    })?;
    conn.query_row(
        "
        SELECT
            game_revisions.game_id,
            game_revisions.runtime_build_id,
            runtime_builds.runtime,
            runtime_builds.display_version,
            runtime_builds.git_commit,
            runtime_builds.protocol_version,
            runtime_builds.source_extension,
            runtime_builds.status,
            game_revisions.source_code,
            game_revisions.min_players,
            game_revisions.max_players
        FROM game_revisions
        JOIN games ON games.id = game_revisions.game_id
        JOIN runtime_builds ON runtime_builds.id = game_revisions.runtime_build_id
        WHERE game_revisions.id = ?1
          AND game_revisions.status IN ('draft', 'pending', 'approved', 'rejected')
          AND games.archived_at IS NULL
          AND (games.author_id = ?2 OR ?3 = 1)
        ",
        rusqlite::params![revision_id, user_id, i64::from(is_admin)],
        |row| {
            Ok(RuntimeSpec::new(
                row.get(0)?,
                revision_id,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
            ))
        },
    )
    .optional()
    .map_err(|error| {
        tracing::error!(%error, revision_id, "falha ao selecionar revisão de teste");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar banco de dados".to_string(),
        )
    })?
    .ok_or((StatusCode::NOT_FOUND, "Revisão não encontrada".into()))
}

fn get_runtime_spec(db: &Db, game_id: i64) -> Result<RuntimeSpec, (StatusCode, String)> {
    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar banco de dados".to_string(),
        )
    })?;

    let result = conn
        .query_row(
            "
            SELECT
                game_revisions.id,
                game_revisions.runtime_build_id,
                runtime_builds.runtime,
                runtime_builds.display_version,
                runtime_builds.git_commit,
                runtime_builds.protocol_version,
                runtime_builds.source_extension,
                runtime_builds.status,
                game_revisions.source_code,
                game_revisions.min_players,
                game_revisions.max_players
            FROM game_revisions
            JOIN games
              ON games.id = game_revisions.game_id
            JOIN runtime_builds
              ON runtime_builds.id = game_revisions.runtime_build_id
            WHERE game_revisions.game_id = ?1
              AND games.archived_at IS NULL
              AND game_revisions.status = 'approved'
              AND runtime_builds.status IN ('development', 'active')
            ORDER BY
                COALESCE(
                    game_revisions.reviewed_at,
                    game_revisions.created_at
                ) DESC,
                game_revisions.id DESC
            LIMIT 1
            ",
            [game_id],
            |row| {
                Ok(RuntimeSpec::new(
                    game_id,
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            },
        )
        .optional()
        .map_err(|error| {
            tracing::error!(%error, game_id, "falha ao selecionar runtime da sala");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Falha ao acessar banco de dados".to_string(),
            )
        })?;

    result.ok_or((StatusCode::NOT_FOUND, "Jogo não encontrado".into()))
}

fn room_error_to_http(error: RoomError) -> (StatusCode, String) {
    let status = match error {
        RoomError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        RoomError::NotFound => StatusCode::NOT_FOUND,
        RoomError::NotHost | RoomError::InvalidInviteToken => StatusCode::FORBIDDEN,
        _ => StatusCode::BAD_REQUEST,
    };

    (status, error.message().to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use rusqlite::Connection;

    use backend::game_runtime::version::{
        SGLEAM_BUILD_ID, SGLEAM_GIT_COMMIT, SGLEAM_VERSION, SINTERACT_PROTOCOL_VERSION,
    };

    use super::get_runtime_spec;

    #[test]
    fn room_pins_the_latest_approved_revision_and_its_runtime_build() {
        let db = test_db();

        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO users (email, name) VALUES ('autor@example.com', 'Autora')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO games (title, description, author_id) VALUES ('Jogo', 'Descrição', 1)",
                [],
            )
            .unwrap();
            conn.execute(
                "
                INSERT INTO game_revisions
                    (game_id, revision_number, runtime_build_id, title, description,
                     source_code, status, min_players, max_players, reviewed_at)
                VALUES
                    (1, 1, 'spython-dev', 'Jogo', 'Descrição', 'fonte antiga', 'approved', 1, 2, '2026-01-01 00:00:00'),
                    (1, 2, ?1, 'Jogo', 'Descrição', 'fonte aprovada', 'approved', 2, 4, '2026-02-01 00:00:00'),
                    (1, 3, 'spython-dev', 'Jogo futuro', 'Descrição futura', 'fonte pendente', 'pending', 8, 8, NULL)
                ",
                [SGLEAM_BUILD_ID],
            )
            .unwrap();
        }

        let spec = get_runtime_spec(&db, 1).unwrap();

        assert_eq!(spec.game_id(), 1);
        assert_eq!(spec.revision_id(), 2);
        assert_eq!(spec.runtime_build_id(), SGLEAM_BUILD_ID);
        assert_eq!(spec.runtime(), "sgleam");
        assert_eq!(spec.display_version(), SGLEAM_VERSION);
        assert_eq!(spec.git_commit(), Some(SGLEAM_GIT_COMMIT));
        assert_eq!(
            spec.protocol_version(),
            Some(i64::from(SINTERACT_PROTOCOL_VERSION))
        );
        assert_eq!(spec.source_extension(), ".gleam");
        assert_eq!(spec.runtime_status(), "active");
        assert_eq!(spec.source_code(), "fonte aprovada");
        assert_eq!(spec.min_players(), 2);
        assert_eq!(spec.max_players(), 4);
    }

    fn test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::db::initialize_schema(&conn).unwrap();
        Arc::new(Mutex::new(conn))
    }
}
