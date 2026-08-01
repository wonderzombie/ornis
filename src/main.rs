mod methods;
mod params;
mod rpc;

use anyhow::{Result, anyhow, bail};
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
use log::{LevelFilter, info, trace, warn};
use serde_json::{Map, Value};
use simple_logger::{self};
use std::collections::BTreeMap;
use std::fmt::Write;

use crate::params::QParam;
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
    Component(String, Option<Value>),
}

const CRATE_PATH: &str = "wanderrust";

const MAX_RESULTS: usize = 25;

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

impl Ornis {
    fn update_scrollback(&mut self, txt: String) {
        self.scrollback.push(txt);
        self.text_content = Content::with_text(self.scrollback.join("\n").as_str());
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
    ListRegistry,
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

            pub(crate) fn names(&self) -> &[&'static str] {
                use $enum_name::*;
                match self {
                    $(
                        $variant => &[ $( $name, )* ],
                    )*
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
        ReloadRegistry => [names: ["rl", "rlreg", "reloadreg"], help_short: "reload the registry from bevy"],
        PrintHelp => [names: ["?", "help"], help_short: "print this help"],
    ]
);

fn handle_command(ornis: &mut Ornis) -> Result<Message> {
    let words: Vec<&str> = ornis.text_input.split_ascii_whitespace().collect();
    let Some(command) = words.first().and_then(|it| Command::from_str(it)) else {
        warn!("unrecognized command: {:?}", words);
        return Ok(Message::Noop);
    };

    info!("command: {command:?}");

    match command {
        Command::WorldQuery => handle_world_query(&words, &ornis.registry),
        Command::ListComponents => handle_list_components(&words, &ornis.current_ns),
        Command::ListRegistry => Ok(Message::ListRegistry),
        Command::SearchRegistry => Ok(query_type_registry(&words, &ornis.registry)),
        Command::PrintHelp => Ok(print_help(&words, &ornis.current_ns)),
        Command::ReloadRegistry => handle_registry_req(),
    }
}

fn print_help(_words: &[&str], _current_ns: impl AsRef<str>) -> Message {
    let mut out = String::new();

    writeln!(out, "Ornis commands:").unwrap();

    for c in Command::ALL {
        let aliases = c
            .names()
            .iter()
            .map(|it| it.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(out, "- {} ({})\n\t{}", aliases, c.name(), c.help_short(),).unwrap();
    }

    writeln!(out).unwrap();

    writeln!(
        out,
        "NB: If you see empty results, check the component you're querying in the source. #[reflect(Component)]` is required for types used in queries to show their data, and it's not the default."
    ).unwrap();

    info!("help is: {}", out);

    Message::PrintHelp(out)
}

fn query_type_registry(words: &Vec<&str>, lookup: &BTreeMap<String, Value>) -> Message {
    info!("querying registry for strings: {words:?}");
    let mut out = String::new();

    for word in words.iter().skip(1) {
        out.push_str(&format!("### matches for `{word}` ###\n"));
        for ty in lookup.keys() {
            if ty.to_lowercase().contains(&word.to_lowercase()) {
                out.push_str(&format!("\t- {ty}\n"))
            }
        }
    }

    if out.is_empty() {
        info!("no results for any of {words:?}");
        out.push_str(&format!("### no matches for any word in {words:?} ###\n"));
    }

    Message::QueryRegistry(out)
}

fn handle_registry_req() -> Result<Message> {
    let params = RegistryParams {
        with_crates: vec!["wanderrust".into()],
        ..Default::default()
    };

    send(params)
}

fn handle_list_components(words: &Vec<&str>, _ns: impl AsRef<str>) -> Result<Message> {
    trace!("handle_list_components");

    let entity_str = words
        .get(1)
        .ok_or(anyhow!("entity name missing from {words:?}"))?;

    let entity = i64::from_str_radix(entity_str, 10).unwrap_or_default();
    trace!("querying for entity {entity_str} as {entity:?}");

    rpc::send(ListComponentsParams { entity })
}

fn get_typepath(registry: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    registry
        .get(key)?
        .as_object()?
        .get("typePath")?
        .as_str()
        .map(String::from)
}

fn handle_world_query(words: &Vec<&str>, registry: &BTreeMap<String, Value>) -> Result<Message> {
    info!("handle_world_query {words:?}");

    let resolved = resolve_query(words, registry)?;

    let query_params = params::collect(resolved);
    info!("assembled query parameters: {query_params:?}");

    rpc::send(query_params)
}

fn resolve_query(
    words: &Vec<&str>,
    registry: &BTreeMap<String, Value>,
) -> Result<Vec<(QParam, String)>> {
    let mut resolved = Vec::new();
    let mut unknown = Vec::new();
    for w in words.iter().skip(1) {
        let (ty, name) = QParam::parse(w)?;

        match name.strip_prefix('!') {
            Some(literal) => resolved.push((ty, literal.to_string())),
            None => match get_typepath(registry, name) {
                Some(path) => resolved.push((ty, path)),
                None => unknown.push(name.to_string()),
            },
        }
    }
    if !unknown.is_empty() {
        bail!(format!("unknown component(s): {}", unknown.join(" ")));
    }
    Ok(resolved)
}

fn update_structured_view(state: &mut Ornis, resp: &BrpQueryResponse) -> Task<Message> {
    let mut structures: Vec<Structure> = vec![];

    for entity in resp.result.iter() {
        structures.push(Structure::Entity(format!("{}", entity.id)));
        for (name, val_opt) in &entity.components {
            structures.push(Structure::Component(name.clone(), val_opt.clone()));
        }

        if let Some(has) = &entity.has {
            for (name, boolean) in has {
                let b = Value::Bool(*boolean);
                structures.push(Structure::Component(name.clone(), Some(b)));
            }
        }
    }

    state.last_response = structures;

    Task::done(Message::UpdatePane)
}

fn update(state: &mut Ornis, message: Message) -> Task<Message> {
    match message {
        Message::Error(e) => {
            state.update_scrollback(format!("!! {e}"));
            return snap_to_end(MAIN_OUTPUT_ID);
        }
        Message::EnterPressed => {
            state.update_scrollback(format!("> {}", state.text_input.clone()));
            state.command_hist.push(state.text_input.clone());
            state.hist_idx = 0;
            let out = match handle_command(state) {
                Ok(m) => Task::done(m),
                Err(e) => Task::done(Message::Error(format!("error handling command: {e}"))),
            };
            state.text_input.clear();
            return out;
        }
        Message::ContentChanged(new_input) => {
            state.text_input = new_input;
        }
        Message::WindowOpened => {
            info!("welcome to ornis");
            state.update_scrollback("=== welcome to ornis ===".to_string());
            let registry_message = handle_registry_req()
                .unwrap_or(Message::Error("unable to read registry from bevy".into()));
            // TODO: map the Result's error type to `Message::Error`.
            return focus(MAIN_INPUT_ID).chain(Task::done(registry_message));
        }
        Message::QueryResults(mut resp) => {
            resp.result.truncate(MAX_RESULTS);
            let out = serde_json::to_string_pretty::<BrpQueryResponse>(&resp);
            if let Ok(out) = out {
                info!("{}", out);
            }
            state.update_scrollback(format!("{} results", resp.result.len()));
            let update_task = update_structured_view(state, &resp);
            return snap_to_end(MAIN_OUTPUT_ID).chain(update_task);
        }
        Message::LoadRegistry(BrpRegistryResponse { result }) => {
            load_registry(state, result);
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
            state.update_scrollback(out);
            return snap_to_end(MAIN_OUTPUT_ID);
        }
        Message::ListRegistry => {
            let mut out = String::new();
            for ty in state.registry.keys() {
                out.push_str(&format!("{ty}\n"));
            }
            state.update_scrollback(out);
            return snap_to_end(MAIN_OUTPUT_ID);
        }
        Message::QueryRegistry(output) => {
            state.update_scrollback(output);
            return snap_to_end(MAIN_OUTPUT_ID);
        }
        Message::PrintHelp(msg) => {
            state.update_scrollback(msg);
            return snap_to_end(MAIN_OUTPUT_ID);
        }
        _ => return Task::none(),
    }
    Task::none()
}

fn load_registry(state: &mut Ornis, map: Map<String, Value>) {
    let mut ntypes = 0;
    for (ty, type_info) in map.iter() {
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
            .insert(short_name.to_lowercase(), type_info.clone());
        state
            .registry
            .insert(type_path.to_lowercase(), type_info.clone());
        ntypes += 1;
    }

    info!("registered info for {ntypes} types");
    state.update_scrollback(format!("registered info for {ntypes} types"));
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

fn structured_view(state: &Ornis) -> Element<'_, Message> {
    let mut col = column![text("last response").font(Font::MONOSPACE)];

    for item in state.last_response.iter() {
        let t = match item {
            Structure::Entity(id) => {
                row![mono_text(format!("‣ {}", id), Alignment::Left)].spacing(10)
            }
            Structure::Component(name, Some(val)) => {
                let p = serde_json::to_string_pretty(val).unwrap_or(val.to_string());
                row![
                    mono_text("   •".into(), Alignment::Right),
                    mono_text(format!("{name}: {p}"), Alignment::Left)
                ]
                .spacing(10)
            }
            Structure::Component(name, None) => {
                row![mono_text(name.to_string(), Alignment::Left)].spacing(10)
            }
            _ => {
                info!("skipping {:?}", item);
                continue;
            }
        };
        col = col.push(t);
    }

    col.into()
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
