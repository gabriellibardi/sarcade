use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::db::Db;

#[derive(Deserialize)]
pub struct RejectRequest {
    pub message: String,
}

pub async fn approve_game(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let affected = conn
        .execute(
            "
            UPDATE games
            SET
                status = 'approved',
                review_message = NULL,
                reviewed_at = datetime('now')
            WHERE id = ?1
            ",
            [id],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if affected == 0 {
        return Err((StatusCode::NOT_FOUND, "Jogo não encontrado".into()));
    }

    Ok(Json(json!({
        "id": id,
        "status": "approved",
        "message": "Jogo aprovado com sucesso"
    })))
}

pub async fn reject_game(
    State(db): State<Db>,
    Path(id): Path<i64>,
    Json(payload): Json<RejectRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let message = payload.message.trim().to_string();

    if message.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Mensagem de rejeição é obrigatória".into(),
        ));
    }

    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let affected = conn
        .execute(
            "
            UPDATE games
            SET
                status = 'rejected',
                review_message = ?2,
                reviewed_at = datetime('now')
            WHERE id = ?1
            ",
            rusqlite::params![id, message],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if affected == 0 {
        return Err((StatusCode::NOT_FOUND, "Jogo não encontrado".into()));
    }

    Ok(Json(json!({
        "id": id,
        "status": "rejected",
        "message": "Jogo rejeitado com sucesso"
    })))
}
