//! Cerebro's X-Mansion: a session's room and its X-Men, drawn as Cerebro's
//! panel draws them (github.com/L00rdD/cerebro, `ui/src/lib/scene`), ported
//! line by line: the same wandering, the same powers, the same emotes, a pure
//! function of (actor, time) on the wall clock, ten frames a second, so the
//! screen and the Mac show the same instant. Around it, Cerebro's session card
//! (`SessionCard.svelte`): the status dot and the name, the room, the cast, the
//! folder and the status. Its colours and its monospace, in every look.

// the powers keep the original's short names (p, e, x, y…) to read side by side with it
#![allow(clippy::many_single_char_names)]

use std::f64::consts::PI;

use errol_domain::entities::{Actor, ActorState, Cerebro, Session, SessionScene, SessionStatus};
use femtovg::{Color, ImageId, Paint, Path};

use crate::look::{fade, rgb};
use crate::render::{Anchor, Face, Finish, Gfx, Textures, Type};

/// Characters are drawn larger than the rooms' scale so they read at a glance.
const CHARACTER_SCALE: f64 = 1.5;
const EMOTE_SIZE: f64 = 9.0;
/// Width asked of a sprite sheet: more than any, so it is never resized.
const FULL: u32 = 4096;

// Cerebro's colours (ui/src/styles.css)
const CARD: u32 = 0x1f1f26;
const TEXT: u32 = 0xececf1;
const MUTED: u32 = 0x9a9aa8;
const BUSY: u32 = 0x3fd07f;
const IDLE: u32 = 0x8e8e99;
const WAITING: u32 = 0xffd23f;

// ---------- wander.ts ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Facing {
    Front,
    Back,
    Side,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Movement {
    Walk,
    Fly,
    Teleport,
    Phase,
    Dash,
}

impl Movement {
    fn parse(text: &str) -> Self {
        match text {
            "fly" => Self::Fly,
            "teleport" => Self::Teleport,
            "phase" => Self::Phase,
            "dash" => Self::Dash,
            _ => Self::Walk,
        }
    }

    /// `pace` scales the leg, `share` is the part of it spent moving.
    fn timing(self) -> (f64, f64) {
        match self {
            Self::Walk | Self::Phase => (1.0, 0.6),
            Self::Fly => (1.1, 0.55),
            Self::Teleport => (1.0, 0.3),
            Self::Dash => (0.8, 0.2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Pose {
    x: f64,
    y: f64,
    facing: Facing,
    flip: bool,
    frame: u32,
    lift: f64,
    alpha: f64,
    /// from, to, progress
    travel: Option<(Point, Point, f64)>,
    /// leg, progress
    cast: Option<(f64, f64)>,
}

const WALK_CYCLE: [u32; 4] = [1, 2, 3, 2];
const CAST_CHANCE: f64 = 0.75;

/// JavaScript's `ToInt32`, as bits: what `Math.imul` and `|` make of a number.
fn int32(n: f64) -> u32 {
    n.rem_euclid(4_294_967_296.0) as u32
}

/// `Math.round`: halves go up, also below zero.
fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// 32-bit FNV-1a with a murmur3 finalizer, over UTF-16 code units.
fn hash32(text: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for unit in text.encode_utf16() {
        h ^= u32::from(unit);
        h = h.wrapping_mul(0x0100_0193);
    }
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}

/// Number in [0, 1) derived from (seed, n).
fn unit(seed: u32, n: f64) -> f64 {
    let mut t = seed.wrapping_add(int32(n).wrapping_mul(0x9e37_79b9));
    t = (t ^ (t >> 15)).wrapping_mul(t | 1);
    t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 0x3d));
    f64::from(t ^ (t >> 14)) / 4_294_967_296.0
}

fn hover(seed: u32, seconds: f64) -> f64 {
    8.0 + round((seconds * 3.0 + f64::from(seed % 7)).sin() * 2.0)
}

/// Walk area: x, y, width, height.
type Area = (f64, f64, f64, f64);

fn pose_at(actor: &str, area: Area, seconds: f64, walking: bool, movement: Movement) -> Pose {
    let seed = hash32(actor);
    let point = |k: f64| Point {
        x: area.0 + unit(seed, k * 2.0) * area.2,
        y: area.1 + unit(seed, k * 2.0 + 1.0) * area.3,
    };
    let still = |at: Point, flip: bool| Pose {
        x: at.x.floor(),
        y: at.y.floor(),
        facing: Facing::Front,
        flip,
        frame: 0,
        lift: 0.0,
        alpha: 1.0,
        travel: None,
        cast: None,
    };
    if !walking {
        return still(point(0.0), false);
    }
    let (pace, share) = movement.timing();
    let leg_seconds = (3.0 + unit(seed, 999.0) * 2.5) * pace;
    let clock = seconds / leg_seconds + unit(seed, 998.0) * 100.0;
    let k = clock.floor();
    let progress = clock - k;
    let (from, to) = (point(k), point(k + 1.0));
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let flying = movement == Movement::Fly;

    if progress >= share {
        return Pose {
            lift: if flying { hover(seed, seconds) } else { 0.0 },
            cast: (unit(seed, 5000.0 + k) < CAST_CHANCE)
                .then(|| (k, (progress - share) / (1.0 - share))),
            ..still(to, dx < 0.0)
        };
    }
    let u = progress / share;
    let travel = Some((from, to, u));
    if movement == Movement::Teleport {
        let at = if u < 0.5 { from } else { to };
        let alpha = if u < 0.3 {
            1.0 - u / 0.3
        } else if u > 0.7 {
            (u - 0.7) / 0.3
        } else {
            0.0
        };
        return Pose {
            alpha,
            travel,
            ..still(at, dx < 0.0)
        };
    }
    let steps = if movement == Movement::Dash {
        16.0
    } else {
        8.0
    };
    // the floor is seen at an angle: vertical moves count double
    let vertical = dy.abs() * 2.0 > dx.abs();
    Pose {
        x: (from.x + dx * u).floor(),
        y: (from.y + dy * u).floor(),
        facing: if !vertical {
            Facing::Side
        } else if dy < 0.0 {
            Facing::Back
        } else {
            Facing::Front
        },
        flip: dx < 0.0,
        frame: if flying {
            0
        } else {
            WALK_CYCLE[(seconds * steps).floor() as usize % WALK_CYCLE.len()]
        },
        lift: if flying {
            hover(seed, seconds) + 3.0
        } else {
            0.0
        },
        alpha: if movement == Movement::Phase {
            0.45
        } else {
            1.0
        },
        travel,
        cast: None,
    }
}

// ---------- pixels.ts: 1px rects, gathered by colour ----------

/// Draws in the room's pixels. Rects of one colour go into one path, filled
/// when the colour changes or a picture comes; `alpha` is the canvas'
/// `globalAlpha` of the original.
struct Px<'g, 'a> {
    gfx: &'g mut Gfx<'a>,
    alpha: f64,
    fill: Color,
    path: Path,
    pending: Option<Color>,
}

impl<'g, 'a> Px<'g, 'a> {
    fn new(gfx: &'g mut Gfx<'a>) -> Self {
        Self {
            gfx,
            alpha: 1.0,
            fill: Color::white(),
            path: Path::new(),
            pending: None,
        }
    }

    fn flush(&mut self) {
        if let Some(colour) = self.pending.take() {
            let path = std::mem::replace(&mut self.path, Path::new());
            let paint = Paint::color(colour).with_anti_alias(false);
            self.gfx.canvas.fill_path(&path, &paint);
        }
    }

    fn style(&mut self, colour: Color) {
        self.fill = colour;
    }

    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        if w <= 0.0 || h <= 0.0 || self.alpha <= 0.0 {
            return;
        }
        let colour = fade(self.fill, self.alpha as f32);
        if self.pending != Some(colour) {
            self.flush();
            self.pending = Some(colour);
        }
        self.path.rect(x as f32, y as f32, w as f32, h as f32);
    }

    fn disc(&mut self, cx: f64, cy: f64, radius: f64, colour: Color) {
        self.style(colour);
        let mut dy = -radius;
        while dy <= radius {
            let half = round((radius * radius - dy * dy).sqrt());
            self.rect(cx - half, cy + dy, half * 2.0 + 1.0, 1.0);
            dy += 1.0;
        }
    }

    fn ring(&mut self, cx: f64, cy: f64, radius: f64, colour: Color, squash: f64) {
        self.style(colour);
        let steps = round(PI * 2.0 * radius).max(8.0) as u32;
        for i in 0..steps {
            let angle = f64::from(i) / f64::from(steps) * PI * 2.0;
            self.rect(
                round(cx + angle.cos() * radius),
                round(cy + angle.sin() * radius * squash),
                1.0,
                1.0,
            );
        }
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, colour: Color) {
        self.style(colour);
        let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1.0);
        let mut i = 0.0;
        while i <= steps {
            let t = i / steps;
            self.rect(
                round(x0 + (x1 - x0) * t),
                round(y0 + (y1 - y0) * t),
                1.0,
                1.0,
            );
            i += 1.0;
        }
    }

    fn burst(&mut self, cx: f64, cy: f64, radius: f64, colour: Color) {
        self.style(colour);
        let rays = round(radius * 2.0).max(8.0) as u32;
        for i in 0..rays {
            let angle = f64::from(i) / f64::from(rays) * PI * 2.0;
            self.rect(
                round(cx + angle.cos() * radius),
                round(cy + angle.sin() * radius),
                1.0,
                1.0,
            );
        }
    }

    /// Part of a picture at (dx, dy), `dw` × `dh`, nearest pixels.
    #[allow(clippy::too_many_arguments)]
    fn image(
        &mut self,
        image: ImageId,
        (sx, sy, sw, sh): (f64, f64, f64, f64),
        (dx, dy, dw, dh): (f64, f64, f64, f64),
        alpha: f64,
    ) {
        self.flush();
        let Ok((width, height)) = self.gfx.canvas.image_size(image) else {
            return;
        };
        let (kx, ky) = (dw / sw, dh / sh);
        let paint = Paint::image(
            image,
            (dx - sx * kx) as f32,
            (dy - sy * ky) as f32,
            (width as f64 * kx) as f32,
            (height as f64 * ky) as f32,
            0.0,
            alpha.clamp(0.0, 1.0) as f32,
        )
        .with_anti_alias(false);
        let mut path = Path::new();
        path.rect(dx as f32, dy as f32, dw as f32, dh as f32);
        self.gfx.canvas.fill_path(&path, &paint);
    }
}

const WHITE: u32 = 0xffffff;

fn hex(text: &str) -> Color {
    let digits = text.trim_start_matches('#');
    u32::from_str_radix(digits, 16).map_or(Color::white(), |value| {
        if digits.len() == 3 {
            let (r, g, b) = ((value >> 8) & 0xf, (value >> 4) & 0xf, value & 0xf);
            rgb(((r * 17) << 16) | ((g * 17) << 8) | (b * 17))
        } else {
            rgb(value)
        }
    })
}

// ---------- renderer.ts ----------

/// A character's pictures and manifest, ready to draw.
struct Cast<'s> {
    actor: &'s Actor,
    image: ImageId,
    movement: Movement,
    pose: Pose,
}

/// Everything a scene needs, loaded: the room, the emotes, the cast.
struct Sheets {
    room: ImageId,
    emotes: ImageId,
    room_size: (f64, f64),
    character: (f64, f64),
}

/// Draws `scene` with its top left corner at (x, y), each room pixel
/// `scale` screen pixels; false while its pictures load (nothing is drawn).
#[allow(clippy::too_many_arguments)]
pub fn draw_scene(
    gfx: &mut Gfx,
    textures: &mut Textures,
    cerebro: &Cerebro,
    scene: &SessionScene,
    x: f32,
    y: f32,
    scale: f32,
    now_ms: i64,
) -> bool {
    let room = textures.get(gfx.canvas, &scene.room, FULL, Finish::Pixel);
    let emotes = cerebro
        .emotes
        .as_ref()
        .and_then(|emotes| textures.get(gfx.canvas, emotes, FULL, Finish::Pixel));
    let (Some(room), Some(emotes)) = (room, emotes) else {
        return false;
    };
    let images: Vec<Option<ImageId>> = scene
        .actors
        .iter()
        .map(|actor| textures.get(gfx.canvas, &actor.sprite, FULL, Finish::Pixel))
        .collect();
    let sheets = Sheets {
        room,
        emotes,
        room_size: (
            f64::from(cerebro.room_size.0),
            f64::from(cerebro.room_size.1),
        ),
        character: (
            f64::from(cerebro.character_size.0),
            f64::from(cerebro.character_size.1),
        ),
    };
    // Cerebro's ticker: ten frames a second, on the wall clock
    let seconds = (now_ms as f64 / 100.0).floor() / 10.0;
    let (width, height) = sheets.room_size;
    gfx.canvas.save();
    gfx.canvas.translate(x, y);
    gfx.canvas.scale(scale, scale);
    gfx.canvas
        .intersect_scissor(0.0, 0.0, width as f32, height as f32);
    let mut px = Px::new(gfx);

    let frame = if scene.frames > 1 {
        (seconds * f64::from(scene.fps)).floor() % f64::from(scene.frames)
    } else {
        0.0
    };
    px.image(
        sheets.room,
        (frame * width, 0.0, width, height),
        (0.0, 0.0, width, height),
        1.0,
    );

    let area = (
        f64::from(scene.walk.0),
        f64::from(scene.walk.1),
        f64::from(scene.walk.2),
        f64::from(scene.walk.3),
    );
    // back to front, so characters lower on screen overlap those behind them
    let mut cast: Vec<Cast> = scene
        .actors
        .iter()
        .zip(images)
        .filter_map(|(actor, image)| {
            let movement = Movement::parse(&actor.movement);
            Some(Cast {
                actor,
                image: image?,
                movement,
                pose: pose_at(
                    &actor.id,
                    area,
                    seconds,
                    actor.state == ActorState::Walking,
                    movement,
                ),
            })
        })
        .collect();
    cast.sort_by(|a, b| a.pose.y.total_cmp(&b.pose.y));
    for member in &cast {
        draw_actor(&mut px, &sheets, member, seconds);
    }
    px.flush();
    gfx.canvas.restore();
    true
}

const AIMED: [&str; 4] = ["beam", "claws", "throw", "sonic"];

fn draw_actor(px: &mut Px, sheets: &Sheets, member: &Cast, seconds: f64) {
    let actor = member.actor;
    let size = (
        sheets.character.0 * CHARACTER_SCALE,
        sheets.character.1 * CHARACTER_SCALE,
    );
    let mut pose = member.pose;
    // aimed powers are used side-on, towards the last direction walked
    if pose.cast.is_some() && AIMED.contains(&actor.power.as_str()) {
        pose.facing = Facing::Side;
    }
    let (dx, lift_offset) = cast_offset(&actor.power, &pose, seconds);
    let x = pose.x + dx;
    let lift = pose.lift + lift_offset;
    let top = pose.y - lift - size.1 + 1.0;
    let sprite = Sprite {
        image: member.image,
        frame: sheets.character,
        size,
        pose,
    };

    // the shadow stays on the floor and shrinks as the character rises
    px.alpha = pose.alpha;
    px.style(Color::rgbaf(20.0 / 255.0, 12.0 / 255.0, 30.0 / 255.0, 0.28));
    let shadow = if lift > 0.0 { 12.0 } else { 18.0 };
    px.rect(x - shadow / 2.0, pose.y, shadow, 2.0);
    px.rect(x - shadow / 2.0 + 2.0, pose.y + 2.0, shadow - 4.0, 1.0);

    if let Some(aura) = &actor.aura {
        px.alpha = pose.alpha * (0.18 + 0.08 * (seconds * 3.0 + pose.x).sin());
        px.disc(x, pose.y - lift - 15.0, 16.0, hex(aura));
    }
    px.alpha = 1.0;

    if let Some((from, to, progress)) = pose.travel.filter(|_| member.movement == Movement::Dash) {
        // super speed: fading copies left behind along the path
        for i in (1..=3).rev() {
            let u = progress - f64::from(i) * 0.08;
            if u <= 0.0 {
                continue;
            }
            sprite.draw(
                px,
                (from.x + (to.x - from.x) * u).floor(),
                (from.y + (to.y - from.y) * u).floor(),
                0.4 / f64::from(i),
            );
        }
    }

    let asleep = actor.state == ActorState::Asleep;
    sprite.draw(
        px,
        x,
        pose.y - lift,
        pose.alpha * if asleep { 0.75 } else { 1.0 },
    );

    if let Some(travel) = pose
        .travel
        .filter(|_| member.movement == Movement::Teleport)
    {
        teleport_smoke(px, travel, hex(&actor.colour));
    }

    draw_power(
        px,
        &actor.power,
        &Power {
            pose,
            x,
            lift,
            seed: hash32(&actor.id),
            seconds,
            scale: CHARACTER_SCALE,
            colour: hex(&actor.colour),
            width: sheets.room_size.0,
            height: sheets.room_size.1,
            sprite: &sprite,
        },
    );

    if asleep {
        let rise = (seconds * 2.0).floor() % 4.0;
        px.alpha = 1.0 - rise * 0.2;
        emote(px, sheets, 0.0, x + 3.0, top - 2.0 - rise);
        px.alpha = 1.0;
    } else if actor.state == ActorState::Alert && (seconds * 3.0).floor() % 2.0 == 0.0 {
        emote(
            px,
            sheets,
            1.0,
            x - (EMOTE_SIZE / 2.0).floor(),
            top - EMOTE_SIZE,
        );
    }
}

fn emote(px: &mut Px, sheets: &Sheets, index: f64, x: f64, y: f64) {
    let alpha = px.alpha;
    px.image(
        sheets.emotes,
        (index * EMOTE_SIZE, 0.0, EMOTE_SIZE, EMOTE_SIZE),
        (x, y, EMOTE_SIZE, EMOTE_SIZE),
        alpha,
    );
}

/// A character's sheet: frames side by side, rows front, back, side.
struct Sprite {
    image: ImageId,
    frame: (f64, f64),
    size: (f64, f64),
    pose: Pose,
}

impl Sprite {
    /// The current frame with its feet at (x, y).
    fn draw(&self, px: &mut Px, x: f64, y: f64, alpha: f64) {
        if alpha <= 0.0 {
            return;
        }
        let (width, height) = self.size;
        let left = round(x - width / 2.0);
        let top = round(y - height + 1.0);
        let row = match self.pose.facing {
            Facing::Front => 0.0,
            Facing::Back => 1.0,
            Facing::Side => 2.0,
        };
        let source = (
            f64::from(self.pose.frame) * self.frame.0,
            row * self.frame.1,
            self.frame.0,
            self.frame.1,
        );
        // only the side view is drawn facing right; front and back stay as drawn
        if self.pose.flip && self.pose.facing == Facing::Side {
            px.flush();
            px.gfx.canvas.save();
            px.gfx.canvas.translate((left + width) as f32, top as f32);
            px.gfx.canvas.scale(-1.0, 1.0);
            px.image(self.image, source, (0.0, 0.0, width, height), alpha);
            px.gfx.canvas.restore();
        } else {
            px.image(self.image, source, (left, top, width, height), alpha);
        }
    }
}

/// Teleport: a puff of smoke where the character leaves, then where it lands.
fn teleport_smoke(px: &mut Px, (from, to, progress): (Point, Point, f64), colour: Color) {
    let at = if progress < 0.5 { from } else { to };
    let t = if progress < 0.5 {
        progress / 0.5
    } else {
        (progress - 0.5) / 0.5
    };
    let spread = if progress < 0.5 { t } else { 1.0 - t };
    let (cx, cy) = (at.x.floor(), at.y.floor() - 10.0);
    let saved = px.alpha;
    px.alpha = 0.9
        * if progress < 0.5 {
            1.0 - t * 0.6
        } else {
            0.4 + (1.0 - t) * 0.6
        };
    for i in 0..14 {
        let angle = f64::from(i) / 14.0 * PI * 2.0 + spread;
        let r = 3.0 + spread * 11.0;
        px.style(if i % 3 == 0 { rgb(WHITE) } else { colour });
        px.rect(
            round(cx + angle.cos() * r),
            round(cy + angle.sin() * r * 0.8),
            2.0,
            2.0,
        );
    }
    px.alpha = saved;
}

// ---------- powers.ts ----------

struct Power<'s> {
    pose: Pose,
    /// Feet of the caster, after `cast_offset`.
    x: f64,
    lift: f64,
    seed: u32,
    seconds: f64,
    scale: f64,
    colour: Color,
    width: f64,
    height: f64,
    sprite: &'s Sprite,
}

impl Power<'_> {
    fn leg(&self) -> f64 {
        self.pose.cast.map_or(0.0, |(leg, _)| leg)
    }

    /// Direction, feet, chest and head.
    fn body(&self) -> (f64, f64, f64, f64) {
        let feet = self.pose.y - self.lift;
        (
            if self.pose.flip { -1.0 } else { 1.0 },
            feet,
            feet - round(9.0 * self.scale),
            feet - round(16.0 * self.scale),
        )
    }

    fn to_wall(&self, x: f64, dir: f64) -> f64 {
        if dir > 0.0 { self.width - 1.0 - x } else { x }
    }
}

/// The effect plays in the middle of the pause.
fn effect_progress(cast: f64) -> Option<f64> {
    (0.1..0.9).contains(&cast).then(|| (cast - 0.1) / 0.8)
}

fn cast_offset(kind: &str, pose: &Pose, seconds: f64) -> (f64, f64) {
    let Some(e) = pose
        .cast
        .and_then(|(_, progress)| effect_progress(progress))
    else {
        return (0.0, 0.0);
    };
    match kind {
        "leap" => (0.0, round((PI * e).sin() * 18.0)),
        "quake" => ((seconds * 20.0).floor() % 2.0, 0.0),
        _ => (0.0, 0.0),
    }
}

fn draw_power(px: &mut Px, kind: &str, p: &Power) {
    let Some(e) = p
        .pose
        .cast
        .and_then(|(_, progress)| effect_progress(progress))
    else {
        return;
    };
    let saved = px.alpha;
    // fade in and out so effects never pop
    px.alpha = 1.0_f64.min(e * 6.0).min((1.0 - e) * 6.0);
    match kind {
        "beam" => beam(px, p, e),
        "bolt" => bolt(px, p, e),
        "psychic" => psychic(px, p, e),
        "fire" => fire(px, p, e),
        "frost" => frost(px, p, e),
        "claws" => claws(px, p, e),
        "throw" => projectile(px, p, e),
        "sparks" => sparks(px, p, e),
        "shield" => shield(px, p, e),
        "magnet" => magnet(px, p, e),
        "sonic" => sonic(px, p, e),
        "quake" => quake(px, p, e),
        "clone" => clone(px, p, e),
        "leap" => leap(px, p, e),
        "absorb" => absorb(px, p, e),
        _ => glow(px, p, e),
    }
    px.alpha = saved;
}

/// Optic blast or plasma: a beam from the eyes to the wall.
fn beam(px: &mut Px, p: &Power, e: f64) {
    let (dir, _, _, head) = p.body();
    let x0 = p.x + dir * round(5.0 * p.scale);
    let y = head;
    let length = round((e * 4.0).min(1.0) * p.to_wall(x0, dir));
    let x1 = x0 + dir * length;
    let flicker = (p.seconds * 10.0).floor();
    px.style(p.colour);
    px.rect(x0.min(x1), y - 1.0, length + 1.0, 3.0);
    px.style(rgb(WHITE));
    px.rect(x0.min(x1), y - flicker % 2.0, length + 1.0, 1.0);
    if e > 0.25 {
        px.burst(x1, y, 2.0 + flicker % 4.0, p.colour);
    }
}

/// Lightning: zigzag strikes from the top of the room next to the caster.
fn bolt(px: &mut Px, p: &Power, e: f64) {
    let (dir, _, _, head) = p.body();
    px.style(p.colour);
    px.rect(p.x - 3.0, head, 2.0, 2.0);
    px.rect(p.x + 1.0, head, 2.0, 2.0);
    let strike = (e * 3.0).floor();
    let local = e * 3.0 - strike;
    if local > 0.6 {
        return;
    }
    let n = p.leg() * 7.0 + strike;
    let target = round(
        (p.x + dir * (12.0 + unit(p.seed, n) * 28.0))
            .min(p.width - 5.0)
            .max(4.0),
    );
    let ground = p.pose.y;
    let mut x = target + round((unit(p.seed, n + 100.0) - 0.5) * 20.0);
    let mut y = 0.0;
    while y < ground {
        let next = if y + 6.0 >= ground {
            target
        } else {
            x + round((unit(p.seed, n * 31.0 + y) - 0.5) * 10.0)
        };
        let bottom = (y + 6.0).min(ground);
        px.line(x, y, next, bottom, p.colour);
        px.line(x + 1.0, y, next + 1.0, bottom, rgb(WHITE));
        x = next;
        y += 6.0;
    }
    px.burst(target, ground, 3.0, rgb(WHITE));
    px.burst(target, ground, 5.0, p.colour);
    // a faint flash over the whole room
    px.alpha *= 0.12;
    px.style(rgb(WHITE));
    px.rect(0.0, 0.0, p.width, p.height);
}

/// Telepathy: rings spreading from the head.
fn psychic(px: &mut Px, p: &Power, e: f64) {
    let (_, _, _, head) = p.body();
    let alpha = px.alpha;
    for i in 0..3 {
        let r = ((e * 2.5 + f64::from(i) / 3.0) % 1.0) * 20.0;
        px.alpha = alpha * (1.0 - r / 20.0);
        px.ring(p.x, head - 2.0, round(r).max(3.0), p.colour, 0.7);
    }
    px.alpha = alpha;
    px.style(p.colour);
    px.rect(p.x - 1.0, head - 4.0, 2.0, 2.0);
}

/// Flames rising around the body.
fn fire(px: &mut Px, p: &Power, e: f64) {
    let (_, feet, _, _) = p.body();
    let alpha = px.alpha;
    for i in 0..24 {
        let i = f64::from(i);
        let life = (e * 4.0 + unit(p.seed, i)) % 1.0;
        let x = round(p.x + (unit(p.seed, i + 50.0) - 0.5) * 20.0 + (life * 6.0 + i).sin() * 2.0);
        let y = round(feet - life * 28.0);
        px.alpha = alpha * (1.0 - life);
        px.style(if life < 0.3 { rgb(0xffe27a) } else { p.colour });
        let size = if life < 0.3 {
            3.0
        } else if life < 0.6 {
            2.0
        } else {
            1.0
        };
        px.rect(x, y, size, size);
    }
    px.alpha = alpha;
}

/// Ice crystals spreading on the floor and a few snowflakes.
fn frost(px: &mut Px, p: &Power, e: f64) {
    let ground = p.pose.y;
    let spread = (e * 2.0).min(1.0) * 22.0;
    for i in 0..10 {
        let i = f64::from(i);
        let angle = i / 10.0 * PI * 2.0 + unit(p.seed, i) * 0.4;
        let cx = round(p.x + angle.cos() * spread);
        let cy = round(ground + angle.sin() * spread * 0.35);
        px.style(p.colour);
        px.rect(cx - 2.0, cy, 5.0, 1.0);
        px.rect(cx, cy - 2.0, 1.0, 5.0);
        px.style(rgb(WHITE));
        px.rect(cx, cy, 1.0, 1.0);
    }
    px.style(rgb(WHITE));
    for i in 0..10 {
        let i = f64::from(i);
        let fall = (e * 2.0 + unit(p.seed, i + 30.0)) % 1.0;
        px.rect(
            round(p.x + (unit(p.seed, i + 30.0) - 0.5) * 36.0),
            round(ground - 32.0 + fall * 32.0),
            1.0,
            1.0,
        );
    }
}

/// Three slashes in front of the caster, twice.
fn claws(px: &mut Px, p: &Power, e: f64) {
    let (dir, _, chest, _) = p.body();
    let swipe = (e * 2.0) % 1.0;
    let reach = round((swipe * 3.0).min(1.0) * 11.0);
    let alpha = px.alpha;
    px.alpha = alpha * (1.0 - swipe);
    for i in 0..3 {
        let x0 = p.x + dir * (round(6.0 * p.scale) + f64::from(i) * 3.0);
        let y0 = chest - 7.0;
        px.line(x0, y0, x0 + dir * reach, y0 + reach, p.colour);
        px.line(
            x0 + dir,
            y0,
            x0 + dir * (reach + 1.0),
            y0 + reach,
            rgb(WHITE),
        );
    }
    px.alpha = alpha;
}

/// A glowing projectile thrown forward, exploding at the end of its flight.
fn projectile(px: &mut Px, p: &Power, e: f64) {
    let (dir, _, chest, _) = p.body();
    let x0 = p.x + dir * round(7.0 * p.scale);
    let range = p.to_wall(x0, dir).min(64.0);
    let flight = (e / 0.55).min(1.0);
    let x = round(x0 + dir * range * flight);
    let y = round(chest - (PI * flight).sin() * 8.0);
    if e < 0.55 {
        px.style(p.colour);
        px.rect(x - 1.0, y - 1.0, 3.0, 3.0);
        px.style(rgb(WHITE));
        px.rect(x, y, 1.0, 1.0);
        px.rect(x - dir * 3.0, y, 1.0, 1.0);
        px.rect(x - dir * 5.0, y, 1.0, 1.0);
    } else {
        let blast = (e - 0.55) / 0.45;
        px.burst(x, y, 2.0 + round(blast * 9.0), p.colour);
        px.burst(x, y, 1.0 + round(blast * 5.0), rgb(WHITE));
    }
}

/// Fireworks bursting above the caster.
fn sparks(px: &mut Px, p: &Power, e: f64) {
    let (_, _, _, head) = p.body();
    let leg = p.leg();
    for b in 0..3 {
        let b = f64::from(b);
        let local = e * 1.6 - b * 0.3;
        if !(0.0..1.0).contains(&local) {
            continue;
        }
        let cx = round(p.x + (unit(p.seed, leg * 3.0 + b) - 0.5) * 32.0);
        let cy = round(head - 8.0 - unit(p.seed, leg * 3.0 + b + 40.0) * 14.0);
        let even = b % 2.0 == 0.0;
        let (first, second) = if even {
            (p.colour, rgb(WHITE))
        } else {
            (rgb(WHITE), p.colour)
        };
        px.burst(cx, cy, 2.0 + round(local * 8.0), first);
        px.burst(cx, cy, 1.0 + round(local * 4.0), second);
    }
}

/// A force field around the body.
fn shield(px: &mut Px, p: &Power, e: f64) {
    let (_, _, chest, _) = p.body();
    let radius = round((e * 5.0).min(1.0) * 12.0 * p.scale);
    let alpha = px.alpha;
    px.alpha = alpha * 0.25;
    px.disc(p.x, chest, radius, p.colour);
    px.alpha = alpha * (0.7 + 0.3 * (p.seconds * 8.0).sin());
    px.ring(p.x, chest, radius, p.colour, 1.0);
    px.alpha = alpha;
}

/// Debris orbiting the caster, held by a magnetic or telekinetic field.
fn magnet(px: &mut Px, p: &Power, e: f64) {
    let (_, _, chest, _) = p.body();
    let radius = 8.0 + (e * 4.0).min(1.0) * 10.0;
    px.ring(p.x, chest, round(radius), p.colour, 0.4);
    for i in 0..4 {
        let angle = p.seconds * 4.0 + f64::from(i) / 4.0 * PI * 2.0;
        let x = round(p.x + angle.cos() * radius);
        let y = round(chest + angle.sin() * radius * 0.4 - 5.0);
        px.style(rgb(0x9aa0a6));
        px.rect(x, y, 2.0, 2.0);
        px.style(p.colour);
        px.rect(x, y, 1.0, 1.0);
    }
}

/// Sonic scream: arcs travelling forward.
fn sonic(px: &mut Px, p: &Power, e: f64) {
    let (dir, _, _, head) = p.body();
    let alpha = px.alpha;
    let range = p.to_wall(p.x, dir).min(44.0);
    for i in 0..3 {
        let d = ((e * 2.0 + f64::from(i) / 3.0) % 1.0) * range;
        let half = 2.0 + round(d / 5.0);
        px.alpha = alpha * (1.0 - d / range.max(1.0));
        px.style(p.colour);
        let mut dy = -half;
        while dy <= half {
            let bend = round(dy * dy / (half * 1.5));
            px.rect(
                round(p.x + dir * (7.0 * p.scale + d - bend)),
                head + 4.0 + dy,
                1.0,
                1.0,
            );
            dy += 1.0;
        }
    }
    px.alpha = alpha;
}

/// Cracks running along the floor, with dust.
fn quake(px: &mut Px, p: &Power, e: f64) {
    let ground = p.pose.y + 1.0;
    let length = round((e * 3.0).min(1.0) * 26.0) as u32;
    for dir in [-1.0, 1.0] {
        let mut y = ground;
        for s in 0..length {
            let s = f64::from(s);
            let side = if dir > 0.0 { 1.0 } else { 0.0 };
            if unit(p.seed, s * 2.0 + side) < 0.3 {
                y += if unit(p.seed, s + 77.0) < 0.5 {
                    -1.0
                } else {
                    1.0
                };
            }
            px.style(rgb(0xa89878));
            px.rect(p.x + dir * (5.0 + s), y, 1.0, 1.0);
        }
    }
    px.style(rgb(0xc8b89a));
    for i in 0..10 {
        let i = f64::from(i);
        let hop = (e * 3.0 + unit(p.seed, i)) % 1.0;
        let x = round(p.x + (unit(p.seed, i + 10.0) - 0.5) * 30.0);
        px.rect(x, round(ground - (PI * hop).sin() * 8.0), 2.0, 2.0);
    }
}

/// Duplicates popping up around the caster.
fn clone(px: &mut Px, p: &Power, e: f64) {
    let spots = [-20.0, 20.0, -11.0, 11.0].map(|dx: f64| round(dx * p.scale));
    let shown = ((e * 6.0).floor() as usize + 1).min(spots.len());
    for (i, spot) in spots.iter().enumerate().take(shown) {
        let x = round((p.x + spot).min(p.width - 12.0).max(12.0));
        let y = p.pose.y + if i < 2 { 0.0 } else { -7.0 };
        p.sprite.draw(px, x, y, 0.85);
        if e > 0.85 {
            px.burst(
                x,
                y - round(10.0 * p.scale),
                round((e - 0.85) * 60.0),
                rgb(WHITE),
            );
        }
    }
}

/// Dust at take-off and landing; the jump itself is `cast_offset`.
fn leap(px: &mut Px, p: &Power, e: f64) {
    let ground = p.pose.y;
    if e < 0.2 {
        px.burst(p.x, ground, 3.0 + round(e * 30.0), rgb(0xc8b89a));
    } else if e > 0.8 {
        px.burst(p.x, ground, 3.0 + round((e - 0.8) * 30.0), rgb(0xc8b89a));
    }
}

/// Energy pulled in towards the caster.
fn absorb(px: &mut Px, p: &Power, e: f64) {
    let (_, _, chest, _) = p.body();
    let pull = (e * 2.0) % 1.0;
    let radius = (1.0 - pull) * 24.0;
    px.style(p.colour);
    for i in 0..12 {
        let angle = f64::from(i) / 12.0 * PI * 2.0 + e * 3.0;
        px.rect(
            round(p.x + angle.cos() * radius),
            round(chest + angle.sin() * radius * 0.7),
            1.0,
            1.0,
        );
    }
    if pull > 0.8 {
        px.ring(p.x, chest, 4.0, rgb(WHITE), 1.0);
    }
}

/// A pulse of light from the body, for everyone else.
fn glow(px: &mut Px, p: &Power, e: f64) {
    let (_, _, chest, _) = p.body();
    let alpha = px.alpha;
    px.alpha = alpha * (0.3 + 0.2 * (p.seconds * 6.0).sin());
    px.disc(
        p.x,
        chest - 2.0,
        9.0 + round((PI * e).sin() * 5.0),
        p.colour,
    );
    px.alpha = alpha;
    let leg = p.leg();
    px.style(rgb(WHITE));
    for i in 0..6 {
        let i = f64::from(i);
        let twinkle = (e * 3.0 + unit(p.seed, leg + i)) % 1.0;
        if twinkle > 0.5 {
            continue;
        }
        let tx = round(p.x + (unit(p.seed, leg * 5.0 + i) - 0.5) * 28.0);
        let ty = round(chest - 12.0 + unit(p.seed, i + 60.0) * 18.0);
        px.rect(tx, ty, 1.0, 1.0);
        if twinkle < 0.2 {
            px.rect(tx - 1.0, ty, 3.0, 1.0);
            px.rect(tx, ty - 1.0, 1.0, 3.0);
        }
    }
}

// ---------- SessionCard.svelte ----------

/// Cerebro's `humanizeDuration`: 42s, 12min, 1h05, 3d.
pub fn humanize(ms: i64) -> String {
    let s = ms.max(0) / 1000;
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}min", s / 60)
    } else if s < 86_400 {
        format!("{}h{:02}", s / 3600, s % 3600 / 60)
    } else {
        format!("{}d", s / 86_400)
    }
}

fn status_colour(status: SessionStatus) -> Color {
    rgb(match status {
        SessionStatus::Busy => BUSY,
        SessionStatus::Waiting => WAITING,
        SessionStatus::Idle => IDLE,
    })
}

fn status_label(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Busy => "busy",
        SessionStatus::Idle => "idle",
        SessionStatus::Waiting => "needs you",
    }
}

/// `text` cut to `width`, with an ellipsis, as CSS' `text-overflow`.
fn ellipsis(gfx: &Gfx, text: &str, width: f32, style: Type) -> String {
    if gfx.measure(text, style) <= width {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let cut: String = chars.iter().collect::<String>() + "…";
        if gfx.measure(&cut, style) <= width {
            return cut;
        }
    }
    String::new()
}

/// The card's measures for a scene `scale` room pixels wide: padding, gap,
/// title, caption sizes (Cerebro's panel at 1, its phone page from 2).
fn measures(scale: f32) -> (f32, f32, f32, f32) {
    let k = (scale / 2.0).clamp(0.55, 1.0);
    (10.0 * k, 8.0 * k, 15.0 * k, 12.0 * k)
}

/// Height of a session card whose scene is `scale` room pixels a pixel.
pub fn card_height(cerebro: &Cerebro, scale: f32) -> f32 {
    let (pad, gap, title, caption) = measures(scale);
    pad * 2.0 + title * 1.25 + cerebro.room_size.1 as f32 * scale + caption * 1.3 * 2.0 + gap * 3.0
}

pub fn card_width(cerebro: &Cerebro, scale: f32) -> f32 {
    let (pad, ..) = measures(scale);
    cerebro.room_size.0 as f32 * scale + pad * 2.0
}

/// Cerebro's session card at (x, y), its scene `scale` room pixels a pixel.
#[allow(clippy::too_many_arguments)]
pub fn draw_card(
    gfx: &mut Gfx,
    textures: &mut Textures,
    cerebro: &Cerebro,
    session: &Session,
    x: f32,
    y: f32,
    scale: f32,
    now_ms: i64,
) {
    let (pad, gap, title, caption) = measures(scale);
    let (w, h) = (card_width(cerebro, scale), card_height(cerebro, scale));
    let inner = w - pad * 2.0;
    let mut card = Path::new();
    card.rounded_rect(
        x + 0.5,
        y + 0.5,
        w - 1.0,
        h - 1.0,
        8.0 * (scale / 2.0).clamp(0.6, 1.5),
    );
    gfx.fill(&card, rgb(CARD));
    let border = match session.status {
        SessionStatus::Idle => fade(Color::white(), 0.08),
        status => fade(status_colour(status), 0.7),
    };
    gfx.stroke(&card, border, 1.0);

    let mut top = y + pad;
    // the dot and the name
    let name_style = Type::new(Face::MonoBold, title);
    let dot = title * 0.66;
    gfx.disc(
        x + pad + dot / 2.0,
        top + title * 0.62,
        dot / 2.0,
        status_colour(session.status),
    );
    let name = ellipsis(gfx, &session.name, inner - dot - 6.0, name_style);
    gfx.text(
        x + pad + dot + 6.0,
        top + title * 0.62,
        &name,
        name_style,
        rgb(TEXT),
        Anchor::MIDDLE_LEFT,
    );
    top += title * 1.25 + gap;

    // the room
    let room_h = cerebro.room_size.1 as f32 * scale;
    if let Some(scene) = &session.scene {
        let mut frame = Path::new();
        frame.rounded_rect(x + pad, top, inner, room_h, 5.0);
        gfx.fill(&frame, fade(Color::black(), 0.25));
        draw_scene(gfx, textures, cerebro, scene, x + pad, top, scale, now_ms);
    }
    top += room_h + gap;

    // the room — the cast
    let cast_style = Type::new(Face::MonoBold, caption);
    let room = session.room.clone().unwrap_or_default();
    let names: Vec<&str> = session
        .scene
        .iter()
        .flat_map(|scene| scene.actors.iter().map(|actor| actor.name.as_str()))
        .collect();
    let lead = if names.is_empty() {
        session.lead.clone().unwrap_or_default()
    } else {
        names.join(" · ")
    };
    let muted = format!("{room} —");
    let muted_w = gfx.text(
        x + pad,
        top,
        &muted,
        Type::new(Face::Mono, caption),
        rgb(MUTED),
        Anchor::TOP_LEFT,
    );
    let cast = ellipsis(gfx, &format!(" {lead}"), inner - muted_w, cast_style);
    gfx.text(
        x + pad + muted_w,
        top,
        &cast,
        cast_style,
        rgb(TEXT),
        Anchor::TOP_LEFT,
    );
    top += caption * 1.3 + gap;

    // the folder, and the status for how long
    let meta = Type::new(Face::Mono, caption);
    let since = session.since.map_or(0, |since| now_ms - since.0);
    let status = format!("{} {}", status_label(session.status), humanize(since));
    let status_w = gfx.text(
        x + w - pad,
        top,
        &status,
        meta,
        rgb(MUTED),
        Anchor::TOP_RIGHT,
    );
    let project = session.project.clone().unwrap_or_default();
    let project = ellipsis(gfx, &project, inner - status_w - 6.0, meta);
    gfx.text(x + pad, top, &project, meta, rgb(MUTED), Anchor::TOP_LEFT);
}

/// The largest scale whose card fits `w` × `h`.
fn fitting(cerebro: &Cerebro, w: f32, h: f32) -> f32 {
    let (mut low, mut high) = (0.3_f32, 3.0_f32);
    for _ in 0..24 {
        let mid = f32::midpoint(low, high);
        if card_width(cerebro, mid) <= w && card_height(cerebro, mid) <= h {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}

/// Up to four sessions' cards in the box: one large, two side by side, or
/// two by two; those waiting for an answer first, as the feed sorts them.
#[allow(clippy::too_many_arguments)]
pub fn draw_sessions(
    gfx: &mut Gfx,
    textures: &mut Textures,
    cerebro: &Cerebro,
    (x, y, w, h): (f32, f32, f32, f32),
    now_ms: i64,
) {
    let sessions: Vec<&Session> = cerebro.sessions.iter().take(4).collect();
    let (columns, rows) = match sessions.len() {
        0 => return,
        1 => (1, 1),
        2 => (2, 1),
        _ => (2, 2),
    };
    let gap = 10.0;
    let cell_w = (w - gap * (columns as f32 - 1.0)) / columns as f32;
    let cell_h = (h - gap * (rows as f32 - 1.0)) / rows as f32;
    let scale = fitting(cerebro, cell_w, cell_h);
    let (card_w, card_h) = (card_width(cerebro, scale), card_height(cerebro, scale));
    let grid_w = card_w * columns as f32 + gap * (columns as f32 - 1.0);
    let grid_h = card_h * rows as f32 + gap * (rows as f32 - 1.0);
    let (left, top) = (x + (w - grid_w) / 2.0, y + (h - grid_h) / 2.0);
    for (index, session) in sessions.into_iter().enumerate() {
        let (column, row) = (index % columns, index / columns);
        draw_card(
            gfx,
            textures,
            cerebro,
            session,
            left + column as f32 * (card_w + gap),
            top + row as f32 * (card_h + gap),
            scale,
            now_ms,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // expected values computed with Cerebro's own wander.ts (node)
    #[test]
    fn the_hash_is_cerebros() {
        assert_eq!(hash32("lead:abc"), HASH_LEAD_ABC);
        assert_eq!(hash32(""), HASH_EMPTY);
    }

    #[test]
    fn the_random_numbers_are_cerebros() {
        let seed = hash32("lead:abc");
        for (n, expected) in UNITS {
            assert!(
                (unit(seed, n) - expected).abs() < 1e-12,
                "unit({seed}, {n})"
            );
        }
    }

    #[test]
    fn a_pose_is_cerebros() {
        let pose = pose_at(
            "lead:abc",
            (10.0, 76.0, 172.0, 40.0),
            POSE_AT,
            true,
            Movement::Walk,
        );
        assert_eq!((pose.x, pose.y, pose.frame), POSE);
    }

    #[test]
    fn durations_read_as_in_cerebro() {
        assert_eq!(humanize(42_000), "42s");
        assert_eq!(humanize(754_000), "12min");
        assert_eq!(humanize(3_900_000), "1h05");
        assert_eq!(humanize(200_000_000), "2d");
    }

    const HASH_LEAD_ABC: u32 = 1_030_476_213;
    const HASH_EMPTY: u32 = 2_872_998_923;
    const UNITS: [(f64, f64); 5] = [
        (0.0, 0.007_708_108_518_272_638),
        (1.0, 0.938_921_611_988_917),
        (999.0, 0.132_219_368_126_243_35),
        (449_005_123.0, 0.739_066_651_556_640_9),
        (93_000_003_831.0, 0.225_672_401_254_996_66),
    ];
    const POSE_AT: f64 = 1_793_195_441.3;
    const POSE: (f64, f64, u32) = (105.0, 94.0, 3);
}
