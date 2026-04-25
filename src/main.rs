use iced::{
    Element, Task, color,
    widget::{column, text_editor, text_editor::Content, text_input},
};

#[derive(Debug, Default)]
pub struct Ornis {
    // UI state
    scrollback: Vec<String>,
    input: String,
    content: Content,
}

const MAIN_INPUT_ID: &str = "main_input";

#[derive(Debug, Clone)]
pub enum Message {
    ContentChanged(String),
    EnterPressed,
    WindowClosed,
    WindowOpened,
    Noop,
}

impl Ornis {
    fn new() -> (Self, Task<Message>) {
        let w: Ornis = Self::default();
        let task = text_input::focus(MAIN_INPUT_ID);
        (w, task)
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Editor and input take up the full width and height of the window.
        // Output from the Wayline system will be displayed in the editor (scrollback) area.
        column![
            // Scrollback
            text_editor(&self.content)
                .padding(10)
                .size(14)
                .style(|theme, status| {
                    let mut style = iced::widget::text_editor::default(theme, status);
                    style.value = color!(0xEEEEEE);
                    style
                })
                .height(iced::Length::FillPortion(9)),
            // Input area
            text_input("enter command", &self.input)
                .id(MAIN_INPUT_ID)
                .padding(10)
                .size(14)
                .on_input(Message::ContentChanged)
                .on_submit(Message::EnterPressed),
        ]
        .spacing(10)
        .into()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ContentChanged(new_content) => {
                self.input = new_content;
                Task::none()
            }
            Message::EnterPressed => {
                self.scrollback.push(self.input.clone());
                self.content = Content::with_text(self.scrollback.join("\n").as_str());
                self.input.clear();
                Task::none()
            }
            Message::WindowClosed => todo!(),
            Message::WindowOpened => {
                self.update_scrollback("Window opened".to_string());
                Task::none()
            }
            Message::Noop => todo!(),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::window::events().map(|(_, event)| match event {
            iced::window::Event::Opened { .. } => Message::WindowOpened,
            iced::window::Event::Closed => Message::WindowClosed,
            _ => Message::Noop,
        })
    }

    fn update_scrollback<S: Into<String>>(&mut self, new_line: S) {
        self.scrollback.push(new_line.into());
        let new_content = self.scrollback.join("\n");
        self.content = Content::with_text(&new_content);
    }
}

fn main() {
    iced::application("ornis", Ornis::update, Ornis::view)
        .theme(theme)
        .subscription(Ornis::subscription)
        .run_with(Ornis::new)
        .expect("unable to run application")
}

fn theme(_state: &Ornis) -> iced::Theme {
    iced::Theme::Ferra
}
