//! Persistent app config (~/.config/eve-settings-manager/config.json).

use crate::theme::{Choice, Custom, Mode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const APP_ID: &str = "eve-settings-manager";

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_ID)
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_ID)
}

/// Where the matugen template writes its output.
pub fn matugen_colors_path() -> PathBuf {
    config_dir().join("colors.json")
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Store {
    /// Notes per account id.
    pub descriptions: HashMap<String, String>,
    /// Last selected profile per server folder path.
    pub server_profiles: HashMap<String, String>,
    /// Cached character names by id.
    pub char_names: HashMap<String, String>,
    pub last_active_server: String,
    pub custom_eve_folder: String,
    pub theme_mode: Mode,
    /// None = not chosen yet (matugen if available, otherwise Caldari).
    pub theme: Option<Choice>,
    pub custom_theme: Custom,
}

impl Store {
    fn path() -> PathBuf {
        config_dir().join("config.json")
    }

    pub fn load() -> Store {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(config_dir());
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let tmp = Self::path().with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(tmp, Self::path());
            }
        }
    }
}
