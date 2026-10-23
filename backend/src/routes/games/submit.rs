use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use serde_json::{Value, json};

use crate::{auth::current_user, state::AppState};

use super::inspect::inspect_source_code;
use super::thumbnail::validate_thumbnail;

const MAX_TITLE_LEN: usize = 100;
const MAX_DESCRIPTION_LEN: usize = 2_000;
pub(crate) const MAX_SOURCE_CODE_LEN: usize = 200_000;
pub(crate) const MAX_THUMBNAIL_SIZE: usize = 1_000_000;

pub async fn submit_game(
    State(state): State<AppState>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, String)> {
    let user = current_user(&state.db, &jar)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Faça login para publicar um jogo".to_string(),
    ))?;

    let mut title = None;
    let mut description = None;
    let mut source_code = None;
    let mut uploaded_filename = None;
    let mut runtime_build_id = None;
    let mut thumbnail: Option<Vec<u8>> = None;

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
    if let Some(thumbnail) = &thumbnail {
        validate_thumbnail(thumbnail)?;
    }

    let runtime_build_id = runtime_build_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Selecione uma linguagem e uma versão".to_string(),
        ))?;

    let inspected = inspect_source_code(
        &state,
        &runtime_build_id,
        &source_code,
        uploaded_filename.as_deref(),
    )
    .await?;
    let min_players = inspected.min_players;
    let max_players = inspected.max_players;

    let mut conn = state.db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let transaction = conn
        .transaction()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    transaction
        .execute(
            "
            INSERT INTO games (title, description, thumbnail, author_id)
            VALUES (?1, ?2, ?3, ?4)
            ",
            rusqlite::params![title, description, thumbnail, user.id],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let game_id = transaction.last_insert_rowid();

    transaction
        .execute(
            "
            INSERT INTO game_revisions
                (game_id, revision_number, created_by_user_id, runtime_build_id,
                 title, description, thumbnail, source_code, status,
                 min_players, max_players)
            VALUES
                (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7, 'draft', ?8, ?9)
            ",
            rusqlite::params![
                game_id,
                user.id,
                runtime_build_id,
                title,
                description,
                thumbnail,
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
        "message": "Rascunho criado com sucesso",
        "status": "draft",
        "runtime": inspected.runtime,
        "runtime_build_id": runtime_build_id,
        "min_players": min_players,
        "max_players": max_players
    })))
}

pub(crate) fn validate_text_fields(
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
