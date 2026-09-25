use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub const DEFAULT_THEME: &str = "azure-devops-dark";

/// Where a newly opened file lands: `"tab"` in the focused window, or
/// `"window"` for one window per document.
pub const DEFAULT_OPEN_MODE: &str = "tab";

/// What a launch does with the windows the last one left behind: `"restore"`
/// to bring them straight back, `"ask"` to offer them on the empty screen, or
/// `"off"` to keep no session at all. Asking is the default because coming
/// back with windows nobody asked for is the more startling of the two.
pub const DEFAULT_REOPEN: &str = "ask";

/// The reopen setting as one of the three values the app acts on. The file is
/// the user's to edit, and every reader of this compares strings.
pub fn reopen_mode(raw: &str) -> &'static str {
    match raw {
        "restore" => "restore",
        "off" => "off",
        _ => DEFAULT_REOPEN,
    }
}

/// The sidebar's sort as one of the two values the app acts on, for the same
/// reason as `reopen_mode`: the menu ticks one of exactly two rows.
pub fn folder_sort(raw: &str) -> &'static str {
    match raw {
        "modified" => "modified",
        _ => "name",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    pub open_mode: String,
    /// Whether to ask GitHub for a newer release once per launch. On by
    /// default: an unsigned app that never mentions its own updates is how
    /// people end up running a year-old build.
    pub auto_update_check: bool,
    /// Where a picker starts when no document is open. Written whenever a
    /// folder opens in the sidebar.
    pub last_folder: String,
    /// What to do at startup with the session the last run left behind — see
    /// `DEFAULT_REOPEN`.
    pub reopen: String,
    /// How the sidebar orders a folder's files: `"modified"` for newest first,
    /// anything else by name. One setting for every folder, because the
    /// question people ask of a list is about the list, not about where it is.
    pub folder_sort: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_string(),
            open_mode: DEFAULT_OPEN_MODE.to_string(),
            auto_update_check: true,
            last_folder: String::new(),
            reopen: DEFAULT_REOPEN.to_string(),
            folder_sort: "name".to_string(),
        }
    }
}

/// Where a picker should open: the caller's candidate, else the folder last
/// opened, else home. Each is checked because a picker given a path that has
/// gone opens its *parent* with the name pre-filled, and either candidate can
/// have been renamed or unplugged since it was recorded.
///
/// The check is a blocking stat, so a saved folder on a share that has gone
/// away holds the picker for as long as the share takes to answer. Accepted:
/// the alternative is handing the dialog a path it would open the *parent* of,
/// and the same stat is what every folder this app lists already pays.
pub fn start_dir(candidate: &str, saved: &str) -> String {
    for dir in [candidate, saved] {
        if !dir.is_empty() && Path::new(dir).is_dir() {
            return dir.to_string();
        }
    }
    dirs::home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `%APPDATA%\t4-markdown-viewer`
#[cfg(not(test))]
pub fn dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("t4-markdown-viewer")
}

/// Never the reader's own folder under test: each thread gets a temp folder of
/// its own, gone when the thread is. Per thread because tests run in parallel,
/// each on its own thread, and every setting and session they write lands in
/// here — a thread a test spawns writes to a folder of its own, not the test's.
#[cfg(test)]
pub fn dir() -> PathBuf {
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    thread_local! {
        static SCRATCH: Scratch = Scratch(std::env::temp_dir().join(format!(
            "t4-test-config-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )));
    }
    SCRATCH.with(|s| s.0.clone())
}

fn file() -> PathBuf {
    dir().join("config.json")
}

pub fn load() -> Config {
    let mut cfg: Config = std::fs::read_to_string(file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    cfg.reopen = reopen_mode(&cfg.reopen).to_string();
    cfg.folder_sort = folder_sort(&cfg.folder_sort).to_string();
    cfg
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
/// The temp name carries the process id and a counter, because two writes can
/// be in here at once: an update snapshot writes `session.json` from the
/// install task while ordinary saves run on the main thread, and a shared temp
/// name would let one truncate the other's bytes before either rename lands.
/// The config setters cannot collide this way — they are synchronous commands,
/// and Tauri runs those on the main thread one at a time.
///
/// The temp file is flushed to disk before the rename: otherwise a power cut
/// can keep the rename but not the bytes, leaving an empty file where the old
/// one was — the same defaults, by another road. About half a millisecond on
/// an SSD, and saves are already debounced.
pub fn write_json(path: &Path, value: &impl Serialize) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(value) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("{}-{n}.tmp", std::process::id()));
        let written = std::fs::File::create(&tmp).and_then(|mut f| {
            std::io::Write::write_all(&mut f, json.as_bytes())?;
            f.sync_all()
        });
        if written.is_err() || std::fs::rename(&tmp, path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

/// Delete the temp files `write_json` left behind: a process killed between
/// the write and the rename leaves its `<name>.<pid>-<n>.tmp` beside the target
/// for good. Only that shape of name, only in this folder — never the themes
/// under it — and only once a minute old, so a write still in flight is never
/// pulled out from under its rename. Failure is silent, as it is for the write.
pub fn sweep_temps() {
    let Ok(entries) = std::fs::read_dir(dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > std::time::Duration::from_secs(60));
        if old && is_temp(&entry.file_name().to_string_lossy()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// `<something>.<digits>-<digits>.tmp`, the name `write_json` gives its temp.
fn is_temp(name: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    name.strip_suffix(".tmp")
        .and_then(|s| s.rsplit_once('.'))
        .and_then(|(stem, tail)| Some((stem, tail.split_once('-')?)))
        .is_some_and(|(stem, (pid, n))| !stem.is_empty() && digits(pid) && digits(n))
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

    /// And once more for `last_folder`: a config from before pickers remembered
    /// anything must load with an empty folder, not parse to nothing and take
    /// the user's theme down with it.
    #[test]
    fn config_without_last_folder_still_loads() {
        let c: Config = serde_json::from_str(
            r#"{"theme":"dracula","open_mode":"window","auto_update_check":false}"#,
        )
        .unwrap();
        assert_eq!(c.theme, "dracula");
        assert_eq!(c.open_mode, "window");
        assert!(!c.auto_update_check);
        assert_eq!(c.last_folder, "");
    }

    /// And once more for `reopen`: every config written before sessions were
    /// kept must arrive on the default, not parse to nothing and take the
    /// user's theme down with it.
    #[test]
    fn config_without_reopen_still_loads() {
        let c: Config = serde_json::from_str(
            r#"{"theme":"dracula","open_mode":"window","auto_update_check":false,"last_folder":"C:\\notes"}"#,
        )
        .unwrap();
        assert_eq!(c.theme, "dracula");
        assert_eq!(c.last_folder, "C:\\notes");
        assert_eq!(c.reopen, DEFAULT_REOPEN);
    }

    /// And once more for `folder_sort`: every config written before the sidebar
    /// could be sorted must arrive sorted by name, as it always was.
    #[test]
    fn config_without_folder_sort_still_loads() {
        let c: Config = serde_json::from_str(r#"{"theme":"dracula","reopen":"restore"}"#).unwrap();
        assert_eq!(c.theme, "dracula");
        assert_eq!(c.reopen, "restore");
        assert_eq!(c.folder_sort, "name");
    }

    #[test]
    fn folder_sort_accepts_only_the_two() {
        assert_eq!(folder_sort("modified"), "modified");
        assert_eq!(folder_sort("name"), "name");
        assert_eq!(folder_sort("Modified"), "name");
        assert_eq!(folder_sort(""), "name");
    }

    /// A hand-edited `"Ask"` used to fall through every comparison and restore,
    /// with no radio selected in Settings. Anything unrecognised is the default.
    #[test]
    fn reopen_mode_accepts_only_the_three() {
        assert_eq!(reopen_mode("restore"), "restore");
        assert_eq!(reopen_mode("off"), "off");
        assert_eq!(reopen_mode("ask"), "ask");
        assert_eq!(reopen_mode("Ask"), DEFAULT_REOPEN);
        assert_eq!(reopen_mode(""), DEFAULT_REOPEN);
    }

    /// The candidate wins when it is a real directory: the folder of what is on
    /// screen is a better guess than anything saved earlier.
    #[test]
    fn start_dir_prefers_a_real_candidate() {
        let real = env!("CARGO_MANIFEST_DIR");
        assert_eq!(start_dir(real, "Z:\\nope"), real);
    }

    /// Empty or gone, the candidate falls through to the saved folder rather
    /// than being handed to a dialog that would open its parent.
    #[test]
    fn start_dir_falls_back_to_saved() {
        let real = env!("CARGO_MANIFEST_DIR");
        assert_eq!(start_dir("", real), real);
        assert_eq!(start_dir("Z:\\nope", real), real);
    }

    /// With neither candidate standing, home is the one place always worth
    /// opening in.
    #[test]
    fn start_dir_falls_back_to_home() {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        assert_eq!(start_dir("", ""), home);
        assert_eq!(start_dir("Z:\\nope", "Z:\\gone"), home);
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

    /// Only an old temp of `write_json`'s own shape goes: a fresh one may be a
    /// write still under way, and nothing else here is the sweep's to delete.
    #[test]
    fn sweep_temps_takes_only_old_write_json_temps() {
        let dir = dir();
        std::fs::create_dir_all(dir.join("themes.1-2.tmp")).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        for name in [
            "config.29232-1.tmp",
            "notes.tmp",
            "config.abc-1.tmp",
            "config.json",
        ] {
            std::fs::File::create(dir.join(name))
                .unwrap()
                .set_modified(old)
                .unwrap();
        }
        std::fs::File::create(dir.join("session.1-3.tmp")).unwrap();

        sweep_temps();

        assert!(!dir.join("config.29232-1.tmp").exists());
        for kept in [
            "session.1-3.tmp",
            "notes.tmp",
            "config.abc-1.tmp",
            "config.json",
            "themes.1-2.tmp",
        ] {
            assert!(dir.join(kept).exists(), "{kept} was deleted");
        }
    }
}
