mod assets;
mod auth;
mod config;
mod db;
mod models;
mod rooms;
mod routes;
mod state;

use config::Config;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let db = db::init(&config.database_url);
    auth::sync_admin_roles(&db, &config.admin_emails)
        .expect("falha ao sincronizar administradores");
    let auth = auth::AuthService::new(config.google_oauth, config.admin_emails);
    let state = state::AppState::new(db, auth);
    let app = routes::router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("servidor rodando em http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
