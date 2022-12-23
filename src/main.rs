mod chip8;
mod display;
mod lsfr;

use std::time::{Duration, Instant};

use clap::Parser;
use log::debug;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::mem::MaybeUninit;

use self::display::Display;

#[inline(always)]
fn keycode_to_idx(key: Keycode) -> Option<usize> {
    match key {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::Num4 => Some(0xC),
        Keycode::R => Some(0xD),
        Keycode::F => Some(0xE),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

pub(crate) struct Screen<'a> {
    canvas: &'a mut Canvas<Window>,
    rects: [Rect; Display::SIZE],
}

impl<'a> Screen<'a> {
    const DISPLAY_ON_PIXEL: Color = Color::RGB(255, 255, 255);
    const DISPLAY_OFF_PIXEL: Color = Color::RGB(0, 0, 0);

    pub(crate) fn new(canvas: &'a mut Canvas<Window>) -> Self {
        let (pixel_size_x, pixel_size_y) = Self::pixel_size(canvas);
        let rects = {
            // Safety:
            // `assume_init` is safe here because the type we are claiming to have initialised here is a
            // bunch of `MaybeUninit`s, which do not require initialisation
            let mut rects: [MaybeUninit<Rect>; Display::SIZE] =
                unsafe { MaybeUninit::uninit().assume_init() };

            for (i, item) in rects.iter_mut().enumerate() {
                *item = MaybeUninit::new(Rect::from_center(
                    (
                        ((pixel_size_x * (i % Display::VIDEO_WIDTH) as u32) + pixel_size_x / 2)
                            as i32,
                        ((pixel_size_y * (i / Display::VIDEO_WIDTH) as u32) + pixel_size_y / 2)
                            as i32,
                    ),
                    pixel_size_x,
                    pixel_size_y,
                ));
            }
            // Safety:
            // Everything is now initialised. Transmute the array to the initialised type.
            unsafe { std::mem::transmute::<_, [Rect; Display::SIZE]>(rects) }
        };
        Self { canvas, rects }
    }

    #[inline(always)]
    fn pixel_size(canvas: &Canvas<Window>) -> (u32, u32) {
        let (window_width, window_height) = canvas.window().size();
        (
            (window_width as usize / Display::VIDEO_WIDTH) as u32,
            (window_height as usize / Display::VIDEO_HEIGHT) as u32,
        )
    }

    pub(crate) fn update_from_video(&mut self, video: &[u32; Display::SIZE]) {
        debug_assert_eq!(video.len(), self.rects.len());

        self.canvas.clear();

        for (pixel, rect) in video.iter().zip(self.rects.iter()) {
            if *pixel == 0 {
                self.canvas.set_draw_color(Self::DISPLAY_OFF_PIXEL)
            } else if *pixel == 1 {
                self.canvas.set_draw_color(Self::DISPLAY_ON_PIXEL)
            } else {
                unreachable!("Unknown pixel colour")
            }
            self.canvas.fill_rect(*rect).unwrap();
        }

        self.canvas.present();
    }
}

fn run_chip8(sdl_context: sdl2::Sdl, mut chip8: chip8::Chip8, cycle_delay: u32) {
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut canvas = {
        let mut canvas = sdl_context
            .video()
            .unwrap()
            .window("chip8", 800, 600)
            .position_centered()
            .build()
            .unwrap()
            .into_canvas()
            .build()
            .unwrap();

        canvas.clear();
        canvas.present();
        canvas
    };

    let cycle_delay = Duration::new(0, cycle_delay * 1_000_000); // 10 ms
    let mut last_cycle_time = Instant::now();
    let mut dt: Duration;
    let mut keys_pressed = Vec::new();
    let mut keys_up = Vec::new();

    let mut screen = Screen::new(&mut canvas);

    'running: loop {
        dt = Instant::now().duration_since(last_cycle_time);

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if let Some(k) = keycode_to_idx(key) {
                        keys_pressed.push(k);
                    }
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    if let Some(k) = keycode_to_idx(key) {
                        keys_up.push(k);
                    }
                }
                _ => {}
            }
        }

        if dt > cycle_delay {
            last_cycle_time = Instant::now();

            for i in keys_pressed.drain(..) {
                debug!("Pressing {}", i);
                chip8.press_key(i);
            }

            chip8.cycle();

            for i in keys_up.drain(..) {
                debug!("Lifting {}", i);
                chip8.lift_key(i);
            }
        }

        if chip8.is_dirty() {
            screen.update_from_video(chip8.get_video());
            chip8.set_clean();
        }
    }
}

/// Chip8 emulator
#[derive(Parser, Debug)]
#[command(author, version,about, long_about=None)]
struct Args {
    /// Rom path
    #[arg(short, long)]
    rom_path: String,

    /// Cycle delay in milliseconds
    #[arg(short, long, default_value_t = 10)]
    cycle_delay: u32,
}

fn main() {
    env_logger::init();
    let sdl_context = sdl2::init().unwrap();

    let args = Args::parse();

    let chip8 = chip8::Chip8::read_rom(&args.rom_path).unwrap();
    run_chip8(sdl_context, chip8, args.cycle_delay);
}
