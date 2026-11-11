//! Message dates as a mail list shows them: the time today, the day this year, the full date
//! before that. Always in the system's time zone.

use jiff::{Timestamp, Zoned};

pub fn label(unix_ms: i64, now: &Zoned) -> String {
    let Ok(at) = Timestamp::from_millisecond(unix_ms) else { return String::new() };
    let at = at.to_zoned(now.time_zone().clone());
    let format = if at.date() == now.date() {
        "%-I:%M %p"
    } else if at.year() == now.year() {
        "%b %-d"
    } else {
        "%-m/%-d/%y"
    };
    at.strftime(format).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> i64 {
        s.parse::<Timestamp>().unwrap().as_millisecond()
    }

    #[test]
    fn today_shows_the_time_this_year_the_day_and_older_the_date() {
        let now: Zoned = "2026-11-10T15:00:00[America/New_York]".parse().unwrap();
        assert_eq!(label(at("2026-11-10T14:42:00Z"), &now), "9:42 AM");
        assert_eq!(label(at("2026-11-08T12:00:00Z"), &now), "Nov 8");
        assert_eq!(label(at("2024-03-14T12:00:00Z"), &now), "3/14/24");
    }

    #[test]
    fn the_day_boundary_follows_the_local_zone() {
        // 03:00 UTC on the 10th is still the evening of the 9th in New York.
        let now: Zoned = "2026-11-10T15:00:00[America/New_York]".parse().unwrap();
        assert_eq!(label(at("2026-11-10T03:00:00Z"), &now), "Nov 9");
    }
}
