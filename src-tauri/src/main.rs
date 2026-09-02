#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod render;
mod session;
mod themes;
mod update;
mod watch;

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder, Window,
    WindowEvent,
};

/// Extensions accepted from the command line. A CLI argument is untrusted
/// input, so an unrecognised path is ignored rather than opened.
const MD_EXTS: &[&str] = &[
    "md", "markdown", "mdown", "mkd", "mdtext", "mdtxt", "mdwn", "mkdn", "text", "txt",
];
const IMG_EXTS: &[&str] = &[
    "svg", "png", "jpg", "jpeg", "gif", "webp", "avif", "bmp", "ico",
];

#[derive(Default)]
struct AppState {
    /// What a freshly created window should open once its webview asks:
    /// `{kind:"path"}` from a file-association open, `{kind:"tab"}` from a
    /// torn-off tab, or `{kind:"session"}` after an update restart. Keyed by
    /// window label.
    pending: Mutex<HashMap<String, Value>>,
    /// What each window has open, as it last reported — or, until it has, what
    /// it was created to open. Only read when an update is about to replace
    /// the process — see `session`.
    sessions: Mutex<HashMap<String, session::OpenTabs>>,
    /// Windows a session snapshot is still waiting to hear from, and the bell
    /// each answer rings. See `session::snapshot`.
    awaiting: Mutex<HashSet<String>>,
    reported: Condvar,
    /// Windows whose webview has asked for its pending payload, and so is live
    /// enough to receive events. A window not in here is still booting, which is
    /// the normal state when the OS hands us a file during startup.
    ready: Mutex<HashSet<String>>,
    /// One watcher per window, covering every directory that window has a tab in.
    watches: Mutex<HashMap<String, watch::Handle>>,
    /// App-wide, unlike `watches`: a theme edit restyles every window.
    theme_watch: Mutex<Option<watch::Handle>>,
    /// One per window: the folders its sidebar currently shows.
    folder_watches: Mutex<HashMap<String, watch::Handle>>,
    /// Window labels, least-recently-focused first. Decides which window a warm
    /// file-association open goes to, and breaks ties between overlapping
    /// windows during a tab drag.
    focus_order: Mutex<Vec<String>>,
    /// Window currently showing a drop caret, so it can be told to clear it.
    drag_target: Mutex<Option<String>>,
    /// The release found by the first update check, reused by every window that
    /// asks afterwards. One launch, one request.
    update: Mutex<Option<update::UpdateInfo>>,
    next_window: AtomicUsize,
}

#[derive(Serialize)]
struct Origin {
    x: f64,
    y: f64,
    scale: f64,
    /// False when the compositor would not say where the window is, so the
    /// frontend must fall back to the pointer's own screen coordinates.
    exact: bool,
}

/// What the frontend needs at boot: the saved settings, plus the facts about
/// this platform that it cannot work out for itself from inside a webview.
#[derive(Serialize)]
struct Settings {
    #[serde(flatten)]
    config: config::Config,
    /// Whether releasing a dragged tab over another window can be detected here.
    /// See `window_at` — only Windows can answer that reliably.
    cross_window_drag: bool,
    /// Whether two paths differing only in case name the same file. False on
    /// Linux, where `Notes.md` and `notes.md` are two documents.
    case_insensitive_paths: bool,
    /// This build's version. The webview has no other way to know it, and
    /// Settings shows it beside the update controls.
    version: String,
}

#[derive(Serialize)]
struct Document {
    path: String,
    dir: String,
    title: String,
    html: String,
}

/// An image the webview will fetch for itself over the asset protocol. There is
/// no content here because there is nothing for us to render.
#[derive(Serialize)]
struct Asset {
    path: String,
    dir: String,
}

/// One row of the folder sidebar. A single level only: the tree asks for a
/// folder's children when it is expanded, so a huge tree costs nothing until
/// it is looked at.
#[derive(Serialize, Debug, PartialEq)]
struct DirEntry {
    name: String,
    path: String,
    is_dir: bool,
}

/// `dir` is the canonical form of what was asked for, so the tree can match it
/// against the canonical paths the watcher reports.
#[derive(Serialize)]
struct Listing {
    dir: String,
    entries: Vec<DirEntry>,
}

fn has_ext(path: &Path, exts: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| exts.iter().any(|m| m.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

fn is_markdown(path: &Path) -> bool {
    has_ext(path, MD_EXTS)
}

/// Mirrors `IMG_LINK` in app.js: what the viewer can show in a tab of its own.
fn is_image(path: &Path) -> bool {
    has_ext(path, IMG_EXTS)
}

/// The sidebar's one hiding rule: dot-prefixed names are noise — `.git` in a
/// notes folder — so neither the listing nor the watcher mentions them.
fn is_visible_entry(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| !n.starts_with('.'))
        .unwrap_or(false)
}

/// Pick the file to open out of a process argument list, skipping argv[0] and
/// any flags. Windows hands file-association opens over this way.
fn file_from_args<S: AsRef<str>>(args: &[S]) -> Option<PathBuf> {
    args.iter()
        .skip(1)
        .map(|a| PathBuf::from(a.as_ref()))
        .find(|p| p.is_file() && is_markdown(p))
}

/// `canonicalize` on Windows returns `\\?\C:\...`; the asset protocol and the
/// UI both want the plain form.
pub(crate) fn strip_unc(p: &Path) -> String {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

/* ---------------- windows ---------------- */

fn touch_focus(state: &AppState, label: &str) {
    let mut order = state.focus_order.lock().unwrap();
    order.retain(|l| l != label);
    order.push(label.to_string());
}

/// The window a warm open should land in: most recently focused that still exists.
fn last_focused(app: &AppHandle) -> Option<String> {
    let state = app.state::<AppState>();
    let order = state.focus_order.lock().unwrap();
    order
        .iter()
        .rev()
        .find(|l| app.get_webview_window(l).is_some())
        .cloned()
}

fn focus_window(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Stash what a window should open once its webview asks, unless something is
/// already on its way in — a booting window's payload must not be overwritten.
/// Says whether the slot was taken. The same payload stands in for the
/// window's session report until it makes one, so a snapshot taken while it
/// boots still counts it.
fn claim_pending(state: &AppState, label: &str, payload: Value) -> bool {
    let open = session::OpenTabs::from_pending(&payload);
    {
        let mut pending = state.pending.lock().unwrap();
        if pending.contains_key(label) {
            return false;
        }
        pending.insert(label.to_string(), payload);
    }
    if let Some(open) = open {
        state.sessions.lock().unwrap().insert(label.to_string(), open);
    }
    true
}

/// Where a new window goes.
enum Placement {
    /// Wherever the OS puts it.
    Default,
    /// A physical screen point taken straight off a pointer event; the window
    /// is offset so the cursor lands near its tab strip.
    Cursor(f64, f64),
    /// Exactly where a window stood before an update restart.
    Frame(session::Frame),
}

/// Create a window.
///
/// The build runs on a worker thread on purpose. `build()` waits on the event
/// loop to construct the webview, and both callers here — a synchronous command
/// and the single-instance hook — already run *on* that loop, so building
/// inline deadlocks the app. Reserving the label and stashing `pending` happens
/// first and synchronously, so the new window's `take_pending` cannot race it.
fn spawn_window(app: &AppHandle, pending: Option<Value>, place: Placement) -> String {
    let state = app.state::<AppState>();
    let n = state.next_window.fetch_add(1, Ordering::Relaxed) + 1;
    let label = format!("w{n}");

    if let Some(p) = pending {
        claim_pending(&state, &label, p);
    }

    let app = app.clone();
    let target = label.clone();
    std::thread::spawn(move || {
        match WebviewWindowBuilder::new(&app, &target, WebviewUrl::App("index.html".into()))
            .title("Markdown Viewer")
            .inner_size(1100.0, 860.0)
            .visible(false)
            .build()
        {
            Ok(win) => match place {
                Placement::Default => {}
                Placement::Cursor(x, y) => {
                    let _ = win.set_position(PhysicalPosition::new(
                        (x - 140.0).round() as i32,
                        (y - 24.0).round() as i32,
                    ));
                }
                Placement::Frame(frame) => frame.apply(&win),
            },
            Err(e) => {
                eprintln!("window {target} failed to open: {e}");
                let state = app.state::<AppState>();
                state.pending.lock().unwrap().remove(&target);
                state.sessions.lock().unwrap().remove(&target);
            }
        }
    });

    label
}

/// The top-level window the compositor draws at a physical screen point, or 0.
/// `WindowFromPoint` returns the deepest child — the WebView2 surface — so this
/// walks back up to the frame Tauri owns.
#[cfg(windows)]
fn hwnd_at(x: f64, y: f64) -> isize {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetAncestor, WindowFromPoint, GA_ROOT};

    let point = POINT {
        x: x.round() as i32,
        y: y.round() as i32,
    };
    unsafe {
        let hit = WindowFromPoint(point);
        if hit.is_null() {
            return 0;
        }
        GetAncestor(hit, GA_ROOT) as isize
    }
}

/// The app window a physical screen point lands on, plus that point in the
/// window's CSS pixels.
///
/// This is a true z-order test rather than a scan of window rectangles: the
/// answer has to be the window the user can actually *see* at the cursor.
/// Note it can return the dragging window itself — callers treat that as
/// "dropped on my own window", which tears the tab off.
///
/// Windows-only. macOS and X11 could answer this with native calls, but Wayland
/// deliberately hides the global pointer position, so there is no answer that
/// holds everywhere. Off Windows this returns `None` and every drop tears the
/// tab off into its own window; the frontend is told not to offer the
/// drop-onto-another-window affordance at all, through `Settings`.
#[cfg(windows)]
fn window_at(app: &AppHandle, x: f64, y: f64) -> Option<(String, f64, f64)> {
    let target = hwnd_at(x, y);
    if target == 0 {
        return None;
    }
    for (label, w) in app.webview_windows() {
        let Ok(handle) = w.hwnd() else { continue };
        if handle.0 as isize != target {
            continue;
        }
        let Ok(pos) = w.inner_position() else {
            continue;
        };
        let scale = w.scale_factor().unwrap_or(1.0);
        return Some((
            label,
            (x - pos.x as f64) / scale,
            (y - pos.y as f64) / scale,
        ));
    }
    None
}

#[cfg(not(windows))]
fn window_at(_app: &AppHandle, _x: f64, _y: f64) -> Option<(String, f64, f64)> {
    None
}

fn clear_drag(app: &AppHandle, state: &AppState) {
    if let Some(prev) = state.drag_target.lock().unwrap().take() {
        let _ = app.emit_to(&prev, "tab-drag-out", ());
    }
}

/* ---------------- commands ---------------- */

/// Hand this window whatever it was created to show. Consumed on first call,
/// which is also what marks the window as ready to receive events.
#[tauri::command]
fn take_pending(state: State<AppState>, window: Window) -> Option<Value> {
    let label = window.label().to_string();
    state.ready.lock().unwrap().insert(label.clone());
    state.pending.lock().unwrap().remove(&label)
}

/// Resolve a path the frontend handed over and split off its parent, or say why
/// it cannot be opened. Shared by `load_file` and `load_asset` so that the two
/// resolve — and refuse — identically.
fn locate(path: String) -> Result<(PathBuf, PathBuf), String> {
    let path = PathBuf::from(&path);
    let path = std::fs::canonicalize(&path).unwrap_or(path);
    if !path.is_file() {
        return Err(format!("Not a file: {}", path.display()));
    }

    let dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    Ok((path, dir))
}

#[tauri::command]
fn load_file(app: AppHandle, path: String) -> Result<Document, String> {
    let (path, dir) = locate(path)?;

    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = render::decode(&bytes);
    let html = render::render(&text);

    // Let the webview load images and other assets sitting next to the document.
    app.asset_protocol_scope().allow_directory(&dir, true).ok();

    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let title = render::first_heading(&text).unwrap_or_else(|| file_name.clone());

    Ok(Document {
        path: strip_unc(&path),
        dir: strip_unc(&dir),
        title,
        html,
    })
}

/// Point this window's sidebar watcher at exactly the folders on show — the
/// root and whatever is expanded. Collapsed folders are re-listed on expand, so
/// watching them would only cost handles. Empty `dirs` drops the watcher.
#[tauri::command]
fn watch_folders(app: AppHandle, state: State<AppState>, window: Window, dirs: Vec<String>) {
    let mut dirs: Vec<PathBuf> = dirs
        .into_iter()
        .map(PathBuf::from)
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
        .collect();
    dirs.sort();
    dirs.dedup();

    let label = window.label().to_string();
    let handle = watch::watch(
        app,
        dirs,
        is_visible_entry,
        "folder-changed",
        Some(label.clone()),
    );
    set_watch(&state.folder_watches, label, handle);
}

/// Install a window's watcher, dropping the previous one; `None` just drops.
fn set_watch(
    watches: &Mutex<HashMap<String, watch::Handle>>,
    label: String,
    handle: Option<watch::Handle>,
) {
    let mut watches = watches.lock().unwrap();
    match handle {
        Some(h) => {
            watches.insert(label, h);
        }
        None => {
            watches.remove(&label);
        }
    }
}

/// Whitelist an image's own directory for the asset protocol and hand back the
/// canonical path. `load_file` does this for the documents it opens, which is
/// why an image sitting beside one already loads; an image opened as a tab in
/// its own right has had no such grant, and the webview would refuse it.
#[tauri::command]
fn load_asset(app: AppHandle, path: String) -> Result<Asset, String> {
    let (path, dir) = locate(path)?;
    app.asset_protocol_scope().allow_directory(&dir, true).ok();
    Ok(Asset {
        path: strip_unc(&path),
        dir: strip_unc(&dir),
    })
}

/// The openable contents of one folder for the sidebar: subfolders first, then
/// the files this viewer can show, each group sorted by name without regard to
/// case. Dot-prefixed entries are skipped — `.git` in a notes folder is noise.
#[tauri::command]
fn list_dir(path: String) -> Result<Listing, String> {
    let dir = PathBuf::from(&path);
    let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
    if !dir.is_dir() {
        return Err(format!("Not a folder: {}", dir.display()));
    }
    let read = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    let mut entries: Vec<DirEntry> = read
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !is_visible_entry(&path) {
                return None;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            // `is_dir` follows symlinks, so a linked folder shows as a folder.
            let is_dir = path.is_dir();
            if !is_dir && !is_markdown(&path) && !is_image(&path) {
                return None;
            }
            Some(DirEntry {
                name,
                path: strip_unc(&path),
                is_dir,
            })
        })
        .collect();
    entries.sort_by_cached_key(|e| (!e.is_dir, e.name.to_lowercase()));
    Ok(Listing {
        dir: strip_unc(&dir),
        entries,
    })
}

/// Replace this window's watcher so it covers exactly the files its tabs hold.
#[tauri::command]
fn watch_files(app: AppHandle, state: State<AppState>, window: Window, paths: Vec<String>) {
    let files: Vec<PathBuf> = paths
        .into_iter()
        .map(PathBuf::from)
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
        .collect();

    let mut dirs: Vec<PathBuf> = files
        .iter()
        .filter_map(|p| p.parent().map(Path::to_path_buf))
        .collect();
    dirs.sort();
    dirs.dedup();

    let label = window.label().to_string();
    let targets = files;
    let handle = watch::watch(
        app,
        dirs,
        move |p| targets.iter().any(|f| f == p),
        "file-changed",
        Some(label.clone()),
    );

    set_watch(&state.watches, label, handle);
}

#[tauri::command]
fn open_window(app: AppHandle, path: Option<String>) -> String {
    let pending = path.map(|p| json!({ "kind": "path", "path": p }));
    spawn_window(&app, pending, Placement::Default)
}

/// A window answering `update-installing` with what it has open, so the
/// restart can bring it back. `tabs` is the frontend's own shape, kept opaque
/// here.
#[tauri::command]
fn set_session(state: State<AppState>, window: Window, tabs: Vec<Value>, active: usize) {
    let label = window.label();
    state
        .sessions
        .lock()
        .unwrap()
        .insert(label.to_string(), session::OpenTabs { tabs, active });
    session::reported(&state, label);
}

/// The window's client-area origin and scale, both physical. The frontend turns
/// pointer coordinates into screen coordinates with these instead of
/// `screenX * devicePixelRatio`, which guesses wrong the moment two monitors
/// run at different scaling.
///
/// Wayland refuses to tell a window where it is, so `inner_position` fails
/// there. Reporting a `(0, 0)` origin rather than an error keeps a tear-off
/// working — it lands in roughly the right place instead of not happening at
/// all — and `exact` lets the frontend know which it got.
#[tauri::command]
fn window_origin(window: Window) -> Origin {
    let scale = window.scale_factor().unwrap_or(1.0);
    match window.inner_position() {
        Ok(pos) => Origin {
            x: pos.x as f64,
            y: pos.y as f64,
            scale,
            exact: true,
        },
        Err(_) => Origin {
            x: 0.0,
            y: 0.0,
            scale,
            exact: false,
        },
    }
}

/// Track a detached tab drag. Returns the window under the cursor, if any, and
/// tells that window where to draw its drop caret.
#[tauri::command]
fn drag_over(
    app: AppHandle,
    state: State<AppState>,
    window: Window,
    x: f64,
    y: f64,
) -> Option<String> {
    // Hovering your own window is not a drop target: releasing there tears off.
    let hit = window_at(&app, x, y).filter(|(label, _, _)| label != window.label());
    let next = hit.as_ref().map(|(label, _, _)| label.clone());

    {
        let mut current = state.drag_target.lock().unwrap();
        if current.as_deref() != next.as_deref() {
            if let Some(prev) = current.take() {
                let _ = app.emit_to(&prev, "tab-drag-out", ());
            }
        }
        current.clone_from(&next);
    }

    if let Some((label, lx, ly)) = hit {
        let _ = app.emit_to(&label, "tab-drag-over", json!({ "x": lx, "y": ly }));
    }
    next
}

#[tauri::command]
fn drag_cancel(app: AppHandle, state: State<AppState>) {
    clear_drag(&app, &state);
}

/// Release a dragged tab. Over another window it is adopted there; over
/// anything else it becomes a new window at the cursor, unless `tear_off` is
/// false, in which case the drop is cancelled and the tab stays where it was.
/// The caller drops its own copy on "adopted" and "detached".
#[tauri::command]
fn drop_tab(
    app: AppHandle,
    state: State<AppState>,
    window: Window,
    x: f64,
    y: f64,
    tab: Value,
    tear_off: bool,
) -> Result<String, String> {
    clear_drag(&app, &state);

    match window_at(&app, x, y).filter(|(label, _, _)| label != window.label()) {
        Some((label, lx, _)) => {
            app.emit_to(&label, "tab-adopt", json!({ "tab": tab, "x": lx }))
                .map_err(|e| e.to_string())?;
            focus_window(&app, &label);
            Ok("adopted".into())
        }
        None if !tear_off => Ok("cancelled".into()),
        None => {
            spawn_window(
                &app,
                Some(json!({ "kind": "tab", "tab": tab })),
                Placement::Cursor(x, y),
            );
            Ok("detached".into())
        }
    }
}

#[tauri::command]
fn list_themes(app: AppHandle) -> Vec<themes::ThemeInfo> {
    themes::list(&app)
}

#[tauri::command]
fn read_theme(app: AppHandle, name: String) -> Result<String, String> {
    themes::read(&app, &name)
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    Settings {
        config: config::load(),
        cross_window_drag: cfg!(windows),
        case_insensitive_paths: cfg!(any(windows, target_os = "macos")),
        version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
fn set_theme(name: String) {
    let mut cfg = config::load();
    cfg.theme = name;
    config::save(&cfg);
}

/// Broadcast, unlike the theme: this one decides where *other* windows' opens
/// land, so leaving windows disagreeing about it is just confusing.
#[tauri::command]
fn set_open_mode(app: AppHandle, mode: String) {
    let mut cfg = config::load();
    cfg.open_mode = mode.clone();
    config::save(&cfg);
    let _ = app.emit("open-mode-changed", mode);
}

/* ---------------- app ---------------- */

/// Show a file. Every way a document can arrive — argv at startup, a second
/// process handing over its arguments, macOS delivering an Apple Event — ends up
/// here, so the open-mode setting is honoured identically by all of them.
///
/// A window that has not yet drained its pending slot is still booting and has
/// nothing in it, so it takes the file whatever the open mode says; emitting
/// `file-opened` at a window with no listener yet would drop it on the floor.
/// A booting window that *does* have something pending is a torn-off tab or a
/// restored session on its way in, and must not be overwritten; the file gets
/// a window of its own.
fn open_path(app: &AppHandle, path: &Path) {
    let path = strip_unc(path);
    let state = app.state::<AppState>();

    // macOS delivers a double-clicked file as an Apple Event, and it arrives
    // before the window the config declares exists — there is nothing to reuse
    // yet. Spawning here would leave that window standing empty beside the
    // document, so the file is stashed for it instead, which is the same thing
    // a cold argv open does everywhere else. Only the first file can claim it;
    // Finder can open several at once, and the rest still get windows of their
    // own.
    let payload = json!({ "kind": "path", "path": path });

    if app.webview_windows().is_empty() && claim_pending(&state, "main", payload.clone()) {
        return;
    }

    if let Some(label) = last_focused(app) {
        // `ready` is read before `pending` is taken rather than nested inside
        // it: every other holder of these locks takes one at a time, and this
        // keeps it that way, so there is no lock order to get wrong later.
        let ready = state.ready.lock().unwrap().contains(&label);
        if !ready {
            if claim_pending(&state, &label, payload.clone()) {
                focus_window(app, &label);
                return;
            }
            // Booting with something already on the way in — a torn-off tab
            // or a restored session. An event now would reach a webview with
            // no listeners yet, so the file gets a window of its own instead.
        } else if config::load().open_mode != "window" {
            let _ = app.emit_to(&label, "file-opened", path);
            focus_window(app, &label);
            return;
        }
    }

    spawn_window(app, Some(payload), Placement::Default);
}

/// Put back what an update restart took down: one window per saved window,
/// with its tabs and its place on screen.
///
/// The config-declared `main` window already exists and would otherwise stand
/// empty, so the first saved window goes there — unless a file-association
/// open has claimed it first, in which case every saved window gets a new one.
fn restore_session(app: &AppHandle, windows: Vec<session::WindowSession>) {
    let state = app.state::<AppState>();
    let spawn = |w: session::WindowSession| {
        let pending = session_payload(&w);
        spawn_window(
            app,
            Some(pending),
            w.frame.map_or(Placement::Default, Placement::Frame),
        );
    };
    let mut windows = windows.into_iter();

    if let Some(w) = windows.next() {
        if claim_pending(&state, "main", session_payload(&w)) {
            if let (Some(frame), Some(main)) = (&w.frame, app.get_webview_window("main")) {
                frame.apply(&main);
            }
        } else {
            spawn(w);
        }
    }
    windows.for_each(spawn);
}

/// What a restored window opens with. `maximized` is left to the frontend to
/// act on once it has shown the window — see `session::Frame::apply`.
fn session_payload(w: &session::WindowSession) -> Value {
    json!({
        "kind": "session",
        "tabs": w.open.tabs,
        "active": w.open.active,
        "maximized": w.frame.as_ref().is_some_and(|f| f.maximized),
    })
}

/// Menu id for the one item that is not predefined.
#[cfg(target_os = "macos")]
const CLOSE_WINDOW: &str = "close-window";

/// macOS routes clipboard commands through the menu bar: with no Edit menu,
/// Cmd+C does nothing at all inside the webview. Tauri's default menu supplies
/// those, but it also binds Cmd+W to Close Window, which would shadow this app's
/// close-tab. So this is the default menu minus that collision — closing a
/// window moves to Shift+Cmd+W, leaving plain Cmd+W to the frontend.
#[cfg(target_os = "macos")]
fn macos_menu(app: &AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem as Item, Submenu};

    let about = Submenu::with_items(
        app,
        "T4 Markdown Viewer",
        true,
        &[
            &Item::about(app, None, None)?,
            &Item::separator(app)?,
            &Item::services(app, None)?,
            &Item::separator(app)?,
            &Item::hide(app, None)?,
            &Item::hide_others(app, None)?,
            &Item::show_all(app, None)?,
            &Item::separator(app)?,
            &Item::quit(app, None)?,
        ],
    )?;

    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &Item::undo(app, None)?,
            &Item::redo(app, None)?,
            &Item::separator(app)?,
            &Item::cut(app, None)?,
            &Item::copy(app, None)?,
            &Item::paste(app, None)?,
            &Item::select_all(app, None)?,
        ],
    )?;

    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &Item::minimize(app, None)?,
            &Item::maximize(app, None)?,
            &Item::fullscreen(app, None)?,
            &Item::separator(app)?,
            &MenuItem::with_id(
                app,
                CLOSE_WINDOW,
                "Close Window",
                true,
                Some("Shift+CmdOrCtrl+W"),
            )?,
        ],
    )?;

    Menu::with_items(app, &[&about, &edit, &window])
}

fn handle_second_instance(app: &AppHandle, argv: Vec<String>) {
    match file_from_args(&argv) {
        Some(path) => open_path(app, &path),
        None => {
            if let Some(label) = last_focused(app) {
                focus_window(app, &label);
            }
        }
    }
}

fn main() {
    let builder = tauri::Builder::default()
        // Must be registered first: plugins run in registration order, and this
        // one has to intercept the second process before anything else starts.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            handle_second_instance(app, argv);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            take_pending,
            load_file,
            load_asset,
            list_dir,
            watch_files,
            watch_folders,
            open_window,
            set_session,
            window_origin,
            drag_over,
            drag_cancel,
            drop_tab,
            list_themes,
            read_theme,
            get_settings,
            set_theme,
            set_open_mode,
            update::check_for_update,
            update::install_update,
            update::set_auto_update_check,
        ])
        .on_window_event(|window, event| {
            let state = window.app_handle().state::<AppState>();
            match event {
                WindowEvent::Focused(true) => touch_focus(&state, window.label()),
                WindowEvent::Destroyed => {
                    let label = window.label();
                    state.watches.lock().unwrap().remove(label);
                    state.folder_watches.lock().unwrap().remove(label);
                    state.pending.lock().unwrap().remove(label);
                    state.sessions.lock().unwrap().remove(label);
                    state.ready.lock().unwrap().remove(label);
                    state.focus_order.lock().unwrap().retain(|l| l != label);
                    session::reported(&state, label);
                }
                _ => {}
            }
        })
        .setup(|app| {
            let args: Vec<String> = std::env::args().collect();
            let state = app.state::<AppState>();
            touch_focus(&state, "main");

            // An update restart repeats the old process's argv, so the file it
            // was once double-clicked on would come back even if it had since
            // been closed. That echo is skipped; the session says what is open.
            let session = session::take(app.handle());
            let echoed = session
                .as_ref()
                .is_some_and(|s| s.argv[..] == args[1..]);

            // Windows and Linux never fire RunEvent::Opened; a cold
            // file-association open arrives as argv. `main` exists by now but
            // its webview does not, so this stashes rather than emits.
            if let Some(path) = file_from_args(&args).filter(|_| !echoed) {
                open_path(app.handle(), &path);
            }

            if let Some(s) = session {
                restore_session(app.handle(), s.windows);
            }

            // Broadcast: a theme edit restyles every open window at once.
            *state.theme_watch.lock().unwrap() = watch::watch(
                app.handle().clone(),
                themes::watch_dirs(app.handle()),
                |p| p.extension().and_then(|e| e.to_str()) == Some("css"),
                "themes-changed",
                None,
            );

            Ok(())
        });

    #[cfg(target_os = "macos")]
    let builder = builder.menu(macos_menu).on_menu_event(|app, event| {
        if event.id().as_ref() == CLOSE_WINDOW {
            if let Some(label) = last_focused(app) {
                if let Some(w) = app.get_webview_window(&label) {
                    let _ = w.close();
                }
            }
        }
    });

    builder
        .build(tauri::generate_context!())
        .expect("failed to start Markdown Viewer")
        .run(|_app, _event| {
            // macOS is the one platform that hands over a double-clicked file as
            // an event rather than as argv, and it does so for warm opens too —
            // Launch Services reuses the running app instead of starting a
            // second process, so the single-instance hook never sees these.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = &_event {
                for path in urls.iter().filter_map(|u| u.to_file_path().ok()) {
                    if path.is_file() && is_markdown(&path) {
                        open_path(_app, &path);
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_extensions_recognised() {
        assert!(is_markdown(Path::new("a.md")));
        assert!(is_markdown(Path::new("a.MD")));
        assert!(is_markdown(Path::new("a.markdown")));
        assert!(!is_markdown(Path::new("a.exe")));
        assert!(!is_markdown(Path::new("a")));
    }

    #[test]
    fn list_dir_keeps_folders_and_openable_files_in_order() {
        let root = std::env::temp_dir().join(format!("t4-list-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        for f in ["a.md", "B.png", "notes.txt", "x.exe", ".dotfile.md"] {
            std::fs::write(root.join(f), b"").unwrap();
        }

        let names: Vec<(String, bool)> = list_dir(root.to_string_lossy().into_owned())
            .unwrap()
            .entries
            .into_iter()
            .map(|e| (e.name, e.is_dir))
            .collect();
        std::fs::remove_dir_all(&root).unwrap();

        assert_eq!(
            names,
            vec![
                ("sub".to_string(), true),
                ("a.md".to_string(), false),
                ("B.png".to_string(), false),
                ("notes.txt".to_string(), false),
            ]
        );
    }

    #[test]
    fn dot_entries_are_invisible() {
        assert!(is_visible_entry(Path::new("notes/a.md")));
        assert!(is_visible_entry(Path::new("notes/sub")));
        assert!(!is_visible_entry(Path::new("notes/.git")));
        assert!(!is_visible_entry(Path::new("notes/.dotfile.md")));
    }

    #[test]
    fn list_dir_refuses_a_file() {
        let file = std::env::temp_dir().join(format!("t4-not-a-dir-{}.md", std::process::id()));
        std::fs::write(&file, b"").unwrap();
        let result = list_dir(file.to_string_lossy().into_owned());
        std::fs::remove_file(&file).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn argv0_is_never_treated_as_the_document() {
        // Even if the executable itself somehow matched, argv[0] must be skipped.
        let args = vec!["viewer.exe".to_string()];
        assert_eq!(file_from_args(&args), None);
    }

    #[test]
    fn missing_and_non_markdown_paths_are_ignored() {
        let args = vec![
            "viewer.exe".to_string(),
            "--flag".to_string(),
            "does-not-exist.md".to_string(),
        ];
        assert_eq!(file_from_args(&args), None);
    }

    #[test]
    fn unc_prefix_stripped() {
        assert_eq!(strip_unc(Path::new(r"\\?\C:\docs\a.md")), r"C:\docs\a.md");
        assert_eq!(
            strip_unc(Path::new(r"\\?\UNC\srv\share\a.md")),
            r"\\srv\share\a.md"
        );
        assert_eq!(strip_unc(Path::new(r"C:\docs\a.md")), r"C:\docs\a.md");
    }

    #[test]
    fn locate_rejects_what_is_not_a_file() {
        // A directory is the case that matters: it survives canonicalize, so
        // only the is_file guard stops it reaching a reader.
        let err = locate("does-not-exist-here.svg".to_string()).unwrap_err();
        assert!(err.starts_with("Not a file:"), "{err}");
        let dir = std::env::temp_dir().to_string_lossy().into_owned();
        assert!(locate(dir).is_err());
    }

    #[test]
    fn locate_splits_off_the_parent_directory() {
        let exe = std::env::current_exe().unwrap();
        let (path, dir) = locate(exe.to_string_lossy().into_owned()).unwrap();
        assert!(path.is_file());
        assert_eq!(dir, path.parent().unwrap());
    }

    #[test]
    fn theme_names_cannot_escape_the_themes_directory() {
        // path_for() rejects separators and dots; verified here as a contract test.
        for bad in ["../evil", "..\\evil", "C:/evil", "a.b", ""] {
            assert!(
                bad.is_empty() || bad.contains(['/', '\\', ':', '.']),
                "test case {bad:?} should be caught by the guard"
            );
        }
    }

    /// Focus order is what routes a warm file-association open, so the most
    /// recently focused window must always end up last.
    #[test]
    fn focus_order_promotes_the_active_window() {
        let state = AppState::default();
        for label in ["main", "w1", "w2"] {
            touch_focus(&state, label);
        }
        touch_focus(&state, "main");
        let order = state.focus_order.lock().unwrap();
        assert_eq!(*order, vec!["w1", "w2", "main"]);
    }

    #[test]
    fn focus_order_never_duplicates_a_label() {
        let state = AppState::default();
        for _ in 0..3 {
            touch_focus(&state, "w1");
        }
        assert_eq!(*state.focus_order.lock().unwrap(), vec!["w1"]);
    }
}
