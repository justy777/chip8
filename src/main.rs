use chip8::chip8::{Chip8, VIDEO_HEIGHT, VIDEO_WIDTH};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::TextureAccess;
use sdl2::Sdl;
use std::env;
use std::str::FromStr;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    let video_scale = u32::from_str(&args[1]).map_err(|e| e.to_string())?;
    let cycle_delay = u128::from_str(&args[2]).map_err(|e| e.to_string())?;
    let rom_filename = args[3].clone();

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
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
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture(
            PixelFormatEnum::RGBA8888,
            TextureAccess::Streaming,
            VIDEO_WIDTH as u32,
            VIDEO_HEIGHT as u32,
        )
        .map_err(|e| e.to_string())?;

    let mut chip8 = Chip8::new();
    chip8.load_rom(&rom_filename);

    let video_pitch = size_of::<u32>() * VIDEO_WIDTH;

    let mut last_cycle_time = std::time::Instant::now();
    let mut quit = false;

    while !quit {
        quit = process_input(&sdl_context, &mut chip8.keypad)?;

        let dt = last_cycle_time.elapsed().as_millis();

        if dt > cycle_delay {
            last_cycle_time = std::time::Instant::now();

            chip8.cycle();

            texture
                .update(None, &convert(&chip8.video), video_pitch)
                .map_err(|e| e.to_string())?;
            canvas.clear();
            canvas.copy(&texture, None, None)?;
            canvas.present();
        }
    }

    Ok(())
}

fn process_input(sdl_context: &Sdl, keys: &mut [u8]) -> Result<bool, String> {
    let mut quit = false;

    for event in sdl_context.event_pump()?.poll_iter() {
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
                Keycode::X => {
                    keys[0] = 1;
                }
                Keycode::NUM_1 => {
                    keys[1] = 1;
                }
                Keycode::NUM_2 => {
                    keys[2] = 1;
                }
                Keycode::NUM_3 => {
                    keys[3] = 1;
                }
                Keycode::Q => {
                    keys[4] = 1;
                }
                Keycode::W => {
                    keys[5] = 1;
                }
                Keycode::E => {
                    keys[6] = 1;
                }
                Keycode::A => {
                    keys[7] = 1;
                }
                Keycode::S => {
                    keys[8] = 1;
                }
                Keycode::D => {
                    keys[9] = 1;
                }
                Keycode::Z => {
                    keys[10] = 1;
                }
                Keycode::C => {
                    keys[11] = 1;
                }
                Keycode::NUM_4 => {
                    keys[12] = 1;
                }
                Keycode::R => {
                    keys[13] = 1;
                }
                Keycode::F => {
                    keys[14] = 1;
                }
                Keycode::V => {
                    keys[15] = 1;
                }
                _ => {}
            },
            Event::KeyUp {
                keycode: Some(keycode),
                ..
            } => match keycode {
                Keycode::X => {
                    keys[0] = 0;
                }
                Keycode::NUM_1 => {
                    keys[1] = 0;
                }
                Keycode::NUM_2 => {
                    keys[2] = 0;
                }
                Keycode::NUM_3 => {
                    keys[3] = 0;
                }
                Keycode::Q => {
                    keys[4] = 0;
                }
                Keycode::W => {
                    keys[5] = 0;
                }
                Keycode::E => {
                    keys[6] = 0;
                }
                Keycode::A => {
                    keys[7] = 0;
                }
                Keycode::S => {
                    keys[8] = 0;
                }
                Keycode::D => {
                    keys[9] = 0;
                }
                Keycode::Z => {
                    keys[10] = 0;
                }
                Keycode::C => {
                    keys[11] = 0;
                }
                Keycode::NUM_4 => {
                    keys[12] = 0;
                }
                Keycode::R => {
                    keys[13] = 0;
                }
                Keycode::F => {
                    keys[14] = 0;
                }
                Keycode::V => {
                    keys[15] = 0;
                }
                _ => {}
            },
            _ => {}
        }
    }

    Ok(quit)
}

fn convert(data: &[u32; 2048]) -> [u8; 8192] {
    let mut res = [0; 8192];
    for i in 0..2048 {
        res[4 * i..][..4].copy_from_slice(&data[i].to_be_bytes());
    }
    res
}
