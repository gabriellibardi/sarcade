use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use crate::db::Db;

pub async fn submit_game(
    State(db): State<Db>,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, String)> {
    let mut title = None;
    let mut description = None;
    let mut source_code = None;
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
            "source_code" => source_code = Some(field.text().await.unwrap_or_default()),
            "thumbnail" => {
                if let Ok(bytes) = field.bytes().await {
                    if !bytes.is_empty() {
                        thumbnail = Some(bytes.to_vec());
                    }
                }
            }
            _ => {}
        }
    }

    let (title, description, source_code) = match (title, description, source_code) {
        (Some(t), Some(d), Some(s)) => (t, d, s),
        _ => return Err((StatusCode::BAD_REQUEST, "Campos obrigatórios (title, description, source_code) faltando".into())),
    };

    let conn = db.lock().unwrap();
    let result = conn.execute(
        "INSERT INTO games (title, description, source_code, thumbnail, status) 
         VALUES (?1, ?2, ?3, ?4, 'pending')",
        rusqlite::params![title, description, source_code, thumbnail],
    );

    match result {
        Ok(_) => {
            let id = conn.last_insert_rowid();
            Ok(Json(json!({ 
                "id": id, 
                "message": "Jogo submetido com sucesso", 
                "status": "pending" 
            })))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
