use chrono::{
    Datelike,
    Local,
    TimeZone,
    Timelike,
    Utc,
    Weekday,
};

use crate::db::item::Item;

pub enum TimeFormat {
    Standard,    // 12-hour AM/PM local time (default)
    TwentyFourH, // 24-hour local time, relative labels (Today, Tomorrow, etc.)
    Military,    // absolute YYYY/MM/DD HHMM, no relative labels
    Utc,         // 24-hour UTC, relative labels
}

impl TimeFormat {
    pub fn from_str(s: &str) -> Self {
        match s {
            "24h" => TimeFormat::TwentyFourH,
            "military" => TimeFormat::Military,
            "utc" => TimeFormat::Utc,
            _ => TimeFormat::Standard, // "standard" or anything unrecognized
        }
    }
}

pub struct DisplayConfig {
    pub time_format: TimeFormat,
}

pub struct DisplayRow {
    pub index: String,
    pub category: String,
    pub content: String,
    pub timestr: String,
}

impl DisplayRow {
    pub fn from_task(index: String, task: &Item, dcfg: &DisplayConfig) -> Self {
        let mut category = task.category.clone();
        let content = task.content.clone();

        let mut timestr = if task.action == "recurring_task" {
            category.push_str(" (Recurring)");
            task.human_schedule
                .clone()
                .unwrap_or_else(|| "No schedule".to_string())
        } else {
            timestamp_to_display_string(task.target_time.unwrap(), false, dcfg)
        };

        if task.status != 0 {
            let status_str = translate_status(task.status);
            timestr.push_str(&format!(" ({})", status_str));
        } else if task.recurring_interval_complete {
            timestr.push_str(" (fulfilled)");
        }

        DisplayRow {
            index,
            category,
            content,
            timestr,
        }
    }

    pub fn from_record(index: String, record: &Item, dcfg: &DisplayConfig) -> Self {
        let timestr = timestamp_to_display_string(record.create_time, true, dcfg);
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

fn timestamp_to_display_string(timestamp: i64, is_record: bool, dcfg: &DisplayConfig) -> String {
    if matches!(dcfg.time_format, TimeFormat::Military) {
        let dt = match Local.timestamp_opt(timestamp, 0) {
            chrono::LocalResult::Single(dt) => dt,
            _ => return "Invalid timestamp".to_string(),
        };
        return format!("{}/{:02}/{:02} {:02}{:02}", dt.year(), dt.month(), dt.day(), dt.hour(), dt.minute());
    }

    let is_24h = matches!(dcfg.time_format, TimeFormat::TwentyFourH | TimeFormat::Utc);
    let utc_suffix = if matches!(dcfg.time_format, TimeFormat::Utc) { " UTC" } else { "" };

    let (dt_naive, now_naive, dt_hour, dt_minute, dt_second, dt_year, dt_month, dt_day, dt_weekday, dt_week) =
        if matches!(dcfg.time_format, TimeFormat::Utc) {
            let dt = match Utc.timestamp_opt(timestamp, 0) {
                chrono::LocalResult::Single(dt) => dt,
                _ => return "Invalid timestamp".to_string(),
            };
            let now = Utc::now();
            (dt.date_naive(), now.date_naive(), dt.hour(), dt.minute(), dt.second(),
             dt.year(), dt.month(), dt.day(), dt.weekday(), dt.iso_week().week())
        } else {
            let dt = match Local.timestamp_opt(timestamp, 0) {
                chrono::LocalResult::Single(dt) => dt,
                _ => return "Invalid timestamp".to_string(),
            };
            let now = Local::now();
            (dt.date_naive(), now.date_naive(), dt.hour(), dt.minute(), dt.second(),
             dt.year(), dt.month(), dt.day(), dt.weekday(), dt.iso_week().week())
        };

    let is_end_of_day = dt_hour == 23 && dt_minute == 59 && dt_second == 59;

    // If timestamp is today, use "Today" string.
    if dt_naive == now_naive {
        if is_end_of_day {
            return format!("Today{}", utc_suffix);
        } else {
            let hour_format = format_hour(dt_hour, dt_minute, is_24h);
            return format!("Today {}{}", hour_format, utc_suffix);
        }
    }

    // Skip all further rules if is record.
    if is_record {
        return format!(
            "{}/{}/{} {}{}",
            dt_year,
            dt_month,
            dt_day,
            format_hour(dt_hour, dt_minute, is_24h),
            utc_suffix,
        );
    }

    // If timestamp is in the past, just put date
    // Task list don't contain past task by default, unless prompted.
    if dt_naive < now_naive {
        return format!("{}/{:02}/{:02}{}", dt_year, dt_month, dt_day, utc_suffix);
    }

    // If timestamp is in the future but not current year
    if dt_year != now_naive.year() {
        return format!("{}/{:02}/{:02}{}", dt_year, dt_month, dt_day, utc_suffix);
    }

    // If timestamp is tomorrow
    let tomorrow = now_naive + chrono::Duration::days(1);
    if dt_naive == tomorrow {
        if is_end_of_day {
            return format!("Tomorrow{}", utc_suffix);
        } else {
            let hour_format = format_hour(dt_hour, dt_minute, is_24h);
            return format!("Tomorrow {}{}", hour_format, utc_suffix);
        }
    }

    // If timestamp is within next 7 days
    if dt_naive <= now_naive + chrono::Duration::days(7) {
        let weekday = match dt_weekday {
            Weekday::Mon => "Monday",
            Weekday::Tue => "Tuesday",
            Weekday::Wed => "Wednesday",
            Weekday::Thu => "Thursday",
            Weekday::Fri => "Friday",
            Weekday::Sat => "Saturday",
            Weekday::Sun => "Sunday",
        };

        let prefix = if dt_week != now_naive.iso_week().week() { "Next " } else { "" };

        if is_end_of_day {
            return format!("{}{}{}", prefix, weekday, utc_suffix);
        } else {
            let hour_format = format_hour(dt_hour, dt_minute, is_24h);
            return format!("{}{} {}{}", prefix, weekday, hour_format, utc_suffix);
        }
    }

    // If timestamp is within the year
    if is_end_of_day {
        format!("{}/{}{}", dt_month, dt_day, utc_suffix)
    } else {
        let hour_format = format_hour(dt_hour, dt_minute, is_24h);
        format!("{}/{} {}{}", dt_month, dt_day, hour_format, utc_suffix)
    }
}

fn format_hour(hour: u32, minute: u32, is_24h: bool) -> String {
    if is_24h {
        return format!("{:02}:{:02}", hour, minute);
    }
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
