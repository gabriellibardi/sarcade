use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};

use crate::{
    assets,
    state::AppState,
};

mod games;
mod rooms;
mod ws;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))

        .route("/api/games", post(games::submit_game))
        .route("/api/games/pending", get(games::list_pending_games))
        .route("/api/games/{id}", get(games::get_game))
        .route("/api/games/{id}/thumbnail", get(games::get_thumbnail))
        .route("/api/games/{id}/approve", post(games::approve_game))
        .route("/api/games/{id}/reject", post(games::reject_game))

        .route("/api/rooms", post(rooms::create_room))
        .route("/api/rooms/{code}", get(rooms::get_room))
        .route("/api/rooms/{code}/ws", get(rooms::room_ws))

        .route("/api/echo", get(ws::echo_handler))
        .fallback(assets::serve)
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
