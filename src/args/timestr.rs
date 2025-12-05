use chrono::{
    Datelike,
    Duration,
    Local,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    TimeZone,
    Timelike,
    Weekday,
};

pub fn days_before_to_unix_epoch(d: usize) -> i64 {
    let now = Local::now();
    let past_date = now - Duration::days(d as i64);
    past_date.timestamp()
}

pub fn days_after_to_unix_epoch(d: usize) -> i64 {
    let now = Local::now();
    let future_date = now + Duration::days(d as i64);
    future_date.timestamp()
}

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
    match s.to_lowercase().as_str() {
        "today" | "eod" => return Ok(today),
        "yesterday" => return Ok(today - Duration::days(1)),
        "tomorrow" => return Ok(today + Duration::days(1)),
        "monday" => return Ok(next_weekday(today, Weekday::Mon)),
        "tuesday" => return Ok(next_weekday(today, Weekday::Tue)),
        "wednesday" => return Ok(next_weekday(today, Weekday::Wed)),
        "thursday" => return Ok(next_weekday(today, Weekday::Thu)),
        "friday" => return Ok(next_weekday(today, Weekday::Fri)),
        "saturday" => return Ok(next_weekday(today, Weekday::Sat)),
        "sunday" | "eow" | "week" => return Ok(next_weekday(today, Weekday::Sun)),
        "year" | "eoy" => return Ok(today.with_month(12).unwrap().with_day(31).unwrap()),
        "month" | "eom" => return Ok(last_day_of_month(today)),
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
    if s.contains('/') {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(month), Ok(day)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if let Some(date) = NaiveDate::from_ymd_opt(today.year(), month, day) {
                    return Ok(date);
                }
            }
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
    let s_lower = s.to_lowercase();
    if s_lower.ends_with("am") || s_lower.ends_with("pm") {
        let ampm = &s_lower[s_lower.len() - 2..];
        let hour_str = &s[..s.len() - 2];
        if let Ok(hour) = hour_str.parse::<u8>() {
            let with_minutes = format!("{}:00{}", hour, ampm);
            if let Ok(time) = NaiveTime::parse_from_str(&with_minutes, "%I:%M%p") {
                return Ok(time);
            }
        }
    }

    Err(format!("Couldn't parse '{}' as a time", s))
}

fn last_day_of_month(date: NaiveDate) -> NaiveDate {
    let next_month = if date.month() == 12 {
        date.with_year(date.year() + 1)
            .unwrap()
            .with_month(1)
            .unwrap()
    } else {
        date.with_month(date.month() + 1).unwrap()
    };
    let first_of_next = next_month.with_day(1).unwrap();
    first_of_next - Duration::days(1)
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

// Helper: Get time from parts starting at index, or default to 11:59PM
fn get_time_or_default(parts: &[&str], start_idx: usize) -> Result<String, String> {
    if parts.len() > start_idx {
        let time_str = parts[start_idx..].join(" ");
        let time = parse_time_portion(&time_str)
            .map_err(|_| format!("Couldn't parse '{}' as a time for cron", time_str))?;
        Ok(format!("{} {}", time.minute(), time.hour()))
    } else {
        Ok("59 23".to_string())
    }
}

// Parse a human readable schedule into cron.
pub fn parse_recurring_timestr(s: &str) -> Result<String, String> {
    let s = s.trim();
    let parts: Vec<&str> = s.split_whitespace().collect();

    if parts.is_empty() {
        return Err(String::from("Empty recurring time string"));
    }

    match parts[0].to_lowercase().as_str() {
        "daily" => {
            let time = get_time_or_default(&parts, 1)?;
            Ok(format!("{} * * *", time))
        }
        "weekly" => {
            if parts.len() == 1 {
                return Ok(String::from("59 23 * * 0"));
            }

            // Check for day range like "Monday-Friday"
            let days = if parts[1].contains('-') {
                parse_day_range(parts[1])?
            } else {
                parse_weekday(parts[1])?.to_string()
            };

            let time = get_time_or_default(&parts, 2)?;
            Ok(format!("{} * * {}", time, days))
        }
        "monthly" => {
            if parts.len() == 1 {
                return Ok(String::from("59 23 1 * *"));
            }

            let day = parse_ordinal_day(parts[1])
                .ok_or_else(|| format!("Invalid day format in '{}'", s))?;
            let time = get_time_or_default(&parts, 2)?;
            Ok(format!("{} {} * *", time, day))
        }
        "yearly" => {
            if parts.len() == 1 {
                return Ok(String::from("59 23 31 12 *"));
            }

            let (month, day) = parse_month_day(parts[1])?;
            let time = get_time_or_default(&parts, 2)?;
            Ok(format!("{} {} {} *", time, day, month))
        }
        "every" => {
            if parts.len() == 1 {
                return Err(String::from("'Every' requires additional specification"));
            }

            // Check if it's a time pattern (e.g., "Every 9PM")
            if parse_time_portion(parts[1]).is_ok() {
                let time = get_time_or_default(&parts, 1)?;
                return Ok(format!("{} * * *", time));
            }

            // Check if it's "Day"
            if parts[1].to_lowercase() == "day" {
                let time = get_time_or_default(&parts, 2)?;
                return Ok(format!("{} * * *", time));
            }

            // Check if it's a weekday
            if let Ok(weekday) = parse_weekday(parts[1]) {
                let time = get_time_or_default(&parts, 2)?;
                return Ok(format!("{} * * {}", time, weekday));
            }

            // Check if it's a month/day pattern (e.g., "Every 6/12")
            if parts[1].contains('/') {
                let (month, day) = parse_month_day(parts[1])?;
                let time = get_time_or_default(&parts, 2)?;
                return Ok(format!("{} {} {} *", time, day, month));
            }

            // Check if it's a monthly pattern: "Every <ordinal> of [the] Month [time]"
            if let Some(day) = parse_ordinal_day(parts[1]) {
                // Check for "of the Month" pattern
                if parts.len() >= 5
                    && parts[2].to_lowercase() == "of"
                    && parts[3].to_lowercase() == "the"
                    && parts[4].to_lowercase() == "month"
                {
                    let time = get_time_or_default(&parts, 5)?;
                    return Ok(format!("{} {} * *", time, day));
                }

                // Check for "of Month" pattern
                if parts.len() >= 4
                    && parts[2].to_lowercase() == "of"
                    && parts[3].to_lowercase() == "month"
                {
                    let time = get_time_or_default(&parts, 4)?;
                    return Ok(format!("{} {} * *", time, day));
                }

                // If we have an ordinal but no valid month pattern, it's not a monthly pattern
                // Fall through to check other patterns or return error
            }

            Err(format!("Unrecognized pattern after 'Every' in '{}'", s))
        }
        _ => Err(format!("Unrecognized recurring time format: '{}'", s)),
    }
}

// Parse weekday names to cron weekday numbers (0=Sunday, 1=Monday, etc.)
fn parse_weekday(s: &str) -> Result<u8, String> {
    match s.to_lowercase().as_str() {
        "sunday" | "sun" => Ok(0),
        "monday" | "mon" => Ok(1),
        "tuesday" | "tue" | "tues" => Ok(2),
        "wednesday" | "wed" | "weds" => Ok(3),
        "thursday" | "thu" | "thur" | "thurs" => Ok(4),
        "friday" | "fri" => Ok(5),
        "saturday" | "sat" => Ok(6),
        _ => Err(format!("Invalid weekday: '{}'", s)),
    }
}

// Parse day ranges like "Monday-Friday" into cron format "1-5"
fn parse_day_range(s: &str) -> Result<String, String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid day range format: '{}'", s));
    }

    let start_day = parse_weekday(parts[0])?;
    let mut end_day = parse_weekday(parts[1])?;

    // Handle Saturday-Sunday case: convert Sunday from 0 to 7 for valid range
    if start_day > end_day && end_day == 0 {
        end_day = 7;
    }

    if start_day > end_day {
        return Err(format!(
            "Day range must go forward (e.g., Mon-Fri), not wrap around: '{}'",
            s
        ));
    }

    Ok(format!("{}-{}", start_day, end_day))
}

// Parse ordinal day expressions like "3rd", "15th", etc.
fn parse_ordinal_day(s: &str) -> Option<u8> {
    let s = s.to_lowercase();

    // Handle ordinal suffixes
    let day_str =
        if s.ends_with("st") || s.ends_with("nd") || s.ends_with("rd") || s.ends_with("th") {
            &s[0..s.len() - 2]
        } else {
            &s
        };

    day_str.parse::<u8>().ok().filter(|&d| d >= 1 && d <= 31)
}

// Parse month/day patterns like "2/14"
fn parse_month_day(s: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = s.split('/').collect();

    let (month, day) = match parts.as_slice() {
        [m, d] => {
            let month = m
                .parse::<u32>()
                .map_err(|_| format!("Invalid month in '{}'", s))?;
            let day = d
                .parse::<u32>()
                .map_err(|_| format!("Invalid day in '{}'", s))?;
            (month, day)
        }
        _ => return Err(format!("Invalid month/day format: '{}'", s)),
    };

    // Use NaiveDate to validate the date is real (e.g., rejects Feb 30)
    NaiveDate::from_ymd_opt(2024, month, day)
        .ok_or_else(|| format!("Invalid date: {}/{}", month, day))?;

    Ok((month, day))
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
            "3/24",
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
        let naive_dt = NaiveDateTime::parse_from_str(btime, "%Y-%m-%d %H:%M:%S").unwrap();
        let local_dt = naive_dt.and_local_timezone(Local).unwrap();
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

    #[test]
    fn test_recurring_valid_inputs() {
        let test_cases = [
            // Daily
            ("Daily", "59 23 * * *"),
            ("Daily 5PM", "0 17 * * *"),
            ("Daily 9:30AM", "30 9 * * *"),
            // Weekly
            ("Weekly", "59 23 * * 0"),
            ("Weekly Monday", "59 23 * * 1"),
            ("Weekly Monday 5PM", "0 17 * * 1"),
            ("Weekly Monday-Friday", "59 23 * * 1-5"),
            ("Weekly Monday-Friday 3PM", "0 15 * * 1-5"),
            ("Weekly Saturday-Sunday", "59 23 * * 6-7"),
            ("Weekly Sat-Sun 10AM", "0 10 * * 6-7"),
            // Monthly
            ("Monthly", "59 23 1 * *"),
            ("Monthly 3rd", "59 23 3 * *"),
            ("Monthly 15th 9AM", "0 9 15 * *"),
            // Yearly
            ("Yearly", "59 23 31 12 *"),
            ("Yearly 2/14", "59 23 14 2 *"),
            ("Yearly 7/4 12PM", "0 12 4 7 *"),
            ("Yearly 12/25", "59 23 25 12 *"),
            // Every - time patterns (maps to Daily)
            ("Every 9PM", "0 21 * * *"),
            ("Every 9:30AM", "30 9 * * *"),
            ("Every Day", "59 23 * * *"),
            ("Every Day 5PM", "0 17 * * *"),
            // Every - weekday patterns (maps to Weekly)
            ("Every Monday", "59 23 * * 1"),
            ("Every Monday 5PM", "0 17 * * 1"),
            ("Every Friday 3PM", "0 15 * * 5"),
            // Every - ordinal day patterns (maps to Monthly)
            ("Every 9th of the Month", "59 23 9 * *"),
            ("Every 9th of Month", "59 23 9 * *"),
            ("Every 15th of the Month 9AM", "0 9 15 * *"),
            ("Every 1st of the Month", "59 23 1 * *"),
            ("Every 1st of Month", "59 23 1 * *"),
            ("Every 1st of the Month 10AM", "0 10 1 * *"),
            // Every - month/day patterns (maps to Yearly)
            ("Every 6/12", "59 23 12 6 *"),
            ("Every 2/14 5PM", "0 17 14 2 *"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                parse_recurring_timestr(input).unwrap(),
                expected,
                "Failed for input: '{}'",
                input
            );
        }
    }

    #[test]
    fn test_recurring_invalid_inputs() {
        assert!(parse_recurring_timestr("").is_err());
        assert!(parse_recurring_timestr("Invalid").is_err());
        assert!(parse_recurring_timestr("Monthly 32nd").is_err());
        assert!(parse_recurring_timestr("Weekly InvalidDay").is_err());
        assert!(parse_recurring_timestr("13/45").is_err());

        // Invalid dates should be rejected (Feb 30, June 31, etc.)
        assert!(parse_recurring_timestr("Yearly 2/30").is_err());
        assert!(parse_recurring_timestr("Yearly 6/31").is_err());

        // Standalone month/day patterns should be rejected (conflict with one-time tasks)
        assert!(parse_recurring_timestr("2/14").is_err());
        assert!(parse_recurring_timestr("12/25").is_err());
        assert!(parse_recurring_timestr("February 14th").is_err());
        assert!(parse_recurring_timestr("July 4th").is_err());

        // Invalid Every patterns
        assert!(parse_recurring_timestr("Every").is_err());
        assert!(parse_recurring_timestr("Every 1st").is_err()); // Must use "of the Month"
        assert!(parse_recurring_timestr("Every 9th").is_err()); // Must use "of the Month"
        assert!(parse_recurring_timestr("Every 15th 9AM").is_err()); // Must use "of the Month"
        assert!(parse_recurring_timestr("Every InvalidDay").is_err());
        assert!(parse_recurring_timestr("Every 32nd of the Month").is_err());
        assert!(parse_recurring_timestr("Every 2/30").is_err()); // Invalid date
    }
}
