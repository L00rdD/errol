//! Command line and environment. The service on the Pi sets the environment
//! (`ERROL_LOOK`…), a preview on the Mac uses the flags.

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
}

const USAGE: &str = "usage: errol [--look friday|cyberpunk|hogwarts] [--demo [all|idle|light|cover|projector|cinema|import|download|scan]]
                [--feed URL] [--jellyfin URL] [--window] [--fps N] [--shot FILE.png [--after SECS]]
environment: ERROL_LOOK, ERROL_FEED, ERROL_JELLYFIN, ERROL_FPS";

impl Options {
    pub fn from_env_and_args(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let env = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        let mut options = Self {
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
        };
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            let mut value = |name: &str| {
                args.next()
                    .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
            };
            match arg.as_str() {
                "--look" => options.look = Look::parse(&value("--look")?).ok_or(USAGE)?,
                "--demo" => {
                    let scene = args
                        .next_if(|next| !next.starts_with("--"))
                        .unwrap_or_default();
                    options.demo = Some(DemoScene::parse(&scene).ok_or(USAGE)?);
                }
                "--feed" => options.feed_url = value("--feed")?,
                "--jellyfin" => options.jellyfin_url = value("--jellyfin")?,
                "--window" => options.windowed = true,
                "--fps" => options.fps = value("--fps")?.parse().map_err(|_| USAGE)?,
                "--shot" => options.shot = Some(value("--shot")?),
                "--after" => options.shot_after = value("--after")?.parse().map_err(|_| USAGE)?,
                "-h" | "--help" => return Err(USAGE.into()),
                other => return Err(format!("unknown argument {other}\n{USAGE}")),
            }
        }
        options.fps = options.fps.clamp(5, 60);
        Ok(options)
    }
}
