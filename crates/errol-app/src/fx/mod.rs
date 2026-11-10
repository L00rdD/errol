//! Animated backgrounds, one per look. `heat` (0 → 1.5) rises with a burst
//! when a scene starts and cools down by itself.

mod ambience;
mod grid;
mod marauder;
mod neural;

use errol_domain::entities::{EventKind, Sky};

use crate::look::Look;
use crate::render::Gfx;

use ambience::Ambience;
pub use grid::Grid;
pub use marauder::Marauder;
pub use neural::Neural;

pub enum Background {
    Neural(Neural),
    Grid(Grid),
    Marauder(Marauder),
}

impl Background {
    pub fn new(look: Look) -> Self {
        match look {
            Look::Friday => Self::Neural(Neural::new()),
            Look::Cyberpunk => Self::Grid(Grid::new()),
            Look::Hogwarts => Self::Marauder(Marauder::new()),
        }
    }

    pub fn burst(&mut self, power: f32) {
        match self {
            Self::Neural(neural) => neural.burst(power),
            Self::Grid(grid) => grid.burst(power),
            Self::Marauder(map) => map.burst(power),
        }
    }

    /// Something small happened (a banner): the background answers in its
    /// own way, without taking the screen.
    pub fn react(&mut self, kind: &EventKind) {
        self.ambience().stir();
        match self {
            Self::Neural(neural) => neural.react(kind),
            Self::Grid(grid) => grid.react(kind),
            Self::Marauder(map) => map.react(kind),
        }
    }

    /// How lively the background is: 1 as usual, lower for a film or the
    /// night (0 holds it still).
    pub fn set_calm(&mut self, liveliness: f32) {
        self.ambience().aim(liveliness);
    }

    /// The sky outside, when the feed knows it: rain falls in the background
    /// of the looks that can show it.
    pub fn set_sky(&mut self, sky: Option<Sky>) {
        self.ambience().sky = sky;
    }

    fn ambience(&mut self) -> &mut Ambience {
        match self {
            Self::Neural(neural) => &mut neural.ambience,
            Self::Grid(grid) => &mut grid.ambience,
            Self::Marauder(map) => &mut map.ambience,
        }
    }

    /// Footsteps from one box's door to another's, on the Map; nothing elsewhere.
    pub fn walk(&mut self, from: (f32, f32, f32, f32), to: (f32, f32, f32, f32)) {
        if let Self::Marauder(map) = self {
            map.walk(from, to);
        }
    }

    pub fn draw(&mut self, gfx: &mut Gfx, dt: f32) {
        match self {
            Self::Neural(neural) => neural.draw(gfx, dt),
            Self::Grid(grid) => grid.draw(gfx, dt),
            Self::Marauder(map) => map.draw(gfx, dt),
        }
    }
}
