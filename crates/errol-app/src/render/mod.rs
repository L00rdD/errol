//! The window and the frame loop. SDL2 opens the display: KMS/DRM on the Pi
//! (no X, no Wayland: `SDL_VIDEODRIVER=kmsdrm`), a window on the Mac. femtovg
//! draws on OpenGL ES 3 (Pi) or OpenGL 3.3 (Mac). The 800 × 480 stage is
//! scaled to fit whatever the display is.

mod gfx;
pub mod map;
mod textures;

use std::time::{Duration, Instant};

use errol_domain::usecases::{ObserveFeed, View};
use femtovg::renderer::OpenGl;
use femtovg::{Canvas, Color};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{FullscreenType, GLProfile, Window};

use crate::Options;
use crate::look::Look;
use crate::pictures::Pictures;
use crate::scenes::{HEIGHT, Stage, WIDTH};

pub use gfx::{Anchor, Face, Fonts, Gfx, Type};
pub use map::{Map, ink};
pub use textures::{Finish, Textures};

const MICHROMA: &[u8] = include_bytes!("../../../../assets/fonts/Michroma-Regular.ttf");
const RAJDHANI_MEDIUM: &[u8] = include_bytes!("../../../../assets/fonts/Rajdhani-Medium.ttf");
const RAJDHANI_SEMIBOLD: &[u8] = include_bytes!("../../../../assets/fonts/Rajdhani-SemiBold.ttf");
const RAJDHANI_BOLD: &[u8] = include_bytes!("../../../../assets/fonts/Rajdhani-Bold.ttf");
const FELL: &[u8] = include_bytes!("../../../../assets/fonts/IMFellEnglish-Regular.ttf");
const FELL_ITALIC: &[u8] = include_bytes!("../../../../assets/fonts/IMFellEnglish-Italic.ttf");
/// Where the system keeps a monospace, regular and bold: Cerebro's cards are
/// written in one.
const MONO: [&str; 2] = [
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
];
const MONO_BOLD: [&str; 2] = [
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf",
    "/System/Library/Fonts/Menlo.ttc",
];

pub fn run(options: &Options, observe: &ObserveFeed, pictures: &Pictures) -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = open_window(&video, options)?;
    let _context = window.gl_create_context()?;
    // vsync off: the loop paces itself at `fps`, which costs less than 60 Hz
    let _ = video.gl_set_swap_interval(0);
    sdl.mouse().show_cursor(false);

    let renderer = open_gl(&video)?;
    let mut canvas = Canvas::new(renderer).map_err(|error| format!("femtovg: {error:?}"))?;
    let fonts = load_fonts(&mut canvas)?;
    let mut screen = Screen {
        fonts: &fonts,
        default: options.look,
        look: options.look,
        map: None,
        map_failed: false,
        stage: Stage::new(options.look),
        textures: Textures::new(pictures),
    };
    let mut events = sdl.event_pump()?;
    let start = Instant::now();
    let frame = Duration::from_secs_f64(1.0 / f64::from(options.fps));
    let mut last = start;
    loop {
        let tick = Instant::now();
        if quit_asked(&mut events) {
            return Ok(());
        }
        let t = start.elapsed().as_secs_f32();
        let dt = tick.duration_since(last).as_secs_f32().min(0.1);
        last = tick;
        screen.draw(&mut canvas, &window, observe, (t, dt));
        if let Some(path) = &options.shot
            && f64::from(t) >= options.shot_after
        {
            return save_shot(&mut canvas, path);
        }
        window.gl_swap_window();
        if let Some(rest) = frame.checked_sub(tick.elapsed()) {
            std::thread::sleep(rest);
        }
    }
}

/// The window: the whole display on the Pi, a window (hidden for a
/// screenshot) on the Mac.
fn open_window(video: &sdl2::VideoSubsystem, options: &Options) -> Result<Window, String> {
    set_gl_attributes(video);
    let mut builder = video.window("Errol", WIDTH as u32, HEIGHT as u32);
    builder.opengl().allow_highdpi();
    if options.windowed {
        builder.position_centered();
        if options.shot.is_some() {
            builder.hidden();
        }
    }
    let mut window = builder.build().map_err(|error| error.to_string())?;
    if !options.windowed {
        fill_display(video, &mut window)?;
    }
    Ok(window)
}

/// OpenGL 3.3 core on the Mac, OpenGL ES 3 on the Pi; a stencil for femtovg.
fn set_gl_attributes(video: &sdl2::VideoSubsystem) {
    let attributes = video.gl_attr();
    if cfg!(target_os = "macos") {
        attributes.set_context_profile(GLProfile::Core);
        attributes.set_context_version(3, 3);
        attributes.set_context_flags().forward_compatible().set();
    } else {
        attributes.set_context_profile(GLProfile::GLES);
        attributes.set_context_version(3, 0);
    }
    attributes.set_stencil_size(8);
    attributes.set_double_buffer(true);
}

/// SDL's KMS backend keeps the mode it enters full screen with: set the
/// mode first, then go full screen. Built full screen, it kept 640 × 480,
/// which the 5" screen scales; 800 × 480 is its native size.
fn fill_display(video: &sdl2::VideoSubsystem, window: &mut Window) -> Result<(), String> {
    let mode = pick_mode(video)?;
    eprintln!(
        "errol: display mode {}x{} @ {} Hz",
        mode.w, mode.h, mode.refresh_rate
    );
    window.set_display_mode(mode)?;
    window.set_fullscreen(FullscreenType::True)
}

/// The built-in faces, and the system's monospace (Rajdhani when it has none).
fn load_fonts(canvas: &mut Canvas<OpenGl>) -> Result<Fonts, String> {
    let mut font = |data: &[u8]| {
        canvas
            .add_font_mem(data)
            .map_err(|error| format!("font: {error:?}"))
    };
    let (medium, semibold) = (font(RAJDHANI_MEDIUM)?, font(RAJDHANI_SEMIBOLD)?);
    let mut system = |paths: [&str; 2], fallback| {
        paths
            .iter()
            .find_map(|path| canvas.add_font(path).ok())
            .unwrap_or(fallback)
    };
    let (mono, mono_bold) = (system(MONO, medium), system(MONO_BOLD, semibold));
    let mut font = |data: &[u8]| {
        canvas
            .add_font_mem(data)
            .map_err(|error| format!("font: {error:?}"))
    };
    Ok(Fonts {
        michroma: font(MICHROMA)?,
        medium,
        semibold,
        bold: font(RAJDHANI_BOLD)?,
        fell: font(FELL)?,
        fell_italic: font(FELL_ITALIC)?,
        mono,
        mono_bold,
    })
}

/// Escape, Q or the window closed.
fn quit_asked(events: &mut sdl2::EventPump) -> bool {
    events.poll_iter().any(|event| {
        matches!(
            event,
            Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape | Keycode::Q),
                    ..
                }
        )
    })
}

/// What stays from one frame to the next.
struct Screen<'a> {
    fonts: &'a Fonts,
    /// The look of the options, when the feed asks for none.
    default: Look,
    look: Look,
    /// The Map's images, baked the first time the look is hogwarts.
    map: Option<Map>,
    map_failed: bool,
    stage: Stage,
    textures: Textures<'a>,
}

impl Screen<'_> {
    /// Draws a frame, `t` seconds since the start and `dt` since the last one.
    fn draw(
        &mut self,
        canvas: &mut Canvas<OpenGl>,
        window: &Window,
        observe: &ObserveFeed,
        (t, dt): (f32, f32),
    ) {
        let (width, height) = window.drawable_size();
        let (logical, _) = window.size();
        let dpi = width as f32 / logical.max(1) as f32;
        canvas.set_size(width, height, dpi);
        self.bake_map(canvas);
        canvas.clear_rect(0, 0, width, height, Color::black());
        let (x, y, scale) = fit(width as f32 / dpi, height as f32 / dpi);
        canvas.save();
        canvas.translate(x, y);
        canvas.scale(scale, scale);
        canvas.scissor(0.0, 0.0, WIDTH, HEIGHT);
        let view = observe.view();
        self.follow_look(&view);
        let mut gfx = Gfx {
            canvas: &mut *canvas,
            fonts: self.fonts,
            look: self.look,
            palette: self.look.palette(),
            t,
            map: self.map.as_ref().filter(|_| self.look == Look::Hogwarts),
            alpha: 1.0,
        };
        self.stage.draw(&mut gfx, &view, &mut self.textures, dt);
        canvas.restore();
        canvas.flush();
        self.textures.after_flush(canvas);
    }

    /// Bakes the Map's images the first time the look is hogwarts; a failure
    /// is said once, and the look goes on without them.
    fn bake_map(&mut self, canvas: &mut Canvas<OpenGl>) {
        if self.look != Look::Hogwarts || self.map.is_some() || self.map_failed {
            return;
        }
        match Map::bake(canvas, self.fonts) {
            Ok(baked) => {
                // the images are drawn now, before anything uses them
                canvas.flush();
                self.map = Some(baked);
            }
            Err(error) => {
                eprintln!("errol: {error}");
                self.map_failed = true;
            }
        }
    }

    /// The feed may ask for another look (the home page's remote control).
    fn follow_look(&mut self, view: &View) {
        let asked = view
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.settings.look.as_deref());
        let wanted = Look::choose(asked, self.default);
        if wanted != self.look {
            self.look = wanted;
            self.stage.set_look(wanted);
        }
    }
}

/// Where the stage goes on a screen of `width` × `height`: as large as it
/// fits, centred. Returns the offset and the scale.
fn fit(width: f32, height: f32) -> (f32, f32, f32) {
    let scale = (width / WIDTH).min(height / HEIGHT);
    (
        (width - WIDTH * scale) / 2.0,
        (height - HEIGHT * scale) / 2.0,
        scale,
    )
}

/// The display's 800 × 480 mode (the stage's size), else its largest one.
fn pick_mode(video: &sdl2::VideoSubsystem) -> Result<sdl2::video::DisplayMode, String> {
    let count = video.num_display_modes(0)?;
    let modes: Vec<_> = (0..count)
        .filter_map(|index| video.display_mode(0, index).ok())
        .collect();
    eprintln!(
        "errol: display modes {}",
        modes
            .iter()
            .map(|m| format!("{}x{}@{}", m.w, m.h, m.refresh_rate))
            .collect::<Vec<_>>()
            .join(" ")
    );
    modes
        .iter()
        .find(|mode| mode.w == WIDTH as i32 && mode.h == HEIGHT as i32)
        .or_else(|| modes.iter().max_by_key(|mode| mode.w * mode.h))
        .copied()
        .ok_or_else(|| "no display mode".to_owned())
}

/// The GL functions come from SDL; only unsafe code can hand them to femtovg.
#[allow(unsafe_code)]
fn open_gl(video: &sdl2::VideoSubsystem) -> Result<OpenGl, String> {
    // SAFETY: the GL context was just created on this thread and is current;
    // SDL returns valid function pointers for it.
    unsafe { OpenGl::new_from_function(|name| video.gl_get_proc_address(name).cast()) }
        .map_err(|error| format!("OpenGL: {error:?}"))
}

fn save_shot(canvas: &mut Canvas<OpenGl>, path: &str) -> Result<(), String> {
    let shot = canvas
        .screenshot()
        .map_err(|error| format!("screenshot: {error:?}"))?;
    let (width, height) = (shot.width() as u32, shot.height() as u32);
    let bytes: Vec<u8> = shot
        .pixels()
        .flat_map(|pixel| [pixel.r, pixel.g, pixel.b, pixel.a])
        .collect();
    let image =
        femtovg::img::RgbaImage::from_raw(width, height, bytes).ok_or("screenshot: bad size")?;
    image.save(path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stage_fills_a_screen_of_its_size() {
        assert_eq!(fit(WIDTH, HEIGHT), (0.0, 0.0, 1.0));
    }

    #[test]
    fn a_larger_screen_scales_the_stage_and_centres_it() {
        // 1920 × 1080: the height limits, 2.25 times; bars left and right
        assert_eq!(fit(1920.0, 1080.0), (60.0, 0.0, 2.25));
        // 640 × 480: the width limits, bars above and below
        assert_eq!(fit(640.0, 480.0), (0.0, 48.0, 0.8));
    }
}
