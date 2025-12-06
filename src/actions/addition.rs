use rusqlite::Connection;

use crate::{
    actions::display,
    args::{
        parser::{
            RecordCommand,
            TaskCommand,
        },
        timestr,
    },
    db::{
        crud::insert_item,
        item::{
            Item,
            RECORD,
            TASK,
        },
    },
};

pub fn handle_taskcmd(conn: &Connection, cmd: &TaskCommand) -> Result<(), String> {
    let content = cmd.content.clone();
    let target_timestr = cmd.timestr.clone().unwrap_or_else(|| "today".to_string());
    let category: String = cmd
        .category
        .clone()
        .unwrap_or_else(|| "default".to_string());

    match timestr::to_unix_epoch(&target_timestr) {
        Ok(target_time) => {
            let new_task =
                Item::with_target_time(TASK.to_string(), category, content, Some(target_time));
            insert_item(conn, &new_task).map_err(|e| e.to_string())?;

            display::print_bold("Inserted Task:");
            display::print_items(&[new_task], false, false);
            Ok(())
        }
        Err(_) => match timestr::parse_recurring_timestr(&target_timestr) {
            Ok(cron_schedule) => {
                let new_recurring_task =
                    Item::create_recurring_task(category, content, cron_schedule, target_timestr);
                insert_item(conn, &new_recurring_task).map_err(|e| e.to_string())?;

                display::print_bold("Inserted Recurring Task:");
                display::print_items(&[new_recurring_task], false, false);
                Ok(())
            }
            Err(_) => Err(format!(
                "Could not parse '{}' as a valid time or recurring schedule",
                target_timestr
            )),
        },
    }
}

pub fn handle_recordcmd(conn: &Connection, cmd: &RecordCommand) -> Result<(), String> {
    let content = cmd.content.clone();
    let category: String = cmd
        .category
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let new_record = match &cmd.timestr {
        Some(t) => {
            let create_time = timestr::to_unix_epoch(t)?;
            Item::with_create_time(RECORD.to_string(), category, content, create_time)
        }
        None => Item::new(RECORD.to_string(), category, content),
    };

    insert_item(conn, &new_record).map_err(|e| e.to_string())?;

    display::print_bold("Inserted Record:");
    display::print_items(&[new_record], true, false);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::{
            crud::query_items,
            item::{
                ItemQuery,
                RECURRING_TASK,
            },
        },
        tests::get_test_conn,
    };

    #[test]
    fn test_basic_task() {
        let tc = TaskCommand {
            content: String::from("complete testing of addition.rs"),
            category: None,
            timestr: None,
        };
        let (conn, _temp_file) = get_test_conn();
        handle_taskcmd(&conn, &tc).unwrap();
        let items = query_items(&conn, &ItemQuery::new().with_action(TASK)).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, TASK);
        assert_eq!(items[0].category, "default");
        assert_eq!(items[0].content, "complete testing of addition.rs");
    }

    #[test]
    fn test_filled_task() {
        let tc = TaskCommand {
            content: String::from("complete testing of addition.rs"),
            category: Some("fun".to_string()),
            timestr: Some("tomorrow".to_string()),
        };
        let (conn, _temp_file) = get_test_conn();
        handle_taskcmd(&conn, &tc).unwrap();
        let items = query_items(
            &conn,
            &ItemQuery::new()
                .with_action(TASK)
                .with_category("fun")
                .with_statuses(vec![0]),
        )
        .unwrap();
        let expected_target_time = timestr::to_unix_epoch("tomorrow").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, TASK);
        assert_eq!(items[0].category, "fun");
        assert_eq!(items[0].content, "complete testing of addition.rs");
        assert_eq!(items[0].target_time, Some(expected_target_time));
    }

    #[test]
    fn test_record() {
        let rc = RecordCommand {
            content: String::from("100ML"),
            category: Some("feeding".to_string()),
            timestr: None,
        };
        let (conn, _temp_file) = get_test_conn();
        handle_recordcmd(&conn, &rc).unwrap();
        let items = query_items(
            &conn,
            &ItemQuery::new()
                .with_action(RECORD)
                .with_category("feeding"),
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, RECORD);
        assert_eq!(items[0].category, "feeding");
        assert_eq!(items[0].content, "100ML");
    }

    #[test]
    fn test_recurring_task_patterns() {
        let (conn, _temp_file) = get_test_conn();

        let daily = TaskCommand {
            content: String::from("Daily standup"),
            category: Some("work".to_string()),
            timestr: Some("Daily 9AM".to_string()),
        };
        handle_taskcmd(&conn, &daily).unwrap();

        let weekly = TaskCommand {
            content: String::from("Weekly meeting"),
            category: Some("meetings".to_string()),
            timestr: Some("Weekly Monday-Friday 2PM".to_string()),
        };
        handle_taskcmd(&conn, &weekly).unwrap();

        let monthly = TaskCommand {
            content: String::from("Monthly review"),
            category: Some("admin".to_string()),
            timestr: Some("Monthly 1st".to_string()),
        };
        handle_taskcmd(&conn, &monthly).unwrap();

        let items = query_items(&conn, &ItemQuery::new().with_action(RECURRING_TASK)).unwrap();
        assert_eq!(items.len(), 3);

        assert_eq!(items[0].content, "Daily standup");
        assert_eq!(items[0].cron_schedule, Some("0 9 * * *".to_string()));
        assert_eq!(items[0].human_schedule, Some("Daily 9AM".to_string()));
        assert!(items[0].target_time.is_none());

        assert_eq!(items[1].content, "Weekly meeting");
        assert_eq!(items[1].cron_schedule, Some("0 14 * * 1-5".to_string()));
        assert_eq!(
            items[1].human_schedule,
            Some("Weekly Monday-Friday 2PM".to_string())
        );

        assert_eq!(items[2].content, "Monthly review");
        assert_eq!(items[2].cron_schedule, Some("59 23 1 * *".to_string()));
        assert_eq!(items[2].human_schedule, Some("Monthly 1st".to_string()));
    }

    #[test]
    fn test_task_vs_recurring_task() {
        let (conn, _temp_file) = get_test_conn();

        let regular_task = TaskCommand {
            content: String::from("Finish report"),
            category: Some("work".to_string()),
            timestr: Some("tomorrow".to_string()),
        };
        handle_taskcmd(&conn, &regular_task).unwrap();

        let recurring_task = TaskCommand {
            content: String::from("Check emails"),
            category: Some("work".to_string()),
            timestr: Some("Daily 9AM".to_string()),
        };
        handle_taskcmd(&conn, &recurring_task).unwrap();

        let regular_items = query_items(&conn, &ItemQuery::new().with_action(TASK)).unwrap();
        assert_eq!(regular_items.len(), 1);
        assert_eq!(regular_items[0].content, "Finish report");

        let recurring_items =
            query_items(&conn, &ItemQuery::new().with_action(RECURRING_TASK)).unwrap();
        assert_eq!(recurring_items.len(), 1);
        assert_eq!(recurring_items[0].content, "Check emails");
    }

    #[test]
    fn test_invalid_timestr() {
        let tc = TaskCommand {
            content: String::from("Task"),
            category: None,
            timestr: Some("InvalidTimestr".to_string()),
        };
        let (conn, _temp_file) = get_test_conn();
        let result = handle_taskcmd(&conn, &tc);
        assert!(result.is_err());
    }
}
