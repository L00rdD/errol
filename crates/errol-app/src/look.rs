//! The two looks, with the colours of the hub's theme kit
//! (home/site/theme/theme.css): friday (Reactor's cyan and amber, a neural HUD)
//! and cyberpunk (Netwatch red, yellow and cyan, braindance editor).

use femtovg::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Friday,
    Cyberpunk,
}

impl Look {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "friday" | "reactor" => Some(Self::Friday),
            "cyberpunk" => Some(Self::Cyberpunk),
            _ => None,
        }
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
    /// Alerts in friday, the big yellow in cyberpunk.
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
