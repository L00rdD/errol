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
            let (tx, ty) = (self.rand() * W, self.rand() * H);
            let person = &mut self.people[index];
            let (dx, dy) = (person.tx - person.x, person.ty - person.y);
            let dist = dx.hypot(dy);
            if dist < 10.0 {
                (person.tx, person.ty) = (tx, ty);
            }
            let v = PACE * dt;
            if dist > 0.0 {
                person.x += dx / dist * v;
                person.y += dy / dist * v;
            }
            person.since += v;
            if person.since > STRIDE {
                person.since = 0.0;
                let left = person.trail.len().is_multiple_of(2);
                person
                    .trail
                    .push_back((person.x, person.y, dy.atan2(dx), left));
                if person.trail.len() > TRAIL {
                    person.trail.pop_front();
                }
            }
        }
        for person in &self.people {
            let count = person.trail.len().max(1) as f32;
            for (k, &(x, y, heading, left)) in person.trail.iter().enumerate() {
                let alpha = 0.08 + 0.5 * k as f32 / count;
                footprint(gfx, x, y, heading, left, alpha, 1.0);
            }
            gfx.ribbon(person.x, person.y + 7.0, person.name, 0.62);
        }
        // the viewer's own footsteps
        if let Some((corners, age)) = &mut self.walk {
            *age += dt;
            if *age > WALK_LIFE {
                self.walk = None;
            } else {
                let (corners, age) = (corners.clone(), *age);
                footsteps(gfx, &corners, age);
            }
        }
    }
}

/// The footsteps along `corners`, written one after the other over
/// `WALK_TIME`, fading after `WALK_FADE`.
fn footsteps(gfx: &mut Gfx, corners: &[(f32, f32)], age: f32) {
    let total: f32 = corners
        .windows(2)
        .map(|pair| (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1))
        .sum();
    let steps = (total / STRIDE).max(2.0);
    let mut n = 0;
    for pair in corners.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        let len = (x1 - x0).hypot(y1 - y0);
        let heading = (y1 - y0).atan2(x1 - x0);
        let mut d = if n > 0 { STRIDE / 2.0 } else { 0.0 };
        while d < len {
            let born = age - n as f32 / steps * WALK_TIME;
            if born < 0.0 {
                return;
            }
            let alpha = 0.9 - (born - WALK_FADE).max(0.0) * 0.6;
            if alpha > 0.0 {
                footprint(
                    gfx,
                    x0 + (x1 - x0) * d / len,
                    y0 + (y1 - y0) * d / len,
                    heading,
                    n % 2 == 1,
                    alpha,
                    0.85,
                );
            }
            d += STRIDE;
            n += 1;
        }
    }
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
