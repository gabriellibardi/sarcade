use std::{
    io,
    path::PathBuf,
    process::Stdio,
    time::{Duration, Instant},
};

use tokio::{
    process::Command,
    sync::mpsc::{self, error::TrySendError},
};

use super::{
    MaterializedSource, RuntimeSpec,
    protocol::{self, RuntimeCommand, RuntimeMessage},
    stub,
};

const COMMAND_QUEUE_CAPACITY: usize = 128;
const OUTPUT_QUEUE_CAPACITY: usize = 32;
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct RuntimeHandle {
    commands: mpsc::Sender<RuntimeMessage>,
}

pub enum RuntimeOutput {
    Ready,
    Frame { player_id: u64, svg: Vec<u8> },
    Failed(String),
}

impl RuntimeHandle {
    pub fn reserve(spec: RuntimeSpec) -> (Self, mpsc::Receiver<RuntimeOutput>) {
        match std::env::current_exe() {
            Ok(executable) => Self::reserve_with_executable(spec, executable),
            Err(error) => Self::failed_reservation(error.to_string()),
        }
    }

    #[doc(hidden)]
    pub fn reserve_with_executable(
        spec: RuntimeSpec,
        executable: PathBuf,
    ) -> (Self, mpsc::Receiver<RuntimeOutput>) {
        let (command_tx, command_rx) = mpsc::channel(COMMAND_QUEUE_CAPACITY);
        let (output_tx, output_rx) = mpsc::channel(OUTPUT_QUEUE_CAPACITY);

        tokio::spawn(run_session(spec, executable, command_rx, output_tx));

        (
            Self {
                commands: command_tx,
            },
            output_rx,
        )
    }

    fn failed_reservation(message: String) -> (Self, mpsc::Receiver<RuntimeOutput>) {
        let (command_tx, _command_rx) = mpsc::channel(COMMAND_QUEUE_CAPACITY);
        let (output_tx, output_rx) = mpsc::channel(OUTPUT_QUEUE_CAPACITY);
        tokio::spawn(async move {
            let _ = output_tx.send(RuntimeOutput::Failed(message)).await;
        });

        (
            Self {
                commands: command_tx,
            },
            output_rx,
        )
    }

    #[doc(hidden)]
    pub fn simulated_ready() -> (Self, mpsc::Receiver<RuntimeOutput>) {
        let (handle, output_tx, output_rx) = Self::simulated();

        tokio::spawn(async move {
            let _ = output_tx.send(RuntimeOutput::Ready).await;
        });

        (handle, output_rx)
    }

    #[doc(hidden)]
    pub fn simulated() -> (
        Self,
        mpsc::Sender<RuntimeOutput>,
        mpsc::Receiver<RuntimeOutput>,
    ) {
        let (command_tx, mut command_rx) = mpsc::channel::<RuntimeMessage>(COMMAND_QUEUE_CAPACITY);
        let (output_tx, output_rx) = mpsc::channel(OUTPUT_QUEUE_CAPACITY);

        tokio::spawn(async move {
            while let Some(message) = command_rx.recv().await {
                if matches!(message.command, RuntimeCommand::Stop) {
                    return;
                }
            }
        });

        (
            Self {
                commands: command_tx,
            },
            output_tx,
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
    executable: PathBuf,
    commands: mpsc::Receiver<RuntimeMessage>,
    outputs: mpsc::Sender<RuntimeOutput>,
) {
    if let Err(error) = spec.validate_protocol(protocol::PROTOCOL_VERSION) {
        let _ = outputs.send(RuntimeOutput::Failed(error.to_string())).await;
        return;
    }

    if let Err(error) = run_child(&spec, &executable, commands, &outputs).await {
        let _ = outputs.send(RuntimeOutput::Failed(error.to_string())).await;
    }
}

async fn run_child(
    spec: &RuntimeSpec,
    executable: &std::path::Path,
    mut commands: mpsc::Receiver<RuntimeMessage>,
    outputs: &mpsc::Sender<RuntimeOutput>,
) -> io::Result<()> {
    let preparing_since = Instant::now();
    let materialize_spec = spec.clone();
    let temporary_parent = std::env::temp_dir();
    let materialized = tokio::task::spawn_blocking(move || {
        MaterializedSource::create(&materialize_spec, &temporary_parent)
    })
    .await
    .map_err(|error| io::Error::other(format!("runtime materialization task failed: {error}")))??;
    let mut child = Command::new(executable)
        .arg("runtime-stub")
        .arg(format!("game-{}", spec.game_id()))
        .current_dir(materialized.directory())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()?;

    tracing::info!(
        game_id = spec.game_id(),
        revision_id = spec.revision_id(),
        runtime_build = spec.runtime_build_id(),
        source_sha256 = spec.source_sha256_hex(),
        preparation_ms = preparing_since.elapsed().as_millis(),
        "runtime reserva iniciado com fonte materializada"
    );

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("runtime stdin pipe unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("runtime stdout pipe unavailable"))?;

    let mut reader_task = tokio::spawn(read_outputs(stdout, outputs.clone()));
    outputs
        .send(RuntimeOutput::Ready)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "runtime output receiver closed"))?;

    let mut started = false;
    let mut session_error = None;

    loop {
        tokio::select! {
            message = commands.recv() => {
                let Some(message) = message else {
                    break;
                };

                match &message.command {
                    RuntimeCommand::Start { .. } if started => {
                        session_error = Some(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "runtime recebeu um segundo start",
                        ));
                        break;
                    }
                    RuntimeCommand::Start { .. } => started = true,
                    RuntimeCommand::Stop => {}
                    _ if !started => {
                        session_error = Some(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "runtime recebeu evento antes de start",
                        ));
                        break;
                    }
                    _ => {}
                }

                let stopping = matches!(&message.command, RuntimeCommand::Stop);
                if let Err(error) = write_command(&mut stdin, &message).await {
                    session_error = Some(error);
                    break;
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
                let status = match tokio::time::timeout(SHUTDOWN_TIMEOUT, child.wait()).await {
                    Ok(result) => result?,
                    Err(_) => {
                        child.kill().await?;
                        let _ = child.wait().await;
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "runtime closed stdout but did not exit before the shutdown timeout",
                        ));
                    }
                };
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("runtime stdout closed before stop (status {status})"),
                ));
            }
        }
    }

    drop(stdin);
    let status = match tokio::time::timeout(SHUTDOWN_TIMEOUT, child.wait()).await {
        Ok(result) => result?,
        Err(_) => {
            reader_task.abort();
            child.kill().await?;
            let _ = child.wait().await;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "runtime did not exit before the shutdown timeout",
            ));
        }
    };

    match tokio::time::timeout(SHUTDOWN_TIMEOUT, &mut reader_task).await {
        Ok(result) => result
            .map_err(|error| io::Error::other(format!("runtime output task failed: {error}")))??,
        Err(_) => {
            reader_task.abort();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "runtime output pipe did not close before the shutdown timeout",
            ));
        }
    }

    if !status.success() {
        return Err(io::Error::other(format!(
            "runtime exited with status {status}"
        )));
    }

    if let Some(error) = session_error {
        return Err(error);
    }

    tracing::info!(
        game_id = spec.game_id(),
        revision_id = spec.revision_id(),
        runtime_build = spec.runtime_build_id(),
        lifetime_ms = preparing_since.elapsed().as_millis(),
        "runtime reserva encerrado normalmente"
    );

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
