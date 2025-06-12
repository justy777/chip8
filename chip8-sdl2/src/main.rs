#![allow(clippy::cast_lossless)]

use anyhow::{Context, anyhow};
use chip8_core::{Chip8, VIDEO_HEIGHT, VIDEO_WIDTH};
use sdl2::Sdl;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::TextureAccess;
use std::env;
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    let video_scale = u32::from_str(&args[1])
        .with_context(|| format!("Failed to parse video scale {}", &args[1]))?;
    let cycle_delay = u128::from_str(&args[2])
        .with_context(|| format!("Failed to parse cycle delay {}", &args[2]))?;
    let rom_path = &args[3];

    let sdl_context = sdl2::init()
        .map_err(|err| anyhow!(err))
        .context("Failed to init SDL")?;

    let video_subsystem = sdl_context
        .video()
        .map_err(|err| anyhow!(err))
        .context("Failed to init SDL video subsystem")?;

    let window = video_subsystem
        .window(
            "CHIP-8 Emulator",
            (VIDEO_WIDTH as u32) * video_scale,
            (VIDEO_HEIGHT as u32) * video_scale,
        )
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .context("Failed to build SDL window")?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .build()
        .context("Failed to build SDL canvas")?;

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture(
            PixelFormatEnum::RGBA8888,
            TextureAccess::Streaming,
            VIDEO_WIDTH as u32,
            VIDEO_HEIGHT as u32,
        )
        .context("Failed to build SDL texture")?;

    let rom = std::fs::read(rom_path)
        .with_context(|| format!("Failed to load rom from file {rom_path}"))?;

    let mut chip8 = Chip8::new();
    chip8.load_rom(&rom);

    let video_pitch = size_of::<u32>() * VIDEO_WIDTH;

    let mut last_cycle_time = std::time::Instant::now();
    let mut quit = false;

    while !quit {
        quit = process_input(&sdl_context, &mut chip8.keypad)?;

        let dt = last_cycle_time.elapsed().as_millis();

        if dt > cycle_delay {
            last_cycle_time = std::time::Instant::now();

            chip8
                .emulate()
                .context("Failed while emulating Chip8 instruction")?;

            texture
                .update(None, &convert_to_rgba(&chip8.video), video_pitch)
                .map_err(|err| anyhow!(err))
                .context("Failed to update SDL texture")?;

            canvas.clear();

            canvas
                .copy(&texture, None, None)
                .map_err(|err| anyhow!(err))
                .context("Drawing to SDL canvas failed")?;

            canvas.present();
        }
    }

    Ok(())
}

#[derive(Debug)]
enum ProcessInputError {
    EventPump(String),
}

impl From<String> for ProcessInputError {
    fn from(s: String) -> Self {
        Self::EventPump(s)
    }
}

impl std::fmt::Display for ProcessInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::EventPump(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for ProcessInputError {}

fn process_input(sdl_context: &Sdl, keys: &mut [bool]) -> Result<bool, ProcessInputError> {
    let mut quit = false;

    for event in sdl_context
        .event_pump()
        .map_err(ProcessInputError::from)?
        .poll_iter()
    {
        match event {
            Event::Quit { .. } => {
                quit = true;
                break;
            }
            Event::KeyDown {
                keycode: Some(keycode),
                ..
            } => match keycode {
                Keycode::Escape => {
                    quit = true;
                    break;
                }
                keycode => {
                    if let Some(keycode) = get_keycode(&keycode.name()) {
                        keys[keycode] = true;
                    }
                }
            },
            Event::KeyUp {
                keycode: Some(keycode),
                ..
            } => {
                if let Some(keycode) = get_keycode(&keycode.name()) {
                    keys[keycode] = false;
                }
            }
            _ => {}
        }
    }

    Ok(quit)
}

fn convert_to_rgba(data: &[u32]) -> Vec<u8> {
    data.iter().flat_map(|&pixel| pixel.to_be_bytes()).collect()
}

const KEYPAD_MAPPING: [(&str, usize); 16] = [
    ("1", 0x1),
    ("2", 0x2),
    ("3", 0x3),
    ("4", 0xC),
    ("q", 0x4),
    ("w", 0x5),
    ("e", 0x6),
    ("r", 0xD),
    ("a", 0x7),
    ("s", 0x8),
    ("d", 0x9),
    ("f", 0xE),
    ("z", 0xA),
    ("x", 0x0),
    ("c", 0xB),
    ("v", 0xF),
];

fn get_keycode(key: &str) -> Option<usize> {
    KEYPAD_MAPPING
        .iter()
        .find(|&&(k, _)| k.eq_ignore_ascii_case(key))
        .map(|&(_, v)| v)
}
