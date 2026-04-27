mod rpc;

use anyhow::anyhow;
use iced::{
    Event,
    Length::{Fill, FillPortion},
    Task, event,
    keyboard::{Key, key::Named},
    widget::{
        Container, Row, button, column,
        operation::{focus, snap_to_end},
        row, text,
        text_editor::{self, Action, Content},
        text_input,
    },
};
use log::{LevelFilter, error, info, trace};
use reqwest::{
    self,
    blocking::{self},
};
use serde_json::Value;
use simple_logger::{self};

use crate::rpc::*;

#[derive(Debug, Clone)]
pub struct Ornis {
    // UI state
    scrollback: Vec<String>,
    text_input: String,
    text_content: Content,
    current_ns: String,
    command_hist: Vec<String>,
    hist_idx: usize,

    last_response: String,
}

const URL: &'static str = "http://localhost:15702";

const CRATE_PATH: &'static str = "wanderrust";

impl Default for Ornis {
    fn default() -> Self {
        Self {
            scrollback: Default::default(),
            text_input: Default::default(),
            text_content: Default::default(),
            current_ns: CRATE_PATH.into(),
            command_hist: Default::default(),
            hist_idx: Default::default(),
            last_response: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone)]
enum Message {
    #[default]
    Noop,
    EnterPressed,
    ContentChanged(String),
    WindowOpened,
    WindowClosed,
    QueryResultsReady(BrpQueryResponse),
    LoadRegistry(BrpRegistryResponse),
    OutputChanged,
    CommandPrev,
    CommandNext,
    UpdatePane,
}

macro_rules! enum_with_str {
    ( $enum_name:ident, $( $variant:ident ),* $(,)?  ) => {
        #[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash)]
        pub enum $enum_name {
            #[default]
            Unset,
            $( $variant, )*
        }

        #[allow(dead_code)]
        impl $enum_name {
            pub fn all() -> &'static [$enum_name] {
                &[ $( $enum_name::$variant, )* ]
            }

            pub fn pairs() -> &'static [(&'static str, $enum_name)] {
                &[ $( (stringify!($variant), $enum_name::$variant), )* ]
            }

            pub fn from_name(value: impl AsRef<str>) -> Option<$enum_name> {
                Self::pairs().iter().find(|(s, _)| value.as_ref() == *s).copied().map(|(_, v)| v)
            }
        }
    };
}

impl Command {
    fn from_str(s: impl AsRef<str>) -> Command {
        let words: Vec<&str> = s.as_ref().split_ascii_whitespace().into_iter().collect();
        let Some(first) = words.first() else {
            return Command::Unset;
        };

        match *first {
            "wq" | "q" | "world.query" | "query" => Command::WorldQuery,
            _ => Command::Unset,
        }
    }
}

enum_with_str!(Command, WorldQuery, ListResources, ListComponent);

fn handle_command(ornis: &mut Ornis) -> Message {
    let words: Vec<&str> = ornis
        .text_input
        .split_ascii_whitespace()
        .into_iter()
        .collect();
    let command = words
        .first()
        .map(|s| Command::from_str(s))
        .unwrap_or_default();

    info!("command: {command:?}");

    match command {
        Command::WorldQuery => handle_world_query(&words, &ornis.current_ns),
        _ => Message::Noop,
    }
}

fn handle_registry_req() -> Task<Message> {
    let client = reqwest::blocking::Client::new();

    let req = BrpRegistryRequest {
        method: "registry.schema".into(),
        params: RegistryParams {
            with_crates: vec!["wanderrust".into()],
            ..Default::default()
        },
        ..Default::default()
    };

    let resp_result = client.post(URL).json(&req).send();

    info!("{resp_result:#?}");

    match resp_result {
        Ok(result) => {
            let json = result.json().unwrap();
            info!("{json:#?}");

            return Task::done(Message::LoadRegistry(json));
        }
        Err(err) => {
            error!("handle_registry_req: {}", err);
            return Task::none();
        }
    }
}

fn handle_world_query(words: &Vec<&str>, ns: impl AsRef<str>) -> Message {
    trace!("handle_world_query");
    let client = reqwest::blocking::Client::new();

    let words = words
        .iter()
        .skip(1)
        .map(|&s| format!("{}::{}", ns.as_ref(), s))
        .collect::<Vec<_>>();

    info!("assembling query: {words:?}");

    let req = BrpQueryRequest {
        method: "world.query".into(),
        params: QueryParams {
            data: QueryData {
                components: words,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };

    if let Ok(json) = serde_json::to_string_pretty(&req) {
        trace!("{:?}", req);
        trace!("json: {}", json);
    }

    trace!("outgoing request: {:?}", req);

    let resp = client.post(URL).json(&req).send();

    match resp {
        Ok(http_resp) => {
            trace!("handle_world_query: resp {:?}", http_resp);
            handle_resp(http_resp).unwrap_or(Message::Noop)
        }
        Err(err) => {
            error!("handle_world_query: err {}", err);
            Message::Noop
        }
    }
}

fn handle_resp(response: blocking::Response) -> Result<Message, anyhow::Error> {
    let val: Value = response.json().unwrap();

    match serde_json::from_value(val) {
        Ok(results) => {
            trace!("response val: {:?}", results);
            return Ok(Message::QueryResultsReady(results));
        }
        Err(err) => {
            error!("{}", err);
            return Err(anyhow!(err));
        }
    }
}

fn update_pane(state: &mut Ornis, resp: &BrpQueryResponse) -> Task<Message> {
    let mut lines: Vec<String> = vec![];

    for entity in resp.result.iter() {
        lines.push(format!("entity: {}", entity.id));

        for c in &entity.components {
            lines.push(format!("\t{} = {}", c.0, c.1));
        }
        lines.push("\n".into());
    }

    state.last_response = lines.join("\n");

    Task::done(Message::UpdatePane)
}

fn update(state: &mut Ornis, message: Message) -> Task<Message> {
    match message {
        Message::EnterPressed => {
            state
                .scrollback
                .push(format!("> {}", state.text_input.clone()));
            state.text_content = Content::with_text(state.scrollback.join("\n").as_str());
            state.command_hist.push(state.text_input.clone());
            state.hist_idx = 0;
            let m = handle_command(state);
            state.text_input.clear();
            return Task::done(m);
        }
        Message::ContentChanged(new_input) => {
            state.text_input = new_input;
        }
        Message::WindowOpened => {
            info!("welcome to ornis");
            state.scrollback = vec!["=== welcome to ornis ===".into()];
            state.text_content = Content::with_text(state.scrollback.join("\n").as_str());
            return focus(MAIN_INPUT_ID).chain(handle_registry_req());
        }
        Message::QueryResultsReady(mut resp) => {
            resp.result.truncate(20);
            let out = serde_json::to_string_pretty::<BrpQueryResponse>(&resp);
            if let Ok(out) = out {
                state.scrollback.push(out);
                state.text_content = Content::with_text(state.scrollback.join("\n").as_str());

                let update_task = update_pane(state, &resp);

                return snap_to_end(MAIN_OUTPUT_ID).chain(update_task);
            }
        }
        Message::LoadRegistry(_) => {
            return Task::none();
        }
        Message::CommandPrev => {
            state.text_input = state
                .command_hist
                .iter()
                .nth_back(state.hist_idx as usize)
                .cloned()
                .unwrap_or_default();
            state.hist_idx = state.hist_idx.saturating_add(1);
        }
        Message::CommandNext => {
            state.text_input = state
                .command_hist
                .iter()
                .nth_back(state.hist_idx as usize)
                .cloned()
                .unwrap_or_default();
            state.hist_idx = state.hist_idx.saturating_sub(1);
        }
        Message::OutputChanged => return snap_to_end(MAIN_OUTPUT_ID),
        Message::UpdatePane => return Task::none(),
        _ => return Task::none(),
    }
    Task::none()
}

const MAIN_INPUT_ID: &'static str = "main_input";
const MAIN_OUTPUT_ID: &'static str = "main_output";
const STRUCTURED_VIEW_ID: &'static str = "structured_view";

fn view(state: &Ornis) -> Row<'_, Message> {
    row![
        column![
            text_editor::TextEditor::new(&state.text_content)
                .id(MAIN_OUTPUT_ID)
                .size(14)
                .height(Fill)
                .on_action(on_action),
            row![
                text_input::TextInput::new("commands go here", &state.text_input)
                    .id(MAIN_INPUT_ID)
                    .padding(10)
                    .size(14)
                    .on_input(Message::ContentChanged)
                    .on_submit(Message::EnterPressed),
                button("enter").on_press(Message::EnterPressed),
            ]
            .spacing(10),
        ]
        .width(FillPortion(5))
        .spacing(10),
        column![Container::new(text(&state.last_response)).id(STRUCTURED_VIEW_ID)]
            .width(FillPortion(1))
            .height(Fill)
            .spacing(10)
    ]
    .spacing(10)
    .into()
}

fn on_action(action: Action) -> Message {
    match action {
        text_editor::Action::Edit(text_editor::Edit::Insert(_)) => Message::OutputChanged,
        _ => Message::Noop,
    }
}

fn subscription(_state: &Ornis) -> iced::Subscription<Message> {
    event::listen_with(|evt, _, _| match evt {
        Event::Keyboard(key_event) => match key_event {
            iced::keyboard::Event::KeyPressed { key, .. } => match key {
                Key::Named(Named::ArrowUp) => Some(Message::CommandPrev),
                Key::Named(Named::ArrowDown) => Some(Message::CommandNext),
                _ => None,
            },
            _ => None,
        },
        Event::Window(win_event) => match win_event {
            iced::window::Event::Opened { .. } => Some(Message::WindowOpened),
            iced::window::Event::Closed => Some(Message::WindowClosed),
            _ => None,
        },
        _ => None,
    })
}

fn main() -> iced::Result {
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("ornis", LevelFilter::max())
        .init()
        .unwrap();
    println!("initialized logging");
    iced::application(Ornis::default, update, view)
        .theme(iced::Theme::Ferra)
        .subscription(subscription)
        .run()
}
