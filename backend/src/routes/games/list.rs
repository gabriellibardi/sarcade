use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use rusqlite::{OptionalExtension, Row};

use crate::{
    db::Db,
    models::game::{GameDetails, GameSummary},
};

const SUMMARY_COLUMNS: &str = "
    games.id,
    game_revisions.id,
    games.title,
    games.description,
    game_revisions.status,
    game_revisions.min_players,
    game_revisions.max_players,
    users.id,
    users.name,
    games.is_featured,
    game_revisions.created_at,
    games.thumbnail IS NOT NULL,
    runtime_builds.id,
    runtime_builds.runtime,
    runtime_builds.display_name,
    runtime_builds.display_version,
    runtime_builds.source_extension
";

pub async fn list_pending_games(
    State(db): State<Db>,
) -> Result<Json<Vec<GameSummary>>, (StatusCode, String)> {
    let conn = lock_db(&db)?;
    let sql = format!(
        "
        SELECT {SUMMARY_COLUMNS}
        FROM game_revisions
        JOIN games ON games.id = game_revisions.game_id
        JOIN runtime_builds ON runtime_builds.id = game_revisions.runtime_build_id
        LEFT JOIN users ON users.id = games.author_id
        WHERE game_revisions.status = 'pending'
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
          AND game_revisions.id = (
              SELECT latest.id
              FROM game_revisions AS latest
              WHERE latest.game_id = games.id
                AND latest.status = 'approved'
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
) -> Result<Json<GameDetails>, (StatusCode, String)> {
    let conn = lock_db(&db)?;

    let game = conn
        .query_row(
            "
            SELECT
                games.id,
                game_revisions.id,
                games.title,
                games.description,
                game_revisions.source_code,
                game_revisions.status,
                game_revisions.min_players,
                game_revisions.max_players,
                users.id,
                users.name,
                games.is_featured,
                game_revisions.review_message,
                game_revisions.created_at,
                game_revisions.reviewed_at,
                games.thumbnail IS NOT NULL,
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
              AND game_revisions.status = 'approved'
            ORDER BY COALESCE(game_revisions.reviewed_at, game_revisions.created_at) DESC,
                     game_revisions.id DESC
            LIMIT 1
            ",
            [id],
            game_details_from_row,
        )
        .optional()
        .map_err(internal_error)?;

    match game {
        Some(game) => Ok(Json(game)),
        None => Err((StatusCode::NOT_FOUND, "Jogo aprovado não encontrado".into())),
    }
}

pub async fn get_revision(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<GameDetails>, (StatusCode, String)> {
    let conn = lock_db(&db)?;

    let revision = conn
        .query_row(
            "
            SELECT
                games.id,
                game_revisions.id,
                games.title,
                games.description,
                game_revisions.source_code,
                game_revisions.status,
                game_revisions.min_players,
                game_revisions.max_players,
                users.id,
                users.name,
                games.is_featured,
                game_revisions.review_message,
                game_revisions.created_at,
                game_revisions.reviewed_at,
                games.thumbnail IS NOT NULL,
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
            ",
            [id],
            game_details_from_row,
        )
        .optional()
        .map_err(internal_error)?;

    match revision {
        Some(revision) => Ok(Json(revision)),
        None => Err((StatusCode::NOT_FOUND, "Revisão não encontrada".into())),
    }
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
        title: row.get(2)?,
        description: row.get(3)?,
        source_code: row.get(4)?,
        status: row.get(5)?,
        min_players: row.get(6)?,
        max_players: row.get(7)?,
        author_id: row.get(8)?,
        author_name: row.get(9)?,
        is_featured: row.get::<_, i64>(10)? != 0,
        review_message: row.get(11)?,
        created_at: row.get(12)?,
        reviewed_at: row.get(13)?,
        has_thumbnail: row.get::<_, i64>(14)? != 0,
        runtime_build_id: row.get(15)?,
        runtime: row.get(16)?,
        runtime_name: row.get(17)?,
        runtime_version: row.get(18)?,
        source_extension: row.get(19)?,
    })
}
