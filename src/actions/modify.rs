use rusqlite::Connection;

use crate::{
    actions::display,
    args::{
        parser::{
            DoneCommand,
            UpdateCommand,
        },
        timestr,
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
    match cache::validate_cache(conn) {
        Ok(true) => {}
        Ok(false) => {
            return Err("Cache is not valid, considering running list command first".to_string());
        }
        Err(_) => {
            return Err("Cannot connect to cache".to_string());
        }
    };

    let index = cmd.index as i64;
    let status = cmd.status;
    let row_id = match cache::read(conn, index)
        .map_err(|e| format!("Failed to read cache table: {:?}", e))?
    {
        Some(id) => id,
        None => return Err(format!("index {} does not exist", index)),
    };

    let mut item = get_item(conn, row_id).map_err(|e| format!("Failed to get item: {:?}", e))?;
    if item.action == "record" {
        return Err("Cannot complete a record".to_string());
    }
    item.status = status;
    update_item(conn, &item).map_err(|e| format!("Failed to update item: {:?}", e))?;
    display::print_bold("Completed Task:");
    display::print_items(&[item], true);
    Ok(())
}

pub fn handle_updatecmd(conn: &Connection, cmd: &UpdateCommand) -> Result<(), String> {
    match cache::validate_cache(conn) {
        Ok(true) => {}
        Ok(false) => {
            return Err("Cache is not valid, considering running list command first".to_string());
        }
        Err(_) => {
            return Err("Cannot connect to cache".to_string());
        }
    };

    let index = cmd.index as i64;
    let row_id = match cache::read(conn, index)
        .map_err(|e| format!("Failed to read cache table: {:?}", e))?
    {
        Some(id) => id,
        None => return Err(format!("index {} does not exist", index)),
    };

    let mut item = get_item(conn, row_id).map_err(|e| format!("Failed to get item: {:?}", e))?;

    if let Some(target) = &cmd.target_time {
        let target_time = timestr::to_unix_epoch(target)?;
        item.target_time = Some(target_time);
    }

    if let Some(category) = &cmd.category {
        item.category = category.clone();
    }

    if let Some(content) = &cmd.content {
        item.content = content.clone();
    }

    if let Some(add) = &cmd.add_content {
        item.content.push_str(add);
    }

    if let Some(status) = cmd.status {
        item.status = status;
    }

    update_item(conn, &item).map_err(|e| format!("Failed to update item: {:?}", e))?;

    let is_record = "record" == item.action;
    let action = if is_record { "Record" } else { "Task" };
    display::print_bold(&format!("Updated {}:", action));
    display::print_items(&[item], is_record);
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
        assert_eq!(updated_item.status, 1);

        // update again
        let done_cmd = DoneCommand {
            index: 1,
            status: 2,
        };
        handle_donecmd(&conn, &done_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.status, 2);
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
            category: None,
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
            category: None,
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
            category: None,
            content: None,
            add_content: None,
            status: Some(3),
        };
        handle_updatecmd(&conn, &update_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.status, 3);

        // Test updating target_time and category
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
}
