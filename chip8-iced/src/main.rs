#![allow(clippy::cast_lossless)]

use bytes::Bytes;
use chip8_core::{Chip8, VIDEO_HEIGHT, VIDEO_WIDTH};
use iced::widget::container::Style;
use iced::widget::image::{FilterMethod, Handle};
use iced::{Color, Element, Length, Size, Subscription, Task, widget, window};
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
                self.emulator.load_rom(&rom);
                self.running = true;
                Task::none()
            }
            Message::RomLoaded(Err(err)) => {
                self.error = Some(err);
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
        if self.running {
            iced::time::every(Duration::from_secs(1).div(self.refresh_rate)).map(|_| Message::Step)
        } else {
            Subscription::none()
        }
    }
}

async fn load_file(path: impl AsRef<Path>) -> Result<Arc<Vec<u8>>, io::ErrorKind> {
    tokio::fs::read(path)
        .await
        .map(Arc::new)
        .map_err(|err| err.kind())
}

fn convert_to_rgba(data: &[u32]) -> Vec<u8> {
    let mut buf = vec![0; data.len() * 4];
    for i in 0..data.len() {
        buf[(i * 4)..][..4].copy_from_slice(&data[i].to_be_bytes());
    }
    buf
}
