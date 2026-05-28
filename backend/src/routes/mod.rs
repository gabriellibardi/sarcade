use axum::{Router, routing::get};
use crate::{assets, db::Db};

pub fn router(db: Db) -> Router {
    Router::new()
        .route("/health", get(health))
        .fallback(assets::serve)
        .with_state(db)
}

async fn health() -> &'static str {
    "ok"
}
