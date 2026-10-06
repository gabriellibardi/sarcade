use std::time::{Duration, Instant};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::{
    stream::{SplitSink},
    SinkExt, StreamExt,
};
use serde::Deserialize;

use crate::rooms::{
    service,
    ClientMessage, RoomStore, ServerMessage,
};

const ROOM_WS_SUBPROTOCOL: &str = "sinteract.v1";
const MAX_CLIENT_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_INPUT_EVENTS_PER_SECOND: usize = 120;
const CLIENT_SEND_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Deserialize)]
pub struct JoinQuery {
    pub nickname: String,
    pub host_token: Option<String>,
    pub player_token: Option<String>,
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
            "Subprotocolo WebSocket sinteract.v1 é obrigatório",
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
) {
    let join = match service::join_room(
        &rooms,
        &code,
        nickname,
        host_token,
        player_token,
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
    let mut rx = join.rx;

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
    )
    {
        let _ = service::leave_room(&rooms, &code, player_id, connection_generation);
        return;
    }

    let (mut sender, mut receiver) = socket.split();

    let mut keepalive = tokio::time::interval(Duration::from_secs(25));
    let mut input_limiter = InputRateLimiter::new(Instant::now());

    loop {
        tokio::select! {
            incoming = receiver.next() => {
                let Some(Ok(message)) = incoming else {
                    break;
                };

                let should_continue = handle_incoming_message(
                    &mut sender,
                    &rooms,
                    &code,
                    player_id,
                    connection_generation,
                    &mut input_limiter,
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

    let _ = service::leave_room(&rooms, &code, player_id, connection_generation);
}

async fn handle_incoming_message(
    sender: &mut SplitSink<WebSocket, Message>,
    rooms: &RoomStore,
    code: &str,
    player_id: u64,
    connection_generation: u64,
    input_limiter: &mut InputRateLimiter,
    message: Message,
) -> bool {
    match message {
        Message::Text(text) => {
            let parsed = serde_json::from_str::<ClientMessage>(&text.to_string());

            let client_message = match parsed {
                Ok(message) => message,
                Err(_) => {
                    return send_error(sender, "Mensagem WebSocket inválida").await;
                }
            };

            let result = match client_message {
                ClientMessage::Start => {
                    service::start_room(rooms, code, player_id, connection_generation)
                }

                ClientMessage::Kick { player_id: target_id } => {
                    service::kick_player(
                        rooms,
                        code,
                        player_id,
                        connection_generation,
                        target_id,
                    )
                }

                ClientMessage::Promote { player_id: target_id } => {
                    service::promote_host(
                        rooms,
                        code,
                        player_id,
                        connection_generation,
                        target_id,
                    )
                }

                ClientMessage::Ping => {
                    return send_server_message(sender, &ServerMessage::Pong).await;
                }

                other => {
                    if let Some(event) = other.into_input_event() {
                        if !input_limiter.allow(Instant::now()) {
                            let _ = send_error(
                                sender,
                                "Limite de eventos de entrada excedido",
                            )
                            .await;
                            return false;
                        }
                        service::broadcast_input(
                            rooms,
                            code,
                            player_id,
                            connection_generation,
                            event,
                        )
                    } else {
                        Ok(())
                    }
                }
            };

            match result {
                Ok(_) => true,
                Err(error) => send_error(sender, error.message()).await,
            }
        }

        Message::Ping(payload) => {
            send_with_timeout(sender, Message::Pong(payload)).await
        }

        Message::Pong(_) => true,

        Message::Close(_) => false,

        Message::Binary(_) => {
            send_error(
                sender,
                "A integração binária com o Sinteract ainda não está disponível",
            )
            .await
        }
    }
}

struct InputRateLimiter {
    window_started: Instant,
    accepted: usize,
}

impl InputRateLimiter {
    fn new(now: Instant) -> Self {
        Self {
            window_started: now,
            accepted: 0,
        }
    }

    fn allow(&mut self, now: Instant) -> bool {
        if now.duration_since(self.window_started) >= Duration::from_secs(1) {
            self.window_started = now;
            self.accepted = 0;
        }

        if self.accepted >= MAX_INPUT_EVENTS_PER_SECOND {
            return false;
        }

        self.accepted += 1;
        true
    }
}

async fn send_error(
    sender: &mut SplitSink<WebSocket, Message>,
    message: &str,
) -> bool {
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

async fn send_with_timeout(
    sender: &mut SplitSink<WebSocket, Message>,
    message: Message,
) -> bool {
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
    use std::time::{Duration, Instant};

    use super::{
        InputRateLimiter, MAX_CLIENT_MESSAGE_BYTES, MAX_INPUT_EVENTS_PER_SECOND,
        CLIENT_SEND_TIMEOUT, ROOM_WS_SUBPROTOCOL,
    };

    #[test]
    fn room_socket_uses_the_sinteract_protocol_and_view_size_limit() {
        assert_eq!(ROOM_WS_SUBPROTOCOL, "sinteract.v1");
        assert_eq!(MAX_CLIENT_MESSAGE_BYTES, 64 * 1024);
        assert_eq!(CLIENT_SEND_TIMEOUT, Duration::from_secs(2));
    }

    #[test]
    fn input_rate_limit_closes_a_burst_and_recovers_in_the_next_window() {
        let now = Instant::now();
        let mut limiter = InputRateLimiter::new(now);

        for _ in 0..MAX_INPUT_EVENTS_PER_SECOND {
            assert!(limiter.allow(now));
        }
        assert!(!limiter.allow(now));
        assert!(limiter.allow(now + Duration::from_secs(1)));
    }
}
