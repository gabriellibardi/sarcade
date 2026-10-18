use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use rusqlite::{OptionalExtension, Row};

use crate::{
    auth::{current_user, require_admin},
    db::Db,
    models::game::{GameDetails, GameSummary, OwnedGame, OwnedRevision, PublishedGameDetails},
};

const SUMMARY_COLUMNS: &str = "
    games.id,
    game_revisions.id,
    game_revisions.title,
    game_revisions.description,
    game_revisions.status,
    game_revisions.min_players,
    game_revisions.max_players,
    users.id,
    users.name,
    games.is_featured,
    game_revisions.created_at,
    game_revisions.thumbnail IS NOT NULL,
    runtime_builds.id,
    runtime_builds.runtime,
    runtime_builds.display_name,
    runtime_builds.display_version,
    runtime_builds.source_extension
";

pub async fn list_pending_games(
    State(db): State<Db>,
    jar: CookieJar,
) -> Result<Json<Vec<GameSummary>>, (StatusCode, String)> {
    require_admin(&db, &jar)?;
    let conn = lock_db(&db)?;
    let sql = format!(
        "
        SELECT {SUMMARY_COLUMNS}
        FROM game_revisions
        JOIN games ON games.id = game_revisions.game_id
        JOIN runtime_builds ON runtime_builds.id = game_revisions.runtime_build_id
        LEFT JOIN users ON users.id = games.author_id
        WHERE game_revisions.status = 'pending'
          AND games.archived_at IS NULL
        ORDER BY game_revisions.created_at DESC, game_revisions.id DESC
        "
    );

    let games = query_summaries(&conn, &sql)?;
    Ok(Json(games))
}

pub async fn list_approved_games(
    State(db): State<Db>,
) -> Result<Json<Vec<GameSummary>>, (StatusCode, String)> {
    let conn = lock_db(&db)?;
    let sql = format!(
        "
        SELECT {SUMMARY_COLUMNS}
        FROM games
        JOIN game_revisions ON game_revisions.game_id = games.id
        JOIN runtime_builds ON runtime_builds.id = game_revisions.runtime_build_id
        LEFT JOIN users ON users.id = games.author_id
        WHERE game_revisions.status = 'approved'
          AND games.archived_at IS NULL
          AND runtime_builds.status IN ('development', 'active')
          AND game_revisions.id = (
              SELECT latest.id
              FROM game_revisions AS latest
              WHERE latest.game_id = games.id
                AND latest.status = 'approved'
                AND EXISTS (
                    SELECT 1
                    FROM runtime_builds AS latest_runtime
                    WHERE latest_runtime.id = latest.runtime_build_id
                      AND latest_runtime.status IN ('development', 'active')
                )
              ORDER BY COALESCE(latest.reviewed_at, latest.created_at) DESC, latest.id DESC
              LIMIT 1
          )
        ORDER BY games.is_featured DESC, game_revisions.reviewed_at DESC, games.id DESC
        "
    );

    let games = query_summaries(&conn, &sql)?;
    Ok(Json(games))
}

pub async fn get_game(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<PublishedGameDetails>, (StatusCode, String)> {
    let conn = lock_db(&db)?;

    let game = conn
        .query_row(
            "
            SELECT
                games.id,
                game_revisions.id,
                game_revisions.title,
                game_revisions.description,
                game_revisions.min_players,
                game_revisions.max_players,
                users.id,
                users.name,
                games.is_featured,
                game_revisions.created_at,
                game_revisions.reviewed_at,
                game_revisions.thumbnail IS NOT NULL,
                runtime_builds.id,
                runtime_builds.runtime,
                runtime_builds.display_name,
                runtime_builds.display_version,
                runtime_builds.source_extension
            FROM games
            JOIN game_revisions ON game_revisions.game_id = games.id
            JOIN runtime_builds ON runtime_builds.id = game_revisions.runtime_build_id
            LEFT JOIN users ON users.id = games.author_id
            WHERE games.id = ?1
              AND games.archived_at IS NULL
              AND game_revisions.status = 'approved'
              AND runtime_builds.status IN ('development', 'active')
            ORDER BY COALESCE(game_revisions.reviewed_at, game_revisions.created_at) DESC,
                     game_revisions.id DESC
            LIMIT 1
            ",
            [id],
            published_game_details_from_row,
        )
        .optional()
        .map_err(internal_error)?;

    match game {
        Some(game) => Ok(Json(game)),
        None => Err((StatusCode::NOT_FOUND, "Jogo aprovado não encontrado".into())),
    }
}

pub(crate) fn list_approved_games_by_author(
    conn: &rusqlite::Connection,
    author_id: i64,
) -> Result<Vec<GameSummary>, (StatusCode, String)> {
    let sql = format!(
        "
        SELECT {SUMMARY_COLUMNS}
        FROM games
        JOIN game_revisions ON game_revisions.game_id = games.id
        JOIN runtime_builds ON runtime_builds.id = game_revisions.runtime_build_id
        LEFT JOIN users ON users.id = games.author_id
        WHERE games.author_id = ?1
          AND games.archived_at IS NULL
          AND game_revisions.status = 'approved'
          AND runtime_builds.status IN ('development', 'active')
          AND game_revisions.id = (
              SELECT latest.id
              FROM game_revisions AS latest
              WHERE latest.game_id = games.id
                AND latest.status = 'approved'
                AND EXISTS (
                    SELECT 1
                    FROM runtime_builds AS latest_runtime
                    WHERE latest_runtime.id = latest.runtime_build_id
                      AND latest_runtime.status IN ('development', 'active')
                )
              ORDER BY COALESCE(latest.reviewed_at, latest.created_at) DESC, latest.id DESC
              LIMIT 1
          )
        ORDER BY games.is_featured DESC, game_revisions.reviewed_at DESC, games.id DESC
        "
    );
    let mut stmt = conn.prepare(&sql).map_err(internal_error)?;

    stmt.query_map([author_id], game_summary_from_row)
        .map_err(internal_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)
}

pub async fn get_revision(
    State(db): State<Db>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Result<Json<GameDetails>, (StatusCode, String)> {
    let user = current_user(&db, &jar)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Faça login para consultar uma revisão".to_string(),
    ))?;
    let conn = lock_db(&db)?;

    let revision = conn
        .query_row(
            "
            SELECT
                games.id,
                game_revisions.id,
                game_revisions.revision_number,
                game_revisions.title,
                game_revisions.description,
                game_revisions.source_code,
                game_revisions.status,
                game_revisions.min_players,
                game_revisions.max_players,
                users.id,
                users.name,
                games.is_featured,
                game_revisions.review_message,
                game_revisions.created_at,
                game_revisions.updated_at,
                game_revisions.submitted_at,
                game_revisions.reviewed_at,
                game_revisions.thumbnail IS NOT NULL,
                runtime_builds.id,
                runtime_builds.runtime,
                runtime_builds.display_name,
                runtime_builds.display_version,
                runtime_builds.source_extension
            FROM game_revisions
            JOIN games ON games.id = game_revisions.game_id
            JOIN runtime_builds ON runtime_builds.id = game_revisions.runtime_build_id
            LEFT JOIN users ON users.id = games.author_id
            WHERE game_revisions.id = ?1
              AND (games.author_id = ?2 OR ?3 = 1)
            ",
            rusqlite::params![id, user.id, i64::from(user.is_admin)],
            game_details_from_row,
        )
        .optional()
        .map_err(internal_error)?;

    match revision {
        Some(revision) => Ok(Json(revision)),
        None => Err((StatusCode::NOT_FOUND, "Revisão não encontrada".into())),
    }
}

pub async fn list_my_games(
    State(db): State<Db>,
    jar: CookieJar,
) -> Result<Json<Vec<OwnedGame>>, (StatusCode, String)> {
    let user = current_user(&db, &jar)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Faça login para consultar seus jogos".to_string(),
    ))?;
    let conn = lock_db(&db)?;
    let mut statement = conn
        .prepare(
            "
            SELECT
                games.id,
                games.archived_at,
                game_revisions.id,
                game_revisions.revision_number,
                game_revisions.title,
                game_revisions.description,
                game_revisions.status,
                game_revisions.min_players,
                game_revisions.max_players,
                game_revisions.review_message,
                game_revisions.created_at,
                game_revisions.updated_at,
                game_revisions.submitted_at,
                game_revisions.reviewed_at,
                game_revisions.thumbnail IS NOT NULL,
                runtime_builds.id,
                runtime_builds.display_name,
                runtime_builds.display_version
            FROM games
            JOIN game_revisions ON game_revisions.game_id = games.id
            JOIN runtime_builds ON runtime_builds.id = game_revisions.runtime_build_id
            WHERE games.author_id = ?1
            ORDER BY games.id DESC, game_revisions.revision_number DESC
            ",
        )
        .map_err(internal_error)?;
    let rows = statement
        .query_map([user.id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                OwnedRevision {
                    revision_id: row.get(2)?,
                    revision_number: row.get(3)?,
                    title: row.get(4)?,
                    description: row.get(5)?,
                    status: row.get(6)?,
                    min_players: row.get(7)?,
                    max_players: row.get(8)?,
                    review_message: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                    submitted_at: row.get(12)?,
                    reviewed_at: row.get(13)?,
                    has_thumbnail: row.get::<_, i64>(14)? != 0,
                    runtime_build_id: row.get(15)?,
                    runtime_name: row.get(16)?,
                    runtime_version: row.get(17)?,
                },
            ))
        })
        .map_err(internal_error)?;

    let mut games: Vec<OwnedGame> = Vec::new();
    for row in rows {
        let (game_id, archived_at, revision) = row.map_err(internal_error)?;
        if let Some(game) = games.last_mut()
            && game.id == game_id
        {
            game.revisions.push(revision);
        } else {
            games.push(OwnedGame {
                id: game_id,
                archived_at,
                revisions: vec![revision],
            });
        }
    }

    Ok(Json(games))
}

fn lock_db(
    db: &Db,
) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, (StatusCode, String)> {
    db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })
}

fn query_summaries(
    conn: &rusqlite::Connection,
    sql: &str,
) -> Result<Vec<GameSummary>, (StatusCode, String)> {
    let mut stmt = conn.prepare(sql).map_err(internal_error)?;

    stmt.query_map([], game_summary_from_row)
        .map_err(internal_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)
}

fn internal_error(error: rusqlite::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn game_summary_from_row(row: &Row<'_>) -> rusqlite::Result<GameSummary> {
    Ok(GameSummary {
        id: row.get(0)?,
        revision_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        status: row.get(4)?,
        min_players: row.get(5)?,
        max_players: row.get(6)?,
        author_id: row.get(7)?,
        author_name: row.get(8)?,
        is_featured: row.get::<_, i64>(9)? != 0,
        created_at: row.get(10)?,
        has_thumbnail: row.get::<_, i64>(11)? != 0,
        runtime_build_id: row.get(12)?,
        runtime: row.get(13)?,
        runtime_name: row.get(14)?,
        runtime_version: row.get(15)?,
        source_extension: row.get(16)?,
    })
}

fn game_details_from_row(row: &Row<'_>) -> rusqlite::Result<GameDetails> {
    Ok(GameDetails {
        id: row.get(0)?,
        revision_id: row.get(1)?,
        revision_number: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        source_code: row.get(5)?,
        status: row.get(6)?,
        min_players: row.get(7)?,
        max_players: row.get(8)?,
        author_id: row.get(9)?,
        author_name: row.get(10)?,
        is_featured: row.get::<_, i64>(11)? != 0,
        review_message: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        submitted_at: row.get(15)?,
        reviewed_at: row.get(16)?,
        has_thumbnail: row.get::<_, i64>(17)? != 0,
        runtime_build_id: row.get(18)?,
        runtime: row.get(19)?,
        runtime_name: row.get(20)?,
        runtime_version: row.get(21)?,
        source_extension: row.get(22)?,
    })
}

fn published_game_details_from_row(row: &Row<'_>) -> rusqlite::Result<PublishedGameDetails> {
    Ok(PublishedGameDetails {
        id: row.get(0)?,
        revision_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        min_players: row.get(4)?,
        max_players: row.get(5)?,
        author_id: row.get(6)?,
        author_name: row.get(7)?,
        is_featured: row.get::<_, i64>(8)? != 0,
        created_at: row.get(9)?,
        published_at: row.get(10)?,
        has_thumbnail: row.get::<_, i64>(11)? != 0,
        runtime_build_id: row.get(12)?,
        runtime: row.get(13)?,
        runtime_name: row.get(14)?,
        runtime_version: row.get(15)?,
        source_extension: row.get(16)?,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::{
        extract::{Path, State},
        http::StatusCode,
    };
    use rusqlite::Connection;

    use backend::game_runtime::version::SGLEAM_BUILD_ID;

    use super::get_game;

    fn test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::db::initialize_schema(&conn).unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[tokio::test]
    async fn public_game_details_use_the_latest_approval_without_private_review_data() {
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
                     source_code, status, min_players, max_players, review_message, reviewed_at)
                VALUES
                    (1, 1, ?1, 'Jogo', 'Descrição', 'segredo antigo', 'approved', 1, 2, 'nota antiga', '2026-01-01 00:00:00'),
                    (1, 2, ?1, 'Jogo', 'Descrição', 'segredo atual', 'approved', 2, 4, 'nota atual', '2026-02-01 00:00:00'),
                    (1, 3, ?1, 'Jogo futuro', 'Descrição futura', 'ainda em revisão', 'pending', 8, 8, NULL, NULL)
                ",
                [SGLEAM_BUILD_ID],
            )
            .unwrap();
        }

        let response = get_game(State(db), Path(1)).await.unwrap();
        let json = serde_json::to_value(response.0).unwrap();

        assert_eq!(json["revision_id"], 2);
        assert_eq!(json["min_players"], 2);
        assert_eq!(json["max_players"], 4);
        assert!(json.get("source_code").is_none());
        assert!(json.get("review_message").is_none());
    }

    #[tokio::test]
    async fn public_game_details_hide_an_unavailable_runtime_build() {
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
                    (1, 1, 'spython-dev', 'Jogo', 'Descrição', 'fonte', 'approved', 1, 1, '2026-01-01 00:00:00')
                ",
                [],
            )
            .unwrap();
        }

        let result = get_game(State(db), Path(1)).await;

        assert!(matches!(result, Err((StatusCode::NOT_FOUND, _))));
    }
}
