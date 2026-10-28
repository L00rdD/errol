//! The window and the frame loop. SDL2 opens the display: KMS/DRM on the Pi
//! (no X, no Wayland: `SDL_VIDEODRIVER=kmsdrm`), a window on the Mac. femtovg
//! draws on OpenGL ES 3 (Pi) or OpenGL 3.3 (Mac). The 800 × 480 stage is
//! scaled to fit whatever the display is.

mod gfx;
pub mod map;
mod textures;

use std::time::{Duration, Instant};

use errol_domain::usecases::ObserveFeed;
use femtovg::renderer::OpenGl;
use femtovg::{Canvas, Color};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{FullscreenType, GLProfile};

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
        // SDL's KMS backend keeps the mode it enters full screen with: set the
        // mode first, then go full screen. Built full screen, it kept 640 × 480,
        // which the 5" screen scales; 800 × 480 is its native size.
        let mode = pick_mode(&video)?;
        eprintln!(
            "errol: display mode {}x{} @ {} Hz",
            mode.w, mode.h, mode.refresh_rate
        );
        window.set_display_mode(mode)?;
        window.set_fullscreen(FullscreenType::True)?;
    }
    let _context = window.gl_create_context()?;
    // vsync off: the loop paces itself at `fps`, which costs less than 60 Hz
    let _ = video.gl_set_swap_interval(0);
    sdl.mouse().show_cursor(false);

    let renderer = open_gl(&video)?;
    let mut canvas = Canvas::new(renderer).map_err(|error| format!("femtovg: {error:?}"))?;
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
    let fonts = Fonts {
        michroma: font(MICHROMA)?,
        medium,
        semibold,
        bold: font(RAJDHANI_BOLD)?,
        fell: font(FELL)?,
        fell_italic: font(FELL_ITALIC)?,
        mono,
        mono_bold,
    };

    let mut look = options.look;
    // the Map's images, baked the first time the look is hogwarts
    let mut map: Option<Map> = None;
    let mut map_failed = false;
    let mut stage = Stage::new(look);
    let mut textures = Textures::new(pictures);
    let mut events = sdl.event_pump()?;
    let start = Instant::now();
    let frame = Duration::from_secs_f64(1.0 / f64::from(options.fps));
    let mut last = start;
    loop {
        let tick = Instant::now();
        for event in events.poll_iter() {
            if matches!(
                event,
                Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape | Keycode::Q),
                        ..
                    }
            ) {
                return Ok(());
            }
        }
        let (width, height) = window.drawable_size();
        let (logical, _) = window.size();
        let dpi = width as f32 / logical.max(1) as f32;
        canvas.set_size(width, height, dpi);
        if look == Look::Hogwarts && map.is_none() && !map_failed {
            match Map::bake(&mut canvas, &fonts) {
                Ok(baked) => {
                    // the images are drawn now, before anything uses them
                    canvas.flush();
                    map = Some(baked);
                }
                Err(error) => {
                    eprintln!("errol: {error}");
                    map_failed = true;
                }
            }
        }
        canvas.clear_rect(0, 0, width, height, Color::black());
        // fit the stage, centred
        let (screen_w, screen_h) = (width as f32 / dpi, height as f32 / dpi);
        let scale = (screen_w / WIDTH).min(screen_h / HEIGHT);
        canvas.save();
        canvas.translate(
            (screen_w - WIDTH * scale) / 2.0,
            (screen_h - HEIGHT * scale) / 2.0,
        );
        canvas.scale(scale, scale);
        canvas.scissor(0.0, 0.0, WIDTH, HEIGHT);
        let view = observe.view();
        // the feed may ask for another look (the home page's remote control)
        let asked = view
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.settings.look.as_deref());
        let wanted = Look::choose(asked, options.look);
        if wanted != look {
            look = wanted;
            stage.set_look(look);
        }
        let t = start.elapsed().as_secs_f32();
        let dt = tick.duration_since(last).as_secs_f32().min(0.1);
        last = tick;
        let mut gfx = Gfx {
            canvas: &mut canvas,
            fonts: &fonts,
            look,
            palette: look.palette(),
            t,
            map: map.as_ref().filter(|_| look == Look::Hogwarts),
            alpha: 1.0,
        };
        stage.draw(&mut gfx, &view, &mut textures, dt);
        canvas.restore();
        canvas.flush();
        textures.after_flush(&mut canvas);
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
