use std::io;

use tokio::io::{AsyncRead, AsyncWrite};

use super::protocol::{self, PROTOCOL_VERSION, RuntimeCommand, RuntimeInputEvent, RuntimeMessage};

const MAX_HISTORY_LINES: usize = 24;

pub async fn run_stdio(game_label: String) -> io::Result<()> {
    run(tokio::io::stdin(), tokio::io::stdout(), game_label).await
}

pub async fn run<R, W>(mut reader: R, mut writer: W, game_label: String) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut history = Vec::new();

    while let Some(payload) = protocol::read_framed(&mut reader).await? {
        let message: RuntimeMessage = serde_json::from_slice(&payload).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid runtime message: {error}"),
            )
        })?;

        if message.protocol_version != PROTOCOL_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "unsupported runtime protocol version: {}",
                    message.protocol_version
                ),
            ));
        }

        let should_stop = matches!(message.command, RuntimeCommand::Stop);
        record(&mut history, &message.command);
        let svg = render_svg(&game_label, &history);
        protocol::write_framed(&mut writer, svg.as_bytes()).await?;

        if should_stop {
            break;
        }
    }

    Ok(())
}

fn record(history: &mut Vec<String>, command: &RuntimeCommand) {
    let line = match command {
        RuntimeCommand::Start { game_id, players } => {
            let players = players
                .iter()
                .map(|player| format!("#{} {}", player.id, player.nickname))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Jogo #{game_id} iniciado com: {players}")
        }
        RuntimeCommand::Input { player_id, event } => {
            format!("Jogador #{player_id}: {}", describe_input(event))
        }
        RuntimeCommand::PlayerLeft { player_id } => format!("Jogador #{player_id} saiu"),
        RuntimeCommand::Tick => "Tick".to_string(),
        RuntimeCommand::Stop => "Partida encerrada".to_string(),
    };

    history.push(line);
    if history.len() > MAX_HISTORY_LINES {
        history.remove(0);
    }
}

fn describe_input(event: &RuntimeInputEvent) -> String {
    match event {
        RuntimeInputEvent::KeyDown { key } => format!("keydown {key}"),
        RuntimeInputEvent::KeyUp { key } => format!("keyup {key}"),
        RuntimeInputEvent::PadDown { input } => format!("paddown {input}"),
        RuntimeInputEvent::PadUp { input } => format!("padup {input}"),
    }
}

fn render_svg(game_label: &str, history: &[String]) -> String {
    let mut lines = String::new();
    for (index, line) in history.iter().rev().enumerate() {
        let y = 92 + index * 24;
        lines.push_str(&format!(
            r##"<text x="24" y="{y}" font-family="monospace" font-size="16" fill="#dbeafe">{}</text>"##,
            escape_xml(line)
        ));
    }

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="720" height="680" viewBox="0 0 720 680"><rect width="720" height="680" fill="#0f172a"/><text x="24" y="38" font-family="sans-serif" font-size="24" font-weight="bold" fill="#f8fafc">Runtime reserva do Sarcade</text><text x="24" y="64" font-family="monospace" font-size="15" fill="#94a3b8">{}</text>{lines}</svg>"##,
        escape_xml(game_label)
    )
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use tokio::io::split;

    use super::*;
    use crate::game_runtime::protocol::{RuntimePlayer, read_framed, write_framed};

    #[tokio::test]
    async fn answers_each_message_with_an_svg_containing_the_event_history() {
        let (server, runtime) = tokio::io::duplex(64 * 1024);
        let (mut server_reader, mut server_writer) = split(server);
        let (runtime_reader, runtime_writer) = split(runtime);

        let runtime_task = tokio::spawn(run(runtime_reader, runtime_writer, "jogo-teste".into()));

        let start = RuntimeMessage::new(RuntimeCommand::Start {
            game_id: 7,
            players: vec![RuntimePlayer {
                id: 1,
                nickname: "Ana & Bia".into(),
            }],
        });
        write_framed(&mut server_writer, &serde_json::to_vec(&start).unwrap())
            .await
            .unwrap();
        let first = read_framed(&mut server_reader).await.unwrap().unwrap();
        let first = String::from_utf8(first).unwrap();
        assert!(first.starts_with("<svg"));
        assert!(first.contains("Ana &amp; Bia"));

        let input = RuntimeMessage::new(RuntimeCommand::Input {
            player_id: 1,
            event: RuntimeInputEvent::PadDown {
                input: "Left".into(),
            },
        });
        write_framed(&mut server_writer, &serde_json::to_vec(&input).unwrap())
            .await
            .unwrap();
        let second = read_framed(&mut server_reader).await.unwrap().unwrap();
        let second = String::from_utf8(second).unwrap();
        assert!(second.contains("Jogador #1: paddown Left"));

        let stop = RuntimeMessage::new(RuntimeCommand::Stop);
        write_framed(&mut server_writer, &serde_json::to_vec(&stop).unwrap())
            .await
            .unwrap();
        assert!(read_framed(&mut server_reader).await.unwrap().is_some());
        runtime_task.await.unwrap().unwrap();
    }
}
