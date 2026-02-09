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

#[derive(Debug)]
pub struct ItemUpdates {
    pub category: Option<String>,
    pub status: Option<u8>,
    pub target_time: Option<i64>,
}

const VALID_ORDER_COLUMNS: &[&str] = &["id", "create_time", "target_time"];

pub fn insert_item(conn: &Connection, item: &Item) -> Result<i64> {
    conn.execute(
        "INSERT INTO items (action, category, content, create_time, target_time, cron_schedule, human_schedule, recurring_task_id, good_until)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            item.action,
            item.category,
            item.content,
            item.create_time,
            item.target_time,
            item.cron_schedule,
            item.human_schedule,
            item.recurring_task_id,
            item.good_until
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
            status = ?5,
            cron_schedule = ?6,
            human_schedule = ?7,
            recurring_task_id = ?8,
            good_until = ?9
        WHERE id = ?10",
        params![
            item.category,
            item.content,
            item.target_time,
            now,
            item.status,
            item.cron_schedule,
            item.human_schedule,
            item.recurring_task_id,
            item.good_until,
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

    if let Some(actions) = &item_query.actions {
        if actions.len() == 1 {
            conditions.push("action = ?".to_string());
            params.push(actions[0].to_string());
        } else {
            let action_list = actions
                .iter()
                .map(|a| format!("'{}'", a))
                .collect::<Vec<String>>()
                .join(", ");
            conditions.push(format!("action IN ({})", action_list));
        }
    }

    if let Some(c) = item_query.category {
        conditions.push("category = ?".to_string());
        params.push(c.to_string());
    }

    if let Some(content) = item_query.content_like {
        conditions.push("content LIKE ?".to_string());
        params.push(format!("%{}%", content));
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

    if let Some(gu_min) = item_query.good_until_min {
        conditions.push("good_until > ?".to_string());
        params.push(gu_min.to_string());
    }

    if let Some(gu_max) = item_query.good_until_max {
        conditions.push("good_until <= ?".to_string());
        params.push(gu_max.to_string());
    }

    if let Some(rt_id) = item_query.recurring_task_id {
        conditions.push("recurring_task_id = ?".to_string());
        params.push(rt_id.to_string());
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

pub fn build_batch_where_clause(
    actions: Option<Vec<&str>>,
    category: Option<&str>,
    create_time_min: Option<i64>,
    create_time_max: Option<i64>,
    target_time_min: Option<i64>,
    target_time_max: Option<i64>,
    statuses: Option<Vec<u8>>,
) -> (String, Vec<String>) {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(actions) = actions {
        if actions.len() == 1 {
            conditions.push("action = ?".to_string());
            params.push(actions[0].to_string());
        } else if actions.len() > 1 {
            let action_list = actions
                .iter()
                .map(|a| format!("'{}'", a))
                .collect::<Vec<String>>()
                .join(", ");
            conditions.push(format!("action IN ({})", action_list));
        }
    }

    if let Some(c) = category {
        conditions.push("category = ?".to_string());
        params.push(c.to_string());
    }

    if let Some(time) = create_time_min {
        conditions.push("create_time > ?".to_string());
        params.push(time.to_string());
    }

    if let Some(time) = create_time_max {
        conditions.push("create_time <= ?".to_string());
        params.push(time.to_string());
    }

    if let Some(time) = target_time_min {
        conditions.push("target_time > ?".to_string());
        params.push(time.to_string());
    }

    if let Some(time) = target_time_max {
        conditions.push("target_time <= ?".to_string());
        params.push(time.to_string());
    }

    if let Some(cc) = statuses {
        let status_list = cc
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(", ");
        conditions.push(format!("status IN ({})", status_list));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    (where_clause, params)
}

pub fn batch_update_items(
    conn: &Connection,
    actions: Option<Vec<&str>>,
    category: Option<&str>,
    create_time_min: Option<i64>,
    create_time_max: Option<i64>,
    target_time_min: Option<i64>,
    target_time_max: Option<i64>,
    statuses: Option<Vec<u8>>,
    updates: &ItemUpdates,
) -> Result<usize> {
    let mut set_parts = Vec::new();
    let mut set_params = Vec::new();

    if let Some(cat) = &updates.category {
        set_parts.push("category = ?");
        set_params.push(cat.to_string());
    }
    if let Some(status) = updates.status {
        set_parts.push("status = ?");
        set_params.push(status.to_string());
    }
    if let Some(target_time) = updates.target_time {
        set_parts.push("target_time = ?");
        set_params.push(target_time.to_string());
    }

    if set_parts.is_empty() {
        return Err(rusqlite::Error::InvalidQuery);
    }

    let set_clause = set_parts.join(", ");
    let (where_clause, where_params) = build_batch_where_clause(
        actions,
        category,
        create_time_min,
        create_time_max,
        target_time_min,
        target_time_max,
        statuses,
    );

    let mut all_params = set_params;
    all_params.extend(where_params);

    let query = format!("UPDATE items SET {}{}", set_clause, where_clause);
    let affected = conn.execute(&query, params_from_iter(all_params))?;

    Ok(affected)
}

pub fn batch_delete_items(
    conn: &Connection,
    actions: Option<Vec<&str>>,
    category: Option<&str>,
    create_time_min: Option<i64>,
    create_time_max: Option<i64>,
    target_time_min: Option<i64>,
    target_time_max: Option<i64>,
    statuses: Option<Vec<u8>>,
) -> Result<usize> {
    let (where_clause, params) = build_batch_where_clause(
        actions,
        category,
        create_time_min,
        create_time_max,
        target_time_min,
        target_time_max,
        statuses,
    );

    let query = format!("DELETE FROM items{}", where_clause);
    let affected = conn.execute(&query, params_from_iter(params))?;

    Ok(affected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::item::{
            Item,
            RECORD,
            RECURRING_TASK_RECORD,
            TASK,
        },
        tests::{
            get_test_conn,
            insert_record,
            insert_recurring_record,
            insert_recurring_task,
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
        assert_eq!(updated_item.status, 1);

        // Test updating a recurring task
        let recurring_id =
            insert_recurring_task(&conn, "work", "Weekly standup", "Weekly Monday 9AM");
        let mut recurring_item = get_item(&conn, recurring_id).unwrap();

        // Store original recurring fields
        let original_cron = recurring_item.cron_schedule.clone();
        let original_human = recurring_item.human_schedule.clone();

        // Update category
        recurring_item.category = "meetings".to_string();
        let result = update_item(&conn, &recurring_item);
        assert!(
            result.is_ok(),
            "Cannot update recurring item: {:?}",
            result.err()
        );

        // Verify category changed but recurring fields stayed the same
        let updated_recurring = get_item(&conn, recurring_id).unwrap();
        assert_eq!(updated_recurring.category, "meetings");
        assert_eq!(updated_recurring.cron_schedule, original_cron);
        assert_eq!(updated_recurring.human_schedule, original_human);
    }

    #[test]
    fn test_delete_item() {
        let (conn, _temp_file) = get_test_conn();
        let item1 = get_test_item("task", "work", "meeting 1");
        let item1_id = insert_item(&conn, &item1).unwrap();
        let item2 = get_test_item("task", "work", "meeting 2");
        let item2_id = insert_item(&conn, &item2).unwrap();
        let item_query = ItemQuery::new().with_action(TASK);
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

        let item_query = ItemQuery::new().with_action(TASK).with_category("work");
        let work_items = query_items(&conn, &item_query);
        assert!(
            work_items.is_ok(),
            "Error querying items: {:?}",
            work_items.err()
        );
        let work_items = work_items.unwrap();
        assert_eq!(work_items.len(), 5);
        for item in &work_items {
            assert_eq!(item.action, TASK);
            assert_eq!(item.category, "work");
            assert!(item.content.starts_with("meeting"));
        }

        let item_query = ItemQuery::new().with_action(TASK).with_category("life");
        let life_items = query_items(&conn, &item_query).unwrap();
        assert_eq!(life_items.len(), 3);
        for item in &life_items {
            assert_eq!(item.action, TASK);
            assert_eq!(item.category, "life");
            assert!(item.content.starts_with("feeding"));
        }

        let all_items = query_items(&conn, &ItemQuery::new()).unwrap();
        assert_eq!(all_items.len(), 8);

        let task_items = query_items(&conn, &ItemQuery::new().with_action(TASK)).unwrap();
        assert_eq!(task_items.len(), 8);

        let empty_items = query_items(&conn, &ItemQuery::new().with_action(RECORD)).unwrap();
        assert_eq!(empty_items.len(), 0);

        let limited_items =
            query_items(&conn, &ItemQuery::new().with_action(TASK).with_limit(4)).unwrap();
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
            &ItemQuery::new().with_statuses(vec![0]).with_action(TASK),
        )
        .expect("Unable to execute query");
        assert_eq!(ongoing_tasks.len(), 4);
        assert!(ongoing_tasks.iter().all(|t| t.category == "ongoing"));

        let open_tasks = query_items(
            &conn,
            &ItemQuery::new().with_statuses(vec![0, 6]).with_action(TASK),
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
                .with_action(TASK),
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
                .with_action(TASK)
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
                .with_action(TASK)
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
                .with_action(TASK)
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
                .with_action(RECORD)
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
                .with_action(RECORD)
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
                .with_action(RECORD)
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
                .with_action(RECORD)
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
                .with_action(TASK)
                .with_order_by("target_time"),
        )
        .unwrap();
        assert_eq!(result.first().unwrap().content, "task3");
        assert_eq!(result.last().unwrap().content, "task2");
    }

    #[test]
    fn test_query_content_like() {
        let (conn, _temp_file) = get_test_conn();

        // Insert tasks with different content patterns
        insert_task(&conn, "work", "meeting with client", "today");
        insert_task(&conn, "work", "team meeting", "today");
        insert_task(&conn, "work", "review code", "today");
        insert_task(&conn, "personal", "doctor appointment", "tomorrow");
        insert_task(&conn, "personal", "meeting friends", "tomorrow");

        // Insert records with different content patterns
        insert_record(&conn, "work", "attended meeting", "yesterday");
        insert_record(&conn, "work", "completed review", "yesterday");
        insert_record(&conn, "personal", "had appointment", "yesterday");

        // Test searching for "meeting" - should find 4 items
        let meeting_items =
            query_items(&conn, &ItemQuery::new().with_content_like("meeting")).unwrap();
        assert_eq!(meeting_items.len(), 4);
        for item in &meeting_items {
            assert!(item.content.contains("meeting"));
        }

        // Test searching for "appointment" - should find 2 items
        let appointment_items =
            query_items(&conn, &ItemQuery::new().with_content_like("appointment")).unwrap();
        assert_eq!(appointment_items.len(), 2);
        for item in &appointment_items {
            assert!(item.content.contains("appointment"));
        }

        // Test combining content_like with other filters
        let work_meeting_items = query_items(
            &conn,
            &ItemQuery::new()
                .with_content_like("meeting")
                .with_category("work"),
        )
        .unwrap();
        assert_eq!(work_meeting_items.len(), 3);
        for item in &work_meeting_items {
            assert!(item.content.contains("meeting"));
            assert_eq!(item.category, "work");
        }

        // Test combining content_like with action filter
        let task_meeting_items = query_items(
            &conn,
            &ItemQuery::new()
                .with_content_like("meeting")
                .with_action(TASK),
        )
        .unwrap();
        assert_eq!(task_meeting_items.len(), 3);
        for item in &task_meeting_items {
            assert!(item.content.contains("meeting"));
            assert_eq!(item.action, TASK);
        }

        // Test search that should return no results
        let no_results =
            query_items(&conn, &ItemQuery::new().with_content_like("nonexistent")).unwrap();
        assert_eq!(no_results.len(), 0);

        // Test partial word matching
        let review_items = query_items(&conn, &ItemQuery::new().with_content_like("rev")).unwrap();
        assert_eq!(review_items.len(), 2);
        for item in &review_items {
            assert!(item.content.contains("review"));
        }
    }

    #[test]
    fn test_query_multiple_actions() {
        let (conn, _temp_file) = get_test_conn();

        // Insert records and recurring_task_records
        insert_record(&conn, "feeding", "100ML", "yesterday 2PM");
        insert_record(&conn, "feeding", "110ML", "yesterday 5PM");
        insert_task(&conn, "work", "meeting", "today");

        let task1_id = insert_recurring_task(&conn, "work", "Weekly standup", "Weekly Monday 9AM");
        insert_recurring_record(&conn, "work", "Standup completed", task1_id, 1000);
        insert_recurring_record(&conn, "work", "Standup completed", task1_id, 2000);

        // Test single action (backward compatibility)
        let records = query_items(&conn, &ItemQuery::new().with_action(RECORD)).unwrap();
        assert_eq!(records.len(), 2);
        for item in &records {
            assert_eq!(item.action, RECORD);
        }

        // Test multiple actions
        let all_records = query_items(
            &conn,
            &ItemQuery::new().with_actions(vec![RECORD, RECURRING_TASK_RECORD]),
        )
        .unwrap();
        assert_eq!(all_records.len(), 4);
        for item in &all_records {
            assert!(item.action == RECORD || item.action == RECURRING_TASK_RECORD);
        }

        // Test multiple actions with category filter
        let work_records = query_items(
            &conn,
            &ItemQuery::new()
                .with_actions(vec![RECORD, RECURRING_TASK_RECORD])
                .with_category("work"),
        )
        .unwrap();
        assert_eq!(work_records.len(), 2);
        for item in &work_records {
            assert!(item.action == RECORD || item.action == RECURRING_TASK_RECORD);
            assert_eq!(item.category, "work");
        }
    }

    #[test]
    fn test_query_recurring_task_fields() {
        let (conn, _temp_file) = get_test_conn();

        // Create recurring tasks and records
        let task1_id = insert_recurring_task(&conn, "work", "Weekly standup", "Weekly Monday 9AM");
        insert_recurring_record(&conn, "work", "Standup completed", task1_id, 1000);
        insert_recurring_record(&conn, "work", "Standup completed", task1_id, 2000);
        insert_recurring_record(&conn, "work", "Standup completed", task1_id, 3000);

        let task2_id = insert_recurring_task(&conn, "personal", "Daily exercise", "Daily 7AM");
        insert_recurring_record(&conn, "personal", "Exercise completed", task2_id, 1500);

        // Test querying by recurring_task_id
        let records_for_task1 = query_items(
            &conn,
            &ItemQuery::new()
                .with_action(RECURRING_TASK_RECORD)
                .with_recurring_task_id(task1_id),
        )
        .unwrap();
        assert_eq!(records_for_task1.len(), 3);
        for record in &records_for_task1 {
            assert_eq!(record.recurring_task_id, Some(task1_id));
        }

        let records_for_task2 = query_items(
            &conn,
            &ItemQuery::new()
                .with_action(RECURRING_TASK_RECORD)
                .with_recurring_task_id(task2_id),
        )
        .unwrap();
        assert_eq!(records_for_task2.len(), 1);
        assert_eq!(records_for_task2[0].recurring_task_id, Some(task2_id));

        // Test querying by good_until range
        let records_after_1500 = query_items(
            &conn,
            &ItemQuery::new()
                .with_action(RECURRING_TASK_RECORD)
                .with_good_until_min(1500),
        )
        .unwrap();
        assert_eq!(records_after_1500.len(), 2); // records at 2000 and 3000

        let records_before_2500 = query_items(
            &conn,
            &ItemQuery::new()
                .with_action(RECURRING_TASK_RECORD)
                .with_good_until_max(2500),
        )
        .unwrap();
        assert_eq!(records_before_2500.len(), 3); // records at 1000, 1500, and 2000

        // Test querying by good_until range (min and max)
        let records_in_range = query_items(
            &conn,
            &ItemQuery::new()
                .with_action(RECURRING_TASK_RECORD)
                .with_good_until_min(1000)
                .with_good_until_max(2500),
        )
        .unwrap();
        assert_eq!(records_in_range.len(), 2); // records at 1500 and 2000

        // Test combining recurring_task_id with good_until range
        let specific_records = query_items(
            &conn,
            &ItemQuery::new()
                .with_action(RECURRING_TASK_RECORD)
                .with_recurring_task_id(task1_id)
                .with_good_until_min(1500),
        )
        .unwrap();
        assert_eq!(specific_records.len(), 2); // records at 2000 and 3000 for task1
        for record in &specific_records {
            assert_eq!(record.recurring_task_id, Some(task1_id));
            assert!(record.good_until.unwrap() > 1500);
        }
    }

    #[test]
    fn test_build_batch_where_clause() {
        // Test with all parameters
        let (where_clause, params) = build_batch_where_clause(
            Some(vec![TASK]),
            Some("work"),
            Some(1000),
            Some(2000),
            Some(3000),
            Some(4000),
            Some(vec![0, 1]),
        );
        assert_eq!(
            where_clause,
            " WHERE action = ? AND category = ? AND create_time > ? AND create_time <= ? AND target_time > ? AND target_time <= ? AND status IN (0, 1)"
        );
        assert_eq!(params, vec!["task", "work", "1000", "2000", "3000", "4000"]);

        // Test with multiple actions
        let (where_clause, params) = build_batch_where_clause(
            Some(vec![TASK, RECORD]),
            Some("life"),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            where_clause,
            " WHERE action IN ('task', 'record') AND category = ?"
        );
        assert_eq!(params, vec!["life"]);

        // Test with only time ranges
        let (where_clause, params) = build_batch_where_clause(
            None,
            None,
            Some(5000),
            Some(6000),
            None,
            None,
            None,
        );
        assert_eq!(
            where_clause,
            " WHERE create_time > ? AND create_time <= ?"
        );
        assert_eq!(params, vec!["5000", "6000"]);

        // Test with no parameters (empty WHERE clause)
        let (where_clause, params) = build_batch_where_clause(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(where_clause, "");
        assert_eq!(params.len(), 0);

        // Test with target_time only (for tasks)
        let (where_clause, params) = build_batch_where_clause(
            Some(vec![TASK]),
            None,
            None,
            None,
            Some(7000),
            Some(8000),
            Some(vec![0]),
        );
        assert_eq!(
            where_clause,
            " WHERE action = ? AND target_time > ? AND target_time <= ? AND status IN (0)"
        );
        assert_eq!(params, vec!["task", "7000", "8000"]);
    }

    #[test]
    fn test_batch_update_items() {
        let (conn, _temp_file) = get_test_conn();

        // Test 1: Update category on filtered tasks
        insert_task(&conn, "work", "task 1", "today");
        insert_task(&conn, "work", "task 2", "today");
        insert_task(&conn, "work", "task 3", "tomorrow");
        insert_task(&conn, "life", "task 4", "today");
        insert_record(&conn, "work", "record 1", "yesterday");

        let updates = ItemUpdates {
            category: Some("meetings".to_string()),
            status: None,
            target_time: None,
        };
        let affected = batch_update_items(
            &conn,
            Some(vec![TASK]),
            Some("work"),
            None,
            None,
            None,
            None,
            None,
            &updates,
        )
        .expect("batch update failed");
        assert_eq!(affected, 3);

        let meetings_tasks = query_items(&conn, &ItemQuery::new().with_category("meetings"))
            .expect("query failed");
        assert_eq!(meetings_tasks.len(), 3);
        for task in &meetings_tasks {
            assert_eq!(task.action, TASK);
        }

        let work_records = query_items(
            &conn,
            &ItemQuery::new().with_action(RECORD).with_category("work"),
        )
        .expect("query failed");
        assert_eq!(work_records.len(), 1);

        // Test 2: Update status on all tasks
        let updates = ItemUpdates {
            category: None,
            status: Some(1),
            target_time: None,
        };
        let affected = batch_update_items(
            &conn,
            Some(vec![TASK]),
            None,
            None,
            None,
            None,
            None,
            None,
            &updates,
        )
        .expect("batch update failed");
        assert_eq!(affected, 4);

        let done_tasks =
            query_items(&conn, &ItemQuery::new().with_statuses(vec![1])).expect("query failed");
        assert_eq!(done_tasks.len(), 4);

        // Test 3: Update both category and status
        insert_task(&conn, "personal", "task 5", "today");
        let updates = ItemUpdates {
            category: Some("home".to_string()),
            status: Some(2),
            target_time: None,
        };
        let affected = batch_update_items(
            &conn,
            Some(vec![TASK]),
            Some("personal"),
            None,
            None,
            None,
            None,
            None,
            &updates,
        )
        .expect("batch update failed");
        assert_eq!(affected, 1);

        let home_tasks = query_items(&conn, &ItemQuery::new().with_category("home"))
            .expect("query failed");
        assert_eq!(home_tasks.len(), 1);
        assert_eq!(home_tasks[0].status, 2);

        // Test 4: Update with time range
        let time_1000 = 1000;
        let time_2000 = 2000;
        let time_3000 = 3000;

        let item1 = Item::with_create_time(RECORD.to_string(), "work".to_string(), "rec1".to_string(), time_1000);
        insert_item(&conn, &item1).expect("insert failed");

        let item2 = Item::with_create_time(RECORD.to_string(), "work".to_string(), "rec2".to_string(), time_2000);
        insert_item(&conn, &item2).expect("insert failed");

        let item3 = Item::with_create_time(RECORD.to_string(), "work".to_string(), "rec3".to_string(), time_3000);
        insert_item(&conn, &item3).expect("insert failed");

        let updates = ItemUpdates {
            category: Some("archived".to_string()),
            status: None,
            target_time: None,
        };
        let affected = batch_update_items(
            &conn,
            Some(vec![RECORD]),
            Some("work"),
            Some(1500),
            Some(2500),
            None,
            None,
            None,
            &updates,
        )
        .expect("batch update failed");
        assert_eq!(affected, 1); // Only time_2000

        let archived = query_items(&conn, &ItemQuery::new().with_category("archived"))
            .expect("query failed");
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].content, "rec2");

        let work_items = query_items(&conn, &ItemQuery::new().with_category("work"))
            .expect("query failed");
        assert_eq!(work_items.len(), 3); // 1 from Test 1 + 2 from Test 4

        // Test 5: Update target_time
        let new_target = 9999999;
        let updates = ItemUpdates {
            category: None,
            status: None,
            target_time: Some(new_target),
        };
        let affected = batch_update_items(
            &conn,
            Some(vec![TASK]),
            Some("home"),
            None,
            None,
            None,
            None,
            None,
            &updates,
        )
        .expect("batch update failed");
        assert_eq!(affected, 1);

        let home_tasks = query_items(&conn, &ItemQuery::new().with_category("home"))
            .expect("query failed");
        assert_eq!(home_tasks[0].target_time, Some(new_target));

        // Test 6: Error on empty updates
        let updates = ItemUpdates {
            category: None,
            status: None,
            target_time: None,
        };
        let result = batch_update_items(
            &conn,
            Some(vec![TASK]),
            None,
            None,
            None,
            None,
            None,
            None,
            &updates,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_delete_items() {
        let (conn, _temp_file) = get_test_conn();

        // Test 1: Delete by action and category
        insert_task(&conn, "work", "task 1", "today");
        insert_task(&conn, "work", "task 2", "today");
        insert_task(&conn, "work", "task 3", "tomorrow");
        insert_task(&conn, "life", "task 4", "today");
        insert_record(&conn, "work", "record 1", "yesterday");

        let affected = batch_delete_items(
            &conn,
            Some(vec![TASK]),
            Some("work"),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("batch delete failed");
        assert_eq!(affected, 3);

        let work_tasks = query_items(
            &conn,
            &ItemQuery::new().with_action(TASK).with_category("work"),
        )
        .expect("query failed");
        assert_eq!(work_tasks.len(), 0);

        let life_tasks = query_items(
            &conn,
            &ItemQuery::new().with_action(TASK).with_category("life"),
        )
        .expect("query failed");
        assert_eq!(life_tasks.len(), 1);

        let work_records = query_items(
            &conn,
            &ItemQuery::new().with_action(RECORD).with_category("work"),
        )
        .expect("query failed");
        assert_eq!(work_records.len(), 1);

        // Test 2: Delete with time range
        let time_1000 = 1000;
        let time_2000 = 2000;
        let time_3000 = 3000;

        let mut item1 = Item::with_create_time(TASK.to_string(), "cleanup".to_string(), "task1".to_string(), time_1000);
        item1.target_time = Some(time_1000);
        insert_item(&conn, &item1).expect("insert failed");

        let mut item2 = Item::with_create_time(TASK.to_string(), "cleanup".to_string(), "task2".to_string(), time_2000);
        item2.target_time = Some(time_2000);
        insert_item(&conn, &item2).expect("insert failed");

        let mut item3 = Item::with_create_time(TASK.to_string(), "cleanup".to_string(), "task3".to_string(), time_3000);
        item3.target_time = Some(time_3000);
        insert_item(&conn, &item3).expect("insert failed");

        let affected = batch_delete_items(
            &conn,
            Some(vec![TASK]),
            Some("cleanup"),
            Some(1500),
            Some(2500),
            None,
            None,
            None,
        )
        .expect("batch delete failed");
        assert_eq!(affected, 1); // Only time_2000

        let remaining = query_items(&conn, &ItemQuery::new().with_category("cleanup"))
            .expect("query failed");
        assert_eq!(remaining.len(), 2);
        let contents: Vec<&str> = remaining.iter().map(|i| i.content.as_str()).collect();
        assert!(contents.contains(&"task1"));
        assert!(contents.contains(&"task3"));

        // Test 3: Delete with status filter
        let id1 = insert_task(&conn, "archive", "task 1", "today");
        let id2 = insert_task(&conn, "archive", "task 2", "today");
        let _id3 = insert_task(&conn, "archive", "task 3", "today");

        update_status(&conn, id1, 1); // done
        update_status(&conn, id2, 2); // cancelled
        // _id3 remains status 0 (ongoing)

        let affected = batch_delete_items(
            &conn,
            Some(vec![TASK]),
            Some("archive"),
            None,
            None,
            None,
            None,
            Some(vec![1, 2]),
        )
        .expect("batch delete failed");
        assert_eq!(affected, 2);

        let remaining = query_items(&conn, &ItemQuery::new().with_category("archive"))
            .expect("query failed");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].status, 0);
    }
}
