use std::{
    collections::HashMap,
    sync::MutexGuard,
    time::{Duration, Instant},
};

use rand::{distributions::Alphanumeric, Rng};
use tokio::sync::broadcast;

use backend::game_runtime::{
    RuntimeCommand, RuntimeHandle, RuntimeInputEvent, RuntimeOutput, RuntimePlayer,
    RuntimeSpec,
};

use crate::rooms::{
    model::{
        InputEvent, Player, Room, RoomError, RoomSnapshot, RoomStatus, ServerMessage,
    },
    store::RoomStore,
};

const EMPTY_ROOM_TTL: Duration = Duration::from_secs(10 * 60);
const DISCONNECTED_ROOM_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_NICKNAME_LENGTH: usize = 30;

pub struct CreatedRoom {
    pub code: String,
    pub host_token: String,
}

pub struct JoinRoom {
    pub player_id: u64,
    pub connection_generation: u64,
    pub is_host: bool,
    pub player_token: String,
    pub room: RoomSnapshot,
    pub rx: broadcast::Receiver<ServerMessage>,
}

pub fn create_room(
    store: &RoomStore,
    spec: RuntimeSpec,
) -> Result<CreatedRoom, RoomError> {
    let (runtime, runtime_outputs) = RuntimeHandle::reserve(spec.clone());
    create_room_with_runtime(store, spec, runtime, runtime_outputs)
}

fn create_room_with_runtime(
    store: &RoomStore,
    spec: RuntimeSpec,
    runtime: RuntimeHandle,
    runtime_outputs: tokio::sync::mpsc::Receiver<RuntimeOutput>,
) -> Result<CreatedRoom, RoomError> {
    let mut rooms = lock_rooms(store)?;

    cleanup_empty_expired_rooms(&mut rooms);

    let code = generate_unique_room_code(&rooms);
    let host_token = random_token(32);
    let created_at = Instant::now();

    let (tx, _) = broadcast::channel(100);

    let room = Room {
        code: code.clone(),
        spec,
        status: RoomStatus::Preparing,
        host_token: host_token.clone(),
        host_player_id: None,
        next_player_id: 1,
        players: Vec::new(),
        tx: tx.clone(),
        runtime,
        created_at,
        started_at: None,
        all_disconnected_at: None,
    };

    rooms.insert(code.clone(), room);
    drop(rooms);

    forward_runtime_outputs(store.clone(), code.clone(), runtime_outputs, tx);
    schedule_empty_room_expiration(store.clone(), code.clone(), created_at);

    Ok(CreatedRoom { code, host_token })
}

pub fn get_room_snapshot(
    store: &RoomStore,
    code: &str,
) -> Result<RoomSnapshot, RoomError> {
    let mut rooms = lock_rooms(store)?;

    cleanup_empty_expired_rooms(&mut rooms);

    let room = rooms.get(code).ok_or(RoomError::NotFound)?;

    Ok(room_snapshot(room))
}

pub fn join_room(
    store: &RoomStore,
    code: &str,
    nickname: String,
    host_token: Option<String>,
    player_token: Option<String>,
) -> Result<JoinRoom, RoomError> {
    let mut rooms = lock_rooms(store)?;

    cleanup_empty_expired_rooms(&mut rooms);

    let room = rooms.get_mut(code).ok_or(RoomError::NotFound)?;

    if let Some(player_token) = player_token {
        return reconnect_player(room, &player_token);
    }

    let nickname = validate_nickname(&nickname)?;

    match room.status {
        RoomStatus::Preparing | RoomStatus::Waiting => {}
        RoomStatus::Running => return Err(RoomError::AlreadyRunning),
        RoomStatus::Failed => return Err(RoomError::RuntimeUnavailable),
    }

    if room.players.len() >= room.spec.max_players() as usize {
        return Err(RoomError::Full);
    }

    if room
        .players
        .iter()
        .any(|player| player.nickname.eq_ignore_ascii_case(&nickname))
    {
        return Err(RoomError::NicknameTaken);
    }

    let is_host = host_token
        .as_deref()
        .is_some_and(|token| token == room.host_token);

    let player_id = room.next_player_id;
    room.next_player_id += 1;

    if is_host {
        for player in &mut room.players {
            player.is_host = false;
        }

        room.host_player_id = Some(player_id);
    }

    let player = Player {
        id: player_id,
        nickname,
        is_host,
        connected: true,
        reconnect_token: random_token(32),
        connection_generation: 1,
    };

    let player_token = player.reconnect_token.clone();
    let connection_generation = player.connection_generation;

    room.players.push(player);

    let rx = room.tx.subscribe();
    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::RoomState {
        room: snapshot.clone(),
    });

    Ok(JoinRoom {
        player_id,
        connection_generation,
        is_host,
        player_token,
        room: snapshot,
        rx,
    })
}

fn reconnect_player(room: &mut Room, player_token: &str) -> Result<JoinRoom, RoomError> {
    let Some(player) = room
        .players
        .iter_mut()
        .find(|player| player.reconnect_token == player_token)
    else {
        return Err(RoomError::InvalidPlayerToken);
    };

    let player_id = player.id;
    let is_host = player.is_host;
    let was_connected = player.connected;
    let previous_generation = player.connection_generation;
    player.connection_generation = player
        .connection_generation
        .checked_add(1)
        .ok_or(RoomError::Internal)?;
    player.connected = true;
    room.all_disconnected_at = None;
    let connection_generation = player.connection_generation;
    let player_token = player.reconnect_token.clone();

    let rx = room.tx.subscribe();
    let snapshot = room_snapshot(room);

    if was_connected {
        let _ = room.tx.send(ServerMessage::ConnectionReplaced {
            player_id,
            connection_generation: previous_generation,
        });
    }

    let _ = room.tx.send(ServerMessage::RoomState {
        room: snapshot.clone(),
    });

    Ok(JoinRoom {
        player_id,
        connection_generation,
        is_host,
        player_token,
        room: snapshot,
        rx,
    })
}

pub fn validate_nickname(nickname: &str) -> Result<String, RoomError> {
    let nickname = nickname.trim();

    if nickname.is_empty() {
        return Err(RoomError::NicknameRequired);
    }

    if nickname.chars().count() > MAX_NICKNAME_LENGTH {
        return Err(RoomError::NicknameTooLong);
    }

    Ok(nickname.to_string())
}

pub fn leave_room(
    store: &RoomStore,
    code: &str,
    player_id: u64,
    connection_generation: u64,
) -> Result<(), RoomError> {
    let mut rooms = lock_rooms(store)?;

    let Some(room) = rooms.get_mut(code) else {
        return Ok(());
    };

    let Some(player) = room.players.iter_mut().find(|player| player.id == player_id) else {
        return Ok(());
    };

    if player.connection_generation != connection_generation {
        return Ok(());
    }

    if room.status == RoomStatus::Running {
        player.connected = false;
        let _ = room.runtime.send(RuntimeCommand::PlayerLeft { player_id });

        let disconnected_at = room
            .players
            .iter()
            .all(|player| !player.connected)
            .then(Instant::now);
        if let Some(disconnected_at) = disconnected_at {
            room.all_disconnected_at = Some(disconnected_at);
        }

        let snapshot = room_snapshot(room);
        let _ = room.tx.send(ServerMessage::PlayerLeft {
            player_id,
            room: snapshot,
        });

        if let Some(disconnected_at) = disconnected_at {
            drop(rooms);
            schedule_disconnected_room_expiration(
                store.clone(),
                code.to_string(),
                disconnected_at,
            );
        }
        return Ok(());
    }

    room.players.retain(|player| player.id != player_id);

    if room.host_player_id == Some(player_id) {
        room.host_player_id = None;

        for player in &mut room.players {
            player.is_host = false;
        }
    }

    if room.players.is_empty() {
        let _ = room.runtime.send(RuntimeCommand::Stop);
        rooms.remove(code);
        return Ok(());
    }

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::PlayerLeft {
        player_id,
        room: snapshot,
    });

    Ok(())
}

pub fn start_room(
    store: &RoomStore,
    code: &str,
    player_id: u64,
    connection_generation: u64,
) -> Result<(), RoomError> {
    let mut rooms = lock_rooms(store)?;
    let room = rooms.get_mut(code).ok_or(RoomError::NotFound)?;

    ensure_current_connection(room, player_id, connection_generation)?;
    ensure_host(room, player_id)?;

    match room.status {
        RoomStatus::Preparing => return Err(RoomError::RuntimePreparing),
        RoomStatus::Waiting => {}
        RoomStatus::Running => return Err(RoomError::AlreadyRunning),
        RoomStatus::Failed => return Err(RoomError::RuntimeUnavailable),
    }

    if room.players.len() < room.spec.min_players() as usize {
        return Err(RoomError::InsufficientPlayers);
    }

    room.runtime
        .send(RuntimeCommand::Start {
            game_id: room.spec.game_id(),
            players: room
                .players
                .iter()
                .map(|player| RuntimePlayer {
                    id: player.id,
                    nickname: player.nickname.clone(),
                })
                .collect(),
        })
        .map_err(|_| RoomError::RuntimeUnavailable)?;

    room.status = RoomStatus::Running;
    room.started_at = Some(Instant::now());

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::RoomStarted {
        room: snapshot,
    });

    Ok(())
}

pub fn kick_player(
    store: &RoomStore,
    code: &str,
    requester_id: u64,
    connection_generation: u64,
    target_id: u64,
) -> Result<(), RoomError> {
    let mut rooms = lock_rooms(store)?;
    let room = rooms.get_mut(code).ok_or(RoomError::NotFound)?;

    ensure_current_connection(room, requester_id, connection_generation)?;
    ensure_host(room, requester_id)?;

    if room.status == RoomStatus::Running {
        return Err(RoomError::AlreadyRunning);
    }

    if requester_id == target_id {
        return Err(RoomError::CannotKickSelf);
    }

    if !room.players.iter().any(|player| player.id == target_id) {
        return Err(RoomError::PlayerNotFound);
    }

    room.players.retain(|player| player.id != target_id);

    if room.status == RoomStatus::Running {
        let _ = room.runtime.send(RuntimeCommand::PlayerLeft {
            player_id: target_id,
        });
    }

    if room.host_player_id == Some(target_id) {
        room.host_player_id = None;
    }

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::PlayerKicked {
        player_id: target_id,
        room: snapshot.clone(),
    });

    let _ = room.tx.send(ServerMessage::RoomState {
        room: snapshot,
    });

    Ok(())
}

pub fn promote_host(
    store: &RoomStore,
    code: &str,
    requester_id: u64,
    connection_generation: u64,
    target_id: u64,
) -> Result<(), RoomError> {
    let mut rooms = lock_rooms(store)?;
    let room = rooms.get_mut(code).ok_or(RoomError::NotFound)?;

    ensure_current_connection(room, requester_id, connection_generation)?;
    ensure_host(room, requester_id)?;

    if !room.players.iter().any(|player| player.id == target_id) {
        return Err(RoomError::PlayerNotFound);
    }

    room.host_player_id = Some(target_id);

    for player in &mut room.players {
        player.is_host = player.id == target_id;
    }

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::HostChanged {
        host_player_id: target_id,
        room: snapshot.clone(),
    });

    let _ = room.tx.send(ServerMessage::RoomState {
        room: snapshot,
    });

    Ok(())
}

pub fn broadcast_input(
    store: &RoomStore,
    code: &str,
    player_id: u64,
    connection_generation: u64,
    event: InputEvent,
) -> Result<(), RoomError> {
    let rooms = lock_rooms(store)?;
    let room = rooms.get(code).ok_or(RoomError::NotFound)?;

    ensure_current_connection(room, player_id, connection_generation)?;

    if room.status != RoomStatus::Running {
        return Err(RoomError::NotRunning);
    }

    room.runtime
        .send(RuntimeCommand::Input {
            player_id,
            event: runtime_input_event(event),
        })
        .map_err(|_| RoomError::RuntimeUnavailable)?;

    Ok(())
}

fn runtime_input_event(event: InputEvent) -> RuntimeInputEvent {
    match event {
        InputEvent::KeyDown { key } => RuntimeInputEvent::KeyDown { key },
        InputEvent::KeyUp { key } => RuntimeInputEvent::KeyUp { key },
        InputEvent::PadDown { input } => RuntimeInputEvent::PadDown { input },
        InputEvent::PadUp { input } => RuntimeInputEvent::PadUp { input },
    }
}

fn forward_runtime_outputs(
    store: RoomStore,
    code: String,
    mut outputs: tokio::sync::mpsc::Receiver<RuntimeOutput>,
    tx: broadcast::Sender<ServerMessage>,
) {
    tokio::spawn(async move {
        let mut first_frame_logged = false;

        while let Some(output) = outputs.recv().await {
            let message = match output {
                RuntimeOutput::Ready => {
                    let snapshot = {
                        let Ok(mut rooms) = store.lock() else {
                            tracing::error!(room = %code, "mutex de salas envenenado");
                            return;
                        };
                        let Some(room) = rooms.get_mut(&code) else {
                            return;
                        };
                        if room.status != RoomStatus::Preparing {
                            continue;
                        }
                        room.status = RoomStatus::Waiting;
                        room_snapshot(room)
                    };

                    tracing::info!(room = %code, "runtime da sala está pronto");
                    let _ = tx.send(ServerMessage::RoomState { room: snapshot });
                    continue;
                }
                RuntimeOutput::Frame { player_id, svg } => match String::from_utf8(svg) {
                    Ok(svg) if svg.trim_start().starts_with("<svg") => {
                        if !first_frame_logged
                            && let Some((elapsed, revision_id, runtime_build)) =
                                first_frame_metrics(&store, &code)
                        {
                            tracing::info!(
                                room = %code,
                                revision_id,
                                runtime_build,
                                start_to_first_frame_ms = elapsed.as_millis(),
                                "runtime produziu o primeiro frame"
                            );
                            first_frame_logged = true;
                        }
                        ServerMessage::Frame { player_id, svg }
                    }
                    Ok(_) => ServerMessage::Error {
                        message: "Runtime respondeu um frame que não é SVG".into(),
                    },
                    Err(_) => ServerMessage::Error {
                        message: "Runtime respondeu um SVG com UTF-8 inválido".into(),
                    },
                },
                RuntimeOutput::Failed(error) => {
                    let snapshot = {
                        let Ok(mut rooms) = store.lock() else {
                            tracing::error!(room = %code, "mutex de salas envenenado");
                            return;
                        };
                        let Some(room) = rooms.get_mut(&code) else {
                            return;
                        };
                        room.status = RoomStatus::Failed;
                        room_snapshot(room)
                    };

                    tracing::error!(room = %code, %error, "runtime da partida falhou");
                    let _ = tx.send(ServerMessage::RoomState { room: snapshot });
                    ServerMessage::Error {
                        message: "Runtime da partida foi encerrado".into(),
                    }
                }
            };

            let _ = tx.send(message);
        }
    });
}

fn first_frame_metrics(
    store: &RoomStore,
    code: &str,
) -> Option<(Duration, i64, String)> {
    let rooms = store.lock().ok()?;
    let room = rooms.get(code)?;
    let started_at = room.started_at?;

    Some((
        started_at.elapsed(),
        room.spec.revision_id(),
        room.spec.runtime_build_id().to_string(),
    ))
}

fn ensure_host(room: &Room, player_id: u64) -> Result<(), RoomError> {
    if room.host_player_id == Some(player_id) {
        Ok(())
    } else {
        Err(RoomError::NotHost)
    }
}

fn ensure_current_connection(
    room: &Room,
    player_id: u64,
    connection_generation: u64,
) -> Result<(), RoomError> {
    let player = room
        .players
        .iter()
        .find(|player| player.id == player_id)
        .ok_or(RoomError::PlayerNotFound)?;

    if player.connected && player.connection_generation == connection_generation {
        Ok(())
    } else {
        Err(RoomError::ConnectionReplaced)
    }
}

fn room_snapshot(room: &Room) -> RoomSnapshot {
    RoomSnapshot {
        code: room.code.clone(),
        game_id: room.spec.game_id(),
        status: room.status.clone(),
        min_players: room.spec.min_players(),
        max_players: room.spec.max_players(),
        players: room.players.clone(),
        can_start: room.host_player_id.is_some()
            && room.status == RoomStatus::Waiting
            && room.players.len() >= room.spec.min_players() as usize,
    }
}

fn lock_rooms<'a>(
    store: &'a RoomStore,
) -> Result<MutexGuard<'a, HashMap<String, Room>>, RoomError> {
    store.lock().map_err(|_| RoomError::Internal)
}

fn cleanup_empty_expired_rooms(rooms: &mut HashMap<String, Room>) {
    let now = Instant::now();

    rooms.retain(|_, room| {
        !room.players.is_empty()
            || now.duration_since(room.created_at) < EMPTY_ROOM_TTL
    });
}

fn schedule_empty_room_expiration(store: RoomStore, code: String, created_at: Instant) {
    tokio::spawn(expire_empty_room_after(
        store,
        code,
        created_at,
        EMPTY_ROOM_TTL,
    ));
}

async fn expire_empty_room_after(
    store: RoomStore,
    code: String,
    created_at: Instant,
    ttl: Duration,
) {
    tokio::time::sleep(ttl).await;

    let expired = {
        let Ok(mut rooms) = store.lock() else {
            tracing::error!(room = %code, "mutex de salas envenenado durante expiração");
            return;
        };
        let should_remove = rooms.get(&code).is_some_and(|room| {
            room.created_at == created_at && room.players.is_empty()
        });

        should_remove.then(|| rooms.remove(&code)).flatten()
    };

    if let Some(room) = expired {
        let _ = room.runtime.send(RuntimeCommand::Stop);
        tracing::info!(room = %code, "sala vazia expirada e runtime encerrado");
    }
}

fn schedule_disconnected_room_expiration(
    store: RoomStore,
    code: String,
    disconnected_at: Instant,
) {
    tokio::spawn(expire_disconnected_room_after(
        store,
        code,
        disconnected_at,
        DISCONNECTED_ROOM_TTL,
    ));
}

async fn expire_disconnected_room_after(
    store: RoomStore,
    code: String,
    disconnected_at: Instant,
    ttl: Duration,
) {
    tokio::time::sleep(ttl).await;

    let expired = {
        let Ok(mut rooms) = store.lock() else {
            tracing::error!(room = %code, "mutex de salas envenenado durante expiração");
            return;
        };
        let should_remove = rooms.get(&code).is_some_and(|room| {
            room.status == RoomStatus::Running
                && room.all_disconnected_at == Some(disconnected_at)
                && room.players.iter().all(|player| !player.connected)
        });

        should_remove.then(|| rooms.remove(&code)).flatten()
    };

    if let Some(room) = expired {
        let _ = room.runtime.send(RuntimeCommand::Stop);
        tracing::info!(
            room = %code,
            "partida sem conexões expirou e o runtime foi encerrado"
        );
    }
}

fn generate_unique_room_code(rooms: &HashMap<String, Room>) -> String {
    loop {
        let code = random_token(6).to_uppercase();

        if !rooms.contains_key(&code) {
            return code;
        }
    }
}

fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use backend::game_runtime::{RuntimeHandle, RuntimeOutput, RuntimeSpec};

    use super::{
        CreatedRoom, broadcast_input, create_room_with_runtime, expire_disconnected_room_after,
        expire_empty_room_after, join_room, leave_room, start_room, validate_nickname,
    };
    use crate::rooms::{InputEvent, RoomError, RoomStatus, ServerMessage, new_store};

    #[test]
    fn nickname_is_trimmed_and_limited_on_the_server() {
        assert_eq!(validate_nickname("  Jogador  ").unwrap(), "Jogador");
        assert!(matches!(
            validate_nickname("   "),
            Err(RoomError::NicknameRequired)
        ));
        assert!(validate_nickname(&"á".repeat(30)).is_ok());
        assert!(matches!(
            validate_nickname(&"á".repeat(31)),
            Err(RoomError::NicknameTooLong)
        ));
    }

    #[tokio::test]
    async fn created_room_keeps_the_selected_revision_and_source() {
        let store = new_store();
        let spec = runtime_spec();
        let expected_digest = *spec.source_sha256();

        let created = create_test_room(&store, spec);
        let rooms = store.lock().unwrap();
        let room = rooms.get(&created.code).unwrap();

        assert_eq!(room.spec.revision_id(), 17);
        assert_eq!(room.spec.runtime_build_id(), "sgleam-0.7");
        assert_eq!(room.spec.source_code(), "pub fn main() { 42 }");
        assert_eq!(room.spec.source_sha256(), &expected_digest);
    }

    #[tokio::test]
    async fn running_room_keeps_the_seat_and_reconnects_with_its_token() {
        let store = new_store();
        let created = create_test_room(&store, runtime_spec());
        let joined = join_room(
            &store,
            &created.code,
            "Jogadora".into(),
            Some(created.host_token),
            None,
        )
        .unwrap();

        {
            let mut rooms = store.lock().unwrap();
            rooms.get_mut(&created.code).unwrap().status = RoomStatus::Running;
        }

        leave_room(
            &store,
            &created.code,
            joined.player_id,
            joined.connection_generation,
        )
        .unwrap();

        let disconnected_at = {
            let rooms = store.lock().unwrap();
            let room = rooms.get(&created.code).unwrap();
            let player = &room.players[0];
            assert_eq!(player.id, joined.player_id);
            assert!(!player.connected);
            room.all_disconnected_at.unwrap()
        };

        let reconnected = join_room(
            &store,
            &created.code,
            String::new(),
            None,
            Some(joined.player_token.clone()),
        )
        .unwrap();

        assert_eq!(reconnected.player_id, joined.player_id);
        assert_eq!(reconnected.player_token, joined.player_token);
        assert!(reconnected.connection_generation > joined.connection_generation);

        expire_disconnected_room_after(
            store.clone(),
            created.code.clone(),
            disconnected_at,
            Duration::ZERO,
        )
        .await;
        assert!(store.lock().unwrap().contains_key(&created.code));

        let old_connection = broadcast_input(
            &store,
            &created.code,
            joined.player_id,
            joined.connection_generation,
            InputEvent::KeyDown { key: "a".into() },
        );
        assert!(matches!(old_connection, Err(RoomError::ConnectionReplaced)));
    }

    #[tokio::test]
    async fn running_room_rejects_a_new_player_without_a_valid_token() {
        let store = new_store();
        let created = create_test_room(&store, runtime_spec());

        {
            let mut rooms = store.lock().unwrap();
            rooms.get_mut(&created.code).unwrap().status = RoomStatus::Running;
        }

        assert!(matches!(
            join_room(&store, &created.code, "Nova".into(), None, None),
            Err(RoomError::AlreadyRunning)
        ));
        assert!(matches!(
            join_room(
                &store,
                &created.code,
                "Nova".into(),
                None,
                Some("token-inexistente".into()),
            ),
            Err(RoomError::InvalidPlayerToken)
        ));
    }

    #[tokio::test]
    async fn runtime_ready_transitions_the_room_from_preparing_to_waiting() {
        let store = new_store();
        let (runtime, runtime_output, outputs) = RuntimeHandle::simulated();
        let created = create_room_with_runtime(
            &store,
            runtime_spec(),
            runtime,
            outputs,
        )
        .unwrap();
        let mut events = {
            let rooms = store.lock().unwrap();
            let room = rooms.get(&created.code).unwrap();
            assert!(matches!(room.status, RoomStatus::Preparing));
            room.tx.subscribe()
        };

        runtime_output.send(RuntimeOutput::Ready).await.unwrap();
        let event = tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(
            event,
            ServerMessage::RoomState {
                room: crate::rooms::RoomSnapshot {
                    status: RoomStatus::Waiting,
                    ..
                }
            }
        ));
    }

    #[tokio::test]
    async fn start_is_rejected_until_the_runtime_is_ready() {
        let store = new_store();
        let (runtime, _runtime_output, outputs) = RuntimeHandle::simulated();
        let created = create_room_with_runtime(
            &store,
            runtime_spec(),
            runtime,
            outputs,
        )
        .unwrap();
        let joined = join_room(
            &store,
            &created.code,
            "Host".into(),
            Some(created.host_token),
            None,
        )
        .unwrap();

        let result = start_room(
            &store,
            &created.code,
            joined.player_id,
            joined.connection_generation,
        );

        assert!(matches!(result, Err(RoomError::RuntimePreparing)));
    }

    #[tokio::test]
    async fn runtime_failure_marks_the_room_and_blocks_new_players() {
        let store = new_store();
        let (runtime, runtime_output, outputs) = RuntimeHandle::simulated();
        let created = create_room_with_runtime(
            &store,
            runtime_spec(),
            runtime,
            outputs,
        )
        .unwrap();
        let mut events = {
            let rooms = store.lock().unwrap();
            rooms.get(&created.code).unwrap().tx.subscribe()
        };

        runtime_output
            .send(RuntimeOutput::Failed("fixture encerrou".into()))
            .await
            .unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(
            join_room(&store, &created.code, "Nova".into(), None, None),
            Err(RoomError::RuntimeUnavailable)
        ));
        let rooms = store.lock().unwrap();
        assert!(matches!(
            rooms.get(&created.code).unwrap().status,
            RoomStatus::Failed
        ));
    }

    #[tokio::test]
    async fn empty_room_expiration_removes_the_room_and_its_runtime() {
        let store = new_store();
        let (runtime, _runtime_output, outputs) = RuntimeHandle::simulated();
        let created = create_room_with_runtime(
            &store,
            runtime_spec(),
            runtime,
            outputs,
        )
        .unwrap();
        let created_at = store
            .lock()
            .unwrap()
            .get(&created.code)
            .unwrap()
            .created_at;

        expire_empty_room_after(
            store.clone(),
            created.code.clone(),
            created_at,
            Duration::ZERO,
        )
        .await;

        assert!(!store.lock().unwrap().contains_key(&created.code));
    }

    #[tokio::test]
    async fn disconnected_running_room_expires_after_the_reconnection_window() {
        let store = new_store();
        let created = create_test_room(&store, runtime_spec());
        let joined = join_room(
            &store,
            &created.code,
            "Jogadora".into(),
            Some(created.host_token),
            None,
        )
        .unwrap();
        {
            let mut rooms = store.lock().unwrap();
            rooms.get_mut(&created.code).unwrap().status = RoomStatus::Running;
        }

        leave_room(
            &store,
            &created.code,
            joined.player_id,
            joined.connection_generation,
        )
        .unwrap();
        let disconnected_at = store
            .lock()
            .unwrap()
            .get(&created.code)
            .unwrap()
            .all_disconnected_at
            .unwrap();

        expire_disconnected_room_after(
            store.clone(),
            created.code.clone(),
            disconnected_at,
            Duration::ZERO,
        )
        .await;

        assert!(!store.lock().unwrap().contains_key(&created.code));
    }

    fn runtime_spec() -> RuntimeSpec {
        RuntimeSpec::new(
            9,
            17,
            "sgleam-0.7".into(),
            "sgleam".into(),
            "0.7.0".into(),
            Some("abc123".into()),
            Some(2),
            ".gleam".into(),
            "active".into(),
            "pub fn main() { 42 }".into(),
            2,
            6,
        )
    }

    fn create_test_room(
        store: &crate::rooms::RoomStore,
        spec: RuntimeSpec,
    ) -> CreatedRoom {
        let (runtime, outputs) = RuntimeHandle::simulated_ready();
        create_room_with_runtime(store, spec, runtime, outputs).unwrap()
    }
}
