use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use rusqlite::{
    params,
    Connection,
    Result,
};

use crate::db::item::Item;

pub fn store(conn: &Connection, items: &[Item]) -> Result<()> {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64;

    // Store current time at index 0
    // For invalidations after some time.
    let mut kv: Vec<(i64, i64)> = vec![(0, current_time)];

    let items_kv: Vec<(i64, i64)> = items
        .iter()
        .enumerate()
        .map(|(index, item)| ((index + 1) as i64, item.id.unwrap()))
        .collect();

    kv.extend(items_kv);
    store_kv(conn, kv)
}

// add a next token marker
pub fn store_with_next(conn: &Connection, items: &[Item]) -> Result<()> {
    store(conn, items)?;
    conn.execute(
        "INSERT OR REPLACE INTO cache (key, value) VALUES (?1, ?2)",
        [-1, items.len() as i64],
    )?;
    Ok(())
}

pub fn validate_cache(conn: &Connection) -> Result<bool> {
    let timestamp = match read(conn, 0)? {
        Some(t) => t,
        None => return Ok(false),
    };
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64;
    if current_time - timestamp > 3600 {
        clear(conn)?;
        return Ok(false);
    }
    Ok(true)
}

fn store_kv(conn: &Connection, kv: Vec<(i64, i64)>) -> Result<()> {
    let mut stmt = conn.prepare("INSERT OR REPLACE INTO cache (key, value) VALUES (?1, ?2)")?;

    // Execute each statement individually
    for (key, value) in kv {
        stmt.execute(params![key, value])?;
    }

    Ok(())
}

pub fn read(conn: &Connection, index: i64) -> Result<Option<i64>> {
    let result = conn.query_row(
        "SELECT value FROM cache WHERE key = ?1",
        params![index],
        |row| row.get(0),
    );

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err),
    }
}

pub fn get_next_index(conn: &Connection) -> Result<Option<i64>> {
    read(conn, -1)
}

pub fn clear(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM cache", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::get_test_conn;

    #[test]
    fn test_cache() {
        let kv = vec![(1, 3), (2, 9), (3, 8)];
        let (conn, _temp_file) = get_test_conn();
        store_kv(&conn, kv).unwrap();

        let val = read(&conn, 1).expect("Error reading key value");
        assert_eq!(val, Some(3));
        let val = read(&conn, 2).expect("Error reading key value");
        assert_eq!(val, Some(9));
        let val = read(&conn, 3).expect("Error reading key value");
        assert_eq!(val, Some(8));
        let val = read(&conn, 4).expect("Error reading key value");
        assert_eq!(val, None);

        clear(&conn).unwrap();
        let val = read(&conn, 1).expect("Error reading key value");
        assert_eq!(val, None);
    }

    #[test]
    fn test_validate_cache_empty() {
        let (conn, _temp_file) = get_test_conn();
        let valid = validate_cache(&conn).expect("Failed to validate cache");
        assert!(!valid);
    }

    #[test]
    fn test_validate_cache_fresh() {
        let (conn, _temp_file) = get_test_conn();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        store_kv(&conn, vec![(0, current_time)]).expect("Failed to store timestamp");
        let valid = validate_cache(&conn).expect("Failed to validate cache");
        assert!(valid);
    }

    #[test]
    fn test_validate_cache_expired() {
        // Test with expired cache (more than an hour old)
        let (conn, _temp_file) = get_test_conn();

        // Create timestamp from more than an hour ago
        let expired_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64
            - 3601; // 1 hour + 1 second
        store_kv(&conn, vec![(0, expired_time)]).expect("Failed to store timestamp");
        let valid = validate_cache(&conn).expect("Failed to validate cache");
        assert!(!valid);
    }

    #[test]
    fn test_store() {
        let (conn, _temp_file) = get_test_conn();

        let mut item1 = Item::new(
            "task".to_string(),
            "work".to_string(),
            "Test task 1".to_string(),
        );
        item1.id = Some(123);

        let mut item2 = Item::new(
            "task".to_string(),
            "work".to_string(),
            "Test note 2".to_string(),
        );
        item2.id = Some(456);

        let mut item3 = Item::new(
            "delete".to_string(),
            "reminder".to_string(),
            "Test reminder 3".to_string(),
        );
        item3.id = Some(789);

        let items = vec![item1, item2, item3];

        store(&conn, &items).expect("Failed to store items in cache");
        let valid = validate_cache(&conn).expect("Failed to validate cache");
        assert!(valid);

        // Verify item IDs were stored at correct indices
        let id1 = read(&conn, 1).expect("Failed to read item 1");
        let id2 = read(&conn, 2).expect("Failed to read item 2");
        let id3 = read(&conn, 3).expect("Failed to read item 3");

        assert_eq!(id1, Some(123), "Item 1 ID should be stored at index 1");
        assert_eq!(id2, Some(456), "Item 2 ID should be stored at index 2");
        assert_eq!(id3, Some(789), "Item 3 ID should be stored at index 3");

        // Verify non-existent index returns None
        let id4 = read(&conn, 4).expect("Failed to read non-existent index");
        assert_eq!(id4, None, "Non-existent index should return None");

        // Test next token
        assert_eq!(get_next_index(&conn).unwrap(), None);
        clear(&conn).unwrap();
        store_with_next(&conn, &items).expect("Failed to store items in cache");
        assert_eq!(get_next_index(&conn).unwrap(), Some(3));
    }
}
