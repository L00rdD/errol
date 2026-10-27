//! The bar at the top: the AI's name, the mode, three status chips, the time.

use errol_domain::strategies::Scene;
use errol_domain::usecases::View;

use super::text;
use crate::look::fade;
use crate::render::{Anchor, Face, Gfx, Type};

const BRAND_FRIDAY: &str = "F.R.I.D.A.Y.";
const BRAND_FRIDAY_TYPE: Type = Type::new(Face::Display, 13.0).spaced(3.9);
const BRAND_CYBER: &str = "NETWATCH//ERROL";
const BRAND_CYBER_TYPE: Type = Type::new(Face::Bold, 19.0).spaced(1.5);

pub fn draw(gfx: &mut Gfx, view: &View) {
    let palette = gfx.palette;
    if gfx.cyber() {
        gfx.gradient_rect(
            0.0,
            0.0,
            800.0,
            38.0,
            fade(palette.accent, 0.16),
            fade(palette.accent, 0.0),
        );
        gfx.rect(0.0, 36.0, 800.0, 2.0, palette.accent);
        gfx.split_text(
            16.0,
            19.0,
            BRAND_CYBER,
            BRAND_CYBER_TYPE,
            palette.hot,
            Anchor::MIDDLE_LEFT,
            0.0,
        );
    } else {
        gfx.vertical_gradient(0.0, 0.0, 800.0, 38.0, palette.bg, fade(palette.bg, 0.0));
        gfx.rect(0.0, 37.0, 800.0, 1.0, fade(palette.accent, 0.12));
        gfx.glow_text(
            16.0,
            19.0,
            BRAND_FRIDAY,
            BRAND_FRIDAY_TYPE,
            palette.accent,
            Anchor::MIDDLE_LEFT,
            1.0,
        );
    }
    let brand_end =
        16.0 + if gfx.cyber() {
            gfx.measure(BRAND_CYBER, BRAND_CYBER_TYPE)
        } else {
            gfx.measure(BRAND_FRIDAY, BRAND_FRIDAY_TYPE)
        } + 12.0;
    let mode = match (&view.scene, gfx.cyber()) {
        (Scene::Idle, false) => "VEILLE",
        (Scene::Idle, true) => "STANDBY",
        (Scene::Event { .. }, false) => "ALERTE",
        (Scene::Event { .. }, true) => "INCOMING",
        (Scene::Cinema(_), false) => "PROJECTION",
        (Scene::Cinema(_), true) => "BRAINDANCE",
    };
    let blink = 0.35 + 0.65 * (gfx.t * std::f32::consts::PI).sin().abs();
    let dot = if view.warning.is_some() {
        palette.hot
    } else {
        palette.ok
    };
    gfx.halo(brand_end + 3.0, 19.0, 7.0, fade(dot, 0.5 * blink));
    gfx.disc(brand_end + 3.0, 19.0, 3.0, fade(dot, blink));
    gfx.text(
        brand_end + 12.0,
        19.5,
        mode,
        Type::new(Face::Strong, 11.0).spaced(1.8),
        palette.muted,
        Anchor::MIDDLE_LEFT,
    );

    gfx.text(
        784.0,
        19.0,
        &text::clock(view.now),
        Type::new(Face::Display, 13.0).spaced(1.0),
        palette.text,
        Anchor::MIDDLE_RIGHT,
    );

    let Some(snapshot) = &view.snapshot else {
        return;
    };
    let services = snapshot.services;
    let lamps = snapshot
        .house
        .as_ref()
        .map(|house| house.outlets.iter().filter(|outlet| outlet.on).count());
    let projecting = snapshot
        .house
        .as_ref()
        .is_some_and(|house| house.projectors.iter().any(|projector| projector.on));
    let chips = [
        (
            "SERVICES",
            if services.total > 0 {
                format!("{}/{}", services.up, services.total)
            } else {
                "–".into()
            },
            services.total > 0 && services.up == services.total,
        ),
        (
            "FLUX",
            snapshot.downloads.len().to_string(),
            !snapshot.downloads.is_empty(),
        ),
        (
            "MAISON",
            lamps.map_or("–".into(), |lamps| lamps.to_string()),
            lamps.is_some(),
        ),
    ];
    let mut x = 726.0;
    for (label, value, good) in chips.iter().rev() {
        let label_style = Type::new(Face::Strong, 10.5).spaced(1.4);
        let value_style = Type::new(Face::Bold, 11.5);
        // the projector, when on, is a small play triangle after the lamps
        let play = *label == "MAISON" && projecting;
        let width = 22.0
            + gfx.measure(label, label_style)
            + 6.0
            + gfx.measure(value, value_style)
            + 10.0
            + if play { 12.0 } else { 0.0 };
        x -= width + 8.0;
        if gfx.cyber() {
            let mut path = femtovg::Path::new();
            path.move_to(x + 6.0, 10.0);
            path.line_to(x + width, 10.0);
            path.line_to(x + width - 6.0, 28.0);
            path.line_to(x, 28.0);
            path.close();
            gfx.fill(&path, fade(palette.accent, 0.12));
        } else {
            gfx.rect_line(x, 10.0, width, 18.0, fade(palette.accent, 0.14), 1.0);
        }
        let light = if *good { palette.ok } else { palette.muted };
        gfx.rect(x + 9.0, 16.5, 5.0, 5.0, light);
        let label_width = gfx.text(
            x + 20.0,
            19.5,
            label,
            label_style,
            palette.muted,
            Anchor::MIDDLE_LEFT,
        );
        let value_width = gfx.text(
            x + 26.0 + label_width,
            19.5,
            value,
            value_style,
            palette.text,
            Anchor::MIDDLE_LEFT,
        );
        if play {
            let px = x + 32.0 + label_width + value_width;
            let mut triangle = femtovg::Path::new();
            triangle.move_to(px, 15.5);
            triangle.line_to(px + 7.0, 19.5);
            triangle.line_to(px, 23.5);
            triangle.close();
            gfx.fill(&triangle, palette.accent2);
        }
    }
}
