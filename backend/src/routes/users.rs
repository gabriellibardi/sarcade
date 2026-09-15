use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use rusqlite::OptionalExtension;

use crate::{
    db::Db, models::user::PublicUserProfile, routes::games::list_approved_games_by_author,
};

pub async fn get_user_profile(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<PublicUserProfile>, (StatusCode, String)> {
    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let user = conn
        .query_row(
            "SELECT id, name, avatar, created_at FROM users WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;

    let Some((id, name, avatar, created_at)) = user else {
        return Err((StatusCode::NOT_FOUND, "Usuário não encontrado".into()));
    };

    let published_games = list_approved_games_by_author(&conn, id)?;

    Ok(Json(PublicUserProfile {
        id,
        name,
        avatar,
        created_at,
        published_games,
    }))
}

fn database_error(error: rusqlite::Error) -> (StatusCode, String) {
    tracing::error!(%error, "falha ao consultar perfil público");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Falha ao acessar o banco de dados".into(),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::extract::{Path, State};
    use rusqlite::Connection;

    use super::get_user_profile;

    #[tokio::test]
    async fn public_profile_contains_only_published_games_and_safe_user_fields() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(include_str!("../db/schema.sql"))
            .unwrap();
        conn.execute(
            "INSERT INTO users (email, name, avatar) VALUES ('privado@example.com', 'Autor', 'https://example.com/avatar.png')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO games (title, description, author_id) VALUES ('Publicado', 'Visível', 1), ('Pendente', 'Oculto', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "
            INSERT INTO game_revisions
                (game_id, runtime_build_id, source_code, status, reviewed_at)
            VALUES
                (1, 'sgleam-dev', 'fonte publicada', 'approved', '2026-01-01 00:00:00'),
                (2, 'sgleam-dev', 'fonte pendente', 'pending', NULL)
            ",
            [],
        )
        .unwrap();
        let db = Arc::new(Mutex::new(conn));

        let response = get_user_profile(State(db), Path(1)).await.unwrap();
        let json = serde_json::to_value(response.0).unwrap();

        assert_eq!(json["name"], "Autor");
        assert_eq!(json["published_games"].as_array().unwrap().len(), 1);
        assert_eq!(json["published_games"][0]["title"], "Publicado");
        assert!(json["published_games"][0].get("source_code").is_none());
        assert!(json.get("email").is_none());
        assert!(json.get("is_admin").is_none());
    }
}
