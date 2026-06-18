use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::Serialize;
use tokio::sync::broadcast;

pub type RoomStore = Arc<Mutex<HashMap<String, Room>>>;

#[derive(Clone)]
pub struct Room {
    pub code: String,
    pub game_id: i64,
    pub status: RoomStatus,
    pub min_players: i64,
    pub max_players: i64,
    pub host_player_id: u64,
    pub next_player_id: u64,
    pub players: Vec<Player>,
    pub tx: broadcast::Sender<String>,
}

#[derive(Clone, Serialize)]
pub struct Player {
    pub id: u64,
    pub nickname: String,
    pub is_host: bool,
}

#[derive(Clone, Serialize)]
pub enum RoomStatus {
    Waiting,
    Running,
}

pub fn new_store() -> RoomStore {
    Arc::new(Mutex::new(HashMap::new()))
}
