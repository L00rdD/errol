//! Friday's background: a neural network. Nodes drift, neighbours are linked,
//! signals run along the links and light up the node they reach, which may
//! fire on. A burst fires from the centre of the scene.

use femtovg::{Color, Path};

use crate::anim::unit;
use crate::look::fade;
use crate::render::Gfx;

const NODES: usize = 64;
/// Two nodes closer than this are linked.
const REACH: f32 = 135.0;
const MAX_SIGNALS: usize = 140;
const W: f32 = 800.0;
const H: f32 = 480.0;

struct Node {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r: f32,
    /// 1 when a signal just arrived, fading to 0.
    lit: f32,
}

struct Signal {
    from: usize,
    to: usize,
    /// 0 → 1 along the link.
    k: f32,
    speed: f32,
}

pub struct Neural {
    nodes: Vec<Node>,
    links: Vec<(usize, usize, f32)>,
    signals: Vec<Signal>,
    heat: f32,
    /// Deterministic "random" draws.
    seed: u64,
    relink_in: f32,
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
            })
            .collect();
        let mut neural = Self {
            nodes,
            links: Vec::new(),
            signals: Vec::new(),
            heat: 0.0,
            seed: 7,
            relink_in: 0.0,
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
        });
    }

    pub fn burst(&mut self, power: f32) {
        self.heat = (self.heat + power).min(1.5);
        let mut near: Vec<(f32, usize)> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| ((n.x - W * 0.55).hypot(n.y - H * 0.5), i))
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        for &(_, i) in near.iter().take(6) {
            self.nodes[i].lit = 1.0;
            for _ in 0..4 {
                self.fire(i);
            }
        }
    }

    pub fn draw(&mut self, gfx: &mut Gfx, dt: f32) {
        let heat = self.heat;
        self.heat *= (-dt * 0.45).exp();
        let pace = 1.0 + heat * 3.0;
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
        }
        self.relink_in -= dt;
        if self.relink_in <= 0.0 {
            self.relink_in = 0.4;
            self.relink();
        }
        // a few spontaneous signals a second, many more when hot
        if self.rand() < (3.5 + heat * 18.0) * dt {
            let from = (self.rand() * NODES as f32) as usize % NODES;
            self.fire(from);
        }
        let accent = gfx.palette.accent;

        // links: four strokes, one per brightness level
        let mut levels: [Path; 4] = [Path::new(), Path::new(), Path::new(), Path::new()];
        for &(i, j, d) in &self.links {
            let lit = self.nodes[i].lit.max(self.nodes[j].lit);
            let alpha = (1.0 - d / REACH) * (0.42 + 0.6 * lit + heat * 0.25);
            let path = &mut levels[((alpha * 4.0) as usize).min(3)];
            path.move_to(self.nodes[i].x, self.nodes[i].y);
            path.line_to(self.nodes[j].x, self.nodes[j].y);
        }
        for (level, path) in levels.iter().enumerate() {
            gfx.stroke(path, fade(accent, (level as f32 + 0.6) / 4.0 * 0.55), 1.0);
        }

        // signals
        let mut arrived = Vec::new();
        let mut dots = Path::new();
        for signal in &mut self.signals {
            signal.k += signal.speed * dt * (1.0 + heat * 2.0);
            let (a, b) = (&self.nodes[signal.from], &self.nodes[signal.to]);
            let (x, y) = (
                a.x + (b.x - a.x) * signal.k.min(1.0),
                a.y + (b.y - a.y) * signal.k.min(1.0),
            );
            dots.circle(x, y, 1.7);
            gfx.halo(x, y, 7.0, fade(accent, 0.5));
            if signal.k >= 1.0 {
                arrived.push(signal.to);
            }
        }
        gfx.fill(&dots, Color::rgbaf(1.0, 1.0, 1.0, 0.92));
        self.signals.retain(|signal| signal.k < 1.0);
        for node in arrived {
            self.nodes[node].lit = 1.0;
            if self.rand() < 0.55 + heat * 0.4 {
                self.fire(node);
            }
        }

        // nodes
        let mut plain = Path::new();
        for node in &self.nodes {
            plain.circle(node.x, node.y, node.r);
        }
        gfx.fill(&plain, fade(accent, 0.5));
        for node in self.nodes.iter().filter(|node| node.lit > 0.15) {
            gfx.halo(
                node.x,
                node.y,
                16.0 * node.lit,
                fade(accent, 0.35 * node.lit),
            );
            gfx.disc(
                node.x,
                node.y,
                node.r + node.lit * 2.4,
                fade(accent, node.lit),
            );
        }
    }
}
