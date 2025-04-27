use rusqlite::Connection;

use crate::config::get_data_path;

// Going forward, all schema changes require toggling
// this DB_VERSION to a higher number.
const DB_VERSION: i32 = 1;

pub fn init_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    let current_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if current_version == DB_VERSION {
        return Ok(());
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action TEXT NOT NULL,
            category TEXT NOT NULL,
            content TEXT NOT NULL,
            create_time INTEGER NOT NULL,
            target_time INTEGER,
            modify_time INTEGER,
            status INTEGER DEFAULT 0
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

    conn.execute("PRAGMA user_version = 1", [])?;

    Ok(())
}

pub fn connect() -> Result<Connection, String> {
    let db_path = get_data_path()?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    init_table(&conn).map_err(|e| e.to_string())?;

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use rusqlite::Row;

    use super::*;
    use crate::tests::get_test_conn;

    #[test]
    fn test_init_table() {
        let (conn, _temp_file) = get_test_conn();

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
        let pragma_version = conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0));
        assert_eq!(1, pragma_version.unwrap());

        let second_result = init_table(&conn);
        assert!(
            second_result.is_ok(),
            "Second initialization failed: {:?}",
            second_result.err()
        );
    }
}
