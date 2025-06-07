#![allow(clippy::cast_lossless)]

use bytes::Bytes;
use chip8_core::{Chip8, VIDEO_HEIGHT, VIDEO_WIDTH};
use iced::widget::container::Style;
use iced::widget::image::{FilterMethod, Handle};
use iced::{Color, Element, Length, Size, Subscription, Task, widget, window};
use std::env;
use std::ops::Div;
use std::str::FromStr;
use std::time::Duration;

fn main() -> iced::Result {
    let args: Vec<String> = env::args().collect();

    let video_scale = f32::from_str(&args[1])
        .unwrap_or_else(|_| panic!("Failed to parse video scale {}", &args[1]));
    let refresh_rate = u32::from_str(&args[2])
        .unwrap_or_else(|_| panic!("Failed to parse refresh rate {}", &args[2]));
    let rom_path = args[3].clone();

    iced::application("CHIP-8 Emulator", App::update, App::view)
        .subscription(App::subscription)
        .window_size(Size::new(
            VIDEO_WIDTH as f32 * video_scale,
            VIDEO_HEIGHT as f32 * video_scale,
        ))
        .run_with(move || {
            let app = App::new(&rom_path, refresh_rate);
            (app, Task::none())
        })
}

#[derive(Debug, Clone)]
enum Message {
    Step,
    Exit,
}

struct App {
    emulator: Chip8,
    refresh_rate: u32,
}

impl App {
    fn new(rom_path: &str, refresh_rate: u32) -> Self {
        let mut emulator = Chip8::new();
        let rom = std::fs::read(rom_path)
            .unwrap_or_else(|_| panic!("Failed to load rom from file {rom_path}"));
        emulator.load_rom(&rom);
        Self {
            emulator,
            refresh_rate,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Step => {
                self.emulator
                    .emulate()
                    .expect("Failed while emulating Chip8 instruction");
                Task::none()
            }
            Message::Exit => window::get_latest().and_then(window::close),
        }
    }

    fn view(&self) -> Element<Message> {
        let pixels = convert_to_rgba(&self.emulator.video);
        let content = widget::image(Handle::from_rgba(
            VIDEO_WIDTH as u32,
            VIDEO_HEIGHT as u32,
            Bytes::from_owner(pixels),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .filter_method(FilterMethod::Nearest);

        widget::container(content)
            .style(|_| Style::from(Color::BLACK))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_secs(1).div(self.refresh_rate)).map(|_| Message::Step)
    }
}

fn convert_to_rgba(
    data: &[u32; VIDEO_WIDTH * VIDEO_HEIGHT],
) -> [u8; VIDEO_WIDTH * VIDEO_HEIGHT * 4] {
    let mut buf = [0; VIDEO_WIDTH * VIDEO_HEIGHT * 4];
    for i in 0..(VIDEO_WIDTH * VIDEO_HEIGHT) {
        buf[(i * 4)..][..4].copy_from_slice(&data[i].to_be_bytes());
    }
    buf
}
