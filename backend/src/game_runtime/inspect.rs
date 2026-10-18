use std::{io, path::PathBuf, process::Stdio, time::Duration};

use sinteract::{
    server::LobbyCore,
    session::{ENGINE_TO_SERVER_FD, PlayerRange, SERVER_TO_ENGINE_FD, SESSION_VAR},
};
use tokio::io::{AsyncRead, AsyncReadExt};

use super::MaterializedSource;

const INSPECTION_TIMEOUT: Duration = Duration::from_secs(15);
const READ_BYTES: usize = 64 * 1024;
const MAX_DIAGNOSTIC_BYTES: usize = 32 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerLimits {
    pub min: u32,
    pub max: u32,
}

/// Runs a source file only until its Sinteract hello declares the player
/// range. No players are admitted and the game loop is never started.
pub async fn inspect_sgleam_players(
    executable: PathBuf,
    source_code: String,
    source_extension: String,
) -> io::Result<PlayerLimits> {
    #[cfg(unix)]
    {
        inspect_unix(executable, source_code, source_extension).await
    }

    #[cfg(not(unix))]
    {
        let _ = (executable, source_code, source_extension);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Sinteract source inspection requires Unix",
        ))
    }
}

#[cfg(unix)]
async fn inspect_unix(
    executable: PathBuf,
    source_code: String,
    source_extension: String,
) -> io::Result<PlayerLimits> {
    use std::{fs::File, io::pipe, os::fd::OwnedFd, process::Command as StdCommand};

    use command_fds::{CommandFdExt, FdMapping};

    let temporary_parent = std::env::temp_dir();
    let materialized = tokio::task::spawn_blocking(move || {
        MaterializedSource::create_raw(&source_code, &source_extension, &temporary_parent)
    })
    .await
    .map_err(|error| io::Error::other(format!("runtime inspection task failed: {error}")))??;

    let (engine_reads, to_engine) = pipe()?;
    let (from_engine, engine_writes) = pipe()?;
    let mut command = StdCommand::new(executable);
    command
        .arg(materialized.source_path())
        .current_dir(materialized.directory())
        .env_clear()
        .env(SESSION_VAR, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
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
    let stderr = child.stderr.take();
    let diagnostic = tokio::spawn(async move {
        match stderr {
            Some(stderr) => read_diagnostic(stderr).await,
            None => String::new(),
        }
    });

    // Keep the server-to-engine pipe alive while waiting for the hello. The
    // engine blocks at mworld.run until the server supplies the player list.
    let _to_engine = tokio::fs::File::from_std(File::from(OwnedFd::from(to_engine)));
    let mut from_engine = tokio::fs::File::from_std(File::from(OwnedFd::from(from_engine)));

    let inspected = match tokio::time::timeout(
        INSPECTION_TIMEOUT,
        read_player_limits(&mut from_engine),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "O jogo demorou demais para declarar o número de jogadores",
        )),
    };

    let _ = child.kill().await;
    let _ = child.wait().await;
    let diagnostic = diagnostic.await.unwrap_or_default();

    inspected.map_err(|error| {
        if diagnostic.trim().is_empty() {
            error
        } else {
            io::Error::new(error.kind(), diagnostic.trim().to_string())
        }
    })
}

async fn read_player_limits(reader: &mut (impl AsyncRead + Unpin)) -> io::Result<PlayerLimits> {
    let mut lobby = LobbyCore::new();
    let mut buffer = vec![0; READ_BYTES];

    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "O jogo terminou antes de declarar o número de jogadores",
            ));
        }

        lobby = lobby
            .from_engine(&buffer[..read])
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        if let Some(range) = lobby.players() {
            return Ok(limits(range));
        }
    }
}

fn limits(range: PlayerRange) -> PlayerLimits {
    PlayerLimits {
        min: range.min().get(),
        max: range.max().get(),
    }
}

async fn read_diagnostic(mut reader: impl AsyncRead + Unpin) -> String {
    let mut captured = Vec::new();
    let mut buffer = [0_u8; 4096];

    loop {
        match reader.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                let remaining = MAX_DIAGNOSTIC_BYTES.saturating_sub(captured.len());
                captured.extend_from_slice(&buffer[..read.min(remaining)]);
            }
        }
    }

    String::from_utf8_lossy(&captured).into_owned()
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Cursor, Write},
        sync::{Arc, Mutex},
    };

    use sinteract::session::{PlayerRange, Session};

    use super::{PlayerLimits, read_player_limits};

    #[tokio::test]
    async fn reads_the_player_range_from_a_sinteract_hello() {
        let range = PlayerRange::new(2, 4).unwrap();
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedWriter(bytes.clone());
        let _ = Session::start(range, Cursor::new(Vec::<u8>::new()), writer);
        let bytes = bytes.lock().unwrap().clone();

        let limits = read_player_limits(&mut Cursor::new(bytes)).await.unwrap();

        assert_eq!(limits, PlayerLimits { min: 2, max: 4 });
    }

    #[derive(Clone)]
    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedWriter {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buffer)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
