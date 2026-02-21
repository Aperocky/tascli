use rusqlite::Connection;

use super::{get_rowid_from_cache, validate_cache};
use crate::{
    actions::{
        display,
        list::query_all_tasks,
        ops::batch::prompt_y_n_q,
    },
    args::{
        cron,
        parser::{DoneCommand, ListTaskCommand},
    },
    db::{
        crud::{insert_item, query_items, update_item},
        item::{Item, ItemQuery, RECORD, RECURRING_TASK, RECURRING_TASK_RECORD},
    },
};

pub fn handle_donecmd(conn: &Connection, cmd: &DoneCommand) -> Result<(), String> {
    if let Ok(index) = cmd.target.trim().parse::<usize>() {
        return handle_done_by_index(conn, index, cmd.status, cmd.comment.as_deref());
    }
    match cmd.target.trim() {
        "today" => handle_done_today(conn, cmd.status, cmd.comment.as_deref()),
        "overdue" => handle_done_overdue(conn, cmd.status, cmd.comment.as_deref()),
        other => Err(format!("Unknown target '{}'. Expected an index, 'today', or 'overdue'", other)),
    }
}

fn handle_done_by_index(
    conn: &Connection,
    index: usize,
    status: u8,
    comment: Option<&str>,
) -> Result<(), String> {
    validate_cache(conn)?;
    let row_id = get_rowid_from_cache(conn, index)?;
    let mut item = crate::db::crud::get_item(conn, row_id)
        .map_err(|e| format!("Failed to get item: {:?}", e))?;
    complete_item(conn, &mut item, status, comment)
}

fn handle_done_today(
    conn: &Connection,
    status: u8,
    comment: Option<&str>,
) -> Result<(), String> {
    let list_cmd = ListTaskCommand {
        timestr: Some("today".to_string()),
        category: None,
        days: None,
        status: 254,
        overdue: false,
        limit: 100,
        next_page: false,
        search: None,
    };
    run_interactive_done(conn, &list_cmd, "No open tasks found for today", status, comment)
}

fn handle_done_overdue(
    conn: &Connection,
    status: u8,
    comment: Option<&str>,
) -> Result<(), String> {
    let list_cmd = ListTaskCommand {
        timestr: Some("today".to_string()),
        category: None,
        days: None,
        status: 254,
        overdue: true,
        limit: 100,
        next_page: false,
        search: None,
    };
    run_interactive_done(conn, &list_cmd, "No open overdue tasks found", status, comment)
}

fn run_interactive_done(
    conn: &Connection,
    list_cmd: &ListTaskCommand,
    empty_msg: &str,
    status: u8,
    comment: Option<&str>,
) -> Result<(), String> {
    let (tasks, _, _) = query_all_tasks(conn, list_cmd)?;

    if tasks.is_empty() {
        display::print_bold(empty_msg);
        return Ok(());
    }

    let total = tasks.len();
    display::print_bold(&format!("Interactive done: {} tasks found", total));

    let mut completed = 0;
    let mut skipped = 0;

    for (idx, item) in tasks.iter().enumerate() {
        println!();
        display::print_bold(&format!("Task {}/{}:", idx + 1, total));
        display::print_items(std::slice::from_ref(item), false);

        match prompt_y_n_q("Done")? {
            'y' => {
                let mut item = item.clone();
                match complete_item(conn, &mut item, status, comment) {
                    Ok(()) => completed += 1,
                    Err(e) => {
                        display::print_red(&format!("Error: {}", e));
                        skipped += 1;
                    }
                }
            }
            'n' => skipped += 1,
            'q' => {
                let remaining = total - idx - 1;
                display::print_bold(&format!(
                    "✓ Completed {}, skipped {}, quit with {} remaining",
                    pluralize(completed, "task"),
                    skipped,
                    remaining
                ));
                return Ok(());
            }
            _ => unreachable!(),
        }
    }

    display::print_bold(&format!(
        "✓ Completed {}, skipped {}",
        pluralize(completed, "task"),
        skipped
    ));
    Ok(())
}

fn complete_item(
    conn: &Connection,
    item: &mut Item,
    status: u8,
    comment: Option<&str>,
) -> Result<(), String> {
    if item.action == RECORD || item.action == RECURRING_TASK_RECORD {
        return Err("Cannot complete a record".to_string());
    }

    if item.action == RECURRING_TASK {
        let cron_schedule = item
            .cron_schedule
            .as_ref()
            .ok_or_else(|| "Recurring task missing cron schedule".to_string())?;

        let last_occurrence = cron::get_last_occurrence(cron_schedule)?;

        let existing_records = query_items(
            conn,
            &ItemQuery::new()
                .with_action(RECURRING_TASK_RECORD)
                .with_recurring_task_id(item.id.unwrap())
                .with_good_until_min(last_occurrence),
        )
        .map_err(|e| format!("Failed to query existing records: {:?}", e))?;

        if !existing_records.is_empty() {
            return Err(
                "This recurring task has already been completed for this iteration".to_string(),
            );
        }

        let next_occurrence = cron::get_next_occurrence(cron_schedule)?;

        let mut record_content = format!("Completed Recurring Task: {}", item.content);
        if let Some(c) = comment {
            record_content.push('\n');
            record_content.push_str(c);
        }

        let completion_record = Item::create_recurring_record(
            item.category.clone(),
            record_content,
            item.id.unwrap(),
            next_occurrence,
        );
        insert_item(conn, &completion_record)
            .map_err(|e| format!("Failed to create completion record: {:?}", e))?;

        display::print_bold("Completed Recurring Task:");
        display::print_items(std::slice::from_ref(item), false);
        return Ok(());
    }

    if let Some(c) = comment {
        item.content.push('\n');
        item.content.push_str(c);
    }

    let completion_content = format!("Completed Task: {}", item.content);
    let completion_record = Item::new(RECORD.to_string(), item.category.clone(), completion_content);
    insert_item(conn, &completion_record)
        .map_err(|e| format!("Failed to create completion record: {:?}", e))?;

    item.status = status;
    update_item(conn, item).map_err(|e| format!("Failed to update item: {:?}", e))?;
    display::print_bold("Completed Task:");
    display::print_items(std::slice::from_ref(item), false);
    Ok(())
}

fn pluralize(n: usize, word: &str) -> String {
    if n == 1 { format!("{} {}", n, word) } else { format!("{} {}s", n, word) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::{
            cache,
            crud::{get_item, query_items},
            item::{ItemQuery, TASK},
        },
        tests::{get_test_conn, insert_recurring_task, insert_task},
    };

    #[test]
    fn test_handle_donecmd() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "work", "finish report", "tomorrow");
        let items = query_items(&conn, &ItemQuery::new().with_action(TASK)).unwrap();
        cache::store(&conn, &items).unwrap();

        let done_cmd = DoneCommand { target: "1".to_string(), status: 1, comment: None };
        handle_donecmd(&conn, &done_cmd).unwrap();
        let item_id = cache::read(&conn, 1).unwrap().unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.status, 1);

        let records = query_items(&conn, &ItemQuery::new().with_action(RECORD)).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].content, "Completed Task: finish report");
        assert_eq!(records[0].category, "work");

        let done_cmd = DoneCommand { target: "1".to_string(), status: 2, comment: None };
        handle_donecmd(&conn, &done_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.status, 2);

        let records = query_items(&conn, &ItemQuery::new().with_action(RECORD)).unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_handle_donecmd_with_comment() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "work", "finish report", "tomorrow");
        let items = query_items(&conn, &ItemQuery::new().with_action(TASK)).unwrap();
        cache::store(&conn, &items).unwrap();

        let done_cmd = DoneCommand {
            target: "1".to_string(),
            status: 1,
            comment: Some("Added extra analysis section".to_string()),
        };
        handle_donecmd(&conn, &done_cmd).unwrap();
        let item_id = cache::read(&conn, 1).unwrap().unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();

        assert_eq!(updated_item.content, "finish report\nAdded extra analysis section");
        assert_eq!(updated_item.status, 1);

        let records = query_items(&conn, &ItemQuery::new().with_action(RECORD)).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].content, "Completed Task: finish report\nAdded extra analysis section");
        assert_eq!(records[0].category, "work");
    }

    #[test]
    fn test_handle_donecmd_recurring_task() {
        let (conn, _temp_file) = get_test_conn();
        let task_id = insert_recurring_task(&conn, "work", "Daily standup", "Daily 9AM");
        let items = query_items(&conn, &ItemQuery::new().with_action(RECURRING_TASK)).unwrap();
        cache::store(&conn, &items).unwrap();

        let done_cmd = DoneCommand {
            target: "1".to_string(),
            status: 1,
            comment: Some("Discussed sprint goals".to_string()),
        };
        let result = handle_donecmd(&conn, &done_cmd);
        assert!(result.is_ok());

        let records = query_items(&conn, &ItemQuery::new().with_action(RECURRING_TASK_RECORD)).unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0].content.contains("Completed Recurring Task: Daily standup"));
        assert!(records[0].content.contains("Discussed sprint goals"));
        assert_eq!(records[0].category, "work");
        assert_eq!(records[0].recurring_task_id, Some(task_id));
        assert!(records[0].good_until.is_some());

        let done_cmd2 = DoneCommand { target: "1".to_string(), status: 1, comment: None };
        let result = handle_donecmd(&conn, &done_cmd2);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "This recurring task has already been completed for this iteration"
        );

        let records = query_items(&conn, &ItemQuery::new().with_action(RECURRING_TASK_RECORD)).unwrap();
        assert_eq!(records.len(), 1);
    }
}
