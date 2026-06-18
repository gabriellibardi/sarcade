use axum::extract::FromRef;

use crate::{
    db::Db,
    rooms::{RoomStore, new_store},
};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub rooms: RoomStore,
}

impl AppState {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            rooms: new_store(),
        }
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
