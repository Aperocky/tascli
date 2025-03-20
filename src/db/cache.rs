use rusqlite::{
    params,
    Connection,
    Result,
};

pub fn store(conn: &Connection, kv: Vec<(i64, i64)>) -> Result<()> {
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

pub fn clear(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM cache", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;
    use crate::db::conn::init_table;

    fn get_test_conn() -> (Connection, NamedTempFile) {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        let conn = Connection::open(db_path).unwrap();
        init_table(&conn).unwrap();
        (conn, temp_file)
    }

    #[test]
    fn test_cache() {
        let kv = vec![(1, 3), (2, 9), (3, 8)];
        let (conn, _temp_file) = get_test_conn();
        store(&conn, kv).unwrap();

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
}
