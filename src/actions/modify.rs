use rusqlite::Connection;

use crate::{
    actions::display,
    args::parser::{
        DoneCommand,
        UpdateCommand,
    },
    db::{
        cache,
        crud::{
            get_item,
            update_item,
        },
    },
};

pub fn handle_donecmd(conn: &Connection, cmd: &DoneCommand) -> Result<(), String> {
    let index = cmd.index as i64;
    let status = cmd.status;

    let row_id = match cache::read(conn, index)
        .map_err(|e| format!("Failed to read cache table: {:?}", e))?
    {
        Some(id) => id,
        None => return Err(format!("index {} does not exist", index)),
    };

    let mut item = get_item(conn, row_id).map_err(|e| format!("Failed to get item: {:?}", e))?;
    item.closing_code = status;
    update_item(conn, &item).map_err(|e| format!("Failed to update item: {:?}", e))?;
    display::debug_print_items("Completed item:", &[item]);
    Ok(())
}

pub fn handle_updatecmd(conn: &Connection, cmd: &UpdateCommand) -> Result<(), String> {
    let index = cmd.index as i64;

    let row_id = match cache::read(conn, index)
        .map_err(|e| format!("Failed to read cache table: {:?}", e))?
    {
        Some(id) => id,
        None => return Err(format!("index {} does not exist", index)),
    };

    let mut item = get_item(conn, row_id).map_err(|e| format!("Failed to get item: {:?}", e))?;

    // Update target time if provided
    if let Some(target) = cmd.target_time {
        item.target_time = Some(target as i64);
    }

    // Replace content if provided
    if let Some(content) = &cmd.content {
        item.content = content.clone();
    }

    // Append to content if provided
    if let Some(add) = &cmd.add_content {
        item.content.push_str(add);
    }

    // Update status/closing code if provided
    if let Some(status) = cmd.status {
        item.closing_code = status;
    }

    // Set modify time to current time
    item.modify_time = Some(chrono::Utc::now().timestamp());

    update_item(conn, &item).map_err(|e| format!("Failed to update item: {:?}", e))?;
    display::debug_print_items("Updated item:", &[item]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::{
            cache,
            crud::{
                get_item,
                query_items,
            },
            item::ItemQuery,
        },
        tests::{
            get_test_conn,
            insert_task,
        },
    };

    #[test]
    fn test_handle_donecmd() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "work", "finish report", "tomorrow");
        let items = query_items(&conn, &ItemQuery::new().with_action("task")).unwrap();
        cache::store(&conn, &items).unwrap();

        let done_cmd = DoneCommand {
            index: 1,
            status: 1,
        };
        handle_donecmd(&conn, &done_cmd).unwrap();
        let item_id = cache::read(&conn, 1).unwrap().unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.closing_code, 1);

        // update again
        let done_cmd = DoneCommand {
            index: 1,
            status: 2,
        };
        handle_donecmd(&conn, &done_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.closing_code, 2);
    }

    #[test]
    fn test_handle_updatecmd() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "home", "clean garage", "saturday");
        let items = query_items(&conn, &ItemQuery::new().with_action("task")).unwrap();
        cache::store(&conn, &items).unwrap();
        let item_id = cache::read(&conn, 1).unwrap().unwrap();

        let update_cmd = UpdateCommand {
            index: 1,
            target_time: None,
            content: Some("reorganize garage thoroughly".to_string()),
            add_content: None,
            status: None,
        };
        handle_updatecmd(&conn, &update_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.content, "reorganize garage thoroughly");

        // Test adding to content
        let update_cmd = UpdateCommand {
            index: 1,
            target_time: None,
            content: None,
            add_content: Some(" and sort tools".to_string()),
            status: None,
        };
        handle_updatecmd(&conn, &update_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(
            updated_item.content,
            "reorganize garage thoroughly and sort tools"
        );

        // Test updating status
        let update_cmd = UpdateCommand {
            index: 1,
            target_time: None,
            content: None,
            add_content: None,
            status: Some(3),
        };
        handle_updatecmd(&conn, &update_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.closing_code, 3);
    }
}
