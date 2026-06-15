use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use crate::{
    db::Db,
    models::game::GameSummary,
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
                id,
                title,
                description,
                status,
                min_players,
                max_players,
                is_featured,
                created_at,
                thumbnail IS NOT NULL
            FROM games
            WHERE status = 'pending'
            ORDER BY created_at DESC
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
