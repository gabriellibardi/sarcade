use std::{collections::HashSet, time::Duration};

use axum::http::StatusCode;
use axum_extra::extract::cookie::CookieJar;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{RngCore, rngs::OsRng};
use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};

use crate::{config::GoogleOAuthConfig, db::Db, models::user::CurrentUser};

pub const SESSION_COOKIE: &str = "sarcade_session";
pub const OAUTH_STATE_COOKIE: &str = "sarcade_oauth_state";

#[derive(Clone)]
pub struct AuthService {
    google: Option<GoogleOAuthConfig>,
    http: reqwest::Client,
    admin_emails: HashSet<String>,
}

impl AuthService {
    pub fn new(google: Option<GoogleOAuthConfig>, admin_emails: Vec<String>) -> Self {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .expect("falha ao configurar cliente HTTP do OAuth");

        if let Some(config) = &google {
            tracing::info!(redirect_uri = %config.redirect_uri, "login com Google habilitado");
        } else {
            tracing::info!("login com Google desabilitado");
        }

        let admin_emails = admin_emails.into_iter().collect::<HashSet<_>>();
        tracing::info!(
            count = admin_emails.len(),
            "lista de promoção de administradores carregada"
        );

        Self {
            google,
            http,
            admin_emails,
        }
    }

    pub fn google(&self) -> Option<&GoogleOAuthConfig> {
        self.google.as_ref()
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn google_enabled(&self) -> bool {
        self.google.is_some()
    }

    pub fn should_promote_admin(&self, email: &str) -> bool {
        self.admin_emails.contains(&email.to_ascii_lowercase())
    }
}

pub fn current_user(db: &Db, jar: &CookieJar) -> Result<Option<CurrentUser>, (StatusCode, String)> {
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return Ok(None);
    };

    let session_id = hash_token(cookie.value());
    let conn = db
        .lock()
        .map_err(|_| internal_error("Falha ao acessar o banco de dados"))?;

    conn.query_row(
        "
        SELECT users.id, users.email, users.name, users.avatar, users.is_admin
        FROM sessions
        JOIN users ON users.id = sessions.user_id
        WHERE sessions.id = ?1
          AND sessions.expires_at > datetime('now')
        ",
        [session_id],
        |row| {
            Ok(CurrentUser {
                id: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
                avatar: row.get(3)?,
                is_admin: row.get::<_, i64>(4)? != 0,
            })
        },
    )
    .optional()
    .map_err(|error| {
        tracing::error!(%error, "falha ao consultar sessão");
        internal_error("Falha ao consultar a sessão")
    })
}

pub fn sync_admin_roles(db: &Db, admin_emails: &[String]) -> Result<(), String> {
    let mut conn = db
        .lock()
        .map_err(|_| "Falha ao acessar o banco para sincronizar administradores".to_string())?;
    let transaction = conn.transaction().map_err(|error| error.to_string())?;

    transaction
        .execute("UPDATE users SET is_admin = 0", [])
        .map_err(|error| error.to_string())?;

    let mut promoted = 0;
    for email in admin_emails {
        promoted += transaction
            .execute(
                "UPDATE users SET is_admin = 1 WHERE lower(email) = lower(?1)",
                [email],
            )
            .map_err(|error| error.to_string())?;
    }

    transaction.commit().map_err(|error| error.to_string())?;
    tracing::info!(promoted, "papéis administrativos sincronizados");
    Ok(())
}

pub fn require_admin(db: &Db, jar: &CookieJar) -> Result<CurrentUser, (StatusCode, String)> {
    let user = current_user(db, jar)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Faça login para acessar a área de revisão".to_string(),
    ))?;

    if !user.is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            "Apenas administradores podem revisar jogos".to_string(),
        ));
    }

    Ok(user)
}

pub fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub fn hash_token(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}

fn internal_error(message: &str) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_url_safe_and_have_enough_entropy() {
        let first = random_token();
        let second = random_token();

        assert_eq!(first.len(), 43);
        assert_ne!(first, second);
        assert!(first.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        }));
    }

    #[test]
    fn recognizes_admin_email_without_case_sensitivity() {
        let auth = AuthService::new(None, vec!["admin@example.com".to_string()]);

        assert!(auth.should_promote_admin("ADMIN@example.com"));
        assert!(!auth.should_promote_admin("user@example.com"));
    }

    #[test]
    fn synchronizes_admin_roles_from_configured_emails() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL,
                is_admin INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO users (email, is_admin) VALUES
                ('admin@example.com', 0),
                ('old-admin@example.com', 1);
            ",
        )
        .unwrap();
        let db = std::sync::Arc::new(std::sync::Mutex::new(conn));

        sync_admin_roles(&db, &["admin@example.com".to_string()]).unwrap();

        let conn = db.lock().unwrap();
        let admin: i64 = conn
            .query_row(
                "SELECT is_admin FROM users WHERE email = 'admin@example.com'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let old_admin: i64 = conn
            .query_row(
                "SELECT is_admin FROM users WHERE email = 'old-admin@example.com'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(admin, 1);
        assert_eq!(old_admin, 0);
    }

    #[test]
    fn pkce_challenge_matches_rfc_7636_example() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            pkce_challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
}
