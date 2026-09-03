use std::io;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_PAYLOAD_BYTES: u32 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RuntimeMessage {
    pub protocol_version: u32,
    #[serde(flatten)]
    pub command: RuntimeCommand,
}

impl RuntimeMessage {
    pub fn new(command: RuntimeCommand) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            command,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeCommand {
    Start {
        game_id: i64,
        players: Vec<RuntimePlayer>,
    },
    Input {
        player_id: u64,
        event: RuntimeInputEvent,
    },
    PlayerLeft {
        player_id: u64,
    },
    Tick,
    Stop,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RuntimePlayer {
    pub id: u64,
    pub nickname: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum RuntimeInputEvent {
    #[serde(rename = "keydown")]
    KeyDown { key: String },
    #[serde(rename = "keyup")]
    KeyUp { key: String },
    #[serde(rename = "paddown")]
    PadDown { input: String },
    #[serde(rename = "padup")]
    PadUp { input: String },
}

pub async fn read_framed<R>(reader: &mut R) -> io::Result<Option<Vec<u8>>>
where
    R: AsyncRead + Unpin,
{
    let length = match reader.read_u32().await {
        Ok(length) => length,
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    };

    if length == 0 || length > MAX_PAYLOAD_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid runtime payload length: {length}"),
        ));
    }

    let mut payload = vec![0; length as usize];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

pub async fn write_framed<W>(writer: &mut W, payload: &[u8]) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let length = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "runtime payload is too large"))?;

    if length == 0 || length > MAX_PAYLOAD_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid runtime payload length: {length}"),
        ));
    }

    writer.write_u32(length).await?;
    writer.write_all(payload).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;

    #[tokio::test]
    async fn framing_uses_a_big_endian_u32_and_exact_payload() {
        let (mut writer, mut reader) = tokio::io::duplex(64);

        let write = tokio::spawn(async move { write_framed(&mut writer, b"svg").await });

        let mut header = [0; 4];
        reader.read_exact(&mut header).await.unwrap();
        assert_eq!(header, [0, 0, 0, 3]);

        let mut payload = [0; 3];
        reader.read_exact(&mut payload).await.unwrap();
        assert_eq!(&payload, b"svg");
        write.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn reads_back_to_back_frames_without_losing_boundaries() {
        let (mut writer, mut reader) = tokio::io::duplex(64);

        let write = tokio::spawn(async move {
            write_framed(&mut writer, b"first").await?;
            write_framed(&mut writer, b"second").await
        });

        assert_eq!(read_framed(&mut reader).await.unwrap().unwrap(), b"first");
        assert_eq!(read_framed(&mut reader).await.unwrap().unwrap(), b"second");
        assert!(read_framed(&mut reader).await.unwrap().is_none());
        write.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn rejects_a_length_above_the_cap_before_allocating() {
        let (mut writer, mut reader) = tokio::io::duplex(16);
        writer.write_u32(MAX_PAYLOAD_BYTES + 1).await.unwrap();

        let error = read_framed(&mut reader).await.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn input_names_match_the_existing_websocket_contract() {
        let events = [
            (RuntimeInputEvent::KeyDown { key: "a".into() }, "keydown"),
            (RuntimeInputEvent::KeyUp { key: "a".into() }, "keyup"),
            (
                RuntimeInputEvent::PadDown {
                    input: "ActionA".into(),
                },
                "paddown",
            ),
            (
                RuntimeInputEvent::PadUp {
                    input: "ActionA".into(),
                },
                "padup",
            ),
        ];

        for (event, expected_type) in events {
            let message = RuntimeMessage::new(RuntimeCommand::Input {
                player_id: 3,
                event,
            });
            let json = serde_json::to_value(message).unwrap();

            assert_eq!(json["type"], "input");
            assert_eq!(json["event"]["type"], expected_type);
        }
    }
}
