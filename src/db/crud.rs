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
    Offset,
};

const VALID_ORDER_COLUMNS: &[&str] = &["id", "create_time", "target_time"];

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

pub fn update_item(conn: &Connection, item: &Item) -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "UPDATE items SET 
            category = ?1,
            content = ?2,
            target_time = ?3,
            modify_time = ?4,
            status = ?5
        WHERE id = ?6",
        params![
            item.category,
            item.content,
            item.target_time,
            now,
            item.status,
            item.id
        ],
    )?;

    Ok(())
}

pub fn get_item(conn: &Connection, item_id: i64) -> Result<Item> {
    let item = conn.query_row(
        "SELECT * FROM items WHERE id = ?1",
        params![item_id],
        Item::from_row,
    )?;

    Ok(item)
}

pub fn delete_item(conn: &Connection, item_id: i64) -> Result<()> {
    conn.execute("DELETE FROM items WHERE id = ?1", params![item_id])?;

    Ok(())
}

pub fn query_items(
    conn: &Connection,
    item_query: &ItemQuery,
) -> Result<Vec<Item>, rusqlite::Error> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(a) = item_query.action {
        conditions.push("action = ?".to_string());
        params.push(a.to_string());
    }

    if let Some(c) = item_query.category {
        conditions.push("category = ?".to_string());
        params.push(c.to_string());
    }

    let ct_min = if let Offset::CreateTime(time) = item_query.offset {
        Some(time)
    } else {
        item_query.create_time_min
    };
    if let Some(time) = ct_min {
        conditions.push("create_time > ?".to_string());
        params.push(time.to_string());
    }

    let tt_min = if let Offset::TargetTime(time) = item_query.offset {
        Some(time)
    } else {
        item_query.target_time_min
    };
    if let Some(time) = tt_min {
        conditions.push("target_time > ?".to_string());
        params.push(time.to_string());
    }

    if let Some(ct_max) = item_query.create_time_max {
        conditions.push("create_time <= ?".to_string());
        params.push(ct_max.to_string());
    }

    if let Some(tt_max) = item_query.target_time_max {
        conditions.push("target_time <= ?".to_string());
        params.push(tt_max.to_string());
    }

    if let Some(cc) = &item_query.statuses {
        let status_list = cc
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(", ");
        conditions.push(format!("status IN ({})", status_list));
    }

    if let Offset::Id(rowid) = item_query.offset {
        conditions.push("id > ?".to_string());
        params.push(rowid.to_string());
    }

    let mut querystr = String::from("SELECT * FROM items");
    if !conditions.is_empty() {
        querystr.push_str(" WHERE ");
        querystr.push_str(&conditions.join(" AND "));
    }

    let order_column = match item_query.offset {
        Offset::Id(_) => "id",
        Offset::CreateTime(_) => "create_time",
        Offset::TargetTime(_) => "target_time",
        Offset::None => item_query.order_by.unwrap_or("id"),
    };
    if !VALID_ORDER_COLUMNS.contains(&order_column) {
        return Err(rusqlite::Error::InvalidColumnName(format!(
            "invalid column: {}",
            order_column
        )));
    }
    querystr.push_str(&format!(" ORDER BY {} ASC", order_column));

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
        tests::{
            get_test_conn,
            insert_record,
            insert_task,
            update_status,
        },
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
        item_db.status = 1;
        let result = update_item(&conn, &item_db);
        assert!(result.is_ok(), "Cannot update item: {:?}", result.err());
        let updated_item = get_item(&conn, item_id).unwrap();
        assert_eq!(updated_item.status, 1)
    }

    #[test]
    fn test_delete_item() {
        let (conn, _temp_file) = get_test_conn();
        let item1 = get_test_item("task", "work", "meeting 1");
        let item1_id = insert_item(&conn, &item1).unwrap();
        let item2 = get_test_item("task", "work", "meeting 2");
        let item2_id = insert_item(&conn, &item2).unwrap();
        let item_query = ItemQuery::new().with_action("task");
        let items = query_items(&conn, &item_query).unwrap();
        assert_eq!(items.len(), 2);
        delete_item(&conn, item2_id).expect("Unable to delete item");
        let items = query_items(&conn, &item_query).unwrap();
        assert_eq!(items.len(), 1);
        delete_item(&conn, item1_id).expect("Unable to delete item");
        let items = query_items(&conn, &item_query).unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_query_items() {
        let (conn, _temp_file) = get_test_conn();
        for i in 1..=5 {
            insert_task(&conn, "work", &format!("meeting{}", i), "today");
        }
        for i in 1..=3 {
            insert_task(&conn, "life", &format!("feeding{}", i), "today");
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

    #[test]
    fn test_query_statuses() {
        let (conn, _temp_file) = get_test_conn();
        for i in 1..=2 {
            let rowid = insert_task(&conn, "pending", &format!("pending-task-{}", i), "today");
            update_status(&conn, rowid, 6);
        }
        for i in 1..=3 {
            let rowid = insert_task(&conn, "done", &format!("completed-task-{}", i), "today");
            update_status(&conn, rowid, 1);
        }
        for i in 1..=4 {
            insert_task(&conn, "ongoing", &format!("ongoing-task-{}", i), "today");
        }
        let rowid = insert_task(&conn, "cancelled", "cancelled-task-0", "today");
        update_status(&conn, rowid, 2);

        let ongoing_tasks = query_items(
            &conn,
            &ItemQuery::new().with_statuses(vec![0]).with_action("task"),
        )
        .expect("Unable to execute query");
        assert_eq!(ongoing_tasks.len(), 4);
        assert!(ongoing_tasks.iter().all(|t| t.category == "ongoing"));

        let open_tasks = query_items(
            &conn,
            &ItemQuery::new()
                .with_statuses(vec![0, 6])
                .with_action("task"),
        )
        .expect("Unable to execute query");
        assert_eq!(open_tasks.len(), 6);
        assert!(open_tasks
            .iter()
            .all(|t| t.category == "ongoing" || t.category == "pending"));

        let closed_tasks = query_items(
            &conn,
            &ItemQuery::new()
                .with_statuses(vec![1, 2, 3])
                .with_action("task"),
        )
        .expect("Unable to execute query");
        assert_eq!(closed_tasks.len(), 4);
        assert!(closed_tasks
            .iter()
            .all(|t| t.category == "done" || t.category == "cancelled"));
    }

    // Test pagination capability for tasks
    #[test]
    fn test_query_offset_tasks() {
        let (conn, _temp_file) = get_test_conn();
        for i in 1..=11 {
            insert_task(
                &conn,
                "test",
                &format!("index {}", i),
                &format!("tomorrow {}AM", i),
            );
        }

        let items = query_items(
            &conn,
            &ItemQuery::new()
                .with_action("task")
                .with_limit(5)
                .with_order_by("target_time"),
        )
        .expect("Unable to execute query");
        assert_eq!(items.len(), 5);
        let last_item = items.last().unwrap();
        assert_eq!(last_item.content, "index 5");
        let offset_target_time = last_item.target_time.unwrap();

        let next_items = query_items(
            &conn,
            &ItemQuery::new()
                .with_action("task")
                .with_limit(5)
                .with_offset(Offset::TargetTime(offset_target_time)),
        )
        .unwrap();
        assert_eq!(next_items.len(), 5);
        assert_eq!(next_items.first().unwrap().content, "index 6");
        let last_item = next_items.last().unwrap();
        assert_eq!(last_item.content, "index 10");
        let offset_target_time = last_item.target_time.unwrap();

        let next_items = query_items(
            &conn,
            &ItemQuery::new()
                .with_action("task")
                .with_limit(5)
                .with_offset(Offset::TargetTime(offset_target_time)),
        )
        .unwrap();
        assert_eq!(next_items.len(), 1);
        assert_eq!(next_items[0].content, "index 11");
    }

    #[test]
    fn test_query_offset_records() {
        let (conn, _temp_file) = get_test_conn();
        for i in 1..=11 {
            insert_record(
                &conn,
                "test",
                &format!("index {}", i),
                &format!("yesterday {}PM", i),
            );
        }

        let items = query_items(
            &conn,
            &ItemQuery::new()
                .with_action("record")
                .with_limit(5)
                .with_order_by("create_time"),
        )
        .expect("Unable to execute query");
        assert_eq!(items.len(), 5);
        let last_item = items.last().unwrap();
        assert_eq!(last_item.content, "index 5");

        let next_items = query_items(
            &conn,
            &ItemQuery::new()
                .with_action("record")
                .with_limit(5)
                .with_offset(Offset::CreateTime(last_item.create_time)),
        )
        .unwrap();
        assert_eq!(next_items.len(), 5);
        assert_eq!(next_items.first().unwrap().content, "index 6");
        let last_item = next_items.last().unwrap();
        assert_eq!(last_item.content, "index 10");

        let next_items = query_items(
            &conn,
            &ItemQuery::new()
                .with_action("record")
                .with_limit(5)
                .with_offset(Offset::CreateTime(last_item.create_time)),
        )
        .unwrap();
        assert_eq!(next_items.len(), 1);
        assert_eq!(next_items[0].content, "index 11");
    }

    #[test]
    fn test_offset_id() {
        let (conn, _temp_file) = get_test_conn();
        for i in 1..=11 {
            insert_record(
                &conn,
                "test",
                &format!("index {}", i),
                &format!("yesterday {}PM", i),
            );
        }
        let final_item = query_items(
            &conn,
            &ItemQuery::new()
                .with_action("record")
                .with_limit(10)
                .with_offset(Offset::Id(10)),
        )
        .expect("Unable to execute query");
        assert_eq!(final_item.len(), 1);
        assert_eq!(final_item[0].content, "index 11");
    }

    #[test]
    fn test_order_by() {
        let (conn, _temp_file) = get_test_conn();
        insert_record(&conn, "rec", "rec1", "yesterday");
        insert_record(&conn, "rec", "rec2", "today");
        insert_record(&conn, "rec", "rec3", "tomorrow");
        insert_task(&conn, "task", "task1", "today");
        insert_task(&conn, "task", "task2", "tomorrow");
        insert_task(&conn, "task", "task3", "yesterday");
        let result = query_items(&conn, &ItemQuery::new().with_order_by("create_time")).unwrap();
        assert_eq!(result.first().unwrap().content, "rec1");
        assert_eq!(result.last().unwrap().content, "rec3");
        let result = query_items(
            &conn,
            &ItemQuery::new()
                .with_action("task")
                .with_order_by("target_time"),
        )
        .unwrap();
        assert_eq!(result.first().unwrap().content, "task3");
        assert_eq!(result.last().unwrap().content, "task2");
    }
}
