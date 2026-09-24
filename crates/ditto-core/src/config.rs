use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub max_items: usize,
    pub ignored_apps: Vec<String>,
    pub escape_clears_search: bool,
    pub theme: String,
    pub opacity: usize,
    pub custom_accent: String,
    pub custom_primary: String,
    pub custom_secondary: String,
    pub persistent_window: bool,
    #[serde(default)]
    pub window_x: Option<f32>,
    #[serde(default)]
    pub window_y: Option<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_items: 100,
            ignored_apps: vec![
                "1password".to_string(),
                "bitwarden".to_string(),
                "keepassxc".to_string(),
            ],
            escape_clears_search: true,
            theme: "dark".to_string(),
            opacity: 75,
            custom_accent: "#eab308".to_string(),
            custom_primary: "#f3f4f6".to_string(),
            custom_secondary: "#9ca3af".to_string(),
            persistent_window: false,
            window_x: None,
            window_y: None,
        }
    }
}

impl Config {
    pub fn file_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ditto")
            .join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::file_path();
        if !path.exists() {
            let default_config = Self::default();
            let _ = default_config.save();
            return default_config;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Self>(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!(
                        "ditto: warning: config file corrupted ({}), resetting to defaults",
                        e
                    );
                    let default_config = Self::default();
                    let _ = default_config.save();
                    default_config
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::file_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, content)
    }
}
