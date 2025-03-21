use rusqlite::Connection;
use tempfile::NamedTempFile;

use crate::{
    args::timestr,
    db::{
        conn::init_table,
        crud::insert_item,
        item::Item,
    },
};

pub fn get_test_conn() -> (Connection, NamedTempFile) {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let conn = Connection::open(db_path).unwrap();
    init_table(&conn).unwrap();
    (conn, temp_file)
}

pub fn insert_task(conn: &Connection, category: &str, content: &str, timestr: &str) {
    let target_time = timestr::to_unix_epoch(timestr).unwrap();
    let new_task = Item::with_target_time(
        "task".to_string(),
        category.to_string(),
        content.to_string(),
        Some(target_time),
    );
    insert_item(conn, &new_task).unwrap();
}

pub fn insert_record(conn: &Connection, category: &str, content: &str, timestr: &str) {
    let create_time = timestr::to_unix_epoch(timestr).unwrap();
    let new_record = Item::with_create_time(
        "task".to_string(),
        category.to_string(),
        content.to_string(),
        create_time,
    );
    insert_item(conn, &new_record).unwrap();
}
