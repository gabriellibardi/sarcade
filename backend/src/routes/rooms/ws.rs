use std::time::Duration;

use axum::{
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use backend::game_runtime::{SinteractConn, SinteractHandle, SinteractOutput};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use serde::Deserialize;

use crate::rooms::{ClientMessage, RoomError, RoomStatus, RoomStore, ServerMessage, service};

const ROOM_WS_SUBPROTOCOL: &str = "sarcade.room.v1";
const MAX_CLIENT_MESSAGE_BYTES: usize = 64 * 1024;
const CLIENT_SEND_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SocketPhase {
    Lobby,
    Running,
}

struct ActiveSinteract {
    runtime: SinteractHandle,
    conn: SinteractConn,
}

#[derive(Clone, Copy)]
struct RoomConnection<'a> {
    rooms: &'a RoomStore,
    code: &'a str,
    player_id: u64,
    generation: u64,
}

impl SocketPhase {
    fn from_room_status(status: &RoomStatus) -> Self {
        match status {
            RoomStatus::Running => Self::Running,
            RoomStatus::Preparing | RoomStatus::Waiting | RoomStatus::Failed => Self::Lobby,
        }
    }

    fn accepts_client_text(self) -> bool {
        self == Self::Lobby
    }

    fn accepts_client_binary(self) -> bool {
        self == Self::Running
    }
}

#[derive(Deserialize)]
pub struct JoinQuery {
    pub nickname: String,
    pub host_token: Option<String>,
    pub player_token: Option<String>,
    pub invite_token: Option<String>,
}

pub async fn room_ws(
    State(rooms): State<RoomStore>,
    Path(code): Path<String>,
    Query(query): Query<JoinQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let ws = ws
        .protocols([ROOM_WS_SUBPROTOCOL])
        .max_message_size(MAX_CLIENT_MESSAGE_BYTES)
        .max_frame_size(MAX_CLIENT_MESSAGE_BYTES);

    if ws.selected_protocol().is_none() {
        return (
            StatusCode::BAD_REQUEST,
            "Subprotocolo WebSocket sarcade.room.v1 é obrigatório",
        )
            .into_response();
    }

    ws.on_upgrade(move |socket| {
        handle_room_socket(
            socket,
            rooms,
            code,
            query.nickname,
            query.host_token,
            query.player_token,
            query.invite_token,
        )
    })
}

async fn handle_room_socket(
    mut socket: WebSocket,
    rooms: RoomStore,
    code: String,
    nickname: String,
    host_token: Option<String>,
    player_token: Option<String>,
    invite_token: Option<String>,
) {
    let join = match service::join_room_with_invite(
        &rooms,
        &code,
        nickname,
        host_token,
        player_token,
        invite_token,
    ) {
        Ok(join) => join,
        Err(error) => {
            let message = ServerMessage::Error {
                message: error.message().to_string(),
            };

            let _ = tokio::time::timeout(
                CLIENT_SEND_TIMEOUT,
                socket.send(Message::Text(to_ws_text(&message).into())),
            )
            .await;

            return;
        }
    };

    let player_id = join.player_id;
    let connection_generation = join.connection_generation;
    let connection = RoomConnection {
        rooms: &rooms,
        code: &code,
        player_id,
        generation: connection_generation,
    };
    let mut phase = SocketPhase::from_room_status(&join.room.status);
    let mut rx = join.rx;
    let mut sinteract = if phase == SocketPhase::Running {
        match connect_sinteract(&rooms, &code, player_id, connection_generation).await {
            Ok(session) => session,
            Err(error) => {
                let message = ServerMessage::Error {
                    message: error.message().to_string(),
                };
                let _ = socket
                    .send(Message::Text(to_ws_text(&message).into()))
                    .await;
                let _ = service::leave_room(&rooms, &code, player_id, connection_generation);
                return;
            }
        }
    } else {
        None
    };

    let joined_message = ServerMessage::Joined {
        player_id,
        is_host: join.is_host,
        player_token: join.player_token,
        room: join.room,
    };

    if !matches!(
        tokio::time::timeout(
            CLIENT_SEND_TIMEOUT,
            socket.send(Message::Text(to_ws_text(&joined_message).into())),
        )
        .await,
        Ok(Ok(()))
    ) {
        let _ = service::leave_room(&rooms, &code, player_id, connection_generation);
        return;
    }

    let (mut sender, mut receiver) = socket.split();

    let mut keepalive = tokio::time::interval(Duration::from_secs(25));

    loop {
        tokio::select! {
            incoming = receiver.next() => {
                let Some(Ok(message)) = incoming else {
                    break;
                };

                let should_continue = handle_incoming_message(
                    &mut sender,
                    connection,
                    phase,
                    sinteract.as_ref(),
                    message,
                ).await;

                if !should_continue {
                    break;
                }
            }

            broadcast_message = rx.recv() => {
                let message = match broadcast_message {
                    Ok(message) => message,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            room = %code,
                            player_id,
                            skipped,
                            "conexão lenta perdeu mensagens da sala"
                        );
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };

                if !message.is_for_connection(player_id, connection_generation) {
                    continue;
                }

                if matches!(&message, ServerMessage::SinteractChanged) {
                    if let Some(session) = &sinteract
                        && !drain_sinteract(&mut sender, session).await
                    {
                        break;
                    }
                    continue;
                }

                if matches!(&message, ServerMessage::RoomStarted { .. }) {
                    match connect_sinteract(&rooms, &code, player_id, connection_generation).await {
                        Ok(session) => sinteract = session,
                        Err(error) => {
                            let _ = send_error(&mut sender, error.message()).await;
                            break;
                        }
                    }
                    phase = SocketPhase::Running;
                }

                let should_close = message.closes_connection(player_id, connection_generation);

                if !send_server_message(&mut sender, &message).await {
                    break;
                }

                if should_close {
                    break;
                }
            }

            _ = keepalive.tick() => {
                if !send_with_timeout(
                    &mut sender,
                    Message::Ping(Vec::new().into()),
                )
                .await
                {
                    break;
                }
            }
        }
    }

    if let Some(session) = sinteract {
        let _ = session.runtime.leave(session.conn).await;
    }
    let _ = service::leave_room(&rooms, &code, player_id, connection_generation);
}

async fn handle_incoming_message(
    sender: &mut SplitSink<WebSocket, Message>,
    connection: RoomConnection<'_>,
    phase: SocketPhase,
    sinteract: Option<&ActiveSinteract>,
    message: Message,
) -> bool {
    match message {
        Message::Text(_) if !phase.accepts_client_text() => {
            let _ = send_error(
                sender,
                "Mensagens de texto do cliente não são aceitas durante a partida",
            )
            .await;
            false
        }

        Message::Text(text) => {
            let parsed = serde_json::from_str::<ClientMessage>(&text.to_string());

            let client_message = match parsed {
                Ok(message) => message,
                Err(_) => {
                    return send_error(sender, "Mensagem WebSocket inválida").await;
                }
            };

            let result = match client_message {
                ClientMessage::Start => service::start_room(
                    connection.rooms,
                    connection.code,
                    connection.player_id,
                    connection.generation,
                ),

                ClientMessage::Kick {
                    player_id: target_id,
                } => service::kick_player(
                    connection.rooms,
                    connection.code,
                    connection.player_id,
                    connection.generation,
                    target_id,
                ),

                ClientMessage::Promote {
                    player_id: target_id,
                } => service::promote_host(
                    connection.rooms,
                    connection.code,
                    connection.player_id,
                    connection.generation,
                    target_id,
                ),
            };

            match result {
                Ok(_) => true,
                Err(error @ RoomError::AlreadyRunning) => {
                    let _ = send_error(sender, error.message()).await;
                    false
                }
                Err(error) => send_error(sender, error.message()).await,
            }
        }

        Message::Ping(payload) => send_with_timeout(sender, Message::Pong(payload)).await,

        Message::Pong(_) => true,

        Message::Close(_) => false,

        Message::Binary(_) if !phase.accepts_client_binary() => {
            let _ = send_error(
                sender,
                "Mensagens binárias só são aceitas durante a partida",
            )
            .await;
            false
        }

        Message::Binary(payload) => {
            let Some(session) = sinteract else {
                let _ = send_error(
                    sender,
                    "Esta sala ainda usa o runtime reserva sem entrada binária",
                )
                .await;
                return false;
            };
            match session
                .runtime
                .from_view(session.conn, payload.to_vec())
                .await
            {
                Ok(()) => true,
                Err(_) => {
                    let _ = send_error(sender, "Mensagem Sinteract inválida").await;
                    false
                }
            }
        }
    }
}

async fn connect_sinteract(
    rooms: &RoomStore,
    code: &str,
    player_id: u64,
    connection_generation: u64,
) -> Result<Option<ActiveSinteract>, RoomError> {
    let Some(runtime) = service::sinteract_runtime(rooms, code, player_id, connection_generation)?
    else {
        return Ok(None);
    };
    let conn = runtime
        .connect(player_id)
        .await
        .map_err(|_| RoomError::RuntimeUnavailable)?;
    Ok(Some(ActiveSinteract { runtime, conn }))
}

async fn drain_sinteract(
    sender: &mut SplitSink<WebSocket, Message>,
    session: &ActiveSinteract,
) -> bool {
    loop {
        match session.runtime.next_for(session.conn).await {
            Ok(SinteractOutput::Send(payload)) => {
                if !send_with_timeout(sender, Message::Binary(payload.to_vec().into())).await {
                    return false;
                }
            }
            Ok(SinteractOutput::Idle) => return true,
            Ok(SinteractOutput::Gone) | Err(_) => return false,
        }
    }
}

async fn send_error(sender: &mut SplitSink<WebSocket, Message>, message: &str) -> bool {
    let error = ServerMessage::Error {
        message: message.to_string(),
    };

    send_server_message(sender, &error).await
}

async fn send_server_message(
    sender: &mut SplitSink<WebSocket, Message>,
    message: &ServerMessage,
) -> bool {
    send_with_timeout(sender, Message::Text(to_ws_text(message).into())).await
}

async fn send_with_timeout(sender: &mut SplitSink<WebSocket, Message>, message: Message) -> bool {
    matches!(
        tokio::time::timeout(CLIENT_SEND_TIMEOUT, sender.send(message)).await,
        Ok(Ok(()))
    )
}

fn to_ws_text(message: &ServerMessage) -> String {
    serde_json::to_string(message).unwrap_or_else(|_| {
        r#"{"type":"error","message":"Falha ao serializar mensagem"}"#.to_string()
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{CLIENT_SEND_TIMEOUT, MAX_CLIENT_MESSAGE_BYTES, ROOM_WS_SUBPROTOCOL, SocketPhase};
    use crate::rooms::RoomStatus;

    #[test]
    fn room_socket_uses_the_sarcade_protocol_and_view_size_limit() {
        assert_eq!(ROOM_WS_SUBPROTOCOL, "sarcade.room.v1");
        assert_eq!(MAX_CLIENT_MESSAGE_BYTES, 64 * 1024);
        assert_eq!(CLIENT_SEND_TIMEOUT, Duration::from_secs(2));
    }

    #[test]
    fn only_running_rooms_start_in_the_binary_phase() {
        assert_eq!(
            SocketPhase::from_room_status(&RoomStatus::Running),
            SocketPhase::Running
        );
        assert_eq!(
            SocketPhase::from_room_status(&RoomStatus::Waiting),
            SocketPhase::Lobby
        );
        assert!(SocketPhase::Lobby.accepts_client_text());
        assert!(!SocketPhase::Lobby.accepts_client_binary());
        assert!(!SocketPhase::Running.accepts_client_text());
        assert!(SocketPhase::Running.accepts_client_binary());
    }
}
