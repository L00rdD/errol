//! French wording shared by the scenes: dates, durations, sizes.

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use errol_domain::entities::Timestamp;

const DAYS: [&str; 7] = [
    "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
];
const MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

pub fn local(at: Timestamp) -> DateTime<Local> {
    Local
        .timestamp_millis_opt(at.0)
        .single()
        .unwrap_or_else(Local::now)
}

/// 22:05
pub fn clock(at: Timestamp) -> String {
    let time = local(at);
    format!("{:02}:{:02}", time.hour(), time.minute())
}

/// lundi 26 octobre
pub fn date(at: Timestamp) -> String {
    let time = local(at);
    format!(
        "{} {} {}",
        DAYS[time.weekday().num_days_from_monday() as usize],
        time.day(),
        MONTHS[time.month0() as usize]
    )
}

/// jeudi 29
pub fn weekday(at: Timestamp) -> String {
    let time = local(at);
    format!(
        "{} {}",
        DAYS[time.weekday().num_days_from_monday() as usize],
        time.day()
    )
}

/// 1:27:19, or 4:05 under an hour.
pub fn duration(secs: f64) -> String {
    let secs = secs.max(0.0) as u64;
    let (hours, minutes, seconds) = (secs / 3600, secs / 60 % 60, secs % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// 6,4 Go / 1,43 To
pub fn size(bytes: u64) -> String {
    let bytes = bytes as f64;
    if bytes >= 1e12 {
        format!("{:.2} To", bytes / 1e12).replace('.', ",")
    } else {
        format!("{:.1} Go", bytes / 1e9).replace('.', ",")
    }
}

/// A release name made readable: dots and underscores become spaces.
pub fn release(name: &str) -> String {
    name.replace(['.', '_'], " ")
}

/// +4,6 % / -4,6 %
pub fn percent(value: f64) -> String {
    format!("{}{:.1} %", if value >= 0.0 { "+" } else { "" }, value).replace('.', ",")
}

/// S02E04
pub fn episode(season: Option<u32>, episode: Option<u32>) -> String {
    format!(
        "S{:02}E{:02}",
        season.unwrap_or_default(),
        episode.unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_drop_the_hours_under_an_hour() {
        assert_eq!(duration(5239.0), "1:27:19");
        assert_eq!(duration(245.0), "4:05");
    }

    #[test]
    fn sizes_and_percents_use_a_french_comma() {
        assert_eq!(size(6_400_000_000), "6,4 Go");
        assert_eq!(percent(-4.63), "-4,6 %");
    }
}
