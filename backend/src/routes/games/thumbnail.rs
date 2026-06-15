use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, Response, StatusCode},
};
use rusqlite::OptionalExtension;

use crate::db::Db;

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
            "SELECT thumbnail FROM games WHERE id = ?1",
            [id],
            |row| row.get::<_, Option<Vec<u8>>>(0),
        )
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    let Some(Some(bytes)) = thumbnail else {
        return Err((StatusCode::NOT_FOUND, "Thumbnail não encontrada".into()));
    };

    let content_type = detect_image_content_type(&bytes);

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static(content_type),
        )
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
