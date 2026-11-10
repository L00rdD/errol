//! Friday's background: a neural network. Nodes drift, neighbours are linked,
//! signals run along the links and light up the node they reach, which may
//! fire on. A burst fires from the centre of the scene. News lights a
//! cluster of neurons in its own colour, where it happens, and a ripple runs
//! from it; a lamp going out dims the whole network a moment. Rain and snow
//! drift through it as faint motes.

use errol_domain::entities::{EventKind, Sky};
use femtovg::{Color, Path};

use super::ambience::{self, Ambience, Fall, Tone};
use crate::anim::unit;
use crate::look::fade;
use crate::render::Gfx;

const NODES: usize = 64;
/// Two nodes closer than this are linked.
const REACH: f32 = 135.0;
const MAX_SIGNALS: usize = 140;
const W: f32 = 800.0;
const H: f32 = 480.0;
/// The ripple of news runs this many seconds; a dimming lasts `DIM`.
const ECHO: f32 = 1.6;
const DIM: f32 = 3.0;

struct Node {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r: f32,
    /// 1 when a signal just arrived, fading to 0.
    lit: f32,
    /// The tone of the news that lit it, if any.
    tone: Option<Tone>,
}

struct Signal {
    from: usize,
    to: usize,
    /// 0 → 1 along the link.
    k: f32,
    speed: f32,
    tone: Option<Tone>,
}

/// The ripple of a piece of news: where, how long ago, in which tone.
struct Echo {
    x: f32,
    y: f32,
    age: f32,
    tone: Tone,
}

pub struct Neural {
    nodes: Vec<Node>,
    links: Vec<(usize, usize, f32)>,
    signals: Vec<Signal>,
    heat: f32,
    /// Deterministic "random" draws.
    seed: u64,
    relink_in: f32,
    echo: Option<Echo>,
    pub ambience: Ambience,
}

impl Neural {
    pub fn new() -> Self {
        let nodes = (0..NODES as u64)
            .map(|i| Node {
                x: unit(i, 1) * W,
                y: unit(i, 2) * H,
                vx: (unit(i, 3) - 0.5) * 9.0,
                vy: (unit(i, 4) - 0.5) * 7.0,
                r: 1.0 + unit(i, 5) * 1.4,
                lit: 0.0,
                tone: None,
            })
            .collect();
        let mut neural = Self {
            nodes,
            links: Vec::new(),
            signals: Vec::new(),
            heat: 0.0,
            seed: 7,
            relink_in: 0.0,
            echo: None,
            ambience: Ambience::new(),
        };
        neural.relink();
        neural
    }

    fn rand(&mut self) -> f32 {
        self.seed += 1;
        unit(self.seed, 99)
    }

    fn relink(&mut self) {
        self.links.clear();
        for i in 0..self.nodes.len() {
            for j in i + 1..self.nodes.len() {
                let d =
                    (self.nodes[i].x - self.nodes[j].x).hypot(self.nodes[i].y - self.nodes[j].y);
                if d < REACH {
                    self.links.push((i, j, d));
                }
            }
        }
    }

    fn fire(&mut self, from: usize) {
        let out: Vec<usize> = self
            .links
            .iter()
            .filter_map(|&(a, b, _)| {
                if a == from {
                    Some(b)
                } else if b == from {
                    Some(a)
                } else {
                    None
                }
            })
            .collect();
        if out.is_empty() || self.signals.len() >= MAX_SIGNALS {
            return;
        }
        let to = out[(self.rand() * out.len() as f32) as usize % out.len()];
        let speed = 0.6 + self.rand() * 0.75;
        self.signals.push(Signal {
            from,
            to,
            k: 0.0,
            speed,
            tone: self.nodes[from].tone,
        });
    }

    pub fn burst(&mut self, power: f32) {
        self.heat = (self.heat + power).min(1.5);
        self.light_near((W * 0.55, H * 0.5), 6, None, 4);
    }

    /// News: a cluster of neurons lights up where it happens, in its tone,
    /// and fires a ripple of signals.
    pub fn react(&mut self, kind: &EventKind) {
        let tone = ambience::tone(kind);
        let (x, y) = ambience::place(kind);
        self.light_near((x, y), 5, Some(tone), 3);
        self.echo = Some(Echo {
            x,
            y,
            age: 0.0,
            tone,
        });
    }

    /// Lights the `count` nodes nearest `at` in `tone`, each firing `fires`
    /// signals.
    fn light_near(&mut self, at: (f32, f32), count: usize, tone: Option<Tone>, fires: usize) {
        let mut near: Vec<(f32, usize)> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| ((n.x - at.0).hypot(n.y - at.1), i))
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        for &(_, i) in near.iter().take(count) {
            self.nodes[i].lit = 1.0;
            self.nodes[i].tone = tone;
            for _ in 0..fires {
                self.fire(i);
            }
        }
    }

    pub fn draw(&mut self, gfx: &mut Gfx, real_dt: f32) {
        let dt = self.ambience.tick(real_dt);
        if let Some(fall) = fall(self.ambience.sky) {
            let alpha = if fall.length > 0.0 { 0.16 } else { 0.24 };
            ambience::draw_fall(gfx, fall, self.ambience.clock, gfx.palette.accent, alpha);
        }
        let heat = self.heat;
        self.advance(dt, heat);
        let palette = gfx.palette;
        let colours = ambience::tones(&palette, palette.accent2, palette.accent, palette.hot);
        self.draw_links(gfx, palette.accent, heat);
        let arrived = self.draw_signals(gfx, colours, dt, 1.0 + heat * 2.0);
        self.arrive(&arrived, heat);
        self.draw_nodes(gfx, colours);
        self.draw_echo(gfx, colours, real_dt);
    }

    /// `dt` seconds of the network's own time: it cools, drifts, relinks
    /// now and then and fires a few spontaneous signals a second, many more
    /// when hot. Nothing moves when `dt` is zero.
    fn advance(&mut self, dt: f32, heat: f32) {
        self.heat *= (-dt * 0.45).exp();
        self.drift(dt, 1.0 + heat * 3.0);
        self.relink_in -= dt;
        if self.relink_in <= 0.0 {
            self.relink_in = 0.4;
            self.relink();
        }
        if self.rand() < (3.5 + heat * 18.0) * dt {
            let from = (self.rand() * NODES as f32) as usize % NODES;
            self.fire(from);
        }
    }

    /// Signals reached these nodes: they light up in the signal's tone and
    /// may fire on.
    fn arrive(&mut self, arrived: &[(usize, Option<Tone>)], heat: f32) {
        for &(node, tone) in arrived {
            self.nodes[node].lit = 1.0;
            self.nodes[node].tone = tone;
            if self.rand() < 0.55 + heat * 0.4 {
                self.fire(node);
            }
        }
    }

    /// The ripple of the last news, a ring widening from where it happened
    /// over a warm halo; or, for a lamp going out, the network dimming.
    fn draw_echo(&mut self, gfx: &mut Gfx, colours: [Color; 5], dt: f32) {
        let Some(echo) = &mut self.echo else {
            return;
        };
        echo.age += dt;
        if echo.age > ECHO.max(DIM) {
            self.echo = None;
            return;
        }
        if echo.tone == Tone::Dim {
            let bg = gfx.palette.bg;
            gfx.rect(0.0, 0.0, W, H, fade(bg, dimming(echo.age)));
            return;
        }
        let (radius, alpha) = ripple(echo.age);
        if alpha <= 0.0 {
            return;
        }
        let colour = ambience::tone_colour(echo.tone, colours);
        gfx.halo(echo.x, echo.y, 110.0, fade(colour, alpha * 0.45));
        let mut ring = Path::new();
        ring.circle(echo.x, echo.y, radius);
        gfx.stroke(&ring, fade(colour, alpha), 1.4);
    }

    /// Nodes drift, `pace` times faster when hot, bounce off the edges, and dim.
    fn drift(&mut self, dt: f32, pace: f32) {
        for node in &mut self.nodes {
            node.x += node.vx * dt * pace;
            node.y += node.vy * dt * pace;
            if !(-10.0..=W + 10.0).contains(&node.x) {
                node.vx = -node.vx;
            }
            if !(-10.0..=H + 10.0).contains(&node.y) {
                node.vy = -node.vy;
            }
            node.lit *= (-dt * 1.8).exp();
            if node.lit < 0.15 {
                node.tone = None;
            }
        }
    }

    /// Links: four strokes, one per brightness level.
    fn draw_links(&self, gfx: &mut Gfx, accent: Color, heat: f32) {
        let mut levels: [Path; 4] = [Path::new(), Path::new(), Path::new(), Path::new()];
        for &(i, j, d) in &self.links {
            let lit = self.nodes[i].lit.max(self.nodes[j].lit);
            let path = &mut levels[link_level(d, lit, heat)];
            path.move_to(self.nodes[i].x, self.nodes[i].y);
            path.line_to(self.nodes[j].x, self.nodes[j].y);
        }
        for (level, path) in levels.iter().enumerate() {
            gfx.stroke(path, fade(accent, (level as f32 + 0.6) / 4.0 * 0.55), 1.0);
        }
    }

    /// Moves the signals along their links, `pace` times faster when hot,
    /// and draws them, each in its tone; returns the nodes they reached.
    fn draw_signals(
        &mut self,
        gfx: &mut Gfx,
        colours: [Color; 5],
        dt: f32,
        pace: f32,
    ) -> Vec<(usize, Option<Tone>)> {
        let accent = gfx.palette.accent;
        let mut arrived = Vec::new();
        let mut dots = Path::new();
        for signal in &mut self.signals {
            signal.k += signal.speed * dt * pace;
            let (a, b) = (&self.nodes[signal.from], &self.nodes[signal.to]);
            let (x, y) = (
                a.x + (b.x - a.x) * signal.k.min(1.0),
                a.y + (b.y - a.y) * signal.k.min(1.0),
            );
            dots.circle(x, y, 1.7);
            let colour = signal
                .tone
                .map_or(accent, |tone| ambience::tone_colour(tone, colours));
            gfx.halo(x, y, 7.0, fade(colour, 0.5));
            if signal.k >= 1.0 {
                arrived.push((signal.to, signal.tone));
            }
        }
        gfx.fill(&dots, Color::rgbaf(1.0, 1.0, 1.0, 0.92));
        self.signals.retain(|signal| signal.k < 1.0);
        arrived
    }

    /// Nodes, the lit ones with a halo in the tone of what lit them.
    fn draw_nodes(&self, gfx: &mut Gfx, colours: [Color; 5]) {
        let accent = gfx.palette.accent;
        let mut plain = Path::new();
        for node in &self.nodes {
            plain.circle(node.x, node.y, node.r);
        }
        gfx.fill(&plain, fade(accent, 0.5));
        for node in self.nodes.iter().filter(|node| node.lit > 0.15) {
            let colour = node
                .tone
                .map_or(accent, |tone| ambience::tone_colour(tone, colours));
            gfx.halo(
                node.x,
                node.y,
                16.0 * node.lit,
                fade(colour, 0.35 * node.lit),
            );
            gfx.disc(
                node.x,
                node.y,
                node.r + node.lit * 2.4,
                fade(colour, node.lit),
            );
        }
    }
}

/// Which of the four strokes a link `d` long goes in: brighter when short,
/// lit or hot.
fn link_level(d: f32, lit: f32, heat: f32) -> usize {
    let alpha = (1.0 - d / REACH) * (0.42 + 0.6 * lit + heat * 0.25);
    ((alpha * 4.0) as usize).min(3)
}

/// The ring of news `age` seconds old: its radius and opacity.
fn ripple(age: f32) -> (f32, f32) {
    (24.0 + age * 170.0, (1.0 - age / ECHO).max(0.0) * 0.5)
}

/// How dark the network is `age` seconds after a lamp went out: it darkens
/// quickly, then comes back slowly.
fn dimming(age: f32) -> f32 {
    let k = if age < 0.4 {
        age / 0.4
    } else {
        1.0 - (age - 0.4) / (DIM - 0.4)
    };
    k.clamp(0.0, 1.0) * 0.55
}

/// Faint motes for rain, slow dots for snow, nothing in a dry sky.
fn fall(sky: Option<Sky>) -> Option<Fall> {
    let (count, speed, length) = match sky? {
        Sky::Drizzle => (18, 60.0, 6.0),
        Sky::Rain => (32, 80.0, 8.0),
        Sky::Storm => (44, 110.0, 10.0),
        Sky::Snow => (40, 18.0, 0.0),
        _ => return None,
    };
    Some(Fall {
        count,
        speed,
        length,
        slant: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_lit_links_are_brightest() {
        assert_eq!(link_level(REACH, 0.0, 0.0), 0);
        assert_eq!(link_level(0.0, 0.0, 0.0), 1);
        assert_eq!(link_level(0.0, 1.0, 1.5), 3);
    }

    #[test]
    fn nodes_bounce_off_the_edges() {
        let mut neural = Neural::new();
        neural.nodes[0].x = W + 20.0;
        neural.nodes[0].y = -20.0;
        let (vx, vy) = (neural.nodes[0].vx, neural.nodes[0].vy);
        neural.nodes[0].lit = 1.0;
        neural.drift(1.0, 0.0);
        assert_eq!((neural.nodes[0].vx, neural.nodes[0].vy), (-vx, -vy));
        assert!(neural.nodes[0].lit < 0.2);
    }

    #[test]
    fn a_burst_fires_signals_from_the_centre() {
        let mut neural = Neural::new();
        neural.burst(1.0);
        assert!(!neural.signals.is_empty());
        assert!(neural.nodes.iter().filter(|node| node.lit > 0.99).count() >= 6);
    }

    #[test]
    fn news_lights_a_cluster_in_its_tone_where_it_happens() {
        let mut neural = Neural::new();
        neural.react(&EventKind::Light {
            name: "Salon".into(),
            on: true,
        });
        let warm: Vec<&Node> = neural
            .nodes
            .iter()
            .filter(|node| node.tone == Some(Tone::Warm))
            .collect();
        assert_eq!(warm.len(), 5);
        assert!(warm.iter().all(|node| node.lit > 0.99));
        assert!(neural.signals.iter().any(|s| s.tone == Some(Tone::Warm)));
        assert_eq!(neural.echo.as_ref().map(|echo| echo.tone), Some(Tone::Warm));
    }

    #[test]
    fn a_still_network_holds_and_a_live_one_moves() {
        let mut neural = Neural::new();
        let (x, signals) = (neural.nodes[0].x, neural.signals.len());
        neural.advance(0.0, 1.0);
        assert!((neural.nodes[0].x - x).abs() < f32::EPSILON);
        assert_eq!(neural.signals.len(), signals);
        for _ in 0..30 {
            neural.advance(0.5, 1.5);
        }
        assert!((neural.nodes[0].x - x).abs() > 1.0);
        assert!(!neural.signals.is_empty());
    }

    #[test]
    fn a_signal_lights_its_node_in_its_tone() {
        let mut neural = Neural::new();
        neural.arrive(&[(3, Some(Tone::Alert))], 1.5);
        assert!((neural.nodes[3].lit - 1.0).abs() < f32::EPSILON);
        assert_eq!(neural.nodes[3].tone, Some(Tone::Alert));
    }

    #[test]
    fn a_tone_fades_with_its_light() {
        let mut neural = Neural::new();
        neural.nodes[0].tone = Some(Tone::Good);
        neural.nodes[0].lit = 1.0;
        neural.drift(2.0, 0.0);
        assert_eq!(neural.nodes[0].tone, None);
    }

    #[test]
    fn the_ripple_widens_and_fades() {
        let (r0, a0) = ripple(0.0);
        let (r1, a1) = ripple(1.0);
        assert!(r1 > r0 && a1 < a0);
        assert!(ripple(ECHO).1.abs() < 1e-6);
    }

    #[test]
    fn a_lamp_out_dims_quickly_then_comes_back() {
        assert!(dimming(0.0).abs() < 1e-6);
        assert!((dimming(0.4) - 0.55).abs() < 1e-6);
        assert!(dimming(1.5) < 0.55 && dimming(1.5) > 0.0);
        assert!(dimming(DIM).abs() < 1e-6);
    }

    #[test]
    fn rain_and_snow_drift_dry_skies_stay_clear() {
        assert_eq!(fall(None), None);
        assert_eq!(fall(Some(Sky::Clear)), None);
        assert_eq!(fall(Some(Sky::Fog)), None);
        let drizzle = fall(Some(Sky::Drizzle)).expect("motes");
        let storm = fall(Some(Sky::Storm)).expect("motes");
        assert!(storm.count > drizzle.count);
        assert!(fall(Some(Sky::Rain)).is_some());
        assert!(fall(Some(Sky::Snow)).expect("dots").length.abs() < f32::EPSILON);
    }
}
