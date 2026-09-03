use std::{io, process::Stdio};

use tokio::{
    process::Command,
    sync::mpsc::{self, error::TrySendError},
};

use super::{
    protocol::{self, RuntimeCommand, RuntimeMessage},
    stub,
};

const COMMAND_QUEUE_CAPACITY: usize = 128;
const OUTPUT_QUEUE_CAPACITY: usize = 32;

#[derive(Clone)]
pub struct RuntimeHandle {
    commands: mpsc::Sender<RuntimeMessage>,
}

pub enum RuntimeOutput {
    Frame(Vec<u8>),
    Failed(String),
}

impl RuntimeHandle {
    pub fn reserve(game_id: i64) -> (Self, mpsc::Receiver<RuntimeOutput>) {
        let (command_tx, command_rx) = mpsc::channel(COMMAND_QUEUE_CAPACITY);
        let (output_tx, output_rx) = mpsc::channel(OUTPUT_QUEUE_CAPACITY);

        tokio::spawn(run_session(game_id, command_rx, output_tx));

        (
            Self {
                commands: command_tx,
            },
            output_rx,
        )
    }

    pub fn send(&self, command: RuntimeCommand) -> Result<(), RuntimeSendError> {
        self.commands
            .try_send(RuntimeMessage::new(command))
            .map_err(|error| match error {
                TrySendError::Full(_) => RuntimeSendError::Busy,
                TrySendError::Closed(_) => RuntimeSendError::Closed,
            })
    }
}

#[derive(Debug)]
pub enum RuntimeSendError {
    Busy,
    Closed,
}

async fn run_session(
    game_id: i64,
    mut commands: mpsc::Receiver<RuntimeMessage>,
    outputs: mpsc::Sender<RuntimeOutput>,
) {
    let Some(first) = commands.recv().await else {
        return;
    };

    if matches!(&first.command, RuntimeCommand::Stop) {
        return;
    }

    if !matches!(&first.command, RuntimeCommand::Start { .. }) {
        let _ = outputs
            .send(RuntimeOutput::Failed(
                "runtime recebeu evento antes de start".into(),
            ))
            .await;
        return;
    }

    if let Err(error) = run_child(game_id, first, &mut commands, &outputs).await {
        let _ = outputs.send(RuntimeOutput::Failed(error.to_string())).await;
    }
}

async fn run_child(
    game_id: i64,
    first: RuntimeMessage,
    commands: &mut mpsc::Receiver<RuntimeMessage>,
    outputs: &mpsc::Sender<RuntimeOutput>,
) -> io::Result<()> {
    let executable = std::env::current_exe()?;
    let mut child = Command::new(executable)
        .arg("runtime-stub")
        .arg(format!("game-{game_id}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("runtime stdin pipe unavailable"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("runtime stdout pipe unavailable"))?;

    exchange(&mut stdin, &mut stdout, &first, outputs).await?;

    while let Some(message) = commands.recv().await {
        let stopping = matches!(&message.command, RuntimeCommand::Stop);
        exchange(&mut stdin, &mut stdout, &message, outputs).await?;
        if stopping {
            break;
        }
    }

    drop(stdin);
    let status = child.wait().await?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "runtime exited with status {status}"
        )));
    }

    Ok(())
}

async fn exchange<W, R>(
    writer: &mut W,
    reader: &mut R,
    message: &RuntimeMessage,
    outputs: &mpsc::Sender<RuntimeOutput>,
) -> io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
    R: tokio::io::AsyncRead + Unpin,
{
    let payload = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    protocol::write_framed(writer, &payload).await?;

    let frame = protocol::read_framed(reader)
        .await?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "runtime stdout closed"))?;
    outputs
        .send(RuntimeOutput::Frame(frame))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "runtime output receiver closed"))
}

pub async fn run_stub_from_args(game_label: Option<String>) -> io::Result<()> {
    stub::run_stdio(game_label.unwrap_or_else(|| "jogo-sem-nome".into())).await
}
