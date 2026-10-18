mod assets;
mod auth;
mod config;
mod db;
mod models;
mod rooms;
mod routes;
mod state;

use backend::game_runtime::SgleamExecutable;
use backend::game_runtime::version::{SGLEAM_BUILD_ID, SGLEAM_GIT_COMMIT, SINTERACT_GIT_COMMIT};
use config::Config;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    if let Some("sinteract-test-engine") = args.next().as_deref() {
        if let Err(error) = backend::game_runtime::sinteract_process::run_test_engine_from_fds() {
            eprintln!("engine Sinteract de teste falhou: {error}");
            std::process::exit(1);
        }
        return;
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    tracing::info!(
        max_runtime_processes = config.max_runtime_processes,
        "limite compartilhado de processos de runtime configurado"
    );
    let sgleam_executable =
        SgleamExecutable::configured(config.sgleam_executable).unwrap_or_else(|error| {
            tracing::error!(%error, "falha ao preparar runtime Sgleam");
            std::process::exit(1);
        });
    if let Some(executable) = &sgleam_executable
        && let Err(error) = executable.verify_build_identity()
    {
        tracing::error!(%error, "build do runtime Sgleam incompatível");
        std::process::exit(1);
    }
    tracing::info!(
        runtime_build = SGLEAM_BUILD_ID,
        sgleam_commit = SGLEAM_GIT_COMMIT,
        sinteract_commit = SINTERACT_GIT_COMMIT,
        "build de runtime verificado"
    );
    let db = db::init(&config.database_url);
    auth::sync_admin_roles(&db, &config.admin_emails)
        .expect("falha ao sincronizar administradores");
    let auth = auth::AuthService::new(config.google_oauth, config.admin_emails);
    let state = state::AppState::new(db, auth, sgleam_executable, config.max_runtime_processes);
    let app = routes::router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("servidor rodando em http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
