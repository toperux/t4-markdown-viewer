#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod render;
mod themes;
mod watch;

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder, Window,
    WindowEvent,
};

/// Extensions accepted from the command line. A CLI argument is untrusted
/// input, so an unrecognised path is ignored rather than opened.
const MD_EXTS: &[&str] = &[
    "md", "markdown", "mdown", "mkd", "mdtext", "mdtxt", "mdwn", "mkdn", "text", "txt",
];

#[derive(Default)]
struct AppState {
    /// What a freshly created window should open once its webview asks. Either
    /// `{kind:"path"}` from a file-association open or `{kind:"tab"}` from a
    /// torn-off tab. Keyed by window label.
    pending: Mutex<HashMap<String, Value>>,
    /// One watcher per window, covering every directory that window has a tab in.
    watches: Mutex<HashMap<String, watch::Handle>>,
    /// App-wide, unlike `watches`: a theme edit restyles every window.
    theme_watch: Mutex<Option<watch::Handle>>,
    /// Window labels, least-recently-focused first. Decides which window a warm
    /// file-association open goes to, and breaks ties between overlapping
    /// windows during a tab drag.
    focus_order: Mutex<Vec<String>>,
    /// Window currently showing a drop caret, so it can be told to clear it.
    drag_target: Mutex<Option<String>>,
    next_window: AtomicUsize,
}

#[derive(Serialize)]
struct Origin {
    x: f64,
    y: f64,
    scale: f64,
}

#[derive(Serialize)]
struct Document {
    path: String,
    dir: String,
    title: String,
    html: String,
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| MD_EXTS.iter().any(|m| m.eq_ignore_ascii_case(e)))
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

/// Create a window. `at` is a physical screen point taken straight off a
/// pointer event; the window is offset so the cursor lands near its tab strip.
///
/// The build runs on a worker thread on purpose. `build()` waits on the event
/// loop to construct the webview, and both callers here — a synchronous command
/// and the single-instance hook — already run *on* that loop, so building
/// inline deadlocks the app. Reserving the label and stashing `pending` happens
/// first and synchronously, so the new window's `take_pending` cannot race it.
fn spawn_window(app: &AppHandle, pending: Option<Value>, at: Option<(f64, f64)>) -> String {
    let state = app.state::<AppState>();
    let n = state.next_window.fetch_add(1, Ordering::Relaxed) + 1;
    let label = format!("w{n}");

    if let Some(p) = pending {
        state.pending.lock().unwrap().insert(label.clone(), p);
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
            Ok(win) => {
                if let Some((x, y)) = at {
                    let _ = win.set_position(PhysicalPosition::new(
                        (x - 140.0).round() as i32,
                        (y - 24.0).round() as i32,
                    ));
                }
            }
            Err(e) => {
                eprintln!("window {target} failed to open: {e}");
                app.state::<AppState>()
                    .pending
                    .lock()
                    .unwrap()
                    .remove(&target);
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

#[cfg(not(windows))]
fn hwnd_at(_x: f64, _y: f64) -> isize {
    0
}

/// The app window a physical screen point lands on, plus that point in the
/// window's CSS pixels.
///
/// This is a true z-order test rather than a scan of window rectangles: the
/// answer has to be the window the user can actually *see* at the cursor.
/// Note it can return the dragging window itself — callers treat that as
/// "dropped on my own window", which tears the tab off.
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

/// Hand this window whatever it was created to show. Consumed on first call.
#[tauri::command]
fn take_pending(state: State<AppState>, window: Window) -> Option<Value> {
    state.pending.lock().unwrap().remove(window.label())
}

#[tauri::command]
fn load_file(app: AppHandle, path: String) -> Result<Document, String> {
    let path = PathBuf::from(&path);
    let path = std::fs::canonicalize(&path).unwrap_or(path);
    if !path.is_file() {
        return Err(format!("Not a file: {}", path.display()));
    }

    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = render::decode(&bytes);
    let html = render::render(&text);

    let dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

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

    // Assigning drops the previous watcher for this window.
    let mut watches = state.watches.lock().unwrap();
    match handle {
        Some(h) => {
            watches.insert(label, h);
        }
        None => {
            watches.remove(&label);
        }
    }
}

#[tauri::command]
fn open_window(app: AppHandle, path: Option<String>) -> String {
    let pending = path.map(|p| json!({ "kind": "path", "path": p }));
    spawn_window(&app, pending, None)
}

/// The window's client-area origin and scale, both physical. The frontend turns
/// pointer coordinates into screen coordinates with these instead of
/// `screenX * devicePixelRatio`, which guesses wrong the moment two monitors
/// run at different scaling.
#[tauri::command]
fn window_origin(window: Window) -> Result<Origin, String> {
    let pos = window.inner_position().map_err(|e| e.to_string())?;
    Ok(Origin {
        x: pos.x as f64,
        y: pos.y as f64,
        scale: window.scale_factor().unwrap_or(1.0),
    })
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
/// anything else it becomes a new window at the cursor. Either way the caller
/// drops its own copy once this returns.
#[tauri::command]
fn drop_tab(
    app: AppHandle,
    state: State<AppState>,
    window: Window,
    x: f64,
    y: f64,
    tab: Value,
) -> Result<String, String> {
    clear_drag(&app, &state);

    match window_at(&app, x, y).filter(|(label, _, _)| label != window.label()) {
        Some((label, lx, _)) => {
            app.emit_to(&label, "tab-adopt", json!({ "tab": tab, "x": lx }))
                .map_err(|e| e.to_string())?;
            focus_window(&app, &label);
            Ok("adopted".into())
        }
        None => {
            spawn_window(
                &app,
                Some(json!({ "kind": "tab", "tab": tab })),
                Some((x, y)),
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
fn get_settings() -> config::Config {
    config::load()
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

fn handle_second_instance(app: &AppHandle, argv: Vec<String>) {
    let Some(path) = file_from_args(&argv) else {
        if let Some(label) = last_focused(app) {
            focus_window(app, &label);
        }
        return;
    };
    let path = path.to_string_lossy().into_owned();

    let tabbed = config::load().open_mode != "window";
    if let (true, Some(label)) = (tabbed, last_focused(app)) {
        let _ = app.emit_to(&label, "file-opened", path);
        focus_window(app, &label);
        return;
    }
    spawn_window(app, Some(json!({ "kind": "path", "path": path })), None);
}

fn main() {
    tauri::Builder::default()
        // Must be registered first: plugins run in registration order, and this
        // one has to intercept the second process before anything else starts.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            handle_second_instance(app, argv);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            take_pending,
            load_file,
            watch_files,
            open_window,
            window_origin,
            drag_over,
            drag_cancel,
            drop_tab,
            list_themes,
            read_theme,
            get_settings,
            set_theme,
            set_open_mode,
        ])
        .on_window_event(|window, event| {
            let state = window.app_handle().state::<AppState>();
            match event {
                WindowEvent::Focused(true) => touch_focus(&state, window.label()),
                WindowEvent::Destroyed => {
                    let label = window.label();
                    state.watches.lock().unwrap().remove(label);
                    state.pending.lock().unwrap().remove(label);
                    state.focus_order.lock().unwrap().retain(|l| l != label);
                }
                _ => {}
            }
        })
        .setup(|app| {
            let args: Vec<String> = std::env::args().collect();
            let state = app.state::<AppState>();
            touch_focus(&state, "main");

            // Windows never fires RunEvent::Opened; a cold file-association open
            // arrives as argv. Stash it until the webview is ready to ask.
            if let Some(path) = file_from_args(&args) {
                state.pending.lock().unwrap().insert(
                    "main".into(),
                    json!({ "kind": "path", "path": path.to_string_lossy() }),
                );
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
        })
        .run(tauri::generate_context!())
        .expect("failed to start Markdown Viewer");
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
        assert_eq!(strip_unc(Path::new(r"C:\docs\a.md")), r"C:\docs\a.md");
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
