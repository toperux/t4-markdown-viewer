use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub const DEFAULT_THEME: &str = "azure-devops-dark";

/// Where a newly opened file lands: `"tab"` in the focused window, or
/// `"window"` for one window per document.
pub const DEFAULT_OPEN_MODE: &str = "tab";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    pub open_mode: String,
    /// Whether to ask GitHub for a newer release once per launch. On by
    /// default: an unsigned app that never mentions its own updates is how
    /// people end up running a year-old build.
    pub auto_update_check: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_string(),
            open_mode: DEFAULT_OPEN_MODE.to_string(),
            auto_update_check: true,
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
    write_json(&file(), cfg);
}

/// Write a value as JSON, creating the directory on the way. Failure is
/// silent: nothing this app persists is worth interrupting the reader over.
///
/// The bytes go to a sibling temp file and are renamed over `path`, which is
/// one step as far as anything reading is concerned. Writing in place would
/// truncate first, so a full disk or a kill in the middle would leave half a
/// file — and half a `config.json` parses as nothing, which `load` answers by
/// silently handing back the defaults the user had changed.
///
/// The temp name carries the process id and a counter, because `set_theme` and
/// `set_open_mode` are ordinary commands: two windows can be inside this
/// function at once, and a shared temp name lets one truncate the other's bytes
/// before either rename lands.
pub fn write_json(path: &Path, value: &impl Serialize) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(value) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("{}-{n}.tmp", std::process::id()));
        if std::fs::write(&tmp, json).is_err() || std::fs::rename(&tmp, path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
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

    /// Same contract one version later: a config written by 1.1.2 predates
    /// auto-update entirely, and must arrive opted in rather than parsed to
    /// nothing.
    #[test]
    fn config_without_auto_update_check_still_loads() {
        let c: Config =
            serde_json::from_str(r#"{"theme":"dracula","open_mode":"window"}"#).unwrap();
        assert_eq!(c.theme, "dracula");
        assert_eq!(c.open_mode, "window");
        assert!(c.auto_update_check);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let c: Config = serde_json::from_str(r#"{"theme":"dracula","future":1}"#).unwrap();
        assert_eq!(c.theme, "dracula");
    }

    /// The rename is the whole point: what lands is parseable, and the temp the
    /// bytes travelled through is not left beside it for the next reader to find.
    #[test]
    fn write_json_lands_whole_and_leaves_no_temp() {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("t4-write-json-{}-{n}", std::process::id()));
        let path = dir.join("config.json");

        let cfg = Config {
            theme: "dracula-blue".to_string(),
            ..Config::default()
        };
        write_json(&path, &cfg);

        let back: Config = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back.theme, "dracula-blue");
        let temps: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name()))
            .filter(|n| n.to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(temps.is_empty(), "temp files left behind: {temps:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
