// Shared scheduling utilities

use chrono::{DateTime, Datelike, NaiveTime, Timelike, Utc, Weekday};

/// Returns the last day of the given month
pub fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(28)
}

pub fn next_hourly(from: DateTime<Utc>, minute: u32) -> DateTime<Utc> {
    let naive = from.naive_utc();
    let clamped = minute.min(59);
    // with_minute cannot fail when clamped is in 0..=59; zero out seconds/nanos
    let next_naive = naive
        .with_minute(clamped)
        .and_then(|t| t.with_second(0))
        .and_then(|t| t.with_nanosecond(0))
        .expect("clamped minute 0..=59 and second/nanosecond 0 are always valid");
    let mut next = next_naive.and_utc();
    if next <= from {
        next += chrono::TimeDelta::hours(1);
    }
    next
}

pub fn next_daily(from: DateTime<Utc>, time: &NaiveTime) -> DateTime<Utc> {
    let date = from.date_naive();
    let mut next = date.and_time(*time).and_utc();
    if next <= from {
        next += chrono::TimeDelta::days(1);
    }
    next
}

pub fn next_weekly(from: DateTime<Utc>, weekday: Weekday, time: &NaiveTime) -> DateTime<Utc> {
    if from.naive_utc().weekday() == weekday {
        let today = from.date_naive().and_time(*time).and_utc();
        if today > from {
            return today;
        }
    }
    let mut next = from;
    loop {
        next += chrono::TimeDelta::days(1);
        if next.naive_utc().weekday() == weekday {
            let date = next.date_naive();
            next = date.and_time(*time).and_utc();
            if next > from {
                break;
            }
        }
    }
    next
}

pub fn next_monthly(from: DateTime<Utc>, day: u32, time: &NaiveTime) -> DateTime<Utc> {
    if from.naive_utc().day() == day {
        let today = from.date_naive().and_time(*time).and_utc();
        if today > from {
            return today;
        }
    }
    let mut current = from;
    for _ in 0..60 {
        current += chrono::TimeDelta::days(1);
        if current.naive_utc().day() == day {
            let date = current.date_naive();
            let candidate = date.and_time(*time).and_utc();
            if candidate > from {
                return candidate;
            }
        }
        if current.naive_utc().day() == 1 && day > 28 {
            let year = current.naive_utc().year();
            let month = current.naive_utc().month();
            let last_day = last_day_of_month(year, month);
            if day > last_day {
                if let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, last_day) {
                    let candidate = date.and_time(*time).and_utc();
                    if candidate > from {
                        return candidate;
                    }
                }
            }
        }
    }
    // Fallback: ensure result is always after `from`
    let fallback = current.date_naive().and_time(*time).and_utc();
    if fallback > from {
        fallback
    } else {
        // Advance one more day to guarantee we're past `from`
        (current + chrono::TimeDelta::days(1))
            .date_naive()
            .and_time(*time)
            .and_utc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, min, sec)
            .unwrap()
    }

    fn time(hour: u32, min: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, min, 0).unwrap()
    }

    // ---- last_day_of_month ----

    #[test]
    fn test_last_day_january() {
        assert_eq!(last_day_of_month(2024, 1), 31);
    }

    #[test]
    fn test_last_day_february_leap_year() {
        assert_eq!(last_day_of_month(2024, 2), 29);
    }

    #[test]
    fn test_last_day_february_non_leap_year() {
        assert_eq!(last_day_of_month(2023, 2), 28);
    }

    #[test]
    fn test_last_day_february_century_non_leap() {
        // 1900 is not a leap year (divisible by 100 but not 400)
        assert_eq!(last_day_of_month(1900, 2), 28);
    }

    #[test]
    fn test_last_day_february_century_leap() {
        // 2000 is a leap year (divisible by 400)
        assert_eq!(last_day_of_month(2000, 2), 29);
    }

    #[test]
    fn test_last_day_april() {
        assert_eq!(last_day_of_month(2024, 4), 30);
    }

    #[test]
    fn test_last_day_june() {
        assert_eq!(last_day_of_month(2024, 6), 30);
    }

    #[test]
    fn test_last_day_september() {
        assert_eq!(last_day_of_month(2024, 9), 30);
    }

    #[test]
    fn test_last_day_november() {
        assert_eq!(last_day_of_month(2024, 11), 30);
    }

    #[test]
    fn test_last_day_december() {
        assert_eq!(last_day_of_month(2024, 12), 31);
    }

    #[test]
    fn test_last_day_march() {
        assert_eq!(last_day_of_month(2024, 3), 31);
    }

    #[test]
    fn test_last_day_july() {
        assert_eq!(last_day_of_month(2024, 7), 31);
    }

    #[test]
    fn test_last_day_all_months() {
        let expected = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]; // 2024 is a leap year
        for (i, &exp) in expected.iter().enumerate() {
            assert_eq!(
                last_day_of_month(2024, i as u32 + 1),
                exp,
                "Failed for month {}",
                i + 1
            );
        }
    }

    // ---- next_hourly ----

    #[test]
    fn test_next_hourly_minute_in_future() {
        // Current time is 10:15, target minute is 30 -> should be 10:30 same hour
        let from = utc(2024, 6, 15, 10, 15, 0);
        let next = next_hourly(from, 30);
        assert_eq!(next, utc(2024, 6, 15, 10, 30, 0));
    }

    #[test]
    fn test_next_hourly_minute_in_past() {
        // Current time is 10:45, target minute is 15 -> should be 11:15 next hour
        let from = utc(2024, 6, 15, 10, 45, 0);
        let next = next_hourly(from, 15);
        assert_eq!(next, utc(2024, 6, 15, 11, 15, 0));
    }

    #[test]
    fn test_next_hourly_minute_is_now() {
        // Current time is 10:30:00, target minute is 30 -> should advance to 11:30
        let from = utc(2024, 6, 15, 10, 30, 0);
        let next = next_hourly(from, 30);
        assert_eq!(next, utc(2024, 6, 15, 11, 30, 0));
    }

    #[test]
    fn test_next_hourly_rolls_over_midnight() {
        // Current time is 23:50, target minute is 10 -> should be 00:10 next day
        let from = utc(2024, 6, 15, 23, 50, 0);
        let next = next_hourly(from, 10);
        assert_eq!(next, utc(2024, 6, 16, 0, 10, 0));
    }

    #[test]
    fn test_next_hourly_minute_zero() {
        let from = utc(2024, 6, 15, 10, 30, 0);
        let next = next_hourly(from, 0);
        assert_eq!(next, utc(2024, 6, 15, 11, 0, 0));
    }

    #[test]
    fn test_next_hourly_clamps_large_minute() {
        // Minutes > 59 should be clamped to 59
        let from = utc(2024, 6, 15, 10, 0, 0);
        let next = next_hourly(from, 100);
        assert_eq!(next, utc(2024, 6, 15, 10, 59, 0));
    }

    // ---- next_daily ----

    #[test]
    fn test_next_daily_time_in_future_today() {
        // Current time is 08:00, target time is 14:00 -> should be today 14:00
        let from = utc(2024, 6, 15, 8, 0, 0);
        let t = time(14, 0);
        let next = next_daily(from, &t);
        assert_eq!(next, utc(2024, 6, 15, 14, 0, 0));
    }

    #[test]
    fn test_next_daily_time_in_past_today() {
        // Current time is 18:00, target time is 06:00 -> should be tomorrow 06:00
        let from = utc(2024, 6, 15, 18, 0, 0);
        let t = time(6, 0);
        let next = next_daily(from, &t);
        assert_eq!(next, utc(2024, 6, 16, 6, 0, 0));
    }

    #[test]
    fn test_next_daily_time_is_exactly_now() {
        // Current time equals target time -> should be tomorrow
        let from = utc(2024, 6, 15, 14, 0, 0);
        let t = time(14, 0);
        let next = next_daily(from, &t);
        assert_eq!(next, utc(2024, 6, 16, 14, 0, 0));
    }

    #[test]
    fn test_next_daily_rolls_over_month() {
        // Last day of June, target time past -> should be July 1
        let from = utc(2024, 6, 30, 23, 0, 0);
        let t = time(2, 0);
        let next = next_daily(from, &t);
        assert_eq!(next, utc(2024, 7, 1, 2, 0, 0));
    }

    #[test]
    fn test_next_daily_midnight() {
        let from = utc(2024, 6, 15, 0, 0, 0);
        let t = time(0, 0);
        let next = next_daily(from, &t);
        // 00:00 == from, so advances to next day
        assert_eq!(next, utc(2024, 6, 16, 0, 0, 0));
    }

    // ---- next_weekly ----

    #[test]
    fn test_next_weekly_today_is_target_day_time_in_future() {
        // 2024-06-17 is a Monday, target is Monday at 14:00, current time is 08:00
        let from = utc(2024, 6, 17, 8, 0, 0);
        let t = time(14, 0);
        let next = next_weekly(from, Weekday::Mon, &t);
        assert_eq!(next, utc(2024, 6, 17, 14, 0, 0));
    }

    #[test]
    fn test_next_weekly_today_is_target_day_time_in_past() {
        // 2024-06-17 is Monday, target is Monday at 06:00, current time is 18:00
        let from = utc(2024, 6, 17, 18, 0, 0);
        let t = time(6, 0);
        let next = next_weekly(from, Weekday::Mon, &t);
        // Should be next Monday
        assert_eq!(next, utc(2024, 6, 24, 6, 0, 0));
    }

    #[test]
    fn test_next_weekly_target_day_in_future() {
        // 2024-06-17 is Monday, target is Wednesday at 10:00
        let from = utc(2024, 6, 17, 8, 0, 0);
        let t = time(10, 0);
        let next = next_weekly(from, Weekday::Wed, &t);
        assert_eq!(next, utc(2024, 6, 19, 10, 0, 0));
    }

    #[test]
    fn test_next_weekly_target_day_in_past_this_week() {
        // 2024-06-19 is Wednesday, target is Monday -> next Monday
        let from = utc(2024, 6, 19, 8, 0, 0);
        let t = time(10, 0);
        let next = next_weekly(from, Weekday::Mon, &t);
        assert_eq!(next, utc(2024, 6, 24, 10, 0, 0));
    }

    #[test]
    fn test_next_weekly_sunday() {
        // 2024-06-17 is Monday, target is Sunday
        let from = utc(2024, 6, 17, 8, 0, 0);
        let t = time(3, 0);
        let next = next_weekly(from, Weekday::Sun, &t);
        assert_eq!(next, utc(2024, 6, 23, 3, 0, 0));
    }

    // ---- next_monthly ----

    #[test]
    fn test_next_monthly_day_15_before_15th() {
        // June 10, target day 15 at 10:00 -> should be June 15
        let from = utc(2024, 6, 10, 8, 0, 0);
        let t = time(10, 0);
        let next = next_monthly(from, 15, &t);
        assert_eq!(next, utc(2024, 6, 15, 10, 0, 0));
    }

    #[test]
    fn test_next_monthly_day_15_after_15th() {
        // June 20, target day 15 at 10:00 -> should be July 15
        let from = utc(2024, 6, 20, 8, 0, 0);
        let t = time(10, 0);
        let next = next_monthly(from, 15, &t);
        assert_eq!(next, utc(2024, 7, 15, 10, 0, 0));
    }

    #[test]
    fn test_next_monthly_day_15_same_day_time_in_future() {
        // June 15 at 08:00, target day 15 at 14:00 -> should be June 15 14:00
        let from = utc(2024, 6, 15, 8, 0, 0);
        let t = time(14, 0);
        let next = next_monthly(from, 15, &t);
        assert_eq!(next, utc(2024, 6, 15, 14, 0, 0));
    }

    #[test]
    fn test_next_monthly_day_15_same_day_time_in_past() {
        // June 15 at 18:00, target day 15 at 10:00 -> should be July 15
        let from = utc(2024, 6, 15, 18, 0, 0);
        let t = time(10, 0);
        let next = next_monthly(from, 15, &t);
        assert_eq!(next, utc(2024, 7, 15, 10, 0, 0));
    }

    #[test]
    fn test_next_monthly_day_31_skips_short_month_to_next_31day_month() {
        // From April 1, requesting day 31. April has only 30 days.
        // The fallback triggers when entering a new month (day==1) and
        // day > last_day_of_month. When reaching May 1, May has 31 days
        // (31 > 31 is false), so no fallback. May 31 is found directly.
        let from = utc(2024, 4, 1, 0, 0, 0);
        let t = time(2, 0);
        let next = next_monthly(from, 31, &t);
        assert_eq!(next, utc(2024, 5, 31, 2, 0, 0));
    }

    #[test]
    fn test_next_monthly_day_31_fallback_to_feb_last_day() {
        // From Jan 31 at 23:00, requesting day 31 at 02:00.
        // Jan 31 matches but time is past, so continues.
        // Feb 1 (day==1): last_day_of_month(2024, 2) = 29, 31 > 29 -> fallback to Feb 29.
        let from = utc(2024, 1, 31, 23, 0, 0);
        let t = time(2, 0);
        let next = next_monthly(from, 31, &t);
        assert_eq!(next, utc(2024, 2, 29, 2, 0, 0));
    }

    #[test]
    fn test_next_monthly_day_31_from_may() {
        // May has 31 days, requesting day 31 on May 1 -> should be May 31
        let from = utc(2024, 5, 1, 0, 0, 0);
        let t = time(2, 0);
        let next = next_monthly(from, 31, &t);
        assert_eq!(next, utc(2024, 5, 31, 2, 0, 0));
    }

    #[test]
    fn test_next_monthly_day_29_in_february_non_leap() {
        // 2023 is not a leap year, Feb has 28 days, requesting day 29
        // From Jan 30 -> should fall back to Feb 28
        let from = utc(2023, 1, 30, 0, 0, 0);
        let t = time(3, 0);
        let next = next_monthly(from, 29, &t);
        assert_eq!(next, utc(2023, 2, 28, 3, 0, 0));
    }

    #[test]
    fn test_next_monthly_day_29_in_february_leap() {
        // 2024 is a leap year, Feb has 29 days, requesting day 29
        // From Jan 30 -> should be Feb 29
        let from = utc(2024, 1, 30, 0, 0, 0);
        let t = time(3, 0);
        let next = next_monthly(from, 29, &t);
        assert_eq!(next, utc(2024, 2, 29, 3, 0, 0));
    }

    #[test]
    fn test_next_monthly_day_1() {
        // From Jan 15 with day=1 -> should be Feb 1
        let from = utc(2024, 1, 15, 10, 0, 0);
        let t = time(0, 0);
        let next = next_monthly(from, 1, &t);
        assert_eq!(next, utc(2024, 2, 1, 0, 0, 0));
    }

    #[test]
    fn test_next_monthly_day_28() {
        // From Jan 1, day=28 -> should be Jan 28
        let from = utc(2024, 1, 1, 0, 0, 0);
        let t = time(12, 0);
        let next = next_monthly(from, 28, &t);
        assert_eq!(next, utc(2024, 1, 28, 12, 0, 0));
    }
}
