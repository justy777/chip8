#![allow(clippy::cast_lossless)]

use bytes::Bytes;
use chip8_core::{Chip8, VIDEO_HEIGHT, VIDEO_WIDTH};
use iced::keyboard::Key;
use iced::widget::container::Style;
use iced::widget::image::{FilterMethod, Handle};
use iced::{Color, Element, Length, Size, Subscription, Task, keyboard, widget, window};
use std::ops::Div;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::{env, io};

fn main() -> iced::Result {
    let args: Vec<String> = env::args().collect();

    let video_scale = f32::from_str(&args[1])
        .unwrap_or_else(|_| panic!("Failed to parse video scale {}", &args[1]));
    let refresh_rate = u32::from_str(&args[2])
        .unwrap_or_else(|_| panic!("Failed to parse refresh rate {}", &args[2]));
    let rom_path = args[3].clone();

    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .window_size(Size::new(
            VIDEO_WIDTH as f32 * video_scale,
            VIDEO_HEIGHT as f32 * video_scale,
        ))
        .run_with(move || {
            let app = App::new(refresh_rate);
            (app, Task::perform(load_file(rom_path), Message::RomLoaded))
        })
}

#[derive(Debug, Clone)]
enum Message {
    RomLoaded(Result<Arc<Vec<u8>>, io::ErrorKind>),
    KeyPress(Key),
    KeyRelease(Key),
    Step,
    Exit,
}

struct App {
    emulator: Chip8,
    refresh_rate: u32,
    running: bool,
    error: Option<io::ErrorKind>,
}

impl App {
    fn new(refresh_rate: u32) -> Self {
        let emulator = Chip8::new();
        Self {
            emulator,
            refresh_rate,
            running: false,
            error: None,
        }
    }

    fn title(&self) -> String {
        String::from("CHIP-8 Emulator")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RomLoaded(Ok(rom)) => {
                self.emulator.load(&rom);
                self.running = true;
                Task::none()
            }
            Message::RomLoaded(Err(err)) => {
                self.error = Some(err);
                Task::none()
            }
            Message::KeyPress(key) => {
                if let Key::Character(c) = key.as_ref() {
                    if let Some(key_idx) = get_key_idx(c) {
                        self.emulator.set_key(key_idx, true);
                    }
                }
                Task::none()
            }
            Message::KeyRelease(key) => {
                if let Key::Character(c) = key.as_ref() {
                    if let Some(key_idx) = get_key_idx(c) {
                        self.emulator.set_key(key_idx, false);
                    }
                }
                Task::none()
            }
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
        let pixels = convert_to_rgba(self.emulator.framebuffer());
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
        let every = if self.running {
            iced::time::every(Duration::from_secs(1).div(self.refresh_rate)).map(|_| Message::Step)
        } else {
            Subscription::none()
        };
        Subscription::batch(vec![
            every,
            keyboard::on_key_press(|key, _| Some(Message::KeyPress(key))),
            keyboard::on_key_release(|key, _| Some(Message::KeyRelease(key))),
        ])
    }
}

async fn load_file(path: impl AsRef<Path>) -> Result<Arc<Vec<u8>>, io::ErrorKind> {
    tokio::fs::read(path)
        .await
        .map(Arc::new)
        .map_err(|err| err.kind())
}

fn convert_to_rgba(data: &[bool]) -> Vec<u8> {
    data.iter()
        .map(|&pixel| if pixel { Color::WHITE } else { Color::BLACK })
        .flat_map(Color::into_rgba8)
        .collect()
}

const KEYPAD_MAPPING: [(&str, usize); 16] = [
    ("1", 0x1),
    ("2", 0x2),
    ("3", 0x3),
    ("4", 0xC),
    ("Q", 0x4),
    ("W", 0x5),
    ("E", 0x6),
    ("R", 0xD),
    ("A", 0x7),
    ("S", 0x8),
    ("D", 0x9),
    ("F", 0xE),
    ("Z", 0xA),
    ("X", 0x0),
    ("C", 0xB),
    ("V", 0xF),
];

fn get_key_idx(key: &str) -> Option<usize> {
    KEYPAD_MAPPING
        .iter()
        .find(|&&(k, _)| k.eq_ignore_ascii_case(key))
        .map(|&(_, v)| v)
}
