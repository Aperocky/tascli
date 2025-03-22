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
        item::Item,
    },
};

pub fn handle_taskcmd(conn: &Connection, cmd: &TaskCommand) -> Result<(), String> {
    let content = cmd.content.clone();
    let target_timestr = cmd.timestr.clone().unwrap_or_else(|| "today".to_string());
    let category: String = cmd
        .category
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let target_time = timestr::to_unix_epoch(&target_timestr)?;

    let new_task = Item::with_target_time("task".to_string(), category, content, Some(target_time));
    insert_item(conn, &new_task).map_err(|e| e.to_string())?;

    display::debug_print_items("Inserted task: ", &[new_task]);
    Ok(())
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
            Item::with_create_time("record".to_string(), category, content, create_time)
        }
        None => Item::new("record".to_string(), category, content),
    };

    insert_item(conn, &new_record).map_err(|e| e.to_string())?;

    display::debug_print_items("Inserted task: ", &[new_record]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::{
            crud::query_items,
            item::ItemQuery,
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
        let items = query_items(&conn, &ItemQuery::new().with_action("task")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, "task");
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
                .with_action("task")
                .with_category("fun")
                .with_status(0),
        )
        .unwrap();
        let expected_target_time = timestr::to_unix_epoch("tomorrow").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, "task");
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
                .with_action("record")
                .with_category("feeding"),
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, "record");
        assert_eq!(items[0].category, "feeding");
        assert_eq!(items[0].content, "100ML");
    }
}
