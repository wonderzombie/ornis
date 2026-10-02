mod config;
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
use log::{LevelFilter, debug, error, info, trace, warn};
use serde_json::{Map, Value};
use simple_logger::{self};
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;
use std::path::PathBuf;

use crate::config::{Config, write_default_config};
use crate::params::QParam;
use crate::rpc::BrpRequestExt;

#[derive(Debug, Clone)]
pub struct Ornis {
    // UI state
    scrollback: Vec<String>,
    text_input: String,
    text_content: Content,
    command_hist: Vec<String>,
    hist_idx: usize,
    last_response: Vec<Structure>,
    registry: BTreeMap<String, Value>,

    // Configuration state
    config: Config,
}

#[derive(Default, Debug, Clone)]
enum Structure {
    #[default]
    Empty,
    Entity(String),
    Component(String, Option<Value>),
}

const MAX_RESULTS: usize = 25;

impl Default for Ornis {
    fn default() -> Self {
        Self {
            scrollback: Default::default(),
            text_input: Default::default(),
            text_content: Default::default(),
            command_hist: Default::default(),
            hist_idx: Default::default(),
            last_response: Default::default(),
            registry: Default::default(),
            config: Config::default(),
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
    BrpResponse(rpc::Message),
    ListRegistry,
    QueryRegistry(String),
    OutputChanged,
    CommandPrev,
    CommandNext,
    UpdatePane,
    Delegate(Action),
    PrintHelp(String),
    Error(String),
    ShowConfig,
    ReloadConfig,
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
        WorldQuery => [ names: ["q", "wq", "world.query", "query"], help_short: "query for entities which have one or more component:\n\t> q name transform +sprite" ],
        ListComponents => [ names: ["lc", "l", "world.list_components", "list_components"], help_short: "list components w/ data on a single entity"],
        ListRegistry => [ names: ["lt", "lrg", "lreg", "listreg"], help_short: "show types reported by bevy remote protocol" ],
        SearchRegistry => [names: ["sr", "sreg", "searchreg"], help_short: "query registry of types via substring match"],
        LoadRpcSchema => [names: ["rl", "rlreg", "reloadreg"], help_short: "reload the registry from bevy"],
        PrintHelp => [names: ["?", "help"], help_short: "print this help"],
        ListResources => [ names: ["lsrs", "lrs", "lsres", "listres"], help_short: "list resources"],
        GetResources => [ names: ["grs", "gres", "getres"], help_short: "display a resource"],
        GetComponents => [ names: ["g", "gc", "gcs", "gcom"], help_short: "read components for an entity:\n\t> g [entity_id] mycomponent myothercomponent"],
        ShowConfig => [ names: ["cfg", "showcfg"], help_short: "show currently used ornis configuration"],
        ReloadConfig => [ names: ["rlcfg"], help_short: "reload config from disk"],
        SetNamespace => [ names: ["setns"], help_short: "set the in-memory configuration's namespace"]
    ]
);

fn handle_command(ornis: &mut Ornis) -> Result<Message> {
    let w = ornis.text_input.to_ascii_lowercase();
    let words: Vec<&str> = w.split_ascii_whitespace().collect();

    let Some(command) = words.first().and_then(|it| Command::from_str(it)) else {
        warn!("unrecognized command: {:?}", words);
        return Ok(Message::Noop);
    };

    info!("command: {command:?}");

    match command {
        Command::WorldQuery => {
            send_world_query_request(&words, &ornis.registry, &ornis.config.current_ns)
        }
        Command::ListComponents => send_list_components_request(&words, &ornis.config.current_ns),
        Command::LoadRpcSchema => send_rpc_schema_request(&ornis.config),
        Command::ListRegistry => Ok(Message::ListRegistry),
        Command::SearchRegistry => Ok(query_type_registry(&words, &ornis.registry)),
        Command::PrintHelp => Ok(print_help(&words, &ornis.config.current_ns)),
        Command::ListResources => send_list_resources_request(),
        Command::GetResources => send_get_resources_request(&words, &ornis.registry),
        Command::GetComponents => send_get_components_request(&words, &ornis.registry),
        Command::ShowConfig => Ok(Message::ShowConfig),
        Command::ReloadConfig => Ok(Message::ReloadConfig),
        Command::SetNamespace => set_namespace(&words, &mut ornis.config),
    }
}

fn set_namespace(words: &[&str], config: &mut Config) -> Result<Message> {
    let Some(ns) = words.get(1) else {
        return Ok(Message::Error(
            "SetNamespace takes one argument; found none".into(),
        ));
    };

    config.current_ns = ns.to_string();

    Ok(Message::Noop)
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
        writeln!(out, "- {} ({})\n\t{}", aliases, c.name(), c.help_short()).unwrap();
    }

    writeln!(out).unwrap();

    writeln!(
        out,
        "An empty result quite often means that the type in question hasn't been configured for reflection, meaning the remote protocol can't send a representation of it."
    ).unwrap();

    writeln!(
        out,
        "For `Component` or `Resource`, this can mean deriving Reflect and adding `#[reflect(Component)]` or `Resource`."
    )
    .unwrap();

    writeln!(
        out,
        "However, it won't work if either datatype includes a type which is NOT suitable for reflection or serde."
    )
    .unwrap();

    debug!("help is: {}", out);

    Message::PrintHelp(out)
}

fn query_type_registry(words: &[&str], lookup: &BTreeMap<String, Value>) -> Message {
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

fn send<P: BrpRequestExt>(req: P) -> Result<Message> {
    rpc::send(req, Message::BrpResponse)
}

fn send_rpc_schema_request(config: &Config) -> Result<Message> {
    let params = rpc::RegistryParams {
        with_crates: config.with_crates.clone(),
        without_crates: config.without_crates.clone(),
        ..Default::default()
    };

    send(params)
}

fn send_list_components_request(words: &[&str], _ns: &str) -> Result<Message> {
    trace!("handle_list_components");

    let entity_str = words
        .get(1)
        .ok_or(anyhow!("entity name missing from {words:?}"))?;

    let entity = i64::from_str_radix(entity_str, 10).unwrap_or_default();
    info!("querying for entity {entity_str} as {entity:?}");

    send(rpc::ListComponentsParams { entity })
}

fn send_list_resources_request() -> Result<Message> {
    send(rpc::BrpListResourcesParams)
}

fn send_get_resources_request(
    words: &[&str],
    registry: &BTreeMap<String, Value>,
) -> Result<Message> {
    let ty_opt = words.get(1).and_then(|it| get_typepath(registry, it));

    let resource = match ty_opt {
        Some(r) => r,
        None => bail!("missing argument for resource: {words:?}"),
    };

    send(rpc::BrpGetResourcesParams { resource })
}

fn send_get_components_request(
    words: &[&str],
    registry: &BTreeMap<String, Value>,
    // ns: &str,
) -> Result<Message> {
    // For this entity ...
    let entity: i64 = match words.get(1) {
        Some(w) => w.parse::<i64>()?,
        None => bail!("entity required for this command"),
    };

    // ...request these components.
    let components: Vec<String> = words
        .iter()
        .skip(1)
        .flat_map(|it| get_typepath(registry, it))
        .collect();

    send(rpc::BrpGetComponentsParams {
        entity,
        components,
        strict: false,
    })
}

fn get_typepath(registry: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    registry
        .get(&key.to_lowercase())?
        .as_object()?
        .get("typePath")?
        .as_str()
        .map(String::from)
}

fn send_world_query_request(
    words: &[&str],
    registry: &BTreeMap<String, Value>,
    current_ns: &String,
) -> Result<Message> {
    info!("handle_world_query {words:?}");

    let resolved = resolve_world_query(words, registry, current_ns)?;

    let query_params = params::collect(resolved);
    info!("assembled query parameters: {query_params:?}");

    send(query_params)
}

fn resolve_world_query(
    words: &[&str],
    registry: &BTreeMap<String, Value>,
    current_ns: &String,
) -> Result<Vec<(QParam, String)>> {
    let mut resolved = Vec::new();
    let mut unknown = Vec::new();
    for w in words.iter().skip(1) {
        let (ty, name) = QParam::parse(w)?;

        let name: String = match name.strip_prefix("::") {
            Some(rest) => format!("{current_ns}::{rest}"),
            None => name.to_string(),
        };

        match name.as_str().strip_prefix('!') {
            Some(literal) => resolved.push((ty, literal.to_string())),
            None => match get_typepath(registry, name.as_str()) {
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

fn update_structured_view(state: &mut Ornis, entities: &Vec<rpc::BrpEntity>) -> Task<Message> {
    let mut structures: Vec<Structure> = vec![];

    for entity in entities {
        structures.push(Structure::Entity(format!("{}", entity.id)));
        for (name, val_opt) in &entity.components {
            structures.push(Structure::Component(name.clone(), Some(val_opt.clone())));
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

fn handle_rpc_message(state: &mut Ornis, m: rpc::Message) -> Task<Message> {
    state.scrollback.clear();
    match m {
        rpc::Message::Query(mut resp) => {
            resp.result.truncate(MAX_RESULTS);
            if let Ok(out) = serde_json::to_string_pretty::<rpc::BrpQueryResponse>(&resp) {
                info!("{}", out);
            }
            let entities = resp
                .result
                .iter()
                .map(|it| format!("{:?} {:#?}", it.id, &it.components))
                .collect::<Vec<_>>()
                .join("\n");
            state.update_scrollback(format!("{} results\n{}", resp.result.len(), entities));

            return update_structured_view(state, &resp.result);
        }
        rpc::Message::ListComponents(rpc::BrpListComponentsResponse { result }) => {
            let out = result
                .iter()
                .map(|c| format!("{c}"))
                .collect::<Vec<_>>()
                .join("\n");
            state.update_scrollback(out);
        }
        rpc::Message::RegistrySchema(rpc::BrpRegistrySchemaResponse { result }) => {
            load_registry(state, result);
        }
        rpc::Message::ListResources(rpc::BrpListResourcesResponse { result }) => {
            let out = result.join("\n");
            state.update_scrollback(out);
        }
        rpc::Message::GetResources(rpc::BrpGetResourcesResponse { ref result }) => {
            let out = match serde_json::to_string_pretty(result) {
                Ok(s) => s,
                Err(_) => result.to_string(),
            };
            state.update_scrollback(out);
        }
        rpc::Message::GetComponents(rpc::BrpGetComponentsResponse { ref result }) => {
            let component_map = result.get("components").and_then(|it| it.as_object());
            state.update_scrollback(format!("{component_map:#?}"));
        }
    }
    Task::none()
}

pub fn load_config(path_str: impl AsRef<str>) -> Config {
    let pb = PathBuf::from(path_str.as_ref());
    config::try_load_config(&pb)
        .or_else(|e| {
            warn!("unable to load config at {pb:?}; writing default config and using that: {e}");
            write_default_config(&pb)
        })
        .inspect_err(|e| {
            warn!("unable to write config at {pb:?}; using default, in-memory config: {e}");
        })
        .unwrap_or_default()
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
                Err(e) => Task::done(Message::Error(e.to_string())),
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

            state.config = load_config(config::CONFIG_PATH);

            let schema_task = match send_rpc_schema_request(&state.config) {
                Ok(m) => Task::done(m),
                Err(e) => Task::done(Message::Error(e.to_string())),
            };

            return focus(MAIN_INPUT_ID).chain(schema_task);
        }
        Message::BrpResponse(rpc_message) => {
            return handle_rpc_message(state, rpc_message);
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
        Message::UpdatePane => {
            return snap_to_end(MAIN_OUTPUT_ID);
        }
        Message::ListRegistry => {
            let mut out: Vec<String> = vec![];
            let type_paths = state
                .registry
                .values()
                .flat_map(|v| v.as_object()?.get("typePath")?.as_str())
                .collect::<HashSet<_>>();

            type_paths.iter().for_each(|type_path| {
                out.push(format!("{type_path}"));
            });

            out.sort();

            state.update_scrollback(out.join("\n"));
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
        Message::ShowConfig => match toml::to_string_pretty(&state.config) {
            Ok(c) => state.update_scrollback(c),
            Err(e) => {
                error!("unable to print config: {e}");
                state.update_scrollback(format!("unable to print config: {e}"));
            }
        },
        Message::ReloadConfig => {
            state.config = load_config(config::CONFIG_PATH);
            state.update_scrollback(format!("reloaded config at {}", config::CONFIG_PATH));
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
        .with_module_level("rpc", LevelFilter::Debug)
        .init()
        .unwrap();
    println!("initialized logging");
    iced::application(Ornis::default, update, view)
        .theme(iced::Theme::Ferra)
        .subscription(subscription)
        .run()
}
