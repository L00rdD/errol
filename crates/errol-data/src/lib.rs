//! Data layer of Errol: implements the domain's ports.
//!
//! ```text
//! models       serde mirrors of screen.json, turned into domain entities
//! datasources  the feed over HTTP, the demo feed, pictures over HTTP
//! services     the system clock
//! ```

pub mod datasources;
pub mod models;
pub mod services;
