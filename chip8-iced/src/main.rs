#![allow(clippy::cast_lossless)]

use bytes::Bytes;
use chip8_core::{Chip8, VIDEO_HEIGHT, VIDEO_WIDTH};
use iced::keyboard::Key;
use iced::widget::container::Style;
use iced::widget::image::{FilterMethod, Handle};
use iced::{Color, Element, Length, Size, Subscription, Task, keyboard, widget, window};
use rfd::AsyncFileDialog;
use std::io;
use std::ops::Div;
use std::path::{Path, PathBuf};
use std::time::Duration;

const VIDEO_SCALE: f32 = 10.0;

const TIMER_HZ: u32 = 60;

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .window_size(Size::new(
            VIDEO_WIDTH as f32 * VIDEO_SCALE,
            VIDEO_HEIGHT as f32 * VIDEO_SCALE + 30.0,
        ))
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Open,
    RomSelected(Option<PathBuf>),
    RomLoaded(Result<Vec<u8>, io::ErrorKind>),
    KeyPress(Key),
    KeyRelease(Key),
    Start,
    Pause,
    Stop,
    Emulate,
    TickTimer,
    Exit,
}

struct App {
    emulator: Chip8,
    clock_speed: u32,
    running: bool,
    loaded: bool,
    error: Option<io::ErrorKind>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    fn new() -> Self {
        let emulator = Chip8::new();
        Self {
            emulator,
            clock_speed: 500,
            running: false,
            loaded: false,
            error: None,
        }
    }

    fn title(&self) -> String {
        String::from("CHIP-8 Emulator")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open => Task::perform(pick_file(), Message::RomSelected),
            Message::RomSelected(path) => {
                if let Some(path) = path {
                    Task::perform(load_file(path), Message::RomLoaded)
                } else {
                    Task::none()
                }
            }
            Message::RomLoaded(Ok(rom)) => {
                if self.loaded {
                    self.emulator.reset();
                }
                self.emulator.load(&rom);
                self.running = true;
                self.loaded = true;
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
            Message::Start => {
                self.running = true;
                Task::none()
            }
            Message::Pause => {
                self.running = false;
                Task::none()
            }
            Message::Stop => {
                self.running = false;
                self.loaded = false;
                self.emulator.reset();
                Task::none()
            }
            Message::Emulate => {
                self.emulator
                    .emulate()
                    .expect("Failed while emulating Chip8 instruction");
                Task::none()
            }
            Message::TickTimer => {
                self.emulator.tick_timers();
                Task::none()
            }
            Message::Exit => window::get_latest().and_then(window::close),
        }
    }

    fn view(&self) -> Element<Message> {
        let controls = widget::row![
            widget::button("Open").on_press(Message::Open),
            widget::button("Start").on_press_maybe(if self.loaded && !self.running {
                Some(Message::Start)
            } else {
                None
            }),
            widget::button("Pause").on_press_maybe(if self.loaded && self.running {
                Some(Message::Pause)
            } else {
                None
            }),
            widget::button("Stop").on_press_maybe(if self.loaded {
                Some(Message::Stop)
            } else {
                None
            }),
            widget::button("Exit").on_press(Message::Exit)
        ]
        .width(Length::Fill);

        let pixels = convert_to_rgba(self.emulator.framebuffer());
        let content = widget::image(Handle::from_rgba(
            VIDEO_WIDTH as u32,
            VIDEO_HEIGHT as u32,
            Bytes::from_owner(pixels),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .filter_method(FilterMethod::Nearest);

        widget::container(widget::column![controls, content])
            .style(|_| Style::from(Color::BLACK))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let emulate = if self.loaded && self.running {
            iced::time::every(Duration::from_secs(1).div(self.clock_speed))
                .map(|_| Message::Emulate)
        } else {
            Subscription::none()
        };
        let timer = if self.loaded && self.running {
            iced::time::every(Duration::from_secs(1).div(TIMER_HZ)).map(|_| Message::TickTimer)
        } else {
            Subscription::none()
        };
        Subscription::batch(vec![
            emulate,
            timer,
            keyboard::on_key_press(|key, _| Some(Message::KeyPress(key))),
            keyboard::on_key_release(|key, _| Some(Message::KeyRelease(key))),
        ])
    }
}

async fn pick_file() -> Option<PathBuf> {
    AsyncFileDialog::new()
        .set_title("Select ROM")
        .pick_file()
        .await
        .map(PathBuf::from)
}

async fn load_file(path: impl AsRef<Path>) -> Result<Vec<u8>, io::ErrorKind> {
    tokio::fs::read(path).await.map_err(|err| err.kind())
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
