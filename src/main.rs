mod methods;
mod params;
mod rpc;

use anyhow::anyhow;
use iced::widget::text;
use iced::{
    Element, Event, Font,
    Length::{Fill, FillPortion},
    Task, event,
    keyboard::{Event::KeyPressed, Key, key::Named},
    widget::{
        Row, button, column,
        operation::{focus, snap_to_end},
        row, scrollable,
        text::Alignment,
        text_editor::{self, Action, Content},
        text_input,
    },
};
use log::{LevelFilter, error, info, trace, warn};
use methods::*;
use reqwest::{
    self,
    blocking::{self},
};
use serde_json::Value;
use simple_logger::{self};
use std::collections::BTreeMap;
use std::fmt::Write;

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
    last_response: Vec<Structure>,
    registry: BTreeMap<String, Value>,
}

#[derive(Default, Debug, Clone)]
enum Structure {
    #[default]
    Empty,
    Entity(String),
    Component(String, Value),
}

const URL: &str = "http://localhost:15702";

const CRATE_PATH: &str = "wanderrust";

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
            registry: Default::default(),
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
    QueryResults(BrpQueryResponse),
    ComponentsList(BrpListComponentsResponse),
    LoadRegistry(BrpRegistryResponse),
    ShowRegistry,
    QueryRegistry(String),
    OutputChanged,
    CommandPrev,
    CommandNext,
    UpdatePane,
    Delegate(Action),
    PrintHelp(String),
    Error(String),
}

macro_rules! define_commands {
    ( $enum_name:ident, [ $( $variant:ident => [ names: [ $(  $name:expr$(,)? )+ ], help_short: $help:expr ] $(,)? )* ] ) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
        enum $enum_name {
            $( $variant, )*
        }

        impl $enum_name {
            const ALL: &[$enum_name] = &[ $( $enum_name::$variant, )* ];

            pub(crate) fn from_str(s: &str) -> Option<Command> {
                use $enum_name::*;
                match s {
                    $(
                        $( $name => Some($variant), )+
                    )*
                    _ => None,
                }
            }

            pub(crate) fn name(self) -> &'static str {
                use $enum_name::*;
                match self {
                    $( $variant => stringify!($variant), )+
                }
            }

            pub(crate) fn help_short(&self) -> &'static str {
                use $enum_name::*;
                match self {
                    $( $variant => $help, )+
                }
            }
        }

    };
}

define_commands! (
    Command, [
        WorldQuery => [ names: ["q", "wq", "world.query", "query"], help_short: "query for entities which have one or more component" ],
        ListComponents => [ names: ["lc", "l", "world.list_components", "list_components"], help_short: "list components w/ data on a single entity"],
        ListRegistry => [ names: ["lr", "lreg", "listreg"], help_short: "show types reported by bevy remote protocol" ],
        SearchRegistry => [names: ["sr", "sreg", "searchreg"], help_short: "query registry of types via substring match"],
        PrintHelp => [names: ["?", "help"], help_short: "print this help"],
    ]
);

fn handle_command(ornis: &mut Ornis) -> Message {
    let words: Vec<&str> = ornis.text_input.split_ascii_whitespace().collect();
    let Some(command) = words.first().and_then(|it| Command::from_str(*it)) else {
        warn!("unrecognized command: {:?}", words);
        return Message::Noop;
    };

    info!("command: {command:?}");

    match command {
        Command::WorldQuery => handle_world_query(&words, &ornis.current_ns, &ornis.registry),
        Command::ListComponents => handle_list_components(&words, &ornis.current_ns),
        Command::ListRegistry => Message::ShowRegistry,
        Command::SearchRegistry => handle_query_registry(&words, &ornis.registry),
        Command::PrintHelp => handle_print_help(&words, &ornis.current_ns),
    }
}

fn handle_print_help(_words: &[&str], _current_ns: impl AsRef<str>) -> Message {
    let mut out = String::new();

    writeln!(out, "Ornis commands:").unwrap();

    for c in Command::ALL {
        writeln!(out, "{} => {}", c.name(), c.help_short()).unwrap();
    }

    writeln!(
        out,
        "NB: `#[reflect(Component)]` is required for types to appear in results even though they may appear in the registry."
    ).unwrap();

    info!("help is: {}", out);

    Message::PrintHelp(out)
}

const JSONRPC_VER: &'static str = "2.0";

fn handle_query_registry(words: &Vec<&str>, lookup: &BTreeMap<String, Value>) -> Message {
    info!("querying registry for strings: {words:?}");
    let mut out = String::new();

    for word in words.iter().skip(1) {
        out.push_str(&format!("### RESULTS FOR {word} ###\n"));
        for ty in lookup.keys() {
            if ty.to_lowercase().contains(&word.to_lowercase()) {
                out.push_str(&format!("\t- {ty}\n"))
            }
        }
    }

    if out.is_empty() {
        info!("no results for any of {words:?}");
        out.push_str(&format!("### No results for any word in {words:?} ###\n"));
    }

    Message::QueryRegistry(out)
}

fn handle_registry_req() -> Task<Message> {
    if true {
        return Task::none();
    }

    let client = reqwest::blocking::Client::new();

    let Ok(params) = serde_json::to_value(RegistryParams {
        with_crates: vec!["wanderrust".into()],
        ..Default::default()
    }) else {
        return Task::none();
    };

    let req = BrpRequest {
        jsonrpc: JSONRPC_VER.to_string(),
        method: BRP_REGISTRY_SCHEMA_METHOD.to_string(),
        params,
        ..Default::default()
    };

    let resp_result = client.post(URL).json(&req).send();

    info!("handle_registry_req: RESPONSE: {resp_result:#?}");

    match resp_result {
        Ok(resp) => match resp.json() {
            Ok(json) => {
                info!("handle_registry_req: RESPONSE JSON: {json:#?}");
                Task::done(Message::LoadRegistry(json))
            }
            Err(err) => {
                error!("handle_registry_req: json: {err}");
                Task::none()
            }
        },
        Err(err) => {
            error!("handle_registry_req: {}", err);
            Task::none()
        }
    }
}

fn handle_list_components(words: &Vec<&str>, _ns: impl AsRef<str>) -> Message {
    trace!("handle_list_components");

    let Some(entity_str) = words.iter().nth(1) else {
        return Message::Noop;
    };

    let entity = i64::from_str_radix(entity_str, 10).unwrap_or_default();
    trace!("querying for entity {entity_str} as {entity:?}");

    let Ok(params) = serde_json::to_value(ListComponentsParams { entity }) else {
        return Message::Noop;
    };

    let req = BrpRequest {
        jsonrpc: JSONRPC_VER.to_string(),
        method: BRP_LIST_COMPONENTS_METHOD.to_string(),
        params,
        ..Default::default()
    };
    trace!("outgoing request: {req:?}");

    let client = reqwest::blocking::Client::new();
    let resp = client.post(URL).json(&req).send();

    match resp {
        Ok(http_resp) => {
            trace!("handle_get_components: resp {http_resp:?}");
            handle_list_components_resp(http_resp).unwrap_or(Message::Noop)
        }
        Err(err) => {
            error!("handle_get_components: err {err}");
            return Message::Noop;
        }
    }
}

fn get_typepath(registry: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    registry
        .get(key)?
        .as_object()?
        .get("typePath")?
        .as_str()
        .map(|it| String::from(it))
}

fn handle_world_query(
    words: &Vec<&str>,
    ns: impl AsRef<str>,
    registry: &BTreeMap<String, Value>,
) -> Message {
    info!("handle_world_query {words:?}");

    let mut components = vec![];

    for word in words.iter().skip(1) {
        if word.starts_with("::") {
            components.push(format!("{}{}", ns.as_ref(), word));
        } else if word.starts_with("!") {
            components.push(word.to_string());
        } else if registry.contains_key(*word) {
            let ty = match get_typepath(registry, word) {
                Some(tp) => tp,
                None => word.to_string(),
            };
            info!("found typepath: {}", ty);
            components.push(ty);
        } else {
            // TODO: lol error handling lol
            return Message::Noop;
        }
    }

    info!("assembling query: {components:?}");

    let qp = QueryParams {
        data: QueryData {
            components,
            ..Default::default()
        },
        ..Default::default()
    };

    let params = match serde_json::to_value(qp) {
        Ok(query_params) => query_params,
        Err(e) => {
            error!("unable to convert query params: {e}");
            return Message::Noop;
        }
    };

    let req = BrpRequest {
        jsonrpc: JSONRPC_VER.to_string(),
        method: BRP_QUERY_METHOD.to_string(),
        params,
        ..Default::default()
    };

    if let Ok(json) = serde_json::to_string_pretty(&req) {
        trace!("{:?}", req);
        trace!("json: {}", json);
    }

    trace!("outgoing request: {:?}", req);

    let client = reqwest::blocking::Client::new();
    let resp = client.post(URL).json(&req).send();

    match resp {
        Ok(http_resp) => {
            trace!("handle_world_query: ok: {:?}", http_resp);
            handle_query_resp(http_resp).unwrap_or(Message::Noop)
        }
        Err(err) => {
            error!("handle_world_query: error: {}", err);
            Message::Noop
        }
    }
}

fn handle_query_resp(response: blocking::Response) -> Result<Message, anyhow::Error> {
    let val: Value = response.json().unwrap();
    match serde_json::from_value(val) {
        Ok(results) => {
            trace!("response val: {:?}", results);
            Ok(Message::QueryResults(results))
        }
        Err(err) => {
            error!("{}", err);
            Err(anyhow!(err))
        }
    }
}

fn handle_list_components_resp(response: blocking::Response) -> Result<Message, anyhow::Error> {
    let val: Value = response.json().unwrap();

    trace!("retrieved value: {val:?}");

    match serde_json::from_value(val) {
        Ok(results) => {
            trace!("response val: {:?}", results);
            Ok(Message::ComponentsList(results))
        }
        Err(err) => {
            error!("{}", err);
            Err(anyhow!(err))
        }
    }
}

fn update_pane(state: &mut Ornis, resp: &BrpQueryResponse) -> Task<Message> {
    let mut structures: Vec<Structure> = vec![];

    for entity in resp.result.iter() {
        structures.push(Structure::Entity(format!("{}", entity.id)));
        for c in &entity.components {
            structures.push(Structure::Component(c.0.clone(), c.1.clone()))
        }
    }

    state.last_response = structures;

    Task::done(Message::UpdatePane)
}

fn update(state: &mut Ornis, message: Message) -> Task<Message> {
    match message {
        Message::Error(e) => {
            state.scrollback.push(format!("!! {e}"));
            state.text_content = Content::with_text(state.scrollback.join("\n").as_str());
            return snap_to_end(MAIN_OUTPUT_ID);
        }
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
        Message::QueryResults(mut resp) => {
            resp.result.truncate(20);
            let out = serde_json::to_string_pretty::<BrpQueryResponse>(&resp);
            if let Ok(out) = out {
                state.scrollback.push(out);
                state.text_content = Content::with_text(state.scrollback.join("\n").as_str());

                let update_task = update_pane(state, &resp);

                return snap_to_end(MAIN_OUTPUT_ID).chain(update_task);
            }
        }
        Message::LoadRegistry(brp_registry_resp) => {
            load_registry(state, brp_registry_resp);
        }
        Message::CommandPrev => {
            state.text_input = state
                .command_hist
                .iter()
                .nth_back(state.hist_idx)
                .cloned()
                .unwrap_or_default();
            state.hist_idx = state.hist_idx.saturating_add(1);
        }
        Message::CommandNext => {
            state.text_input = state
                .command_hist
                .iter()
                .nth_back(state.hist_idx)
                .cloned()
                .unwrap_or_default();
            state.hist_idx = state.hist_idx.saturating_sub(1);
        }
        Message::Delegate(action) => state.text_content.perform(action),
        Message::OutputChanged => return snap_to_end(MAIN_OUTPUT_ID),
        Message::UpdatePane => return Task::none(),
        Message::ComponentsList(BrpListComponentsResponse { result }) => {
            let mut out = String::new();
            for component in result {
                out.push_str(&format!("{component}\n"));
            }
            state.scrollback.push(out);
            state.text_content = Content::with_text(state.scrollback.join("\n").as_str());
            return snap_to_end(MAIN_OUTPUT_ID);
        }
        Message::ShowRegistry => {
            let mut out = String::new();
            for ty in state.registry.keys() {
                out.push_str(&format!("{ty}\n"));
            }
            state.scrollback.push(out);
            state.text_content = Content::with_text(state.scrollback.join("\n").as_str());
            return snap_to_end(MAIN_OUTPUT_ID);
        }
        Message::QueryRegistry(output) => {
            state.scrollback.push(output);
            state.text_content = Content::with_text(state.scrollback.join("\n").as_str());
            return snap_to_end(MAIN_OUTPUT_ID);
        }
        Message::PrintHelp(msg) => {
            state.scrollback.push(msg);
            state.text_content = Content::with_text(state.scrollback.join("\n").as_str());
            return snap_to_end(MAIN_OUTPUT_ID);
        }
        _ => return Task::none(),
    }
    Task::none()
}

fn load_registry(state: &mut Ornis, registry_resp: BrpRegistryResponse) {
    let mut ntypes = 0;
    for (ty, type_info) in registry_resp.result.iter() {
        let o = match type_info.as_object() {
            Some(o) => o,
            None => {
                warn!("unable to deserialize info for {ty}");
                continue;
            }
        };

        let short_name = match o.get("shortPath").and_then(|it| it.as_str()) {
            Some(s) => s,
            None => {
                warn!("unable to deserialize shortname for {ty}");
                continue;
            }
        };

        let type_path = match o.get("typePath").and_then(|it| it.as_str()) {
            Some(p) => p,
            None => {
                warn!("unable to deserialize typepath for {ty}");
                continue;
            }
        };

        state
            .registry
            .insert(short_name.to_lowercase().into(), type_info.clone());
        state
            .registry
            .insert(type_path.to_lowercase().into(), type_info.clone());
        ntypes += 1;
    }

    info!("registered info for {ntypes} types");
}

const MAIN_INPUT_ID: &str = "main_input";
const MAIN_OUTPUT_ID: &str = "main_output";
const STRUCTURED_VIEW_ID: &str = "structured_view";

fn view(state: &Ornis) -> Row<'_, Message> {
    row![
        column![
            scrollable(
                text_editor::TextEditor::new(&state.text_content)
                    .id(MAIN_OUTPUT_ID)
                    .size(12)
                    .font(Font::MONOSPACE)
                    .on_action(on_action)
            )
            .height(FillPortion(8)),
            row![
                text_input::TextInput::new("commands go here", &state.text_input)
                    .id(MAIN_INPUT_ID)
                    .padding(10)
                    .size(12)
                    .font(Font::MONOSPACE)
                    .on_input(Message::ContentChanged)
                    .on_submit(Message::EnterPressed),
                button("enter").on_press(Message::EnterPressed),
            ]
        ]
        .width(FillPortion(5))
        .spacing(10),
        scrollable(structured_view(state))
            .id(STRUCTURED_VIEW_ID)
            .width(FillPortion(4))
            .height(Fill)
            .spacing(10)
    ]
    .spacing(10)
}

fn mono_text<'a>(t: String, align: Alignment) -> Element<'a, Message> {
    text(t).align_x(align).font(Font::MONOSPACE).size(11).into()
}

fn structured_view(state: &Ornis) -> impl Into<Element<'_, Message>> {
    let mut col = column![text("last response").font(Font::MONOSPACE)];

    for item in state.last_response.iter() {
        let t = match item {
            Structure::Entity(id) => {
                row![mono_text(format!("‣ {}", id), Alignment::Left)].spacing(10)
            }
            Structure::Component(name, val) => row![
                mono_text("   •".into(), Alignment::Right),
                mono_text(format!("{}: {}", name, val), Alignment::Left)
            ]
            .spacing(10),
            _ => continue,
        };
        col = col.push(t);
    }

    col
}

fn on_action(action: Action) -> Message {
    match action {
        text_editor::Action::Edit(text_editor::Edit::Insert(_)) => Message::OutputChanged,
        _ => Message::Delegate(action),
    }
}

fn subscription(_state: &Ornis) -> iced::Subscription<Message> {
    event::listen_with(|evt, _, _| match evt {
        Event::Keyboard(KeyPressed { key, .. }) => match key {
            Key::Named(Named::ArrowUp) => Some(Message::CommandPrev),
            Key::Named(Named::ArrowDown) => Some(Message::CommandNext),
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
        .with_module_level("ornis", LevelFilter::Info)
        .init()
        .unwrap();
    println!("initialized logging");
    iced::application(Ornis::default, update, view)
        .theme(iced::Theme::Ferra)
        .subscription(subscription)
        .run()
}
