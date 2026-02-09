use rusqlite::Connection;

use crate::{
    actions::display,
    args::{
        parser::OpsStatCommand,
        timestr,
    },
    db::ops::get_stats,
};

pub fn handle_statcmd(conn: &Connection, cmd: &OpsStatCommand) -> Result<(), String> {
    let stats = query_stats(conn, cmd)?;

    if stats.rows.is_empty() {
        display::print_bold("No statistics found");
        return Ok(());
    }

    display::print_bold("Statistics:");
    print_stats_table(&stats);
    Ok(())
}

fn query_stats(
    conn: &Connection,
    cmd: &OpsStatCommand,
) -> Result<crate::db::ops::StatTable, String> {
    // Parse time filters
    let create_time_min = if let Some(ref starting_time) = cmd.starting_time {
        Some(timestr::to_unix_epoch(starting_time)?)
    } else {
        None
    };

    let create_time_max = if let Some(ref ending_time) = cmd.ending_time {
        Some(timestr::to_unix_epoch(ending_time)?)
    } else {
        None
    };

    get_stats(
        conn,
        cmd.category.as_deref(),
        create_time_min,
        create_time_max,
        None,
        None,
    )
    .map_err(|e| e.to_string())
}

fn print_stats_table(stats: &crate::db::ops::StatTable) {
    // Define column widths
    let category_width = 20;
    let number_width = 12;
    // Separator width: category + 5 number columns + delimiters (6 "| " + 1 final "|" = 13 chars)
    let separator_width = category_width + number_width * 5 + 13;

    // Print header
    println!(
        "{:-<width$}",
        "",
        width = separator_width
    );
    println!(
        "| {:<cat_w$}| {:<num_w$}| {:<num_w$}| {:<num_w$}| {:<num_w$}| {:<num_w$}|",
        "Category",
        "Task",
        "Record",
        "Recur Task",
        "Recur Record",
        "Total",
        cat_w = category_width,
        num_w = number_width
    );
    println!(
        "{:-<width$}",
        "",
        width = separator_width
    );

    // Print data rows
    for row in &stats.rows {
        println!(
            "| {:<cat_w$}| {:<num_w$}| {:<num_w$}| {:<num_w$}| {:<num_w$}| {:<num_w$}|",
            truncate_string(&row.category, category_width),
            row.task,
            row.record,
            row.recurring_task,
            row.recurring_task_record,
            row.total,
            cat_w = category_width,
            num_w = number_width
        );
    }

    // Print separator before totals
    println!(
        "{:-<width$}",
        "",
        width = separator_width
    );

    // Print totals row
    println!(
        "| {:<cat_w$}| {:<num_w$}| {:<num_w$}| {:<num_w$}| {:<num_w$}| {:<num_w$}|",
        "TOTAL",
        stats.totals.task,
        stats.totals.record,
        stats.totals.recurring_task,
        stats.totals.recurring_task_record,
        stats.totals.total,
        cat_w = category_width,
        num_w = number_width
    );
    println!(
        "{:-<width$}",
        "",
        width = separator_width
    );
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        get_test_conn,
        insert_record,
        insert_recurring_record,
        insert_recurring_task,
        insert_task,
    };

    impl OpsStatCommand {
        fn default_test() -> Self {
            OpsStatCommand {
                category: None,
                starting_time: None,
                ending_time: None,
            }
        }

        fn with_category(mut self, category: &str) -> Self {
            self.category = Some(category.to_string());
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
    }

    #[test]
    fn test_query_stats_all() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "Work", "Task 1", "tomorrow");
        insert_task(&conn, "Work", "Task 2", "tomorrow");
        insert_task(&conn, "Personal", "Task 3", "tomorrow");
        insert_record(&conn, "Work", "Record 1", "yesterday");
        insert_record(&conn, "Personal", "Record 2", "yesterday");
        insert_record(&conn, "Personal", "Record 3", "yesterday");

        let cmd = OpsStatCommand::default_test();
        let stats = query_stats(&conn, &cmd).unwrap();

        assert_eq!(stats.rows.len(), 2);
        assert_eq!(stats.totals.task, 3);
        assert_eq!(stats.totals.record, 3);
        assert_eq!(stats.totals.total, 6);
    }

    #[test]
    fn test_query_stats_with_recurring() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "Work", "Task 1", "tomorrow");
        let recurring_id = insert_recurring_task(&conn, "Work", "Recurring Task", "Daily 9AM");
        insert_recurring_record(&conn, "Work", "Recurring Record", recurring_id, 1000);
        insert_record(&conn, "Work", "Record 1", "yesterday");

        let cmd = OpsStatCommand::default_test();
        let stats = query_stats(&conn, &cmd).unwrap();

        assert_eq!(stats.rows.len(), 1);
        assert_eq!(stats.totals.task, 1);
        assert_eq!(stats.totals.record, 1);
        assert_eq!(stats.totals.recurring_task, 1);
        assert_eq!(stats.totals.recurring_task_record, 1);
        assert_eq!(stats.totals.total, 4);
    }

    #[test]
    fn test_query_stats_by_category() {
        let (conn, _temp_file) = get_test_conn();
        insert_task(&conn, "Work", "Task 1", "tomorrow");
        insert_task(&conn, "Work", "Task 2", "tomorrow");
        insert_task(&conn, "Personal", "Task 3", "tomorrow");
        insert_record(&conn, "Work", "Record 1", "yesterday");

        let cmd = OpsStatCommand::default_test().with_category("Work");
        let stats = query_stats(&conn, &cmd).unwrap();

        assert_eq!(stats.rows.len(), 1);
        assert_eq!(stats.rows[0].category, "Work");
        assert_eq!(stats.totals.task, 2);
        assert_eq!(stats.totals.record, 1);
        assert_eq!(stats.totals.total, 3);
    }

    #[test]
    fn test_query_stats_with_time_filter() {
        let (conn, _temp_file) = get_test_conn();
        // Records are created with specific create_time
        insert_record(&conn, "Work", "Record 1", "2025/02/23 10AM");
        insert_record(&conn, "Work", "Record 2", "2025/02/25 10AM");
        insert_record(&conn, "Work", "Record 3", "2025/02/26 10AM");
        insert_record(&conn, "Work", "Record 4", "2025/02/27 10AM");

        let cmd = OpsStatCommand::default_test()
            .with_starting_time("2025/02/24")
            .with_ending_time("2025/02/26");
        let stats = query_stats(&conn, &cmd).unwrap();

        // Should only include Record 2 and Record 3 (created between 2025/02/24 and 2025/02/26)
        // Record 1 is before the range, Record 4 is after
        assert_eq!(stats.rows.len(), 1);
        assert_eq!(stats.totals.record, 2);
        assert_eq!(stats.totals.total, 2);
    }
}
