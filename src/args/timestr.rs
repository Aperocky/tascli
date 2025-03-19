use chrono::{
    Datelike,
    Duration,
    Local,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    TimeZone,
    Weekday,
};

pub fn to_unix_epoch(s: &str) -> Result<i64, String> {
    let dt = parse_flexible_timestr(s)?;
    Local
        .from_local_datetime(&dt)
        .earliest()
        .ok_or_else(|| String::from("cannot parse timestr into unix epoch"))
        .map(|dt| dt.timestamp())
}

pub fn parse_flexible_timestr(s: &str) -> Result<NaiveDateTime, String> {
    let s = s.trim();
    let now = Local::now().naive_local();

    // Default time when only date is specified (end of day)
    let default_time = NaiveTime::from_hms_opt(23, 59, 59).unwrap();

    // Default date when only time is specified (today)
    let default_date = now.date();

    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() > 2 {
        return Err(format!("There are too many parts in timestr {}", s));
    }

    // If we have multiple parts, try to parse the first part as a date and the rest as a time
    if parts.len() > 1 {
        let potential_date = parts[0];
        let potential_time = parts[1..].join(" ");
        let date_result = parse_date_portion(potential_date, default_date);
        let time_result = parse_time_portion(&potential_time);
        if let (Ok(date), Ok(time)) = (date_result, time_result) {
            return Ok(date.and_time(time));
        }
    }

    // Try to parse the whole string as just a date
    if let Ok(date) = parse_date_portion(s, default_date) {
        return Ok(date.and_time(default_time));
    }

    // Try to parse the whole string as just a time
    if let Ok(time) = parse_time_portion(s) {
        return Ok(default_date.and_time(time));
    }

    Err(format!("Couldn't parse '{}' as a valid date/time", s))
}

fn parse_date_portion(s: &str, today: NaiveDate) -> Result<NaiveDate, String> {
    // Try keywords first
    match s.to_lowercase().as_str() {
        "today" | "eod" => return Ok(today),
        "tomorrow" => return Ok(today + Duration::days(1)),
        "monday" => return Ok(next_weekday(today, Weekday::Mon)),
        "tuesday" => return Ok(next_weekday(today, Weekday::Tue)),
        "wednesday" => return Ok(next_weekday(today, Weekday::Wed)),
        "thursday" => return Ok(next_weekday(today, Weekday::Thu)),
        "friday" => return Ok(next_weekday(today, Weekday::Fri)),
        "saturday" => return Ok(next_weekday(today, Weekday::Sat)),
        "sunday" | "eow" | "week" => return Ok(next_weekday(today, Weekday::Sun)),
        _ => {}
    }

    let full_date_formats = [
        "%Y/%m/%d", // 2025/06/12
        "%Y-%m-%d", // 2025-06-12
        "%m/%d/%Y", // 06/12/2025
        "%m-%d-%Y", // 06-12-2025
    ];

    for format in &full_date_formats {
        if let Ok(mut date) = NaiveDate::parse_from_str(s, format) {
            // For month/day formats without year, use current year
            if *format == "%m/%d" || *format == "%m-%d" {
                date =
                    NaiveDate::from_ymd_opt(today.year(), date.month(), date.day()).unwrap_or(date);
            }
            return Ok(date);
        }
    }

    // Also accept month/date shorthand like 3/24
    let md_regex = regex::Regex::new(r"^(\d{1,2})/(\d{1,2})$").unwrap();
    if let Some(caps) = md_regex.captures(s) {
        let month: u32 = caps[1].parse().unwrap();
        let day: u32 = caps[2].parse().unwrap();
        if let Some(date) = NaiveDate::from_ymd_opt(today.year(), month, day) {
            return Ok(date);
        }
    }

    Err(format!("Couldn't parse '{}' as a date", s))
}

fn parse_time_portion(s: &str) -> Result<NaiveTime, String> {
    // Try common time formats
    let time_formats = [
        "%H:%M",    // 21:06
        "%H:%M:%S", // 21:06:30
        "%I:%M%p",  // 3:00PM
    ];

    for format in &time_formats {
        if let Ok(time) = NaiveTime::parse_from_str(s, format) {
            return Ok(time);
        }
    }

    // Check for hour + AM/PM pattern without minutes (e.g., "3PM")
    let hour_ampm_regex = regex::Regex::new(r"^(\d{1,2})(AM|PM|am|pm)$").unwrap();
    if let Some(caps) = hour_ampm_regex.captures(s) {
        let with_minutes = format!("{}:00{}", &caps[1], &caps[2]);
        if let Ok(time) = NaiveTime::parse_from_str(&with_minutes, "%I:%M%p") {
            return Ok(time);
        }
    }

    Err(format!("Couldn't parse '{}' as a time", s))
}

fn next_weekday(from_date: NaiveDate, weekday: Weekday) -> NaiveDate {
    let days_from_today =
        weekday.num_days_from_monday() as i64 - from_date.weekday().num_days_from_monday() as i64;

    // If the target day is today or earlier in the week, go to next week
    if days_from_today <= 0 {
        from_date + Duration::days(days_from_today + 7)
    } else {
        from_date + Duration::days(days_from_today)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_valid_inputs() {
        // Collection of inputs that should be successfully parsed
        let valid_inputs = [
            "2025-10-15",
            "14:30",
            "3PM",
            "2025-10-15 14:30",
            "  2025-10-15  ",
            "today",
            "eod",
            "tomorrow 5PM",
            "tomorrow",
            "monday",
            "friday",
            "friday 3PM",
        ];

        for input in valid_inputs {
            let result = parse_flexible_timestr(input);
            assert!(
                result.is_ok(),
                "Should parse valid input '{}' but got error: {:?}",
                input,
                result.err()
            );
            let unix_epoch = to_unix_epoch(input);
            assert!(
                unix_epoch.is_ok(),
                "Should parse valid input '{}' but got error: {:?}",
                input,
                unix_epoch.err()
            );
        }
    }

    #[test]
    fn test_invalid_inputs() {
        // Collection of inputs that should fail to parse
        let invalid_inputs = [
            "",
            "not a date or time",
            "2025-10-15 14:30 0300TZ",
            "invalid-date 12:30",
            "2025-13-45",
            "25:70",
            "20PM",
            "13AM",
            "monday 0AM",
        ];

        for input in invalid_inputs {
            let result = parse_flexible_timestr(input);
            assert!(
                result.is_err(),
                "Expected error for invalid input '{}' but got success",
                input
            );
        }
    }

    #[test]
    fn test_unix_epoch() {
        let btime = "2025-02-23 20:35:00";
        let local_dt = Local.datetime_from_str(btime, "%Y-%m-%d %H:%M:%S").unwrap();
        let utc_dt = local_dt.with_timezone(&Utc);
        let expected_timestamp = utc_dt.timestamp();

        let unix_epoch = to_unix_epoch(btime).unwrap();
        assert_eq!(
            unix_epoch,
            expected_timestamp,
            "to_unix_epoch should use local timezone in conversion. \
             Expected timestamp: {} (using local timezone: {}), \
             but got: {}",
            expected_timestamp,
            Local::now().offset(),
            unix_epoch
        );
    }
}
