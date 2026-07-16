use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};

use crate::{assets, state::AppState};

mod games;
mod rooms;
mod runtimes;
mod ws;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/api/games",
            get(games::list_approved_games).post(games::submit_game),
        )
        .route("/api/games/pending", get(games::list_pending_games))
        .route("/api/games/{id}", get(games::get_game))
        .route("/api/games/{id}/thumbnail", get(games::get_thumbnail))
        .route("/api/revisions/{id}", get(games::get_revision))
        .route("/api/revisions/{id}/approve", post(games::approve_revision))
        .route("/api/revisions/{id}/reject", post(games::reject_revision))
        .route("/api/runtimes", get(runtimes::list_runtime_builds))
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
