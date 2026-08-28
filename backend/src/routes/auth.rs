use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::Redirect,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use time::Duration;
use url::Url;

use crate::{
    auth::{
        AuthService, OAUTH_STATE_COOKIE, SESSION_COOKIE, current_user, hash_token, pkce_challenge,
        random_token,
    },
    config::GoogleOAuthConfig,
    db::Db,
    models::user::{AuthStatus, CurrentUser},
};

const GOOGLE_AUTHORIZATION_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_ENDPOINT: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const OAUTH_STATE_LIFETIME_MINUTES: i64 = 10;
const SESSION_LIFETIME_DAYS: i64 = 30;

type AuthError = (StatusCode, String);

#[derive(Deserialize)]
pub struct StartQuery {
    return_to: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GoogleProfile {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    picture: Option<String>,
}

pub async fn google_start(
    State(db): State<Db>,
    State(auth): State<AuthService>,
    Query(query): Query<StartQuery>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AuthError> {
    let google = configured_google(&auth)?;
    let state = random_token();
    let code_verifier = random_token();
    let code_challenge = pkce_challenge(&code_verifier);
    let return_to = safe_return_to(query.return_to.as_deref());

    {
        let conn = lock_db(&db)?;
        conn.execute(
            "DELETE FROM oauth_states WHERE expires_at <= datetime('now')",
            [],
        )
        .map_err(database_error)?;
        conn.execute(
            "
            INSERT INTO oauth_states (state, code_verifier, return_to, expires_at)
            VALUES (?1, ?2, ?3, datetime('now', '+10 minutes'))
            ",
            rusqlite::params![state, code_verifier, return_to],
        )
        .map_err(database_error)?;
    }

    let mut authorization_url =
        Url::parse(GOOGLE_AUTHORIZATION_ENDPOINT).expect("endpoint OAuth Google inválido");
    authorization_url
        .query_pairs_mut()
        .append_pair("client_id", &google.client_id)
        .append_pair("redirect_uri", &google.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email profile")
        .append_pair("state", &state)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256");

    let jar = jar.add(build_oauth_cookie(state, google.cookie_secure));
    Ok((jar, Redirect::to(authorization_url.as_str())))
}

pub async fn google_callback(
    State(db): State<Db>,
    State(auth): State<AuthService>,
    Query(query): Query<CallbackQuery>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AuthError> {
    let google = configured_google(&auth)?;

    if query.error.is_some() {
        let jar = jar.remove(clear_oauth_cookie(google.cookie_secure));
        return Ok((jar, Redirect::to("/?auth_error=google")));
    }

    let state = query
        .state
        .as_deref()
        .ok_or((StatusCode::BAD_REQUEST, "Estado OAuth ausente".into()))?;
    let code = query
        .code
        .as_deref()
        .ok_or((StatusCode::BAD_REQUEST, "Código OAuth ausente".into()))?;
    let cookie_state = jar
        .get(OAUTH_STATE_COOKIE)
        .map(Cookie::value)
        .ok_or((StatusCode::UNAUTHORIZED, "Sessão OAuth ausente".into()))?;

    if state != cookie_state {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Estado OAuth inválido ou expirado".into(),
        ));
    }

    let (code_verifier, return_to) = take_oauth_state(&db, state)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Estado OAuth inválido ou expirado".into(),
    ))?;

    let access_token = exchange_authorization_code(&auth, google, code, &code_verifier).await?;
    let profile = fetch_google_profile(&auth, &access_token).await?;
    let user = upsert_google_user(&db, profile, &auth)?;
    let session_token = create_session(&db, user.id)?;

    let jar = jar
        .remove(clear_oauth_cookie(google.cookie_secure))
        .add(build_session_cookie(session_token, google.cookie_secure));

    Ok((jar, Redirect::to(&return_to)))
}

pub async fn me(
    State(db): State<Db>,
    State(auth): State<AuthService>,
    jar: CookieJar,
) -> Result<Json<AuthStatus>, AuthError> {
    Ok(Json(AuthStatus {
        user: current_user(&db, &jar)?,
        google_oauth_enabled: auth.google_enabled(),
    }))
}

pub async fn logout(
    State(db): State<Db>,
    State(auth): State<AuthService>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), AuthError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        let conn = lock_db(&db)?;
        conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            [hash_token(cookie.value())],
        )
        .map_err(database_error)?;
    }

    let secure = auth
        .google()
        .map(|config| config.cookie_secure)
        .unwrap_or(false);
    let jar = jar.remove(clear_session_cookie(secure));

    Ok((jar, StatusCode::NO_CONTENT))
}

async fn exchange_authorization_code(
    auth: &AuthService,
    google: &GoogleOAuthConfig,
    code: &str,
    code_verifier: &str,
) -> Result<String, AuthError> {
    let response = auth
        .http()
        .post(GOOGLE_TOKEN_ENDPOINT)
        .form(&[
            ("client_id", google.client_id.as_str()),
            ("client_secret", google.client_secret.as_str()),
            ("code", code),
            ("code_verifier", code_verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", google.redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(%error, "falha na troca do código OAuth");
            oauth_upstream_error()
        })?;

    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "Google rejeitou a troca do código OAuth");
        return Err(oauth_upstream_error());
    }

    response
        .json::<GoogleTokenResponse>()
        .await
        .map(|token| token.access_token)
        .map_err(|error| {
            tracing::warn!(%error, "resposta de token OAuth inválida");
            oauth_upstream_error()
        })
}

async fn fetch_google_profile(
    auth: &AuthService,
    access_token: &str,
) -> Result<GoogleProfile, AuthError> {
    let response = auth
        .http()
        .get(GOOGLE_USERINFO_ENDPOINT)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(%error, "falha ao consultar perfil Google");
            oauth_upstream_error()
        })?;

    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "Google rejeitou a consulta do perfil");
        return Err(oauth_upstream_error());
    }

    let profile = response.json::<GoogleProfile>().await.map_err(|error| {
        tracing::warn!(%error, "perfil Google inválido");
        oauth_upstream_error()
    })?;

    if profile.sub.trim().is_empty()
        || profile.email.as_deref().is_none_or(str::is_empty)
        || profile.email_verified != Some(true)
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            "A Conta Google precisa ter um e-mail verificado".into(),
        ));
    }

    Ok(profile)
}

fn upsert_google_user(
    db: &Db,
    profile: GoogleProfile,
    auth: &AuthService,
) -> Result<CurrentUser, AuthError> {
    let email = profile
        .email
        .expect("e-mail Google validado")
        .trim()
        .to_ascii_lowercase();
    let name = profile
        .name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| email.split('@').next().unwrap_or("Jogador").to_string());
    let subject = profile.sub.trim().to_string();
    let promote_to_admin = i64::from(auth.should_promote_admin(&email));

    let mut conn = lock_db(db)?;
    let transaction = conn.transaction().map_err(database_error)?;

    let existing_user_id = transaction
        .query_row(
            "
            SELECT user_id
            FROM user_identities
            WHERE provider = 'google' AND subject = ?1
            ",
            [&subject],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;

    let user_id = if let Some(user_id) = existing_user_id {
        transaction
            .execute(
                "
                UPDATE users
                SET
                    name = ?1,
                    avatar = ?2,
                    is_admin = ?3
                WHERE id = ?4
                ",
                rusqlite::params![name, profile.picture, promote_to_admin, user_id],
            )
            .map_err(database_error)?;
        user_id
    } else {
        let user_with_email = transaction
            .query_row(
                "SELECT id FROM users WHERE lower(email) = lower(?1)",
                [&email],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(database_error)?;

        let user_id = match user_with_email {
            Some(user_id) => {
                transaction
                    .execute(
                        "
                        UPDATE users
                        SET
                            name = ?1,
                            avatar = ?2,
                            is_admin = ?3
                        WHERE id = ?4
                        ",
                        rusqlite::params![name, profile.picture, promote_to_admin, user_id],
                    )
                    .map_err(database_error)?;
                user_id
            }
            None => {
                transaction
                    .execute(
                        "
                        INSERT INTO users (email, name, avatar, is_admin)
                        VALUES (?1, ?2, ?3, ?4)
                        ",
                        rusqlite::params![email, name, profile.picture, promote_to_admin],
                    )
                    .map_err(database_error)?;
                transaction.last_insert_rowid()
            }
        };

        transaction
            .execute(
                "
                INSERT INTO user_identities (provider, subject, user_id)
                VALUES ('google', ?1, ?2)
                ",
                rusqlite::params![subject, user_id],
            )
            .map_err(database_error)?;
        user_id
    };

    let user = transaction
        .query_row(
            "SELECT id, email, name, avatar, is_admin FROM users WHERE id = ?1",
            [user_id],
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
        .map_err(database_error)?;

    transaction.commit().map_err(database_error)?;
    Ok(user)
}

fn create_session(db: &Db, user_id: i64) -> Result<String, AuthError> {
    let token = random_token();
    let session_id = hash_token(&token);
    let conn = lock_db(db)?;

    conn.execute(
        "DELETE FROM sessions WHERE expires_at <= datetime('now')",
        [],
    )
    .map_err(database_error)?;
    conn.execute(
        "
        INSERT INTO sessions (id, user_id, expires_at)
        VALUES (?1, ?2, datetime('now', '+30 days'))
        ",
        rusqlite::params![session_id, user_id],
    )
    .map_err(database_error)?;

    Ok(token)
}

fn take_oauth_state(db: &Db, state: &str) -> Result<Option<(String, String)>, AuthError> {
    let mut conn = lock_db(db)?;
    let transaction = conn.transaction().map_err(database_error)?;

    let stored = transaction
        .query_row(
            "
            SELECT code_verifier, return_to
            FROM oauth_states
            WHERE state = ?1 AND expires_at > datetime('now')
            ",
            [state],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(database_error)?;

    transaction
        .execute("DELETE FROM oauth_states WHERE state = ?1", [state])
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)?;

    Ok(stored)
}

fn configured_google(auth: &AuthService) -> Result<&GoogleOAuthConfig, AuthError> {
    auth.google().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Login com Google ainda não foi configurado".into(),
    ))
}

fn safe_return_to(value: Option<&str>) -> String {
    match value.map(str::trim) {
        Some(path) if path.starts_with('/') && !path.starts_with("//") => path.to_string(),
        _ => "/".to_string(),
    }
}

fn build_oauth_cookie(value: String, secure: bool) -> Cookie<'static> {
    Cookie::build((OAUTH_STATE_COOKIE, value))
        .path("/api/auth/callback")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(Duration::minutes(OAUTH_STATE_LIFETIME_MINUTES))
        .build()
}

fn clear_oauth_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((OAUTH_STATE_COOKIE, ""))
        .path("/api/auth/callback")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(Duration::ZERO)
        .build()
}

fn build_session_cookie(value: String, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, value))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(Duration::days(SESSION_LIFETIME_DAYS))
        .build()
}

fn clear_session_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(Duration::ZERO)
        .build()
}

fn lock_db(db: &Db) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AuthError> {
    db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco".into(),
        )
    })
}

fn database_error(error: rusqlite::Error) -> AuthError {
    tracing::error!(%error, "erro de banco durante autenticação");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Falha ao persistir autenticação".into(),
    )
}

fn oauth_upstream_error() -> AuthError {
    (
        StatusCode::BAD_GATEWAY,
        "Não foi possível concluir o login com o Google".into(),
    )
}

#[cfg(test)]
mod tests {
    use super::safe_return_to;

    #[test]
    fn return_path_accepts_only_local_absolute_paths() {
        assert_eq!(safe_return_to(Some("/submit")), "/submit");
        assert_eq!(safe_return_to(Some("//example.com")), "/");
        assert_eq!(safe_return_to(Some("https://example.com")), "/");
        assert_eq!(safe_return_to(None), "/");
    }
}
