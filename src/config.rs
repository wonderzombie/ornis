use std::path::PathBuf;

use anyhow::Error;
use serde::{Deserialize, Serialize};

pub const CONFIG_PATH: &str = "ornis.toml";

const DEFAULT_CRATE_PATH: &str = "wanderrust";
const DEFAULT_WITH_CRATES: &[&'static str] = &[
    "bevy_app",
    "bevy_ecs",
    "bevy_camera",
    "bevy_picking",
    "bevy_state",
    "bevy_ui",
    "bevy_northstar",
    DEFAULT_CRATE_PATH,
];

const DEFAULT_MAX_RESULTS: usize = 25;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub current_ns: String,
    pub with_crates: Vec<String>,
    pub without_crates: Vec<String>,
    pub max_results: usize,
}

pub fn try_load_config(path: &PathBuf) -> Result<Config, Error> {
    let s = std::fs::read_to_string(path)?;
    toml::from_str(s.as_ref()).map_err(|e| e.into())
}

pub fn write_default_config(path: &PathBuf) -> Result<Config, Error> {
    let s = toml::to_string_pretty::<Config>(&Config::default())?;
    let _ = std::fs::write(path, s)?;
    Ok(Config::default())
}

fn crates(crates: &[&'static str]) -> Vec<String> {
    crates.iter().map(|it| it.to_string()).collect::<Vec<_>>()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            current_ns: "wanderrust".into(),
            with_crates: crates(DEFAULT_WITH_CRATES),
            without_crates: crates(Default::default()),
            max_results: DEFAULT_MAX_RESULTS,
        }
    }
}
