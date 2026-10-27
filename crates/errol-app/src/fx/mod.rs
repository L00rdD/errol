//! Animated backgrounds, one per look. `heat` (0 → 1.5) rises with a burst
//! when a scene starts and cools down by itself.

mod grid;
mod neural;

use crate::look::Look;
use crate::render::Gfx;

pub use grid::Grid;
pub use neural::Neural;

pub enum Background {
    Neural(Neural),
    Grid(Grid),
}

impl Background {
    pub fn new(look: Look) -> Self {
        match look {
            Look::Friday => Self::Neural(Neural::new()),
            Look::Cyberpunk => Self::Grid(Grid::new()),
        }
    }

    pub fn burst(&mut self, power: f32) {
        match self {
            Self::Neural(neural) => neural.burst(power),
            Self::Grid(grid) => grid.burst(power),
        }
    }

    pub fn draw(&mut self, gfx: &mut Gfx, dt: f32) {
        match self {
            Self::Neural(neural) => neural.draw(gfx, dt),
            Self::Grid(grid) => grid.draw(gfx, dt),
        }
    }
}
