use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};

use crate::{assets, db::Db};

mod games;
mod ws;

pub fn router(db: Db) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/games", post(games::submit_game))
        .route("/api/games/pending", get(games::list_pending_games))
        .route("/api/games/{id}", get(games::get_game))
        .route("/api/games/{id}/thumbnail", get(games::get_thumbnail))
        .route("/api/games/{id}/approve", post(games::approve_game))
        .route("/api/games/{id}/reject", post(games::reject_game))
        .route("/api/echo", get(ws::echo_handler))
        .fallback(assets::serve)
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(db)
}

async fn health() -> &'static str {
    "ok"
}
