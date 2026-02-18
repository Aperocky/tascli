use std::io::{
    self,
    Write,
};

use rusqlite::Connection;

use crate::{
    actions::{
        display,
        list::{
            CLOSED_STATUS_CODES,
            OPEN_STATUS_CODES,
        },
        ops::backup::backup_path,
    },
    args::{
        parser::OpsBatchCommand,
        timestr,
    },
    db::{
        crud::query_items,
        item::{
            Item,
            ItemQuery,
        },
        ops::{
            batch_delete_items,
            batch_update_items,
            ItemUpdates,
        },
    },
};

pub fn handle_batchcmd(conn: &Connection, cmd: &OpsBatchCommand) -> Result<(), String> {
    let actions = parse_action_filter(&cmd.action)?;

    if cmd.action != "task"
        && (cmd.status_to.is_some() || cmd.target_time_to.is_some() || cmd.status.is_some())
    {
        return Err(
            "--status, --status-to and --target-time-to are only valid with --action task"
                .to_string(),
        );
    }

    if cmd.status_to.is_some_and(|s| s >= 240) {
        return Err(
            "--status-to requires a concrete status: ongoing|done|cancelled|duplicate|suspended|removed|pending"
                .to_string(),
        );
    }

    if !cmd.delete
        && cmd.category_to.is_none()
        && cmd.status_to.is_none()
        && cmd.target_time_to.is_none()
    {
        return Err(
            "Must specify an operation: --delete, --category-to, --status-to, or --target-time-to"
                .to_string(),
        );
    }

    let create_time_min = cmd
        .starting_time
        .as_ref()
        .map(|t| timestr::to_unix_epoch(t))
        .transpose()?;
    let create_time_max = cmd
        .ending_time
        .as_ref()
        .map(|t| timestr::to_unix_epoch(t))
        .transpose()?;

    let items = query_items_for_batch(
        conn,
        actions.as_ref(),
        cmd.category.as_deref(),
        cmd.status,
        create_time_min,
        create_time_max,
    )?;

    if items.is_empty() {
        display::print_bold("No items found matching the filters");
        return Ok(());
    }

    if items.len() > 1 {
        display::print_bold("backing up database prior to batch operation");
        if let Err(e) = backup_path(None) {
            display::print_red(&e);
        }
    }

    let updates = if cmd.delete {
        None
    } else {
        let target_time = cmd
            .target_time_to
            .as_ref()
            .map(|t| timestr::to_unix_epoch(t))
            .transpose()?;
        Some(ItemUpdates {
            category: cmd.category_to.clone(),
            status: cmd.status_to,
            target_time,
        })
    };

    if cmd.interactive {
        execute_interactive(conn, &items, cmd, updates.as_ref())
    } else {
        execute_bulk(conn, &items, cmd, updates.as_ref())
    }
}

fn execute_bulk(
    conn: &Connection,
    items: &[Item],
    cmd: &OpsBatchCommand,
    updates: Option<&ItemUpdates>,
) -> Result<(), String> {
    let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();

    display::print_bold(&format!("Found {} items matching filters:", items.len()));
    display::print_items(items, true);
    println!();
    print_operation_description(cmd);

    print!("\nProceed? (y/n): ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;

    if input.trim().to_lowercase() != "y" {
        display::print_bold("Cancelled");
        return Ok(());
    }

    let affected = apply_operation(conn, &item_ids, cmd.delete, updates)?;
    let verb = if cmd.delete { "deleted" } else { "updated" };
    display::print_bold(&format!(
        "✓ Successfully {} {}",
        verb,
        pluralize(affected, "item")
    ));
    Ok(())
}

fn execute_interactive(
    conn: &Connection,
    items: &[Item],
    cmd: &OpsBatchCommand,
    updates: Option<&ItemUpdates>,
) -> Result<(), String> {
    let total = items.len();
    display::print_bold(&format!("Interactive mode: {} items found", total));
    println!();
    print_operation_description(cmd);

    let mut accepted = 0;
    let mut skipped = 0;

    for (idx, item) in items.iter().enumerate() {
        println!();
        display::print_bold(&format!("Item {}/{}:", idx + 1, total));
        display::print_items(std::slice::from_ref(item), false);

        match prompt_y_n_q("Apply")? {
            'y' => {
                apply_operation(conn, &[item.id.unwrap()], cmd.delete, updates)?;
                accepted += 1;
            }
            'n' => skipped += 1,
            'q' => {
                let remaining = total - accepted - skipped;
                let verb = if cmd.delete { "Deleted" } else { "Updated" };
                display::print_bold(&format!(
                    "✓ {} {}, skipped {}, quit with {} remaining",
                    verb,
                    pluralize(accepted, "item"),
                    skipped,
                    remaining
                ));
                return Ok(());
            }
            _ => unreachable!(),
        }
    }

    let verb = if cmd.delete { "Deleted" } else { "Updated" };
    display::print_bold(&format!(
        "✓ {} {}, skipped {}",
        verb,
        pluralize(accepted, "item"),
        skipped
    ));
    Ok(())
}

fn apply_operation(
    conn: &Connection,
    item_ids: &[i64],
    delete: bool,
    updates: Option<&ItemUpdates>,
) -> Result<usize, String> {
    if delete {
        batch_delete_items(conn, item_ids).map_err(|e| e.to_string())
    } else {
        batch_update_items(conn, item_ids, updates.unwrap()).map_err(|e| e.to_string())
    }
}

fn print_operation_description(cmd: &OpsBatchCommand) {
    if cmd.delete {
        display::print_red("⚠ WARNING: Selected items will be DELETED permanently!");
    } else {
        display::print_bold("Operation to apply:");
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
}

pub(crate) fn prompt_y_n_q(prompt: &str) -> Result<char, String> {
    loop {
        print!("{} (y/n/q): ", prompt);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;
        match input.trim().to_lowercase().chars().next() {
            Some('y') => return Ok('y'),
            Some('n') => return Ok('n'),
            Some('q') => return Ok('q'),
            _ => println!("Please enter y, n, or q"),
        }
    }
}

fn pluralize(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{} {}", n, word)
    } else {
        format!("{} {}s", n, word)
    }
}

fn parse_action_filter(action: &str) -> Result<Option<Vec<String>>, String> {
    match action {
        "all" => Ok(None),
        "task" | "record" | "recurring_task" | "recurring_task_record" => {
            Ok(Some(vec![action.to_string()]))
        }
        _ => Err(format!(
            "Invalid action: '{}'. Expected: all, task, record, recurring_task, recurring_task_record",
            action
        )),
    }
}

fn query_items_for_batch(
    conn: &Connection,
    actions: Option<&Vec<String>>,
    category: Option<&str>,
    status: Option<u8>,
    create_time_min: Option<i64>,
    create_time_max: Option<i64>,
) -> Result<Vec<Item>, String> {
    let mut query = ItemQuery::new();
    if let Some(acts) = actions {
        query = query.with_actions(acts.iter().map(|s| s.as_str()).collect());
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
    match status {
        None | Some(255) => {}
        Some(254) => query = query.with_statuses(OPEN_STATUS_CODES.to_vec()),
        Some(253) => query = query.with_statuses(CLOSED_STATUS_CODES.to_vec()),
        Some(s) => query = query.with_statuses(vec![s]),
    }
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
    use crate::{
        db::{
            crud::query_items,
            item::ItemQuery,
        },
        tests::{
            get_test_conn,
            insert_record,
            insert_recurring_record,
            insert_recurring_task,
            insert_task,
            update_status,
        },
    };

    #[test]
    fn test_non_task_action_rejects_task_only_args() {
        let (conn, _temp_file) = get_test_conn();
        let base = OpsBatchCommand {
            action: "all".to_string(),
            category: None,
            starting_time: None,
            ending_time: None,
            delete: false,
            interactive: false,
            category_to: None,
            status_to: None,
            target_time_to: None,
            status: None,
        };

        let failing = vec![
            (
                "--status-to",
                OpsBatchCommand {
                    action: "all".to_string(),
                    status_to: Some(1),
                    category_to: Some("x".to_string()),
                    ..base.clone()
                },
            ),
            (
                "--target-time-to",
                OpsBatchCommand {
                    action: "all".to_string(),
                    target_time_to: Some("tomorrow".to_string()),
                    category_to: Some("x".to_string()),
                    ..base.clone()
                },
            ),
            (
                "--status",
                OpsBatchCommand {
                    action: "record".to_string(),
                    status: Some(1),
                    category_to: Some("x".to_string()),
                    ..base.clone()
                },
            ),
        ];
        for (expected, cmd) in &failing {
            let err = handle_batchcmd(&conn, cmd).unwrap_err();
            assert!(
                err.contains(expected),
                "expected '{}' in error: {}",
                expected,
                err
            );
        }

        // aggregate status values rejected for --status-to
        for aggregate in [240u8, 253, 254, 255] {
            let result = handle_batchcmd(&conn, &OpsBatchCommand {
                action: "task".to_string(),
                status_to: Some(aggregate),
                ..base.clone()
            });
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("concrete status"));
        }
    }

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
        assert!(parse_action_filter("invalid").is_err());
    }

    #[test]
    fn test_query_items_for_batch() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "work", "task 1", "today");
        insert_task(&conn, "work", "task 2", "today");
        insert_task(&conn, "personal", "task 3", "today");
        insert_record(&conn, "work", "record 1", "yesterday");

        assert_eq!(
            query_items_for_batch(&conn, None, None, None, None, None)
                .unwrap()
                .len(),
            4
        );

        let actions = Some(vec!["task".to_string()]);
        let items = query_items_for_batch(&conn, actions.as_ref(), None, None, None, None).unwrap();
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.action == "task"));

        let items = query_items_for_batch(&conn, None, Some("work"), None, None, None).unwrap();
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.category == "work"));
    }

    #[test]
    fn test_batch_update_category() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "work", "task 1", "today");
        insert_task(&conn, "work", "task 2", "today");
        insert_record(&conn, "work", "record 1", "yesterday");

        let actions = Some(vec!["task".to_string()]);
        let items =
            query_items_for_batch(&conn, actions.as_ref(), Some("work"), None, None, None).unwrap();
        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();

        let updates = ItemUpdates {
            category: Some("personal".to_string()),
            status: None,
            target_time: None,
        };
        assert_eq!(batch_update_items(&conn, &item_ids, &updates).unwrap(), 2);

        let query = ItemQuery::new()
            .with_category("personal")
            .with_actions(vec!["task"]);
        assert_eq!(query_items(&conn, &query).unwrap().len(), 2);

        let query = ItemQuery::new()
            .with_category("work")
            .with_actions(vec!["record"]);
        assert_eq!(query_items(&conn, &query).unwrap().len(), 1);
    }

    #[test]
    fn test_batch_update_status() {
        let (conn, _temp_file) = get_test_conn();
        let id1 = insert_task(&conn, "work", "task 1", "today");
        let id2 = insert_task(&conn, "work", "task 2", "today");
        update_status(&conn, id1, 0);
        update_status(&conn, id2, 0);

        let updates = ItemUpdates {
            category: None,
            status: Some(1),
            target_time: None,
        };
        assert_eq!(batch_update_items(&conn, &[id1, id2], &updates).unwrap(), 2);

        let query = ItemQuery::new()
            .with_category("work")
            .with_actions(vec!["task"]);
        let items = query_items(&conn, &query).unwrap();
        assert!(items.iter().all(|i| i.status == 1));
    }

    #[test]
    fn test_batch_delete() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "work", "task 1", "today");
        insert_task(&conn, "work", "task 2", "today");
        insert_task(&conn, "personal", "task 3", "today");

        let actions = Some(vec!["task".to_string()]);
        let items =
            query_items_for_batch(&conn, actions.as_ref(), Some("work"), None, None, None).unwrap();
        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();

        assert_eq!(batch_delete_items(&conn, &item_ids).unwrap(), 2);

        let query = ItemQuery::new().with_actions(vec!["task"]);
        let items = query_items(&conn, &query).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category, "personal");
    }

    #[test]
    fn test_batch_with_recurring() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "work", "task 1", "today");
        let rt_id = insert_recurring_task(&conn, "work", "standup", "Daily 9AM");
        insert_recurring_record(&conn, "work", "standup done", rt_id, 1000);

        let items = query_items_for_batch(&conn, None, Some("work"), None, None, None).unwrap();
        assert_eq!(items.len(), 3);

        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        let updates = ItemUpdates {
            category: Some("standup".to_string()),
            status: None,
            target_time: None,
        };
        assert_eq!(batch_update_items(&conn, &item_ids, &updates).unwrap(), 3);

        assert_eq!(
            query_items_for_batch(&conn, None, Some("standup"), None, None, None)
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn test_batch_with_time_filters() {
        let (conn, _temp_file) = get_test_conn();
        insert_record(&conn, "test", "record 1", "2025/02/20 10AM");
        insert_record(&conn, "test", "record 2", "2025/02/25 10AM");
        insert_record(&conn, "test", "record 3", "2025/02/28 10AM");

        let start = timestr::to_unix_epoch("2025/02/22").unwrap();
        let end = timestr::to_unix_epoch("2025/02/27").unwrap();
        let items =
            query_items_for_batch(&conn, None, Some("test"), None, Some(start), Some(end)).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("record 2"));

        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        assert_eq!(batch_delete_items(&conn, &item_ids).unwrap(), 1);
        assert_eq!(
            query_items_for_batch(&conn, None, Some("test"), None, None, None)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn test_batch_with_mixed_tasks_and_records() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "personal", "task 1", "today");
        insert_task(&conn, "personal", "task 2", "tomorrow");
        insert_record(&conn, "personal", "record 1", "yesterday");
        insert_record(&conn, "personal", "record 2", "today");

        let items = query_items_for_batch(&conn, None, Some("personal"), None, None, None).unwrap();
        assert_eq!(items.len(), 4);

        let item_ids: Vec<i64> = items.iter().map(|i| i.id.unwrap()).collect();
        let updates = ItemUpdates {
            category: Some("person".to_string()),
            status: None,
            target_time: None,
        };
        assert_eq!(batch_update_items(&conn, &item_ids, &updates).unwrap(), 4);

        let items = query_items_for_batch(&conn, None, Some("person"), None, None, None).unwrap();
        assert_eq!(items.iter().filter(|i| i.action == "task").count(), 2);
        assert_eq!(items.iter().filter(|i| i.action == "record").count(), 2);
    }

    #[test]
    fn test_batch_status_filter() {
        let (conn, _temp_file) = get_test_conn();
        let id1 = insert_task(&conn, "work", "task 1", "today");
        let id2 = insert_task(&conn, "work", "task 2", "today");
        let id3 = insert_task(&conn, "work", "task 3", "today");
        update_status(&conn, id1, 1); // done
        update_status(&conn, id2, 2); // cancelled
                                      // id3 remains ongoing (0)

        // Filter by open (254 = ongoing|suspended|pending)
        let items =
            query_items_for_batch(&conn, None, Some("work"), Some(254), None, None).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id.unwrap(), id3);

        // Filter by closed (253 = done|cancelled|duplicate|removed)
        let items =
            query_items_for_batch(&conn, None, Some("work"), Some(253), None, None).unwrap();
        assert_eq!(items.len(), 2);

        // Filter by specific status (done = 1)
        let items = query_items_for_batch(&conn, None, Some("work"), Some(1), None, None).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id.unwrap(), id1);

        // No filter / all (255) returns everything
        let items = query_items_for_batch(&conn, None, Some("work"), None, None, None).unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_apply_operation() {
        let (conn, _temp_file) = get_test_conn();
        let id1 = insert_task(&conn, "work", "task 1", "today");
        let id2 = insert_task(&conn, "work", "task 2", "today");
        let id3 = insert_task(&conn, "work", "task 3", "today");

        let updates = ItemUpdates {
            category: Some("done".to_string()),
            status: Some(1),
            target_time: None,
        };
        assert_eq!(
            apply_operation(&conn, &[id1], false, Some(&updates)).unwrap(),
            1
        );
        assert_eq!(apply_operation(&conn, &[id2], true, None).unwrap(), 1);

        let items = query_items_for_batch(&conn, None, Some("work"), None, None, None).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id.unwrap(), id3);
    }
}
