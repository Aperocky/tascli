use std::{
    io,
    io::Write,
};

use rusqlite::Connection;

use crate::{
    actions::display,
    args::{
        parser::{
            DeleteCommand,
            DoneCommand,
            UpdateCommand,
        },
        timestr,
    },
    db::{
        cache,
        crud::{
            delete_item,
            get_item,
            insert_item,
            update_item,
        },
        item::Item,
    },
};

pub fn handle_donecmd(conn: &Connection, cmd: &DoneCommand) -> Result<(), String> {
    validate_cache(conn)?;
    let row_id = get_rowid_from_cache(conn, cmd.index)?;
    let status = cmd.status;

    let mut item = get_item(conn, row_id).map_err(|e| format!("Failed to get item: {:?}", e))?;
    if item.action == "record" {
        return Err("Cannot complete a record".to_string());
    }

    // Create completion record before updating task status
    let completion_content = format!("Completed Task: {}", item.content);
    let completion_record = Item::new(
        "record".to_string(),
        item.category.clone(),
        completion_content,
    );
    insert_item(conn, &completion_record)
        .map_err(|e| format!("Failed to create completion record: {:?}", e))?;

    item.status = status;
    update_item(conn, &item).map_err(|e| format!("Failed to update item: {:?}", e))?;
    display::print_bold("Completed Task:");
    display::print_items(&[item], false, false);
    Ok(())
}

pub fn handle_deletecmd(conn: &Connection, cmd: &DeleteCommand) -> Result<(), String> {
    validate_cache(conn)?;
    let row_id = get_rowid_from_cache(conn, cmd.index)?;
    let item = get_item(conn, row_id).map_err(|e| format!("Failed to find item: {:?}", e))?;
    let item_type = item.action.clone();
    display::print_items(&[item], item_type == "record", false);
    let accept = prompt_yes_no(&format!(
        "Are you sure you want to delete this {}? ",
        &item_type
    ));

    if !accept {
        return Err(format!("Not deleting the {}", &item_type));
    }
    delete_item(conn, row_id).map_err(|e| format!("Failed to update item: {:?}", e))?;
    display::print_bold("Deletion success");
    Ok(())
}

pub fn handle_updatecmd(conn: &Connection, cmd: &UpdateCommand) -> Result<(), String> {
    validate_cache(conn)?;
    let row_id = get_rowid_from_cache(conn, cmd.index)?;
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
        item.content.push('\n');
        item.content.push_str(add);
    }

    if let Some(status) = cmd.status {
        item.status = status;
    }

    update_item(conn, &item).map_err(|e| format!("Failed to update item: {:?}", e))?;

    let is_record = "record" == item.action;
    let action = if is_record { "Record" } else { "Task" };
    display::print_bold(&format!("Updated {}:", action));
    display::print_items(&[item], is_record, false);
    Ok(())
}

fn validate_cache(conn: &Connection) -> Result<(), String> {
    match cache::validate_cache(conn) {
        Ok(true) => Ok(()),
        Ok(false) => Err("Cache is not valid, considering running list command first".to_string()),
        Err(_) => Err("Cannot connect to cache".to_string()),
    }
}

fn get_rowid_from_cache(conn: &Connection, index: usize) -> Result<i64, String> {
    let index = index as i64;
    match cache::read(conn, index).map_err(|e| format!("Failed to read cache table: {:?}", e))? {
        Some(id) => Ok(id),
        None => Err(format!("index {} does not exist", index)),
    }
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

        // Check that a completion record was created
        let records = query_items(&conn, &ItemQuery::new().with_action("record")).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].content, "Completed Task: finish report");
        assert_eq!(records[0].category, "work");

        // update again
        let done_cmd = DoneCommand {
            index: 1,
            status: 2,
        };
        handle_donecmd(&conn, &done_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.status, 2);

        // Check that another completion record was created
        let records = query_items(&conn, &ItemQuery::new().with_action("record")).unwrap();
        assert_eq!(records.len(), 2);
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
            add_content: Some("move stuff to basement".to_string()),
            status: None,
        };
        handle_updatecmd(&conn, &update_cmd).unwrap();
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(
            updated_item.content,
            "reorganize garage thoroughly\nmove stuff to basement"
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
