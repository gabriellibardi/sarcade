use serde::Serialize;

#[derive(Serialize)]
pub struct GameSummary {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub min_players: i64,
    pub max_players: i64,
    pub author_id: Option<i64>,
    pub author_name: Option<String>,
    pub is_featured: bool,
    pub created_at: String,
    pub has_thumbnail: bool,
}

#[derive(Serialize)]
pub struct GameDetails {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub source_code: String,
    pub status: String,
    pub min_players: i64,
    pub max_players: i64,
    pub author_id: Option<i64>,
    pub author_name: Option<String>,
    pub is_featured: bool,
    pub review_message: Option<String>,
    pub created_at: String,
    pub reviewed_at: Option<String>,
    pub has_thumbnail: bool,
}
