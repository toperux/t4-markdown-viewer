//! Window routing against `tauri::test`'s mock runtime: real windows as far as
//! Tauri's own bookkeeping goes, with no webview behind them.
//!
//! The mock runs a task handed to the main thread inline and builds a window on
//! whatever thread asks, so `spawn_window`'s worker needs no event loop here.
//! Settings and sessions land in the test thread's own folder — see
//! `config::dir`.
//!
//! Not here: a tab adopted by another window — `window_at` asks the real
//! desktop through `WindowFromPoint`, and no mock window is ever on it.
//! Not here: a restored frame's placement — the mock has no monitors, so
//! `Frame::apply` leaves every window where it is.
//! Not here: more than one window spawned per test — two mock windows built on
//! two threads at once share a table that is not made for it.
//! Not here: the save a close's grace thread makes — it writes to that
//! thread's folder, so the tests make the same `remember` call themselves.

use super::*;
use std::sync::mpsc;
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::{Listener, WebviewWindow};

/// An app with nothing written to disk: `reopen` off makes `remember` a no-op.
fn app() -> tauri::App<MockRuntime> {
    let app = mock_builder()
        .manage(AppState::default())
        .build(mock_context(noop_assets()))
        .unwrap();
    *app.state::<AppState>().reopen.lock().unwrap() = "off".into();
    app
}

/// An app that writes its session down, as `restore` and `ask` both do.
fn remembering_app() -> tauri::App<MockRuntime> {
    let app = app();
    *app.state::<AppState>().reopen.lock().unwrap() = "restore".into();
    app
}

/// `main` as the config would declare it, and as focused as it is at launch.
fn main_window(app: &tauri::App<MockRuntime>) -> WebviewWindow<MockRuntime> {
    let w = window(app, "main");
    touch_focus(&app.state::<AppState>(), "main");
    w
}

/// A window built here and now, rather than by `spawn_window`'s thread.
fn window(app: &tauri::App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .build()
        .unwrap()
}

/// `spawn_window` builds on a thread of its own.
fn wait_for(app: &tauri::App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(w) = app.get_webview_window(label) {
            return w;
        }
        assert!(Instant::now() < deadline, "{label} never opened");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn pending(app: &tauri::App<MockRuntime>, label: &str) -> Option<Value> {
    app.state::<AppState>()
        .boot
        .lock()
        .unwrap()
        .pending
        .get(label)
        .cloned()
}

fn ready(app: &tauri::App<MockRuntime>, label: &str) -> bool {
    app.state::<AppState>()
        .boot
        .lock()
        .unwrap()
        .ready
        .contains(label)
}

fn restoring(app: &tauri::App<MockRuntime>, label: &str) -> bool {
    app.state::<AppState>()
        .restoring
        .lock()
        .unwrap()
        .contains(label)
}

fn open_mode(mode: &str) {
    config::save(&config::Config {
        open_mode: mode.into(),
        ..config::Config::default()
    });
}

/// What the last save left on disk.
fn saved() -> session::Session {
    let text = std::fs::read_to_string(config::dir().join("session.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn saved_paths() -> Vec<Vec<String>> {
    saved()
        .windows
        .iter()
        .map(|w| {
            w.open
                .tabs
                .iter()
                .map(|t| t["path"].as_str().unwrap().to_string())
                .collect()
        })
        .collect()
}

fn saved_window(paths: &[&str], sidebar: bool) -> session::WindowSession {
    session::WindowSession {
        open: session::OpenTabs {
            tabs: paths.iter().map(|p| json!({ "path": p })).collect(),
            active: 0,
            sidebar,
        },
        frame: Some(session::Frame {
            x: 10,
            y: 20,
            width: 800,
            height: 600,
            maximized: false,
        }),
    }
}

fn report(app: &tauri::App<MockRuntime>, w: &WebviewWindow<MockRuntime>, paths: &[&str]) {
    let tabs = paths.iter().map(|p| json!({ "path": p })).collect();
    set_session(app.state(), w.as_ref().window(), tabs, 0, false, false);
}

fn url(s: &str) -> Url {
    Url::parse(s).unwrap()
}

/* ---------------- opening a file ---------------- */

/// A file opened while `main` boots waits in its slot, and `take_pending`
/// hands it over once — the second ask is a reload, with nothing to give.
#[test]
fn a_file_opened_while_main_boots_is_handed_over_once() {
    let app = app();
    let main = main_window(&app);

    open_path(app.handle(), Path::new("C:\\notes\\a.md"));

    let expected = json!({ "kind": "path", "path": "C:\\notes\\a.md" });
    assert_eq!(pending(&app, "main"), Some(expected.clone()));
    // It counts as open before the window has said so.
    assert!(app
        .state::<AppState>()
        .sessions
        .lock()
        .unwrap()
        .contains_key("main"));

    let window = main.as_ref().window();
    assert_eq!(take_pending(app.state(), window.clone()), Some(expected));
    assert_eq!(take_pending(app.state(), window), None);
    assert_eq!(app.webview_windows().len(), 1);
}

/// A ready window takes a warm open as an event, and no window is made.
#[test]
fn a_file_opened_with_a_ready_window_goes_to_it() {
    let app = app();
    let main = main_window(&app);
    take_pending(app.state(), main.as_ref().window());
    open_mode("tab");

    let (tx, rx) = mpsc::channel();
    app.listen_any("file-opened", move |e| {
        let _ = tx.send(e.payload().to_string());
    });
    open_path(app.handle(), Path::new("C:\\notes\\b.md"));

    assert_eq!(rx.try_recv().unwrap(), r#""C:\\notes\\b.md""#);
    assert_eq!(app.webview_windows().len(), 1);
    assert_eq!(pending(&app, "main"), None);
}

/// Unless the reader's config says every file gets a window of its own.
#[test]
fn a_file_opened_in_window_mode_gets_a_window_of_its_own() {
    let app = app();
    let main = main_window(&app);
    take_pending(app.state(), main.as_ref().window());
    open_mode("window");

    let (tx, rx) = mpsc::channel();
    app.listen_any("file-opened", move |e| {
        let _ = tx.send(e.payload().to_string());
    });
    open_path(app.handle(), Path::new("C:\\notes\\b.md"));

    wait_for(&app, "w1");
    assert!(rx.try_recv().is_err());
    assert_eq!(
        pending(&app, "w1"),
        Some(json!({ "kind": "path", "path": "C:\\notes\\b.md" }))
    );
}

/// `main` booting with a restored session on its way in keeps it; the file
/// gets a new window, created with the file as its pending payload.
#[test]
fn a_file_opened_while_main_boots_with_a_payload_gets_a_new_window() {
    let app = app();
    let _main = main_window(&app);
    let restore = json!({ "kind": "session", "tabs": [{ "path": "r.md" }], "active": 0 });
    assert!(claim_pending(
        &app.state::<AppState>(),
        "main",
        restore.clone()
    ));

    open_path(app.handle(), Path::new("C:\\notes\\c.md"));

    let w1 = wait_for(&app, "w1");
    assert_eq!(pending(&app, "main"), Some(restore));
    assert_eq!(
        take_pending(app.state(), w1.as_ref().window()),
        Some(json!({ "kind": "path", "path": "C:\\notes\\c.md" }))
    );
}

/// A session payload marks the window as restoring and stands in for its
/// report; a second claim on the slot is refused and changes nothing.
#[test]
fn a_session_claim_is_restoring_and_cannot_be_overwritten() {
    let app = app();
    let state = app.state::<AppState>();
    let restore =
        json!({ "kind": "session", "tabs": [{ "path": "r.md" }], "active": 0, "sidebar": true });

    assert!(claim_pending(&state, "main", restore.clone()));
    assert!(!claim_pending(
        &state,
        "main",
        json!({ "kind": "path", "path": "x.md" })
    ));

    assert!(state.restoring.lock().unwrap().contains("main"));
    let open = state.sessions.lock().unwrap().get("main").cloned().unwrap();
    assert_eq!(open.tabs, vec![json!({ "path": "r.md" })]);
    assert!(open.sidebar);
    assert_eq!(pending(&app, "main"), Some(restore));
}

/// A report replaces whatever the window was created with.
#[test]
fn a_report_replaces_the_pending_stand_in() {
    let app = app();
    let main = main_window(&app);
    claim_pending(
        &app.state::<AppState>(),
        "main",
        json!({ "kind": "path", "path": "a.md" }),
    );

    let tabs = vec![json!({ "path": "a.md" }), json!({ "path": "b.md" })];
    set_session(
        app.state(),
        main.as_ref().window(),
        tabs.clone(),
        1,
        false,
        false,
    );

    let open = app.state::<AppState>().sessions.lock().unwrap()["main"].clone();
    assert_eq!(open.tabs, tabs);
    assert_eq!(open.active, 1);
}

/* ---------------- the reload guard ---------------- */

/// A booted window reloading its own page gets back what it last reported,
/// and boots again: not ready, and not restoring from disk either. Both
/// spellings of the app's own origin count.
#[test]
fn a_reload_gets_the_last_report_back() {
    let app = app();
    let main = main_window(&app);
    let tabs = vec![json!({ "path": "a.md" }), json!({ "path": "b.md" })];
    set_session(
        app.state(),
        main.as_ref().window(),
        tabs.clone(),
        1,
        true,
        false,
    );

    for page in ["http://tauri.localhost/", "tauri://localhost/"] {
        take_pending(app.state(), main.as_ref().window());
        assert!(stay(main.as_ref(), &url(page)), "{page}");
        assert!(!ready(&app, "main"), "{page}");
        assert_eq!(
            pending(&app, "main"),
            Some(json!({ "kind": "session", "tabs": tabs, "active": 1, "sidebar": true })),
            "{page}"
        );
    }
    assert!(!restoring(&app, "main"));
}

/// A booted page never goes anywhere else: another page of the app's own, or
/// another origin, is refused and leaves the window as it was.
#[test]
fn a_booted_window_goes_nowhere_else() {
    let app = app();
    let main = main_window(&app);
    take_pending(app.state(), main.as_ref().window());
    report(&app, &main, &["a.md"]);

    for page in [
        "http://tauri.localhost/other.html",
        "https://example.com/",
        "http://evil.localhost/",
        "tauri://elsewhere/",
    ] {
        assert!(!stay(main.as_ref(), &url(page)), "{page}");
    }
    assert!(ready(&app, "main"));
    assert_eq!(pending(&app, "main"), None);
}

/// Before `take_pending` it is the window's own first load, whatever it is,
/// and nothing is stashed for it.
#[test]
fn a_first_load_is_let_through() {
    let app = app();
    let main = main_window(&app);
    report(&app, &main, &["a.md"]);

    assert!(stay(main.as_ref(), &url("http://tauri.localhost/")));
    assert!(stay(main.as_ref(), &url("https://example.com/")));
    assert_eq!(pending(&app, "main"), None);
    assert!(!ready(&app, "main"));
}

/// The dev server's port is let through in a dev build and refused otherwise;
/// a `#id` jump is a reload on Windows, where WebView2 never asks about one,
/// and let through untouched everywhere else.
#[test]
fn a_dev_port_and_a_fragment_follow_the_build_and_the_platform() {
    let app = app();
    let main = main_window(&app);
    take_pending(app.state(), main.as_ref().window());
    report(&app, &main, &["a.md"]);

    assert_eq!(
        stay(main.as_ref(), &url("http://localhost:1420/")),
        cfg!(dev)
    );
    assert!(ready(&app, "main"));

    assert!(stay(main.as_ref(), &url("http://tauri.localhost/#h")));
    assert_eq!(ready(&app, "main"), !cfg!(windows));
    assert_eq!(pending(&app, "main").is_some(), cfg!(windows));
}

/* ---------------- restoring a session ---------------- */

/// The first saved window goes to `main`, which would otherwise stand empty;
/// the next gets a window of its own. Nothing stands behind `main`, which is
/// showing a saved window rather than a file the reader opened.
#[test]
fn a_restore_puts_the_first_saved_window_in_main() {
    let app = app();
    let _main = main_window(&app);
    let first = saved_window(&["a.md", "b.md"], true);
    let second = saved_window(&["c.md"], false);

    restore_session(app.handle(), vec![first.clone(), second], false);

    assert_eq!(
        pending(&app, "main"),
        Some(json!({
            "kind": "session",
            "tabs": [{ "path": "a.md" }, { "path": "b.md" }],
            "active": 0,
            "sidebar": true,
            "maximized": false,
        }))
    );
    let sessions = app.state::<AppState>().sessions.lock().unwrap().clone();
    assert_eq!(sessions["main"], first.open);
    assert!(restoring(&app, "main"));

    wait_for(&app, "w1");
    let w1 = pending(&app, "w1").unwrap();
    assert_eq!(w1["tabs"], json!([{ "path": "c.md" }]));
    assert_eq!(w1.get("behind"), None);
    assert!(restoring(&app, "w1"));
}

/// With `main` claimed by a file, every saved window gets one of its own and
/// stands behind it, so the file the reader opened stays in front.
#[test]
fn a_restore_behind_a_file_gets_windows_of_its_own() {
    let app = app();
    let _main = main_window(&app);
    let file = json!({ "kind": "path", "path": "f.md" });
    claim_pending(&app.state::<AppState>(), "main", file.clone());

    restore_session(app.handle(), vec![saved_window(&["a.md"], false)], false);

    assert_eq!(pending(&app, "main"), Some(file));
    assert!(!restoring(&app, "main"));
    wait_for(&app, "w1");
    let w1 = pending(&app, "w1").unwrap();
    assert_eq!(w1["kind"], "session");
    assert_eq!(w1["tabs"], json!([{ "path": "a.md" }]));
    assert_eq!(w1["behind"], "main");
    assert!(restoring(&app, "w1"));
}

/// The window the button was on takes the first saved window as its answer,
/// and is in the record from then on — restoring, with those tabs — so a save
/// before it reports keeps them. The rest get windows of their own, and the
/// offer is used up.
#[test]
fn an_offered_session_answers_the_first_window_and_spawns_the_rest() {
    let app = app();
    let main = main_window(&app);
    let first = saved_window(&["a.md"], true);
    *app.state::<AppState>().offered.lock().unwrap() = Some(session::Session {
        version: "1.0.0".into(),
        argv: vec![],
        restart: false,
        restoring: false,
        windows: vec![first.clone(), saved_window(&["c.md"], false)],
    });

    let answer = restore_offered_session(app.handle().clone(), main.as_ref().window());

    assert_eq!(answer, Some(session_payload(&first, false)));
    assert!(restoring(&app, "main"));
    let sessions = app.state::<AppState>().sessions.lock().unwrap().clone();
    assert_eq!(sessions["main"], first.open);
    assert_eq!(pending(&app, "main"), None);

    wait_for(&app, "w1");
    assert_eq!(
        pending(&app, "w1").unwrap()["tabs"],
        json!([{ "path": "c.md" }])
    );
    assert_eq!(
        restore_offered_session(app.handle().clone(), main.as_ref().window()),
        None
    );
}

/* ---------------- saving ---------------- */

/// Least-recently-focused first, whatever the labels: `w1` was focused before
/// `main` was last, so it is written first, each with its tabs and sidebar.
#[test]
fn a_save_lists_windows_least_recently_focused_first() {
    let app = remembering_app();
    let main = main_window(&app);
    let w1 = window(&app, "w1");
    touch_focus(&app.state::<AppState>(), "w1");
    touch_focus(&app.state::<AppState>(), "main");

    set_session(
        app.state(),
        main.as_ref().window(),
        vec![json!({ "path": "m.md" })],
        0,
        true,
        false,
    );
    report(&app, &w1, &["w.md", "x.md"]);

    assert_eq!(saved_paths(), vec![vec!["w.md", "x.md"], vec!["m.md"]]);
    let session = saved();
    assert!(!session.windows[0].open.sidebar);
    assert!(session.windows[1].open.sidebar);
    assert!(!session.restart);
    assert!(!session.restoring);
}

/// A quit is every window closing one after another: the last close lands
/// within the grace of the one before, so both are written — in the order
/// they stood — and stay written once the grace is over, since nothing is left
/// open to take their place.
#[test]
fn a_quit_within_the_grace_keeps_every_window() {
    let app = remembering_app();
    let main = main_window(&app);
    let w1 = window(&app, "w1");
    report(&app, &main, &["m.md"]);
    report(&app, &w1, &["w.md"]);

    session::note_frame(app.handle(), "w1");
    session::window_closed(app.handle(), "w1");
    session::note_frame(app.handle(), "main");
    session::window_closed(app.handle(), "main");
    assert_eq!(saved_paths(), vec![vec!["m.md"], vec!["w.md"]]);
    assert!(saved().windows.iter().all(|w| w.frame.is_some()));

    // What the grace thread does once it is over.
    std::thread::sleep(session::CLOSE_GRACE + Duration::from_millis(100));
    session::remember(app.handle());
    assert_eq!(saved_paths(), vec![vec!["m.md"], vec!["w.md"]]);
}

/// A window closed on purpose, with the app carrying on past the grace, drops
/// out: the last window's close finds the chain expired and writes only
/// itself.
#[test]
fn a_window_closed_outside_the_grace_drops_out() {
    let app = remembering_app();
    let main = main_window(&app);
    let w1 = window(&app, "w1");
    report(&app, &main, &["m.md"]);
    report(&app, &w1, &["w.md"]);

    session::window_closed(app.handle(), "w1");
    assert_eq!(saved_paths(), vec![vec!["m.md"], vec!["w.md"]]);

    std::thread::sleep(session::CLOSE_GRACE + Duration::from_millis(100));
    session::window_closed(app.handle(), "main");
    assert_eq!(saved_paths(), vec![vec!["m.md"]]);
}

/* ---------------- dropping a tab ---------------- */

/// Far off every screen, so no window is under the point. Nothing — not the
/// mock's windows, nor anything else on the desktop — can adopt the tab there.
const NOWHERE: f64 = -100_000.0;

/// Dropped over nothing, a tab tears off into a window of its own, created
/// with the tab as its payload, and the last window's caret is cleared.
#[test]
fn a_tab_dropped_over_nothing_tears_off() {
    let app = app();
    let main = main_window(&app);
    *app.state::<AppState>().drag_target.lock().unwrap() = Some("w9".into());
    let tab = json!({ "path": "t.md", "entries": [{ "path": "t.md" }], "index": 0 });

    let result = drop_tab(
        app.handle().clone(),
        app.state(),
        main.as_ref().window(),
        NOWHERE,
        NOWHERE,
        tab.clone(),
        true,
    );

    assert_eq!(result, Ok("detached".into()));
    assert_eq!(*app.state::<AppState>().drag_target.lock().unwrap(), None);
    assert_eq!(
        pending(&app, "w1"),
        Some(json!({ "kind": "tab", "tab": tab }))
    );
    wait_for(&app, "w1");
}

/// Not torn off when the page says not to: the drop is cancelled, the tab
/// stays where it was, and no window is made.
#[test]
fn a_tab_dropped_without_tear_off_is_cancelled() {
    let app = app();
    let main = main_window(&app);
    *app.state::<AppState>().drag_target.lock().unwrap() = Some("w9".into());

    let result = drop_tab(
        app.handle().clone(),
        app.state(),
        main.as_ref().window(),
        NOWHERE,
        NOWHERE,
        json!({ "path": "t.md" }),
        false,
    );

    assert_eq!(result, Ok("cancelled".into()));
    assert_eq!(*app.state::<AppState>().drag_target.lock().unwrap(), None);
    assert_eq!(pending(&app, "w1"), None);
    assert_eq!(app.webview_windows().len(), 1);
}
