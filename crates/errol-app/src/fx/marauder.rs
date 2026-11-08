//! Hogwarts' background: the Marauder's Map, after Pensieve's. The parchment
//! and the castle come baked (`render::Map`); over them, people walk about
//! with their names on ribbons and their footprints behind them, and the
//! viewer's own footsteps walk through the corridors from one room's door to
//! the next when the screen moves on. News stirs the castle: Filch hurries
//! to a lamp, Peeves spills ink, an owl crosses the map with what arrived,
//! Dumbledore comes to whoever waits. Rain and snow fall in faint ink.

use errol_domain::entities::{EventKind, Sky};
use femtovg::Path;
use std::collections::VecDeque;

use super::ambience::{self, Ambience, Fall};
use crate::anim::{ease_out, unit};
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
/// Someone sent by news hurries this many seconds, this many times faster.
const HURRY: f32 = 5.0;
const HURRY_PACE: f32 = 3.0;
/// The owl crosses the map in this many seconds.
const OWL_TIME: f32 = 6.0;
/// Spilt ink spreads, stays, then fades out by this many seconds.
const BLOT_LIFE: f32 = 5.0;
/// The corridor along the bottom of the map, below the cards.
const HALL: f32 = 441.0;
const FILCH: usize = 0;
const NORRIS: usize = 1;
const PEEVES: usize = 2;
const DUMBLEDORE: usize = 3;

/// What news does on the Map.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Happening {
    /// Someone of the castle hurries to where it happened.
    Hurry(usize),
    /// Peeves spills ink there, and comes to look.
    Blot,
    /// An owl crosses the map at that height, bringing what arrived.
    Owl,
}

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
    /// 1 when news sent them somewhere, wearing off over `HURRY` seconds.
    hurry: f32,
}

pub struct Marauder {
    people: Vec<Walker>,
    /// The corners of the walk being written, and how long ago it started.
    walk: Option<(Vec<(f32, f32)>, f32)>,
    seed: u64,
    /// Spilt ink: where, and how long ago.
    blot: Option<(f32, f32, f32)>,
    /// The owl: how long ago it set off.
    owl: Option<f32>,
    pub ambience: Ambience,
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
                hurry: 0.0,
            })
            .collect();
        Self {
            people,
            walk: None,
            seed,
            blot: None,
            owl: None,
            ambience: Ambience::new(),
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

    /// News: someone hurries there, ink is spilt, or an owl flies in.
    pub fn react(&mut self, kind: &EventKind) {
        let (x, y) = spot(ambience::place(kind));
        match happening(kind) {
            Happening::Hurry(index) => self.send(index, (x, y)),
            Happening::Blot => {
                self.blot = Some((x, y, 0.0));
                self.send(PEEVES, (x - 30.0, y));
            }
            Happening::Owl => self.owl = Some(0.0),
        }
    }

    /// Sends someone of the castle hurrying to `to`.
    fn send(&mut self, index: usize, to: (f32, f32)) {
        let person = &mut self.people[index];
        (person.tx, person.ty) = to;
        person.hurry = 1.0;
    }

    pub fn draw(&mut self, gfx: &mut Gfx, real_dt: f32) {
        let dt = self.ambience.tick(real_dt);
        if let Some(map) = gfx.map {
            gfx.image(map.parchment, 0.0, 0.0, W, H, 1.0);
        } else {
            gfx.rect(0.0, 0.0, W, H, ink::paper());
        }
        if let Some(fall) = fall(self.ambience.sky) {
            ambience::draw_fall(gfx, fall, self.ambience.clock, ink::ink(), 0.35);
        }
        self.draw_blot(gfx, real_dt);
        // the people of the castle, still when the background is
        for index in 0..self.people.len() {
            let next = (self.rand() * W, self.rand() * H);
            let person = &mut self.people[index];
            let pace = 1.0 + HURRY_PACE * person.hurry;
            person.hurry = (person.hurry - real_dt / HURRY).max(0.0);
            person.step(PACE * dt * pace, next);
        }
        for person in &self.people {
            person.draw(gfx);
        }
        self.draw_owl(gfx, real_dt);
        self.draw_walk(gfx, real_dt);
    }

    /// The ink Peeves spilt: a blot and its splashes, spreading then fading.
    fn draw_blot(&mut self, gfx: &mut Gfx, dt: f32) {
        let Some((x, y, age)) = &mut self.blot else {
            return;
        };
        *age += dt;
        let (x, y) = (*x, *y);
        let Some((size, alpha)) = blot(*age) else {
            self.blot = None;
            return;
        };
        let mut path = Path::new();
        path.ellipse(x, y, 11.0 * size, 8.0 * size);
        for k in 0..7u64 {
            let (angle, reach) = (
                unit(k, 41) * std::f32::consts::TAU,
                13.0 + unit(k, 42) * 12.0,
            );
            let r = (0.8 + unit(k, 43) * 2.4) * size;
            path.circle(
                x + angle.cos() * reach * size,
                y + angle.sin() * reach * size,
                r,
            );
        }
        gfx.fill(&path, fade(ink::ink(), alpha));
    }

    /// The owl, wings beating, its name on a ribbon trailing behind, along
    /// the bottom corridor.
    fn draw_owl(&mut self, gfx: &mut Gfx, dt: f32) {
        let Some(age) = &mut self.owl else {
            return;
        };
        *age += dt;
        let Some((x, y, flap)) = owl_at(*age) else {
            self.owl = None;
            return;
        };
        let mut wings = Path::new();
        wings.move_to(x - 13.0, y - 2.0 + flap * 7.0);
        wings.quad_to(x - 6.0, y - 6.0 - flap * 2.0, x, y);
        wings.quad_to(
            x + 6.0,
            y - 6.0 - flap * 2.0,
            x + 13.0,
            y - 2.0 + flap * 7.0,
        );
        gfx.stroke(&wings, fade(ink::ink(), 0.8), 1.8);
        let mut body = Path::new();
        body.ellipse(x, y + 2.5, 2.6, 4.0);
        gfx.fill(&body, fade(ink::ink(), 0.8));
        gfx.ribbon(x - 42.0, y - 8.0, "Errol", 0.62);
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

/// What each kind of news does on the Map: Filch hurries to a lamp,
/// Peeves spills ink by the cinema or the printer, an owl brings what
/// arrived, Dumbledore comes to whoever waits, Mrs Norris to the film.
fn happening(kind: &EventKind) -> Happening {
    match kind {
        EventKind::Light { .. } => Happening::Hurry(FILCH),
        EventKind::Cover { .. } | EventKind::Projector { .. } | EventKind::Print(_) => {
            Happening::Blot
        }
        EventKind::Download { .. } | EventKind::Import(_) | EventKind::Scan(_) => Happening::Owl,
        EventKind::Waiting { .. } => Happening::Hurry(DUMBLEDORE),
        EventKind::Play { .. } => Happening::Hurry(NORRIS),
    }
}

/// The blot `age` seconds after the spill: its size and opacity, `None`
/// once dry and gone.
fn blot(age: f32) -> Option<(f32, f32)> {
    if age >= BLOT_LIFE {
        return None;
    }
    let size = ease_out(age / 0.5);
    let alpha = 0.6 * ((BLOT_LIFE - age) / 2.0).min(1.0);
    Some((size, alpha))
}

/// Where the owl is `age` seconds after setting off, bobbing along the
/// bottom corridor, and how its wings are (-1 up, 1 down); `None` once
/// across.
fn owl_at(age: f32) -> Option<(f32, f32, f32)> {
    if age >= OWL_TIME {
        return None;
    }
    let x = -30.0 + (W + 90.0) * age / OWL_TIME;
    Some((x, HALL + (age * 1.4).sin() * 3.0, (age * 9.0).sin()))
}

/// Where news happens on the Map: the cards hide the right of the screen,
/// so what happens there happens in the bottom corridor below.
fn spot((x, y): (f32, f32)) -> (f32, f32) {
    if x > 280.0 { (x, HALL) } else { (x, y) }
}

/// Faint ink drops for rain, dots for snow.
fn fall(sky: Option<Sky>) -> Option<Fall> {
    let (count, speed, length, slant) = match sky? {
        Sky::Drizzle => (20, 110.0, 5.0, 0.1),
        Sky::Rain => (40, 140.0, 7.0, 0.12),
        Sky::Storm => (60, 180.0, 9.0, 0.2),
        Sky::Snow => (45, 22.0, 0.0, 0.0),
        _ => return None,
    };
    Some(Fall {
        count,
        speed,
        length,
        slant,
    })
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
            hurry: 0.0,
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

    #[test]
    fn a_lamp_sends_filch_hurrying_there() {
        let mut marauder = Marauder::new();
        let lamp = EventKind::Light {
            name: "Salon".into(),
            on: false,
        };
        marauder.react(&lamp);
        let filch = &marauder.people[FILCH];
        assert_eq!((filch.tx, filch.ty), spot(ambience::place(&lamp)));
        assert!((filch.hurry - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn the_cinema_spills_ink_and_arrivals_bring_an_owl() {
        let mut marauder = Marauder::new();
        marauder.react(&EventKind::Cover {
            id: "c".into(),
            name: "Écran".into(),
            down: true,
        });
        assert!(marauder.blot.is_some());
        assert!(marauder.people[PEEVES].hurry > 0.0);
        marauder.react(&EventKind::Download {
            name: "x".into(),
            size: None,
        });
        assert!(marauder.owl.is_some());
    }

    #[test]
    fn everyone_has_their_news() {
        let waiting = EventKind::Waiting {
            name: "errol".into(),
            project: None,
            room: None,
        };
        assert_eq!(happening(&waiting), Happening::Hurry(DUMBLEDORE));
        let play = EventKind::Play { title: "x".into() };
        assert_eq!(happening(&play), Happening::Hurry(NORRIS));
        let projector = EventKind::Projector {
            name: "p".into(),
            on: true,
            app: None,
        };
        assert_eq!(happening(&projector), Happening::Blot);
    }

    #[test]
    fn ink_spreads_then_dries_away() {
        let (s0, a0) = blot(0.0).expect("a blot");
        let (s1, a1) = blot(1.0).expect("a blot");
        assert!(s0 < 0.01 && (s1 - 1.0).abs() < 1e-6);
        assert!((a0 - a1).abs() < 1e-6);
        assert!(blot(4.0).expect("a blot").1 < a1);
        assert_eq!(blot(BLOT_LIFE), None);
    }

    #[test]
    fn the_owl_crosses_the_map_beating_its_wings() {
        let (x0, _, _) = owl_at(0.0).expect("an owl");
        let (x1, y1, flap) = owl_at(OWL_TIME / 2.0).expect("an owl");
        assert!(x0 < 0.0 && x1 > 300.0 && x1 < 500.0);
        assert!((y1 - HALL).abs() <= 3.0 && flap.abs() <= 1.0);
        assert_eq!(owl_at(OWL_TIME), None);
    }

    #[test]
    fn what_happens_behind_the_cards_happens_in_the_corridor() {
        assert_eq!(spot((190.0, 300.0)), (190.0, 300.0));
        assert_eq!(spot((630.0, 130.0)), (630.0, HALL));
    }

    #[test]
    fn rain_and_snow_fall_in_ink() {
        assert_eq!(fall(None), None);
        assert_eq!(fall(Some(Sky::Cloudy)), None);
        assert!(
            fall(Some(Sky::Storm)).expect("rain").count
                > fall(Some(Sky::Drizzle)).expect("rain").count
        );
        assert!(fall(Some(Sky::Rain)).is_some());
        assert!(fall(Some(Sky::Snow)).expect("snow").length.abs() < f32::EPSILON);
    }
}
