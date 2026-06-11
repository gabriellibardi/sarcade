use axum::{
    Router, 
    routing::{get, post}
};
use crate::{assets, db::Db};

mod games;
mod ws;

pub fn router(db: Db) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/games", post(games::submit_game))
        .route("/api/echo", get(ws::echo_handler))
        
        .fallback(assets::serve)
        .with_state(db)
}

async fn health() -> &'static str {
    "ok"
}
