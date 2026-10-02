use std::path::PathBuf;

use anyhow::Error;
use log::{info, warn};
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG_PATH: &str = "ornis.toml";

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

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Config {
    pub crate_path: String,
    pub with_crates: Vec<String>,
    pub without_crates: Vec<String>,
    pub max_results: usize,
}

pub fn config(path: impl AsRef<PathBuf>) -> Config {
    let p = path.as_ref();
    match try_load_config(p) {
        Ok(c) => c,
        Err(e) => {
            warn!("unable to load config at {p:?}: {e}; using default config");
            let c = Config::default();
            info!("config: {c:#?}");
            c
        }
    }
}

fn try_load_config(path: &PathBuf) -> Result<Config, Error> {
    let s = std::fs::read_to_string(path)?;
    toml::from_str(s.as_ref()).map_err(|e| e.into())
}

pub fn write_default_config(path: &PathBuf) -> Result<Config, Error> {
    let s = toml::to_string_pretty::<Config>(&Config::default())?;
    let _ = std::fs::write(path, s)?;
    Ok(Config::default())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            crate_path: "wanderrust".into(),
            with_crates: Default::default(),
            without_crates: Default::default(),
            max_results: Default::default(),
        }
    }
}
