use chrono::{
    Datelike,
    Duration,
    Local,
    TimeZone,
    Timelike,
};

// Parse a cron string and return the next or last occurrence timestamp
// The cron implementation is specific to this project
// avoiding additional dependency while implementing specific
// subset of cron functionalities.
pub fn get_next_occurrence(cron_str: &str) -> Result<i64, String> {
    get_occurrence_from(cron_str, Local::now(), true)
}

pub fn get_last_occurrence(cron_str: &str) -> Result<i64, String> {
    get_occurrence_from(cron_str, Local::now(), false)
}

// Underlying implementation to allow for testing
fn get_occurrence_from(cron_str: &str, now: chrono::DateTime<Local>, forward: bool) -> Result<i64, String> {
    let parts: Vec<&str> = cron_str.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(format!("Invalid cron format: {}", cron_str));
    }

    let minute: u32 = parts[0]
        .parse()
        .map_err(|_| format!("Invalid minute: {}", parts[0]))?;
    let hour: u32 = parts[1]
        .parse()
        .map_err(|_| format!("Invalid hour: {}", parts[1]))?;
    let day: Option<u32> = if parts[2] == "*" {
        None
    } else {
        Some(
            parts[2]
                .parse()
                .map_err(|_| format!("Invalid day: {}", parts[2]))?,
        )
    };
    let month: Option<u32> = if parts[3] == "*" {
        None
    } else {
        Some(
            parts[3]
                .parse()
                .map_err(|_| format!("Invalid month: {}", parts[3]))?,
        )
    };
    let weekday_str = parts[4];

    match (day, month, weekday_str) {
        // Daily: "minute hour * * *"
        (None, None, "*") => calculate_daily(now, minute, hour, forward),
        // Weekly: "minute hour * * weekday" or "minute hour * * range"
        (None, None, wd) => calculate_weekly(now, minute, hour, wd, forward),
        // Monthly: "minute hour day * *"
        (Some(d), None, "*") => calculate_monthly(now, minute, hour, d, forward),
        // Yearly: "minute hour day month *"
        (Some(d), Some(m), "*") => calculate_yearly(now, minute, hour, d, m, forward),
        _ => Err(format!("Unsupported cron pattern: {}", cron_str)),
    }
}

fn calculate_daily(now: chrono::DateTime<Local>, minute: u32, hour: u32, forward: bool) -> Result<i64, String> {
    let mut candidate = now
        .with_hour(hour)
        .ok_or("Invalid hour")?
        .with_minute(minute)
        .ok_or("Invalid minute")?
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();

    if forward {
        // If we're past that time today, move to tomorrow
        if candidate <= now {
            candidate += Duration::days(1);
        }
    } else {
        // If we're before that time today, move to yesterday
        if candidate >= now {
            candidate -= Duration::days(1);
        }
    }

    Ok(candidate.timestamp())
}

fn calculate_weekly(
    now: chrono::DateTime<Local>,
    minute: u32,
    hour: u32,
    weekday_str: &str,
    forward: bool,
) -> Result<i64, String> {
    let mut candidate = now
        .with_hour(hour)
        .ok_or("Invalid hour")?
        .with_minute(minute)
        .ok_or("Invalid minute")?
        .with_second(0)
        .unwrap();

    // Parse weekday - either single weekday or range between "1-7"
    let (start, end) = if weekday_str.contains('-') {
        let parts: Vec<&str> = weekday_str.split('-').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid weekday range: {}", weekday_str));
        }
        let s: u32 = parts[0]
            .parse()
            .map_err(|_| format!("Invalid weekday: {}", parts[0]))?;
        let e: u32 = parts[1]
            .parse()
            .map_err(|_| format!("Invalid weekday: {}", parts[1]))?;
        (s, e)
    } else {
        let wd: u32 = weekday_str
            .parse()
            .map_err(|_| format!("Invalid weekday: {}", weekday_str))?;
        // Normalize single weekday 7 to 0 (both mean Sunday)
        let normalized = if wd == 7 { 0 } else { wd };
        (normalized, normalized)
    };

    // Find next/last matching weekday
    for _ in 0..8 {
        let wd = candidate.weekday().num_days_from_sunday();
        // Check if weekday matches range
        // Sunday is 0 from chrono, but also matches 7 in cron ranges
        let matches = (wd >= start && wd <= end) || (wd == 0 && 7 == end);

        let time_match = if forward { candidate > now } else { candidate < now };
        if matches && time_match {
            return Ok(candidate.timestamp());
        }

        if forward {
            candidate += Duration::days(1);
        } else {
            candidate -= Duration::days(1);
        }
    }

    Err("Could not find valid weekday".to_string())
}

fn calculate_monthly(
    now: chrono::DateTime<Local>,
    minute: u32,
    hour: u32,
    day: u32,
    forward: bool,
) -> Result<i64, String> {
    let mut year = now.year();
    let mut month = now.month();

    // Try current month
    if let Some(dt) = Local
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .earliest()
    {
        let time_match = if forward { dt > now } else { dt < now };
        if time_match {
            return Ok(dt.timestamp());
        }
    }

    if forward {
        // Try next month
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
        let dt = Local
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .earliest();
        match dt {
            Some(dt) => Ok(dt.timestamp()),
            None => Err(format!("Day {} does not exist in month {}", day, month)),
        }
    } else {
        // Try previous months (up to 12 months back) to find valid day
        for _ in 0..12 {
            if month == 1 {
                month = 12;
                year -= 1;
            } else {
                month -= 1;
            }

            if let Some(dt) = Local
                .with_ymd_and_hms(year, month, day, hour, minute, 0)
                .earliest()
            {
                return Ok(dt.timestamp());
            }
        }

        Err(format!("Day {} does not exist in any month", day))
    }
}

fn calculate_yearly(
    now: chrono::DateTime<Local>,
    minute: u32,
    hour: u32,
    day: u32,
    month: u32,
    forward: bool,
) -> Result<i64, String> {
    let mut year = now.year();

    // Try this year
    if let Some(dt) = Local
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .earliest()
    {
        let time_match = if forward { dt > now } else { dt < now };
        if time_match {
            return Ok(dt.timestamp());
        }
    }

    // Try next/previous year
    if forward {
        year += 1;
    } else {
        year -= 1;
    }
    let dt = Local
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .earliest();
    match dt {
        Some(dt) => Ok(dt.timestamp()),
        None => Err(format!("Invalid date: {}/{}/{}", month, day, year)),
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;

    use super::*;

    #[test]
    fn test_next_occurrence() {
        let test_cases = vec![
            // (now, cron, expected)
            // Daily tests
            ("2024-03-15 10:00", "30 14 * * *", "2024-03-15 14:30"), // Same day, future time
            ("2024-03-15 15:00", "30 14 * * *", "2024-03-16 14:30"), // Next day
            ("2024-03-15 23:00", "30 2 * * *", "2024-03-16 02:30"),  // Next day early AM
            // Weekly tests - single day
            ("2024-03-15 10:00", "0 9 * * 1", "2024-03-18 09:00"), // Friday -> Monday
            ("2024-03-18 10:00", "0 9 * * 1", "2024-03-25 09:00"), // Monday after time -> next Monday
            ("2024-03-18 08:00", "0 9 * * 1", "2024-03-18 09:00"), // Monday before time -> same Monday
            ("2024-03-17 08:00", "0 9 * * 7", "2024-03-17 09:00"), // Sunday before time -> same Sunday
            ("2024-03-17 10:00", "0 9 * * 7", "2024-03-24 09:00"), // Sunday after time -> next Sunday
            // Weekly tests - range
            ("2024-03-16 10:00", "0 9 * * 1-5", "2024-03-18 09:00"), // Sat -> Mon
            ("2024-03-17 10:00", "0 9 * * 1-5", "2024-03-18 09:00"), // Sun -> Mon
            ("2024-03-18 08:00", "0 9 * * 1-5", "2024-03-18 09:00"), // Mon before time
            ("2024-03-18 10:00", "0 9 * * 1-5", "2024-03-19 09:00"), // Mon after time -> Tue
            ("2024-03-20 08:00", "0 9 * * 1-5", "2024-03-20 09:00"), // Wed before time -> Wed
            ("2024-03-20 10:00", "0 9 * * 1-5", "2024-03-21 09:00"), // Wed after time -> Thu
            ("2024-03-15 10:00", "0 9 * * 6-7", "2024-03-16 09:00"), // Fri -> Sat
            ("2024-03-16 08:00", "0 9 * * 6-7", "2024-03-16 09:00"), // Sat before time -> Sat
            ("2024-03-16 10:00", "0 9 * * 6-7", "2024-03-17 09:00"), // Sat after time -> Sun
            ("2024-03-17 10:00", "0 9 * * 6-7", "2024-03-23 09:00"), // Sun after time -> next Sat
            // Monthly tests
            ("2024-03-10 10:00", "0 9 15 * *", "2024-03-15 09:00"), // Same month
            ("2024-03-20 10:00", "0 9 15 * *", "2024-04-15 09:00"), // Next month
            ("2024-03-15 08:00", "0 9 15 * *", "2024-03-15 09:00"), // Same day before time
            ("2024-03-15 10:00", "0 9 15 * *", "2024-04-15 09:00"), // Same day after time
            // Monthly edge case - Feb 30 doesn't exist
            ("2024-02-15 10:00", "0 9 30 * *", "2024-03-30 09:00"), // Skip Feb
            // Yearly tests
            ("2024-03-15 10:00", "0 9 25 12 *", "2024-12-25 09:00"), // Same year
            ("2024-12-26 10:00", "0 9 25 12 *", "2025-12-25 09:00"), // Next year
            ("2024-12-25 08:00", "0 9 25 12 *", "2024-12-25 09:00"), // Same day before time
            ("2024-12-25 10:00", "0 9 25 12 *", "2025-12-25 09:00"), // Same day after time
        ];

        for (now_str, cron, expected_str) in test_cases {
            let now_naive = NaiveDateTime::parse_from_str(now_str, "%Y-%m-%d %H:%M")
                .unwrap_or_else(|_| panic!("Invalid test date: {}", now_str));
            let now = Local.from_local_datetime(&now_naive).unwrap();

            let expected_naive = NaiveDateTime::parse_from_str(expected_str, "%Y-%m-%d %H:%M")
                .unwrap_or_else(|_| panic!("Invalid expected date: {}", expected_str));
            let expected = Local.from_local_datetime(&expected_naive).unwrap();

            let result = get_occurrence_from(cron, now, true);
            assert!(
                result.is_ok(),
                "Failed for cron '{}' at '{}': {:?}",
                cron,
                now_str,
                result.err()
            );

            let actual_ts = result.unwrap();
            let actual = Local.timestamp_opt(actual_ts, 0).unwrap();

            assert_eq!(
                actual,
                expected,
                "Cron '{}' at '{}': expected '{}', got '{}'",
                cron,
                now_str,
                expected_str,
                actual.format("%Y-%m-%d %H:%M")
            );
        }
    }

    #[test]
    fn test_last_occurrence() {
        let test_cases = vec![
            // (now, cron, expected)
            // Daily tests
            ("2024-03-15 10:00", "30 14 * * *", "2024-03-14 14:30"), // Same day, past time -> yesterday
            ("2024-03-15 15:00", "30 14 * * *", "2024-03-15 14:30"), // Same day, past
            ("2024-03-16 02:00", "30 23 * * *", "2024-03-15 23:30"),  // Early AM -> yesterday
            // Weekly tests - single day
            ("2024-03-18 10:00", "0 9 * * 1", "2024-03-18 09:00"), // Monday after time -> same Monday
            ("2024-03-18 08:00", "0 9 * * 1", "2024-03-11 09:00"), // Monday before time -> previous Monday
            ("2024-03-17 10:00", "0 9 * * 7", "2024-03-17 09:00"), // Sunday after time -> same Sunday
            ("2024-03-17 08:00", "0 9 * * 7", "2024-03-10 09:00"), // Sunday before time -> previous Sunday
            // Weekly tests - range
            ("2024-03-18 10:00", "0 9 * * 1-5", "2024-03-18 09:00"), // Mon after time -> same Mon
            ("2024-03-18 08:00", "0 9 * * 1-5", "2024-03-15 09:00"), // Mon before time -> Fri
            ("2024-03-20 10:00", "0 9 * * 1-5", "2024-03-20 09:00"), // Wed after time -> same Wed
            ("2024-03-20 08:00", "0 9 * * 1-5", "2024-03-19 09:00"), // Wed before time -> Tue
            ("2024-03-17 10:00", "0 9 * * 6-7", "2024-03-17 09:00"), // Sun after time -> same Sun
            ("2024-03-17 08:00", "0 9 * * 6-7", "2024-03-16 09:00"), // Sun before time -> Sat
            ("2024-03-16 10:00", "0 9 * * 6-7", "2024-03-16 09:00"), // Sat after time -> same Sat
            ("2024-03-16 08:00", "0 9 * * 6-7", "2024-03-10 09:00"), // Sat before time -> previous Sun
            // Monthly tests
            ("2024-03-20 10:00", "0 9 15 * *", "2024-03-15 09:00"), // After day -> same month
            ("2024-03-10 10:00", "0 9 15 * *", "2024-02-15 09:00"), // Before day -> previous month
            ("2024-03-15 10:00", "0 9 15 * *", "2024-03-15 09:00"), // Same day after time
            ("2024-03-15 08:00", "0 9 15 * *", "2024-02-15 09:00"), // Same day before time
            // Monthly edge case - Feb 30 doesn't exist, skip to Jan
            ("2024-03-15 10:00", "0 9 30 * *", "2024-01-30 09:00"), // Skip Feb, go to Jan 30
            // Yearly tests
            ("2024-12-26 10:00", "0 9 25 12 *", "2024-12-25 09:00"), // After date -> same year
            ("2024-03-15 10:00", "0 9 25 12 *", "2023-12-25 09:00"), // Before date -> previous year
            ("2024-12-25 10:00", "0 9 25 12 *", "2024-12-25 09:00"), // Same day after time
            ("2024-12-25 08:00", "0 9 25 12 *", "2023-12-25 09:00"), // Same day before time
        ];

        for (now_str, cron, expected_str) in test_cases {
            let now_naive = NaiveDateTime::parse_from_str(now_str, "%Y-%m-%d %H:%M")
                .unwrap_or_else(|_| panic!("Invalid test date: {}", now_str));
            let now = Local.from_local_datetime(&now_naive).unwrap();

            let expected_naive = NaiveDateTime::parse_from_str(expected_str, "%Y-%m-%d %H:%M")
                .unwrap_or_else(|_| panic!("Invalid expected date: {}", expected_str));
            let expected = Local.from_local_datetime(&expected_naive).unwrap();

            let result = get_occurrence_from(cron, now, false);
            assert!(
                result.is_ok(),
                "Failed for cron '{}' at '{}': {:?}",
                cron,
                now_str,
                result.err()
            );

            let actual_ts = result.unwrap();
            let actual = Local.timestamp_opt(actual_ts, 0).unwrap();

            assert_eq!(
                actual,
                expected,
                "Cron '{}' at '{}': expected '{}', got '{}'",
                cron,
                now_str,
                expected_str,
                actual.format("%Y-%m-%d %H:%M")
            );
        }
    }
}
