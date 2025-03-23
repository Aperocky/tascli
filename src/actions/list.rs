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

pub fn handle_listrecords(conn: &Connection, cmd: ListRecordCommand) -> Result<(), String> {
    let records = query_records(conn, &cmd)?;
    cache::clear(conn).map_err(|e| e.to_string())?;
    cache::store(conn, &records).map_err(|e| e.to_string())?;
    display::print_bold("Records List:");
    display::print_items(&records, true);
    Ok(())
}

pub fn handle_listtasks(conn: &Connection, cmd: ListTaskCommand) -> Result<(), String> {
    let tasks = query_tasks(conn, &cmd)?;
    cache::clear(conn).map_err(|e| e.to_string())?;
    cache::store(conn, &tasks).map_err(|e| e.to_string())?;

    display::print_bold("Tasks List:");
    display::print_items(&tasks, false);
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
        };
        let list_all = ListRecordCommand {
            category: None,
            days: Some(2),
            limit: 100,
        };
        let results = query_records(&conn, &listfeeding).unwrap();
        assert_eq!(results.len(), 3);
        let results = query_records(&conn, &list_all).unwrap();
        assert_eq!(results.len(), 4);
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
