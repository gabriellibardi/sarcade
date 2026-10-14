use std::{
    collections::{HashMap, HashSet},
    io,
    num::NonZeroU32,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use sinteract::{
    server::{Conn, LobbyCore, Next, ServerCore},
    session::{Player, PlayerRange},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, mpsc::error::TrySendError, oneshot, watch},
};

use super::{MaterializedSource, RuntimeSpec};

const COMMAND_QUEUE_CAPACITY: usize = 128;
const STATUS_QUEUE_CAPACITY: usize = 16;
const READ_BYTES: usize = 64 * 1024;
const TICK_RATE: NonZeroU32 = NonZeroU32::new(60_000).expect("60 Hz is not zero");
const TICK_PERIOD: Duration = Duration::from_nanos(1_000_000_000_000 / TICK_RATE.get() as u64);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SinteractPlayer {
    pub id: u64,
    pub nickname: String,
}

pub type SinteractConn = Conn;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SinteractStatus {
    Ready,
    Ended,
    Failed(String),
}

#[derive(Clone, Debug)]
pub enum SinteractOutput {
    Send(Arc<[u8]>),
    Idle,
    Gone,
}

#[derive(Clone)]
pub struct SinteractHandle {
    commands: mpsc::Sender<Command>,
    changes: watch::Receiver<u64>,
}

impl SinteractHandle {
    /// Prepare a Sinteract session by running `executable` with the exact
    /// approved source path as its only argument.
    pub fn reserve(
        spec: RuntimeSpec,
        executable: PathBuf,
    ) -> (Self, mpsc::Receiver<SinteractStatus>) {
        Self::reserve_with_launch(spec, Launch::Runtime(executable))
    }

    #[doc(hidden)]
    pub fn reserve_fixture(
        spec: RuntimeSpec,
        executable: PathBuf,
    ) -> (Self, mpsc::Receiver<SinteractStatus>) {
        Self::reserve_with_launch(spec, Launch::Fixture(executable))
    }

    fn reserve_with_launch(
        spec: RuntimeSpec,
        launch: Launch,
    ) -> (Self, mpsc::Receiver<SinteractStatus>) {
        let (command_tx, command_rx) = mpsc::channel(COMMAND_QUEUE_CAPACITY);
        let (status_tx, status_rx) = mpsc::channel(STATUS_QUEUE_CAPACITY);
        let (change_tx, change_rx) = watch::channel(0);

        tokio::spawn(async move {
            if let Err(error) =
                run_session(spec, launch, command_rx, status_tx.clone(), change_tx).await
            {
                let _ = status_tx
                    .send(SinteractStatus::Failed(error.to_string()))
                    .await;
            }
        });

        (
            Self {
                commands: command_tx,
                changes: change_rx,
            },
            status_rx,
        )
    }

    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.changes.clone()
    }

    pub async fn start(&self, players: Vec<SinteractPlayer>) -> Result<(), SinteractCommandError> {
        request(&self.commands, |reply| Command::Start { players, reply }).await
    }

    /// Queue the start without waiting for pipe I/O. Commands queued after it
    /// observe the resulting `ServerCore` because the actor is serial.
    pub fn try_start(&self, players: Vec<SinteractPlayer>) -> Result<(), SinteractCommandError> {
        let (reply, _ignored) = oneshot::channel();
        self.commands
            .try_send(Command::Start { players, reply })
            .map_err(|error| match error {
                TrySendError::Full(_) => SinteractCommandError::Busy,
                TrySendError::Closed(_) => SinteractCommandError::Closed,
            })
    }

    pub async fn connect(&self, player_id: u64) -> Result<SinteractConn, SinteractCommandError> {
        request(&self.commands, |reply| Command::Connect {
            player_id,
            reply,
        })
        .await
    }

    pub async fn from_view(
        &self,
        conn: SinteractConn,
        payload: Vec<u8>,
    ) -> Result<(), SinteractCommandError> {
        request(&self.commands, |reply| Command::FromView {
            conn,
            payload,
            reply,
        })
        .await
    }

    pub async fn next_for(
        &self,
        conn: SinteractConn,
    ) -> Result<SinteractOutput, SinteractCommandError> {
        request(&self.commands, |reply| Command::Next { conn, reply }).await
    }

    pub async fn leave(&self, conn: SinteractConn) -> Result<(), SinteractCommandError> {
        request(&self.commands, |reply| Command::Leave { conn, reply }).await
    }

    pub async fn stop(&self) -> Result<(), SinteractCommandError> {
        request(&self.commands, |reply| Command::Stop { reply }).await
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SinteractCommandError {
    Busy,
    Closed,
    NotReady,
    AlreadyStarted,
    DuplicatePlayer,
    InvalidPlayers,
    UnknownPlayer,
    EngineEnded,
    InvalidView,
}

impl std::fmt::Display for SinteractCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Busy => "the Sinteract command queue is full",
            Self::Closed => "the Sinteract session is closed",
            Self::NotReady => "the Sinteract engine has not sent its hello",
            Self::AlreadyStarted => "the Sinteract session has already started",
            Self::DuplicatePlayer => "the Sinteract player list contains a duplicate id",
            Self::InvalidPlayers => "the Sinteract player count is outside the engine range",
            Self::UnknownPlayer => "the player has no seat in the Sinteract session",
            Self::EngineEnded => "the Sinteract engine has ended",
            Self::InvalidView => "the Sinteract view message is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SinteractCommandError {}

enum Command {
    Start {
        players: Vec<SinteractPlayer>,
        reply: oneshot::Sender<Result<(), SinteractCommandError>>,
    },
    Connect {
        player_id: u64,
        reply: oneshot::Sender<Result<SinteractConn, SinteractCommandError>>,
    },
    FromView {
        conn: SinteractConn,
        payload: Vec<u8>,
        reply: oneshot::Sender<Result<(), SinteractCommandError>>,
    },
    Next {
        conn: SinteractConn,
        reply: oneshot::Sender<Result<SinteractOutput, SinteractCommandError>>,
    },
    Leave {
        conn: SinteractConn,
        reply: oneshot::Sender<Result<(), SinteractCommandError>>,
    },
    Stop {
        reply: oneshot::Sender<Result<(), SinteractCommandError>>,
    },
}

async fn request<T>(
    commands: &mpsc::Sender<Command>,
    make_command: impl FnOnce(oneshot::Sender<Result<T, SinteractCommandError>>) -> Command,
) -> Result<T, SinteractCommandError> {
    let (reply_tx, reply_rx) = oneshot::channel();
    commands
        .send(make_command(reply_tx))
        .await
        .map_err(|_| SinteractCommandError::Closed)?;
    reply_rx.await.unwrap_or(Err(SinteractCommandError::Closed))
}

enum Core {
    Lobby(Option<LobbyCore>),
    Running {
        server: ServerCore,
        players: HashMap<u64, Player>,
    },
}

enum Launch {
    Runtime(PathBuf),
    Fixture(PathBuf),
}

#[cfg(unix)]
async fn run_session(
    spec: RuntimeSpec,
    launch: Launch,
    commands: mpsc::Receiver<Command>,
    statuses: mpsc::Sender<SinteractStatus>,
    changes: watch::Sender<u64>,
) -> io::Result<()> {
    run_unix_session(spec, launch, commands, statuses, changes).await
}

#[cfg(not(unix))]
async fn run_session(
    _spec: RuntimeSpec,
    _launch: Launch,
    _commands: mpsc::Receiver<Command>,
    _statuses: mpsc::Sender<SinteractStatus>,
    _changes: watch::Sender<u64>,
) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Sinteract fd sessions require Unix",
    ))
}

#[cfg(unix)]
async fn run_unix_session(
    spec: RuntimeSpec,
    launch: Launch,
    mut commands: mpsc::Receiver<Command>,
    statuses: mpsc::Sender<SinteractStatus>,
    changes: watch::Sender<u64>,
) -> io::Result<()> {
    use std::{
        fs::File,
        io::pipe,
        os::fd::OwnedFd,
        process::{Command as StdCommand, Stdio},
    };

    use command_fds::{CommandFdExt, FdMapping};
    use sinteract::session::{ENGINE_TO_SERVER_FD, SERVER_TO_ENGINE_FD, SESSION_VAR};

    if spec.runtime_status() == "unavailable" {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "runtime build is unavailable",
        ));
    }

    let materialize_spec = spec.clone();
    let temporary_parent = std::env::temp_dir();
    let materialized = tokio::task::spawn_blocking(move || {
        MaterializedSource::create(&materialize_spec, &temporary_parent)
    })
    .await
    .map_err(|error| io::Error::other(format!("runtime materialization task failed: {error}")))??;

    let (executable, args) = match launch {
        Launch::Runtime(executable) => (
            executable,
            vec![materialized.source_path().as_os_str().to_owned()],
        ),
        Launch::Fixture(executable) => (
            executable,
            vec![
                "sinteract-runtime-stub".into(),
                spec.min_players().to_string().into(),
                spec.max_players().to_string().into(),
            ],
        ),
    };

    let (engine_reads, to_engine) = pipe()?;
    let (from_engine, engine_writes) = pipe()?;
    let mut command = StdCommand::new(executable);
    command
        .args(args)
        .current_dir(materialized.directory())
        .env_clear()
        .env(SESSION_VAR, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .fd_mappings(vec![
            FdMapping {
                parent_fd: OwnedFd::from(engine_reads),
                child_fd: SERVER_TO_ENGINE_FD,
            },
            FdMapping {
                parent_fd: OwnedFd::from(engine_writes),
                child_fd: ENGINE_TO_SERVER_FD,
            },
        ])
        .map_err(io::Error::other)?;

    let mut child = tokio::process::Command::from(command)
        .kill_on_drop(true)
        .spawn()?;
    let mut to_engine = tokio::fs::File::from_std(File::from(OwnedFd::from(to_engine)));
    let mut from_engine = tokio::fs::File::from_std(File::from(OwnedFd::from(from_engine)));

    let result = drive_session(
        &spec,
        &mut from_engine,
        &mut to_engine,
        &mut commands,
        &statuses,
        &changes,
    )
    .await;

    drop(to_engine);
    match tokio::time::timeout(SHUTDOWN_TIMEOUT, child.wait()).await {
        Ok(waited) => {
            let status = waited?;
            if result.is_ok() && !status.success() {
                return Err(io::Error::other(format!(
                    "Sinteract engine exited with status {status}"
                )));
            }
        }
        Err(_) => {
            child.kill().await?;
            let _ = child.wait().await;
            if result.is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Sinteract engine did not stop before the shutdown timeout",
                ));
            }
        }
    }

    result
}

async fn drive_session<R, W>(
    spec: &RuntimeSpec,
    from_engine: &mut R,
    to_engine: &mut W,
    commands: &mut mpsc::Receiver<Command>,
    statuses: &mpsc::Sender<SinteractStatus>,
    changes: &watch::Sender<u64>,
) -> io::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut core = Core::Lobby(Some(LobbyCore::new()));
    let mut read_buffer = vec![0; READ_BYTES];
    let mut engine_open = true;
    let mut ready_sent = false;
    let mut ticker = tokio::time::interval(TICK_PERIOD);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(command) = command else {
                    close_core(&mut core, to_engine).await?;
                    return Ok(());
                };
                if handle_command(command, &mut core, to_engine, changes).await? {
                    return Ok(());
                }
            }
            read = from_engine.read(&mut read_buffer), if engine_open => {
                let read = read?;
                if read == 0 {
                    engine_open = false;
                    match &mut core {
                        Core::Lobby(_) => {
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "Sinteract engine ended before start",
                            ));
                        }
                        Core::Running { server, .. } => {
                            if let Some(error) = server.engine_ended() {
                                return Err(io::Error::new(io::ErrorKind::InvalidData, error));
                            }
                            notify(changes);
                            let _ = statuses.send(SinteractStatus::Ended).await;
                        }
                    }
                    continue;
                }

                let bytes = read_buffer.get(..read).expect("a read fits its buffer");
                match &mut core {
                    Core::Lobby(lobby_slot) => {
                        let lobby = lobby_slot.take().ok_or_else(|| {
                            io::Error::other("Sinteract lobby core is unavailable")
                        })?;
                        let lobby = lobby
                            .from_engine(bytes)
                            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                        if let Some(range) = lobby.players() {
                            validate_range(spec, range)?;
                            if !ready_sent {
                                statuses
                                    .send(SinteractStatus::Ready)
                                    .await
                                    .map_err(|_| io::Error::new(
                                        io::ErrorKind::BrokenPipe,
                                        "Sinteract status receiver closed",
                                    ))?;
                                ready_sent = true;
                            }
                        }
                        *lobby_slot = Some(lobby);
                    }
                    Core::Running { server, .. } => {
                        for error in server.from_engine(bytes) {
                            tracing::warn!(%error, "Sinteract descartou saída inválida da engine");
                        }
                        flush_engine(server, to_engine).await?;
                        notify(changes);
                    }
                }
            }
            _ = ticker.tick(), if matches!(core, Core::Running { .. }) && engine_open => {
                if let Core::Running { server, .. } = &mut core {
                    server.tick();
                    flush_engine(server, to_engine).await?;
                }
            }
        }
    }
}

async fn handle_command<W>(
    command: Command,
    core: &mut Core,
    to_engine: &mut W,
    changes: &watch::Sender<u64>,
) -> io::Result<bool>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    match command {
        Command::Start { players, reply } => {
            let result = start_core(core, players);
            if result.is_ok()
                && let Core::Running { server, .. } = core
            {
                flush_engine(server, to_engine).await?;
            }
            let _ = reply.send(result);
        }
        Command::Connect { player_id, reply } => {
            let result = match core {
                Core::Lobby(_) => Err(SinteractCommandError::NotReady),
                Core::Running { server, players } => players
                    .get(&player_id)
                    .copied()
                    .ok_or(SinteractCommandError::UnknownPlayer)
                    .and_then(|player| {
                        server
                            .connect(player)
                            .ok_or(SinteractCommandError::EngineEnded)
                    }),
            };
            if result.is_ok() {
                if let Core::Running { server, .. } = core {
                    flush_engine(server, to_engine).await?;
                }
                notify(changes);
            }
            let _ = reply.send(result);
        }
        Command::FromView {
            conn,
            payload,
            reply,
        } => {
            let result = match core {
                Core::Lobby(_) => Err(SinteractCommandError::NotReady),
                Core::Running { server, .. } if server.is_over() => {
                    Err(SinteractCommandError::EngineEnded)
                }
                Core::Running { server, .. } => server
                    .from_view(conn, &payload)
                    .map_err(|_| SinteractCommandError::InvalidView),
            };
            if result.is_ok()
                && let Core::Running { server, .. } = core
            {
                flush_engine(server, to_engine).await?;
            }
            let _ = reply.send(result);
        }
        Command::Next { conn, reply } => {
            let result = match core {
                Core::Lobby(_) => Err(SinteractCommandError::NotReady),
                Core::Running { server, .. } => Ok(match server.next_for(conn) {
                    Next::Send(payload) => SinteractOutput::Send(payload),
                    Next::Idle => SinteractOutput::Idle,
                    Next::Gone => SinteractOutput::Gone,
                }),
            };
            let _ = reply.send(result);
        }
        Command::Leave { conn, reply } => {
            let result = match core {
                Core::Lobby(_) => Err(SinteractCommandError::NotReady),
                Core::Running { server, .. } => {
                    server.leave(conn);
                    flush_engine(server, to_engine).await?;
                    notify(changes);
                    Ok(())
                }
            };
            let _ = reply.send(result);
        }
        Command::Stop { reply } => {
            close_core(core, to_engine).await?;
            let _ = reply.send(Ok(()));
            return Ok(true);
        }
    }
    Ok(false)
}

fn start_core(
    core: &mut Core,
    requested: Vec<SinteractPlayer>,
) -> Result<(), SinteractCommandError> {
    if matches!(core, Core::Running { .. }) {
        return Err(SinteractCommandError::AlreadyStarted);
    }
    let unique: HashSet<_> = requested.iter().map(|player| player.id).collect();
    if unique.len() != requested.len() {
        return Err(SinteractCommandError::DuplicatePlayer);
    }

    let Core::Lobby(lobby_slot) = core else {
        return Err(SinteractCommandError::AlreadyStarted);
    };
    let lobby = lobby_slot.take().ok_or(SinteractCommandError::NotReady)?;
    if lobby.players().is_none() {
        *lobby_slot = Some(lobby);
        return Err(SinteractCommandError::NotReady);
    }

    let nicknames: Vec<_> = requested
        .iter()
        .map(|player| player.nickname.as_str())
        .collect();
    let server = match lobby.start(&nicknames, TICK_RATE) {
        Ok(server) => server,
        Err((lobby, _)) => {
            *lobby_slot = Some(lobby);
            return Err(SinteractCommandError::InvalidPlayers);
        }
    };
    let players = requested
        .iter()
        .zip(server.players())
        .map(|(requested, (player, _))| (requested.id, player))
        .collect();
    *core = Core::Running { server, players };
    Ok(())
}

async fn close_core<W>(core: &mut Core, to_engine: &mut W) -> io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    if let Core::Running { server, .. } = core {
        server.close();
        flush_engine(server, to_engine).await?;
    }
    to_engine.shutdown().await
}

async fn flush_engine<W>(server: &mut ServerCore, writer: &mut W) -> io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut output = Vec::new();
    if !server.take_engine_output(&mut output) || output.is_empty() {
        return Ok(());
    }
    writer.write_all(&output).await?;
    writer.flush().await
}

fn validate_range(spec: &RuntimeSpec, range: PlayerRange) -> io::Result<()> {
    let min = i64::from(range.min().get());
    let max = i64::from(range.max().get());
    if min == spec.min_players() && max == spec.max_players() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Sinteract hello declared {min}..{max} players, approved revision declares {}..{}",
                spec.min_players(),
                spec.max_players(),
            ),
        ))
    }
}

fn notify(changes: &watch::Sender<u64>) {
    let next = changes.borrow().wrapping_add(1);
    changes.send_replace(next);
}

#[cfg(unix)]
pub fn run_stub_from_fds() -> io::Result<()> {
    use std::{fs::File, os::fd::FromRawFd};

    use sinteract::{
        scene::Scene,
        session::{ENGINE_TO_SERVER_FD, SERVER_TO_ENGINE_FD, Session, SessionEvent, Target},
    };

    let mut args = std::env::args().skip(2);
    let min = parse_player_limit(args.next(), "minimum")?;
    let max = parse_player_limit(args.next(), "maximum")?;
    let range = PlayerRange::new(min, max).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "invalid fixture player range")
    })?;

    // SAFETY: this process is launched specifically as a Sinteract engine,
    // and the parent maps its unique pipe ends to the protocol descriptors.
    let from_server = unsafe { File::from_raw_fd(SERVER_TO_ENGINE_FD) };
    // SAFETY: as above; fd 4 is owned by this process and is consumed once.
    let to_server = unsafe { File::from_raw_fd(ENGINE_TO_SERVER_FD) };
    let (mut session, players) = Session::start(range, from_server, to_server)
        .map_err(|error| io::Error::other(error.to_string()))?;

    loop {
        match session.wait()? {
            SessionEvent::Tick => {
                for (player, _) in &players {
                    session
                        .write_frame(Target::Player(*player), &Scene::new(320.0, 240.0))
                        .map_err(|error| io::Error::other(error.to_string()))?;
                }
            }
            SessionEvent::End => return Ok(()),
            SessionEvent::Input { .. } | SessionEvent::Error(_) => {}
        }
    }
}

#[cfg(not(unix))]
pub fn run_stub_from_fds() -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Sinteract fd sessions require Unix",
    ))
}

#[cfg(unix)]
fn parse_player_limit(value: Option<String>, name: &str) -> io::Result<u32> {
    value
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("missing {name}")))?
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {name}")))
}
