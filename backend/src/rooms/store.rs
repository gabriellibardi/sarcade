use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::rooms::model::Room;

pub type RoomStore = Arc<Mutex<HashMap<String, Room>>>;

pub fn new_store() -> RoomStore {
    Arc::new(Mutex::new(HashMap::new()))
}
