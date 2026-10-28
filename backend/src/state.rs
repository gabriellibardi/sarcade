use axum::extract::FromRef;

use crate::{
    auth::AuthService,
    db::Db,
    rooms::{RoomStore, new_store},
};
use backend::game_runtime::RuntimeCatalog;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

#[derive(Clone)]
pub struct AppState {
    pub auth: AuthService,
    pub db: Db,
    pub rooms: RoomStore,
    pub runtimes: RuntimeCatalog,
    runtime_processes: std::sync::Arc<Semaphore>,
}

impl AppState {
    pub fn new(
        db: Db,
        auth: AuthService,
        runtimes: RuntimeCatalog,
        max_runtime_processes: usize,
    ) -> Self {
        Self {
            auth,
            db,
            rooms: new_store(),
            runtimes,
            runtime_processes: std::sync::Arc::new(Semaphore::new(max_runtime_processes)),
        }
    }

    pub fn reserve_runtime_process(&self) -> Result<OwnedSemaphorePermit, TryAcquireError> {
        self.runtime_processes.clone().try_acquire_owned()
    }
}

impl FromRef<AppState> for AuthService {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}

impl FromRef<AppState> for Db {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for RoomStore {
    fn from_ref(state: &AppState) -> Self {
        state.rooms.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use rusqlite::Connection;

    use backend::game_runtime::RuntimeCatalog;

    use super::AppState;
    use crate::auth::AuthService;

    #[test]
    fn runtime_process_slots_are_bounded_and_released_on_drop() {
        let state = AppState::new(
            Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            AuthService::new(None, Vec::new()),
            RuntimeCatalog::default(),
            1,
        );

        let permit = state.reserve_runtime_process().unwrap();
        assert!(state.reserve_runtime_process().is_err());

        drop(permit);
        assert!(state.reserve_runtime_process().is_ok());
    }
}
