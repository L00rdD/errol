//! The three looks, with the colours of Pensieve's: friday (cyan and amber,
//! a neural HUD), cyberpunk (Netwatch red, yellow and cyan, braindance
//! editor) and hogwarts (the Marauder's Map: brown and red ink on warm
//! parchment).

use femtovg::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Friday,
    Cyberpunk,
    Hogwarts,
}

impl Look {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "friday" | "reactor" => Some(Self::Friday),
            "cyberpunk" => Some(Self::Cyberpunk),
            "hogwarts" => Some(Self::Hogwarts),
            _ => None,
        }
    }

    /// The look to draw: the one the feed asks for (the user's remote control,
    /// set from the home page) when it names a known look, else `default`
    /// (`--look` or `ERROL_LOOK`).
    pub fn choose(asked: Option<&str>, default: Self) -> Self {
        asked.and_then(Self::parse).unwrap_or(default)
    }

    pub fn palette(self) -> Palette {
        match self {
            Self::Friday => Palette {
                bg: rgb(0x02070d),
                accent: rgb(0x6fe3ff),
                accent2: rgb(0xffc857),
                text: rgb(0xd9f6ff),
                muted: rgb(0x7fa9b8),
                hot: rgb(0xff4b3e),
                ok: rgb(0x7dffb3),
            },
            Self::Cyberpunk => Palette {
                bg: rgb(0x07080d),
                accent: rgb(0xff6158),
                accent2: rgb(0x5ef6ff),
                text: rgb(0xf1e9e4),
                muted: rgb(0xa2726d),
                hot: rgb(0xf3e600),
                ok: rgb(0x39ff88),
            },
            Self::Hogwarts => Palette {
                bg: rgb(0xf6dda6),
                accent: rgb(0x3f210b),
                accent2: rgb(0x9c1f12),
                text: rgb(0x2a1608),
                muted: rgb(0x74471f),
                hot: rgb(0x9c1f12),
                ok: rgb(0x2d6a36),
            },
        }
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
    fn no_look_or_an_unknown_one_keeps_the_default() {
        assert_eq!(Look::choose(None, Look::Cyberpunk), Look::Cyberpunk);
        assert_eq!(
            Look::choose(Some("slytherin"), Look::Cyberpunk),
            Look::Cyberpunk
        );
    }
}
