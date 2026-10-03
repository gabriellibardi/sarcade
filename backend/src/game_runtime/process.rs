use std::{io, process::Stdio};

use tokio::{
    process::Command,
    sync::mpsc::{self, error::TrySendError},
};

use super::{
    protocol::{self, RuntimeCommand, RuntimeMessage},
    stub, RuntimeSpec,
};

const COMMAND_QUEUE_CAPACITY: usize = 128;
const OUTPUT_QUEUE_CAPACITY: usize = 32;

#[derive(Clone)]
pub struct RuntimeHandle {
    commands: mpsc::Sender<RuntimeMessage>,
}

pub enum RuntimeOutput {
    Frame { player_id: u64, svg: Vec<u8> },
    Failed(String),
}

impl RuntimeHandle {
    pub fn reserve(spec: RuntimeSpec) -> (Self, mpsc::Receiver<RuntimeOutput>) {
        let (command_tx, command_rx) = mpsc::channel(COMMAND_QUEUE_CAPACITY);
        let (output_tx, output_rx) = mpsc::channel(OUTPUT_QUEUE_CAPACITY);

        tokio::spawn(run_session(spec, command_rx, output_tx));

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
    spec: RuntimeSpec,
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

    if let Err(error) = run_child(&spec, first, commands, &outputs).await {
        let _ = outputs.send(RuntimeOutput::Failed(error.to_string())).await;
    }
}

async fn run_child(
    spec: &RuntimeSpec,
    first: RuntimeMessage,
    mut commands: mpsc::Receiver<RuntimeMessage>,
    outputs: &mpsc::Sender<RuntimeOutput>,
) -> io::Result<()> {
    let executable = std::env::current_exe()?;
    let mut child = Command::new(executable)
        .arg("runtime-stub")
        .arg(format!("game-{}", spec.game_id()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("runtime stdin pipe unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("runtime stdout pipe unavailable"))?;

    write_command(&mut stdin, &first).await?;
    let mut reader_task = tokio::spawn(read_outputs(stdout, outputs.clone()));

    loop {
        tokio::select! {
            message = commands.recv() => {
                let Some(message) = message else {
                    break;
                };
                let stopping = matches!(&message.command, RuntimeCommand::Stop);
                if let Err(error) = write_command(&mut stdin, &message).await {
                    reader_task.abort();
                    return Err(error);
                }
                if stopping {
                    break;
                }
            }
            result = &mut reader_task => {
                let result = result.map_err(|error| {
                    io::Error::other(format!("runtime output task failed: {error}"))
                })?;
                result?;

                drop(stdin);
                let status = child.wait().await?;
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("runtime stdout closed before stop (status {status})"),
                ));
            }
        }
    }

    drop(stdin);
    let status = child.wait().await?;
    reader_task
        .await
        .map_err(|error| io::Error::other(format!("runtime output task failed: {error}")))??;

    if !status.success() {
        return Err(io::Error::other(format!(
            "runtime exited with status {status}"
        )));
    }

    Ok(())
}

async fn write_command<W>(writer: &mut W, message: &RuntimeMessage) -> io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let payload = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    protocol::write_framed(writer, &payload).await
}

async fn read_outputs<R>(mut reader: R, outputs: mpsc::Sender<RuntimeOutput>) -> io::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
{
    while let Some(frame) = protocol::read_player_frame(&mut reader).await? {
        outputs
            .send(RuntimeOutput::Frame {
                player_id: frame.player_id,
                svg: frame.svg,
            })
            .await
            .map_err(|_| {
                io::Error::new(io::ErrorKind::BrokenPipe, "runtime output receiver closed")
            })?;
    }

    Ok(())
}

pub async fn run_stub_from_args(game_label: Option<String>) -> io::Result<()> {
    stub::run_stdio(game_label.unwrap_or_else(|| "jogo-sem-nome".into())).await
}
