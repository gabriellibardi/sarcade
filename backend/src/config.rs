use std::env;
use std::path::PathBuf;

const DEFAULT_MAX_RUNTIME_PROCESSES: usize = 32;
const MAX_CONFIGURED_RUNTIME_PROCESSES: usize = 256;

#[derive(Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub cookie_secure: bool,
}

pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub google_oauth: Option<GoogleOAuthConfig>,
    pub admin_emails: Vec<String>,
    pub runtime_manifest: PathBuf,
    pub max_runtime_processes: usize,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let port = env::var("PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3000);
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "arcade.db".to_string());
        let public_base_url =
            non_empty_env("PUBLIC_BASE_URL").map(|value| value.trim_end_matches('/').to_string());
        let admin_emails = comma_separated_env("ADMIN_EMAILS");
        let runtime_manifest = non_empty_env("RUNTIME_MANIFEST")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runtimes.json"));
        let max_runtime_processes = runtime_process_limit();

        let client_id = non_empty_env("GOOGLE_CLIENT_ID");
        let client_secret = non_empty_env("GOOGLE_CLIENT_SECRET");

        let google_oauth = match (client_id, client_secret) {
            (Some(client_id), Some(client_secret)) => {
                let redirect_uri = public_base_url
                    .as_ref()
                    .map(|base_url| format!("{base_url}/api/auth/callback"));

                match redirect_uri {
                    Some(redirect_uri) => Some(GoogleOAuthConfig {
                        client_id,
                        client_secret,
                        cookie_secure: redirect_uri.starts_with("https://"),
                        redirect_uri,
                    }),
                    None => {
                        tracing::warn!("login Google desabilitado: defina PUBLIC_BASE_URL");
                        None
                    }
                }
            }
            (None, None) => None,
            _ => {
                tracing::warn!(
                    "login Google desabilitado: GOOGLE_CLIENT_ID e GOOGLE_CLIENT_SECRET devem ser configurados juntos"
                );
                None
            }
        };

        Self {
            port,
            database_url,
            google_oauth,
            admin_emails,
            runtime_manifest,
            max_runtime_processes,
        }
    }
}

fn runtime_process_limit() -> usize {
    let configured = non_empty_env("MAX_RUNTIME_PROCESSES");
    let parsed = parse_bounded_positive_usize(
        configured.as_deref(),
        DEFAULT_MAX_RUNTIME_PROCESSES,
        MAX_CONFIGURED_RUNTIME_PROCESSES,
    );

    if configured.is_some()
        && configured.as_deref().and_then(|value| value.parse().ok()) != Some(parsed)
    {
        tracing::warn!(
            value = configured.as_deref().unwrap_or_default(),
            default = DEFAULT_MAX_RUNTIME_PROCESSES,
            maximum = MAX_CONFIGURED_RUNTIME_PROCESSES,
            "MAX_RUNTIME_PROCESSES inválido; usando o valor padrão"
        );
    }

    parsed
}

fn parse_bounded_positive_usize(value: Option<&str>, default: usize, maximum: usize) -> usize {
    value
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| (1..=maximum).contains(value))
        .unwrap_or(default)
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn comma_separated_env(name: &str) -> Vec<String> {
    parse_comma_separated(&env::var(name).unwrap_or_default())
}

fn parse_comma_separated(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_bounded_positive_usize, parse_comma_separated};

    #[test]
    fn parses_normalized_email_list() {
        let emails = parse_comma_separated(" Admin@Example.com, orientador@example.com ");

        assert_eq!(emails, vec!["admin@example.com", "orientador@example.com"]);
    }

    #[test]
    fn runtime_process_limit_is_positive_and_bounded() {
        assert_eq!(parse_bounded_positive_usize(Some("1"), 32, 256), 1);
        assert_eq!(parse_bounded_positive_usize(Some("256"), 32, 256), 256);
        assert_eq!(parse_bounded_positive_usize(Some("0"), 32, 256), 32);
        assert_eq!(parse_bounded_positive_usize(Some("257"), 32, 256), 32);
        assert_eq!(parse_bounded_positive_usize(Some("invalid"), 32, 256), 32);
        assert_eq!(parse_bounded_positive_usize(None, 32, 256), 32);
    }
}
