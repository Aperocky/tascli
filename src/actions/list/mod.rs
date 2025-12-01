mod records;
mod tasks;

pub use records::handle_listrecords;
pub use tasks::handle_listtasks;

use rusqlite::Connection;

use crate::{
    args::parser::ShowContentCommand,
    db::{
        cache,
        crud::get_item,
        item::Offset,
    },
};

// Shared constants
pub(crate) const CREATE_TIME_COL: &str = "create_time";
pub(crate) const TARGET_TIME_COL: &str = "target_time";
pub(crate) const OPEN_STATUS_CODES: &[u8] = &[0, 4, 6];
pub(crate) const CLOSED_STATUS_CODES: &[u8] = &[1, 2, 3, 5];

// Shared function for showing content
pub fn handle_showcontent(conn: &Connection, cmd: ShowContentCommand) -> Result<(), String> {
    if !cache::validate_cache(conn).map_err(|e| e.to_string())? {
        return Err("No valid cache found. Please run a list command first.".to_string());
    }

    let item_id = match cache::read(conn, cmd.index as i64).map_err(|e| e.to_string())? {
        Some(id) => id,
        None => {
            return Err(format!(
                "Index {} not found in cache. Use a valid index from the previous list command.",
                cmd.index
            ))
        }
    };

    let item = get_item(conn, item_id).map_err(|e| e.to_string())?;
    println!("{}", item.content);
    Ok(())
}

// Shared function for pagination
pub(crate) fn handle_next_page(conn: &Connection) -> Offset {
    let offset_index = match cache::get_next_index(conn) {
        Ok(Some(index)) => index,
        Ok(None) | Err(_) => return Offset::None,
    };
    let end_item_index = match cache::read(conn, offset_index) {
        Ok(Some(index)) => index,
        Ok(None) | Err(_) => return Offset::None,
    };
    let end_item = match get_item(conn, end_item_index) {
        Ok(item) => item,
        Err(_) => return Offset::None,
    };
    if end_item.action == "task" {
        return Offset::TargetTime(end_item.target_time.unwrap());
    } else if end_item.action == "recurring_task" {
        return Offset::Id(end_item.id.unwrap());
    } else if end_item.action == "record" || end_item.action == "recurring_task_record" {
        return Offset::CreateTime(end_item.create_time);
    }
    Offset::None
}
