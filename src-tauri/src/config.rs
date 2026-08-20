use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_THEME: &str = "azure-devops";

/// Where a newly opened file lands: `"tab"` in the focused window, or
/// `"window"` for one window per document.
pub const DEFAULT_OPEN_MODE: &str = "tab";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    pub open_mode: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_string(),
            open_mode: DEFAULT_OPEN_MODE.to_string(),
        }
    }
}

/// `%APPDATA%\t4-markdown-viewer`
pub fn dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("t4-markdown-viewer")
}

fn file() -> PathBuf {
    dir().join("config.json")
}

pub fn load() -> Config {
    std::fs::read_to_string(file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(cfg: &Config) {
    let _ = std::fs::create_dir_all(dir());
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(file(), json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Settings written before `open_mode` existed must still load, rather than
    /// failing to parse and silently resetting the user's theme.
    #[test]
    fn config_without_open_mode_still_loads() {
        let c: Config = serde_json::from_str(r#"{"theme":"dracula-blue"}"#).unwrap();
        assert_eq!(c.theme, "dracula-blue");
        assert_eq!(c.open_mode, DEFAULT_OPEN_MODE);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let c: Config = serde_json::from_str(r#"{"theme":"dracula","future":1}"#).unwrap();
        assert_eq!(c.theme, "dracula");
    }
}
