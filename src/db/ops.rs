use rusqlite::{
    params_from_iter,
    Connection,
    Result,
};

#[derive(Debug)]
pub struct ItemUpdates {
    pub category: Option<String>,
    pub status: Option<u8>,
    pub target_time: Option<i64>,
}

#[derive(Debug, PartialEq)]
pub struct StatRow {
    pub category: String,
    pub task: usize,
    pub record: usize,
    pub recurring_task: usize,
    pub recurring_task_record: usize,
    pub total: usize,
}

#[derive(Debug)]
pub struct StatTable {
    pub rows: Vec<StatRow>,
    pub totals: StatRow,
}

pub fn build_stat_where_clause(
    category: Option<&str>,
    create_time_min: Option<i64>,
    create_time_max: Option<i64>,
    target_time_min: Option<i64>,
    target_time_max: Option<i64>,
) -> (String, Vec<String>) {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

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

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    (where_clause, params)
}

pub fn batch_update_items(
    conn: &Connection,
    item_ids: &[i64],
    updates: &ItemUpdates,
) -> Result<usize> {
    if item_ids.is_empty() {
        return Ok(0);
    }

    let mut set_parts = Vec::new();
    let mut set_params: Vec<String> = Vec::new();

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

    // Build WHERE id IN (?, ?, ...)
    let placeholders = vec!["?"; item_ids.len()].join(", ");
    let mut all_params = set_params;
    all_params.extend(item_ids.iter().map(|id| id.to_string()));

    let query = format!("UPDATE items SET {} WHERE id IN ({})", set_clause, placeholders);
    let affected = conn.execute(&query, params_from_iter(all_params))?;

    Ok(affected)
}

pub fn batch_delete_items(
    conn: &Connection,
    item_ids: &[i64],
) -> Result<usize> {
    if item_ids.is_empty() {
        return Ok(0);
    }

    let placeholders = vec!["?"; item_ids.len()].join(", ");
    let params: Vec<String> = item_ids.iter().map(|id| id.to_string()).collect();

    let query = format!("DELETE FROM items WHERE id IN ({})", placeholders);
    let affected = conn.execute(&query, params_from_iter(params))?;

    Ok(affected)
}

pub fn get_stats(
    conn: &Connection,
    category: Option<&str>,
    create_time_min: Option<i64>,
    create_time_max: Option<i64>,
    target_time_min: Option<i64>,
    target_time_max: Option<i64>,
) -> Result<StatTable> {
    let (where_clause, params) = build_stat_where_clause(
        category,
        create_time_min,
        create_time_max,
        target_time_min,
        target_time_max,
    );

    let query = format!(
        "SELECT category, action, COUNT(*) as count FROM items{} GROUP BY category, action ORDER BY category, action",
        where_clause
    );

    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map(params_from_iter(params), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, usize>(2)?,
        ))
    })?;

    use std::collections::HashMap;
    let mut data: HashMap<String, HashMap<String, usize>> = HashMap::new();

    for row_result in rows {
        let (cat, action, count) = row_result?;
        data.entry(cat).or_insert_with(HashMap::new).insert(action, count);
    }

    let mut stat_rows = Vec::new();
    let mut total_task = 0;
    let mut total_record = 0;
    let mut total_recurring_task = 0;
    let mut total_recurring_task_record = 0;

    for (cat, actions_map) in data.iter() {
        let task = *actions_map.get("task").unwrap_or(&0);
        let record = *actions_map.get("record").unwrap_or(&0);
        let recurring_task = *actions_map.get("recurring_task").unwrap_or(&0);
        let recurring_task_record = *actions_map.get("recurring_task_record").unwrap_or(&0);
        let row_total = task + record + recurring_task + recurring_task_record;

        total_task += task;
        total_record += record;
        total_recurring_task += recurring_task;
        total_recurring_task_record += recurring_task_record;

        stat_rows.push(StatRow {
            category: cat.clone(),
            task,
            record,
            recurring_task,
            recurring_task_record,
            total: row_total,
        });
    }

    stat_rows.sort_by(|a, b| b.total.cmp(&a.total));

    let totals = StatRow {
        category: "TOTAL".to_string(),
        task: total_task,
        record: total_record,
        recurring_task: total_recurring_task,
        recurring_task_record: total_recurring_task_record,
        total: total_task + total_record + total_recurring_task + total_recurring_task_record,
    };

    Ok(StatTable {
        rows: stat_rows,
        totals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::item::{
            Item,
            RECORD,
        },
        tests::{
            get_test_conn,
            insert_record,
            insert_recurring_record,
            insert_recurring_task,
            insert_task,
        },
    };

    use crate::db::crud::{
        get_item,
        insert_item,
    };

    #[test]
    fn test_build_stat_where_clause() {
        // Test with all parameters
        let (where_clause, params) = build_stat_where_clause(
            Some("work"),
            Some(1000),
            Some(2000),
            Some(3000),
            Some(4000),
        );
        assert_eq!(
            where_clause,
            " WHERE category = ? AND create_time > ? AND create_time <= ? AND target_time > ? AND target_time <= ?"
        );
        assert_eq!(params, vec!["work", "1000", "2000", "3000", "4000"]);

        // Test with only category
        let (where_clause, params) = build_stat_where_clause(
            Some("life"),
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            where_clause,
            " WHERE category = ?"
        );
        assert_eq!(params, vec!["life"]);

        // Test with only time ranges
        let (where_clause, params) = build_stat_where_clause(
            None,
            Some(5000),
            Some(6000),
            None,
            None,
        );
        assert_eq!(
            where_clause,
            " WHERE create_time > ? AND create_time <= ?"
        );
        assert_eq!(params, vec!["5000", "6000"]);

        // Test with no parameters (empty WHERE clause)
        let (where_clause, params) = build_stat_where_clause(
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(where_clause, "");
        assert_eq!(params.len(), 0);

        // Test with target_time only
        let (where_clause, params) = build_stat_where_clause(
            None,
            None,
            None,
            Some(7000),
            Some(8000),
        );
        assert_eq!(
            where_clause,
            " WHERE target_time > ? AND target_time <= ?"
        );
        assert_eq!(params, vec!["7000", "8000"]);
    }

    #[test]
    fn test_batch_update_items_by_ids() {
        let (conn, _temp_file) = get_test_conn();

        // Test 1: Update category by IDs
        let id1 = insert_task(&conn, "work", "task 1", "today");
        let id2 = insert_task(&conn, "work", "task 2", "today");
        let id3 = insert_task(&conn, "personal", "task 3", "today");
        insert_record(&conn, "work", "record 1", "yesterday"); // Not updated

        let updates = ItemUpdates {
            category: Some("meetings".to_string()),
            status: None,
            target_time: None,
        };
        let affected = batch_update_items(&conn, &[id1, id2], &updates).unwrap();
        assert_eq!(affected, 2);

        // Verify updates
        let item1 = get_item(&conn, id1).unwrap();
        let item2 = get_item(&conn, id2).unwrap();
        let item3 = get_item(&conn, id3).unwrap();
        assert_eq!(item1.category, "meetings");
        assert_eq!(item2.category, "meetings");
        assert_eq!(item3.category, "personal"); // Unchanged

        // Test 2: Update status
        let updates = ItemUpdates {
            category: None,
            status: Some(1), // completed
            target_time: None,
        };
        let affected = batch_update_items(&conn, &[id1, id3], &updates).unwrap();
        assert_eq!(affected, 2);

        let item1 = get_item(&conn, id1).unwrap();
        let item2 = get_item(&conn, id2).unwrap();
        let item3 = get_item(&conn, id3).unwrap();
        assert_eq!(item1.status, 1);
        assert_eq!(item2.status, 0); // Unchanged
        assert_eq!(item3.status, 1);

        // Test 3: Update multiple fields
        let new_target = 9999999;
        let updates = ItemUpdates {
            category: Some("urgent".to_string()),
            status: Some(2), // cancelled
            target_time: Some(new_target),
        };
        let affected = batch_update_items(&conn, &[id2], &updates).unwrap();
        assert_eq!(affected, 1);

        let item2 = get_item(&conn, id2).unwrap();
        assert_eq!(item2.category, "urgent");
        assert_eq!(item2.status, 2);
        assert_eq!(item2.target_time, Some(new_target));

        // Test 4: Empty IDs returns 0
        let updates = ItemUpdates {
            category: Some("test".to_string()),
            status: None,
            target_time: None,
        };
        let affected = batch_update_items(&conn, &[], &updates).unwrap();
        assert_eq!(affected, 0);

        // Test 5: Error on empty updates
        let updates = ItemUpdates {
            category: None,
            status: None,
            target_time: None,
        };
        let result = batch_update_items(&conn, &[id1], &updates);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_delete_items_by_ids() {
        let (conn, _temp_file) = get_test_conn();

        // Test 1: Delete specific items by IDs
        let id1 = insert_task(&conn, "work", "task 1", "today");
        let id2 = insert_task(&conn, "work", "task 2", "today");
        let id3 = insert_task(&conn, "personal", "task 3", "today");
        let id4 = insert_record(&conn, "work", "record 1", "yesterday");

        let affected = batch_delete_items(&conn, &[id1, id2]).unwrap();
        assert_eq!(affected, 2);

        // Verify deletions
        assert!(get_item(&conn, id1).is_err()); // Deleted
        assert!(get_item(&conn, id2).is_err()); // Deleted
        assert!(get_item(&conn, id3).is_ok()); // Still exists
        assert!(get_item(&conn, id4).is_ok()); // Still exists

        // Test 2: Delete mixed item types
        let id5 = insert_task(&conn, "cleanup", "task 5", "today");
        let id6 = insert_record(&conn, "cleanup", "record 2", "yesterday");
        let rt_id = insert_recurring_task(&conn, "cleanup", "recurring", "Daily 9AM");

        let affected = batch_delete_items(&conn, &[id5, id6, rt_id]).unwrap();
        assert_eq!(affected, 3);

        assert!(get_item(&conn, id5).is_err());
        assert!(get_item(&conn, id6).is_err());
        assert!(get_item(&conn, rt_id).is_err());

        // Test 3: Empty IDs returns 0
        let affected = batch_delete_items(&conn, &[]).unwrap();
        assert_eq!(affected, 0);

        // Test 4: Non-existent IDs (graceful handling)
        let affected = batch_delete_items(&conn, &[999999, 888888]).unwrap();
        assert_eq!(affected, 0);

        // Verify remaining items still exist
        assert!(get_item(&conn, id3).is_ok());
        assert!(get_item(&conn, id4).is_ok());
    }

    #[test]
    fn test_get_stats() {
        let (conn, _temp_file) = get_test_conn();

        // Insert test data
        insert_task(&conn, "work", "task 1", "today");
        insert_task(&conn, "work", "task 2", "today");
        insert_task(&conn, "personal", "task 3", "today");
        insert_record(&conn, "work", "record 1", "yesterday");
        insert_record(&conn, "personal", "record 2", "yesterday");
        insert_record(&conn, "personal", "record 3", "yesterday");
        let rt_id = insert_recurring_task(&conn, "work", "standup", "Daily 9AM");
        insert_recurring_record(&conn, "work", "standup done", rt_id, 1000);

        // Test 1: Get all stats
        let stats = get_stats(&conn, None, None, None, None, None).expect("get_stats failed");

        assert_eq!(stats.rows.len(), 2);

        // Verify sorted by total descending
        assert_eq!(stats.rows[0].category, "work");
        assert_eq!(stats.rows[0].total, 5);
        assert_eq!(stats.rows[1].category, "personal");
        assert_eq!(stats.rows[1].total, 3);

        let personal_row = stats.rows.iter().find(|r| r.category == "personal").unwrap();
        assert_eq!(personal_row.task, 1);
        assert_eq!(personal_row.record, 2);
        assert_eq!(personal_row.recurring_task, 0);
        assert_eq!(personal_row.recurring_task_record, 0);
        assert_eq!(personal_row.total, 3);

        let work_row = stats.rows.iter().find(|r| r.category == "work").unwrap();
        assert_eq!(work_row.task, 2);
        assert_eq!(work_row.record, 1);
        assert_eq!(work_row.recurring_task, 1);
        assert_eq!(work_row.recurring_task_record, 1);
        assert_eq!(work_row.total, 5);

        assert_eq!(stats.totals.task, 3);
        assert_eq!(stats.totals.record, 3);
        assert_eq!(stats.totals.recurring_task, 1);
        assert_eq!(stats.totals.recurring_task_record, 1);
        assert_eq!(stats.totals.total, 8);

        // Test 2: Filter by category
        let stats = get_stats(&conn, Some("work"), None, None, None, None)
            .expect("get_stats failed");

        assert_eq!(stats.rows.len(), 1);
        assert_eq!(stats.rows[0].category, "work");
        assert_eq!(stats.rows[0].total, 5);
        assert_eq!(stats.totals.total, 5);

        // Test 3: Filter by time range
        let time_500 = 500;
        let time_1500 = 1500;
        let item1 = Item::with_create_time(RECORD.to_string(), "early".to_string(), "early rec".to_string(), time_500);
        insert_item(&conn, &item1).expect("insert failed");
        let item2 = Item::with_create_time(RECORD.to_string(), "later".to_string(), "later rec".to_string(), time_1500);
        insert_item(&conn, &item2).expect("insert failed");

        let stats = get_stats(&conn, None, Some(1000), None, None, None)
            .expect("get_stats failed");

        let later_row = stats.rows.iter().find(|r| r.category == "later");
        assert!(later_row.is_some());
        assert_eq!(later_row.unwrap().record, 1);

        let early_row = stats.rows.iter().find(|r| r.category == "early");
        assert!(early_row.is_none());

        // Test 4: No results
        let stats = get_stats(&conn, Some("nonexistent"), None, None, None, None)
            .expect("get_stats failed");

        assert_eq!(stats.rows.len(), 0);
        assert_eq!(stats.totals.total, 0);
    }
}
