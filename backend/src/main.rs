mod assets;
mod auth;
mod config;
mod db;
mod models;
mod rooms;
mod routes;
mod state;

use backend::game_runtime::SgleamExecutable;
use config::Config;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("runtime-stub") => {
            if let Err(error) =
                backend::game_runtime::process::run_stub_from_args(args.next()).await
            {
                eprintln!("runtime reserva falhou: {error}");
                std::process::exit(1);
            }
            return;
        }
        Some("sinteract-runtime-stub") => {
            if let Err(error) = backend::game_runtime::sinteract_process::run_stub_from_fds() {
                eprintln!("runtime Sinteract de teste falhou: {error}");
                std::process::exit(1);
            }
            return;
        }
        _ => {}
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let sgleam_executable =
        SgleamExecutable::configured(config.sgleam_executable).unwrap_or_else(|error| {
            tracing::error!(%error, "falha ao preparar runtime Sgleam");
            std::process::exit(1);
        });
    let db = db::init(&config.database_url);
    auth::sync_admin_roles(&db, &config.admin_emails)
        .expect("falha ao sincronizar administradores");
    let auth = auth::AuthService::new(config.google_oauth, config.admin_emails);
    let state = state::AppState::new(db, auth, sgleam_executable);
    let app = routes::router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("servidor rodando em http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
