use std::{collections::HashMap, default};

use anyhow::anyhow;
use iced::{
    Task,
    widget::{
        Column, button, column,
        operation::focus,
        text_editor::{self, Content},
        text_input,
    },
};
use log::{self, Level, LevelFilter, error, info};
use reqwest::{self, Response, blocking};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use simple_logger::{self};

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
    ResultsReady(BrpResponse),
}

#[derive(Default, Debug, Copy, Clone)]
enum Command {
    #[default]
    Noop,
    WorldQuery,
}

fn handle_command(_ornis: &mut Ornis) -> Message {
    handle_world_query()
}

#[derive(Debug, Serialize, Deserialize)]
struct BrpRequest {
    jsonrpc: String,
    method: String,
    id: serde_json::Value,
    params: Params,
}

impl Default for BrpRequest {
    fn default() -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: String::default(),
            id: Value::default(),
            params: Params::default(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Params {
    data: Data,
    filter: Filter,
    strict: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Data {
    components: Vec<String>,
    option: Vec<String>,
    has: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Filter {
    with: Vec<String>,
    without: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrpResponse {
    result: Vec<BrpEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GridCoords {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrpEntity {
    components: HashMap<String, GridCoords>,
    entity: i64,
}

fn handle_world_query() -> Message {
    info!("handle_world_query");
    let client = reqwest::blocking::Client::new();

    let req = BrpRequest {
        method: "world.query".into(),
        params: Params {
            data: Data {
                components: vec!["bevy_ecs_ldtk::components::GridCoords".into()],
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let resp = client.post("http://localhost:15702").json(&req);

    let result = match resp.send() {
        Ok(http_resp) => {
            info!("response {:?}", http_resp);
            handle_resp(http_resp)
        }
        Err(err) => {
            error!("handle_world_query: {}", err);
            anyhow::Result::Err(anyhow!("bad response: {}", err))
        }
    };

    result.unwrap_or(Message::Noop)
}

fn handle_resp(response: blocking::Response) -> Result<Message, anyhow::Error> {
    let val: Value = response.json().unwrap();

    match serde_json::from_value(val) {
        Ok(res) => {
            info!("response val: {:?}", res);
            return Ok(Message::ResultsReady(res));
        }
        Err(err) => {
            error!("{}", err);
            return Err(anyhow!(err));
        }
    }
}

fn update(state: &mut Ornis, message: Message) -> Task<Message> {
    match message {
        Message::EnterPressed => {
            state.scrollback.push(format!("> {}", state.input.clone()));
            state.content = Content::with_text(state.scrollback.join("\n").as_str());
            info!("doing a canned command");
            let m = handle_command(state);
            state.input.clear();
            return Task::done(m);
        }
        Message::ContentChanged(new_input) => {
            state.input = new_input;
        }
        Message::WindowOpened => {
            state.scrollback = vec!["=== welcome to ornith ===".into()];
            state.content = Content::with_text(state.scrollback.join("\n").as_str());
            return focus(MAIN_INPUT_ID);
        }
        Message::ResultsReady(mut resp) => {
            resp.result.truncate(20);
            let out = serde_json::to_string_pretty::<BrpResponse>(&resp);
            if let Ok(out) = out {
                state.scrollback.push(out);
                state.content = Content::with_text(state.scrollback.join("\n").as_str());
            }
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
