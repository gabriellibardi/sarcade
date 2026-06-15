use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use rusqlite::OptionalExtension;

use crate::{
    db::Db,
    models::game::{GameDetails, GameSummary},
};

pub async fn list_pending_games(
    State(db): State<Db>,
) -> Result<Json<Vec<GameSummary>>, (StatusCode, String)> {
    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let mut stmt = conn
        .prepare(
            "
            SELECT
                games.id,
                games.title,
                games.description,
                games.status,
                games.min_players,
                games.max_players,
                games.is_featured,
                games.created_at,
                games.thumbnail IS NOT NULL,
                users.id,
                users.name
            FROM games
            LEFT JOIN users ON users.id = games.author_id
            WHERE games.status = 'pending'
            ORDER BY games.created_at DESC
            ",
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let games = stmt
        .query_map([], |row| {
            Ok(GameSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                min_players: row.get(4)?,
                max_players: row.get(5)?,
                author_id: row.get(9)?,
                author_name: row.get(10)?,
                is_featured: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
                has_thumbnail: row.get::<_, i64>(8)? != 0,
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(games))
}

pub async fn get_game(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<GameDetails>, (StatusCode, String)> {
    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let game = conn
        .query_row(
            "
            SELECT
                games.id,
                games.title,
                games.description,
                games.source_code,
                games.status,
                games.min_players,
                games.max_players,
                games.is_featured,
                games.review_message,
                games.created_at,
                games.reviewed_at,
                games.thumbnail IS NOT NULL,
                users.id,
                users.name
            FROM games
            LEFT JOIN users ON users.id = games.author_id
            WHERE games.id = ?1
            ",
            [id],
            |row| {
                Ok(GameDetails {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    source_code: row.get(3)?,
                    status: row.get(4)?,
                    min_players: row.get(5)?,
                    max_players: row.get(6)?,
                    is_featured: row.get::<_, i64>(7)? != 0,
                    review_message: row.get(8)?,
                    created_at: row.get(9)?,
                    reviewed_at: row.get(10)?,
                    has_thumbnail: row.get::<_, i64>(11)? != 0,
                    author_id: row.get(12)?,
                    author_name: row.get(13)?,
                })
            },
        )
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match game {
        Some(game) => Ok(Json(game)),
        None => Err((StatusCode::NOT_FOUND, "Jogo não encontrado".into())),
    }
}
