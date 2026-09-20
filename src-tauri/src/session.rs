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

/// How long a closed window stays in the saved session. Quitting is every
/// window closing one after another, and each close restarts this clock — so
/// a quit ends with all of them still written down, and a window closed on
/// purpose while the app carries on drops out once this has passed.
pub const CLOSE_GRACE: Duration = Duration::from_secs(4);

/// How far past the grace the follow-up save runs, so that it lands on the far
/// side of `chain_expired`'s comparison rather than on it.
const GRACE_MARGIN: Duration = Duration::from_millis(50);

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

/// Paths as the frontend compares them: by case only where the filesystem does.
fn same_path(a: &str, b: &str) -> bool {
    if cfg!(any(windows, target_os = "macos")) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

/// Bring the saved window showing `path` to the front of the list, with that
/// tab active. First rather than last because the first saved window is the
/// one that takes `main`, and `main` is what the others then stand behind —
/// see `restore_session`. False, and nothing moved, when no tab has it.
pub fn surface(windows: &mut Vec<WindowSession>, path: &str) -> bool {
    let hit = windows.iter().enumerate().find_map(|(w, window)| {
        let tab = window.open.tabs.iter().position(|t| {
            t.get("path")
                .and_then(Value::as_str)
                .is_some_and(|p| same_path(p, path))
        })?;
        Some((w, tab))
    });
    let Some((w, tab)) = hit else {
        return false;
    };
    windows[w].open.active = tab;
    let window = windows.remove(w);
    windows.insert(0, window);
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WindowSession {
    #[serde(flatten)]
    pub open: OpenTabs,
    /// Missing when the compositor would not say where the window was.
    pub frame: Option<Frame>,
}

/// What a missing `restart` means — see the field.
fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Session {
    /// The version being installed. Only that version's first launch is the
    /// restart this was written for: on Windows the process is gone before the
    /// installer has done anything, so a cancelled install leaves this file
    /// behind, and whatever launches next must not treat it as a restart —
    /// see `read_from`. Means nothing on an ordinary session.
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
    ///
    /// Missing means true: 1.6.2 and earlier wrote this file for restarts
    /// only, and had no field to say so.
    #[serde(default = "yes")]
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

    // Lossy for the reason `setup` gives: `args()` panics on a name that is
    // not Unicode.
    let args = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    save(app, version, args, true);
}

/// Write down what is open right now, for an ordinary launch to come back to.
/// Called whenever a window reports a change, when one closes, and once more
/// when the grace after a close is over — nothing fires as the app quits, so
/// the last write before the end is what comes back.
///
/// Nothing is written when nothing is open and no window has just closed: the
/// reader closing their last tab must leave the waiting session where it is
/// rather than replacing it with the nothing that follows it.
///
/// Main thread only. This reads the state, then writes and renames, and two
/// of them on different threads can finish out of order and leave the older
/// content on disk. Synchronous commands and window events are there already;
/// anything else gets here through `run_on_main_thread`. A lock around `save`
/// would not do instead: `Frame::of` asks the window where it stands, which
/// off the main thread is a round trip through the event loop, and a main
/// thread parked on that lock would never answer it.
pub fn remember(app: &AppHandle) {
    let state = app.state::<AppState>();
    if *state.reopen.lock().unwrap() == "off" {
        return;
    }
    if state.installing.load(Ordering::SeqCst) {
        return;
    }
    let any_open_tab = state
        .sessions
        .lock()
        .unwrap()
        .values()
        .any(|open| !open.tabs.is_empty());
    {
        let mut closed = state.closed.lock().unwrap();
        if chain_expired(closed.last().map(|(t, _)| *t), Instant::now(), any_open_tab) {
            closed.clear();
        }
        if !any_open_tab && closed.is_empty() {
            return;
        }
    }
    save(app, app.package_info().version.to_string(), vec![], false);
}

/// Note where a window stands while it can still be asked — it is about to
/// close, and the save that follows has only this to go on.
pub fn note_frame(app: &AppHandle, label: &str) {
    if let Some(frame) = app.get_webview_window(label).and_then(|w| Frame::of(&w)) {
        app.state::<AppState>()
            .frames
            .lock()
            .unwrap()
            .insert(label.to_string(), frame);
    }
}

/// A window has gone. Whether the reader closed it on purpose or is on the way
/// out of the app cannot be told from here — a quit is every window closing
/// one after another — so it is not forgotten yet: it joins the chain, the
/// file is written with it still in, and a second save once the grace is over
/// drops it if the app is still running by then.
///
/// Runs on the main thread (a window event), as every ordinary save must.
pub fn window_closed(app: &AppHandle, label: &str) {
    let state = app.state::<AppState>();
    let open = state.sessions.lock().unwrap().remove(label);
    let frame = state.frames.lock().unwrap().remove(label);
    // "Start fresh" keeps nothing, and that includes this.
    if *state.reopen.lock().unwrap() == "off" {
        return;
    }
    // An empty window is never written, open or closed.
    let Some(open) = open.filter(|open| !open.tabs.is_empty()) else {
        return;
    };

    // Judged with this window still counted as open: closing the one window
    // that has documents, long after some other close, must drop that old
    // chain rather than carry it along.
    {
        let mut closed = state.closed.lock().unwrap();
        if chain_expired(closed.last().map(|(t, _)| *t), Instant::now(), true) {
            closed.clear();
        }
        closed.push((Instant::now(), WindowSession { open, frame }));
    }
    remember(app);

    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(CLOSE_GRACE + GRACE_MARGIN);
        // Back on the main thread for the write itself — see `remember`.
        // Every one of these does the same thing, so a run of closes leaves
        // nothing to cancel.
        let app = handle.clone();
        let _ = handle.run_on_main_thread(move || remember(&app));
    });
}

/// A window has answered `snapshot` — or gone away, which is as much of an
/// answer as it will give.
pub fn reported(state: &AppState, label: &str) {
    state.awaiting.lock().unwrap().remove(label);
    state.reported.notify_all();
}

/// Whether the chain of recent closes has had its time. Two conditions, and
/// the second is the one that keeps a lone close from wiping the session:
/// with no tab open anywhere, the chain is all there is to come back to.
fn chain_expired(last_close: Option<Instant>, now: Instant, any_open_tab: bool) -> bool {
    any_open_tab && last_close.is_some_and(|t| now.duration_since(t) >= CLOSE_GRACE)
}

/// What a save writes: the open windows, then the chain with its newest close
/// last. Quitting by hand closes the window in front first, so reversing the
/// chain puts the windows back in the order they stood in.
fn restorable(open: Vec<WindowSession>, chain: &[(Instant, WindowSession)]) -> Vec<WindowSession> {
    open.into_iter()
        .chain(chain.iter().rev().map(|(_, w)| w.clone()))
        .collect()
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

    let open: Vec<WindowSession> = windows
        .into_iter()
        .map(|(label, open)| {
            // A window still being built has no frame to speak of yet, and a
            // minimized one has none worth keeping — the last one it did have
            // stands in, which is also what a closed window is written with.
            let live = app.get_webview_window(&label).and_then(|w| Frame::of(&w));
            let mut frames = state.frames.lock().unwrap();
            if let Some(frame) = &live {
                frames.insert(label.clone(), frame.clone());
            }
            WindowSession {
                frame: live.or_else(|| frames.get(&label).cloned()),
                open,
            }
        })
        .collect();

    // A restart comes back as the app stood; a window closed a moment before
    // the update was closed. Only an ordinary save carries the chain.
    let windows = if restart {
        open
    } else {
        restorable(open, &state.closed.lock().unwrap())
    };

    let session = Session {
        version,
        argv,
        restart,
        windows,
    };
    config::write_json(&file(), &session);
}

/// What the previous process left behind. Left on disk: whether it is consumed
/// depends on what the reader has asked for, and an offer they never take has
/// to still be there at the next launch. Nothing has to clean that up either —
/// the first thing this run opens writes over it.
pub fn read(app: &AppHandle) -> Option<Session> {
    read_from(&file(), &app.package_info().version.to_string())
}

/// Used up, or thrown away: a restored session must not come back on every
/// launch after this one. A snapshot whose install never happened is not this
/// function's business — `read_from` demotes it.
pub fn discard() {
    discard_at(&file());
}

fn discard_at(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// A restart file is only a restart for the build it was written for. Found by
/// any other — the install was cancelled, or a different build came later — it
/// is still what the reader last had open, so it is demoted to an ordinary
/// session rather than dropped, and rewritten so it stays one. An ordinary
/// session is read whatever wrote it.
fn read_from(path: &Path, current: &str) -> Option<Session> {
    let mut session: Session = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    if session.restart && session.version != current {
        session.restart = false;
        session.argv.clear();
        config::write_json(path, &session);
    }
    Some(session)
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

    /// A restart file for some other version is an install that never
    /// happened. What it holds is still what the reader had open, so it comes
    /// back as an ordinary session — theirs to decide about — and the file is
    /// rewritten, so installing that version later does not force it on them.
    #[test]
    fn a_cancelled_install_becomes_an_ordinary_session() {
        let path = temp_path("cancelled");
        config::write_json(&path, &sample());
        let demoted = Session {
            restart: false,
            argv: vec![],
            ..sample()
        };
        assert_eq!(read_from(&path, "1.4.5"), Some(demoted));
        // On disk too: the version it was written for no longer sees a restart.
        assert!(!read_from(&path, "1.4.6").unwrap().restart);
        discard_at(&path);
    }

    /// An ordinary session belongs to the reader, not to a build: updating by
    /// hand, or through a package manager, must not cost them their windows.
    #[test]
    fn an_ordinary_session_survives_a_version_change() {
        let ordinary = Session {
            argv: vec![],
            restart: false,
            ..sample()
        };
        let path = temp_path("upgraded");
        config::write_json(&path, &ordinary);
        assert_eq!(read_from(&path, "9.9.9"), Some(ordinary));
        discard_at(&path);
    }

    /// 1.6.2 and earlier wrote this file for update restarts and nothing else,
    /// and wrote no `restart` into it. Read as an ordinary session, the update
    /// that brought this build in would offer what it promised to restore.
    #[test]
    fn a_file_without_restart_is_a_restart() {
        let s: Session =
            serde_json::from_str(r#"{"version":"1.6.3","argv":[],"windows":[]}"#).unwrap();
        assert!(s.restart);
    }

    /// Launched on a file the session already has: that window moves to the
    /// front of the list — the first one takes `main` — with that tab active,
    /// instead of the file opening a second time beside it.
    #[test]
    fn surfacing_a_held_file_picks_its_window_and_tab() {
        let window = |paths: &[&str]| WindowSession {
            open: OpenTabs {
                tabs: paths.iter().map(|p| json!({ "path": p })).collect(),
                active: 0,
            },
            frame: None,
        };
        let mut windows = vec![window(&["/n/a.md"]), window(&["/n/b.md", "/n/c.md"])];

        assert!(surface(&mut windows, "/n/c.md"));
        assert_eq!(windows[0].open.tabs.len(), 2);
        assert_eq!(windows[0].open.active, 1);
        assert_eq!(windows[1].open.tabs[0], json!({ "path": "/n/a.md" }));

        let before = windows.clone();
        assert!(!surface(&mut windows, "/n/zzz.md"));
        assert_eq!(windows, before);
    }

    /// The chain goes when its last close is older than the grace *and*
    /// something is open to take its place. With no tab open anywhere — the
    /// window with the documents was the one that closed — it waits, however
    /// long: what it holds is the only thing worth coming back to.
    #[test]
    fn the_chain_expires_after_the_grace_once_something_is_open() {
        let closed = Instant::now();
        let inside = closed + CLOSE_GRACE / 2;
        let after = closed + CLOSE_GRACE;

        assert!(!chain_expired(Some(closed), inside, true));
        assert!(chain_expired(Some(closed), after, true));
        assert!(!chain_expired(Some(closed), after, false));
        assert!(!chain_expired(None, after, true));
    }

    /// Open windows first, then the chain newest-close-last. Quitting by hand
    /// closes the window in front first, so the chain runs front-to-back and
    /// is written back-to-front — the order the windows stood in.
    #[test]
    fn the_chain_is_written_behind_first() {
        let window = |path: &str| WindowSession {
            open: OpenTabs {
                tabs: vec![json!({ "path": path })],
                active: 0,
            },
            frame: None,
        };
        let now = Instant::now();
        // B was in front and closed first, then A.
        let chain = vec![(now, window("b.md")), (now, window("a.md"))];

        let quit = restorable(vec![], &chain);
        assert_eq!(quit, vec![window("a.md"), window("b.md")]);

        let mid = restorable(vec![window("c.md")], &chain[..1]);
        assert_eq!(mid, vec![window("c.md"), window("b.md")]);
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
