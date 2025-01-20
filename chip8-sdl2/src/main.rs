#![allow(clippy::cast_lossless)]

use anyhow::{anyhow, Context};
use chip8_core::{Chip8, VIDEO_HEIGHT, VIDEO_WIDTH};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::TextureAccess;
use sdl2::Sdl;
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
                .update(None, &convert(&chip8.video), video_pitch)
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

fn process_input(sdl_context: &Sdl, keys: &mut [u8]) -> Result<bool, ProcessInputError> {
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
                keycode => update_keys(keys, keycode, 1),
            },
            Event::KeyUp {
                keycode: Some(keycode),
                ..
            } => update_keys(keys, keycode, 0),
            _ => {}
        }
    }

    Ok(quit)
}

fn update_keys(keys: &mut [u8], keycode: Keycode, value: u8) {
    match keycode {
        Keycode::X => {
            keys[0] = value;
        }
        Keycode::NUM_1 => {
            keys[1] = value;
        }
        Keycode::NUM_2 => {
            keys[2] = value;
        }
        Keycode::NUM_3 => {
            keys[3] = value;
        }
        Keycode::Q => {
            keys[4] = value;
        }
        Keycode::W => {
            keys[5] = value;
        }
        Keycode::E => {
            keys[6] = value;
        }
        Keycode::A => {
            keys[7] = value;
        }
        Keycode::S => {
            keys[8] = value;
        }
        Keycode::D => {
            keys[9] = value;
        }
        Keycode::Z => {
            keys[10] = value;
        }
        Keycode::C => {
            keys[11] = value;
        }
        Keycode::NUM_4 => {
            keys[12] = value;
        }
        Keycode::R => {
            keys[13] = value;
        }
        Keycode::F => {
            keys[14] = value;
        }
        Keycode::V => {
            keys[15] = value;
        }
        _ => {}
    }
}

fn convert(data: &[u32; 2048]) -> [u8; 8192] {
    let mut res = [0; 8192];
    for i in 0..2048 {
        res[4 * i..][..4].copy_from_slice(&data[i].to_be_bytes());
    }
    res
}
