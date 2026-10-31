use std::{
    collections::HashMap,
    sync::MutexGuard,
    time::{Duration, Instant},
};

use rand::{Rng, distributions::Alphanumeric};
use tokio::sync::{OwnedSemaphorePermit, broadcast};

use backend::game_runtime::{RuntimeSpec, SinteractHandle, SinteractPlayer, SinteractStatus};

use crate::rooms::{
    model::{Player, Room, RoomError, RoomSnapshot, RoomStatus, ServerMessage},
    store::RoomStore,
};

const EMPTY_ROOM_TTL: Duration = Duration::from_secs(10 * 60);
const DISCONNECTED_ROOM_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_NICKNAME_LENGTH: usize = 30;

pub struct CreatedRoom {
    pub code: String,
    pub host_token: String,
    pub invite_token: Option<String>,
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
    executable: std::path::PathBuf,
    runtime_permit: OwnedSemaphorePermit,
) -> Result<CreatedRoom, RoomError> {
    let (runtime, statuses) = SinteractHandle::reserve(spec.clone(), executable);
    create_room_with_runtime(store, spec, runtime, statuses, false, Some(runtime_permit))
}

pub fn create_preview_room(
    store: &RoomStore,
    spec: RuntimeSpec,
    executable: std::path::PathBuf,
    runtime_permit: OwnedSemaphorePermit,
) -> Result<CreatedRoom, RoomError> {
    let (runtime, statuses) = SinteractHandle::reserve(spec.clone(), executable);
    create_room_with_runtime(store, spec, runtime, statuses, true, Some(runtime_permit))
}

fn create_room_with_runtime(
    store: &RoomStore,
    spec: RuntimeSpec,
    runtime: SinteractHandle,
    statuses: tokio::sync::mpsc::Receiver<SinteractStatus>,
    is_preview: bool,
    runtime_permit: Option<OwnedSemaphorePermit>,
) -> Result<CreatedRoom, RoomError> {
    let changes = runtime.subscribe();
    insert_room(
        store,
        spec,
        runtime,
        runtime_permit,
        is_preview,
        |store, code, tx| {
            forward_sinteract_statuses(store, code, statuses, tx.clone());
            forward_sinteract_changes(changes, tx);
        },
    )
}

fn insert_room(
    store: &RoomStore,
    spec: RuntimeSpec,
    runtime: SinteractHandle,
    runtime_permit: Option<OwnedSemaphorePermit>,
    is_preview: bool,
    forward: impl FnOnce(RoomStore, String, broadcast::Sender<ServerMessage>),
) -> Result<CreatedRoom, RoomError> {
    let mut rooms = lock_rooms(store)?;

    cleanup_empty_expired_rooms(&mut rooms);

    let code = generate_unique_room_code(&rooms);
    let host_token = random_token(32);
    let invite_token = is_preview.then(|| random_token(32));
    let created_at = Instant::now();

    let (tx, _) = broadcast::channel(100);

    let room = Room {
        code: code.clone(),
        spec,
        status: RoomStatus::Preparing,
        host_token: host_token.clone(),
        invite_token: invite_token.clone(),
        host_player_id: None,
        next_player_id: 1,
        players: Vec::new(),
        tx: tx.clone(),
        runtime,
        _runtime_permit: runtime_permit,
        created_at,
        all_disconnected_at: None,
    };

    rooms.insert(code.clone(), room);
    drop(rooms);

    forward(store.clone(), code.clone(), tx);
    schedule_empty_room_expiration(store.clone(), code.clone(), created_at);

    Ok(CreatedRoom {
        code,
        host_token,
        invite_token,
    })
}

pub fn get_room_snapshot_with_access(
    store: &RoomStore,
    code: &str,
    invite_token: Option<&str>,
    player_token: Option<&str>,
) -> Result<RoomSnapshot, RoomError> {
    let mut rooms = lock_rooms(store)?;

    cleanup_empty_expired_rooms(&mut rooms);

    let room = rooms.get(code).ok_or(RoomError::NotFound)?;

    let is_returning_player = player_token.is_some_and(|token| {
        room.players
            .iter()
            .any(|player| player.reconnect_token == token)
    });
    if !is_returning_player {
        ensure_invited(room, invite_token, None)?;
    }

    Ok(room_snapshot(room))
}

#[cfg(test)]
pub fn join_room(
    store: &RoomStore,
    code: &str,
    nickname: String,
    host_token: Option<String>,
    player_token: Option<String>,
) -> Result<JoinRoom, RoomError> {
    join_room_with_invite(store, code, nickname, host_token, player_token, None)
}

pub fn join_room_with_invite(
    store: &RoomStore,
    code: &str,
    nickname: String,
    host_token: Option<String>,
    player_token: Option<String>,
    invite_token: Option<String>,
) -> Result<JoinRoom, RoomError> {
    let mut rooms = lock_rooms(store)?;

    cleanup_empty_expired_rooms(&mut rooms);

    let room = rooms.get_mut(code).ok_or(RoomError::NotFound)?;

    if let Some(player_token) = player_token {
        return reconnect_player(room, &player_token);
    }

    ensure_invited(room, invite_token.as_deref(), host_token.as_deref())?;

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

fn ensure_invited(
    room: &Room,
    invite_token: Option<&str>,
    host_token: Option<&str>,
) -> Result<(), RoomError> {
    let Some(expected) = room.invite_token.as_deref() else {
        return Ok(());
    };
    if invite_token == Some(expected) || host_token == Some(room.host_token.as_str()) {
        Ok(())
    } else {
        Err(RoomError::InvalidInviteToken)
    }
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

    let Some(player) = room
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
    else {
        return Ok(());
    };

    if player.connection_generation != connection_generation {
        return Ok(());
    }

    if room.status == RoomStatus::Running {
        player.connected = false;

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
            schedule_disconnected_room_expiration(store.clone(), code.to_string(), disconnected_at);
        }
        return Ok(());
    }

    room.players.retain(|player| player.id != player_id);

    let successor = if room.host_player_id == Some(player_id) {
        promote_oldest_player(room)
    } else {
        None
    };

    if room.players.is_empty() {
        stop_runtime(&room.runtime);
        rooms.remove(code);
        return Ok(());
    }

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::PlayerLeft {
        player_id,
        room: snapshot.clone(),
    });

    if let Some(host_player_id) = successor {
        let _ = room.tx.send(ServerMessage::HostChanged {
            host_player_id,
            room: snapshot,
        });
    }

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

    match room.status {
        RoomStatus::Preparing => return Err(RoomError::RuntimePreparing),
        RoomStatus::Waiting => {}
        RoomStatus::Running => return Err(RoomError::AlreadyRunning),
        RoomStatus::Failed => return Err(RoomError::RuntimeUnavailable),
    }

    ensure_host(room, player_id)?;

    if room.players.len() < room.spec.min_players() as usize {
        return Err(RoomError::InsufficientPlayers);
    }

    room.runtime
        .try_start(
            room.players
                .iter()
                .map(|player| SinteractPlayer {
                    id: player.id,
                    nickname: player.nickname.clone(),
                })
                .collect(),
        )
        .map_err(|_| RoomError::RuntimeUnavailable)?;

    room.status = RoomStatus::Running;

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::RoomStarted { room: snapshot });

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

    if room.status == RoomStatus::Running {
        return Err(RoomError::AlreadyRunning);
    }

    ensure_host(room, requester_id)?;

    if requester_id == target_id {
        return Err(RoomError::CannotKickSelf);
    }

    if !room.players.iter().any(|player| player.id == target_id) {
        return Err(RoomError::PlayerNotFound);
    }

    room.players.retain(|player| player.id != target_id);

    if room.host_player_id == Some(target_id) {
        room.host_player_id = None;
    }

    let snapshot = room_snapshot(room);

    let _ = room.tx.send(ServerMessage::PlayerKicked {
        player_id: target_id,
        room: snapshot.clone(),
    });

    let _ = room.tx.send(ServerMessage::RoomState { room: snapshot });

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

    if room.status == RoomStatus::Running {
        return Err(RoomError::AlreadyRunning);
    }

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

    let _ = room.tx.send(ServerMessage::RoomState { room: snapshot });

    Ok(())
}

fn forward_sinteract_statuses(
    store: RoomStore,
    code: String,
    mut statuses: tokio::sync::mpsc::Receiver<SinteractStatus>,
    tx: broadcast::Sender<ServerMessage>,
) {
    tokio::spawn(async move {
        while let Some(status) = statuses.recv().await {
            let message = match status {
                SinteractStatus::Ready => {
                    let snapshot = {
                        let Ok(mut rooms) = store.lock() else {
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
                    let _ = tx.send(ServerMessage::RoomState { room: snapshot });
                    continue;
                }
                SinteractStatus::Ended => {
                    let snapshot = mark_runtime_failed(&store, &code);
                    if let Some(snapshot) = snapshot {
                        let _ = tx.send(ServerMessage::RoomState { room: snapshot });
                    }
                    ServerMessage::Error {
                        message: "A partida foi encerrada".into(),
                    }
                }
                SinteractStatus::Failed(error) => {
                    let snapshot = mark_runtime_failed(&store, &code);
                    tracing::error!(room = %code, %error, "runtime Sinteract falhou");
                    if let Some(snapshot) = snapshot {
                        let _ = tx.send(ServerMessage::RoomState { room: snapshot });
                    }
                    ServerMessage::Error {
                        message: "A partida foi encerrada".into(),
                    }
                }
            };
            let _ = tx.send(message);
        }
    });
}

fn mark_runtime_failed(store: &RoomStore, code: &str) -> Option<RoomSnapshot> {
    let mut rooms = store.lock().ok()?;
    let room = rooms.get_mut(code)?;
    room.status = RoomStatus::Failed;
    room._runtime_permit.take();
    Some(room_snapshot(room))
}

fn forward_sinteract_changes(
    mut changes: tokio::sync::watch::Receiver<u64>,
    tx: broadcast::Sender<ServerMessage>,
) {
    tokio::spawn(async move {
        while changes.changed().await.is_ok() {
            let _ = tx.send(ServerMessage::SinteractChanged);
        }
    });
}

pub fn runtime(
    store: &RoomStore,
    code: &str,
    player_id: u64,
    connection_generation: u64,
) -> Result<SinteractHandle, RoomError> {
    let rooms = lock_rooms(store)?;
    let room = rooms.get(code).ok_or(RoomError::NotFound)?;
    ensure_current_connection(room, player_id, connection_generation)?;
    Ok(room.runtime.clone())
}

fn stop_runtime(runtime: &SinteractHandle) {
    let runtime = runtime.clone();
    tokio::spawn(async move {
        let _ = runtime.stop().await;
    });
}

fn ensure_host(room: &Room, player_id: u64) -> Result<(), RoomError> {
    if room.host_player_id == Some(player_id) {
        Ok(())
    } else {
        Err(RoomError::NotHost)
    }
}

fn promote_oldest_player(room: &mut Room) -> Option<u64> {
    let host_player_id = room.players.first()?.id;
    room.host_player_id = Some(host_player_id);
    for player in &mut room.players {
        player.is_host = player.id == host_player_id;
    }
    Some(host_player_id)
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
    let join_error = match room.status {
        RoomStatus::Running => Some(RoomError::AlreadyRunning.message().to_string()),
        RoomStatus::Failed => Some(RoomError::RuntimeUnavailable.message().to_string()),
        RoomStatus::Preparing | RoomStatus::Waiting
            if room.players.len() >= room.spec.max_players() as usize =>
        {
            Some(RoomError::Full.message().to_string())
        }
        RoomStatus::Preparing | RoomStatus::Waiting => None,
    };
    RoomSnapshot {
        code: room.code.clone(),
        game_id: room.spec.game_id(),
        is_preview: room.invite_token.is_some(),
        status: room.status.clone(),
        min_players: room.spec.min_players(),
        max_players: room.spec.max_players(),
        players: room.players.clone(),
        can_start: room.host_player_id.is_some()
            && room.status == RoomStatus::Waiting
            && room.players.len() >= room.spec.min_players() as usize,
        can_join: join_error.is_none(),
        join_error,
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
        !room.players.is_empty() || now.duration_since(room.created_at) < EMPTY_ROOM_TTL
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
        let should_remove = rooms
            .get(&code)
            .is_some_and(|room| room.created_at == created_at && room.players.is_empty());

        should_remove.then(|| rooms.remove(&code)).flatten()
    };

    if let Some(room) = expired {
        stop_runtime(&room.runtime);
        tracing::info!(room = %code, "sala vazia expirada e runtime encerrado");
    }
}

fn schedule_disconnected_room_expiration(store: RoomStore, code: String, disconnected_at: Instant) {
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
        stop_runtime(&room.runtime);
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
    use std::{sync::Arc, time::Duration};

    use backend::game_runtime::{RuntimeSpec, SinteractHandle, SinteractStatus};
    use tokio::sync::Semaphore;

    use super::{
        CreatedRoom, create_room_with_runtime, expire_disconnected_room_after,
        expire_empty_room_after, get_room_snapshot_with_access, join_room, join_room_with_invite,
        kick_player, leave_room, promote_host, start_room, validate_nickname,
    };
    use crate::rooms::{RoomError, RoomStatus, ServerMessage, new_store};

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
    async fn preview_room_requires_its_invite_but_allows_the_host() {
        let store = new_store();
        let created = create_test_room(&store, runtime_spec());
        {
            let mut rooms = store.lock().unwrap();
            rooms.get_mut(&created.code).unwrap().invite_token = Some("convite-secreto".into());
        }

        assert!(matches!(
            get_room_snapshot_with_access(&store, &created.code, None, None),
            Err(RoomError::InvalidInviteToken)
        ));
        assert!(
            get_room_snapshot_with_access(&store, &created.code, Some("convite-secreto"), None,)
                .is_ok()
        );
        assert!(matches!(
            join_room(&store, &created.code, "Convidada".into(), None, None),
            Err(RoomError::InvalidInviteToken)
        ));
        let invited = join_room_with_invite(
            &store,
            &created.code,
            "Convidada".into(),
            None,
            None,
            Some("convite-secreto".into()),
        )
        .unwrap();
        assert!(get_room_snapshot_with_access(
            &store,
            &created.code,
            None,
            Some(&invited.player_token),
        )
        .is_ok());
        assert!(
            join_room_with_invite(
                &store,
                &created.code,
                "Autora".into(),
                Some(created.host_token),
                None,
                None,
            )
            .is_ok()
        );
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

        let rooms = store.lock().unwrap();
        let room = rooms.get(&created.code).unwrap();
        let player = &room.players[0];
        assert_ne!(player.connection_generation, joined.connection_generation);
    }

    #[tokio::test]
    async fn room_enforces_minimum_and_maximum_player_limits() {
        let store = new_store();
        let below_minimum = create_test_room(&store, runtime_spec());
        let host = join_room(
            &store,
            &below_minimum.code,
            "Host".into(),
            Some(below_minimum.host_token),
            None,
        )
        .unwrap();
        store
            .lock()
            .unwrap()
            .get_mut(&below_minimum.code)
            .unwrap()
            .status = RoomStatus::Waiting;

        assert!(matches!(
            start_room(
                &store,
                &below_minimum.code,
                host.player_id,
                host.connection_generation,
            ),
            Err(RoomError::InsufficientPlayers),
        ));

        let at_capacity = create_test_room(&store, runtime_spec());
        join_room(
            &store,
            &at_capacity.code,
            "Primeira".into(),
            Some(at_capacity.host_token),
            None,
        )
        .unwrap();
        for nickname in ["Segunda", "Terceira", "Quarta", "Quinta", "Sexta"] {
            join_room(&store, &at_capacity.code, nickname.into(), None, None).unwrap();
        }

        assert!(matches!(
            join_room(&store, &at_capacity.code, "Sétima".into(), None, None),
            Err(RoomError::Full),
        ));
    }

    #[tokio::test]
    async fn host_departure_promotes_the_oldest_remaining_player() {
        let store = new_store();
        let created = create_test_room(&store, runtime_spec());
        let host = join_room(
            &store,
            &created.code,
            "Host".into(),
            Some(created.host_token),
            None,
        )
        .unwrap();
        let mut successor =
            join_room(&store, &created.code, "Primeira".into(), None, None).unwrap();
        let newest = join_room(&store, &created.code, "Segunda".into(), None, None).unwrap();

        {
            let mut rooms = store.lock().unwrap();
            rooms.get_mut(&created.code).unwrap().status = RoomStatus::Waiting;
        }

        leave_room(
            &store,
            &created.code,
            host.player_id,
            host.connection_generation,
        )
        .unwrap();

        {
            let rooms = store.lock().unwrap();
            let room = rooms.get(&created.code).unwrap();
            assert_eq!(room.host_player_id, Some(successor.player_id));
            assert!(
                room.players
                    .iter()
                    .find(|player| player.id == successor.player_id)
                    .unwrap()
                    .is_host
            );
            assert!(
                !room
                    .players
                    .iter()
                    .find(|player| player.id == newest.player_id)
                    .unwrap()
                    .is_host
            );
        }

        let mut saw_departure = false;
        let mut saw_promotion = false;
        while let Ok(event) = successor.rx.try_recv() {
            match event {
                ServerMessage::PlayerLeft {
                    player_id, room, ..
                } if player_id == host.player_id => {
                    saw_departure = true;
                    assert!(
                        room.players
                            .iter()
                            .any(|player| { player.id == successor.player_id && player.is_host })
                    );
                }
                ServerMessage::HostChanged {
                    host_player_id,
                    room,
                } if host_player_id == successor.player_id => {
                    saw_promotion = true;
                    assert!(room.can_start);
                }
                _ => {}
            }
        }
        assert!(saw_departure);
        assert!(saw_promotion);

        start_room(
            &store,
            &created.code,
            successor.player_id,
            successor.connection_generation,
        )
        .unwrap();
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
    async fn running_room_rejects_every_lobby_mutation() {
        let store = new_store();
        let created = create_test_room(&store, runtime_spec());
        let joined = join_room(
            &store,
            &created.code,
            "Host".into(),
            Some(created.host_token),
            None,
        )
        .unwrap();
        let guest = join_room(&store, &created.code, "Guest".into(), None, None).unwrap();

        {
            let mut rooms = store.lock().unwrap();
            rooms.get_mut(&created.code).unwrap().status = RoomStatus::Running;
        }

        assert!(matches!(
            kick_player(
                &store,
                &created.code,
                joined.player_id,
                joined.connection_generation,
                joined.player_id,
            ),
            Err(RoomError::AlreadyRunning)
        ));
        assert!(matches!(
            promote_host(
                &store,
                &created.code,
                joined.player_id,
                joined.connection_generation,
                joined.player_id,
            ),
            Err(RoomError::AlreadyRunning)
        ));
        assert!(matches!(
            start_room(
                &store,
                &created.code,
                guest.player_id,
                guest.connection_generation,
            ),
            Err(RoomError::AlreadyRunning)
        ));
        assert!(matches!(
            kick_player(
                &store,
                &created.code,
                guest.player_id,
                guest.connection_generation,
                joined.player_id,
            ),
            Err(RoomError::AlreadyRunning)
        ));
    }

    #[tokio::test]
    async fn runtime_ready_transitions_the_room_from_preparing_to_waiting() {
        let store = new_store();
        let (runtime, runtime_output, outputs) = SinteractHandle::simulated();
        let created =
            create_room_with_runtime(&store, runtime_spec(), runtime, outputs, false, None)
                .unwrap();
        let mut events = {
            let rooms = store.lock().unwrap();
            let room = rooms.get(&created.code).unwrap();
            assert!(matches!(room.status, RoomStatus::Preparing));
            room.tx.subscribe()
        };

        runtime_output.send(SinteractStatus::Ready).await.unwrap();
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
        let (runtime, _runtime_output, outputs) = SinteractHandle::simulated();
        let created =
            create_room_with_runtime(&store, runtime_spec(), runtime, outputs, false, None)
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
        let limiter = Arc::new(Semaphore::new(1));
        let permit = limiter.clone().try_acquire_owned().unwrap();
        let (runtime, runtime_output, outputs) = SinteractHandle::simulated();
        let created = create_room_with_runtime(
            &store,
            runtime_spec(),
            runtime,
            outputs,
            false,
            Some(permit),
        )
        .unwrap();
        let mut events = {
            let rooms = store.lock().unwrap();
            rooms.get(&created.code).unwrap().tx.subscribe()
        };

        runtime_output
            .send(SinteractStatus::Failed("fixture encerrou".into()))
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
        drop(rooms);
        assert!(limiter.try_acquire_owned().is_ok());
    }

    #[tokio::test]
    async fn runtime_failure_in_one_room_does_not_change_another_room() {
        let store = new_store();
        let (failed_runtime, failed_output, failed_statuses) = SinteractHandle::simulated();
        let failed = create_room_with_runtime(
            &store,
            runtime_spec(),
            failed_runtime,
            failed_statuses,
            false,
            None,
        )
        .unwrap();
        let (healthy_runtime, healthy_output, healthy_statuses) = SinteractHandle::simulated();
        let healthy = create_room_with_runtime(
            &store,
            runtime_spec(),
            healthy_runtime,
            healthy_statuses,
            false,
            None,
        )
        .unwrap();
        let mut failed_events = store
            .lock()
            .unwrap()
            .get(&failed.code)
            .unwrap()
            .tx
            .subscribe();
        let mut healthy_events = store
            .lock()
            .unwrap()
            .get(&healthy.code)
            .unwrap()
            .tx
            .subscribe();

        failed_output.send(SinteractStatus::Ready).await.unwrap();
        healthy_output.send(SinteractStatus::Ready).await.unwrap();
        tokio::time::timeout(Duration::from_secs(1), failed_events.recv())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), healthy_events.recv())
            .await
            .unwrap()
            .unwrap();

        failed_output
            .send(SinteractStatus::Failed("fixture encerrou".into()))
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), failed_events.recv())
            .await
            .unwrap()
            .unwrap();

        let rooms = store.lock().unwrap();
        assert!(matches!(
            rooms.get(&failed.code).unwrap().status,
            RoomStatus::Failed
        ));
        assert!(matches!(
            rooms.get(&healthy.code).unwrap().status,
            RoomStatus::Waiting
        ));
        drop(rooms);
        assert!(
            join_room(&store, &healthy.code, "Jogadora".into(), None, None).is_ok(),
            "a sala saudável deve continuar aceitando participantes",
        );
    }

    #[tokio::test]
    async fn runtime_end_marks_the_room_failed_and_releases_its_process_slot() {
        let store = new_store();
        let limiter = Arc::new(Semaphore::new(1));
        let permit = limiter.clone().try_acquire_owned().unwrap();
        let (runtime, runtime_output, outputs) = SinteractHandle::simulated();
        let created = create_room_with_runtime(
            &store,
            runtime_spec(),
            runtime,
            outputs,
            false,
            Some(permit),
        )
        .unwrap();
        let mut events = store
            .lock()
            .unwrap()
            .get(&created.code)
            .unwrap()
            .tx
            .subscribe();

        runtime_output.send(SinteractStatus::Ended).await.unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(
            store.lock().unwrap().get(&created.code).unwrap().status,
            RoomStatus::Failed
        ));
        assert!(limiter.try_acquire_owned().is_ok());
    }

    #[tokio::test]
    async fn empty_room_expiration_removes_the_room_and_its_runtime() {
        let store = new_store();
        let (runtime, _runtime_output, outputs) = SinteractHandle::simulated();
        let created =
            create_room_with_runtime(&store, runtime_spec(), runtime, outputs, false, None)
                .unwrap();
        let created_at = store.lock().unwrap().get(&created.code).unwrap().created_at;

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

    fn create_test_room(store: &crate::rooms::RoomStore, spec: RuntimeSpec) -> CreatedRoom {
        let (runtime, outputs) = SinteractHandle::simulated_ready();
        create_room_with_runtime(store, spec, runtime, outputs, false, None).unwrap()
    }
}
