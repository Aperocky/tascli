use rusqlite::Connection;

use crate::{
    actions::display,
    args::{
        parser::ListRecordCommand,
        timestr,
    },
    db::{
        cache,
        crud::query_items,
        item::{Item, ItemQuery, Offset},
    },
};

use super::{handle_next_page, CREATE_TIME_COL};

pub fn handle_listrecords(conn: &Connection, cmd: ListRecordCommand) -> Result<(), String> {
    let records = match query_records(conn, &cmd) {
        Ok(records) => records,
        Err(estr) => {
            display::print_bold(&estr);
            return Ok(());
        }
    };
    if records.is_empty() {
        display::print_bold("No records found");
        return Ok(());
    }

    cache::clear(conn).map_err(|e| e.to_string())?;
    if records.len() == cmd.limit {
        cache::store_with_next(conn, &records)
    } else {
        cache::store(conn, &records)
    }
    .map_err(|e| e.to_string())?;

    display::print_bold("Records List:");
    display::print_items(&records, true, true);
    Ok(())
}

fn query_records(conn: &Connection, cmd: &ListRecordCommand) -> Result<Vec<Item>, String> {
    let mut record_query = ItemQuery::new().with_actions(vec!["record", "recurring_task_record"]);
    if let Some(cat) = &cmd.category {
        record_query = record_query.with_category(cat);
    }
    if let Some(search_term) = &cmd.search {
        record_query = record_query.with_content_like(search_term);
    }
    if let Some(days) = cmd.days {
        let cutoff_timestamp = timestr::days_before_to_unix_epoch(days);
        record_query = record_query.with_create_time_min(cutoff_timestamp);
    }
    if let Some(starting_time) = &cmd.starting_time {
        let starting_timestamp = timestr::to_unix_epoch(starting_time)?;
        record_query = record_query.with_create_time_min(starting_timestamp);
    }
    if let Some(ending_time) = &cmd.ending_time {
        let ending_timestamp = timestr::to_unix_epoch(ending_time)?;
        record_query = record_query.with_create_time_max(ending_timestamp);
    }

    let mut offset = Offset::None;
    if cmd.next_page {
        offset = handle_next_page(conn);
        match offset {
            Offset::CreateTime(_) => {}
            Offset::None => return Err("No next page available".to_string()),
            _ => return Err("next page not meant for this call".to_string()),
        }
    }
    record_query = record_query.with_offset(offset);
    record_query = record_query.with_limit(cmd.limit);
    record_query = record_query.with_order_by(CREATE_TIME_COL);
    query_items(conn, &record_query).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        get_test_conn,
        insert_record,
        insert_recurring_record,
        insert_recurring_task,
    };

    impl ListRecordCommand {
        fn default_test() -> Self {
            ListRecordCommand {
                category: None,
                days: None,
                limit: 100,
                starting_time: None,
                ending_time: None,
                next_page: false,
                search: None,
            }
        }

        fn with_category(mut self, category: &str) -> Self {
            self.category = Some(category.to_string());
            self
        }

        fn with_days(mut self, days: usize) -> Self {
            self.days = Some(days);
            self
        }

        fn with_starting_time(mut self, starting_time: &str) -> Self {
            self.starting_time = Some(starting_time.to_string());
            self
        }

        fn with_ending_time(mut self, ending_time: &str) -> Self {
            self.ending_time = Some(ending_time.to_string());
            self
        }

        fn with_search(mut self, search: &str) -> Self {
            self.search = Some(search.to_string());
            self
        }

        fn with_limit(mut self, limit: usize) -> Self {
            self.limit = limit;
            self
        }

        fn with_next_page(mut self) -> Self {
            self.next_page = true;
            self
        }
    }

    #[test]
    fn test_query_records() {
        let (conn, _temp_file) = get_test_conn();
        insert_record(&conn, "feeding", "100ML", "yesterday 2PM");
        insert_record(&conn, "feeding", "110ML", "yesterday 5PM");
        insert_record(&conn, "feeding", "100ML", "yesterday 9PM");
        insert_record(&conn, "FTP", "256W", "yesterday 7PM");

        let listfeeding = ListRecordCommand::default_test().with_category("feeding");
        let list_all = ListRecordCommand::default_test().with_days(2);
        let list_timeframe = ListRecordCommand::default_test()
            .with_starting_time("yesterday 4PM")
            .with_ending_time("yesterday 8PM");
        let list_timeframe_start_only =
            ListRecordCommand::default_test().with_starting_time("yesterday 8PM");

        let results = query_records(&conn, &listfeeding).unwrap();
        assert_eq!(results.len(), 3);
        let results = query_records(&conn, &list_all).unwrap();
        assert_eq!(results.len(), 4);
        let results = query_records(&conn, &list_timeframe).unwrap();
        assert_eq!(results.len(), 2);
        let results = query_records(&conn, &list_timeframe_start_only).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].category, "feeding")
    }

    #[test]
    fn test_query_records_with_recurring() {
        let (conn, _temp_file) = get_test_conn();

        // Insert regular records
        insert_record(&conn, "feeding", "100ML", "yesterday 2PM");
        insert_record(&conn, "feeding", "110ML", "yesterday 5PM");
        insert_record(&conn, "sleep", "2 hours", "yesterday 3PM");

        // Insert recurring task and recurring records
        let task_id = insert_recurring_task(&conn, "feeding", "Daily bottle", "Daily 9AM");
        insert_recurring_record(&conn, "feeding", "120ML bottle", task_id, 1000);
        insert_recurring_record(&conn, "feeding", "130ML bottle", task_id, 2000);
        insert_recurring_record(&conn, "sleep", "Nap time", task_id, 1500);

        // Query all records (should include both record and recurring_task_record)
        let list_all = ListRecordCommand::default_test().with_days(2);
        let results = query_records(&conn, &list_all).unwrap();
        assert_eq!(results.len(), 6); // 3 regular records + 3 recurring records

        // Verify we have both action types
        let regular_count = results.iter().filter(|r| r.action == "record").count();
        let recurring_count = results
            .iter()
            .filter(|r| r.action == "recurring_task_record")
            .count();
        assert_eq!(regular_count, 3);
        assert_eq!(recurring_count, 3);

        // Query with category filter (should work for both types)
        let list_feeding = ListRecordCommand::default_test()
            .with_days(2)
            .with_category("feeding");
        let results = query_records(&conn, &list_feeding).unwrap();
        assert_eq!(results.len(), 4); // 2 regular feeding + 2 recurring feeding
        for record in &results {
            assert_eq!(record.category, "feeding");
            assert!(record.action == "record" || record.action == "recurring_task_record");
        }

        // Query with search filter (should work for both types)
        let list_bottle = ListRecordCommand::default_test()
            .with_days(2)
            .with_search("bottle");
        let results = query_records(&conn, &list_bottle).unwrap();
        assert_eq!(results.len(), 2); // 2 recurring records with "bottle"
        for record in &results {
            assert!(record.content.contains("bottle"));
        }
    }

    #[test]
    fn test_query_records_pagination() {
        let (conn, _temp_file) = get_test_conn();
        for i in 1..=11 {
            insert_record(
                &conn,
                "test",
                &format!("B{}", i),
                &format!("2025/02/25 {}AM", i),
            );
            insert_record(
                &conn,
                "test",
                &format!("A{}", i),
                &format!("2025/02/23 {}PM", i),
            );
        }

        let list_record = ListRecordCommand::default_test()
            .with_category("test")
            .with_limit(11)
            .with_starting_time("2025/02/21")
            .with_ending_time("2025/02/27");

        let results = query_records(&conn, &list_record).unwrap();
        cache::clear(&conn).unwrap();
        cache::store_with_next(&conn, &results).unwrap();
        assert_eq!(results.len(), 11);
        assert!(results.iter().all(|i| i.content.contains("A")));

        let list_record_next = list_record.with_next_page();
        let results = query_records(&conn, &list_record_next).unwrap();
        cache::clear(&conn).unwrap();
        cache::store_with_next(&conn, &results).unwrap();
        assert_eq!(results.len(), 11);
        assert!(results.iter().all(|i| i.content.contains("B")));

        let results = query_records(&conn, &list_record_next).unwrap();
        cache::clear(&conn).unwrap();
        cache::store(&conn, &results).unwrap();
        assert_eq!(results.len(), 0);
    }
}
