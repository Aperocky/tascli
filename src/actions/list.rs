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

pub fn handle_listrecords(conn: &Connection, cmd: ListRecordCommand) -> Result<(), String> {
    let mut records = query_records(conn, &cmd)?;
    order_items(&mut records, CREATE_TIME_COL)?;

    cache::clear(conn).map_err(|e| e.to_string())?;
    cache::store(conn, &records).map_err(|e| e.to_string())?;

    display::print_bold("Records List:");
    display::print_items(&records, true, true);
    Ok(())
}

pub fn handle_listtasks(conn: &Connection, cmd: ListTaskCommand) -> Result<(), String> {
    let mut tasks = query_tasks(conn, &cmd)?;
    order_items(&mut tasks, TARGET_TIME_COL)?;

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
    // 255 status means we query all task items regardless of status.
    if cmd.status != 255 {
        task_query = task_query.with_status(cmd.status);
    }
    task_query = task_query.with_limit(cmd.limit);
    query_items(conn, &task_query).map_err(|e| e.to_string())
}

fn order_items(items: &mut [Item], by: &str) -> Result<(), String> {
    match by {
        CREATE_TIME_COL => {
            items.sort_by_key(|item| item.create_time);
            Ok(())
        }
        TARGET_TIME_COL => {
            // Check all items have target_time before sorting
            for item in items.iter() {
                if item.target_time.is_none() {
                    return Err("Task missing target_time, something went wrong".to_string());
                }
            }
            items.sort_by_key(|item| item.target_time.unwrap());
            Ok(())
        }
        _ => Err(format!("Cannot order by '{}'", by)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        get_test_conn,
        insert_record,
        insert_task,
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
    fn test_order_by_create_time() {
        let mut items = vec![
            Item::with_create_time(
                "task".to_string(),
                "test".to_string(),
                "content1".to_string(),
                300,
            ),
            Item::with_create_time(
                "task".to_string(),
                "test".to_string(),
                "content2".to_string(),
                100,
            ),
            Item::with_create_time(
                "task".to_string(),
                "test".to_string(),
                "content3".to_string(),
                200,
            ),
        ];

        let result = order_items(&mut items, CREATE_TIME_COL);
        assert!(result.is_ok());

        assert_eq!(items[0].content, "content2");
        assert_eq!(items[1].content, "content3");
        assert_eq!(items[2].content, "content1");
    }

    #[test]
    fn test_order_by_target_time() {
        let mut items = vec![
            Item::with_target_time(
                "record".to_string(),
                "test".to_string(),
                "content1".to_string(),
                Some(300),
            ),
            Item::with_target_time(
                "record".to_string(),
                "test".to_string(),
                "content2".to_string(),
                Some(100),
            ),
            Item::with_target_time(
                "record".to_string(),
                "test".to_string(),
                "content3".to_string(),
                Some(200),
            ),
        ];

        let result = order_items(&mut items, TARGET_TIME_COL);
        assert!(result.is_ok());

        assert_eq!(items[0].content, "content2");
        assert_eq!(items[1].content, "content3");
        assert_eq!(items[2].content, "content1");
    }

    #[test]
    fn test_query_tasks() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "life", "gather w2", "eow");
        insert_task(&conn, "fun", "write list.rs tests", "today");
        insert_task(&conn, "fun", "expired content", "yesterday");
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
        list_tasks_default.overdue = true;
        let results = query_tasks(&conn, &list_tasks_default).unwrap();
        assert_eq!(results.len(), 3);
    }
}
