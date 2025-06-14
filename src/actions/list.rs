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
        crud::{
            get_item,
            query_items,
        },
        item::{
            Item,
            ItemQuery,
            Offset,
        },
    },
};

const CREATE_TIME_COL: &str = "create_time";
const TARGET_TIME_COL: &str = "target_time";

const OPEN_STATUS_CODES: &[u8] = &[0, 4, 6];
const CLOSED_STATUS_CODES: &[u8] = &[1, 2, 3, 5];

pub fn handle_listrecords(conn: &Connection, cmd: ListRecordCommand) -> Result<(), String> {
    let records = match query_records(conn, &cmd) {
        Ok(records) => records,
        Err(estr) => {
            display::print_bold(&estr);
            return Ok(());
        }
    };
    if records.is_empty() {
        display::print_bold("No records found");
        return Ok(());
    }

    cache::clear(conn).map_err(|e| e.to_string())?;
    if records.len() == cmd.limit {
        cache::store_with_next(conn, &records)
    } else {
        cache::store(conn, &records)
    }
    .map_err(|e| e.to_string())?;

    display::print_bold("Records List:");
    display::print_items(&records, true, true);
    Ok(())
}

pub fn handle_listtasks(conn: &Connection, cmd: ListTaskCommand) -> Result<(), String> {
    let tasks = match query_tasks(conn, &cmd) {
        Ok(tasks) => tasks,
        Err(estr) => {
            display::print_bold(&estr);
            return Ok(());
        }
    };
    if tasks.is_empty() {
        display::print_bold("No tasks found");
        return Ok(());
    }

    cache::clear(conn).map_err(|e| e.to_string())?;
    if tasks.len() == cmd.limit {
        cache::store_with_next(conn, &tasks)
    } else {
        cache::store(conn, &tasks)
    }
    .map_err(|e| e.to_string())?;

    display::print_bold("Tasks List:");
    display::print_items(&tasks, false, true);
    Ok(())
}

fn query_records(conn: &Connection, cmd: &ListRecordCommand) -> Result<Vec<Item>, String> {
    let mut record_query = ItemQuery::new().with_action("record");
    if let Some(cat) = &cmd.category {
        record_query = record_query.with_category(cat);
    }
    if let Some(search_term) = &cmd.search {
        record_query = record_query.with_content_like(search_term);
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

    let mut offset = Offset::None;
    if cmd.next_page {
        offset = handle_next_page(conn);
        match offset {
            Offset::CreateTime(_) => {}
            Offset::None => return Err("No next page available".to_string()),
            _ => return Err("next page not meant for this call".to_string()),
        }
    }
    record_query = record_query.with_offset(offset);
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
    if let Some(search_term) = &cmd.search {
        task_query = task_query.with_content_like(search_term);
    }

    match cmd.status {
        // 255 status means we query all task items regardless of status.
        255 => {}
        // 254 status indicates a combination of statuses that are open
        254 => task_query = task_query.with_statuses(OPEN_STATUS_CODES.to_vec()),
        // 253 status indicates a combination of statuses that are closed
        253 => task_query = task_query.with_statuses(CLOSED_STATUS_CODES.to_vec()),
        // Other statuses are individual statuses for query
        _ => task_query = task_query.with_statuses(vec![cmd.status]),
    }

    let mut offset = Offset::None;
    if cmd.next_page {
        offset = handle_next_page(conn);
        match offset {
            Offset::TargetTime(_) => {}
            Offset::None => return Err("No next page available".to_string()),
            _ => return Err("next page not meant for this call".to_string()),
        }
    }
    task_query = task_query.with_offset(offset);
    task_query = task_query.with_limit(cmd.limit);
    task_query = task_query.with_order_by(TARGET_TIME_COL);
    query_items(conn, &task_query).map_err(|e| e.to_string())
}

fn handle_next_page(conn: &Connection) -> Offset {
    let offset_index = match cache::get_next_index(conn) {
        Ok(Some(index)) => index,
        Ok(None) | Err(_) => return Offset::None,
    };
    let end_item_index = match cache::read(conn, offset_index) {
        Ok(Some(index)) => index,
        Ok(None) | Err(_) => return Offset::None,
    };
    let end_item = match get_item(conn, end_item_index) {
        Ok(item) => item,
        Err(_) => return Offset::None,
    };
    if end_item.action == "task" {
        return Offset::TargetTime(end_item.target_time.unwrap());
    } else if end_item.action == "record" {
        return Offset::CreateTime(end_item.create_time);
    }
    Offset::None
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

    impl ListRecordCommand {
        fn default_test() -> Self {
            ListRecordCommand {
                category: None,
                days: None,
                limit: 100,
                starting_time: None,
                ending_time: None,
                next_page: false,
                search: None,
            }
        }

        fn with_category(mut self, category: &str) -> Self {
            self.category = Some(category.to_string());
            self
        }

        fn with_days(mut self, days: usize) -> Self {
            self.days = Some(days);
            self
        }

        fn with_starting_time(mut self, starting_time: &str) -> Self {
            self.starting_time = Some(starting_time.to_string());
            self
        }

        fn with_ending_time(mut self, ending_time: &str) -> Self {
            self.ending_time = Some(ending_time.to_string());
            self
        }

        fn with_search(mut self, search: &str) -> Self {
            self.search = Some(search.to_string());
            self
        }

        fn with_limit(mut self, limit: usize) -> Self {
            self.limit = limit;
            self
        }

        fn with_next_page(mut self) -> Self {
            self.next_page = true;
            self
        }
    }

    impl ListTaskCommand {
        fn default_test() -> Self {
            ListTaskCommand {
                timestr: None,
                category: None,
                days: None,
                status: 0,
                overdue: false,
                limit: 100,
                next_page: false,
                search: None,
            }
        }

        fn with_category(mut self, category: &str) -> Self {
            self.category = Some(category.to_string());
            self
        }

        fn with_status(mut self, status: u8) -> Self {
            self.status = status;
            self
        }

        fn with_overdue(mut self, overdue: bool) -> Self {
            self.overdue = overdue;
            self
        }

        fn with_limit(mut self, limit: usize) -> Self {
            self.limit = limit;
            self
        }

        fn with_next_page(mut self) -> Self {
            self.next_page = true;
            self
        }

        fn with_search(mut self, search: &str) -> Self {
            self.search = Some(search.to_string());
            self
        }
    }

    #[test]
    fn test_query_records() {
        let (conn, _temp_file) = get_test_conn();
        insert_record(&conn, "feeding", "100ML", "yesterday 2PM");
        insert_record(&conn, "feeding", "110ML", "yesterday 5PM");
        insert_record(&conn, "feeding", "100ML", "yesterday 9PM");
        insert_record(&conn, "FTP", "256W", "yesterday 7PM");

        let listfeeding = ListRecordCommand::default_test().with_category("feeding");
        let list_all = ListRecordCommand::default_test().with_days(2);
        let list_timeframe = ListRecordCommand::default_test()
            .with_starting_time("yesterday 4PM")
            .with_ending_time("yesterday 8PM");
        let list_timeframe_start_only =
            ListRecordCommand::default_test().with_starting_time("yesterday 8PM");

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

        let list_tasks_default = ListTaskCommand::default_test();
        let results = query_tasks(&conn, &list_tasks_default).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results.first().unwrap().content, "second_due");
        assert_eq!(results.last().unwrap().content, "third_due");

        let list_tasks_with_overdue = ListTaskCommand::default_test().with_overdue(true);
        let results = query_tasks(&conn, &list_tasks_with_overdue).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results.first().unwrap().content, "first_due");
    }

    #[test]
    fn test_query_records_pagination() {
        let (conn, _temp_file) = get_test_conn();
        for i in 1..=11 {
            insert_record(
                &conn,
                "test",
                &format!("B{}", i),
                &format!("2025/02/25 {}AM", i),
            );
            insert_record(
                &conn,
                "test",
                &format!("A{}", i),
                &format!("2025/02/23 {}PM", i),
            );
        }

        let list_record = ListRecordCommand::default_test()
            .with_category("test")
            .with_limit(11)
            .with_starting_time("2025/02/21")
            .with_ending_time("2025/02/27");

        let results = query_records(&conn, &list_record).unwrap();
        cache::clear(&conn).unwrap();
        cache::store_with_next(&conn, &results).unwrap();
        assert_eq!(results.len(), 11);
        assert!(results.iter().all(|i| i.content.contains("A")));

        let list_record_next = list_record.with_next_page();
        let results = query_records(&conn, &list_record_next).unwrap();
        cache::clear(&conn).unwrap();
        cache::store_with_next(&conn, &results).unwrap();
        assert_eq!(results.len(), 11);
        assert!(results.iter().all(|i| i.content.contains("B")));

        let results = query_records(&conn, &list_record_next).unwrap();
        cache::clear(&conn).unwrap();
        cache::store(&conn, &results).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_query_tasks_pagination() {
        let (conn, _temp_file) = get_test_conn();
        for i in 1..=11 {
            insert_task(
                &conn,
                "test",
                &format!("index {}PM", i),
                &format!("tomorrow {}PM", i),
            );
            insert_task(
                &conn,
                "test",
                &format!("index {}AM", i),
                &format!("tomorrow {}AM", i),
            );
        }

        let list_task = ListTaskCommand::default_test()
            .with_category("test")
            .with_limit(10);

        let results = query_tasks(&conn, &list_task).unwrap();
        cache::store_with_next(&conn, &results).unwrap();
        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|i| i.content.contains("AM")));

        let list_task_next = list_task.with_next_page();
        let results = query_tasks(&conn, &list_task_next).unwrap();

        cache::clear(&conn).unwrap();
        cache::store_with_next(&conn, &results).unwrap();
        assert_eq!(results.len(), 10);
        assert_eq!(results.first().unwrap().content, "index 11AM");
        assert_eq!(results.last().unwrap().content, "index 9PM");

        let results = query_tasks(&conn, &list_task_next).unwrap();

        cache::clear(&conn).unwrap();
        cache::store(&conn, &results).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results.first().unwrap().content, "index 10PM");
        assert_eq!(results.last().unwrap().content, "index 11PM");

        let results = query_tasks(&conn, &list_task_next);
        assert_eq!(results.unwrap_err(), "No next page available".to_string());
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

        let list_open = ListTaskCommand::default_test().with_status(254);
        let list_closed = ListTaskCommand::default_test().with_status(253);

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

    #[test]
    fn test_search_functionality() {
        let (conn, _temp_file) = get_test_conn();

        // Insert records with different content patterns
        insert_record(&conn, "feeding", "100ML bottle feed", "yesterday 2PM");
        insert_record(&conn, "feeding", "breast feeding session", "yesterday 5PM");
        insert_record(&conn, "feeding", "solid food banana", "yesterday 9PM");
        insert_record(&conn, "sleep", "bottle cleaning", "yesterday 7PM");
        insert_record(&conn, "diaper", "wet diaper change", "yesterday 3PM");

        // Insert tasks with different content patterns
        insert_task(&conn, "work", "team meeting scheduled", "today");
        insert_task(&conn, "work", "client meeting prep", "tomorrow");
        insert_task(&conn, "personal", "doctor appointment", "today");
        insert_task(&conn, "personal", "meeting friends", "tomorrow");
        insert_task(&conn, "home", "bottle sterilization", "today");

        // Test record search for "bottle" - should find 2 records
        let search_bottle_records = ListRecordCommand::default_test()
            .with_days(2)
            .with_search("bottle");
        let results = query_records(&conn, &search_bottle_records).unwrap();
        assert_eq!(results.len(), 2);
        for record in &results {
            assert!(record.content.contains("bottle"));
        }

        // Test record search for "feeding" - should find 1 record
        let search_feeding_records = ListRecordCommand::default_test()
            .with_days(2)
            .with_search("feeding");
        let results = query_records(&conn, &search_feeding_records).unwrap();
        assert_eq!(results.len(), 1);
        for record in &results {
            assert!(record.content.contains("feeding"));
        }

        // Test task search for "meeting" - should find 3 tasks
        let search_meeting_tasks = ListTaskCommand::default_test()
            .with_overdue(true)
            .with_search("meeting");
        let results = query_tasks(&conn, &search_meeting_tasks).unwrap();
        assert_eq!(results.len(), 3);
        for task in &results {
            assert!(task.content.contains("meeting"));
        }

        // Test combined search and category filter for records
        let search_feeding_category = ListRecordCommand::default_test()
            .with_category("feeding")
            .with_days(2)
            .with_search("feed");
        let results = query_records(&conn, &search_feeding_category).unwrap();
        assert_eq!(results.len(), 2);
        for record in &results {
            assert!(record.content.contains("feed"));
            assert_eq!(record.category, "feeding");
        }

        // Test combined search and category filter for tasks
        let search_work_meeting = ListTaskCommand::default_test()
            .with_category("work")
            .with_overdue(true)
            .with_search("meeting");
        let results = query_tasks(&conn, &search_work_meeting).unwrap();
        assert_eq!(results.len(), 2);
        for task in &results {
            assert!(task.content.contains("meeting"));
            assert_eq!(task.category, "work");
        }

        // Test search that returns no results
        let search_nonexistent = ListRecordCommand::default_test()
            .with_days(2)
            .with_search("nonexistent");
        let results = query_records(&conn, &search_nonexistent).unwrap();
        assert_eq!(results.len(), 0);

        // Test partial word matching
        let search_partial = ListRecordCommand::default_test()
            .with_days(2)
            .with_search("banan");
        let results = query_records(&conn, &search_partial).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("banana"));
    }
}
