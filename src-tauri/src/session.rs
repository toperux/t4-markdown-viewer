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
//! A closing window reports on its way out, and the macOS Quit asks every
//! window before the app goes, but a crash or a kill fires nothing at all — so
//! the only way to know what was open at the end is to have been writing it
//! all along.

use crate::{config, AppState};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{
    AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow,
};

/// How long `gather` waits for the windows to answer, for the update snapshot
/// and the macOS Quit alike. A webview that is busy for longer than this keeps
/// whatever it last reported.
pub(crate) const REPORT_TIMEOUT: Duration = Duration::from_secs(1);

/// How long a closed window stays in the saved session. Quitting is every
/// window closing one after another, and each close restarts this clock — so
/// a quit ends with all of them still written down, and a window closed on
/// purpose while the app carries on drops out once this has passed.
#[cfg(not(test))]
pub const CLOSE_GRACE: Duration = Duration::from_secs(4);
/// Short enough for a test to wait out.
#[cfg(test)]
pub const CLOSE_GRACE: Duration = Duration::from_millis(300);

/// How far past the grace the follow-up save runs, so that it lands on the far
/// side of `chain_expired`'s comparison rather than on it.
const GRACE_MARGIN: Duration = Duration::from_millis(50);

/// What one window reports: its tabs in the frontend's own shape — the same
/// JSON a tab travels in when dragged between windows — and which is active.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OpenTabs {
    pub tabs: Vec<Value>,
    pub active: usize,
    /// Whether the window's sidebar was open. Missing means closed: nothing
    /// written before this field ever brought a sidebar back.
    #[serde(default)]
    pub sidebar: bool,
}

impl OpenTabs {
    /// What a window will have open once it boots, read off the payload it
    /// was created with, so a window that has not reported yet still counts.
    pub fn from_pending(payload: &Value) -> Option<Self> {
        match payload.get("kind")?.as_str()? {
            "path" => Some(Self {
                tabs: vec![json!({ "path": payload.get("path")? })],
                active: 0,
                sidebar: false,
            }),
            "tab" => Some(Self {
                tabs: vec![payload.get("tab")?.clone()],
                active: 0,
                sidebar: false,
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
    /// Whether a maximized frame's rectangle is the one the window had before
    /// the maximize, so un-maximizing goes back to it. False when it is the
    /// maximized rectangle itself — the only kind 1.7.3 and earlier wrote,
    /// hence the default — which must never become the window's normal size.
    /// Means nothing on a frame that is not maximized.
    #[serde(default)]
    pub normal: bool,
}

/// A physical rectangle: x, y, width, height.
type Rect = (i32, i32, u32, u32);

fn rect(monitor: &Monitor) -> Rect {
    let (pos, size) = (monitor.position(), monitor.size());
    (pos.x, pos.y, size.width, size.height)
}

/// What a window says about itself at one moment — what `sampled` decides on.
struct Sample {
    minimized: bool,
    fullscreen: bool,
    visible: bool,
    maximized: bool,
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    /// The monitor the window is on and every monitor attached. None when
    /// either cannot be read, and on Wayland, where no window knows where it
    /// stands.
    monitors: Option<(Rect, Vec<Rect>)>,
}

/// The frame to keep for a window, given the one kept before and how it stands
/// now.
///
/// A window not showing its own rectangle keeps the last one it did: minimized
/// (Windows parks it at (-32000, -32000), off every screen), in full screen
/// (it comes back at its size from before), or not shown yet — a restored
/// window saves before its maximize, and must not record itself un-maximized.
///
/// A maximized window's own rectangle is the screen's, and un-maximizing to
/// that gives a screen-sized window. So the normal rectangle kept before the
/// maximize stays, marked maximized — but only while it lies mostly on the
/// monitor the window is maximized on. Moved maximized to another monitor, the
/// window must come back there, and the maximized rectangle is what says so.
/// On Wayland there is no telling, and the normal one always stays.
fn sampled(old: Option<&Frame>, now: Sample) -> Option<Frame> {
    if now.minimized || now.fullscreen || !now.visible {
        return old.cloned();
    }
    let current = Frame {
        x: now.position.x,
        y: now.position.y,
        width: now.size.width,
        height: now.size.height,
        maximized: now.maximized,
        normal: false,
    };
    if !now.maximized {
        return Some(current);
    }
    let kept = old
        .filter(|old| !old.maximized || old.normal)
        .filter(|old| match &now.monitors {
            None => true,
            Some((on, all)) => old.home(all) == Some(*on),
        });
    Some(match kept {
        Some(old) => Frame {
            maximized: true,
            normal: true,
            ..old.clone()
        },
        None => current,
    })
}

impl Frame {
    /// Where the window stands now, judged against `old`, the frame kept for
    /// it before — see `sampled`. A position or size that cannot be read keeps
    /// `old` too.
    ///
    /// Asks the window, which off the main thread is a round trip through the
    /// event loop, so no lock may be held across this.
    fn of<R: Runtime>(window: &WebviewWindow<R>, old: Option<&Self>) -> Option<Self> {
        // Whether this is Wayland is known only once GTK answers; unknown, the
        // monitor check is skipped as it is on Wayland.
        #[cfg(all(target_os = "linux", not(test)))]
        let (size, wayland) = match gtk_read(window) {
            Some((size, wayland)) => (Some(size), Some(wayland)),
            None => (window.inner_size().ok(), None),
        };
        #[cfg(not(all(target_os = "linux", not(test))))]
        let (size, wayland) = (window.inner_size().ok(), Some(false));
        let (Ok(position), Some(size)) = (window.outer_position(), size) else {
            return old.cloned();
        };
        let monitors = match (
            wayland,
            window.current_monitor(),
            window.available_monitors(),
        ) {
            (Some(false), Ok(Some(on)), Ok(all)) => {
                Some((rect(&on), all.iter().map(rect).collect()))
            }
            _ => None,
        };
        sampled(
            old,
            Sample {
                minimized: window.is_minimized().unwrap_or(false),
                fullscreen: window.is_fullscreen().unwrap_or(false),
                visible: window.is_visible().unwrap_or(true),
                maximized: window.is_maximized().unwrap_or(false),
                position,
                size,
                monitors,
            },
        )
    }

    /// Put a window back where it stood. A frame no attached monitor shows any
    /// of — a display unplugged since the snapshot — is left alone, so the
    /// window gets default placement rather than landing where nothing can
    /// reach it.
    ///
    /// A maximized frame holding its normal rectangle gets all of it, so
    /// un-maximizing goes back there. One holding the maximized rectangle gets
    /// its position only: that picks the monitor, while the size is the
    /// monitor's own and would leave un-maximizing with a screen-sized window.
    /// Maximizing itself is left to the frontend, once it shows the window — on
    /// Windows, maximizing a hidden window shows it.
    pub fn apply<R: Runtime>(&self, window: &WebviewWindow<R>) {
        let on_screen = window
            .available_monitors()
            .map(|monitors| monitors.iter().any(|m| self.overlap_area(rect(m)) > 0))
            .unwrap_or(true);
        if !on_screen {
            return;
        }
        let _ = window.set_position(PhysicalPosition::new(self.x, self.y));
        if !self.maximized || self.normal {
            let _ = window.set_size(PhysicalSize::new(self.width, self.height));
        }
    }

    /// How much of a monitor's rectangle this frame covers, in square pixels.
    /// Also what `apply` asks of each monitor, as any overlap at all. Worked in
    /// `i64`, so no coordinate is far enough out to overflow it.
    fn overlap_area(&self, (x, y, width, height): Rect) -> u64 {
        let span = |a: i32, a_len: u32, b: i32, b_len: u32| {
            let (a, b) = (i64::from(a), i64::from(b));
            let end = (a + i64::from(a_len)).min(b + i64::from(b_len));
            (end - a.max(b)).max(0) as u64
        };
        span(self.x, self.width, x, width) * span(self.y, self.height, y, height)
    }

    /// The monitor this frame lies on most, which is how Windows picks the one
    /// to maximize on. The first wins a tie; none when it lies on none.
    fn home(&self, monitors: &[Rect]) -> Option<Rect> {
        let mut best = None;
        let mut most = 0;
        for &m in monitors {
            let area = self.overlap_area(m);
            if area > most {
                (best, most) = (Some(m), area);
            }
        }
        best
    }
}

/// The window's size as GTK has it, in physical pixels. tao's `inner_size` is
/// the size from the last configure event, which on Wayland includes the
/// client-side title bar and shadow, while `set_size` is `gtk_window_resize`,
/// which does not — so a restored window grew by the difference every launch
/// (+52×89 on GNOME Wayland; nothing on X11, where the two agree). This reads
/// the counterpart of that resize instead.
///
/// Also whether the display is Wayland, read in the same trip: there every
/// window reads as standing at 0,0, so where one is says nothing about the
/// monitor it is on.
///
/// GTK is main-thread only. `run_on_main_thread` runs the closure inline when
/// already there, so the reply is in the channel before `recv`; from any other
/// thread (the update snapshot's) it waits on the event loop, which is why no
/// lock may be held across this. A closure dropped unrun drops the sender and
/// ends the wait. Not in tests: the mock runtime has no GTK window.
#[cfg(all(target_os = "linux", not(test)))]
fn gtk_read<R: Runtime>(window: &WebviewWindow<R>) -> Option<(PhysicalSize<u32>, bool)> {
    use gtk::prelude::*;
    let (tx, rx) = std::sync::mpsc::channel();
    let w = window.clone();
    window
        .run_on_main_thread(move || {
            let _ = tx.send(w.gtk_window().ok().map(|g| {
                let wayland = g.display().type_().name() == "GdkWaylandDisplay";
                (g.size(), wayland)
            }));
        })
        .ok()?;
    let ((width, height), wayland) = rx.recv().ok()??;
    let scale = window.scale_factor().ok()?;
    Some((
        PhysicalSize::new(
            (width as f64 * scale).round() as u32,
            (height as f64 * scale).round() as u32,
        ),
        wayland,
    ))
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
    /// Set while a launch is putting this session back, until every window it
    /// brought back has shown its document — see `begin_restore`. Found set,
    /// the launch that wrote it went down on the way, and the next one offers
    /// rather than risk going down the same way. Missing means false: nothing
    /// written before this field was ever mid-restore.
    #[serde(default)]
    pub restoring: bool,
    /// Least-recently-focused first. Each restored window hands the front to
    /// the last one, or to `main` beside a held file — see `behind_of`.
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
    gather(app, "update-installing").await;

    // Lossy for the reason `setup` gives: `args()` panics on a name that is
    // not Unicode.
    let args = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    save(app, version, args, true);
}

/// Send `event` to every window that has booted and wait, up to
/// `REPORT_TIMEOUT`, for each to answer through `set_session`. Shared by the
/// update snapshot and the macOS Quit, which both need the last moment on
/// disk before the process goes.
///
/// A window that has not booted is not waited on: it has no listener yet, and
/// what it was created to open already stands in for its report. Filled
/// before the emit, so an answer that arrives at once still finds itself
/// awaited.
pub async fn gather<R: Runtime>(app: &AppHandle<R>, event: &str) {
    let state = app.state::<AppState>();
    let ready = state.boot.lock().unwrap().ready.clone();
    *state.awaiting.lock().unwrap() = ready;
    let _ = app.emit(event, ());

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
}

/// Write down what is open right now, for an ordinary launch to come back to.
/// Called whenever a window reports a change, when one closes, and once more
/// when the grace after a close is over. The macOS Quit asks every page
/// before it exits, but nothing else fires as the app goes, so the last write
/// before the end is what comes back.
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
pub fn remember<R: Runtime>(app: &AppHandle<R>) {
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
        if chain_expired(closed.last_close(), Instant::now(), any_open_tab) {
            closed.clear();
        }
        if !any_open_tab && closed.entries.is_empty() {
            return;
        }
    }
    save(app, app.package_info().version.to_string(), vec![], false);
}

/// Note where a window stands while it can still be asked — it is about to
/// close, and the save that follows has only this to go on.
pub fn note_frame<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    let state = app.state::<AppState>();
    // Copied and let go before the window is asked — see `Frame::of`.
    let old = state.frames.lock().unwrap().get(label).cloned();
    if let Some(frame) = Frame::of(&window, old.as_ref()) {
        state
            .frames
            .lock()
            .unwrap()
            .insert(label.to_string(), frame);
    }
}

/// Note where a window stands in the focus order while that order still says
/// something. Closing the window in front hands focus to the next, which would
/// then look the most recent of all, so the order is copied at the first
/// request of a chain and every close is ranked against that copy. Taken at
/// the request rather than at `Destroyed`: by then the OS may already have
/// handed the focus on, and with *Close all windows* every request comes
/// before any window goes.
///
/// Called on every request, whatever the reopen setting; `window_closed`
/// takes the rank out again.
pub fn note_close<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let state = app.state::<AppState>();
    // Copied and let go before `closed` is taken: no lock held across another.
    let focus = state.focus_order.lock().unwrap().clone();
    let mut closed = state.closed.lock().unwrap();
    // Judged as if this window had tabs — an empty one's request too, which is
    // harmless — so an old chain is not the one this request ranks against.
    if chain_expired(closed.last_close(), Instant::now(), true) {
        closed.clear();
    }
    let rank = closed.rank(label, &focus);
    closed.pending_rank.insert(label.to_string(), rank);
}

/// A window has gone. Whether the reader closed it on purpose or is on the way
/// out of the app cannot be told from here — a quit is every window closing
/// one after another — so it is not forgotten yet: it joins the chain, the
/// file is written with it still in, and a second save once the grace is over
/// drops it if the app is still running by then.
///
/// Runs on the main thread (a window event), as every ordinary save must.
pub fn window_closed<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let state = app.state::<AppState>();
    // Out first, whatever happens below: a rank left behind would keep the
    // chain looking begun, and the next quit would rank against this one's copy.
    let pending = state.closed.lock().unwrap().pending_rank.remove(label);
    restored(app, label);
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
    let focus = state.focus_order.lock().unwrap().clone();
    {
        let mut closed = state.closed.lock().unwrap();
        let expired = chain_expired(closed.last_close(), Instant::now(), true);
        if expired {
            closed.clear();
        }
        // A window that went without a request — or whose rank was taken
        // against the chain just dropped — is ranked now, the same way.
        let rank = match pending {
            Some(rank) if !expired => rank,
            _ => closed.rank(label, &focus),
        };
        closed
            .entries
            .push((Instant::now(), rank, WindowSession { open, frame }));
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

/// A window has answered `gather`, for the update snapshot or the macOS Quit —
/// or gone away, which is as much of an answer as it will give.
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

/// Windows that closed within the grace of one another, and where each stood
/// among the windows when the chain began — see `note_close`.
#[derive(Default)]
pub struct Chain {
    /// The focus order, least-recently-focused first, copied at the chain's
    /// first close request. Only ever added to, so a rank taken from it holds.
    order: Vec<String>,
    /// Oldest first, each with its window's place in `order` — none for a
    /// window never focused — and the window as it stood when it went.
    entries: Vec<(Instant, Option<usize>, WindowSession)>,
    /// Ranks taken at a close request, waiting for the window to go.
    pending_rank: HashMap<String, Option<usize>>,
}

impl Chain {
    /// All three go together: a copy of the order outliving its entries would
    /// be what the next quit ranks against.
    pub fn clear(&mut self) {
        self.order.clear();
        self.entries.clear();
        self.pending_rank.clear();
    }

    fn last_close(&self) -> Option<Instant> {
        self.entries.last().map(|(t, _, _)| *t)
    }

    /// Where `label` stands, given the focus order as it is now. A chain with
    /// nothing in it yet begins here, with a copy of that order. A window
    /// opened and focused since is added after the rest, as the most recent:
    /// missing, it would rank as never focused.
    fn rank(&mut self, label: &str, focus: &[String]) -> Option<usize> {
        if self.entries.is_empty() && self.pending_rank.is_empty() {
            self.order = focus.to_vec();
        }
        for l in focus {
            if !self.order.contains(l) {
                self.order.push(l.clone());
            }
        }
        self.order.iter().position(|l| l == label)
    }
}

/// What a save writes: the open windows, then the chain least-recently-focused
/// first, as the open ones are. Close order says nothing about that — a close
/// that does not raise its window can take the one behind first — so the
/// chain is sorted by rank: never focused first, and equal ranks in the order
/// they closed.
fn restorable(
    open: Vec<WindowSession>,
    chain: &[(Instant, Option<usize>, WindowSession)],
) -> Vec<WindowSession> {
    let mut closed: Vec<_> = chain.iter().collect();
    closed.sort_by_key(|(_, rank, _)| *rank);
    open.into_iter()
        .chain(closed.into_iter().map(|(_, _, w)| w.clone()))
        .collect()
}

/// Write out every window that has something open.
fn save<R: Runtime>(app: &AppHandle<R>, version: String, argv: Vec<String>, restart: bool) {
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
            // A window still being built has no frame to speak of yet — the
            // last one kept for it stands in, the one its restore recorded or
            // what a closed window is written with. Copied and let go before
            // the window is asked — see `Frame::of`.
            let old = state.frames.lock().unwrap().get(&label).cloned();
            let live = app
                .get_webview_window(&label)
                .and_then(|w| Frame::of(&w, old.as_ref()));
            if let Some(frame) = &live {
                state
                    .frames
                    .lock()
                    .unwrap()
                    .insert(label.clone(), frame.clone());
            }
            WindowSession {
                frame: live.or(old),
                open,
            }
        })
        .collect();

    // A restart comes back as the app stood; a window closed a moment before
    // the update was closed. Only an ordinary save carries the chain.
    let windows = if restart {
        open
    } else {
        restorable(open, &state.closed.lock().unwrap().entries)
    };

    // A snapshot is a record of its own, and its restart must come back as one.
    let restoring = !restart && !state.restoring.lock().unwrap().is_empty();

    let session = Session {
        version,
        argv,
        restart,
        restoring,
        windows,
    };
    config::write_json(&file(), &session);
}

/// Whether this launch offers the saved session on the empty screen instead of
/// putting it back. `ask` offers an ordinary session, never a restart — the
/// reader pressed Update, not Quit. A session found mid-restore is offered
/// whatever the setting, bar `off`: the launch that was putting it back went
/// down doing so, and doing it again unasked could go down the same way.
pub fn offers(reopen: &str, saved: Option<&Session>) -> bool {
    match saved {
        Some(s) if s.restoring => reopen != "off",
        Some(s) => reopen == "ask" && !s.restart,
        None => reopen == "ask",
    }
}

/// Mark the file as a restore under way instead of consuming it up front, so a
/// launch that goes down mid-restore leaves the session for the next one to
/// offer. `save` keeps the mark while any restored window is still loading,
/// and `restored` consumes the file once the last has shown its document.
pub fn begin_restore(session: &Session) {
    mark_at(&file(), session);
}

/// Written as an ordinary session: found again, it is no longer the app coming
/// back mid-read, and its argv would echo into the wrong launch.
fn mark_at(path: &Path, session: &Session) {
    let marked = Session {
        version: session.version.clone(),
        argv: vec![],
        restart: false,
        restoring: true,
        windows: session.windows.clone(),
    };
    config::write_json(path, &marked);
}

/// A restored window has shown its document, or has gone. Once the last of
/// them has, the restore is over and the file is consumed, as it used to be at
/// launch; the save that follows writes what is open now, unmarked.
///
/// Not while an update installs: the file is then the restart's snapshot, and
/// no ordinary save would follow to put anything back.
pub fn restored<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let state = app.state::<AppState>();
    let mut restoring = state.restoring.lock().unwrap();
    if restoring.remove(label) && restoring.is_empty() {
        drop(restoring);
        if !state.installing.load(Ordering::SeqCst) {
            discard();
        }
    }
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
            restoring: false,
            windows: vec![WindowSession {
                open: OpenTabs {
                    tabs: vec![json!({ "path": r"C:\notes\a.md", "entries": [], "index": 0 })],
                    active: 0,
                    sidebar: true,
                },
                frame: Some(Frame {
                    x: 10,
                    y: 20,
                    width: 800,
                    height: 600,
                    maximized: false,
                    normal: false,
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
                sidebar: false,
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

    /// Open windows first, then the chain by where each window stood in the
    /// focus order when the chain began — whichever of them closed first.
    #[test]
    fn the_chain_is_written_behind_first() {
        let window = |label: &str| WindowSession {
            open: OpenTabs {
                tabs: vec![json!({ "path": format!("{label}.md") })],
                active: 0,
                sidebar: false,
            },
            frame: None,
        };
        // Each close as `window_closed` ranks it, with the focus order as it
        // stands then: a window that has gone is out of it.
        let close = |chain: &mut Chain, label: &str, focus: &[&str]| {
            let focus: Vec<String> = focus.iter().map(|l| l.to_string()).collect();
            let rank = chain.rank(label, &focus);
            chain.entries.push((Instant::now(), rank, window(label)));
        };
        let written = |chain: &Chain| restorable(vec![], &chain.entries);

        // B was in front and closed first, then A.
        let mut front_first = Chain::default();
        close(&mut front_first, "b", &["a", "b"]);
        close(&mut front_first, "a", &["a"]);
        assert_eq!(written(&front_first), vec![window("a"), window("b")]);

        // A, behind, closed first, then B: still B in front.
        let mut back_first = Chain::default();
        close(&mut back_first, "a", &["a", "b"]);
        close(&mut back_first, "b", &["b"]);
        assert_eq!(written(&back_first), vec![window("a"), window("b")]);

        // C never had focus: behind every window that did.
        let mut never = Chain::default();
        close(&mut never, "b", &["a", "b"]);
        close(&mut never, "a", &["a"]);
        close(&mut never, "c", &[]);
        assert_eq!(written(&never), vec![window("c"), window("a"), window("b")]);

        // D opened and took focus after the chain began: the most recent.
        let mut late = Chain::default();
        close(&mut late, "a", &["a", "b"]);
        close(&mut late, "d", &["b", "d"]);
        close(&mut late, "b", &["b"]);
        assert_eq!(written(&late), vec![window("a"), window("b"), window("d")]);

        let mid = restorable(vec![window("c")], &front_first.entries[..1]);
        assert_eq!(mid, vec![window("c"), window("b")]);
    }

    /// A request made while another window is still on its way out ranks
    /// against the same copy, though focus has moved since: the chain has
    /// begun as soon as anything is pending.
    #[test]
    fn a_pending_close_keeps_the_chain_begun() {
        let focus = |labels: &[&str]| labels.iter().map(|l| l.to_string()).collect::<Vec<_>>();
        let mut chain = Chain::default();
        let a = chain.rank("a", &focus(&["a", "b"]));
        chain.pending_rank.insert("a".into(), a);
        // B was clicked, and so raised, before A went.
        assert_eq!(chain.rank("b", &focus(&["b", "a"])), Some(1));
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

    /// Files written before the mark existed were never mid-restore.
    #[test]
    fn a_file_without_restoring_is_not_mid_restore() {
        let s: Session =
            serde_json::from_str(r#"{"version":"1.6.6","argv":[],"restart":false,"windows":[]}"#)
                .unwrap();
        assert!(!s.restoring);
    }

    /// The file a restore leaves while it runs: the same windows, marked, and
    /// no longer a restart — found again, it is a launch that went down, not the
    /// app coming back mid-read, and its argv would echo into the wrong launch.
    #[test]
    fn a_restore_under_way_is_marked_on_disk() {
        let path = temp_path("marked");
        mark_at(&path, &sample());
        let marked = read_from(&path, "1.4.6").unwrap();
        assert!(marked.restoring);
        assert!(!marked.restart);
        assert!(marked.argv.is_empty());
        assert_eq!(marked.windows, sample().windows);
        discard_at(&path);
    }

    /// Offered or put back. A session found mid-restore is offered whatever
    /// the setting — bar off, which keeps nothing — because putting it back
    /// unasked could go down the same way the last launch did.
    #[test]
    fn a_session_found_mid_restore_is_offered() {
        let ordinary = || Session {
            argv: vec![],
            restart: false,
            ..sample()
        };
        let died = Session {
            restoring: true,
            ..ordinary()
        };
        let restart = sample();

        assert!(offers("ask", Some(&ordinary())));
        assert!(!offers("restore", Some(&ordinary())));
        assert!(!offers("ask", Some(&restart)));
        assert!(!offers("restore", Some(&restart)));
        assert!(offers("restore", Some(&died)));
        assert!(offers("ask", Some(&died)));
        assert!(!offers("off", Some(&died)));
        assert!(offers("ask", None));
        assert!(!offers("restore", None));
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

    /// 1.7.3 wrote a maximized window's maximized rectangle, with no word on
    /// it: read back, it is not a normal size to un-maximize to.
    #[test]
    fn an_old_maximized_frame_is_not_normal() {
        let s: Session = serde_json::from_str(
            r#"{"version":"1.7.3","argv":[],"restart":false,"windows":[{"tabs":[],"active":0,
                "frame":{"x":-8,"y":-8,"width":1936,"height":1048,"maximized":true}}]}"#,
        )
        .unwrap();
        let frame = s.windows[0].frame.as_ref().unwrap();
        assert!(frame.maximized);
        assert!(!frame.normal);
    }

    /// Two side-by-side 1000×800 monitors.
    const LEFT: Rect = (0, 0, 1000, 800);
    const RIGHT: Rect = (1000, 0, 1000, 800);

    fn frame(x: i32, y: i32, width: u32, height: u32, maximized: bool, normal: bool) -> Frame {
        Frame {
            x,
            y,
            width,
            height,
            maximized,
            normal,
        }
    }

    /// A normal window at 100,100 600×400, on the left monitor.
    fn normal() -> Frame {
        frame(100, 100, 600, 400, false, false)
    }

    /// The window as it stands now: shown, normal, at the given rectangle,
    /// on the left monitor.
    fn now(x: i32, y: i32, width: u32, height: u32) -> Sample {
        Sample {
            minimized: false,
            fullscreen: false,
            visible: true,
            maximized: false,
            position: PhysicalPosition::new(x, y),
            size: PhysicalSize::new(width, height),
            monitors: Some((LEFT, vec![LEFT, RIGHT])),
        }
    }

    /// Maximized on the left monitor.
    fn maximized_left() -> Sample {
        Sample {
            maximized: true,
            ..now(0, 0, 1000, 800)
        }
    }

    #[test]
    fn maximizing_keeps_the_normal_rectangle() {
        assert_eq!(
            sampled(Some(&normal()), maximized_left()),
            Some(frame(100, 100, 600, 400, true, true))
        );
        // And the next sample, still maximized, keeps it again.
        let kept = frame(100, 100, 600, 400, true, true);
        assert_eq!(sampled(Some(&kept), maximized_left()), Some(kept.clone()));
    }

    /// With nothing kept before, or only a 1.7.3 maximized rectangle, there
    /// is no normal size to keep: the maximized one is written, as before.
    #[test]
    fn maximized_with_no_normal_rectangle_writes_the_maximized_one() {
        let maximized = frame(0, 0, 1000, 800, true, false);
        assert_eq!(sampled(None, maximized_left()), Some(maximized.clone()));
        let old = frame(-8, -8, 1016, 816, true, false);
        assert_eq!(sampled(Some(&old), maximized_left()), Some(maximized));
    }

    /// Moved maximized to the other monitor, the window must come back there:
    /// a normal rectangle on the first would take it back with it.
    #[test]
    fn maximized_on_another_monitor_drops_the_normal_rectangle() {
        let on_right = Sample {
            maximized: true,
            monitors: Some((RIGHT, vec![LEFT, RIGHT])),
            ..now(1000, 0, 1000, 800)
        };
        assert_eq!(
            sampled(Some(&normal()), on_right),
            Some(frame(1000, 0, 1000, 800, true, false))
        );
    }

    /// Straddling both monitors, mostly on the one it is maximized on: kept.
    #[test]
    fn a_straddling_rectangle_counts_where_most_of_it_is() {
        let old = frame(500, 100, 800, 400, false, false); // 500 px left, 300 right
        assert_eq!(
            sampled(Some(&old), maximized_left()),
            Some(frame(500, 100, 800, 400, true, true))
        );
    }

    /// No monitors to go by — Wayland, or a read that failed: kept.
    #[test]
    fn maximized_with_no_monitors_keeps_the_normal_rectangle() {
        let unknown = Sample {
            monitors: None,
            ..maximized_left()
        };
        assert_eq!(
            sampled(Some(&normal()), unknown),
            Some(frame(100, 100, 600, 400, true, true))
        );
    }

    /// A normal rectangle on a monitor that is gone is on none of these.
    #[test]
    fn a_rectangle_on_no_monitor_is_not_kept() {
        let gone = frame(5000, 100, 600, 400, false, false);
        assert_eq!(
            sampled(Some(&gone), maximized_left()),
            Some(frame(0, 0, 1000, 800, true, false))
        );
    }

    /// Minimized, in full screen or not shown yet, the window's own rectangle
    /// is not one to keep: the last frame stands, or none if there was none.
    #[test]
    fn a_window_not_showing_its_rectangle_keeps_the_old_frame() {
        let restored = frame(100, 100, 600, 400, true, true);
        let minimized = Sample {
            minimized: true,
            ..now(-32000, -32000, 160, 28)
        };
        let fullscreen = Sample {
            fullscreen: true,
            ..now(0, 0, 1000, 800)
        };
        let hidden = || Sample {
            visible: false,
            ..now(300, 300, 1100, 860)
        };
        assert_eq!(sampled(Some(&normal()), minimized), Some(normal()));
        assert_eq!(sampled(Some(&normal()), fullscreen), Some(normal()));
        assert_eq!(sampled(Some(&restored), hidden()), Some(restored));
        assert_eq!(sampled(None, hidden()), None);
    }

    #[test]
    fn un_maximized_takes_the_new_rectangle() {
        let kept = frame(100, 100, 600, 400, true, true);
        assert_eq!(
            sampled(Some(&kept), now(200, 150, 700, 500)),
            Some(frame(200, 150, 700, 500, false, false))
        );
    }

    /// Files written before the sidebar was saved bring every window back with
    /// it closed, as those windows always came back.
    #[test]
    fn a_window_without_sidebar_comes_back_closed() {
        let s: Session = serde_json::from_str(
            r#"{"version":"1.6.6","argv":[],"windows":[{"tabs":[],"active":0,"frame":null}]}"#,
        )
        .unwrap();
        assert!(!s.windows[0].open.sidebar);
    }

    /// An open sidebar survives the trip to disk, and the payload a restored
    /// window is created with counts it until the window reports.
    #[test]
    fn an_open_sidebar_round_trips() {
        let path = temp_path("sidebar");
        config::write_json(&path, &sample());
        assert!(read_from(&path, "1.4.6").unwrap().windows[0].open.sidebar);
        discard_at(&path);

        let pending = OpenTabs::from_pending(
            &json!({ "kind": "session", "tabs": [{ "path": "c.md" }], "active": 0, "sidebar": true }),
        )
        .unwrap();
        assert!(pending.sidebar);
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
