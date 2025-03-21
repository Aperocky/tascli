use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use rusqlite::{
    params,
    params_from_iter,
    Connection,
    Result,
};

use crate::db::item::{
    Item,
    ItemQuery,
};

pub fn insert_item(conn: &Connection, item: &Item) -> Result<i64> {
    conn.execute(
        "INSERT INTO items (action, category, content, create_time, target_time) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            item.action,
            item.category,
            item.content,
            item.create_time,
            item.target_time
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn update_item(conn: &Connection, item: &Item) -> Result<(), rusqlite::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "UPDATE items SET 
            content = ?1,
            target_time = ?2,
            modify_time = ?3,
            closing_code = ?4
        WHERE id = ?5",
        params![
            item.content,
            item.target_time,
            now,
            item.closing_code,
            item.id
        ],
    )?;

    Ok(())
}

pub fn get_item(conn: &Connection, item_id: i64) -> Result<Item, rusqlite::Error> {
    let item = conn.query_row(
        "SELECT * FROM items WHERE id = ?1",
        params![item_id],
        Item::from_row,
    )?;

    Ok(item)
}

#[allow(clippy::too_many_arguments)]
pub fn query_items(
    conn: &Connection,
    item_query: &ItemQuery,
) -> Result<Vec<Item>, rusqlite::Error> {
    let mut conditions = Vec::new();
    let mut params = Vec::new();

    if let Some(a) = item_query.action {
        conditions.push("action = ?");
        params.push(a.to_string());
    }

    if let Some(c) = item_query.category {
        conditions.push("category = ?");
        params.push(c.to_string());
    }

    if let Some(ct_min) = item_query.create_time_min {
        conditions.push("create_time >= ?");
        params.push(ct_min.to_string());
    }

    if let Some(ct_max) = item_query.create_time_max {
        conditions.push("create_time <= ?");
        params.push(ct_max.to_string());
    }

    if let Some(tt_min) = item_query.target_time_min {
        conditions.push("target_time >= ?");
        params.push(tt_min.to_string());
    }

    if let Some(tt_max) = item_query.target_time_max {
        conditions.push("target_time <= ?");
        params.push(tt_max.to_string());
    }

    if let Some(cc) = item_query.closing_code {
        conditions.push("closing_code = ?");
        params.push(cc.to_string());
    }

    let mut querystr = String::from("SELECT * FROM items");
    if !conditions.is_empty() {
        querystr.push_str(" WHERE ");
        querystr.push_str(&conditions.join(" AND "));
    }

    if let Some(offset_id) = item_query.offset_id {
        if conditions.is_empty() {
            querystr.push_str(" WHERE id > ?");
        } else {
            querystr.push_str(" AND id > ?");
        }
        params.push(offset_id.to_string());
    }

    querystr.push_str(" ORDER BY id ASC");

    if let Some(limit) = item_query.limit {
        querystr.push_str(" LIMIT ?");
        params.push(limit.to_string());
    }

    let mut stmt = conn.prepare(&querystr)?;

    let item_iter = stmt.query_map(params_from_iter(params), Item::from_row)?;

    let mut items = Vec::new();
    for item_result in item_iter {
        items.push(item_result?);
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::item::Item,
        tests::get_test_conn,
    };

    fn get_test_item(action: &str, category: &str, content: &str) -> Item {
        Item::new(
            action.to_string(),
            category.to_string(),
            content.to_string(),
        )
    }

    #[test]
    fn test_insert_item() {
        let (conn, _temp_file) = get_test_conn();
        let item = get_test_item("task", "work", "meeting");
        let result = insert_item(&conn, &item);
        assert!(
            result.is_ok(),
            "Cannot insert basic item: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_get_item() {
        let (conn, _temp_file) = get_test_conn();
        let item = get_test_item("task", "work", "meeting");
        let item_id = insert_item(&conn, &item).unwrap();
        let item_db = get_item(&conn, item_id);
        assert!(
            item_db.is_ok(),
            "Cannot query item base on id: {:?}",
            item_db.err()
        );
        let item_db = item_db.unwrap();
        assert_eq!(item_db.action, "task");
        assert_eq!(item_db.category, "work");
        assert_eq!(item_db.content, "meeting");
    }

    #[test]
    fn test_update_item() {
        let (conn, _temp_file) = get_test_conn();
        let item = get_test_item("task", "work", "meeting");
        let item_id = insert_item(&conn, &item).unwrap();
        let mut item_db = get_item(&conn, item_id).unwrap();
        item_db.closing_code = 1;
        let result = update_item(&conn, &item_db);
        assert!(result.is_ok(), "Cannot update item: {:?}", result.err());
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.closing_code, 1)
    }

    #[test]
    fn test_query_items() {
        let (conn, _temp_file) = get_test_conn();
        for i in 1..=5 {
            let test_meeting = get_test_item("task", "work", &format!("meeting{}", i));
            insert_item(&conn, &test_meeting).unwrap();
        }
        for i in 1..=3 {
            let test_feeding = get_test_item("task", "life", &format!("feeding{}", i));
            insert_item(&conn, &test_feeding).unwrap();
        }

        let item_query = ItemQuery::new().with_action("task").with_category("work");
        let work_items = query_items(&conn, &item_query);
        assert!(
            work_items.is_ok(),
            "Error querying items: {:?}",
            work_items.err()
        );
        let work_items = work_items.unwrap();
        assert_eq!(work_items.len(), 5);
        for item in &work_items {
            assert_eq!(item.action, "task");
            assert_eq!(item.category, "work");
            assert!(item.content.starts_with("meeting"));
        }

        let item_query = ItemQuery::new().with_action("task").with_category("life");
        let life_items = query_items(&conn, &item_query).unwrap();
        assert_eq!(life_items.len(), 3);
        for item in &life_items {
            assert_eq!(item.action, "task");
            assert_eq!(item.category, "life");
            assert!(item.content.starts_with("feeding"));
        }

        let all_items = query_items(&conn, &ItemQuery::new()).unwrap();
        assert_eq!(all_items.len(), 8);

        let task_items = query_items(&conn, &ItemQuery::new().with_action("task")).unwrap();
        assert_eq!(task_items.len(), 8);

        let empty_items = query_items(&conn, &ItemQuery::new().with_action("record")).unwrap();
        assert_eq!(empty_items.len(), 0);

        let limited_items =
            query_items(&conn, &ItemQuery::new().with_action("task").with_limit(4)).unwrap();
        assert_eq!(limited_items.len(), 4);
    }
}
