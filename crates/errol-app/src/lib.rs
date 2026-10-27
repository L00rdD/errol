//! Presentation layer of Errol: the living-room screen, drawn on the GPU.
//!
//! On the Pi it draws straight on the HDMI display (SDL2 on KMS/DRM, OpenGL ES,
//! no browser, no compositor); on a Mac it opens a window, for the preview.
//!
//! ```text
//! options    command line and environment
//! injector   the only place that knows which adapter implements which port
//! poller     reads the feed every second on its own thread
//! pictures   fetches and decodes posters and backdrops on a worker thread
//! render     the window, the GL context, the frame loop, drawing helpers
//! look       the three looks: friday (the AI after J.A.R.V.I.S.), cyberpunk, hogwarts
//! anim       easing, decoding text, timing
//! fx         animated backgrounds: neural network, Netwatch grid, Great Hall candles
//! scenes     header, ticker, idle recap, events, cinema
//! ```

mod anim;
mod fx;
mod injector;
mod look;
mod options;
mod pictures;
mod poller;
mod render;
mod scenes;

pub use options::Options;

/// Starts the screen; returns when the window is closed or a screenshot is taken.
pub fn run() -> Result<(), String> {
    let options = Options::from_env_and_args(std::env::args().skip(1))?;
    let container = injector::build(&options);
    poller::start(container.observe.clone());
    let pictures = pictures::Pictures::start(container.load_picture);
    render::run(&options, &container.observe, &pictures)
}
