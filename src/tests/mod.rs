use rusqlite::Connection;
use tempfile::NamedTempFile;

use crate::{
    args::timestr,
    db::{
        conn::init_table,
        crud::{
            get_item,
            insert_item,
            update_item,
        },
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

pub fn insert_task(conn: &Connection, category: &str, content: &str, timestr: &str) -> i64 {
    let target_time = timestr::to_unix_epoch(timestr).unwrap();
    let new_task = Item::with_target_time(
        "task".to_string(),
        category.to_string(),
        content.to_string(),
        Some(target_time),
    );
    insert_item(conn, &new_task).unwrap()
}

pub fn insert_record(conn: &Connection, category: &str, content: &str, timestr: &str) -> i64 {
    let create_time = timestr::to_unix_epoch(timestr).unwrap();
    let new_record = Item::with_create_time(
        "record".to_string(),
        category.to_string(),
        content.to_string(),
        create_time,
    );
    insert_item(conn, &new_record).unwrap()
}

pub fn update_status(conn: &Connection, rowid: i64, status_code: u8) {
    let mut task = get_item(conn, rowid).unwrap();
    task.status = status_code;
    update_item(conn, &task).unwrap();
}

pub fn insert_recurring_task(
    conn: &Connection,
    category: &str,
    content: &str,
    human_schedule: &str,
) -> i64 {
    let cron_schedule = timestr::parse_recurring_timestr(human_schedule).unwrap();
    let recurring_task = Item::create_recurring_task(
        category.to_string(),
        content.to_string(),
        cron_schedule,
        human_schedule.to_string(),
    );
    insert_item(conn, &recurring_task).unwrap()
}

pub fn insert_recurring_record(
    conn: &Connection,
    category: &str,
    content: &str,
    recurring_task_id: i64,
    good_until: i64,
) -> i64 {
    let record = Item::create_recurring_record(
        category.to_string(),
        content.to_string(),
        recurring_task_id,
        good_until,
    );
    insert_item(conn, &record).unwrap()
}
