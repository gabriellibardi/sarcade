use axum::extract::FromRef;

use crate::{
    auth::AuthService,
    db::Db,
    rooms::{RoomStore, new_store},
};
use backend::game_runtime::SgleamExecutable;

#[derive(Clone)]
pub struct AppState {
    pub auth: AuthService,
    pub db: Db,
    pub rooms: RoomStore,
    pub sgleam_executable: Option<SgleamExecutable>,
}

impl AppState {
    pub fn new(db: Db, auth: AuthService, sgleam_executable: Option<SgleamExecutable>) -> Self {
        Self {
            auth,
            db,
            rooms: new_store(),
            sgleam_executable,
        }
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
