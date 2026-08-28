use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CurrentUser {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub avatar: Option<String>,
    pub is_admin: bool,
}

#[derive(Serialize)]
pub struct AuthStatus {
    pub user: Option<CurrentUser>,
    pub google_oauth_enabled: bool,
}
