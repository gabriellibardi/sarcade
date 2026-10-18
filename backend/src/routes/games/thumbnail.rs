use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, Response, StatusCode, header},
};
use axum_extra::extract::cookie::CookieJar;
use rusqlite::OptionalExtension;

use crate::{auth::current_user, db::Db};

pub async fn get_thumbnail(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let thumbnail = {
        let conn = db.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Falha ao acessar o banco de dados".to_string(),
            )
        })?;

        conn.query_row(
            "
            SELECT game_revisions.thumbnail
            FROM games
            JOIN game_revisions ON game_revisions.game_id = games.id
            WHERE games.id = ?1
              AND games.archived_at IS NULL
              AND game_revisions.status = 'approved'
            ORDER BY COALESCE(game_revisions.reviewed_at, game_revisions.created_at) DESC,
                     game_revisions.id DESC
            LIMIT 1
            ",
            [id],
            |row| row.get::<_, Option<Vec<u8>>>(0),
        )
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    thumbnail_response(thumbnail)
}

pub async fn get_revision_thumbnail(
    State(db): State<Db>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Result<Response<Body>, (StatusCode, String)> {
    let user = current_user(&db, &jar)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Faça login para consultar uma revisão".to_string(),
    ))?;
    let thumbnail = {
        let conn = db.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Falha ao acessar o banco de dados".to_string(),
            )
        })?;
        conn.query_row(
            "
            SELECT game_revisions.thumbnail
            FROM game_revisions
            JOIN games ON games.id = game_revisions.game_id
            WHERE game_revisions.id = ?1
              AND (games.author_id = ?2 OR ?3 = 1)
            ",
            rusqlite::params![id, user.id, i64::from(user.is_admin)],
            |row| row.get::<_, Option<Vec<u8>>>(0),
        )
        .optional()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
    };

    thumbnail_response(thumbnail)
}

fn thumbnail_response(
    thumbnail: Option<Option<Vec<u8>>>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let Some(Some(bytes)) = thumbnail else {
        return Err((StatusCode::NOT_FOUND, "Thumbnail não encontrada".into()));
    };

    let content_type = detect_image_content_type(&bytes);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_static(content_type))
        .body(Body::from(bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

fn detect_image_content_type(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else if bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}
