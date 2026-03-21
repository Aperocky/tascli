use std::path::Path;

use rusqlite::Connection;

use crate::actions::display;
use crate::args::parser::OpsMergeCommand;
use crate::config::get_data_path;
use crate::db::conn::init_table;
use crate::db::item::Item;

use super::backup::backup_path;

fn insert_item_with_status(conn: &Connection, item: &Item) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO items (action, category, content, create_time, target_time, status, cron_schedule, human_schedule, recurring_task_id, good_until)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            item.action,
            item.category,
            item.content,
            item.create_time,
            item.target_time,
            item.status,
            item.cron_schedule,
            item.human_schedule,
            item.recurring_task_id,
            item.good_until
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn handle_mergecmd(conn: &Connection, cmd: &OpsMergeCommand) -> Result<(), String> {
    let source_path = Path::new(&cmd.path);
    if !source_path.exists() {
        return Err(format!("Source database not found: {}", cmd.path));
    }

    // Guard against merging the db into itself
    let current_db = get_data_path()?;
    let source_canonical = source_path.canonicalize().map_err(|e| e.to_string())?;
    let dest_canonical = current_db.canonicalize().map_err(|e| e.to_string())?;
    if source_canonical == dest_canonical {
        return Err("Source and destination are the same database".to_string());
    }

    // Backup current db before merge
    display::print_bold("backing up database prior to merge operation");
    backup_path(None)?;

    let source_conn =
        Connection::open(source_path).map_err(|e| format!("Failed to open source db: {}", e))?;
    init_table(&source_conn).map_err(|e| format!("Source db schema issue: {}", e))?;

    let mut stmt = source_conn
        .prepare(
            "SELECT id, action, category, content, create_time, target_time,
                    modify_time, status, cron_schedule, human_schedule,
                    recurring_task_id, good_until
             FROM items",
        )
        .map_err(|e| e.to_string())?;

    let source_items: Vec<Item> = stmt
        .query_map([], |row| Ok(Item::from_row(row).unwrap()))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut inserted = 0;
    let mut skipped = 0;

    for item in &source_items {
        if is_duplicate(conn, item)? {
            skipped += 1;
        } else {
            insert_item_with_status(conn, item).map_err(|e| e.to_string())?;
            inserted += 1;
        }
    }

    display::print_bold(&format!(
        "Merge complete: {} items inserted, {} duplicates skipped",
        inserted, skipped
    ));
    Ok(())
}

fn is_duplicate(conn: &Connection, item: &Item) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM items
             WHERE action = ?1 AND category = ?2 AND content = ?3
               AND create_time = ?4
               AND status = ?5
               AND COALESCE(target_time, -1) = COALESCE(?6, -1)
               AND COALESCE(cron_schedule, '') = COALESCE(?7, '')
               AND COALESCE(human_schedule, '') = COALESCE(?8, '')
               AND COALESCE(recurring_task_id, -1) = COALESCE(?9, -1)
               AND COALESCE(good_until, -1) = COALESCE(?10, -1)",
            rusqlite::params![
                item.action,
                item.category,
                item.content,
                item.create_time,
                item.status,
                item.target_time,
                item.cron_schedule,
                item.human_schedule,
                item.recurring_task_id,
                item.good_until,
            ],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::conn::init_table;
    use crate::db::crud::insert_item;
    use crate::db::item::{RECORD, TASK};
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_table(&conn).unwrap();
        conn
    }

    fn setup_db_file(items: &[Item]) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let conn = Connection::open(&path).unwrap();
        init_table(&conn).unwrap();
        for item in items {
            insert_item(&conn, item).unwrap();
        }
        drop(conn);
        file
    }

    fn count_items(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
            .unwrap()
    }

    fn query_items(conn: &Connection) -> Vec<Item> {
        let mut stmt = conn
            .prepare(
                "SELECT id, action, category, content, create_time, target_time,
                        modify_time, status, cron_schedule, human_schedule,
                        recurring_task_id, good_until FROM items ORDER BY id",
            )
            .unwrap();
        stmt.query_map([], |row| Ok(Item::from_row(row).unwrap()))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    fn make_task(category: &str, content: &str, target_time: i64) -> Item {
        Item::with_target_time(TASK.into(), category.into(), content.into(), Some(target_time))
    }

    fn make_record(category: &str, content: &str, create_time: i64) -> Item {
        Item::with_create_time(RECORD.into(), category.into(), content.into(), create_time)
    }

    // --- Basic operations ---

    #[test]
    fn test_merge_inserts_new_items() {
        let dest = setup_conn();
        let source_file = setup_db_file(&[make_task("work", "do stuff", 1000)]);

        let items = query_items(&Connection::open(source_file.path()).unwrap());
        assert_eq!(items.len(), 1);
        assert!(!is_duplicate(&dest, &items[0]).unwrap());
        insert_item(&dest, &items[0]).unwrap();
        assert_eq!(count_items(&dest), 1);
    }

    #[test]
    fn test_merge_skips_duplicates() {
        let dest = setup_conn();
        let item = make_task("work", "do stuff", 1000);
        insert_item(&dest, &item).unwrap();
        assert!(is_duplicate(&dest, &item).unwrap());
    }

    #[test]
    fn test_merge_different_content_not_duplicate() {
        let dest = setup_conn();
        insert_item(&dest, &make_task("work", "do stuff", 1000)).unwrap();
        assert!(!is_duplicate(&dest, &make_task("work", "different stuff", 1000)).unwrap());
    }

    #[test]
    fn test_merge_nonexistent_path() {
        let dest = setup_conn();
        let cmd = OpsMergeCommand {
            path: "/nonexistent/path.db".into(),
        };
        let result = handle_mergecmd(&dest, &cmd);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    // --- Empty db edge cases ---

    #[test]
    fn test_merge_from_empty_source() {
        let dest = setup_conn();
        insert_item(&dest, &make_task("work", "existing", 1000)).unwrap();

        let source_file = setup_db_file(&[]);
        let source_items = query_items(&Connection::open(source_file.path()).unwrap());
        assert_eq!(source_items.len(), 0);
        assert_eq!(count_items(&dest), 1);
    }

    #[test]
    fn test_merge_into_empty_dest() {
        let dest = setup_conn();
        assert_eq!(count_items(&dest), 0);

        let items = vec![
            make_task("work", "task1", 1000),
            make_record("life", "record1", 2000),
        ];
        for item in &items {
            assert!(!is_duplicate(&dest, item).unwrap());
            insert_item(&dest, item).unwrap();
        }
        assert_eq!(count_items(&dest), 2);
    }

    #[test]
    fn test_merge_both_empty() {
        let dest = setup_conn();
        let source_file = setup_db_file(&[]);
        let source_items = query_items(&Connection::open(source_file.path()).unwrap());
        assert_eq!(source_items.len(), 0);
        assert_eq!(count_items(&dest), 0);
    }

    // --- Overlapping and non-overlapping data ---

    #[test]
    fn test_merge_fully_overlapping() {
        let dest = setup_conn();
        let items = vec![
            make_task("work", "task1", 1000),
            make_task("work", "task2", 2000),
            make_record("life", "record1", 3000),
        ];
        for item in &items {
            insert_item(&dest, item).unwrap();
        }
        assert_eq!(count_items(&dest), 3);

        // All source items are duplicates
        for item in &items {
            assert!(is_duplicate(&dest, item).unwrap());
        }
    }

    #[test]
    fn test_merge_fully_non_overlapping() {
        let dest = setup_conn();
        insert_item(&dest, &make_task("work", "dest task", 1000)).unwrap();
        insert_item(&dest, &make_record("life", "dest record", 2000)).unwrap();

        let source_items = vec![
            make_task("personal", "source task", 3000),
            make_record("hobby", "source record", 4000),
        ];
        for item in &source_items {
            assert!(!is_duplicate(&dest, item).unwrap());
            insert_item(&dest, item).unwrap();
        }
        assert_eq!(count_items(&dest), 4);
    }

    #[test]
    fn test_merge_partial_overlap() {
        let dest = setup_conn();
        let shared = make_task("work", "shared task", 1000);
        insert_item(&dest, &shared).unwrap();
        insert_item(&dest, &make_record("life", "dest only", 2000)).unwrap();

        let source_items = vec![
            shared.clone(),                              // duplicate
            make_task("work", "source only", 3000),      // new
            make_record("life", "source record", 4000),  // new
        ];

        let mut inserted = 0;
        let mut skipped = 0;
        for item in &source_items {
            if is_duplicate(&dest, item).unwrap() {
                skipped += 1;
            } else {
                insert_item(&dest, item).unwrap();
                inserted += 1;
            }
        }
        assert_eq!(inserted, 2);
        assert_eq!(skipped, 1);
        assert_eq!(count_items(&dest), 4); // 2 original + 2 new
    }

    // --- Tasks vs records separately ---

    #[test]
    fn test_merge_only_tasks() {
        let dest = setup_conn();
        insert_item(&dest, &make_task("work", "existing task", 1000)).unwrap();

        let source_items = vec![
            make_task("work", "existing task", 1000), // dup
            make_task("work", "new task", 2000),      // new
            make_task("personal", "another", 3000),   // new
        ];

        let mut inserted = 0;
        for item in &source_items {
            if !is_duplicate(&dest, item).unwrap() {
                insert_item(&dest, item).unwrap();
                inserted += 1;
            }
        }
        assert_eq!(inserted, 2);
        assert_eq!(count_items(&dest), 3);

        let all = query_items(&dest);
        assert!(all.iter().all(|i| i.action == TASK));
    }

    #[test]
    fn test_merge_only_records() {
        let dest = setup_conn();
        insert_item(&dest, &make_record("feeding", "100ML", 1000)).unwrap();

        let source_items = vec![
            make_record("feeding", "100ML", 1000),    // dup
            make_record("feeding", "110ML", 2000),    // new
            make_record("exercise", "5K run", 3000),  // new
        ];

        let mut inserted = 0;
        for item in &source_items {
            if !is_duplicate(&dest, item).unwrap() {
                insert_item(&dest, item).unwrap();
                inserted += 1;
            }
        }
        assert_eq!(inserted, 2);
        assert_eq!(count_items(&dest), 3);

        let all = query_items(&dest);
        assert!(all.iter().all(|i| i.action == RECORD));
    }

    // --- Recurring items ---

    #[test]
    fn test_merge_recurring_tasks() {
        let dest = setup_conn();
        let recurring = Item::create_recurring_task(
            "life".into(),
            "write diary".into(),
            "0 0 21 * * *".into(),
            "Daily 9PM".into(),
        );
        insert_item(&dest, &recurring).unwrap();

        // Same recurring task is a duplicate
        assert!(is_duplicate(&dest, &recurring).unwrap());

        // Different schedule is not a duplicate
        let different_schedule = Item::create_recurring_task(
            "life".into(),
            "write diary".into(),
            "0 0 9 * * *".into(),
            "Daily 9AM".into(),
        );
        assert!(!is_duplicate(&dest, &different_schedule).unwrap());
    }

    #[test]
    fn test_merge_recurring_records() {
        let dest = setup_conn();
        let rec = Item::create_recurring_record("life".into(), "wrote diary".into(), 42, 99999);
        insert_item(&dest, &rec).unwrap();

        assert!(is_duplicate(&dest, &rec).unwrap());

        // Different good_until is not a duplicate
        let different_gu =
            Item::create_recurring_record("life".into(), "wrote diary".into(), 42, 88888);
        assert!(!is_duplicate(&dest, &different_gu).unwrap());

        // Different recurring_task_id is not a duplicate
        let different_rt =
            Item::create_recurring_record("life".into(), "wrote diary".into(), 99, 99999);
        assert!(!is_duplicate(&dest, &different_rt).unwrap());
    }

    // --- Dedupe field sensitivity ---

    #[test]
    fn test_dedupe_differs_by_category() {
        let dest = setup_conn();
        insert_item(&dest, &make_task("work", "do stuff", 1000)).unwrap();
        assert!(!is_duplicate(&dest, &make_task("personal", "do stuff", 1000)).unwrap());
    }

    #[test]
    fn test_dedupe_differs_by_action_type() {
        let dest = setup_conn();
        insert_item(&dest, &make_task("work", "something", 1000)).unwrap();
        // A record with same content/category/time is NOT a duplicate of a task
        let record = make_record("work", "something", 1000);
        assert!(!is_duplicate(&dest, &record).unwrap());
    }

    #[test]
    fn test_dedupe_differs_by_status() {
        let dest = setup_conn();
        let mut task = make_task("work", "do stuff", 1000);
        task.status = 1; // done
        insert_item_with_status(&dest, &task).unwrap();

        let mut same_but_open = make_task("work", "do stuff", 1000);
        same_but_open.status = 0; // ongoing
        assert!(!is_duplicate(&dest, &same_but_open).unwrap());
    }

    #[test]
    fn test_dedupe_differs_by_target_time() {
        let dest = setup_conn();
        insert_item(&dest, &make_task("work", "do stuff", 1000)).unwrap();
        assert!(!is_duplicate(&dest, &make_task("work", "do stuff", 2000)).unwrap());
    }

    #[test]
    fn test_dedupe_null_target_time_handling() {
        let dest = setup_conn();
        // Record has no target_time (None)
        let rec = make_record("life", "something", 5000);
        insert_item(&dest, &rec).unwrap();
        assert!(is_duplicate(&dest, &rec).unwrap());

        // A task with target_time set is not a dup of a record with None
        let task = make_task("life", "something", 5000);
        assert!(!is_duplicate(&dest, &task).unwrap());
    }

    // --- Large merge ---

    #[test]
    fn test_merge_many_items_mixed() {
        let dest = setup_conn();
        // Pre-populate dest with 5 tasks
        for i in 0..5 {
            insert_item(&dest, &make_task("work", &format!("task {}", i), i * 1000)).unwrap();
        }

        // Source has 3 overlapping + 7 new
        let mut source_items = Vec::new();
        for i in 0..10 {
            if i < 5 && i % 2 == 0 {
                // i=0,2,4 overlap with dest
                source_items.push(make_task("work", &format!("task {}", i), i * 1000));
            } else {
                source_items.push(make_record("log", &format!("record {}", i), i * 1000));
            }
        }

        let mut inserted = 0;
        let mut skipped = 0;
        for item in &source_items {
            if is_duplicate(&dest, item).unwrap() {
                skipped += 1;
            } else {
                insert_item(&dest, item).unwrap();
                inserted += 1;
            }
        }
        assert_eq!(skipped, 3);
        assert_eq!(inserted, 7);
        assert_eq!(count_items(&dest), 12); // 5 original + 7 new
    }

    // --- Status preservation ---

    #[test]
    fn test_merge_preserves_status() {
        let dest = setup_conn();
        let mut done_task = make_task("work", "finished", 1000);
        done_task.status = 1; // done
        insert_item_with_status(&dest, &done_task).unwrap();

        // Verify the status was actually written
        let items = query_items(&dest);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, 1);
    }

    #[test]
    fn test_merge_status_preserved_end_to_end() {
        let dest = setup_conn();

        // Simulate merging a completed task and a cancelled task
        let mut completed = make_task("work", "completed task", 1000);
        completed.status = 1;
        let mut cancelled = make_task("work", "cancelled task", 2000);
        cancelled.status = 3;

        let source_file = NamedTempFile::new().unwrap();
        let source_path = source_file.path().to_str().unwrap().to_string();
        let source_conn = Connection::open(&source_path).unwrap();
        init_table(&source_conn).unwrap();
        insert_item_with_status(&source_conn, &completed).unwrap();
        insert_item_with_status(&source_conn, &cancelled).unwrap();
        drop(source_conn);

        // Read from source and merge into dest
        let source_conn = Connection::open(&source_path).unwrap();
        let source_items = query_items(&source_conn);
        for item in &source_items {
            if !is_duplicate(&dest, item).unwrap() {
                insert_item_with_status(&dest, item).unwrap();
            }
        }

        let dest_items = query_items(&dest);
        assert_eq!(dest_items.len(), 2);
        assert_eq!(dest_items[0].status, 1); // completed
        assert_eq!(dest_items[1].status, 3); // cancelled
    }

    // --- Content with special characters ---

    #[test]
    fn test_merge_content_with_special_chars() {
        let dest = setup_conn();
        let item = Item::with_create_time(
            RECORD.into(),
            "life".into(),
            "notes: 'quotes' & \"doubles\" and\nnewlines".into(),
            5000,
        );
        insert_item(&dest, &item).unwrap();
        assert!(is_duplicate(&dest, &item).unwrap());

        // Slightly different content is not a dup
        let different = Item::with_create_time(
            RECORD.into(),
            "life".into(),
            "notes: 'quotes' & \"doubles\" and\nnewlines!".into(),
            5000,
        );
        assert!(!is_duplicate(&dest, &different).unwrap());
    }
}
