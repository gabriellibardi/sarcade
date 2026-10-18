use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<Connection>>;

const CURRENT_SCHEMA_VERSION: i64 = 2;

pub fn init(path: &str) -> Db {
    let conn = Connection::open(path).expect("falha ao abrir o banco de dados");

    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("falha ao habilitar foreign keys");

    initialize_schema(&conn).expect("falha ao inicializar o schema");

    tracing::info!("banco de dados inicializado em {}", path);
    Arc::new(Mutex::new(conn))
}

pub(crate) fn initialize_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(include_str!("schema.sql"))?;

    let mut version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(rusqlite::Error::InvalidQuery);
    }

    // Databases created before versioned migrations already have this exact
    // schema, but no user_version. Marking the baseline lets later changes be
    // applied transactionally without mistaking an existing database for a
    // fresh installation.
    if version == 0 {
        conn.pragma_update(None, "user_version", 1)?;
        version = 1;
    }

    if version == 1 {
        migrate_revisions_v2(conn)?;
    }

    Ok(())
}

fn migrate_revisions_v2(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        BEGIN IMMEDIATE;

        ALTER TABLE games ADD COLUMN archived_at TEXT;

        ALTER TABLE game_revisions RENAME TO game_revisions_v1;

        CREATE TABLE game_revisions (
            id                 INTEGER PRIMARY KEY AUTOINCREMENT,
            game_id            INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
            revision_number    INTEGER NOT NULL CHECK (revision_number >= 1),
            parent_revision_id INTEGER REFERENCES game_revisions(id),
            created_by_user_id INTEGER REFERENCES users(id),
            runtime_build_id   TEXT NOT NULL REFERENCES runtime_builds(id),
            title              TEXT NOT NULL,
            description        TEXT NOT NULL,
            thumbnail          BLOB,
            source_code        TEXT NOT NULL,
            min_players        INTEGER NOT NULL DEFAULT 1 CHECK (min_players >= 1),
            max_players        INTEGER NOT NULL DEFAULT 1 CHECK (max_players >= min_players),
            status             TEXT NOT NULL DEFAULT 'draft'
                               CHECK (status IN ('draft', 'pending', 'approved', 'rejected', 'withdrawn')),
            review_message     TEXT,
            created_at         TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at         TEXT NOT NULL DEFAULT (datetime('now')),
            submitted_at       TEXT,
            reviewed_at        TEXT,
            UNIQUE (game_id, revision_number)
        );

        INSERT INTO game_revisions (
            id,
            game_id,
            revision_number,
            created_by_user_id,
            runtime_build_id,
            title,
            description,
            thumbnail,
            source_code,
            min_players,
            max_players,
            status,
            review_message,
            created_at,
            updated_at,
            submitted_at,
            reviewed_at
        )
        SELECT
            revisions.id,
            revisions.game_id,
            (
                SELECT COUNT(*)
                FROM game_revisions_v1 AS earlier
                WHERE earlier.game_id = revisions.game_id
                  AND earlier.id <= revisions.id
            ),
            games.author_id,
            revisions.runtime_build_id,
            games.title,
            games.description,
            games.thumbnail,
            revisions.source_code,
            revisions.min_players,
            revisions.max_players,
            revisions.status,
            revisions.review_message,
            revisions.created_at,
            revisions.created_at,
            revisions.created_at,
            revisions.reviewed_at
        FROM game_revisions_v1 AS revisions
        JOIN games ON games.id = revisions.game_id;

        DROP TABLE game_revisions_v1;

        CREATE INDEX game_revisions_game_id
            ON game_revisions(game_id);
        CREATE INDEX game_revisions_status
            ON game_revisions(status);
        CREATE INDEX game_revisions_runtime_build_id
            ON game_revisions(runtime_build_id);

        PRAGMA user_version = 2;
        COMMIT;
        ",
    )
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
        assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn adopts_an_existing_unversioned_database_as_the_baseline() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(include_str!("schema.sql")).unwrap();

        assert_eq!(schema_version(&conn), 0);
        initialize_schema(&conn).unwrap();
        assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrates_existing_revision_metadata_and_history() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(include_str!("schema.sql")).unwrap();
        conn.execute(
            "INSERT INTO users (email, name) VALUES ('autor@example.com', 'Autora')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO games (title, description, thumbnail, author_id) VALUES ('Jogo', 'Descrição', X'0102', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "
            INSERT INTO game_revisions
                (game_id, runtime_build_id, source_code, status, reviewed_at)
            VALUES
                (1, 'sgleam-dev', 'primeira', 'rejected', '2026-01-01'),
                (1, 'sgleam-dev', 'segunda', 'approved', '2026-01-02')
            ",
            [],
        )
        .unwrap();

        initialize_schema(&conn).unwrap();

        let revisions = conn
            .prepare(
                "SELECT revision_number, title, description, thumbnail, created_by_user_id, status FROM game_revisions ORDER BY id",
            )
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(revisions.len(), 2);
        assert_eq!(revisions[0].0, 1);
        assert_eq!(revisions[1].0, 2);
        assert_eq!(revisions[1].1, "Jogo");
        assert_eq!(revisions[1].2, "Descrição");
        assert_eq!(revisions[1].3, vec![1, 2]);
        assert_eq!(revisions[1].4, 1);
        assert_eq!(revisions[0].5, "rejected");
        assert_eq!(revisions[1].5, "approved");
    }

    fn table_exists(conn: &Connection, table: &str) -> bool {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn schema_version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap()
    }
}
