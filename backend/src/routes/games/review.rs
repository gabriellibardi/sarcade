use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{auth::require_admin, db::Db};

#[derive(Deserialize)]
pub struct RejectRequest {
    pub message: String,
}

pub async fn approve_revision(
    State(db): State<Db>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&db, &jar)?;

    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let affected = conn
        .execute(
            "
            UPDATE game_revisions
            SET
                status = 'approved',
                review_message = NULL,
                updated_at = datetime('now'),
                reviewed_at = datetime('now')
            WHERE id = ?1
              AND status = 'pending'
            ",
            [id],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if affected == 0 {
        return Err((
            StatusCode::CONFLICT,
            "Revisão não encontrada ou já analisada".into(),
        ));
    }

    Ok(Json(json!({
        "revision_id": id,
        "status": "approved",
        "message": "Revisão aprovada com sucesso"
    })))
}

pub async fn reject_revision(
    State(db): State<Db>,
    Path(id): Path<i64>,
    jar: CookieJar,
    Json(payload): Json<RejectRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&db, &jar)?;

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
            UPDATE game_revisions
            SET
                status = 'rejected',
                review_message = ?2,
                updated_at = datetime('now'),
                reviewed_at = datetime('now')
            WHERE id = ?1
              AND status = 'pending'
            ",
            rusqlite::params![id, message],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if affected == 0 {
        return Err((
            StatusCode::CONFLICT,
            "Revisão não encontrada ou já analisada".into(),
        ));
    }

    Ok(Json(json!({
        "revision_id": id,
        "status": "rejected",
        "message": "Revisão rejeitada com sucesso"
    })))
}
