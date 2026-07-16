use std::path::Path;

use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};

use crate::db::Db;

const MAX_TITLE_LEN: usize = 100;
const MAX_DESCRIPTION_LEN: usize = 2_000;
const MAX_SOURCE_CODE_LEN: usize = 200_000;
const MAX_THUMBNAIL_SIZE: usize = 1_000_000;
const DEFAULT_RUNTIME_BUILD: &str = "sgleam-dev";

pub async fn submit_game(
    State(db): State<Db>,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, String)> {
    let mut title = None;
    let mut description = None;
    let mut source_code = None;
    let mut uploaded_filename = None;
    let mut runtime_build_id = None;
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
            "runtime_build_id" => runtime_build_id = Some(field.text().await.unwrap_or_default()),
            "source_code" => {
                uploaded_filename = field.file_name().map(str::to_string);

                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

                if bytes.len() > MAX_SOURCE_CODE_LEN {
                    return Err((StatusCode::BAD_REQUEST, "Código-fonte muito grande".into()));
                }

                let text = String::from_utf8(bytes.to_vec()).map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        "O código-fonte deve estar em UTF-8 válido".to_string(),
                    )
                })?;

                source_code = Some(text);
            }
            "thumbnail" => {
                if let Ok(bytes) = field.bytes().await
                    && !bytes.is_empty()
                {
                    thumbnail = Some(bytes.to_vec());
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
        (Some(title), Some(description), Some(source_code)) => (
            title.trim().to_string(),
            description.trim().to_string(),
            source_code.trim().to_string(),
        ),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Campos obrigatórios (title, description, source_code) faltando".into(),
            ));
        }
    };

    validate_text_fields(&title, &description, &source_code)?;

    if let Some(thumbnail) = &thumbnail
        && thumbnail.len() > MAX_THUMBNAIL_SIZE
    {
        return Err((StatusCode::BAD_REQUEST, "Thumbnail muito grande".into()));
    }

    let min_players = min_players.unwrap_or(1);
    let max_players = max_players.unwrap_or(1);

    if min_players < 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            "min_players deve ser pelo menos 1".into(),
        ));
    }

    if max_players < min_players {
        return Err((
            StatusCode::BAD_REQUEST,
            "max_players deve ser maior ou igual a min_players".into(),
        ));
    }

    let runtime_build_id = runtime_build_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_RUNTIME_BUILD.to_string());

    let mut conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let runtime = conn
        .query_row(
            "
            SELECT runtime, source_extension
            FROM runtime_builds
            WHERE id = ?1
              AND status IN ('development', 'active')
            ",
            [&runtime_build_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Build de runtime inválido ou indisponível".to_string(),
        ))?;

    if !has_expected_extension(uploaded_filename.as_deref(), &runtime.1) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("O arquivo selecionado deve usar a extensão {}", runtime.1),
        ));
    }

    let transaction = conn
        .transaction()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    transaction
        .execute(
            "
            INSERT INTO games (title, description, thumbnail)
            VALUES (?1, ?2, ?3)
            ",
            rusqlite::params![title, description, thumbnail],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let game_id = transaction.last_insert_rowid();

    transaction
        .execute(
            "
            INSERT INTO game_revisions
                (game_id, runtime_build_id, source_code, status, min_players, max_players)
            VALUES
                (?1, ?2, ?3, 'pending', ?4, ?5)
            ",
            rusqlite::params![
                game_id,
                runtime_build_id,
                source_code,
                min_players,
                max_players
            ],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let revision_id = transaction.last_insert_rowid();

    transaction
        .commit()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "id": game_id,
        "revision_id": revision_id,
        "message": "Jogo submetido com sucesso",
        "status": "pending",
        "runtime": runtime.0,
        "runtime_build_id": runtime_build_id,
        "min_players": min_players,
        "max_players": max_players
    })))
}

fn validate_text_fields(
    title: &str,
    description: &str,
    source_code: &str,
) -> Result<(), (StatusCode, String)> {
    if title.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Título não pode ser vazio".into()));
    }

    if description.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Descrição não pode ser vazia".into(),
        ));
    }

    if source_code.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Código-fonte não pode ser vazio".into(),
        ));
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

    Ok(())
}

fn has_expected_extension(filename: Option<&str>, expected_extension: &str) -> bool {
    let Some(filename) = filename else {
        return false;
    };
    let Some(actual_extension) = Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
    else {
        return false;
    };
    let expected_extension = expected_extension.trim_start_matches('.');

    actual_extension.eq_ignore_ascii_case(expected_extension)
}

#[cfg(test)]
mod tests {
    use super::has_expected_extension;

    #[test]
    fn accepts_expected_source_extension() {
        assert!(has_expected_extension(Some("pong.gleam"), ".gleam"));
        assert!(has_expected_extension(Some("snake.PY"), ".py"));
    }

    #[test]
    fn rejects_missing_or_wrong_source_extension() {
        assert!(!has_expected_extension(Some("pong.py"), ".gleam"));
        assert!(!has_expected_extension(None, ".gleam"));
    }
}
