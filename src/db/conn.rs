use rusqlite::Connection;

use crate::config::get_data_path;

// Going forward, all schema changes require toggling
// this DB_VERSION to a higher number.
const SCHEMA_VERSION: i32 = 2;

pub fn init_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    let current_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if current_version == SCHEMA_VERSION {
        return Ok(());
    }

    // Single polymorphic table
    // Supports task, record, recurring_task, recurring_task_record
    // distinguished via field "action"
    // common fields: id; action; category; content; create_time; modify_time; status;
    // target_time is specific for type task
    // cron_schedule; human_schedule is specific for type recurring_task
    // recurring_task_id; good_until is for type recurring task record
    conn.execute(
        "CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action TEXT NOT NULL,
            category TEXT NOT NULL,
            content TEXT NOT NULL,
            create_time INTEGER NOT NULL,
            target_time INTEGER,
            modify_time INTEGER,
            status INTEGER DEFAULT 0,
            cron_schedule TEXT,
            human_schedule TEXT,
            recurring_task_id INTEGER,
            good_until INTEGER
        )",
        [],
    )?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_action ON items(action)", [])?;

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

    // Migrate from version 1 to 2 - add columns for recurring task support
    if current_version < 2 && current_version > 0 {
        conn.execute("ALTER TABLE items ADD COLUMN cron_schedule TEXT", [])?;
        conn.execute("ALTER TABLE items ADD COLUMN human_schedule TEXT", [])?;
        conn.execute("ALTER TABLE items ADD COLUMN recurring_task_id INTEGER", [])?;
        conn.execute("ALTER TABLE items ADD COLUMN good_until INTEGER", [])?;
    }

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_recurring_task_id_good_until ON items(recurring_task_id, good_until)",
        [],
    )?;

    conn.execute(&format!("PRAGMA user_version = {SCHEMA_VERSION}"), [])?;

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
        assert_eq!(SCHEMA_VERSION, pragma_version.unwrap());

        let second_result = init_table(&conn);
        assert!(
            second_result.is_ok(),
            "Second initialization failed: {:?}",
            second_result.err()
        );
    }

    #[test]
    fn test_init_table_version_logic() {
        let (conn, _temp_file) = get_test_conn();

        // Manaually drop table and verify with schema version the same
        // init_table DOES NOT recreate it.
        conn.execute("DROP TABLE cache", []).unwrap();
        let second_result = init_table(&conn);
        assert!(second_result.is_ok());

        let cache_exists = conn.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='cache'",
            [],
            |row: &Row| row.get::<_, String>(0),
        );
        assert!(
            cache_exists.is_err(),
            "Cache table should NOT exist - proves early return works"
        );

        // Manually set schema version to a lower number,
        // init_table should then run.
        conn.execute("PRAGMA user_version = 0", []).unwrap();
        let third_result = init_table(&conn);
        assert!(third_result.is_ok());

        let cache_exists_after_update = conn.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='cache'",
            [],
            |row: &Row| row.get::<_, String>(0),
        );
        assert!(
            cache_exists_after_update.is_ok(),
            "Cache table should exist after version reset"
        );

        let final_version = conn
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0))
            .unwrap();
        assert_eq!(SCHEMA_VERSION, final_version);
    }
}
