use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};

pub async fn echo_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                tracing::debug!("Recebido via WS: {}", text);
                
                if socket.send(Message::Text(text)).await.is_err() {
                    tracing::warn!("Cliente desconectou abruptamente");
                    break;
                }
            }
            Message::Close(_) => {
                tracing::info!("Conexão WebSocket encerrada pelo cliente");
                break;
            }
            _ => {}
        }
    }
}
