#![allow(clippy::cast_lossless)]

use bytes::Bytes;
use chip8_core::{Chip8, VIDEO_HEIGHT, VIDEO_WIDTH};
use iced::alignment::Vertical;
use iced::keyboard::Key;
use iced::widget::container::Style;
use iced::widget::image::{FilterMethod, Handle};
use iced::widget::{
    Button, Checkbox, button, checkbox, column as col, container, horizontal_space, image, text,
};
use iced::{Color, Element, Length, Size, Subscription, Task, keyboard, window};
use iced_aw::menu::DrawPath;
use iced_aw::{menu_bar, menu_items};
use rfd::AsyncFileDialog;
use std::io;
use std::ops::Div;
use std::path::{Path, PathBuf};
use std::time::Duration;

type Item<'a, Message> = iced_aw::menu::Item<'a, Message, iced::Theme, iced::Renderer>;
type Menu<'a, Message> = iced_aw::menu::Menu<'a, Message, iced::Theme, iced::Renderer>;

const VIDEO_SCALE: f32 = 10.0;

const TIMER_HZ: u32 = 60;

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .window(iced::window::Settings {
            size: Size::new(
                VIDEO_WIDTH as f32 * VIDEO_SCALE,
                VIDEO_HEIGHT as f32 * VIDEO_SCALE + 30.0,
            ),
            min_size: Some(Size::new(180.0, 180.0)),
            ..Default::default()
        })
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Open,
    RomSelected(Option<PathBuf>),
    RomLoaded(Result<Vec<u8>, io::ErrorKind>),
    KeyPressed(Key),
    KeyReleased(Key),
    PauseToggled(bool),
    Stop,
    Emulate,
    TickTimer,
    Exit,
}

struct App {
    emulator: Chip8,
    clock_speed: u32,
    is_loaded: bool,
    is_paused: bool,
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
            is_loaded: false,
            is_paused: false,
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
                if self.is_loaded {
                    self.emulator.reset();
                }
                self.emulator.load(&rom);
                self.is_loaded = true;
                Task::none()
            }
            Message::RomLoaded(Err(err)) => {
                self.error = Some(err);
                Task::none()
            }
            Message::KeyPressed(key) => {
                if let Key::Character(c) = key.as_ref() {
                    if let Some(key_idx) = get_key_idx(c) {
                        self.emulator.set_key(key_idx, true);
                    }
                }
                Task::none()
            }
            Message::KeyReleased(key) => {
                if let Key::Character(c) = key.as_ref() {
                    if let Some(key_idx) = get_key_idx(c) {
                        self.emulator.set_key(key_idx, false);
                    }
                }
                Task::none()
            }
            Message::PauseToggled(checked) => {
                self.is_paused = checked;
                Task::none()
            }
            Message::Stop => {
                self.is_loaded = false;
                self.is_paused = false;
                self.emulator.reset();
                Task::none()
            }
            Message::Emulate => {
                if self.is_loaded {
                    self.emulator
                        .emulate()
                        .expect("Failed while emulating Chip8 instruction");
                }
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
        let menu_bar = menu_bar!((
            menu_header("File"),
            menu(menu_items!((menu_item("Open").on_press(Message::Open))(
                menu_item("Exit").on_press(Message::Exit)
            )))
        )(
            menu_header("Emulation"),
            menu(menu_items!((menu_checkbox("Pause", self.is_paused)
                .on_toggle_maybe(if self.is_loaded {
                    Some(Message::PauseToggled)
                } else {
                    None
                }))(
                menu_item("Stop").on_press_maybe(if self.is_loaded {
                    Some(Message::Stop)
                } else {
                    None
                })
            )))
        ))
        .draw_path(DrawPath::Backdrop)
        .width(Length::Fill);

        let pixels = convert_to_rgba(self.emulator.framebuffer());
        let content = image(Handle::from_rgba(
            VIDEO_WIDTH as u32,
            VIDEO_HEIGHT as u32,
            Bytes::from_owner(pixels),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .filter_method(FilterMethod::Nearest);

        container(col![menu_bar, horizontal_space().height(5), content])
            .style(|_| Style::from(Color::BLACK))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let emulate = if self.is_loaded && !self.is_paused {
            iced::time::every(Duration::from_secs(1).div(self.clock_speed))
                .map(|_| Message::Emulate)
        } else {
            Subscription::none()
        };
        let timer = if self.is_loaded && !self.is_paused {
            iced::time::every(Duration::from_secs(1).div(TIMER_HZ)).map(|_| Message::TickTimer)
        } else {
            Subscription::none()
        };
        Subscription::batch(vec![
            emulate,
            timer,
            keyboard::on_key_press(|key, _| Some(Message::KeyPressed(key))),
            keyboard::on_key_release(|key, _| Some(Message::KeyReleased(key))),
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

fn menu(items: Vec<Item<Message>>) -> Menu<Message> {
    Menu::new(items).max_width(120.0).offset(5.0).spacing(5.0)
}

fn menu_header(label: &str) -> Button<Message> {
    menu_button(label).width(Length::Shrink)
}

fn menu_item(label: &str) -> Button<Message> {
    menu_button(label).width(Length::Fill)
}

fn menu_button(label: &str) -> Button<Message> {
    button(text(label).align_y(Vertical::Center))
        .padding([4, 8])
        .style(|_, _| Default::default())
}

fn menu_checkbox(label: &str, is_checked: bool) -> Checkbox<Message> {
    checkbox(label, is_checked).width(Length::Fill)
}
