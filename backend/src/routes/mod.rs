use axum::{
    Router,
    extract::{DefaultBodyLimit, Request},
    http::{HeaderMap, HeaderName, HeaderValue},
    middleware::{self, Next},
    response::Response,
    routing::{get, patch, post},
};

use crate::{assets, state::AppState};

mod auth;
mod games;
mod rooms;
mod runtimes;
mod users;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/auth/start", get(auth::google_start))
        .route("/api/auth/callback", get(auth::google_callback))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/logout", post(auth::logout))
        .route(
            "/api/games",
            get(games::list_approved_games).post(games::submit_game),
        )
        .route("/api/games/inspect", post(games::inspect_source))
        .route("/api/games/pending", get(games::list_pending_games))
        .route("/api/me/games", get(games::list_my_games))
        .route("/api/games/{id}", get(games::get_game))
        .route(
            "/api/games/{id}",
            axum::routing::delete(games::archive_game),
        )
        .route("/api/games/{id}/thumbnail", get(games::get_thumbnail))
        .route("/api/revisions/{id}", get(games::get_revision))
        .route("/api/revisions/{id}", patch(games::update_draft))
        .route(
            "/api/revisions/{id}/edit",
            post(games::create_editable_revision),
        )
        .route("/api/revisions/{id}/submit", post(games::submit_draft))
        .route(
            "/api/revisions/{id}/thumbnail",
            get(games::get_revision_thumbnail),
        )
        .route("/api/revisions/{id}/approve", post(games::approve_revision))
        .route("/api/revisions/{id}/reject", post(games::reject_revision))
        .route("/api/runtimes", get(runtimes::list_runtime_builds))
        .route("/api/users/{id}", get(users::get_user_profile))
        .route("/api/rooms", post(rooms::create_room))
        .route(
            "/api/revisions/{id}/rooms",
            post(rooms::create_revision_room),
        )
        .route("/api/rooms/{code}", get(rooms::get_room))
        .route("/api/rooms/{code}/ws", get(rooms::room_ws))
        .fallback(assets::serve)
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    add_security_headers(response.headers_mut());
    response
}

fn add_security_headers(headers: &mut HeaderMap) {
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;

    use super::add_security_headers;

    #[test]
    fn adds_defensive_headers_to_responses() {
        let mut headers = HeaderMap::new();

        add_security_headers(&mut headers);

        assert_eq!(headers["x-content-type-options"], "nosniff");
        assert_eq!(headers["x-frame-options"], "DENY");
        assert_eq!(headers["referrer-policy"], "same-origin");
        assert_eq!(
            headers["permissions-policy"],
            "camera=(), microphone=(), geolocation=()"
        );
    }
}
