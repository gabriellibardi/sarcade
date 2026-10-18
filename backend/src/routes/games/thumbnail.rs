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
            JOIN runtime_builds ON runtime_builds.id = game_revisions.runtime_build_id
            WHERE games.id = ?1
              AND games.archived_at IS NULL
              AND game_revisions.status = 'approved'
              AND runtime_builds.status IN ('development', 'active')
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

    let content_type = thumbnail_content_type(&bytes)
        .ok_or((StatusCode::NOT_FOUND, "Thumbnail não encontrada".into()))?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_static(content_type))
        .header("x-content-type-options", "nosniff")
        .body(Body::from(bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub(crate) fn validate_thumbnail(bytes: &[u8]) -> Result<(), (StatusCode, String)> {
    thumbnail_content_type(bytes).map(|_| ()).ok_or((
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "A thumbnail deve ser PNG, JPEG ou WebP válido".into(),
    ))
}

fn thumbnail_content_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::{thumbnail_content_type, validate_thumbnail};

    #[test]
    fn accepts_only_supported_raster_signatures() {
        assert_eq!(
            thumbnail_content_type(b"\x89PNG\r\n\x1a\nrest"),
            Some("image/png")
        );
        assert_eq!(
            thumbnail_content_type(b"\xff\xd8\xffrest"),
            Some("image/jpeg")
        );
        assert_eq!(
            thumbnail_content_type(b"RIFF\x00\x00\x00\x00WEBPrest"),
            Some("image/webp")
        );
    }

    #[test]
    fn rejects_svg_and_arbitrary_bytes() {
        assert!(matches!(
            validate_thumbnail(b"<svg><script>alert(1)</script></svg>"),
            Err((StatusCode::UNSUPPORTED_MEDIA_TYPE, _))
        ));
        assert!(validate_thumbnail(b"not an image").is_err());
    }
}
