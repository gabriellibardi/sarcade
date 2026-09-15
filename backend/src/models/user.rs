use serde::Serialize;

use crate::models::game::GameSummary;

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

#[derive(Serialize)]
pub struct PublicUserProfile {
    pub id: i64,
    pub name: String,
    pub avatar: Option<String>,
    pub created_at: String,
    pub published_games: Vec<GameSummary>,
}
