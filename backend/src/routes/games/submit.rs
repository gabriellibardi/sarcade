use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::db::Db;

const MAX_TITLE_LEN: usize = 100;
const MAX_DESCRIPTION_LEN: usize = 2_000;
const MAX_SOURCE_CODE_LEN: usize = 200_000;
const MAX_THUMBNAIL_SIZE: usize = 1_000_000;

pub async fn submit_game(
    State(db): State<Db>,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, String)> {
    let mut title = None;
    let mut description = None;
    let mut source_code = None;
    let mut thumbnail: Option<Vec<u8>> = None;
    let mut min_players = None;
    let mut max_players = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "title" => title = Some(field.text().await.unwrap_or_default()),
            "description" => description = Some(field.text().await.unwrap_or_default()),
            "source_code" => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

                if bytes.len() > MAX_SOURCE_CODE_LEN {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "Código-fonte muito grande".into(),
                    ));
                }

                let text = String::from_utf8(bytes.to_vec()).map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        "Arquivo .gleam deve estar em UTF-8 válido".to_string(),
                    )
                })?;

                source_code = Some(text);
            }
            "thumbnail" => {
                if let Ok(bytes) = field.bytes().await {
                    if !bytes.is_empty() {
                        thumbnail = Some(bytes.to_vec());
                    }
                }
            }
            "min_players" => {
                let value = field.text().await.unwrap_or_default();
                min_players = value.parse::<i64>().ok();
            }
            "max_players" => {
                let value = field.text().await.unwrap_or_default();
                max_players = value.parse::<i64>().ok();
            }
            _ => {}
        }
    }

    let (title, description, source_code) = match (title, description, source_code) {
        (Some(t), Some(d), Some(s)) => (t.trim().to_string(), d.trim().to_string(), s.trim().to_string()),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Campos obrigatórios (title, description, source_code) faltando".into(),
            ));
        }
    };

    if title.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Título não pode ser vazio".into()));
    }

    if description.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Descrição não pode ser vazia".into()));
    }

    if source_code.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Código-fonte não pode ser vazio".into()));
    }

    if title.len() > MAX_TITLE_LEN {
        return Err((StatusCode::BAD_REQUEST, "Título muito grande".into()));
    }

    if description.len() > MAX_DESCRIPTION_LEN {
        return Err((StatusCode::BAD_REQUEST, "Descrição muito grande".into()));
    }

    if source_code.len() > MAX_SOURCE_CODE_LEN {
        return Err((StatusCode::BAD_REQUEST, "Código-fonte muito grande".into()));
    }

    if let Some(thumbnail) = &thumbnail {
        if thumbnail.len() > MAX_THUMBNAIL_SIZE {
            return Err((StatusCode::BAD_REQUEST, "Thumbnail muito grande".into()));
        }
    }

    let min_players = min_players.unwrap_or(1);
    let max_players = max_players.unwrap_or(1);

    if min_players < 1 {
        return Err((StatusCode::BAD_REQUEST, "min_players deve ser pelo menos 1".into()));
    }

    if max_players < min_players {
        return Err((StatusCode::BAD_REQUEST, "max_players deve ser maior ou igual a min_players".into()));
    }

    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    conn.execute(
        "
        INSERT INTO games
            (title, description, source_code, thumbnail, status, min_players, max_players)
        VALUES
            (?1, ?2, ?3, ?4, 'pending', ?5, ?6)
        ",
        rusqlite::params![
            title,
            description,
            source_code,
            thumbnail,
            min_players,
            max_players
        ],
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let id = conn.last_insert_rowid();

    Ok(Json(json!({
        "id": id,
        "message": "Jogo submetido com sucesso",
        "status": "pending",
        "min_players": min_players,
        "max_players": max_players
    })))
}
