use chrono::Local;
use rusqlite::Connection;

use crate::{
    actions::display,
    args::{
        parser::{
            ListRecordCommand,
            ListTaskCommand,
        },
        timestr,
    },
    db::{
        cache,
        crud::query_items,
        item::{
            Item,
            ItemQuery,
        },
    },
};

const CREATE_TIME_COL: &str = "create_time";
const TARGET_TIME_COL: &str = "target_time";

const OPEN_STATUS_CODES: &[u8] = &[0, 4, 6];
const CLOSED_STATUS_CODES: &[u8] = &[1, 2, 3, 5];

pub fn handle_listrecords(conn: &Connection, cmd: ListRecordCommand) -> Result<(), String> {
    let records = query_records(conn, &cmd)?;

    cache::clear(conn).map_err(|e| e.to_string())?;
    cache::store(conn, &records).map_err(|e| e.to_string())?;

    display::print_bold("Records List:");
    display::print_items(&records, true, true);
    Ok(())
}

pub fn handle_listtasks(conn: &Connection, cmd: ListTaskCommand) -> Result<(), String> {
    let tasks = query_tasks(conn, &cmd)?;

    cache::clear(conn).map_err(|e| e.to_string())?;
    cache::store(conn, &tasks).map_err(|e| e.to_string())?;

    display::print_bold("Tasks List:");
    display::print_items(&tasks, false, true);
    Ok(())
}

fn query_records(conn: &Connection, cmd: &ListRecordCommand) -> Result<Vec<Item>, String> {
    let mut record_query = ItemQuery::new().with_action("record");
    if let Some(cat) = &cmd.category {
        record_query = record_query.with_category(cat);
    }
    if let Some(days) = cmd.days {
        let cutoff_timestamp = timestr::days_before_to_unix_epoch(days);
        record_query = record_query.with_create_time_min(cutoff_timestamp);
    }
    if let Some(starting_time) = &cmd.starting_time {
        let starting_timestamp = timestr::to_unix_epoch(starting_time)?;
        record_query = record_query.with_create_time_min(starting_timestamp);
    }
    if let Some(ending_time) = &cmd.ending_time {
        let ending_timestamp = timestr::to_unix_epoch(ending_time)?;
        record_query = record_query.with_create_time_max(ending_timestamp);
    }
    record_query = record_query.with_limit(cmd.limit);
    record_query = record_query.with_order_by(CREATE_TIME_COL);
    query_items(conn, &record_query).map_err(|e| e.to_string())
}

fn query_tasks(conn: &Connection, cmd: &ListTaskCommand) -> Result<Vec<Item>, String> {
    let mut task_query = ItemQuery::new().with_action("task");
    if let Some(t) = &cmd.timestr {
        let target_time_before = timestr::to_unix_epoch(t)?;
        task_query = task_query.with_target_time_max(target_time_before);
    } else if let Some(days) = cmd.days {
        let cutoff_timestamp = timestr::days_after_to_unix_epoch(days);
        task_query = task_query.with_target_time_max(cutoff_timestamp);
    }
    if !cmd.overdue {
        task_query = task_query.with_target_time_min(Local::now().timestamp());
    }
    if let Some(cat) = &cmd.category {
        task_query = task_query.with_category(cat);
    }
    match cmd.status {
        // 255 status means we query all task items regardless of status.
        255 => {}
        // 254 status indicates a combination of statuses that are open
        254 => task_query = task_query.with_statuses(OPEN_STATUS_CODES.to_vec()),
        // 254 status indicates a combination of statuses that are closed
        253 => task_query = task_query.with_statuses(CLOSED_STATUS_CODES.to_vec()),
        // Other statuses are individual statuses for query
        _ => task_query = task_query.with_statuses(vec![cmd.status]),
    }
    task_query = task_query.with_limit(cmd.limit);
    task_query = task_query.with_order_by(TARGET_TIME_COL);
    query_items(conn, &task_query).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        get_test_conn,
        insert_record,
        insert_task,
        update_status,
    };

    #[test]
    fn test_query_records() {
        let (conn, _temp_file) = get_test_conn();
        insert_record(&conn, "feeding", "100ML", "yesterday 2PM");
        insert_record(&conn, "feeding", "110ML", "yesterday 5PM");
        insert_record(&conn, "feeding", "100ML", "yesterday 9PM");
        insert_record(&conn, "FTP", "256W", "yesterday 7PM");
        let listfeeding = ListRecordCommand {
            category: Some("feeding".to_string()),
            days: None,
            limit: 100,
            starting_time: None,
            ending_time: None,
        };
        let list_all = ListRecordCommand {
            category: None,
            days: Some(2),
            limit: 100,
            starting_time: None,
            ending_time: None,
        };
        let list_timeframe = ListRecordCommand {
            category: None,
            days: None,
            limit: 100,
            starting_time: Some("yesterday 4PM".to_string()),
            ending_time: Some("yesterday 8PM".to_string()),
        };
        let list_timeframe_start_only = ListRecordCommand {
            category: None,
            days: None,
            limit: 100,
            starting_time: Some("yesterday 8PM".to_string()),
            ending_time: None,
        };
        let results = query_records(&conn, &listfeeding).unwrap();
        assert_eq!(results.len(), 3);
        let results = query_records(&conn, &list_all).unwrap();
        assert_eq!(results.len(), 4);
        let results = query_records(&conn, &list_timeframe).unwrap();
        assert_eq!(results.len(), 2);
        let results = query_records(&conn, &list_timeframe_start_only).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].category, "feeding")
    }

    #[test]
    fn test_query_tasks() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "life", "third_due", "tomorrow");
        insert_task(&conn, "fun", "second_due", "today");
        insert_task(&conn, "fun", "first_due", "yesterday");
        let mut list_tasks_default = ListTaskCommand {
            timestr: None,
            category: None,
            days: None,
            status: 0,
            overdue: false,
            limit: 100,
        };
        let results = query_tasks(&conn, &list_tasks_default).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results.first().unwrap().content, "second_due");
        assert_eq!(results.last().unwrap().content, "third_due");
        list_tasks_default.overdue = true;
        let results = query_tasks(&conn, &list_tasks_default).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results.first().unwrap().content, "first_due");
    }

    #[test]
    fn test_query_tasks_statuses() {
        let (conn, _temp_file) = get_test_conn();
        let rowid = insert_task(&conn, "cancelled", "cancelled-task-0", "today");
        update_status(&conn, rowid, 2);
        for i in 1..=2 {
            let rowid = insert_task(&conn, "pending", &format!("pending-task-{}", i), "today");
            update_status(&conn, rowid, 6);
        }
        for i in 1..=3 {
            let rowid = insert_task(&conn, "done", &format!("completed-task-{}", i), "today");
            update_status(&conn, rowid, 1);
        }
        for i in 1..=4 {
            insert_task(&conn, "ongoing", &format!("ongoing-task-{}", i), "today");
        }
        let list_open = ListTaskCommand {
            timestr: None,
            category: None,
            days: None,
            limit: 100,
            status: 254,
            overdue: false,
        };
        let list_closed = ListTaskCommand {
            timestr: None,
            category: None,
            days: None,
            limit: 100,
            status: 253,
            overdue: false,
        };
        let results = query_tasks(&conn, &list_open).expect("Unable to query");
        assert_eq!(results.len(), 6);
        assert!(results
            .iter()
            .all(|t| t.category == "ongoing" || t.category == "pending"));
        let results = query_tasks(&conn, &list_closed).expect("Unable to query");
        assert_eq!(results.len(), 4);
        assert!(results
            .iter()
            .all(|t| t.category == "done" || t.category == "cancelled"));
    }
}
