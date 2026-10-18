use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use serde_json::{Value, json};

use backend::game_runtime::MAX_PLAYERS_PER_GAME;

use crate::{auth::require_admin, db::Db};

#[derive(Deserialize)]
pub struct RejectRequest {
    pub message: String,
}

pub async fn approve_revision(
    State(db): State<Db>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&db, &jar)?;

    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let affected = conn
        .execute(
            "
            UPDATE game_revisions
            SET
                status = 'approved',
                review_message = NULL,
                updated_at = datetime('now'),
                reviewed_at = datetime('now')
            WHERE id = ?1
              AND status = 'pending'
              AND max_players <= ?2
              AND EXISTS (
                  SELECT 1
                  FROM runtime_builds
                  WHERE runtime_builds.id = game_revisions.runtime_build_id
                    AND runtime_builds.status IN ('development', 'active')
              )
            ",
            rusqlite::params![id, MAX_PLAYERS_PER_GAME],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if affected == 0 {
        return Err((
            StatusCode::CONFLICT,
            "Revisão não encontrada, já analisada, acima dos limites ou com runtime indisponível"
                .into(),
        ));
    }

    Ok(Json(json!({
        "revision_id": id,
        "status": "approved",
        "message": "Revisão aprovada com sucesso"
    })))
}

pub async fn reject_revision(
    State(db): State<Db>,
    Path(id): Path<i64>,
    jar: CookieJar,
    Json(payload): Json<RejectRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&db, &jar)?;

    let message = payload.message.trim().to_string();

    if message.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Mensagem de rejeição é obrigatória".into(),
        ));
    }

    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let affected = conn
        .execute(
            "
            UPDATE game_revisions
            SET
                status = 'rejected',
                review_message = ?2,
                updated_at = datetime('now'),
                reviewed_at = datetime('now')
            WHERE id = ?1
              AND status = 'pending'
            ",
            rusqlite::params![id, message],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if affected == 0 {
        return Err((
            StatusCode::CONFLICT,
            "Revisão não encontrada ou já analisada".into(),
        ));
    }

    Ok(Json(json!({
        "revision_id": id,
        "status": "rejected",
        "message": "Revisão rejeitada com sucesso"
    })))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::{
        Json,
        extract::{Path, State},
        http::StatusCode,
    };
    use axum_extra::extract::cookie::{Cookie, CookieJar};
    use rusqlite::Connection;

    use backend::game_runtime::version::SGLEAM_BUILD_ID;

    use super::{RejectRequest, approve_revision, reject_revision};
    use crate::{
        auth::{AuthService, SESSION_COOKIE, hash_token},
        routes::games::{author::create_editable_revision, author::submit_draft, list::get_game},
        state::AppState,
    };

    #[tokio::test]
    async fn author_submission_requires_an_admin_before_becoming_public() {
        let fixture = fixture();

        let submitted = submit_draft(
            State(fixture.state.clone()),
            Path(1),
            fixture.author.clone(),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(submitted.status, "pending");

        let forbidden = approve_revision(
            State(fixture.state.db.clone()),
            Path(1),
            fixture.author.clone(),
        )
        .await
        .unwrap_err();
        assert_eq!(forbidden.0, StatusCode::FORBIDDEN);
        assert!(
            get_game(State(fixture.state.db.clone()), Path(1))
                .await
                .is_err()
        );

        let _ = approve_revision(State(fixture.state.db.clone()), Path(1), fixture.admin)
            .await
            .unwrap();

        let published = get_game(State(fixture.state.db), Path(1)).await.unwrap().0;
        let published = serde_json::to_value(published).unwrap();
        assert_eq!(published["revision_id"], 1);
        assert_eq!(published["runtime_build_id"], SGLEAM_BUILD_ID);
        assert_eq!(published["min_players"], 2);
        assert_eq!(published["max_players"], 2);
        assert!(published.get("source_code").is_none());
    }

    #[tokio::test]
    async fn rejected_revision_keeps_its_reason_and_can_be_resubmitted_as_a_new_version() {
        let fixture = fixture();
        let _ = submit_draft(
            State(fixture.state.clone()),
            Path(1),
            fixture.author.clone(),
        )
        .await
        .unwrap();

        let _ = reject_revision(
            State(fixture.state.db.clone()),
            Path(1),
            fixture.admin.clone(),
            Json(RejectRequest {
                message: "Ajuste os controles".into(),
            }),
        )
        .await
        .unwrap();

        let draft = create_editable_revision(
            State(fixture.state.clone()),
            Path(1),
            fixture.author.clone(),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(draft.revision_id, 2);
        assert_eq!(draft.revision_number, 2);
        assert_eq!(draft.status, "draft");

        {
            let conn = fixture.state.db.lock().unwrap();
            let rejected: (String, String) = conn
                .query_row(
                    "SELECT status, review_message FROM game_revisions WHERE id = 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            let parent: i64 = conn
                .query_row(
                    "SELECT parent_revision_id FROM game_revisions WHERE id = 2",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(rejected, ("rejected".into(), "Ajuste os controles".into()));
            assert_eq!(parent, 1);
        }

        let _ = submit_draft(State(fixture.state.clone()), Path(2), fixture.author)
            .await
            .unwrap();
        let _ = approve_revision(State(fixture.state.db.clone()), Path(2), fixture.admin)
            .await
            .unwrap();

        let published = get_game(State(fixture.state.db), Path(1)).await.unwrap().0;
        assert_eq!(published.revision_id, 2);
    }

    struct Fixture {
        state: AppState,
        author: CookieJar,
        admin: CookieJar,
    }

    fn fixture() -> Fixture {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::db::initialize_schema(&conn).unwrap();
        conn.execute_batch(
            "
            INSERT INTO users (id, email, name, is_admin) VALUES
                (1, 'autor@example.com', 'Autora', 0),
                (2, 'admin@example.com', 'Admin', 1);
            INSERT INTO games (id, title, description, author_id)
                VALUES (1, 'Jogo', 'Descrição', 1);
            ",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions (id, user_id, expires_at) VALUES (?1, 1, datetime('now', '+1 day'))",
            [hash_token("author-token")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions (id, user_id, expires_at) VALUES (?1, 2, datetime('now', '+1 day'))",
            [hash_token("admin-token")],
        )
        .unwrap();
        conn.execute(
            "
            INSERT INTO game_revisions (
                id, game_id, revision_number, created_by_user_id, runtime_build_id,
                title, description, source_code, min_players, max_players, status
            ) VALUES (1, 1, 1, 1, ?1, 'Jogo', 'Descrição', 'fonte aprovada', 2, 2, 'draft')
            ",
            [SGLEAM_BUILD_ID],
        )
        .unwrap();

        Fixture {
            state: AppState::new(
                Arc::new(Mutex::new(conn)),
                AuthService::new(None, vec![]),
                None,
                4,
            ),
            author: jar("author-token"),
            admin: jar("admin-token"),
        }
    }

    fn jar(token: &'static str) -> CookieJar {
        CookieJar::new().add(Cookie::new(SESSION_COOKIE, token))
    }
}
