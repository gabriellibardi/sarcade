use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use backend::game_runtime::{RuntimeHandle, RuntimeSpec};

#[derive(Clone)]
pub struct Room {
    pub code: String,
    pub spec: RuntimeSpec,
    pub status: RoomStatus,

    pub host_token: String,
    pub host_player_id: Option<u64>,

    pub next_player_id: u64,
    pub players: Vec<Player>,

    pub tx: broadcast::Sender<ServerMessage>,
    pub runtime: RuntimeHandle,

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
    pub connected: bool,
    #[serde(skip)]
    pub reconnect_token: String,
    #[serde(skip)]
    pub connection_generation: u64,
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
        player_token: String,
        room: RoomSnapshot,
    },

    #[serde(rename = "connection_replaced")]
    ConnectionReplaced {
        #[serde(skip)]
        player_id: u64,
        #[serde(skip)]
        connection_generation: u64,
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

    #[serde(rename = "frame")]
    Frame {
        #[serde(skip)]
        player_id: u64,
        svg: String,
    },

    #[serde(rename = "error")]
    Error {
        message: String,
    },

    #[serde(rename = "pong")]
    Pong,
}

impl ServerMessage {
    pub fn is_for_connection(&self, player_id: u64, connection_generation: u64) -> bool {
        match self {
            Self::Frame {
                player_id: target_id,
                ..
            } => *target_id == player_id,
            Self::ConnectionReplaced {
                player_id: target_id,
                connection_generation: target_generation,
            } => *target_id == player_id && *target_generation == connection_generation,
            _ => true,
        }
    }

    pub fn closes_connection(&self, player_id: u64, connection_generation: u64) -> bool {
        matches!(
            self,
            Self::PlayerKicked {
                player_id: target_id,
                ..
            } if *target_id == player_id
        ) || matches!(
            self,
            Self::ConnectionReplaced {
                player_id: target_id,
                connection_generation: target_generation,
            } if *target_id == player_id && *target_generation == connection_generation
        )
    }
}

#[derive(Debug)]
pub enum RoomError {
    Internal,
    NotFound,
    Full,
    NicknameRequired,
    NicknameTooLong,
    NicknameTaken,
    NotHost,
    InsufficientPlayers,
    PlayerNotFound,
    CannotKickSelf,
    AlreadyRunning,
    NotRunning,
    RuntimeUnavailable,
    InvalidPlayerToken,
    ConnectionReplaced,
}

impl RoomError {
    pub fn message(&self) -> &'static str {
        match self {
            RoomError::Internal => "Erro interno ao acessar sala",
            RoomError::NotFound => "Sala não encontrada",
            RoomError::Full => "Sala cheia",
            RoomError::NicknameRequired => "Nickname é obrigatório",
            RoomError::NicknameTooLong => "Nickname deve ter no máximo 30 caracteres",
            RoomError::NicknameTaken => "Já existe um jogador com esse nickname na sala",
            RoomError::NotHost => "Apenas o host pode executar esta ação",
            RoomError::InsufficientPlayers => "Jogadores insuficientes para iniciar",
            RoomError::PlayerNotFound => "Jogador não encontrado",
            RoomError::CannotKickSelf => "O host não pode expulsar a si mesmo",
            RoomError::AlreadyRunning => "A partida já foi iniciada",
            RoomError::NotRunning => "A partida ainda não foi iniciada",
            RoomError::RuntimeUnavailable => "Runtime da partida indisponível",
            RoomError::InvalidPlayerToken => "Token de reconexão inválido",
            RoomError::ConnectionReplaced => "Esta conexão foi substituída por uma reconexão",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Player, ServerMessage};

    #[test]
    fn targeted_frame_is_visible_only_to_its_player() {
        let frame = ServerMessage::Frame {
            player_id: 7,
            svg: "<svg/>".into(),
        };

        assert!(frame.is_for_connection(7, 1));
        assert!(!frame.is_for_connection(8, 1));
    }

    #[test]
    fn replacement_targets_only_the_previous_connection_generation() {
        let replaced = ServerMessage::ConnectionReplaced {
            player_id: 7,
            connection_generation: 2,
        };

        assert!(replaced.is_for_connection(7, 2));
        assert!(replaced.closes_connection(7, 2));
        assert!(!replaced.is_for_connection(7, 3));
        assert!(!replaced.is_for_connection(8, 2));
    }

    #[test]
    fn websocket_frame_omits_internal_routing_metadata() {
        let frame = ServerMessage::Frame {
            player_id: 7,
            svg: "<svg/>".into(),
        };
        let json = serde_json::to_value(frame).unwrap();

        assert_eq!(json["type"], "frame");
        assert_eq!(json["svg"], "<svg/>");
        assert!(json.get("player_id").is_none());
    }

    #[test]
    fn public_player_never_exposes_reconnection_credentials() {
        let player = Player {
            id: 7,
            nickname: "Ana".into(),
            is_host: true,
            connected: true,
            reconnect_token: "segredo".into(),
            connection_generation: 3,
        };
        let json = serde_json::to_value(player).unwrap();

        assert_eq!(json["connected"], true);
        assert!(json.get("reconnect_token").is_none());
        assert!(json.get("connection_generation").is_none());
    }

    #[test]
    fn room_events_remain_visible_to_every_player() {
        let error = ServerMessage::Error {
            message: "falha".into(),
        };

        assert!(error.is_for_connection(1, 1));
        assert!(error.is_for_connection(2, 1));
    }
}
