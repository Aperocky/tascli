use std::{io, io::Write};

use rusqlite::Connection;

use super::{get_rowid_from_cache, validate_cache};
use crate::{
    actions::display,
    args::{
        parser::{DeleteCommand, UpdateCommand},
        timestr,
    },
    db::{
        crud::{delete_item, get_item, update_item},
        item::{RECORD, RECURRING_TASK, RECURRING_TASK_RECORD},
    },
};

pub fn handle_updatecmd(conn: &Connection, cmd: &UpdateCommand) -> Result<(), String> {
    validate_cache(conn)?;
    let row_id = get_rowid_from_cache(conn, cmd.index)?;
    let mut item = get_item(conn, row_id).map_err(|e| format!("Failed to get item: {:?}", e))?;

    if item.action == RECURRING_TASK {
        if cmd.status.is_some() {
            return Err("Cannot update status for recurring tasks".to_string());
        }
        if cmd.add_content.is_some() {
            return Err(
                "Cannot use add_content for recurring tasks, use content instead".to_string(),
            );
        }

        if let Some(schedule_str) = &cmd.target_time {
            match timestr::parse_recurring_timestr(schedule_str) {
                Ok(cron_schedule) => {
                    item.cron_schedule = Some(cron_schedule);
                    item.human_schedule = Some(schedule_str.clone());
                }
                Err(_) => return Err("Cannot parse schedule".to_string()),
            }
        }

        if let Some(category) = &cmd.category {
            item.category = category.clone();
        }
        if let Some(content) = &cmd.content {
            item.content = content.clone();
        }

        update_item(conn, &item).map_err(|e| format!("Failed to update item: {:?}", e))?;
        display::print_bold("Updated Recurring Task:");
        display::print_items(&[item], false);
        return Ok(());
    }

    if let Some(target) = &cmd.target_time {
        item.target_time = Some(timestr::to_unix_epoch(target)?);
    }
    if let Some(category) = &cmd.category {
        item.category = category.clone();
    }
    if let Some(content) = &cmd.content {
        item.content = content.clone();
    }
    if let Some(add) = &cmd.add_content {
        use chrono::Local;
        let timestamp = Local::now().format("%Y-%m-%d %H:%M").to_string();
        item.content.push('\n');
        item.content.push_str(&format!("{} ({})", add, timestamp));
    }
    if let Some(status) = cmd.status {
        item.status = status;
    }

    update_item(conn, &item).map_err(|e| format!("Failed to update item: {:?}", e))?;

    let is_record = item.action == RECORD || item.action == RECURRING_TASK_RECORD;
    let action = if is_record { "Record" } else { "Task" };
    display::print_bold(&format!("Updated {}:", action));
    display::print_items(&[item], false);
    Ok(())
}

pub fn handle_deletecmd(conn: &Connection, cmd: &DeleteCommand) -> Result<(), String> {
    validate_cache(conn)?;
    let row_id = get_rowid_from_cache(conn, cmd.index)?;
    let item = get_item(conn, row_id).map_err(|e| format!("Failed to find item: {:?}", e))?;
    let item_type = item.action.clone();
    display::print_items(&[item], false);

    if !prompt_yes_no(&format!("Are you sure you want to delete this {}? ", &item_type)) {
        return Err(format!("Not deleting the {}", &item_type));
    }
    delete_item(conn, row_id).map_err(|e| format!("Failed to delete item: {:?}", e))?;
    display::print_bold("Deletion success");
    Ok(())
}

fn prompt_yes_no(question: &str) -> bool {
    print!("{} (y/n): ", question);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
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
    fn test_handle_updatecmd() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "home", "clean garage", "saturday");
        let items = query_items(&conn, &ItemQuery::new().with_action(TASK)).unwrap();
        cache::store(&conn, &items).unwrap();
        let item_id = cache::read(&conn, 1).unwrap().unwrap();

        let update_cmd = UpdateCommand {
            index: 1,
            target_time: None,
            category: None,
            content: Some("reorganize garage thoroughly".to_string()),
            add_content: None,
            status: None,
        };
        handle_updatecmd(&conn, &update_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.content, "reorganize garage thoroughly");

        let update_cmd = UpdateCommand {
            index: 1,
            target_time: None,
            category: None,
            content: None,
            add_content: Some("move stuff to basement".to_string()),
            status: None,
        };
        handle_updatecmd(&conn, &update_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert!(updated_item.content.starts_with("reorganize garage thoroughly\nmove stuff to basement ("));
        assert!(updated_item.content.ends_with(")"));

        let update_cmd = UpdateCommand {
            index: 1,
            target_time: None,
            category: None,
            content: None,
            add_content: None,
            status: Some(3),
        };
        handle_updatecmd(&conn, &update_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.status, 3);

        let update_cmd = UpdateCommand {
            index: 1,
            target_time: Some("eow".to_string()),
            category: Some("chore".to_string()),
            content: None,
            add_content: None,
            status: None,
        };
        handle_updatecmd(&conn, &update_cmd).unwrap();
        let got_item = get_item(&conn, item_id).unwrap();
        assert_eq!(got_item.category, "chore");
    }

    #[test]
    fn test_handle_updatecmd_recurring_task() {
        let (conn, _temp_file) = get_test_conn();
        let task_id = insert_recurring_task(&conn, "work", "Daily standup", "Daily 9AM");
        let items = query_items(&conn, &ItemQuery::new().with_action(RECURRING_TASK)).unwrap();
        cache::store(&conn, &items).unwrap();

        let update_cmd = UpdateCommand {
            index: 1,
            target_time: None,
            category: Some("meetings".to_string()),
            content: Some("Daily team sync".to_string()),
            add_content: None,
            status: None,
        };
        assert!(handle_updatecmd(&conn, &update_cmd).is_ok());

        let updated_item = get_item(&conn, task_id).unwrap();
        assert_eq!(updated_item.content, "Daily team sync");
        assert_eq!(updated_item.category, "meetings");

        let update_cmd = UpdateCommand {
            index: 1,
            target_time: Some("Daily 3PM".to_string()),
            category: None,
            content: None,
            add_content: None,
            status: None,
        };
        assert!(handle_updatecmd(&conn, &update_cmd).is_ok());
        let updated_item = get_item(&conn, task_id).unwrap();
        assert_eq!(updated_item.cron_schedule, Some("0 15 * * *".to_string()));
        assert_eq!(updated_item.human_schedule, Some("Daily 3PM".to_string()));

        let update_cmd = UpdateCommand {
            index: 1, target_time: None, category: None, content: None,
            add_content: None, status: Some(1),
        };
        let result = handle_updatecmd(&conn, &update_cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot update status for recurring tasks");

        let update_cmd = UpdateCommand {
            index: 1, target_time: None, category: None, content: None,
            add_content: Some("extra notes".to_string()), status: None,
        };
        let result = handle_updatecmd(&conn, &update_cmd);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot use add_content for recurring tasks, use content instead"
        );
    }

    #[test]
    fn test_block_task_conversions() {
        let (conn, _temp_file) = get_test_conn();

        insert_task(&conn, "work", "finish report", "tomorrow");
        let items = query_items(&conn, &ItemQuery::new().with_action(TASK)).unwrap();
        cache::store(&conn, &items).unwrap();

        let update_cmd = UpdateCommand {
            index: 1,
            target_time: Some("Daily 9AM".to_string()),
            category: None, content: None, add_content: None, status: None,
        };
        let result = handle_updatecmd(&conn, &update_cmd);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Couldn't parse"));

        cache::clear(&conn).unwrap();
        insert_recurring_task(&conn, "work", "Daily standup", "Daily 9AM");
        let items = query_items(&conn, &ItemQuery::new().with_action(RECURRING_TASK)).unwrap();
        cache::store(&conn, &items).unwrap();

        let update_cmd = UpdateCommand {
            index: 1,
            target_time: Some("tomorrow".to_string()),
            category: None, content: None, add_content: None, status: None,
        };
        assert!(handle_updatecmd(&conn, &update_cmd).is_err());
    }
}
