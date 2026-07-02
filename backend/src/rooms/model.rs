use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct Room {
    pub code: String,
    pub game_id: i64,
    pub status: RoomStatus,

    pub min_players: i64,
    pub max_players: i64,

    pub host_token: String,
    pub host_player_id: Option<u64>,

    pub next_player_id: u64,
    pub players: Vec<Player>,

    pub tx: broadcast::Sender<ServerMessage>,

    pub created_at: Instant,
}

#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RoomStatus {
    Waiting,
    Running,
}

#[derive(Clone, Serialize)]
pub struct Player {
    pub id: u64,
    pub nickname: String,
    pub is_host: bool,
}

#[derive(Clone, Serialize)]
pub struct RoomSnapshot {
    pub code: String,
    pub game_id: i64,
    pub status: RoomStatus,
    pub min_players: i64,
    pub max_players: i64,
    pub players: Vec<Player>,
    pub can_start: bool,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "start")]
    Start,

    #[serde(rename = "kick")]
    Kick { player_id: u64 },

    #[serde(rename = "promote")]
    Promote { player_id: u64 },

    #[serde(rename = "keydown")]
    KeyDown { key: String },

    #[serde(rename = "keyup")]
    KeyUp { key: String },

    #[serde(rename = "paddown")]
    PadDown { input: String },

    #[serde(rename = "padup")]
    PadUp { input: String },

    #[serde(rename = "ping")]
    Ping,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum InputEvent {
    #[serde(rename = "keydown")]
    KeyDown { key: String },

    #[serde(rename = "keyup")]
    KeyUp { key: String },

    #[serde(rename = "paddown")]
    PadDown { input: String },

    #[serde(rename = "padup")]
    PadUp { input: String },
}

impl ClientMessage {
    pub fn into_input_event(self) -> Option<InputEvent> {
        match self {
            ClientMessage::KeyDown { key } => Some(InputEvent::KeyDown { key }),
            ClientMessage::KeyUp { key } => Some(InputEvent::KeyUp { key }),
            ClientMessage::PadDown { input } => Some(InputEvent::PadDown { input }),
            ClientMessage::PadUp { input } => Some(InputEvent::PadUp { input }),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "joined")]
    Joined {
        player_id: u64,
        is_host: bool,
        room: RoomSnapshot,
    },

    #[serde(rename = "room_state")]
    RoomState {
        room: RoomSnapshot,
    },

    #[serde(rename = "room_started")]
    RoomStarted {
        room: RoomSnapshot,
    },

    #[serde(rename = "host_changed")]
    HostChanged {
        host_player_id: u64,
        room: RoomSnapshot,
    },

    #[serde(rename = "player_kicked")]
    PlayerKicked {
        player_id: u64,
        room: RoomSnapshot,
    },

    #[serde(rename = "player_left")]
    PlayerLeft {
        player_id: u64,
        room: RoomSnapshot,
    },

    #[serde(rename = "input")]
    Input {
        player_id: u64,
        event: InputEvent,
    },

    #[serde(rename = "error")]
    Error {
        message: String,
    },

    #[serde(rename = "pong")]
    Pong,
}

#[derive(Debug)]
pub enum RoomError {
    Internal,
    NotFound,
    Full,
    NicknameRequired,
    NicknameTaken,
    NotHost,
    InsufficientPlayers,
    PlayerNotFound,
    CannotKickSelf,
    AlreadyRunning,
}

impl RoomError {
    pub fn message(&self) -> &'static str {
        match self {
            RoomError::Internal => "Erro interno ao acessar sala",
            RoomError::NotFound => "Sala não encontrada",
            RoomError::Full => "Sala cheia",
            RoomError::NicknameRequired => "Nickname é obrigatório",
            RoomError::NicknameTaken => "Já existe um jogador com esse nickname na sala",
            RoomError::NotHost => "Apenas o host pode executar esta ação",
            RoomError::InsufficientPlayers => "Jogadores insuficientes para iniciar",
            RoomError::PlayerNotFound => "Jogador não encontrado",
            RoomError::CannotKickSelf => "O host não pode expulsar a si mesmo",
            RoomError::AlreadyRunning => "A partida já foi iniciada",
        }
    }
}
