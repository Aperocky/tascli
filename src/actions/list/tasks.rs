use chrono::Local;
use rusqlite::Connection;

use super::{
    handle_next_page,
    CLOSED_STATUS_CODES,
    OPEN_STATUS_CODES,
    TARGET_TIME_COL,
};
use crate::{
    actions::display,
    args::{
        cron,
        parser::ListTaskCommand,
        timestr,
    },
    db::{
        cache,
        crud::query_items,
        item::{
            Item,
            ItemQuery,
            Offset,
            RECURRING_TASK,
            RECURRING_TASK_RECORD,
            TASK,
        },
    },
};

pub fn handle_listtasks(conn: &Connection, cmd: ListTaskCommand) -> Result<(), String> {
    let recurring_tasks = match query_recurring_tasks(conn, &cmd) {
        Ok(tasks) => tasks,
        Err(estr) => {
            display::print_bold(&estr);
            return Ok(());
        }
    };

    let recurring_hit_limit = recurring_tasks.len() == cmd.limit;
    let last_queried_recurring = if recurring_hit_limit {
        recurring_tasks.last().cloned()
    } else {
        None
    };

    // Mark completion status for all recurring tasks
    let recurring_tasks = mark_recurring_task_by_completion(conn, recurring_tasks)?;
    let recurring_tasks = if cmd.status == 255 {
        recurring_tasks
    } else if cmd.status == 253 || cmd.status == 1 {
        // 253 = closed statuses; 1 = done, show only completed tasks
        recurring_tasks
            .into_iter()
            .filter(|t| t.recurring_interval_complete)
            .collect()
    } else {
        // All other statuses show only incomplete tasks
        recurring_tasks
            .into_iter()
            .filter(|t| !t.recurring_interval_complete)
            .collect()
    };
    let recurring_tasks = filter_recurring_task_by_time(recurring_tasks, &cmd)?;

    let all_tasks = if recurring_hit_limit {
        // If recurring tasks hit the limit, don't query regular tasks yet
        // There might be more recurring tasks on the next page
        recurring_tasks
    } else {
        // Recurring tasks didn't hit limit, safe to query and combine with regular tasks
        let regular_tasks = match query_tasks(conn, &cmd) {
            Ok(tasks) => tasks,
            Err(estr) => {
                display::print_bold(&estr);
                return Ok(());
            }
        };

        // Combine both lists
        let mut all_tasks = Vec::new();
        all_tasks.extend(recurring_tasks);
        all_tasks.extend(regular_tasks);
        all_tasks.truncate(cmd.limit);

        all_tasks
    };

    if all_tasks.is_empty() {
        display::print_bold("No tasks found");
        return Ok(());
    }

    // given we have filtering, the cache must store
    // all items queried even if they had been filtered
    let mut cache_items = all_tasks.clone();
    if recurring_hit_limit {
        if let Some(last_queried) = &last_queried_recurring {
            if all_tasks.last().map(|t| t.id) != Some(last_queried.id) {
                cache_items.push(last_queried.clone());
            }
        }
    }

    cache::clear(conn).map_err(|e| e.to_string())?;
    if recurring_hit_limit || cache_items.len() == cmd.limit {
        cache::store_with_next(conn, &cache_items)
    } else {
        cache::store(conn, &cache_items)
    }
    .map_err(|e| e.to_string())?;

    display::print_bold("Tasks List:");
    display::print_items(&all_tasks, false, true);
    Ok(())
}

// Some cmd query argument do not apply - moved to application layer.
// Skip query for status because recurring tasks do not have status.
fn query_recurring_tasks(conn: &Connection, cmd: &ListTaskCommand) -> Result<Vec<Item>, String> {
    let mut query = ItemQuery::new().with_action(RECURRING_TASK);
    if let Some(cat) = &cmd.category {
        query = query.with_category(cat);
    }
    if let Some(search_term) = &cmd.search {
        query = query.with_content_like(search_term);
    }
    let mut offset = Offset::None;
    if cmd.next_page {
        offset = handle_next_page(conn);
        match offset {
            Offset::Id(_) => {}
            Offset::None => return Err("No next page available".to_string()),
            _ => return Ok(Vec::new()), // Wrong offset type, skip recurring tasks query
        }
    }
    match cmd.status {
        // For open, done, closed and all we capture all items
        // to be filtered at handler level.
        1 | 253 | 254 | 255 => {}
        // retain other specific status query
        _ => query = query.with_statuses(vec![cmd.status]),
    }
    query = query.with_offset(offset);
    query = query.with_limit(cmd.limit);
    query_items(conn, &query).map_err(|e| e.to_string())
}

fn filter_recurring_task_by_time(
    recurring_tasks: Vec<Item>,
    cmd: &ListTaskCommand,
) -> Result<Vec<Item>, String> {
    let mut filtered_tasks: Vec<Item> = Vec::new();
    let mut target_interval_end: Option<i64> = Option::None;
    if let Some(t) = &cmd.timestr {
        target_interval_end = Some(timestr::to_unix_epoch(t)?);
    } else if let Some(days) = cmd.days {
        target_interval_end = Some(timestr::days_after_to_unix_epoch(days));
    }
    match target_interval_end {
        Some(et) => {
            for recurring_task in recurring_tasks {
                let cron_schedule = recurring_task.cron_schedule.as_ref().unwrap();
                let next_occurrence = cron::get_next_occurrence(cron_schedule)?;
                if next_occurrence < et {
                    filtered_tasks.push(recurring_task);
                }
            }
            Ok(filtered_tasks)
        }
        None => Ok(recurring_tasks),
    }
}

fn mark_recurring_task_by_completion(
    conn: &Connection,
    mut recurring_tasks: Vec<Item>,
) -> Result<Vec<Item>, String> {
    for recurring_task in &mut recurring_tasks {
        let cron_schedule = recurring_task.cron_schedule.as_ref().unwrap();
        let last_occurrence = cron::get_last_occurrence(cron_schedule)?;
        let recurring_task_id = recurring_task.id.unwrap();

        // Query for recurring_task_record that covers this interval
        let record_query = ItemQuery::new()
            .with_action(RECURRING_TASK_RECORD)
            .with_recurring_task_id(recurring_task_id)
            .with_good_until_min(last_occurrence);
        let records = query_items(conn, &record_query).map_err(|e| e.to_string())?;
        recurring_task.recurring_interval_complete = !records.is_empty();
    }
    Ok(recurring_tasks)
}

fn query_tasks(conn: &Connection, cmd: &ListTaskCommand) -> Result<Vec<Item>, String> {
    let mut task_query = ItemQuery::new().with_action(TASK);
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
            Offset::Id(_) => offset = Offset::None, // Transition from recurring to regular tasks
            Offset::None => return Err("No next page available".to_string()),
            _ => return Ok(Vec::new()), // Wrong offset type, skip regular tasks query
        }
    }
    task_query = task_query.with_offset(offset);
    task_query = task_query.with_limit(cmd.limit);
    task_query = task_query.with_order_by(TARGET_TIME_COL);
    query_items(conn, &task_query).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        get_test_conn,
        insert_recurring_record,
        insert_recurring_task,
        insert_task,
        update_status,
    };

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
    fn test_query_recurring_tasks() {
        let (conn, _temp_file) = get_test_conn();

        // Insert recurring tasks
        insert_recurring_task(&conn, "work", "Daily standup", "Daily 9AM");
        insert_recurring_task(&conn, "work", "Weekly review", "Weekly Monday 2PM");
        insert_recurring_task(&conn, "personal", "Exercise", "Daily 7AM");

        // Test basic query
        let list_all = ListTaskCommand::default_test();
        let results = query_recurring_tasks(&conn, &list_all).unwrap();
        assert_eq!(results.len(), 3);

        // Test category filter
        let list_work = ListTaskCommand::default_test().with_category("work");
        let results = query_recurring_tasks(&conn, &list_work).unwrap();
        assert_eq!(results.len(), 2);
        for task in &results {
            assert_eq!(task.category, "work");
        }

        // Test search filter
        let list_search = ListTaskCommand::default_test().with_search("standup");
        let results = query_recurring_tasks(&conn, &list_search).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("standup"));

        // Test limit
        let list_limited = ListTaskCommand::default_test().with_limit(2);
        let results = query_recurring_tasks(&conn, &list_limited).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_filter_recurring_task_by_time() {
        let (conn, _temp_file) = get_test_conn();

        // Insert recurring tasks with different schedules
        insert_recurring_task(&conn, "work", "Daily task", "Daily 9AM");
        insert_recurring_task(&conn, "work", "Weekly task", "Weekly Monday 9AM");
        insert_recurring_task(&conn, "work", "Monthly task", "Monthly 1st 9AM");

        // Query all recurring tasks
        let cmd = ListTaskCommand::default_test();
        let all_tasks = query_recurring_tasks(&conn, &cmd).unwrap();
        assert_eq!(all_tasks.len(), 3);

        // Test with no time filter (should return all)
        let results = filter_recurring_task_by_time(all_tasks.clone(), &cmd).unwrap();
        assert_eq!(results.len(), 3);

        // Test with days filter (7 days from now)
        let cmd_days = ListTaskCommand::default_test().with_overdue(false);
        let cmd_days = ListTaskCommand {
            days: Some(7),
            ..cmd_days
        };
        let results = filter_recurring_task_by_time(all_tasks.clone(), &cmd_days).unwrap();
        // Daily and weekly tasks should have next occurrence within 7 days
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_filter_recurring_task_by_completion() {
        let (conn, _temp_file) = get_test_conn();

        // Insert recurring tasks
        let task1_id = insert_recurring_task(&conn, "work", "Daily standup", "Daily 9AM");
        let task2_id = insert_recurring_task(&conn, "work", "Weekly review", "Weekly Monday 2PM");
        let task3_id = insert_recurring_task(&conn, "personal", "Exercise", "Daily 7AM");

        // Insert completion records for task1 and task2
        // Using good_until that's in the future to mark as completed
        let future_time = Local::now().timestamp() + 86400; // Tomorrow
        insert_recurring_record(&conn, "work", "Standup done", task1_id, future_time);
        insert_recurring_record(&conn, "work", "Review done", task2_id, future_time);

        // Query all recurring tasks
        let cmd = ListTaskCommand::default_test();
        let all_tasks = query_recurring_tasks(&conn, &cmd).unwrap();
        assert_eq!(all_tasks.len(), 3);

        // Mark completion status
        let marked_tasks = mark_recurring_task_by_completion(&conn, all_tasks).unwrap();
        assert_eq!(marked_tasks.len(), 3);

        // Verify completion flags are set correctly
        for task in &marked_tasks {
            let task_id = task.id.unwrap();
            if task_id == task1_id || task_id == task2_id {
                assert!(
                    task.recurring_interval_complete,
                    "Task {} should be marked complete",
                    task_id
                );
            } else if task_id == task3_id {
                assert!(
                    !task.recurring_interval_complete,
                    "Task {} should be marked incomplete",
                    task_id
                );
            }
        }
    }

    #[test]
    fn test_handle_listtasks_with_recurring() {
        let (conn, _temp_file) = get_test_conn();

        // Insert recurring tasks
        insert_recurring_task(&conn, "work", "Daily standup", "Daily 9AM");
        insert_recurring_task(&conn, "work", "Weekly review", "Weekly Monday 2PM");

        // Insert regular tasks
        insert_task(&conn, "work", "Finish report", "tomorrow");
        insert_task(&conn, "personal", "Buy groceries", "today");

        // Test listing all tasks (status 255)
        let cmd = ListTaskCommand {
            status: 255,
            ..ListTaskCommand::default_test()
        };
        let result = handle_listtasks(&conn, cmd);
        assert!(result.is_ok());

        // Verify cache was populated
        assert!(cache::validate_cache(&conn).unwrap());
    }

    #[test]
    fn test_list_tasks_pagination_with_recurring() {
        let (conn, _temp_file) = get_test_conn();

        // Insert 5 recurring tasks
        insert_recurring_task(&conn, "work", "Task 1", "Daily 9AM");
        let task2_id = insert_recurring_task(&conn, "work", "Task 2", "Daily 10AM");
        insert_recurring_task(&conn, "work", "Task 3", "Daily 11AM");
        let task4_id = insert_recurring_task(&conn, "work", "Task 4", "Daily 12PM");
        insert_recurring_task(&conn, "work", "Task 5", "Daily 1PM");

        // Mark tasks 2 and 4 as complete
        let future_time = Local::now().timestamp() + 86400;
        insert_recurring_record(&conn, "work", "Task 2 done", task2_id, future_time);
        insert_recurring_record(&conn, "work", "Task 4 done", task4_id, future_time);

        // First page: Query incomplete tasks with limit 2
        // Should query tasks 1-2, filter out task 2, display task 1
        let cmd = ListTaskCommand {
            limit: 2,
            status: 0, // incomplete only
            ..ListTaskCommand::default_test()
        };

        let result = handle_listtasks(&conn, cmd);
        assert!(result.is_ok());
        assert!(cache::validate_cache(&conn).unwrap());

        // Second page: Should query tasks 3-4, filter out task 4, display task 3
        let cmd_next = ListTaskCommand {
            limit: 2,
            status: 0,
            next_page: true,
            ..ListTaskCommand::default_test()
        };

        let result = handle_listtasks(&conn, cmd_next);
        assert!(result.is_ok());
        assert!(cache::validate_cache(&conn).unwrap());
    }

    #[test]
    fn test_pagination_transition_recurring_to_regular() {
        let (conn, _temp_file) = get_test_conn();

        // Insert 3 recurring tasks and 5 regular tasks
        insert_recurring_task(&conn, "work", "Recurring 1", "Daily 9AM");
        insert_recurring_task(&conn, "work", "Recurring 2", "Daily 10AM");
        insert_recurring_task(&conn, "work", "Recurring 3", "Daily 11AM");

        for i in 1..=5 {
            insert_task(&conn, "work", &format!("Regular task {}", i), "tomorrow");
        }

        // First page: limit=2, should get 2 recurring tasks
        let cmd = ListTaskCommand {
            limit: 2,
            status: 255, // all
            ..ListTaskCommand::default_test()
        };
        let result = handle_listtasks(&conn, cmd);
        assert!(result.is_ok());

        // Second page: should get last recurring + first regular (transition page)
        let cmd_next = ListTaskCommand {
            limit: 2,
            status: 255,
            next_page: true,
            ..ListTaskCommand::default_test()
        };
        let recurring_and_regular = query_recurring_tasks(&conn, &cmd_next).unwrap();
        let regular_tasks = query_tasks(&conn, &cmd_next).unwrap();

        // Should have 1 recurring task left (Recurring 3)
        assert_eq!(recurring_and_regular.len(), 1);
        assert_eq!(recurring_and_regular[0].content, "Recurring 3");

        // Should start getting regular tasks (didn't hit recurring limit)
        assert!(regular_tasks.len() > 0);

        // Third page: should transition to regular tasks (not "No tasks found")
        let cmd_next = ListTaskCommand {
            limit: 2,
            status: 255,
            next_page: true,
            ..ListTaskCommand::default_test()
        };
        let result = handle_listtasks(&conn, cmd_next);
        assert!(result.is_ok()); // Should succeed and show regular tasks
    }

    #[test]
    fn test_handle_listtasks_status_filtering() {
        let (conn, _temp_file) = get_test_conn();

        // Insert recurring tasks
        let task1_id = insert_recurring_task(&conn, "work", "Daily standup", "Daily 9AM");
        let _task2_id = insert_recurring_task(&conn, "work", "Weekly review", "Weekly Monday 2PM");

        // Mark task1 as completed with a record
        let future_time = Local::now().timestamp() + 86400;
        insert_recurring_record(&conn, "work", "Standup done", task1_id, future_time);

        // Test with status 253 (closed/completed)
        let cmd_closed = ListTaskCommand {
            status: 253,
            ..ListTaskCommand::default_test()
        };
        let result = handle_listtasks(&conn, cmd_closed);
        assert!(result.is_ok());
        // Should show completed recurring tasks
        assert!(cache::validate_cache(&conn).unwrap());

        cache::clear(&conn).unwrap();

        // Test with status 0 (incomplete/ongoing)
        let cmd_open = ListTaskCommand {
            status: 0,
            ..ListTaskCommand::default_test()
        };
        let result = handle_listtasks(&conn, cmd_open);
        assert!(result.is_ok());
        // Should show incomplete recurring tasks plus any regular tasks
        assert!(cache::validate_cache(&conn).unwrap());
    }

    #[test]
    fn test_pagination_mixed_task_types() {
        let (conn, _temp_file) = get_test_conn();

        // Insert 3 recurring tasks (won't hit limit of 10)
        insert_recurring_task(&conn, "work", "Recurring 1", "Daily 9AM");
        insert_recurring_task(&conn, "work", "Recurring 2", "Daily 10AM");
        insert_recurring_task(&conn, "work", "Recurring 3", "Daily 11AM");

        // Insert 15 regular tasks
        for i in 1..=12 {
            insert_task(
                &conn,
                "work",
                &format!("Regular task {}", i),
                &format!("tomorrow {}PM", i),
            );
        }
        for i in 13..=15 {
            insert_task(
                &conn,
                "work",
                &format!("Regular task {}", i),
                &format!("tomorrow {}AM", i - 12),
            );
        }

        // First page: Should get 3 recurring + 7 regular = 10 total
        let cmd = ListTaskCommand {
            limit: 10,
            status: 255, // Show all
            ..ListTaskCommand::default_test()
        };
        let result = handle_listtasks(&conn, cmd);
        assert!(result.is_ok());
        assert!(cache::validate_cache(&conn).unwrap());

        // Second page: Should get remaining 8 regular tasks
        // This is where the bug was - offset type would be TargetTime (from last regular task)
        // query_recurring_tasks would error, but now it returns empty Vec
        let cmd_page2 = ListTaskCommand {
            limit: 10,
            status: 255,
            next_page: true,
            ..ListTaskCommand::default_test()
        };
        let result = handle_listtasks(&conn, cmd_page2);
        assert!(result.is_ok());
        assert!(cache::validate_cache(&conn).unwrap());
    }

    #[test]
    fn test_search_functionality() {
        let (conn, _temp_file) = get_test_conn();

        // Insert tasks with different content patterns
        insert_task(&conn, "work", "team meeting scheduled", "today");
        insert_task(&conn, "work", "client meeting prep", "tomorrow");
        insert_task(&conn, "personal", "doctor appointment", "today");
        insert_task(&conn, "personal", "meeting friends", "tomorrow");
        insert_task(&conn, "home", "bottle sterilization", "today");

        // Test task search for "meeting" - should find 3 tasks
        let search_meeting_tasks = ListTaskCommand::default_test()
            .with_overdue(true)
            .with_search("meeting");
        let results = query_tasks(&conn, &search_meeting_tasks).unwrap();
        assert_eq!(results.len(), 3);
        for task in &results {
            assert!(task.content.contains("meeting"));
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
    }
}
