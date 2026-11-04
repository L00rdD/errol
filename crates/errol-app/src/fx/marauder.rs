//! Hogwarts' background: the Marauder's Map, after Pensieve's. The parchment
//! and the castle come baked (`render::Map`); over them, people walk about
//! with their names on ribbons and their footprints behind them, and the
//! viewer's own footsteps walk through the corridors from one room's door to
//! the next when the screen moves on.

use femtovg::Path;
use std::collections::VecDeque;

use crate::anim::unit;
use crate::look::fade;
use crate::render::{Gfx, ink};

const W: f32 = 800.0;
const H: f32 = 480.0;
/// Pixels a second for the people of the castle.
const PACE: f32 = 10.0;
/// Space between two footprints of a trail, and of a walk.
const STRIDE: f32 = 9.0;
const TRAIL: usize = 10;
/// A walk writes itself in this many seconds, whatever its length...
const WALK_TIME: f32 = 1.1;
/// ...fades from then, and is gone after this.
const WALK_FADE: f32 = 1.6;
const WALK_LIFE: f32 = 4.5;

struct Walker {
    name: &'static str,
    x: f32,
    y: f32,
    tx: f32,
    ty: f32,
    /// Distance walked since the last footprint.
    since: f32,
    /// Footprints left behind: x, y, heading, left foot.
    trail: VecDeque<(f32, f32, f32, bool)>,
}

pub struct Marauder {
    people: Vec<Walker>,
    /// The corners of the walk being written, and how long ago it started.
    walk: Option<(Vec<(f32, f32)>, f32)>,
    seed: u64,
}

impl Marauder {
    pub fn new() -> Self {
        let mut seed = 5;
        let mut rand = || {
            seed += 1;
            unit(seed, 7)
        };
        let people = ["Rusard", "Miss Teigne", "Peeves", "Albus Dumbledore"]
            .into_iter()
            .map(|name| Walker {
                name,
                x: rand() * W,
                y: rand() * H,
                tx: rand() * W,
                ty: rand() * H,
                since: 0.0,
                trail: VecDeque::new(),
            })
            .collect();
        Self {
            people,
            walk: None,
            seed,
        }
    }

    fn rand(&mut self) -> f32 {
        self.seed += 1;
        unit(self.seed, 7)
    }

    /// A scene starts: someone runs along the bottom corridor to see what is
    /// pinned there (`power` ≥ 1), or just crosses the map (less).
    pub fn burst(&mut self, power: f32) {
        if power >= 1.0 {
            self.walk = Some((vec![(-12.0, 441.0), (400.0, 441.0)], 0.0));
        } else {
            let (y0, y1) = (60.0 + self.rand() * 360.0, 60.0 + self.rand() * 360.0);
            let from_left = self.rand() < 0.5;
            let (x0, x1) = if from_left {
                (-12.0, 160.0 + self.rand() * 480.0)
            } else {
                (812.0, 160.0 + self.rand() * 480.0)
            };
            self.walk = Some((vec![(x0, y0), (x1, y0), (x1, y1)], 0.0));
        }
    }

    /// Footsteps from the door of `from` (the middle of its bottom wall),
    /// along its corridor, down the aisle beside `to`, to its door.
    /// Boxes are (x, y, w, h).
    pub fn walk(&mut self, from: (f32, f32, f32, f32), to: (f32, f32, f32, f32)) {
        let g = 9.0;
        let (ax, ay) = (from.0 + from.2 / 2.0, from.1 + from.3 + g);
        let (bx, by) = (to.0 + to.2 / 2.0, to.1 + to.3 + g);
        let vx = if bx > ax || (bx - ax).abs() < 4.0 {
            to.0 - g
        } else {
            to.0 + to.2 + g
        };
        let corners = if (ay - by).abs() < 4.0 {
            vec![(ax, ay), (bx, by)]
        } else {
            vec![(ax, ay), (vx, ay), (vx, by), (bx, by)]
        };
        self.walk = Some((corners, 0.0));
    }

    pub fn draw(&mut self, gfx: &mut Gfx, dt: f32) {
        if let Some(map) = gfx.map {
            gfx.image(map.parchment, 0.0, 0.0, W, H, 1.0);
        } else {
            gfx.rect(0.0, 0.0, W, H, ink::paper());
        }
        // the people of the castle
        for index in 0..self.people.len() {
            let next = (self.rand() * W, self.rand() * H);
            self.people[index].step(PACE * dt, next);
        }
        for person in &self.people {
            person.draw(gfx);
        }
        self.draw_walk(gfx, dt);
    }

    /// The viewer's own footsteps, until they are gone.
    fn draw_walk(&mut self, gfx: &mut Gfx, dt: f32) {
        let Some((corners, age)) = &mut self.walk else {
            return;
        };
        *age += dt;
        if *age > WALK_LIFE {
            self.walk = None;
            return;
        }
        for (x, y, heading, left, alpha) in footsteps(corners, *age) {
            footprint(gfx, x, y, heading, left, alpha, 0.85);
        }
    }
}

impl Walker {
    /// Walks `v` pixels towards its target, choosing `next` once there, and
    /// leaves a footprint every stride.
    fn step(&mut self, v: f32, next: (f32, f32)) {
        let (dx, dy) = (self.tx - self.x, self.ty - self.y);
        let dist = dx.hypot(dy);
        if dist < 10.0 {
            (self.tx, self.ty) = next;
        }
        if dist > 0.0 {
            self.x += dx / dist * v;
            self.y += dy / dist * v;
        }
        self.since += v;
        if self.since > STRIDE {
            self.since = 0.0;
            let left = self.trail.len().is_multiple_of(2);
            self.trail.push_back((self.x, self.y, dy.atan2(dx), left));
            if self.trail.len() > TRAIL {
                self.trail.pop_front();
            }
        }
    }

    /// Its trail, older prints fainter, and its name on a ribbon.
    fn draw(&self, gfx: &mut Gfx) {
        let count = self.trail.len().max(1) as f32;
        for (k, &(x, y, heading, left)) in self.trail.iter().enumerate() {
            let alpha = 0.08 + 0.5 * k as f32 / count;
            footprint(gfx, x, y, heading, left, alpha, 1.0);
        }
        gfx.ribbon(self.x, self.y + 7.0, self.name, 0.62);
    }
}

/// The footsteps along `corners` that show at `age`: written one after the
/// other over `WALK_TIME`, fading after `WALK_FADE`. Each is x, y, heading,
/// left foot, opacity.
fn footsteps(corners: &[(f32, f32)], age: f32) -> Vec<(f32, f32, f32, bool, f32)> {
    let total: f32 = corners
        .windows(2)
        .map(|pair| (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1))
        .sum();
    let steps = (total / STRIDE).max(2.0);
    let mut prints = Vec::new();
    let mut n = 0;
    for pair in corners.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        let len = (x1 - x0).hypot(y1 - y0);
        let heading = (y1 - y0).atan2(x1 - x0);
        let mut d = if n > 0 { STRIDE / 2.0 } else { 0.0 };
        while d < len {
            let born = age - n as f32 / steps * WALK_TIME;
            if born < 0.0 {
                return prints;
            }
            let alpha = 0.9 - (born - WALK_FADE).max(0.0) * 0.6;
            if alpha > 0.0 {
                prints.push((
                    x0 + (x1 - x0) * d / len,
                    y0 + (y1 - y0) * d / len,
                    heading,
                    n % 2 == 1,
                    alpha,
                ));
            }
            d += STRIDE;
            n += 1;
        }
    }
    prints
}

/// One footprint: the sole and the toe, heading `heading`, the left foot a
/// little to the left of the way.
fn footprint(gfx: &mut Gfx, x: f32, y: f32, heading: f32, left: bool, alpha: f32, scale: f32) {
    gfx.canvas.save();
    gfx.canvas.translate(x, y);
    gfx.canvas.rotate(heading);
    gfx.canvas.scale(scale, scale);
    gfx.canvas.translate(0.0, if left { -3.5 } else { 3.5 });
    let mut foot = Path::new();
    foot.ellipse(0.0, 0.0, 3.5, 1.8);
    foot.ellipse(-4.3, 0.0, 1.5, 1.4);
    gfx.fill(&foot, fade(ink::ink(), alpha));
    gfx.canvas.restore();
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORRIDOR: [(f32, f32); 3] = [(0.0, 0.0), (90.0, 0.0), (90.0, 45.0)];

    #[test]
    fn footsteps_write_themselves_one_after_the_other() {
        assert_eq!(footsteps(&CORRIDOR, 0.0).len(), 1);
        let half = footsteps(&CORRIDOR, WALK_TIME / 2.0);
        let all = footsteps(&CORRIDOR, WALK_TIME);
        assert!(half.len() > 1 && half.len() < all.len());
        assert_eq!(all.len(), 15);
    }

    #[test]
    fn footsteps_alternate_feet_and_turn_the_corner() {
        let all = footsteps(&CORRIDOR, WALK_TIME);
        assert!(!all[0].3 && all[1].3);
        assert!(all[0].2.abs() < f32::EPSILON);
        let last = all.last().expect("a print");
        assert!((last.2 - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!((last.0 - 90.0).abs() < 1e-4);
    }

    #[test]
    fn footsteps_fade_then_vanish() {
        assert!(footsteps(&CORRIDOR, WALK_FADE + 0.5)[0].4 < 0.9);
        assert_eq!(footsteps(&CORRIDOR, 4.4), Vec::new());
    }

    #[test]
    fn a_walker_heads_for_its_target_and_leaves_prints() {
        let mut walker = Walker {
            name: "Peeves",
            x: 0.0,
            y: 0.0,
            tx: 1000.0,
            ty: 0.0,
            since: 0.0,
            trail: VecDeque::new(),
        };
        for _ in 0..40 {
            walker.step(5.0, (0.0, 0.0));
        }
        assert!((walker.x - 200.0).abs() < 1e-3);
        assert_eq!(walker.trail.len(), TRAIL);
        walker.tx = 205.0;
        walker.step(5.0, (7.0, 8.0));
        assert_eq!((walker.tx, walker.ty), (7.0, 8.0));
    }

    #[test]
    fn a_scene_or_a_move_starts_a_walk() {
        let mut marauder = Marauder::new();
        marauder.burst(1.0);
        assert_eq!(marauder.walk.as_ref().map(|walk| walk.0.len()), Some(2));
        marauder.burst(0.5);
        assert_eq!(marauder.walk.as_ref().map(|walk| walk.0.len()), Some(3));
        marauder.walk((0.0, 0.0, 100.0, 50.0), (300.0, 0.0, 100.0, 50.0));
        assert_eq!(marauder.walk.as_ref().map(|walk| walk.0.len()), Some(2));
        marauder.walk((0.0, 0.0, 100.0, 50.0), (300.0, 200.0, 100.0, 50.0));
        assert_eq!(marauder.walk.as_ref().map(|walk| walk.0.len()), Some(4));
    }
}
