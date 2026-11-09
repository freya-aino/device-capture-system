//! UTC formatting without a date-time dependency.

use std::time::{Duration, SystemTime};

/// Microseconds since the Unix epoch, or `None` before it.
pub fn micros_since_epoch(t: SystemTime) -> Option<u64> {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_micros() as u64)
}

/// `YYYY-MM-DDTHH:MM:SS.ffffffZ`.
pub fn rfc3339(t: SystemTime) -> String {
    let (y, mo, d, h, mi, s, us) = civil(t);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{us:06}Z")
}

/// Year, month, day, hour, minute, second and microsecond of `t` in UTC (times before the epoch
/// clamp to it). Days to date with Howard Hinnant's `civil_from_days`.
pub(crate) fn civil(t: SystemTime) -> (i64, u32, u32, u32, u32, u32, u32) {
    let since = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    let secs = since.as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (
        year,
        month,
        day,
        (rem / 3600) as u32,
        (rem % 3600 / 60) as u32,
        (rem % 60) as u32,
        since.subsec_micros(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_instants() {
        let at =
            |secs: u64, micros: u32| SystemTime::UNIX_EPOCH + Duration::new(secs, micros * 1000);
        assert_eq!(rfc3339(at(0, 0)), "1970-01-01T00:00:00.000000Z");
        assert_eq!(rfc3339(at(951_782_400, 5)), "2000-02-29T00:00:00.000005Z");
        assert_eq!(
            rfc3339(at(1_793_960_100, 123_456)),
            "2026-11-06T10:15:00.123456Z"
        );
        assert_eq!(micros_since_epoch(at(2, 7)), Some(2_000_007));
    }
}
