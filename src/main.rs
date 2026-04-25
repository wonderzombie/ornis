use iced::{
    Task,
    widget::{
        Column, button, column,
        operation::focus,
        text_editor::{self, Content},
        text_input,
    },
};

#[derive(Debug, Default, Clone)]
pub struct Ornis {
    // UI state
    scrollback: Vec<String>,
    input: String,
    content: Content,
}

#[derive(Debug, Default, Clone)]
enum Message {
    #[default]
    Noop,
    EnterPressed,
    ContentChanged(String),
    WindowOpened,
    WindowClosed,
}

fn update(state: &mut Ornis, message: Message) -> Task<Message> {
    match message {
        Message::EnterPressed => {
            state.scrollback.push(state.input.clone());
            state.content = Content::with_text(state.scrollback.join("\n").as_str());
            state.input.clear();
        }
        Message::ContentChanged(new_input) => {
            state.input = new_input;
        }
        Message::WindowOpened => {
            state.scrollback = vec!["=== welcome to ornith ===".into()];
            state.content = Content::with_text(state.scrollback.join("\n").as_str());
            return focus(MAIN_INPUT_ID);
        }
        _ => return Task::none(),
    }
    Task::none()
}

const MAIN_INPUT_ID: &'static str = "main_input";

fn view(state: &Ornis) -> Column<'_, Message> {
    column![
        text_editor::TextEditor::new(&state.content)
            .size(14)
            .height(iced::FillPortion(9)),
        text_input::TextInput::new("commands go here", &state.input)
            .id(MAIN_INPUT_ID)
            .padding(10)
            .size(14)
            .on_input(Message::ContentChanged)
            .on_submit(Message::EnterPressed),
        button("enter").on_press(Message::EnterPressed),
    ]
    .spacing(10)
    .into()
}

fn subscription(_state: &Ornis) -> iced::Subscription<Message> {
    iced::window::events().map(|(_, event)| match event {
        iced::window::Event::Opened { .. } => Message::WindowOpened,
        iced::window::Event::Closed => Message::WindowClosed,
        _ => Message::Noop,
    })
}

fn main() -> iced::Result {
    iced::application(Ornis::default, update, view)
        .theme(iced::Theme::Ferra)
        .subscription(subscription)
        .run()
}
