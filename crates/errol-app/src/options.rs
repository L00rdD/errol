//! Command line and environment. The service on the Pi sets the environment
//! (`ERROL_LOOK`…), a preview on the Mac uses the flags.

use std::iter::Peekable;

use errol_data::datasources::DemoScene;

use crate::look::Look;

#[derive(Debug, Clone)]
pub struct Options {
    pub look: Look,
    /// Made-up data instead of the feed.
    pub demo: Option<DemoScene>,
    pub feed_url: String,
    pub jellyfin_url: String,
    /// A window instead of the whole display.
    pub windowed: bool,
    /// Frames per second (the backgrounds drift slowly: 30 is plenty).
    pub fps: u32,
    /// Save a PNG of the screen after `shot_after` seconds, then quit.
    pub shot: Option<String>,
    pub shot_after: f64,
    /// The night whatever the hour and the house: the screen asleep, for
    /// previews and shots.
    pub night: bool,
}

const USAGE: &str = "usage: errol [--look friday|cyberpunk|hogwarts] [--demo [all|idle|light|cover|projector|cinema|import|download|scan|cerebro|print]]
                [--feed URL] [--jellyfin URL] [--window] [--fps N] [--night] [--shot FILE.png [--after SECS]]
environment: ERROL_LOOK, ERROL_FEED, ERROL_JELLYFIN, ERROL_FPS";

impl Options {
    pub fn from_env_and_args(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let env = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        Self::parse(env, args)
    }

    /// The options from `env` (a variable, when set and not empty) and the
    /// command line: a flag wins over the environment.
    fn parse(
        env: impl Fn(&str) -> Option<String>,
        args: impl Iterator<Item = String>,
    ) -> Result<Self, String> {
        let mut options = Self::from_env(env);
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            options.apply(&arg, &mut args)?;
        }
        options.fps = options.fps.clamp(5, 60);
        Ok(options)
    }

    fn from_env(env: impl Fn(&str) -> Option<String>) -> Self {
        Self {
            look: env("ERROL_LOOK")
                .and_then(|look| Look::parse(&look))
                .unwrap_or(Look::Friday),
            demo: None,
            feed_url: env("ERROL_FEED").unwrap_or_else(|| "http://localhost/screen.json".into()),
            jellyfin_url: env("ERROL_JELLYFIN").unwrap_or_else(|| "http://localhost:8096".into()),
            windowed: !cfg!(target_os = "linux"),
            fps: env("ERROL_FPS")
                .and_then(|fps| fps.parse().ok())
                .unwrap_or(30),
            shot: None,
            shot_after: 6.0,
            night: false,
        }
    }

    /// Takes one flag, and its value from `args` when it has one.
    fn apply<I: Iterator<Item = String>>(
        &mut self,
        arg: &str,
        args: &mut Peekable<I>,
    ) -> Result<(), String> {
        match arg {
            "--demo" => self.demo = Some(demo_scene(args)?),
            "--window" => self.windowed = true,
            "--night" => self.night = true,
            "-h" | "--help" => return Err(USAGE.into()),
            "--look" | "--feed" | "--jellyfin" | "--fps" | "--shot" | "--after" => {
                let value = args
                    .next()
                    .ok_or_else(|| format!("{arg} needs a value\n{USAGE}"))?;
                self.set(arg, value)?;
            }
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
        Ok(())
    }

    /// Sets what a flag with a value names.
    fn set(&mut self, flag: &str, value: String) -> Result<(), &'static str> {
        match flag {
            "--look" => self.look = Look::parse(&value).ok_or(USAGE)?,
            "--feed" => self.feed_url = value,
            "--jellyfin" => self.jellyfin_url = value,
            "--fps" => self.fps = value.parse().map_err(|_| USAGE)?,
            "--shot" => self.shot = Some(value),
            _ => self.shot_after = value.parse().map_err(|_| USAGE)?,
        }
        Ok(())
    }
}

/// `--demo`'s scene: the next argument unless it is a flag, else the whole evening.
fn demo_scene<I: Iterator<Item = String>>(args: &mut Peekable<I>) -> Result<DemoScene, String> {
    let scene = args
        .next_if(|next| !next.starts_with("--"))
        .unwrap_or_default();
    DemoScene::parse(&scene).ok_or_else(|| USAGE.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(env: &[(&str, &str)], args: &[&str]) -> Result<Options, String> {
        let env: Vec<(String, String)> = env
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        Options::parse(
            |name| {
                env.iter()
                    .find(|(key, _)| key == name)
                    .map(|(_, value)| value.clone())
            },
            args.iter().map(|arg| (*arg).to_owned()),
        )
    }

    #[test]
    fn without_anything_it_reads_the_local_feed_in_friday() {
        let options = parse(&[], &[]).unwrap();
        assert_eq!(options.look, Look::Friday);
        assert!(options.demo.is_none());
        assert_eq!(options.feed_url, "http://localhost/screen.json");
        assert_eq!(options.jellyfin_url, "http://localhost:8096");
        assert_eq!(options.fps, 30);
        assert!(options.shot.is_none());
        assert!((options.shot_after - 6.0).abs() < f64::EPSILON);
        assert!(!options.night);
    }

    #[test]
    fn the_environment_sets_the_service() {
        let options = parse(
            &[
                ("ERROL_LOOK", "hogwarts"),
                ("ERROL_FEED", "http://hub/screen.json"),
                ("ERROL_JELLYFIN", "http://hub:8096"),
                ("ERROL_FPS", "20"),
            ],
            &[],
        )
        .unwrap();
        assert_eq!(options.look, Look::Hogwarts);
        assert_eq!(options.feed_url, "http://hub/screen.json");
        assert_eq!(options.jellyfin_url, "http://hub:8096");
        assert_eq!(options.fps, 20);
    }

    #[test]
    fn a_bad_variable_keeps_the_default() {
        let options = parse(&[("ERROL_LOOK", "disco"), ("ERROL_FPS", "fast")], &[]).unwrap();
        assert_eq!(options.look, Look::Friday);
        assert_eq!(options.fps, 30);
    }

    #[test]
    fn the_flags_win_over_the_environment() {
        let options = parse(
            &[("ERROL_LOOK", "hogwarts"), ("ERROL_FEED", "http://hub/")],
            &[
                "--look",
                "cyberpunk",
                "--feed",
                "http://a/",
                "--jellyfin",
                "http://b/",
                "--window",
                "--fps",
                "12",
                "--shot",
                "x.png",
                "--after",
                "2.5",
                "--night",
            ],
        )
        .unwrap();
        assert_eq!(options.look, Look::Cyberpunk);
        assert_eq!(options.feed_url, "http://a/");
        assert_eq!(options.jellyfin_url, "http://b/");
        assert!(options.windowed);
        assert_eq!(options.fps, 12);
        assert_eq!(options.shot.as_deref(), Some("x.png"));
        assert!((options.shot_after - 2.5).abs() < f64::EPSILON);
        assert!(options.night);
    }

    #[test]
    fn the_frame_rate_stays_between_5_and_60() {
        assert_eq!(parse(&[], &["--fps", "1"]).unwrap().fps, 5);
        assert_eq!(parse(&[("ERROL_FPS", "240")], &[]).unwrap().fps, 60);
    }

    #[test]
    fn demo_takes_a_scene_or_the_whole_evening() {
        let held = parse(&[], &["--demo", "scan", "--look", "hogwarts"]).unwrap();
        assert_eq!(held.demo, Some(DemoScene::Scan));
        assert_eq!(held.look, Look::Hogwarts);
        let evening = parse(&[], &["--demo", "--window"]).unwrap();
        assert_eq!(evening.demo, Some(DemoScene::All));
        assert!(evening.windowed);
        assert_eq!(parse(&[], &["--demo"]).unwrap().demo, Some(DemoScene::All));
        assert!(parse(&[], &["--demo", "opera"]).is_err());
    }

    #[test]
    fn bad_flags_say_how_to_use_errol() {
        for args in [
            &["--look"][..],
            &["--look", "disco"],
            &["--fps", "fast"],
            &["--after", "soon"],
            &["--help"],
            &["-h"],
            &["--loud"],
        ] {
            let error = parse(&[], args).unwrap_err();
            assert!(error.contains("usage: errol"), "{args:?}: {error}");
        }
        assert!(
            parse(&[], &["--feed"])
                .unwrap_err()
                .starts_with("--feed needs a value")
        );
    }
}
