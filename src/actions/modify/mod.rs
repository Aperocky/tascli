mod done;
mod update;

pub use done::handle_donecmd;
pub use update::{handle_deletecmd, handle_updatecmd};

use rusqlite::Connection;

use crate::db::cache;

pub(super) fn validate_cache(conn: &Connection) -> Result<(), String> {
    match cache::validate_cache(conn) {
        Ok(true) => Ok(()),
        Ok(false) => Err("Cache is not valid, considering running list command first".to_string()),
        Err(_) => Err("Cannot connect to cache".to_string()),
    }
}

pub(super) fn get_rowid_from_cache(conn: &Connection, index: usize) -> Result<i64, String> {
    let index = index as i64;
    match cache::read(conn, index).map_err(|e| format!("Failed to read cache table: {:?}", e))? {
        Some(id) => Ok(id),
        None => Err(format!("index {} does not exist", index)),
    }
}
