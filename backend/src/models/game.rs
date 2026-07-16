use serde::Serialize;

#[derive(Serialize)]
pub struct GameSummary {
    pub id: i64,
    pub revision_id: i64,
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
    pub runtime_build_id: String,
    pub runtime: String,
    pub runtime_name: String,
    pub runtime_version: String,
    pub source_extension: String,
}

#[derive(Serialize)]
pub struct GameDetails {
    pub id: i64,
    pub revision_id: i64,
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
    pub runtime_build_id: String,
    pub runtime: String,
    pub runtime_name: String,
    pub runtime_version: String,
    pub source_extension: String,
}

#[derive(Serialize)]
pub struct RuntimeBuildSummary {
    pub id: String,
    pub runtime: String,
    pub display_name: String,
    pub display_version: String,
    pub git_commit: Option<String>,
    pub protocol_version: Option<i64>,
    pub source_extension: String,
    pub status: String,
    pub is_default: bool,
}
