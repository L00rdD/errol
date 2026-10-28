//! Animated backgrounds, one per look. `heat` (0 → 1.5) rises with a burst
//! when a scene starts and cools down by itself.

mod grid;
mod marauder;
mod neural;

use crate::look::Look;
use crate::render::Gfx;

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
