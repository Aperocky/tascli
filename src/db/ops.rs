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

pub fn get_stats(
    conn: &Connection,
    actions: Option<Vec<&str>>,
    category: Option<&str>,
    create_time_min: Option<i64>,
    create_time_max: Option<i64>,
    target_time_min: Option<i64>,
    target_time_max: Option<i64>,
) -> Result<StatTable> {
    let (where_clause, params) = build_batch_where_clause(
        actions,
        category,
        create_time_min,
        create_time_max,
        target_time_min,
        target_time_max,
        None, // statuses not used for stats
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

    use crate::db::crud::{
        insert_item,
        query_items,
    };
    use crate::db::item::ItemQuery;

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
        let stats = get_stats(&conn, None, None, None, None, None, None).expect("get_stats failed");

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
        let stats = get_stats(&conn, None, Some("work"), None, None, None, None)
            .expect("get_stats failed");

        assert_eq!(stats.rows.len(), 1);
        assert_eq!(stats.rows[0].category, "work");
        assert_eq!(stats.rows[0].total, 5);
        assert_eq!(stats.totals.total, 5);

        // Test 3: Filter by action
        let stats = get_stats(&conn, Some(vec![TASK]), None, None, None, None, None)
            .expect("get_stats failed");

        assert_eq!(stats.rows.len(), 2);
        assert_eq!(stats.totals.task, 3);
        assert_eq!(stats.totals.record, 0);
        assert_eq!(stats.totals.total, 3);

        // Test 4: Filter by time range
        let time_500 = 500;
        let time_1500 = 1500;
        let item1 = Item::with_create_time(RECORD.to_string(), "early".to_string(), "early rec".to_string(), time_500);
        insert_item(&conn, &item1).expect("insert failed");
        let item2 = Item::with_create_time(RECORD.to_string(), "later".to_string(), "later rec".to_string(), time_1500);
        insert_item(&conn, &item2).expect("insert failed");

        let stats = get_stats(&conn, None, None, Some(1000), None, None, None)
            .expect("get_stats failed");

        let later_row = stats.rows.iter().find(|r| r.category == "later");
        assert!(later_row.is_some());
        assert_eq!(later_row.unwrap().record, 1);

        let early_row = stats.rows.iter().find(|r| r.category == "early");
        assert!(early_row.is_none());

        // Test 5: No results
        let stats = get_stats(&conn, None, Some("nonexistent"), None, None, None, None)
            .expect("get_stats failed");

        assert_eq!(stats.rows.len(), 0);
        assert_eq!(stats.totals.total, 0);
    }
}
