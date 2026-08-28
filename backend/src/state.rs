use axum::extract::FromRef;

use crate::{
    auth::AuthService,
    db::Db,
    rooms::{RoomStore, new_store},
};

#[derive(Clone)]
pub struct AppState {
    pub auth: AuthService,
    pub db: Db,
    pub rooms: RoomStore,
}

impl AppState {
    pub fn new(db: Db, auth: AuthService) -> Self {
        Self {
            auth,
            db,
            rooms: new_store(),
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
