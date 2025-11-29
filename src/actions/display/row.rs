use chrono::{
    Datelike,
    Local,
    TimeZone,
    Timelike,
    Weekday,
};

use crate::db::item::Item;

pub struct DisplayRow {
    pub index: String,
    pub category: String,
    pub content: String,
    pub timestr: String,
}

impl DisplayRow {
    pub fn from_task(index: String, task: &Item) -> Self {
        let mut category = task.category.clone();
        let content = task.content.clone();

        let mut timestr = if task.action == "recurring_task" {
            category.push_str(" (Recurring)");
            task.human_schedule
                .clone()
                .unwrap_or_else(|| "No schedule".to_string())
        } else {
            timestamp_to_display_string(task.target_time.unwrap(), false)
        };

        // Add status indicator for both types
        if task.status != 0 {
            let status_str = translate_status(task.status);
            timestr.push_str(&format!(" ({})", status_str));
        }

        DisplayRow {
            index,
            category,
            content,
            timestr,
        }
    }

    pub fn from_record(index: String, record: &Item) -> Self {
        let timestr = timestamp_to_display_string(record.create_time, true);
        let mut category = record.category.clone();
        let content = record.content.clone();
        if record.action == "recurring_task_record" {
            category.push_str(" (Recurring)");
        }

        DisplayRow {
            index,
            category,
            content,
            timestr,
        }
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
        6 => "pending".to_string(),
        _ => "unknown".to_string(),
    }
}
