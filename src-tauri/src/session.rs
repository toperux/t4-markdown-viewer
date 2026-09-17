//! Carrying the open documents across an update restart.
//!
//! Installing an update replaces the process, and the relaunch only knows the
//! arguments the old one started with — at most the one file it was
//! double-clicked on, and nothing at all when it came from the Start menu. So
//! once the update is downloaded every window is asked what it has open, and
//! the moment before the installer takes over that is written to disk for the
//! next launch to pick up.
//!
//! The same file now also tracks ordinary use — `remember` rewrites it as tabs
//! come and go — so an ordinary launch has something to come back to as well.
//! There is no hook that fires as the last window goes, so the only way to
//! know what was open at the end is to have been writing it all along.

use crate::{config, AppState};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};

/// How long `snapshot` waits for the windows to answer. A webview that is
/// busy for longer than this keeps whatever it last reported.
const REPORT_TIMEOUT: Duration = Duration::from_secs(1);

/// What one window reports: its tabs in the frontend's own shape — the same
/// JSON a tab travels in when dragged between windows — and which is active.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OpenTabs {
    pub tabs: Vec<Value>,
    pub active: usize,
}

impl OpenTabs {
    /// What a window will have open once it boots, read off the payload it
    /// was created with, so a window that has not reported yet still counts.
    pub fn from_pending(payload: &Value) -> Option<Self> {
        match payload.get("kind")?.as_str()? {
            "path" => Some(Self {
                tabs: vec![json!({ "path": payload.get("path")? })],
                active: 0,
            }),
            "tab" => Some(Self {
                tabs: vec![payload.get("tab")?.clone()],
                active: 0,
            }),
            // An offer opens nothing: the window shows the empty screen with a
            // button on it. Spelled out rather than left to the catch-all,
            // which would otherwise be free to read the counts as a tab list
            // and have the window report the offer back as its own.
            "offer" => None,
            _ => serde_json::from_value(payload.clone()).ok(),
        }
    }
}

/// A window's place on screen, in physical pixels so it survives mixed-DPI
/// monitors the same way the window itself does.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Frame {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Frame {
    fn of(window: &WebviewWindow) -> Option<Self> {
        // Windows parks a minimized window at (-32000, -32000); restoring that
        // would put it off every screen. No frame means default placement.
        if window.is_minimized().unwrap_or(false) {
            return None;
        }
        let pos = window.outer_position().ok()?;
        let size = window.inner_size().ok()?;
        Some(Self {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
            maximized: window.is_maximized().unwrap_or(false),
        })
    }

    /// Put a window back where it stood. A frame no attached monitor shows any
    /// of — a display unplugged since the snapshot — is left alone, so the
    /// window gets default placement rather than landing where nothing can
    /// reach it.
    ///
    /// A maximized frame gets its position only: that picks the monitor, while
    /// the saved size is the monitor's own and would leave un-maximizing with a
    /// screen-sized window. Maximizing itself is left to the frontend, once it
    /// shows the window — on Windows, maximizing a hidden window shows it.
    pub fn apply(&self, window: &WebviewWindow) {
        let on_screen = window
            .available_monitors()
            .map(|monitors| monitors.iter().any(|m| self.overlaps(m)))
            .unwrap_or(true);
        if !on_screen {
            return;
        }
        let _ = window.set_position(PhysicalPosition::new(self.x, self.y));
        if !self.maximized {
            let _ = window.set_size(PhysicalSize::new(self.width, self.height));
        }
    }

    fn overlaps(&self, monitor: &Monitor) -> bool {
        let (pos, size) = (monitor.position(), monitor.size());
        self.x < pos.x + size.width as i32
            && self.x + self.width as i32 > pos.x
            && self.y < pos.y + size.height as i32
            && self.y + self.height as i32 > pos.y
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WindowSession {
    #[serde(flatten)]
    pub open: OpenTabs,
    /// Missing when the compositor would not say where the window was.
    pub frame: Option<Frame>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Session {
    /// The version being installed. Only that version's first launch is the
    /// restart this was written for: on Windows the process is gone before the
    /// installer has done anything, so a cancelled install leaves this file
    /// behind, and whatever launches next must not pick it up.
    pub version: String,
    /// The arguments the process was started with, minus the program. The
    /// relaunch repeats them verbatim, and this is how that echo is told apart
    /// from a genuine file-association open. Empty for everything but an
    /// update snapshot: nothing else is followed by a relaunch that repeats
    /// them, and recording them would have the launch after this one mistake a
    /// freshly double-clicked file for the echo and ignore it.
    pub argv: Vec<String>,
    /// Written by an update snapshot, and by nothing else. A restart is the
    /// app coming back mid-read rather than a new launch, so it restores
    /// whatever the reopen setting says about ordinary ones — the reader
    /// pressed Update, not Quit.
    #[serde(default)]
    pub restart: bool,
    /// Least-recently-focused first. Restoring in that order is the intent;
    /// each window still comes forward as its own document finishes loading.
    pub windows: Vec<WindowSession>,
}

fn file() -> PathBuf {
    config::dir().join("session.json")
}

/// Ask every live window where its reader is, wait for the answers, and write
/// the session for `version` to pick up. Called with the update downloaded
/// and nothing else left before the install replaces the process.
///
/// The wait is bounded: a webview too busy to answer keeps what it last
/// reported, or what it was created to open if it never has.
pub async fn snapshot(app: &AppHandle, version: String) {
    let state = app.state::<AppState>();
    // From here on the file belongs to the restart. The windows carry on
    // reporting — `sessions` wants to be current when the save comes — but
    // none of those reports may write.
    state.installing.store(true, Ordering::SeqCst);
    let ready = state.boot.lock().unwrap().ready.clone();
    *state.awaiting.lock().unwrap() = ready;
    let _ = app.emit("update-installing", ());

    let waiter = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        let state = waiter.state::<AppState>();
        let deadline = Instant::now() + REPORT_TIMEOUT;
        let mut awaiting = state.awaiting.lock().unwrap();
        while !awaiting.is_empty() {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            awaiting = state.reported.wait_timeout(awaiting, left).unwrap().0;
        }
    })
    .await;

    save(app, version, std::env::args().skip(1).collect(), true);
}

/// Write down what is open right now, for an ordinary launch to come back to.
/// Called whenever a window reports a change, because nothing fires as the app
/// quits — the last report before the end is what comes back.
///
/// Nothing is written when every window is empty: the reader closing their
/// last tab, or the last window going, must leave the waiting session where it
/// is rather than replacing it with the nothing that follows it.
pub fn remember(app: &AppHandle) {
    if config::load().reopen == "off" {
        return;
    }
    let state = app.state::<AppState>();
    if state.installing.load(Ordering::SeqCst) {
        return;
    }
    let nothing_open = state
        .sessions
        .lock()
        .unwrap()
        .values()
        .all(|open| open.tabs.is_empty());
    if nothing_open {
        return;
    }
    save(app, app.package_info().version.to_string(), vec![], false);
}

/// A window has answered `snapshot` — or gone away, which is as much of an
/// answer as it will give.
pub fn reported(state: &AppState, label: &str) {
    state.awaiting.lock().unwrap().remove(label);
    state.reported.notify_all();
}

/// Write out every window that has something open.
fn save(app: &AppHandle, version: String, argv: Vec<String>, restart: bool) {
    let state = app.state::<AppState>();
    let order = state.focus_order.lock().unwrap().clone();
    let open = state.sessions.lock().unwrap().clone();

    // Least-recently-focused first. A window the OS never let take focus is
    // not in the focus list, but it has documents open all the same; those go
    // first, behind every window the user did click on.
    let mut windows: Vec<(String, OpenTabs)> = open
        .into_iter()
        .filter(|(_, open)| !open.tabs.is_empty())
        .collect();
    windows.sort_by_cached_key(|(label, _)| (order.iter().position(|l| l == label), label.clone()));

    let windows = windows
        .into_iter()
        .map(|(label, open)| WindowSession {
            // A window still being built has no frame to speak of yet.
            frame: app.get_webview_window(&label).and_then(|w| Frame::of(&w)),
            open,
        })
        .collect();

    let session = Session {
        version,
        argv,
        restart,
        windows,
    };
    config::write_json(&file(), &session);
}

/// What the previous process left behind, if this build is the one it was
/// written for. Left on disk: whether it is consumed depends on what the
/// reader has asked for, and an offer they never take has to still be there
/// at the next launch. Nothing has to clean that up either — the first thing
/// this run opens writes over it.
pub fn read(app: &AppHandle) -> Option<Session> {
    read_from(&file(), &app.package_info().version.to_string())
}

/// Used up, or thrown away: a restored session must not come back on every
/// launch after this one, and an install that failed after the snapshot must
/// not ambush some later, ordinary launch with the file it left.
pub fn discard() {
    discard_at(&file());
}

fn discard_at(path: &Path) {
    let _ = std::fs::remove_file(path);
}

fn read_from(path: &Path, current: &str) -> Option<Session> {
    serde_json::from_str::<Session>(&std::fs::read_to_string(path).ok()?)
        .ok()
        .filter(|s| s.version == current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Session {
        Session {
            version: "1.4.6".into(),
            argv: vec![r"C:\notes\a.md".into()],
            restart: true,
            windows: vec![WindowSession {
                open: OpenTabs {
                    tabs: vec![json!({ "path": r"C:\notes\a.md", "entries": [], "index": 0 })],
                    active: 0,
                },
                frame: Some(Frame {
                    x: 10,
                    y: 20,
                    width: 800,
                    height: 600,
                    maximized: false,
                }),
            }],
        }
    }

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("t4-session-{tag}-{}.json", std::process::id()))
    }

    /// A session that is restored is read once and then gone: the launch after
    /// an update picks it up, and the launch after that must start clean.
    #[test]
    fn a_restored_session_does_not_come_back() {
        let path = temp_path("once");
        config::write_json(&path, &sample());
        assert_eq!(read_from(&path, "1.4.6"), Some(sample()));
        discard_at(&path);
        assert_eq!(read_from(&path, "1.4.6"), None);
        assert!(!path.exists());
    }

    /// A session written for some other version is a leftover from an install
    /// that never happened — the version that wrote it relaunching, or a
    /// different build installed later — and is thrown away rather than
    /// restored.
    #[test]
    fn a_session_for_another_version_is_dropped() {
        let path = temp_path("stale");
        config::write_json(&path, &sample());
        assert_eq!(read_from(&path, "1.4.5"), None);
    }

    /// What `remember` writes as the reader works: the same file, with no
    /// arguments in it, and read back without being consumed — an offer nobody
    /// accepts has to still be there at the next launch.
    #[test]
    fn an_ordinary_save_round_trips_unconsumed() {
        let ordinary = || Session {
            argv: vec![],
            restart: false,
            ..sample()
        };
        let path = temp_path("ordinary");
        config::write_json(&path, &ordinary());

        assert_eq!(read_from(&path, "1.4.6"), Some(ordinary()));
        assert_eq!(read_from(&path, "1.4.6"), Some(ordinary()));
        assert!(path.exists());
        discard_at(&path);
    }

    /// A window that reported before Wayland refused its position still
    /// restores — without a frame rather than not at all.
    #[test]
    fn frame_is_optional() {
        let s: Session = serde_json::from_str(
            r#"{"version":"1.4.5","argv":[],"windows":[{"tabs":[],"active":0}]}"#,
        )
        .unwrap();
        assert_eq!(s.windows[0].frame, None);
    }

    /// Every kind of payload a window can be created with stands in for its
    /// report until the window makes one.
    #[test]
    fn pending_payloads_count_as_open() {
        let path = OpenTabs::from_pending(&json!({ "kind": "path", "path": "a.md" })).unwrap();
        assert_eq!(path.tabs, vec![json!({ "path": "a.md" })]);

        let tab = json!({ "path": "b.md", "entries": [{ "path": "b.md" }], "index": 0 });
        let torn = OpenTabs::from_pending(&json!({ "kind": "tab", "tab": tab })).unwrap();
        assert_eq!(torn.tabs, vec![tab]);

        let restored = OpenTabs::from_pending(
            &json!({ "kind": "session", "tabs": [{ "path": "c.md" }], "active": 0, "maximized": true }),
        )
        .unwrap();
        assert_eq!(restored.active, 0);
        assert_eq!(OpenTabs::from_pending(&json!({ "kind": "other" })), None);
    }

    /// Except an offer, which says how much is waiting rather than what to
    /// open. Counted as open, the boot window would report the offer back as
    /// its own tabs and the next save would write that out.
    #[test]
    fn an_offer_opens_nothing() {
        assert_eq!(
            OpenTabs::from_pending(&json!({ "kind": "offer", "windows": 2, "tabs": 3 })),
            None
        );
    }
}
