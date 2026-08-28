use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<Connection>>;

pub fn init(path: &str) -> Db {
    let conn = Connection::open(path).expect("falha ao abrir o banco de dados");

    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("falha ao habilitar foreign keys");

    initialize_schema(&conn).expect("falha ao inicializar o schema");

    tracing::info!("banco de dados inicializado em {}", path);
    Arc::new(Mutex::new(conn))
}

fn initialize_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(include_str!("schema.sql"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_current_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

        initialize_schema(&conn).unwrap();

        let runtime_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM runtime_builds", [], |row| row.get(0))
            .unwrap();

        assert_eq!(runtime_count, 2);
        assert!(table_exists(&conn, "games"));
        assert!(table_exists(&conn, "game_revisions"));
        assert!(table_exists(&conn, "user_identities"));
        assert!(table_exists(&conn, "oauth_states"));
        assert!(table_exists(&conn, "sessions"));
    }

    fn table_exists(conn: &Connection, table: &str) -> bool {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )
        .unwrap()
    }
}
