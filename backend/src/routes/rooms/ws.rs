use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::IntoResponse,
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
) -> impl IntoResponse {
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

            let _ = socket
                .send(Message::Text(to_ws_text(&message).into()))
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

    if socket
        .send(Message::Text(to_ws_text(&joined_message).into()))
        .await
        .is_err()
    {
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
                    &rooms,
                    &code,
                    player_id,
                    connection_generation,
                    message,
                ).await;

                if !should_continue {
                    break;
                }
            }

            broadcast_message = rx.recv() => {
                let Ok(message) = broadcast_message else {
                    break;
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
                if sender
                    .send(Message::Ping(Vec::new().into()))
                    .await
                    .is_err()
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
            sender.send(Message::Pong(payload)).await.is_ok()
        }

        Message::Pong(_) => true,

        Message::Close(_) => false,

        _ => true,
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
    sender
        .send(Message::Text(to_ws_text(message).into()))
        .await
        .is_ok()
}

fn to_ws_text(message: &ServerMessage) -> String {
    serde_json::to_string(message).unwrap_or_else(|_| {
        r#"{"type":"error","message":"Falha ao serializar mensagem"}"#.to_string()
    })
}
