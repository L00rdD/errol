//! French wording shared by the scenes: dates, durations, sizes.

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use errol_domain::entities::{Sky, Timestamp};

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

const NUMBERS: [&str; 30] = [
    "zéro",
    "une",
    "deux",
    "trois",
    "quatre",
    "cinq",
    "six",
    "sept",
    "huit",
    "neuf",
    "dix",
    "onze",
    "douze",
    "treize",
    "quatorze",
    "quinze",
    "seize",
    "dix-sept",
    "dix-huit",
    "dix-neuf",
    "vingt",
    "vingt et une",
    "vingt-deux",
    "vingt-trois",
    "vingt-quatre",
    "vingt-cinq",
    "vingt-six",
    "vingt-sept",
    "vingt-huit",
    "vingt-neuf",
];

/// The time as French speakers say it: "une heure moins le quart".
pub fn in_words(hour: u32, minute: u32) -> String {
    let masculine = |word: &str| {
        word.strip_suffix("une")
            .map_or(word.to_owned(), |w| format!("{w}un"))
    };
    let (next, minutes) = match minute {
        0 => (false, String::new()),
        15 => (false, " et quart".into()),
        30 => (false, " et demie".into()),
        45 => (true, " moins le quart".into()),
        m if m > 30 => (
            true,
            format!(" moins {}", masculine(NUMBERS[60 - m as usize])),
        ),
        m => (false, format!(" {}", masculine(NUMBERS[m as usize]))),
    };
    let h = (hour + u32::from(next)) % 24;
    let hours = match h {
        0 => "minuit".to_owned(),
        12 => "midi".to_owned(),
        1 | 13 => "une heure".to_owned(),
        _ => format!("{} heures", NUMBERS[(h % 12) as usize]),
    };
    format!("{hours}{minutes}")
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

/// A long wait as one says it: "5 h 12", "12 min".
pub fn hours(secs: f64) -> String {
    let minutes = (secs.max(0.0) / 60.0).round() as u64;
    if minutes >= 60 {
        format!("{} h {:02}", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} min")
    }
}

/// What the screen calls a print's state.
pub fn print_state(state: &str) -> &str {
    match state {
        "printing" => "en cours",
        "done" => "terminée",
        "cancelled" => "annulée",
        "downloading" => "réception du fichier",
        "checking" => "vérification",
        "preheating" => "préchauffage",
        "levelling" => "mise à niveau",
        "slicing" => "découpage",
        other => other,
    }
}

/// Why a printer shows no print, from what its cloud said.
pub fn print_cloud(cloud: &str) -> &str {
    match cloud {
        "no token" => "pas de jeton Anycubic : impression non suivie",
        "token rejected" => "jeton Anycubic expiré",
        "unreachable" => "cloud Anycubic injoignable",
        "no printer on the account" => "aucune imprimante sur le compte",
        other => other,
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

/// Days from `now`'s date to `at`'s, in local time: 0 today, 1 tomorrow.
pub fn days_after(at: Timestamp, now: Timestamp) -> i64 {
    (local(at).date_naive() - local(now).date_naive()).num_days()
}

/// The day of `at`, said from `now`: "ce soir" (after 18 h), "aujourd'hui",
/// "demain", "hier", or its weekday.
pub fn day_word(at: Timestamp, now: Timestamp) -> String {
    match days_after(at, now) {
        0 if local(at).hour() >= 18 => "ce soir".to_owned(),
        0 => "aujourd'hui".to_owned(),
        1 => "demain".to_owned(),
        -1 => "hier".to_owned(),
        _ => weekday(at),
    }
}

/// When something airs, said from `now`: "ce soir 21:00", "demain 20:30".
pub fn airs(at: Timestamp, now: Timestamp) -> String {
    format!("{} {}", day_word(at, now), clock(at))
}

/// A time to come, said from `now`: "à 19:00" today, "demain à 07:00".
pub fn at_time(at: Timestamp, now: Timestamp) -> String {
    match days_after(at, now) {
        0 => format!("à {}", clock(at)),
        _ => format!("{} à {}", day_word(at, now), clock(at)),
    }
}

/// How soon, `secs` from now: "dans 4 min", "dans 1 h 05", "dans un instant".
pub fn within(secs: f64) -> String {
    if secs < 60.0 {
        "dans un instant".to_owned()
    } else {
        format!("dans {}", hours(secs))
    }
}

/// 9°, -2°: a temperature, rounded.
pub fn temperature(celsius: f64) -> String {
    format!("{}°", celsius.round() as i64)
}

/// The sky in a word or two.
pub fn sky(sky: Sky) -> &'static str {
    match sky {
        Sky::Clear => "ciel dégagé",
        Sky::Cloudy => "éclaircies",
        Sky::Overcast => "couvert",
        Sky::Fog => "brouillard",
        Sky::Drizzle => "bruine",
        Sky::Rain => "pluie",
        Sky::Snow => "neige",
        Sky::Storm => "orage",
    }
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
    fn the_time_is_said_as_french_speakers_do() {
        assert_eq!(in_words(22, 5), "dix heures cinq");
        assert_eq!(in_words(0, 45), "une heure moins le quart");
        assert_eq!(in_words(12, 30), "midi et demie");
        assert_eq!(in_words(23, 21), "onze heures vingt et un");
        assert_eq!(in_words(13, 39), "deux heures moins vingt et un");
    }

    #[test]
    fn long_waits_in_hours_and_minutes() {
        assert_eq!(hours(18_720.0), "5 h 12");
        assert_eq!(hours(3_600.0), "1 h 00");
        assert_eq!(hours(725.0), "12 min");
    }

    #[test]
    fn print_states_and_cloud_words_in_french() {
        assert_eq!(print_state("done"), "terminée");
        assert_eq!(print_state("paused"), "paused");
        assert_eq!(print_cloud("token rejected"), "jeton Anycubic expiré");
        assert_eq!(print_cloud("odd"), "odd");
    }

    #[test]
    fn sizes_and_percents_use_a_french_comma() {
        assert_eq!(size(6_400_000_000), "6,4 Go");
        assert_eq!(percent(-4.63), "-4,6 %");
    }
    fn at(day: u32, hour: u32, minute: u32) -> Timestamp {
        Timestamp(
            Local
                .with_ymd_and_hms(2026, 11, day, hour, minute, 0)
                .single()
                .unwrap()
                .timestamp_millis(),
        )
    }

    #[test]
    fn a_time_is_said_from_today() {
        let now = at(8, 15, 0);
        assert_eq!(airs(at(8, 21, 0), now), "ce soir 21:00");
        assert_eq!(airs(at(8, 16, 30), now), "aujourd'hui 16:30");
        assert_eq!(airs(at(9, 20, 30), now), "demain 20:30");
        assert_eq!(day_word(at(7, 20, 0), now), "hier");
        assert_eq!(day_word(at(12, 20, 0), now), "jeudi 12");
        assert_eq!(at_time(at(8, 19, 0), now), "à 19:00");
        assert_eq!(at_time(at(9, 7, 0), now), "demain à 07:00");
        assert_eq!(days_after(at(10, 0, 5), now), 2);
    }

    #[test]
    fn soon_is_said_in_minutes_or_hours() {
        assert_eq!(within(20.0), "dans un instant");
        assert_eq!(within(240.0), "dans 4 min");
        assert_eq!(within(3900.0), "dans 1 h 05");
    }

    #[test]
    fn the_weather_in_french() {
        assert_eq!(temperature(9.4), "9°");
        assert_eq!(temperature(-0.4), "0°");
        assert_eq!(temperature(-2.6), "-3°");
        assert_eq!(sky(Sky::Rain), "pluie");
        assert_eq!(
            [
                Sky::Clear,
                Sky::Cloudy,
                Sky::Overcast,
                Sky::Fog,
                Sky::Drizzle,
                Sky::Snow,
                Sky::Storm
            ]
            .map(sky)
            .len(),
            7
        );
    }
}
