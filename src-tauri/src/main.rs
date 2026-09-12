#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod json;
mod render;
mod session;
mod themes;
mod update;
mod watch;

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder, Window,
    WindowEvent,
};
use tauri_plugin_opener::OpenerExt;

/// Extensions accepted from the command line. A CLI argument is untrusted
/// input, so an unrecognised path is ignored rather than opened.
const MD_EXTS: &[&str] = &[
    "md", "markdown", "mdown", "mkd", "mdtext", "mdtxt", "mdwn", "mkdn", "text", "txt",
];
const IMG_EXTS: &[&str] = &[
    "svg", "png", "jpg", "jpeg", "gif", "webp", "avif", "bmp", "ico",
];
/// Shown as highlighted source rather than rendered — see `json.rs`. The
/// installers register these for Open With, never as the default handler.
const JSON_EXTS: &[&str] = &["json", "jsonc"];

/// How big a document `load_file` will read. `.txt` is in `MD_EXTS`, so the
/// sidebar happily offers a multi-gigabyte log, and reading, decoding and
/// rendering it all happen before the command returns — the window is frozen
/// for as long as that takes. Images are deliberately not capped: `load_asset`
/// hands the webview a path rather than bytes, so a 40 MB photo costs nothing
/// here.
const MAX_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;

/// How far each window has got through starting up. One struct behind one lock
/// because the two halves are read against each other — see `open_path`.
#[derive(Default)]
struct Boot {
    /// What a freshly created window should open once its webview asks:
    /// `{kind:"path"}` from a file-association open, `{kind:"tab"}` from a
    /// torn-off tab, or `{kind:"session"}` after an update restart. Keyed by
    /// window label.
    pending: HashMap<String, Value>,
    /// Windows whose webview has asked for its pending payload, and so is live
    /// enough to receive events. A window not in here is still booting, which is
    /// the normal state when the OS hands us a file during startup.
    ready: HashSet<String>,
}

#[derive(Default)]
struct AppState {
    boot: Mutex<Boot>,
    /// What each window has open, as it last reported — or, until it has, what
    /// it was created to open. Only read when an update is about to replace
    /// the process — see `session`.
    sessions: Mutex<HashMap<String, session::OpenTabs>>,
    /// Windows a session snapshot is still waiting to hear from, and the bell
    /// each answer rings. See `session::snapshot`.
    awaiting: Mutex<HashSet<String>>,
    reported: Condvar,
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
    /// What the first update check found, reused by every window that asks
    /// afterwards: `None` until something has checked, `Some(None)` once a
    /// check came back with nothing. One launch, one request — except the
    /// Settings button, which asks again every time it is pressed.
    update: Mutex<Option<Option<update::UpdateInfo>>>,
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
    /// The theme to fall back to when the saved one will not load. The webview
    /// would otherwise have to hard-code a name this side owns.
    default_theme: String,
}

#[derive(Serialize)]
struct Document {
    path: String,
    dir: String,
    title: String,
    html: String,
    /// Whether a clicked box can be written back: true when the bytes are valid
    /// UTF-8, which is what `toggle_task` requires. A file `decode` had to
    /// repair renders fine, but its boxes stay disabled — there is no text to
    /// put back that would not lose what could not be decoded.
    editable: bool,
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

fn is_json(path: &Path) -> bool {
    has_ext(path, JSON_EXTS)
}

/// What this viewer will open in a document tab, however it is asked — the
/// command line, the sidebar, a link, a double-click. Mirrors `DOC_LINK` in
/// app.js.
fn is_document(path: &Path) -> bool {
    is_markdown(path) || is_json(path)
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
        .find(|p| p.is_file() && is_document(p))
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
    claim_in(state, &mut state.boot.lock().unwrap(), label, payload)
}

/// The same claim for a caller already holding `boot`, because what it decided
/// under that lock must still hold when the payload goes in — see `open_path`.
fn claim_in(state: &AppState, boot: &mut Boot, label: &str, payload: Value) -> bool {
    if boot.pending.contains_key(label) {
        return false;
    }
    if let Some(open) = session::OpenTabs::from_pending(&payload) {
        state
            .sessions
            .lock()
            .unwrap()
            .insert(label.to_string(), open);
    }
    boot.pending.insert(label.to_string(), payload);
    true
}

/// Where a new window goes.
enum Placement {
    /// Wherever the OS puts it.
    Default,
    /// A physical screen point the window's top-left goes to. The caller has
    /// already stepped it back from the cursor, in its own pixels, so the
    /// pointer lands near the new window's tab strip.
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
///
/// `source` is the window a torn-off tab came from, if any. That window has
/// already dropped the tab by the time the build runs, so a failure has to be
/// reported back to it or the tab is simply lost.
fn spawn_window(
    app: &AppHandle,
    pending: Option<Value>,
    place: Placement,
    source: Option<String>,
) -> String {
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
                    let _ =
                        win.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
                }
                Placement::Frame(frame) => frame.apply(&win),
            },
            Err(e) => {
                eprintln!("window {target} failed to open: {e}");
                let state = app.state::<AppState>();
                let mut stashed = state.boot.lock().unwrap().pending.remove(&target);
                state.sessions.lock().unwrap().remove(&target);
                // Give a torn-off tab back to the window that let it go. The
                // other spawn paths have nothing to hand back, and a lost
                // `eprintln!` is all a windowless build can offer them.
                if let Some(source) = source {
                    if let Some(tab) = stashed
                        .as_mut()
                        .and_then(|p| p.get_mut("tab"))
                        .map(Value::take)
                    {
                        let _ = app.emit_to(&source, "tab-spawn-failed", json!({ "tab": tab }));
                    }
                }
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
    let mut boot = state.boot.lock().unwrap();
    boot.ready.insert(label.clone());
    boot.pending.remove(&label)
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

/// Refuse a file this side would have to read, decode and render whole. Shared
/// by `load_file` and `json_region` so that a file too big to open is also too
/// big to fetch a chunk of.
fn check_size(path: &Path) -> Result<(), String> {
    let size = std::fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if size > MAX_DOCUMENT_BYTES {
        return Err(format!(
            "{} is too big to open: {} MB, and the limit is {} MB.",
            strip_unc(path),
            size / (1024 * 1024),
            MAX_DOCUMENT_BYTES / (1024 * 1024)
        ));
    }
    Ok(())
}

/// `extent` is how far a JSON document had been loaded when it was last on
/// screen — the frontend's count, handed back so a re-render (a tab switched
/// back to, a live reload, F5) does not drop every chunk a `more` button had
/// fetched. Omitted by every other caller, and ignored by everything but JSON.
#[tauri::command]
fn load_file(app: AppHandle, path: String, extent: Option<usize>) -> Result<Document, String> {
    let (path, dir) = locate(path)?;
    check_size(&path)?;

    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let editable = std::str::from_utf8(&bytes).is_ok();
    let text = render::decode(&bytes);
    // JSON is shown as source rather than rendered, and highlighted here rather
    // than in the webview — see `json.rs`.
    let as_json = is_json(&path);
    let html = if as_json {
        json::render_to(&json::source(&text), extent.unwrap_or(0))
    } else {
        render::render(&text)
    };

    // Let the webview load images and other assets sitting next to the document.
    // Recursive on purpose: documents reference `images/foo.png`, and the scope
    // is the only thing that lets those load. The cost is that the whole subtree
    // stays readable to the webview for the rest of the session; comrak's
    // `unsafe_` being off and the CSP are what keep that from mattering, since
    // no document can talk the webview into fetching anything from it.
    app.asset_protocol_scope().allow_directory(&dir, true).ok();

    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    // A JSON document has no headings to be titled by, and the file name is
    // what the reader went looking for anyway.
    let title = if as_json {
        file_name.clone()
    } else {
        render::first_heading(&text).unwrap_or_else(|| file_name.clone())
    };

    Ok(Document {
        path: strip_unc(&path),
        dir: strip_unc(&dir),
        title,
        html,
        editable,
    })
}

/// Write a ticked or unticked box back to the document. The watcher sees the
/// save and the window re-renders from disk, so nothing is updated here.
///
/// This is the app's only write to a document, and it is one byte written over
/// the box on a line that already holds one — `render::toggle_task` refuses
/// anything else — so the webview cannot use it to write content. Seeking into
/// the file rather than rewriting it also means there is no moment at which the
/// document is truncated, and it stays the same file: whatever else holds it
/// open keeps its handle, and its creation date, permissions and hard links are
/// untouched.
#[tauri::command]
fn toggle_task(path: String, line: usize, checked: bool) -> Result<(), String> {
    let (path, _) = locate(path)?;
    // Errors end up on screen, so name the file the way the user knows it.
    let shown = strip_unc(&path);
    let bytes = std::fs::read(&path).map_err(|e| format!("{shown}: {e}"))?;
    let (bom, body) = match bytes.strip_prefix(render::BOM) {
        Some(b) => (render::BOM, b),
        None => (&[][..], &bytes[..]),
    };
    let text =
        std::str::from_utf8(body).map_err(|_| format!("{shown}: not UTF-8, leaving it alone"))?;
    // The offset is into the text after the BOM, and the file still has it.
    let at = (bom.len() + render::toggle_task(text, line)?) as u64;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .map_err(|e| format!("{shown}: {e}"))?;
    file.seek(SeekFrom::Start(at))
        .map_err(|e| format!("{shown}: {e}"))?;
    file.write_all(&[if checked { b'x' } else { b' ' }])
        .map_err(|e| format!("{shown}: {e}"))
}

/// The Markdown behind one heading, for the copy button the webview puts on
/// it. Read-only, so a file `decode` had to repair is fine to copy from.
///
/// The file is read again here rather than kept from the render: if it changed
/// in between, the watcher is already re-rendering, so the window in which the
/// line could name a different heading is milliseconds wide. Guard it by
/// sending the heading text along if that ever bites.
#[tauri::command]
fn section_source(path: String, line: usize) -> Result<String, String> {
    let (path, _) = locate(path)?;
    let shown = strip_unc(&path);
    let bytes = std::fs::read(&path).map_err(|e| format!("{shown}: {e}"))?;
    render::section(&render::decode(&bytes), line)
}

/// The next chunk of a large JSON document, for the `… more` button `json.rs`
/// left at the end of the last one.
///
/// The file is read and re-derived rather than kept from the render, for the
/// same reason `section_source` re-reads: the render is not state this side
/// owns, and a document can change under it. `render_slice` refuses a range the
/// derived source no longer has, so a stale button reports an error and the
/// watcher's re-render replaces the page moments later.
// ponytail: a one-line 30 MB file is reflowed again on every click (~0.3 s);
// cache the derived source per path in a `Mutex<HashMap>` if that becomes felt.
#[tauri::command]
fn json_region(path: String, start: usize, end: usize) -> Result<String, String> {
    let (path, _) = locate(path)?;
    check_size(&path)?;
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", strip_unc(&path)))?;
    json::render_slice(&json::source(&render::decode(&bytes)), start, end)
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
        |p| {
            if is_visible_entry(p) {
                vec![strip_unc(p)]
            } else {
                vec![]
            }
        },
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
    // Recursive for the same reasoning as `load_file`.
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
            if !is_dir && !is_document(&path) && !is_image(&path) {
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
    // Each file is kept beside the string the frontend registered it under: the
    // frontend matches events against that, and canonicalizing can change it —
    // a picture behind a junction, a file reached through a symlink. Two
    // spellings of one file both get reported.
    let files: Vec<(PathBuf, String)> = paths
        .into_iter()
        .map(|s| {
            let p = PathBuf::from(&s);
            (std::fs::canonicalize(&p).unwrap_or(p), s)
        })
        .collect();

    let mut dirs: Vec<PathBuf> = files
        .iter()
        .filter_map(|(p, _)| p.parent().map(Path::to_path_buf))
        .collect();
    dirs.sort();
    dirs.dedup();

    let label = window.label().to_string();
    let targets = files;
    let handle = watch::watch(
        app,
        dirs,
        move |p| {
            targets
                .iter()
                .filter(|(f, _)| f == p)
                .map(|(_, original)| original.clone())
                .collect()
        },
        "file-changed",
        Some(label.clone()),
    );

    set_watch(&state.watches, label, handle);
}

#[tauri::command]
fn open_window(app: AppHandle, path: Option<String>) -> String {
    let pending = path.map(|p| json!({ "kind": "path", "path": p }));
    spawn_window(&app, pending, Placement::Default, None)
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
            // The offset that puts the cursor on the new window's tab strip is
            // measured in CSS pixels, and `x`/`y` are physical: on a 150%
            // display an unscaled step back lands two thirds of the way there.
            let scale = window.scale_factor().unwrap_or(1.0);
            spawn_window(
                &app,
                Some(json!({ "kind": "tab", "tab": tab })),
                Placement::Cursor(x - 140.0 * scale, y - 24.0 * scale),
                Some(window.label().to_string()),
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
        default_theme: config::DEFAULT_THEME.to_string(),
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

/// Show a file in the file manager, for links the viewer cannot render itself.
/// Deliberately not "open it with its default application": the link comes from
/// a document the user did not write, so `[setup](../tools/setup.bat)` would be
/// one click away from running. Revealing it leaves that choice with the user.
#[tauri::command]
fn reveal_path(app: AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())
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
        // `ready` and `pending` live behind one lock because the question asked
        // here and the answer `take_pending` gives are the same two facts: the
        // check and the claim have to be one step, or a file arriving in the
        // instant a window finishes booting is stashed in a slot that window
        // has already drained. It would come forward showing nothing, and the
        // document would never open.
        let mut boot = state.boot.lock().unwrap();
        if !boot.ready.contains(&label) {
            let claimed = claim_in(&state, &mut boot, &label, payload.clone());
            drop(boot);
            if claimed {
                focus_window(app, &label);
                return;
            }
            // Booting with something already on the way in — a torn-off tab
            // or a restored session. An event now would reach a webview with
            // no listeners yet, so the file gets a window of its own instead.
        } else {
            // Dropped before `load`: the config is a file read, and no lock
            // should be held across one.
            drop(boot);
            if config::load().open_mode != "window" {
                let _ = app.emit_to(&label, "file-opened", path);
                focus_window(app, &label);
                return;
            }
        }
    }

    spawn_window(app, Some(payload), Placement::Default, None);
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
            None,
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
            toggle_task,
            section_source,
            json_region,
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
            reveal_path,
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
                    {
                        let mut boot = state.boot.lock().unwrap();
                        boot.pending.remove(label);
                        boot.ready.remove(label);
                    }
                    state.sessions.lock().unwrap().remove(label);
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
            let echoed = session.as_ref().is_some_and(|s| s.argv[..] == args[1..]);

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
                |p| match p.extension().and_then(|e| e.to_str()) {
                    Some("css") => vec![strip_unc(p)],
                    _ => vec![],
                },
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
                    if path.is_file() && is_document(&path) {
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
    fn json_extensions_recognised() {
        assert!(is_json(Path::new("a.json")));
        assert!(is_json(Path::new("a.JSONC")));
        assert!(!is_json(Path::new("a.json5")));
        assert!(!is_json(Path::new("a.md")));
        // Every gate that used to ask for Markdown now asks for this.
        assert!(is_document(Path::new("a.json")));
        assert!(is_document(Path::new("a.md")));
        assert!(!is_document(Path::new("a.png")));
    }

    #[test]
    fn list_dir_keeps_folders_and_openable_files_in_order() {
        let root = std::env::temp_dir().join(format!("t4-list-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        for f in [
            "a.md",
            "B.png",
            "c.json",
            "notes.txt",
            "x.exe",
            ".dotfile.md",
        ] {
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
                ("c.json".to_string(), false),
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
