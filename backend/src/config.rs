use std::env;
use std::path::PathBuf;

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
    pub sgleam_executable: Option<PathBuf>,
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
        let sgleam_executable = non_empty_env("SGLEAM_EXECUTABLE").map(PathBuf::from);

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
            sgleam_executable,
        }
    }
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
    use super::parse_comma_separated;

    #[test]
    fn parses_normalized_email_list() {
        let emails = parse_comma_separated(" Admin@Example.com, orientador@example.com ");

        assert_eq!(emails, vec!["admin@example.com", "orientador@example.com"]);
    }
}
