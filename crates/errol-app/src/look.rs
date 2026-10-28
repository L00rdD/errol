//! The three looks: friday (cyan and amber, a neural HUD), cyberpunk
//! (Netwatch red, yellow and cyan, braindance editor) and hogwarts (the
//! Marauder's Map: brown and red ink on warm parchment). Their colours come
//! from the house's `looks.json`, the reference the hub serves at
//! `/theme/looks.json` and Pensieve embeds too; `assets/looks.json` is a copy
//! of it, built into the binary.

use std::sync::OnceLock;

use femtovg::Color;
use serde::Deserialize;

const LOOKS: &str = include_str!("../../../assets/looks.json");

#[derive(Deserialize)]
struct LooksFile {
    looks: Vec<LookEntry>,
}

#[derive(Deserialize)]
struct LookEntry {
    id: String,
    colors: Colors,
}

#[derive(Deserialize)]
struct Colors {
    bg: String,
    accent: String,
    accent2: String,
    text: String,
    muted: String,
    hot: String,
    ok: String,
}

/// The palettes of `looks.json`, read once; the file is built in, so a bad
/// one is a bug caught by the tests, not something to recover from.
fn palettes() -> &'static [Palette; 3] {
    static PALETTES: OnceLock<[Palette; 3]> = OnceLock::new();
    PALETTES.get_or_init(|| {
        let file: LooksFile = serde_json::from_str(LOOKS).expect("assets/looks.json is valid");
        [Look::Friday, Look::Cyberpunk, Look::Hogwarts].map(|look| {
            let entry = file
                .looks
                .iter()
                .find(|entry| entry.id == look.id())
                .unwrap_or_else(|| panic!("assets/looks.json names {}", look.id()));
            let c = &entry.colors;
            Palette {
                bg: hex(&c.bg),
                accent: hex(&c.accent),
                accent2: hex(&c.accent2),
                text: hex(&c.text),
                muted: hex(&c.muted),
                hot: hex(&c.hot),
                ok: hex(&c.ok),
            }
        })
    })
}

/// "#6fe3ff" as a colour.
fn hex(text: &str) -> Color {
    let value = u32::from_str_radix(text.trim_start_matches('#'), 16)
        .unwrap_or_else(|error| panic!("assets/looks.json: bad colour {text:?}: {error}"));
    rgb(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Friday,
    Cyberpunk,
    Hogwarts,
}

impl Look {
    /// The look named `name`: its id in `looks.json` (what the feed and
    /// Pensieve use), or its name on the hub.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "friday" | "reactor" => Some(Self::Friday),
            "cyberpunk" => Some(Self::Cyberpunk),
            "hogwarts" => Some(Self::Hogwarts),
            _ => None,
        }
    }

    /// The look's id in `looks.json`.
    pub fn id(self) -> &'static str {
        match self {
            Self::Friday => "friday",
            Self::Cyberpunk => "cyberpunk",
            Self::Hogwarts => "hogwarts",
        }
    }

    /// The look to draw: the one the feed asks for (the user's remote control,
    /// set from the home page) when it names a known look, else `default`
    /// (`--look` or `ERROL_LOOK`).
    pub fn choose(asked: Option<&str>, default: Self) -> Self {
        asked.and_then(Self::parse).unwrap_or(default)
    }

    pub fn palette(self) -> Palette {
        palettes()[self as usize]
    }
}

/// Colours of a look.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    /// Titles, frames, links.
    pub accent: Color,
    /// The second colour: the seconds ring, highlights.
    pub accent2: Color,
    pub text: Color,
    pub muted: Color,
    /// Alerts in friday, the big yellow in cyberpunk, red ink in hogwarts.
    pub hot: Color,
    pub ok: Color,
}

pub fn rgb(hex: u32) -> Color {
    Color::rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// The colour with its alpha multiplied by `alpha`.
pub fn fade(color: Color, alpha: f32) -> Color {
    Color::rgbaf(color.r, color.g, color.b, color.a * alpha.clamp(0.0, 1.0))
}

/// Halfway colours, for gradients.
pub fn mix(a: Color, b: Color, k: f32) -> Color {
    let k = k.clamp(0.0, 1.0);
    Color::rgbaf(
        a.r + (b.r - a.r) * k,
        a.g + (b.g - a.g) * k,
        a.b + (b.b - a.b) * k,
        a.a + (b.a - a.a) * k,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_look_the_feed_asks_for_wins() {
        assert_eq!(Look::choose(Some("hogwarts"), Look::Friday), Look::Hogwarts);
    }

    #[test]
    fn every_look_has_its_palette_in_looks_json() {
        for look in [Look::Friday, Look::Cyberpunk, Look::Hogwarts] {
            let palette = look.palette();
            assert!(palette.bg != palette.accent, "{look:?}");
        }
        assert_eq!(Look::Hogwarts.palette().bg, rgb(0xf6dda6));
    }

    #[test]
    fn no_look_or_an_unknown_one_keeps_the_default() {
        assert_eq!(Look::choose(None, Look::Cyberpunk), Look::Cyberpunk);
        assert_eq!(
            Look::choose(Some("slytherin"), Look::Cyberpunk),
            Look::Cyberpunk
        );
    }
}
