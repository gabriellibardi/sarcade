use axum::{Router, routing::get};
use crate::db::Db;

pub fn router(db: Db) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(db)
}

async fn health() -> &'static str {
    "ok"
}
