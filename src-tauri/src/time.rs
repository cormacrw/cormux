//! The clock, in each form Cormux stores it.

use std::time::{SystemTime, UNIX_EPOCH};

pub fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

/// Now as `YYYY-MM-DDTHH:MM:SSZ`, the form GitHub's `updatedAt` takes.
pub fn iso_now() -> String {
    let (date, time) = utc(unix_secs());
    format!("{date}T{time}Z")
}

fn utc(unix_secs: u64) -> (String, String) {
    let rem = unix_secs % 86_400;
    let (year, month, day) = civil_from_days((unix_secs / 86_400) as i64);
    (
        format!("{year:04}-{month:02}-{day:02}"),
        format!("{:02}:{:02}:{:02}", rem / 3600, (rem % 3600) / 60, rem % 60),
    )
}

/// A stored `YYYY-MM-DD HH:MM:SS` (SQLite's `datetime('now')`, which is UTC) as Unix
/// milliseconds, or 0 when the text isn't one.
pub fn sqlite_to_unix_ms(text: &str) -> i64 {
    let parse = || -> Option<i64> {
        let (date, time) = text.trim().split_once(' ')?;
        let mut date = date.splitn(3, '-').map(str::parse::<i64>);
        let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
        let mut time = time.splitn(3, ':').map(str::parse::<i64>);
        let (hour, minute, second) = (time.next()?.ok()?, time.next()?.ok()?, time.next()?.ok()?);
        let valid = (1..=12).contains(&month)
            && (1..=31).contains(&day)
            && (0..24).contains(&hour)
            && (0..60).contains(&minute)
            && (0..=60).contains(&second);
        valid.then(|| {
            let secs =
                days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
            secs * 1000
        })
    };
    parse().unwrap_or(0)
}

/// A proleptic Gregorian date to days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_times_come_out_in_utc() {
        for (secs, date, time) in [
            (0, "1970-01-01", "00:00:00"),
            (951_782_400, "2000-02-29", "00:00:00"),
            (1_727_657_551, "2024-09-30", "00:52:31"),
            (4_102_444_799, "2099-12-31", "23:59:59"),
        ] {
            assert_eq!(utc(secs), (date.to_string(), time.to_string()));
            assert_eq!(
                sqlite_to_unix_ms(&format!("{date} {time}")),
                secs as i64 * 1000
            );
        }
        assert_eq!(sqlite_to_unix_ms(""), 0);
        assert_eq!(sqlite_to_unix_ms("1700000000"), 0);
        assert_eq!(sqlite_to_unix_ms("2024-13-01 00:00:00"), 0);
    }
}
