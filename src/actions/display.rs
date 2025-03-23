use std::{
    cmp,
    env,
};

use chrono::{
    Datelike,
    Local,
    TimeZone,
    Timelike,
    Weekday,
};
use textwrap::{
    wrap,
    Options,
};

use crate::db::item::Item;

// For quick debug purposes
#[allow(dead_code)]
pub fn debug_print_items(header: &str, items: &[Item]) {
    println!("{}", header);
    for item in items {
        println!("  {:?}", item);
    }
}

pub fn print_items(items: &[Item], is_record: bool) {
    let mut results: Vec<DisplayRow> = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        if is_record {
            results.push(DisplayRow::from_record(index + 1, item));
        } else {
            results.push(DisplayRow::from_task(index + 1, item))
        }
    }
    print_table(&results, is_record);
}

pub fn print_bold(text: &str) {
    println!("\x1b[1m{}\x1b[0m", text);
}

fn print_table(rows: &[DisplayRow], is_record: bool) {
    // Get terminal width from environment variable
    let terminal_width = match env::var("COLUMNS") {
        Ok(columns) => columns.parse::<usize>().unwrap_or(120),
        Err(_) => 120, // Default to 120 if COLUMNS is not set
    };

    // Define column widths
    let index_width = 7;
    let category_width = 20;
    let timestr_width = 20;
    let margin = 10;

    // Calculate content width
    // Total used: column widths + 5 delimiters (|) + margin
    let content_width =
        terminal_width.saturating_sub(index_width + category_width + timestr_width + 5 + margin);

    let time_header = if is_record { "Created At" } else { "Deadline" };

    let separator_width = terminal_width - margin + 4;

    // Print table header
    println!("{:-<width$}", "", width = separator_width);
    println!(
        "| {:<index_width$}| {:<category_width$}| {:<content_width$}| {:<timestr_width$}|",
        "Index",
        "Category",
        "Content",
        time_header,
        index_width = index_width,
        category_width = category_width,
        content_width = content_width,
        timestr_width = timestr_width
    );
    println!("{:-<width$}", "", width = separator_width);

    let index_options = Options::new(index_width).break_words(true);
    let category_options = Options::new(category_width).break_words(true);
    let content_options = Options::new(content_width).break_words(false);
    let timestr_options = Options::new(timestr_width).break_words(false);

    for row in rows {
        let wrapped_index = wrap(&row.index, &index_options);
        let wrapped_category = wrap(&row.category, &category_options);
        let wrapped_content = wrap(&row.content, &content_options);
        let wrapped_timestr = wrap(&row.timestr, &timestr_options);

        // Find the maximum number of lines needed
        let max_lines = cmp::max(
            cmp::max(wrapped_index.len(), wrapped_category.len()),
            cmp::max(wrapped_content.len(), wrapped_timestr.len()),
        );

        for i in 0..max_lines {
            let index_line = if i < wrapped_index.len() {
                &wrapped_index[i]
            } else {
                ""
            };
            let category_line = if i < wrapped_category.len() {
                &wrapped_category[i]
            } else {
                ""
            };
            let content_line = if i < wrapped_content.len() {
                &wrapped_content[i]
            } else {
                ""
            };
            let timestr_line = if i < wrapped_timestr.len() {
                &wrapped_timestr[i]
            } else {
                ""
            };

            println!(
                "| {:<index_width$}| {:<category_width$}| {:<content_width$}| {:<timestr_width$}|",
                index_line,
                category_line,
                content_line,
                timestr_line,
                index_width = index_width,
                category_width = category_width,
                content_width = content_width,
                timestr_width = timestr_width
            );
        }

        // Print separator between rows
        println!("{:-<width$}", "", width = separator_width);
    }
}

fn timestamp_to_display_string(timestamp: i64, is_record: bool) -> String {
    let dt = match Local.timestamp_opt(timestamp, 0) {
        chrono::LocalResult::Single(dt) => dt,
        _ => return "Invalid timestamp".to_string(),
    };

    let now = Local::now();
    let is_end_of_day = dt.hour() == 23 && dt.minute() == 59 && dt.second() == 59;

    // If timestamp is today, use "Today" string.
    if dt.date_naive() == now.date_naive() {
        if is_end_of_day {
            return "Today".to_string();
        } else {
            let hour_format = format_hour(dt.hour(), dt.minute());
            return format!("Today {}", hour_format);
        }
    }

    // Skip all further rules if is record.
    if is_record {
        return format!(
            "{}/{}/{} {}",
            dt.year(),
            dt.month(),
            dt.day(),
            format_hour(dt.hour(), dt.minute())
        );
    }

    // If timestamp is in the past, just put date
    // Task list don't contain past task by default, unless prompted.
    if dt.date_naive() < now.date_naive() {
        return format!("{}/{:02}/{:02}", dt.year(), dt.month(), dt.day());
    }

    // If timestamp is in the future but not current year
    if dt.year() != now.year() {
        return format!("{}/{:02}/{:02}", dt.year(), dt.month(), dt.day());
    }

    // If timestamp is tomorrow
    let tomorrow = now.date_naive() + chrono::Duration::days(1);
    if dt.date_naive() == tomorrow {
        if is_end_of_day {
            return "Tomorrow".to_string();
        } else {
            let hour_format = format_hour(dt.hour(), dt.minute());
            return format!("Tomorrow {}", hour_format);
        }
    }

    // If timestamp is within next 7 days
    if dt.date_naive() <= now.date_naive() + chrono::Duration::days(7) {
        let weekday = match dt.weekday() {
            Weekday::Mon => "Monday",
            Weekday::Tue => "Tuesday",
            Weekday::Wed => "Wednesday",
            Weekday::Thu => "Thursday",
            Weekday::Fri => "Friday",
            Weekday::Sat => "Saturday",
            Weekday::Sun => "Sunday",
        };

        // Check if it's next week (different week number)
        let dt_week = dt.iso_week().week();
        let now_week = now.iso_week().week();
        let prefix = if dt_week != now_week { "Next " } else { "" };

        if is_end_of_day {
            return format!("{}{}", prefix, weekday);
        } else {
            let hour_format = format_hour(dt.hour(), dt.minute());
            return format!("{}{} {}", prefix, weekday, hour_format);
        }
    }

    // If timestamp is within the year
    if is_end_of_day {
        format!("{}/{}", dt.month(), dt.day())
    } else {
        let hour_format = format_hour(dt.hour(), dt.minute());
        format!("{}/{} {}", dt.month(), dt.day(), hour_format)
    }
}

fn format_hour(hour: u32, minute: u32) -> String {
    let hour12 = if hour == 0 {
        12
    } else if hour > 12 {
        hour - 12
    } else {
        hour
    };
    let period = if hour < 12 { "AM" } else { "PM" };
    format!("{}:{:02}{}", hour12, minute, period)
}

fn translate_status(status: u8) -> String {
    match status {
        0 => "ongoing".to_string(),
        1 => "completed".to_string(),
        2 => "cancelled".to_string(),
        3 => "duplicate".to_string(),
        4 => "suspended".to_string(),
        5 => "removed".to_string(),
        _ => "unknown".to_string(),
    }
}

pub struct DisplayRow {
    pub index: String,
    pub category: String,
    pub content: String,
    pub timestr: String,
}

impl DisplayRow {
    pub fn from_task(index: usize, task: &Item) -> Self {
        let mut target_timestr = timestamp_to_display_string(task.target_time.unwrap(), false);
        let category = task.category.clone();
        let content = task.content.clone();
        if task.status != 0 {
            let status_str = translate_status(task.status);
            target_timestr.push_str(&format!(" ({})", status_str));
        }
        DisplayRow {
            index: format!("{}", index),
            category,
            content,
            timestr: target_timestr,
        }
    }

    pub fn from_record(index: usize, record: &Item) -> Self {
        let create_timestr = timestamp_to_display_string(record.create_time, true);
        let category = record.category.clone();
        let content = record.content.clone();
        DisplayRow {
            index: format!("{}", index),
            category,
            content,
            timestr: create_timestr,
        }
    }
}
