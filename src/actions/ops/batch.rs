use rusqlite::Connection;
use std::io::{self, Write};

use crate::{
    actions::display,
    args::{
        parser::OpsBatchCommand,
        timestr,
    },
    db::{
        crud::query_items,
        item::ItemQuery,
        ops::{batch_delete_items, batch_update_items, ItemUpdates},
    },
};

pub fn handle_batchcmd(conn: &Connection, cmd: &OpsBatchCommand) -> Result<(), String> {
    // Skip interactive mode for now
    if cmd.interactive {
        return Err("Interactive mode is not yet supported".to_string());
    }

    // Parse action filter
    let actions = parse_action_filter(&cmd.action)?;

    // Validate restrictions for "all"
    if cmd.action == "all" {
        if cmd.status_to.is_some() {
            return Err("Cannot set status when action is 'all' (status only applies to tasks)".to_string());
        }
        if cmd.target_time_to.is_some() {
            return Err("Cannot set target_time when action is 'all' (target_time only applies to tasks)".to_string());
        }
    }

    // Validate at least one operation is specified
    if !cmd.delete && cmd.category_to.is_none() && cmd.status_to.is_none() && cmd.target_time_to.is_none() {
        return Err("Must specify at least one operation: --delete, --category-to, --status-to, or --target-time-to".to_string());
    }

    // Parse time filters
    let create_time_min = if let Some(ref starting_time) = cmd.starting_time {
        Some(timestr::to_unix_epoch(starting_time)?)
    } else {
        None
    };

    let create_time_max = if let Some(ref ending_time) = cmd.ending_time {
        Some(timestr::to_unix_epoch(ending_time)?)
    } else {
        None
    };

    // Query items to preview what will be affected
    let items = query_items_for_batch(
        conn,
        actions.as_ref(),
        cmd.category.as_deref(),
        create_time_min,
        create_time_max,
    )?;

    if items.is_empty() {
        display::print_bold("No items found matching the filters");
        return Ok(());
    }

    // Extract IDs - these are exactly the items we'll operate on
    let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();

    // Display items that will be affected
    display::print_bold(&format!("Found {} items matching filters:", items.len()));
    // Determine if items are records or tasks for display
    let has_records = items.iter().any(|i| i.action == "record" || i.action == "recurring_task_record");
    let has_tasks = items.iter().any(|i| i.action == "task" || i.action == "recurring_task");

    // Display appropriately (if mixed, show as tasks since display format works for both)
    display::print_items(&items, has_records && !has_tasks, false);

    // Show what operation will be performed
    println!();
    if cmd.delete {
        display::print_red("⚠ WARNING: This will DELETE the above items permanently!");
    } else {
        display::print_bold("This will update the above items:");
        if let Some(ref cat) = cmd.category_to {
            println!("  • Change category to: {}", cat);
        }
        if let Some(status) = cmd.status_to {
            println!("  • Change status to: {}", status_to_string(status));
        }
        if let Some(ref time) = cmd.target_time_to {
            println!("  • Change target_time to: {}", time);
        }
    }

    // Ask for confirmation
    print!("\nProceed? (y/n): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;

    if input.trim().to_lowercase() != "y" {
        display::print_bold("Cancelled");
        return Ok(());
    }

    // Execute the batch operation on the exact IDs we showed
    let affected = if cmd.delete {
        batch_delete_items(conn, &item_ids).map_err(|e| e.to_string())?
    } else {
        let target_time_to = if let Some(ref time_str) = cmd.target_time_to {
            Some(timestr::to_unix_epoch(time_str)?)
        } else {
            None
        };

        let updates = ItemUpdates {
            category: cmd.category_to.clone(),
            status: cmd.status_to,
            target_time: target_time_to,
        };

        batch_update_items(conn, &item_ids, &updates).map_err(|e| e.to_string())?
    };

    display::print_bold(&format!("✓ Successfully affected {} items", affected));
    Ok(())
}

fn parse_action_filter(action: &str) -> Result<Option<Vec<String>>, String> {
    match action {
        "all" => Ok(None),
        "task" => Ok(Some(vec!["task".to_string()])),
        "record" => Ok(Some(vec!["record".to_string()])),
        "recurring_task" => Ok(Some(vec!["recurring_task".to_string()])),
        "recurring_task_record" => Ok(Some(vec!["recurring_task_record".to_string()])),
        _ => Err(format!(
            "Invalid action type: '{}'. Expected: all, task, record, recurring_task, or recurring_task_record",
            action
        )),
    }
}

fn query_items_for_batch(
    conn: &Connection,
    actions: Option<&Vec<String>>,
    category: Option<&str>,
    create_time_min: Option<i64>,
    create_time_max: Option<i64>,
) -> Result<Vec<crate::db::item::Item>, String> {
    let mut query = ItemQuery::new();

    if let Some(acts) = actions {
        let action_strs: Vec<&str> = acts.iter().map(|s| s.as_str()).collect();
        query = query.with_actions(action_strs);
    }

    if let Some(cat) = category {
        query = query.with_category(cat);
    }

    if let Some(min) = create_time_min {
        query = query.with_create_time_min(min);
    }

    if let Some(max) = create_time_max {
        query = query.with_create_time_max(max);
    }

    // No limit for batch operations - we want to see everything that will be affected
    query_items(conn, &query).map_err(|e| e.to_string())
}

fn status_to_string(status: u8) -> &'static str {
    match status {
        0 => "ongoing",
        1 => "completed",
        2 => "cancelled",
        3 => "duplicate",
        4 => "suspended",
        5 => "removed",
        6 => "pending",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        get_test_conn,
        insert_record,
        insert_recurring_record,
        insert_recurring_task,
        insert_task,
        update_status,
    };
    use crate::db::crud::query_items;
    use crate::db::item::ItemQuery;

    #[test]
    fn test_parse_action_filter() {
        assert_eq!(parse_action_filter("all").unwrap(), None);
        assert_eq!(
            parse_action_filter("task").unwrap(),
            Some(vec!["task".to_string()])
        );
        assert_eq!(
            parse_action_filter("record").unwrap(),
            Some(vec!["record".to_string()])
        );
        assert_eq!(
            parse_action_filter("recurring_task").unwrap(),
            Some(vec!["recurring_task".to_string()])
        );
        assert_eq!(
            parse_action_filter("recurring_task_record").unwrap(),
            Some(vec!["recurring_task_record".to_string()])
        );
        assert!(parse_action_filter("invalid").is_err());
    }

    #[test]
    fn test_query_items_for_batch() {
        let (conn, _temp_file) = get_test_conn();

        // Insert test data
        insert_task(&conn, "work", "task 1", "today");
        insert_task(&conn, "work", "task 2", "today");
        insert_task(&conn, "personal", "task 3", "today");
        insert_record(&conn, "work", "record 1", "yesterday");
        insert_record(&conn, "personal", "record 2", "yesterday");

        // Test 1: Query all items
        let items = query_items_for_batch(&conn, None, None, None, None).unwrap();
        assert_eq!(items.len(), 5);

        // Test 2: Filter by action
        let actions = Some(vec!["task".to_string()]);
        let items = query_items_for_batch(&conn, actions.as_ref(), None, None, None).unwrap();
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.action == "task"));

        // Test 3: Filter by category
        let items = query_items_for_batch(&conn, None, Some("work"), None, None).unwrap();
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.category == "work"));

        // Test 4: Combine filters
        let actions = Some(vec!["task".to_string()]);
        let items = query_items_for_batch(&conn, actions.as_ref(), Some("work"), None, None).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.action == "task" && i.category == "work"));
    }

    #[test]
    fn test_batch_update_category() {
        let (conn, _temp_file) = get_test_conn();

        // Insert test data
        insert_task(&conn, "work", "task 1", "today");
        insert_task(&conn, "work", "task 2", "today");
        insert_record(&conn, "work", "record 1", "yesterday");

        // Query work tasks
        let actions = Some(vec!["task".to_string()]);
        let items = query_items_for_batch(&conn, actions.as_ref(), Some("work"), None, None).unwrap();
        assert_eq!(items.len(), 2);

        // Extract IDs and update category from "work" to "personal"
        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        let updates = ItemUpdates {
            category: Some("personal".to_string()),
            status: None,
            target_time: None,
        };
        let affected = batch_update_items(&conn, &item_ids, &updates).unwrap();

        assert_eq!(affected, 2);

        // Verify the tasks were updated
        let mut query = ItemQuery::new();
        query = query.with_category("personal").with_actions(vec!["task"]);
        let items = query_items(&conn, &query).unwrap();
        assert_eq!(items.len(), 2);

        // Verify the record was NOT touched (still in "work")
        let mut query = ItemQuery::new();
        query = query.with_category("work").with_actions(vec!["record"]);
        let items = query_items(&conn, &query).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_batch_update_status() {
        let (conn, _temp_file) = get_test_conn();

        // Insert test data
        let id1 = insert_task(&conn, "work", "task 1", "today");
        let id2 = insert_task(&conn, "work", "task 2", "today");
        update_status(&conn, id1, 0); // ongoing
        update_status(&conn, id2, 0); // ongoing

        // Query work tasks
        let actions = Some(vec!["task".to_string()]);
        let items = query_items_for_batch(&conn, actions.as_ref(), Some("work"), None, None).unwrap();
        assert_eq!(items.len(), 2);

        // Extract IDs and update status to completed
        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        let updates = ItemUpdates {
            category: None,
            status: Some(1), // completed
            target_time: None,
        };
        let affected = batch_update_items(&conn, &item_ids, &updates).unwrap();

        assert_eq!(affected, 2);

        // Verify the update
        let mut query = ItemQuery::new();
        query = query.with_category("work").with_actions(vec!["task"]);
        let items = query_items(&conn, &query).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.status == 1));
    }

    #[test]
    fn test_batch_delete() {
        let (conn, _temp_file) = get_test_conn();

        // Insert test data
        insert_task(&conn, "work", "task 1", "today");
        insert_task(&conn, "work", "task 2", "today");
        insert_task(&conn, "personal", "task 3", "today");

        // Query work tasks
        let actions = Some(vec!["task".to_string()]);
        let items = query_items_for_batch(&conn, actions.as_ref(), Some("work"), None, None).unwrap();
        assert_eq!(items.len(), 2);

        // Extract IDs and delete
        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        let affected = batch_delete_items(&conn, &item_ids).unwrap();

        assert_eq!(affected, 2);

        // Verify deletion
        let mut query = ItemQuery::new();
        query = query.with_actions(vec!["task"]);
        let items = query_items(&conn, &query).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category, "personal");
    }

    #[test]
    fn test_batch_with_recurring() {
        let (conn, _temp_file) = get_test_conn();

        // Insert test data
        insert_task(&conn, "work", "task 1", "today");
        let rt_id = insert_recurring_task(&conn, "work", "standup", "Daily 9AM");
        insert_recurring_record(&conn, "work", "standup done", rt_id, 1000);

        // Query all work items (should include recurring)
        let items = query_items_for_batch(&conn, None, Some("work"), None, None).unwrap();
        assert_eq!(items.len(), 3); // 1 task + 1 recurring_task + 1 recurring_task_record

        // Extract IDs and update category for all work items
        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        let updates = ItemUpdates {
            category: Some("standup".to_string()),
            status: None,
            target_time: None,
        };
        let affected = batch_update_items(&conn, &item_ids, &updates).unwrap();

        assert_eq!(affected, 3);

        // Verify all moved to new category
        let items = query_items_for_batch(&conn, None, Some("standup"), None, None).unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_batch_with_time_filters() {
        let (conn, _temp_file) = get_test_conn();

        // Insert records with specific times
        insert_record(&conn, "test", "record 1", "2025/02/20 10AM");
        insert_record(&conn, "test", "record 2", "2025/02/25 10AM");
        insert_record(&conn, "test", "record 3", "2025/02/28 10AM");

        // Query with time range
        let start_time = timestr::to_unix_epoch("2025/02/22").unwrap();
        let end_time = timestr::to_unix_epoch("2025/02/27").unwrap();
        let items = query_items_for_batch(
            &conn,
            None,
            Some("test"),
            Some(start_time),
            Some(end_time),
        )
        .unwrap();

        // Should only get record 2
        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("record 2"));

        // Extract IDs and delete items in time range
        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        let affected = batch_delete_items(&conn, &item_ids).unwrap();

        assert_eq!(affected, 1);

        // Verify only 2 records remain
        let items = query_items_for_batch(&conn, None, Some("test"), None, None).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_batch_no_items_found() {
        let (conn, _temp_file) = get_test_conn();

        insert_task(&conn, "work", "task 1", "today");

        // Query from non-existent category
        let actions = Some(vec!["task".to_string()]);
        let items = query_items_for_batch(&conn, actions.as_ref(), Some("nonexistent"), None, None).unwrap();
        assert_eq!(items.len(), 0);

        // Extract IDs and try to delete (should be empty)
        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        let affected = batch_delete_items(&conn, &item_ids).unwrap();

        assert_eq!(affected, 0);
    }

    #[test]
    fn test_batch_multiple_updates() {
        let (conn, _temp_file) = get_test_conn();

        // Insert test data
        insert_task(&conn, "work", "task 1", "today");
        insert_task(&conn, "work", "task 2", "today");

        // Query work tasks
        let actions = Some(vec!["task".to_string()]);
        let items = query_items_for_batch(&conn, actions.as_ref(), Some("work"), None, None).unwrap();
        assert_eq!(items.len(), 2);

        // Extract IDs and update category, status, and target time
        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        let target_time = timestr::to_unix_epoch("tomorrow").unwrap();
        let updates = ItemUpdates {
            category: Some("urgent".to_string()),
            status: Some(1), // completed
            target_time: Some(target_time),
        };
        let affected = batch_update_items(&conn, &item_ids, &updates).unwrap();

        assert_eq!(affected, 2);

        // Verify all updates applied
        let mut query = ItemQuery::new();
        query = query.with_category("urgent").with_actions(vec!["task"]);
        let items = query_items(&conn, &query).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.status == 1));
        assert!(items.iter().all(|i| i.target_time == Some(target_time)));
    }

    #[test]
    fn test_batch_with_mixed_tasks_and_records() {
        let (conn, _temp_file) = get_test_conn();

        // Insert mixed items (tasks and records) in same category
        insert_task(&conn, "personal", "task 1", "today");
        insert_task(&conn, "personal", "task 2", "tomorrow");
        insert_record(&conn, "personal", "record 1", "yesterday");
        insert_record(&conn, "personal", "record 2", "today");

        // Query all personal items (mix of tasks and records)
        let items = query_items_for_batch(&conn, None, Some("personal"), None, None).unwrap();
        assert_eq!(items.len(), 4);

        // Extract IDs and update category
        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        let updates = ItemUpdates {
            category: Some("person".to_string()),
            status: None,
            target_time: None,
        };
        let affected = batch_update_items(&conn, &item_ids, &updates).unwrap();

        assert_eq!(affected, 4);

        // Verify all items moved to new category
        let items = query_items_for_batch(&conn, None, Some("person"), None, None).unwrap();
        assert_eq!(items.len(), 4);
        assert_eq!(items.iter().filter(|i| i.action == "task").count(), 2);
        assert_eq!(items.iter().filter(|i| i.action == "record").count(), 2);
    }
}
