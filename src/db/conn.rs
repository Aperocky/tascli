use std::{
    env,
    fs,
    path::PathBuf,
};

use rusqlite::Connection;

const DB_NAME: &str = "tascli.db";

fn get_data_dir() -> PathBuf {
    let home_dir = env::var_os("HOME")
        .map(PathBuf::from)
        .expect("$HOME environment variable not set");
    let data_dir = home_dir.join(".local").join("share").join("tascli");
    fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    data_dir
}

fn get_db_path() -> PathBuf {
    let data_dir = get_data_dir();
    data_dir.join(DB_NAME)
}

pub fn init_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY,
            action TEXT NOT NULL,
            category TEXT NOT NULL,
            content TEXT NOT NULL,
            create_time INTEGER NOT NULL,
            target_time INTEGER,
            modify_time INTEGER,
            closing_code INTEGER DEFAULT 0
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_create_time ON items(create_time)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_target_time ON items(target_time)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_category ON items(category)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_category_create_time ON items(category, create_time)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_category_target_time ON items(category, target_time)",
        [],
    )?;

    // Create cache table for list commands
    conn.execute(
        "CREATE TABLE IF NOT EXISTS cache (
            key INTEGER PRIMARY KEY,
            value INTEGER NOT NULL
        )",
        [],
    )?;

    Ok(())
}

pub fn connect() -> Result<Connection, rusqlite::Error> {
    let db_path = get_db_path();
    let conn = Connection::open(db_path)?;
    init_table(&conn)?;

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use rusqlite::{
        Connection,
        Row,
    };
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_init_table() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        let conn = Connection::open(db_path).unwrap();

        let result = init_table(&conn);
        assert!(
            result.is_ok(),
            "Failed to initialize table: {:?}",
            result.err()
        );

        let item_table_exists = conn.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='items'",
            [],
            |row: &Row| row.get::<_, String>(0),
        );
        assert!(item_table_exists.is_ok(), "Table 'items' does not exist");
        let cache_table_exists = conn.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='cache'",
            [],
            |row: &Row| row.get::<_, String>(0),
        );
        assert!(cache_table_exists.is_ok(), "Table 'cache' does not exist");

        let second_result = init_table(&conn);
        assert!(
            second_result.is_ok(),
            "Second initialization failed: {:?}",
            second_result.err()
        );
    }
}
