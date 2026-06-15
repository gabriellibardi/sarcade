use serde::Serialize;

#[derive(Serialize)]
pub struct GameSummary {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub min_players: i64,
    pub max_players: i64,
    pub is_featured: bool,
    pub created_at: String,
    pub has_thumbnail: bool,
}
