use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<Connection>>;

pub fn init(path: &str) -> Db {
    let conn = Connection::open(path)
        .expect("falha ao abrir o banco de dados");

    // Habilita foreign keys — desabilitado por padrão no SQLite
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("falha ao habilitar foreign keys");

    // Roda o schema
    let schema = include_str!("schema.sql");
    conn.execute_batch(schema)
        .expect("falha ao inicializar o schema");

    tracing::info!("banco de dados inicializado em {}", path);
    Arc::new(Mutex::new(conn))
}
