use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use rusqlite::OptionalExtension;
use serde::Serialize;

use backend::game_runtime::MAX_PLAYERS_PER_GAME;

use crate::{auth::current_user, state::AppState};

use super::{
    inspect::inspect_source_code,
    submit::{MAX_SOURCE_CODE_LEN, MAX_THUMBNAIL_SIZE, validate_text_fields},
    thumbnail::validate_thumbnail,
};

pub async fn archive_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    crate::auth::require_admin(&state.db, &jar)?;
    let conn = lock_db(&state)?;
    let affected = conn
        .execute(
            "UPDATE games SET archived_at = datetime('now') WHERE id = ?1 AND archived_at IS NULL",
            [id],
        )
        .map_err(|error| internal_error(&error))?;
    if affected == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "Jogo não encontrado ou já arquivado".into(),
        ));
    }
    Ok(Json(serde_json::json!({
        "game_id": id,
        "archived": true
    })))
}

#[derive(Debug, Serialize)]
pub struct RevisionMutation {
    pub game_id: i64,
    pub revision_id: i64,
    pub revision_number: i64,
    pub status: String,
}

pub async fn create_editable_revision(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Result<Json<RevisionMutation>, (StatusCode, String)> {
    let user = current_user(&state.db, &jar)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Faça login para editar uma revisão".to_string(),
    ))?;
    let mut conn = lock_db(&state)?;
    let transaction = conn.transaction().map_err(|error| internal_error(&error))?;
    let source = transaction
        .query_row(
            "
            SELECT
                game_revisions.game_id,
                game_revisions.revision_number,
                game_revisions.status,
                games.author_id,
                games.archived_at
            FROM game_revisions
            JOIN games ON games.id = game_revisions.game_id
            WHERE game_revisions.id = ?1
            ",
            [id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|error| internal_error(&error))?
        .ok_or((StatusCode::NOT_FOUND, "Revisão não encontrada".into()))?;

    if source.3 != Some(user.id) && !user.is_admin {
        return Err((StatusCode::NOT_FOUND, "Revisão não encontrada".into()));
    }
    if source.4.is_some() {
        return Err((StatusCode::CONFLICT, "O jogo está arquivado".into()));
    }
    if source.2 == "draft" {
        return Ok(Json(RevisionMutation {
            game_id: source.0,
            revision_id: id,
            revision_number: source.1,
            status: source.2,
        }));
    }

    let active: i64 = transaction
        .query_row(
            "
            SELECT COUNT(*)
            FROM game_revisions
            WHERE game_id = ?1
              AND id <> ?2
              AND status IN ('draft', 'pending')
            ",
            rusqlite::params![source.0, id],
            |row| row.get(0),
        )
        .map_err(|error| internal_error(&error))?;
    if active > 0 {
        return Err((
            StatusCode::CONFLICT,
            "Este jogo já possui um rascunho ou uma revisão pendente".into(),
        ));
    }

    if source.2 == "pending" {
        transaction
            .execute(
                "UPDATE game_revisions SET status = 'withdrawn', updated_at = datetime('now') WHERE id = ?1 AND status = 'pending'",
                [id],
            )
            .map_err(|error| internal_error(&error))?;
    }

    transaction
        .execute(
            "
            INSERT INTO game_revisions (
                game_id,
                revision_number,
                parent_revision_id,
                created_by_user_id,
                runtime_build_id,
                title,
                description,
                thumbnail,
                source_code,
                min_players,
                max_players,
                status
            )
            SELECT
                source.game_id,
                (
                    SELECT COALESCE(MAX(existing.revision_number), 0) + 1
                    FROM game_revisions AS existing
                    WHERE existing.game_id = source.game_id
                ),
                source.id,
                ?2,
                source.runtime_build_id,
                source.title,
                source.description,
                source.thumbnail,
                source.source_code,
                source.min_players,
                source.max_players,
                'draft'
            FROM game_revisions AS source
            WHERE source.id = ?1
            ",
            rusqlite::params![id, user.id],
        )
        .map_err(|error| internal_error(&error))?;
    let revision_id = transaction.last_insert_rowid();
    let revision_number = transaction
        .query_row(
            "SELECT revision_number FROM game_revisions WHERE id = ?1",
            [revision_id],
            |row| row.get(0),
        )
        .map_err(|error| internal_error(&error))?;
    transaction
        .commit()
        .map_err(|error| internal_error(&error))?;

    Ok(Json(RevisionMutation {
        game_id: source.0,
        revision_id,
        revision_number,
        status: "draft".into(),
    }))
}

pub async fn update_draft(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Result<Json<RevisionMutation>, (StatusCode, String)> {
    let user = current_user(&state.db, &jar)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Faça login para editar uma revisão".to_string(),
    ))?;

    let mut title = None;
    let mut description = None;
    let mut runtime_build_id = None;
    let mut source_code = None;
    let mut uploaded_filename = None;
    let mut thumbnail: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?
    {
        match field.name().unwrap_or("") {
            "title" => title = Some(field.text().await.unwrap_or_default()),
            "description" => description = Some(field.text().await.unwrap_or_default()),
            "runtime_build_id" => {
                runtime_build_id = Some(field.text().await.unwrap_or_default());
            }
            "source_code" => {
                uploaded_filename = field.file_name().map(str::to_string);
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
                if bytes.len() > MAX_SOURCE_CODE_LEN {
                    return Err((StatusCode::BAD_REQUEST, "Código-fonte muito grande".into()));
                }
                source_code = Some(String::from_utf8(bytes.to_vec()).map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        "O código-fonte deve estar em UTF-8 válido".to_string(),
                    )
                })?);
            }
            "thumbnail" => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
                if !bytes.is_empty() {
                    thumbnail = Some(bytes.to_vec());
                }
            }
            _ => {}
        }
    }

    let title = title.unwrap_or_default().trim().to_string();
    let description = description.unwrap_or_default().trim().to_string();
    let source_code = source_code.unwrap_or_default().trim().to_string();
    validate_text_fields(&title, &description, &source_code)?;
    if thumbnail
        .as_ref()
        .is_some_and(|bytes| bytes.len() > MAX_THUMBNAIL_SIZE)
    {
        return Err((StatusCode::BAD_REQUEST, "Thumbnail muito grande".into()));
    }
    if let Some(thumbnail) = &thumbnail {
        validate_thumbnail(thumbnail)?;
    }
    let runtime_build_id = runtime_build_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Selecione uma linguagem e uma versão".to_string(),
        ))?;

    ensure_editable(&state, id, user.id, user.is_admin)?;
    let inspected = inspect_source_code(
        &state,
        &runtime_build_id,
        &source_code,
        uploaded_filename.as_deref(),
    )
    .await?;

    let conn = lock_db(&state)?;
    let affected = conn
        .execute(
            "
            UPDATE game_revisions
            SET runtime_build_id = ?2,
                title = ?3,
                description = ?4,
                thumbnail = COALESCE(?5, thumbnail),
                source_code = ?6,
                min_players = ?7,
                max_players = ?8,
                updated_at = datetime('now')
            WHERE id = ?1
              AND status = 'draft'
              AND EXISTS (
                  SELECT 1
                  FROM games
                  WHERE games.id = game_revisions.game_id
                    AND games.archived_at IS NULL
                    AND (games.author_id = ?9 OR ?10 = 1)
              )
            ",
            rusqlite::params![
                id,
                runtime_build_id,
                title,
                description,
                thumbnail,
                source_code,
                inspected.min_players,
                inspected.max_players,
                user.id,
                i64::from(user.is_admin),
            ],
        )
        .map_err(|error| internal_error(&error))?;
    if affected == 0 {
        return Err((
            StatusCode::CONFLICT,
            "O rascunho foi alterado ou não está mais disponível para edição".into(),
        ));
    }
    let (game_id, revision_number) = conn
        .query_row(
            "SELECT game_id, revision_number FROM game_revisions WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| internal_error(&error))?;

    Ok(Json(RevisionMutation {
        game_id,
        revision_id: id,
        revision_number,
        status: "draft".into(),
    }))
}

pub async fn submit_draft(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Result<Json<RevisionMutation>, (StatusCode, String)> {
    let user = current_user(&state.db, &jar)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Faça login para enviar uma revisão".to_string(),
    ))?;
    let conn = lock_db(&state)?;
    let affected = conn
        .execute(
            "
            UPDATE game_revisions
            SET status = 'pending',
                submitted_at = datetime('now'),
                updated_at = datetime('now')
            WHERE id = ?1
              AND status = 'draft'
              AND max_players <= ?4
              AND EXISTS (
                  SELECT 1
                  FROM games
                  WHERE games.id = game_revisions.game_id
                    AND games.archived_at IS NULL
                    AND (games.author_id = ?2 OR ?3 = 1)
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM game_revisions AS pending
                  WHERE pending.game_id = game_revisions.game_id
                    AND pending.id <> game_revisions.id
                    AND pending.status = 'pending'
              )
              AND EXISTS (
                  SELECT 1
                  FROM runtime_builds
                  WHERE runtime_builds.id = game_revisions.runtime_build_id
                    AND runtime_builds.status IN ('development', 'active')
              )
            ",
            rusqlite::params![id, user.id, i64::from(user.is_admin), MAX_PLAYERS_PER_GAME],
        )
        .map_err(|error| internal_error(&error))?;
    if affected == 0 {
        return Err((
            StatusCode::CONFLICT,
            "O rascunho não pode mais ser enviado para revisão".into(),
        ));
    }
    let (game_id, revision_number) = conn
        .query_row(
            "SELECT game_id, revision_number FROM game_revisions WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| internal_error(&error))?;

    Ok(Json(RevisionMutation {
        game_id,
        revision_id: id,
        revision_number,
        status: "pending".into(),
    }))
}

fn ensure_editable(
    state: &AppState,
    revision_id: i64,
    user_id: i64,
    is_admin: bool,
) -> Result<(), (StatusCode, String)> {
    let conn = lock_db(state)?;
    let exists = conn
        .query_row(
            "
            SELECT EXISTS(
                SELECT 1
                FROM game_revisions
                JOIN games ON games.id = game_revisions.game_id
                WHERE game_revisions.id = ?1
                  AND game_revisions.status = 'draft'
                  AND games.archived_at IS NULL
                  AND (games.author_id = ?2 OR ?3 = 1)
            )
            ",
            rusqlite::params![revision_id, user_id, i64::from(is_admin)],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| internal_error(&error))?;
    if exists == 0 {
        return Err((StatusCode::NOT_FOUND, "Rascunho não encontrado".into()));
    }
    Ok(())
}

fn lock_db(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, (StatusCode, String)> {
    state.db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })
}

fn internal_error(error: &rusqlite::Error) -> (StatusCode, String) {
    tracing::error!(%error, "falha ao alterar revisão");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Falha ao acessar o banco de dados".into(),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::{
        extract::{Path, State},
        http::StatusCode,
    };
    use axum_extra::extract::cookie::{Cookie, CookieJar};
    use rusqlite::Connection;

    use super::{create_editable_revision, submit_draft};
    use crate::{
        auth::{AuthService, SESSION_COOKIE, hash_token},
        state::AppState,
    };

    #[tokio::test]
    async fn editing_a_pending_revision_withdraws_it_and_creates_a_draft() {
        let (state, jar) = state_with_revision("pending");

        let response = create_editable_revision(State(state.clone()), Path(1), jar.clone())
            .await
            .unwrap()
            .0;

        assert_eq!(response.revision_id, 2);
        assert_eq!(response.revision_number, 2);
        assert_eq!(response.status, "draft");
        {
            let conn = state.db.lock().unwrap();
            let old: String = conn
                .query_row(
                    "SELECT status FROM game_revisions WHERE id = 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(old, "withdrawn");
        }

        let submitted = submit_draft(State(state.clone()), Path(2), jar)
            .await
            .unwrap()
            .0;
        assert_eq!(submitted.status, "pending");
        let conn = state.db.lock().unwrap();
        let status: String = conn
            .query_row(
                "SELECT status FROM game_revisions WHERE id = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "pending");
    }

    #[tokio::test]
    async fn another_author_cannot_clone_a_revision() {
        let (state, _) = state_with_revision("rejected");
        {
            let conn = state.db.lock().unwrap();
            conn.execute(
                "INSERT INTO users (email, name) VALUES ('outra@example.com', 'Outra')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO sessions (id, user_id, expires_at) VALUES (?1, 2, datetime('now', '+1 day'))",
                [hash_token("other-token")],
            )
            .unwrap();
        }
        let jar = CookieJar::new().add(Cookie::new(SESSION_COOKIE, "other-token"));

        let error = create_editable_revision(State(state.clone()), Path(1), jar)
            .await
            .unwrap_err();

        assert_eq!(error.0, StatusCode::NOT_FOUND);
        let conn = state.db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM game_revisions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    fn state_with_revision(status: &str) -> (AppState, CookieJar) {
        use backend::game_runtime::RuntimeCatalog;

        use crate::db::{TEST_RUNTIME_BUILD_ID, install_test_runtime};

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::db::initialize_schema(&conn).unwrap();
        install_test_runtime(&conn, "active").unwrap();
        conn.execute(
            "INSERT INTO users (email, name) VALUES ('autor@example.com', 'Autora')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions (id, user_id, expires_at) VALUES (?1, 1, datetime('now', '+1 day'))",
            [hash_token("author-token")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO games (title, description, author_id) VALUES ('Jogo', 'Descrição', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "
            INSERT INTO game_revisions (
                game_id, revision_number, created_by_user_id, runtime_build_id,
                title, description, source_code, status, submitted_at
            )
            VALUES (1, 1, 1, ?1, 'Jogo', 'Descrição', 'fonte', ?2, datetime('now'))
            ",
            [TEST_RUNTIME_BUILD_ID, status],
        )
        .unwrap();
        let state = AppState::new(
            Arc::new(Mutex::new(conn)),
            AuthService::new(None, vec![]),
            RuntimeCatalog::default(),
            4,
        );
        let jar = CookieJar::new().add(Cookie::new(SESSION_COOKIE, "author-token"));
        (state, jar)
    }
}
