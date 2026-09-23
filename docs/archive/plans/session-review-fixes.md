# Session Review Fixes Implementation Plan

> **Status (2026-09-24):** executed in full and shipped in 1.6.4 — Tasks 1–4 and 6 in 053979c,
> Task 5 in fae6909, Task 7 in 9df9ef0, Task 8 in 6b2c8fa. Paths and unticked boxes below are
> as they stood when it was written.

> **For agentic workers:** Execute per the **Execution** section below — it says who runs each task and in what order, and it overrides the one-subagent-per-task default of superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the 10 findings from the code review of `v1.6.0..HEAD` (commit `cf7d8fa`), most of them in the session-restore feature shipped in 1.6.3.

**Architecture:** No new modules. Rust fixes live in `session.rs` (what a saved file means), `config.rs` (the `reopen` value) and `main.rs` (boot, window events, commands). Frontend fixes are small guards in `src/app.js` and two values in `src/base.css`. Pure logic gets a unit test beside the existing ones; behaviour that only exists in a running window is checked by driving the debug build (`drive-app` skill).

**Tech Stack:** Rust + Tauri v2, serde, vanilla JS (no bundler, no frontend test harness), CSS.

**Spec:** The review findings, restated here (line numbers as of `cf7d8fa`):

| # | Where | Defect |
|---|---|---|
| 1 | `session.rs:146` | `restart` missing from a ≤1.6.2 file reads as `false`; those files are all update snapshots, so the update restart is offered, not restored. |
| 2 | `app.js:2491` | Scroll listener banks `window.scrollY` on the *new* active entry while the *old* document is still on screen (tab switch / Back awaiting `load_file`). |
| 3 | `app.js:758` | 500ms debounced report is never flushed; a tab change right before quit is lost. |
| 4 | `main.rs:1221` | Deliberately closed window comes back; move/resize/maximize never save, so frames go stale. |
| 5 | `main.rs:1255` / `session.rs:276` | Ordinary sessions go through the update-restart version filter; a cancelled install or manual update drops the session, and a stale restart file can be force-restored later. |
| 6 | `main.rs:904` | `set_reopen` writes nothing when leaving `off`; `mode` unvalidated (`"Ask"` restores, dialog shows no radio). |
| 7 | `main.rs:1285` | `restore` + file-association open: restored windows take focus after the opened file; a file already in the session opens twice. |
| 8 | `base.css:305` | 3.5rem fade band covers where anchor jumps land, and washes the first line at rest. |
| 9 | `main.rs:943`, `session.rs:197` | `picker_dir` stats a possibly dead share on the main thread (freezes every window); `remember` re-reads `config.json` on every report. |
| 10 | `app.js:1096`, `app.js:1138` | Mac Ctrl+click falls through to `loadPath`; the context menu it raises is placed as if from the keyboard. |

Decisions already made with the owner:
- **#4:** the closed chain — a window that closes stays in the saved session for a 4s grace, written with the open ones, and drops out once 4s pass with no further close. A quit (windows going within 4s of each other) still restores everything. This replaced a first draft (a counter and one delayed save) whose outcome depended on whether a surviving window happened to report inside the grace. Same rule set as t4-git-ui `3fcf6f5`; see Task 3.
- **#8:** shorten the band to the content's top padding (`--content-pad`, 2.5rem) rather than padding the document down.
- **#4, focus order:** saved on focus change (Task 6), which the chain is what makes safe. A focus-triggered save fires right after a sibling window closes — the survivor takes focus — and under the first draft that would have whittled the session during a quit. With the chain, that save still carries the closed window.

## Global Constraints

- Surgical changes only: every changed line traces to a finding above. Match the surrounding comment style (full-sentence "why" comments) and naming.
- No new dependencies. No new files except this plan.
- Gates, all run from the repo root, all must pass before each commit:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`
  - `node --check src/app.js`
- All commits go straight onto `main` — the owner's decision; this repository has no feature branches, and they squash on `main` before a release. Commit locally only. Never push. Commit subjects are modest, reader-facing sentences with **no prefix**, like the rest of the log: `release.yml` builds the release notes from every subject that does not start with `ci:`, `docs:`, `release:` or `chore:`, so a `fix:` prefix would be printed verbatim and could title the release. Commits that are not user-facing (this plan, the `cdp.mjs` tweak) take `docs:` / `chore:`. The owner squashes before release.
- **Ordinary saves run on the main thread.** `session::remember` reads the state, then writes and renames; two of them on different threads can finish out of order and leave the *older* content on disk. Synchronous commands (`set_session`, `set_reopen`) are already on the main thread. Anything else — the Task 3 timer thread, the async `install_update` failure path — calls it through `app.run_on_main_thread(...)`. Do **not** solve this with a lock around `save` instead: `Frame::of` asks the window for its geometry, which off the main thread is a round trip through the event loop, and a main thread parked on that lock would never answer it.
- Bash on Windows: never `cd` in a compound command that writes; run from the repo root and use `--manifest-path`.
- **The debug build shares `%APPDATA%\t4-markdown-viewer\` with the installed app — `config.json` *and* `session.json`.** The drive-app build overrides the app identifier, not the config directory. Two consequences for every check that reads `session.json`:
  - The owner's installed viewer (1.6.3) rewrites that file whenever it is scrolled or a tab changes, and the debug app overwrites the owner's real session. The owner believes no viewer is open; check with `Get-Process t4-markdown-viewer` before Phase 0b and again before Phase 4, and if one is running, ask before going on, per the drive-app skill.
  - Back up both files to the scratchpad first; restore them last, after the debug process is gone, following the skill's diff-before-restore rule.

## Execution

Subagents where they save money or main-session context, the main session where spawning costs more than the edit. Agent types are the ones in `~/.claude/agents/`; do not pass `model:`.

| Phase | Work | Who | Why |
|---|---|---|---|
| 0a | Confirm no installed viewer is running (`Get-Process t4-markdown-viewer`; the owner believes none is — if one turns up, ask before going on). Commit this plan (`docs:`). Give `cdp.mjs` a page picker and commit it (`chore:`) — see below. | main session | Git writes and a script edit are not `tester` work. |
| 0b | Back up `config.json` + `session.json` to the scratchpad. Task 5 Step 1 (reproduce #2 on the unfixed build). | `tester` + drive-app skill | Must happen before any fix lands. Also warms the cargo build the coder needs. |
| 1 | Tasks 1 → 2 → 3, one commit each | one `coder` | All Rust, all in `session.rs` / `main.rs` / `config.rs`. One agent reads those files once instead of three times. |
| — | Review: read the three diffs and the gate output | main session | |
| 2 | Task 4 | the **same** `coder`, continued with SendMessage | It already holds `main.rs` and `session.rs`; a fresh agent would re-read both. |
| — | Review: read the diff and the gate output | main session | |
| 3 | Tasks 5 → 6 → 7 → 8, one commit each, `node --check` after each JS edit | main session, directly | Each is one file and ≤20 lines, with the code already written out in this plan. |
| 4 | One smoke pass: every "Verify in the running app" step (Tasks 3, 4, 5 Step 3, 6, 8), in one launch session, then the four gates on the final tree. Restore the backed-up files. | `tester` + drive-app skill | One build and one app session instead of five. Raw results only; no code changes. |
| — | Triage the smoke results; anything failing goes back to the `coder` (Rust) or is fixed directly (frontend) | main session | |
| 5 | Task 9 hand-off report | main session | |

**Phase 0a, the `cdp.mjs` page picker.** The script drives "the first tauri page in `/json/list`", and that order is not the window order — with two or three windows up, which is most of Phase 4, it may not be `main`. In `.claude/skills/drive-app/scripts/cdp.mjs` replace the `find` line:

```js
// CDP_PAGE=<n> picks the nth tauri page; which window that is, ask it:
// eval "appWindow.label"
const page = pages.filter((p) => p.type === "page" && p.url.includes("tauri"))[
  Number(process.env.CDP_PAGE ?? 0)
];
```

and in `SKILL.md` replace the "Several windows" gotcha with: `**Several windows.** \`CDP_PAGE=<n> node cdp.mjs …\` picks the nth tauri page in \`/json/list\` (default 0). The order is not the window order — \`eval "appWindow.label"\` says which window a page is.`

**Dispatching `tester`.** It has no Skill tool: tell it to `Read` `.claude/skills/drive-app/SKILL.md` first and follow it, including the `TAURI_CONFIG` identifier override and the rebuild-after-every-`src/`-edit rule. It reports raw results — returned values, `session.json` contents, screenshot paths — and draws no conclusions. It runs the four gates *after* the smoke pass, because a plain `cargo` build resets the identifier override.

Rules for the phases:
- **Serial, not parallel.** Every task shares `main.rs`, `session.rs` or `app.js` with a neighbour, and Task 6 builds on lines Task 4 adds. A worktree per track would buy a few minutes and cost a merge.
- **`coder` skips the drive-app steps** (Task 3 Step 8, Task 4 Step 7) — those run in Phase 4. It runs the unit tests and all four gates for its own changes and reports the output; nobody re-runs what it already ran.
- **`coder` gets its task text, not this whole file:** point it at the task's section by heading, plus Global Constraints.
- **No reviewer subagents.** Review is the main session reading the diff and the gate output before moving on.
- **A Phase 4 failure in Task 4's focus hand-back** (capability denies `setFocus` on another window) goes to the `coder` with the fallback named in Task 4 Step 7.

## File Map

| File | Tasks | What changes |
|---|---|---|
| `src-tauri/src/session.rs` | 1, 2, 3, 4 | `restart` default, version demotion, `remember` reads cached mode, the closed chain (`CLOSE_GRACE`, `chain_expired`, `restorable`, `note_frame`, `window_closed`, frame cache in `save`), `surface()` |
| `src-tauri/src/update.rs` | 1 | failed install writes the ordinary session back |
| `src-tauri/src/config.rs` | 2 | `reopen_mode()`, `load()` normalises |
| `src-tauri/src/main.rs` | 2, 3, 4 | `AppState.reopen`, `AppState.closed`, `AppState.frames`, `set_reopen`, `picker_dir`, `CloseRequested`, `Destroyed`, setup, `restore_session`, `session_payload` |
| `src/app.js` | 4, 5, 6, 7 | `behind` focus hand-back, scroll guard, `reportSoon`, move/resize/focus save, Mac Ctrl+click |
| `src/base.css` | 8 | band height, `scroll-margin-top` |

---

### Task 1: What a saved file means (#1, #5)

**Files:**
- Modify: `src-tauri/src/session.rs:127-150` (struct `Session`), `:256-280` (`read`, `discard`, `read_from`), tests at `:323-332`

**Interfaces:**
- Produces: `session::read(app) -> Option<Session>` now returns ordinary sessions whatever their `version`, and returns a version-mismatched restart file *demoted* (`restart: false`, `argv: []`), having rewritten it on disk. Boot code in `main.rs` needs no change for this task.

- [ ] **Step 1: Replace the stale-version test and add two more**

In `session.rs` tests, delete `a_session_for_another_version_is_dropped` and add:

```rust
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
```

- [ ] **Step 2: Run, expect failures**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked session::`
Expected: the three new tests FAIL (`None` instead of `Some`, `restart` false).

- [ ] **Step 3: Implement**

Above `pub struct Session`:

```rust
/// What a missing `restart` means — see the field.
fn yes() -> bool {
    true
}
```

On the field, replace `#[serde(default)]` and extend its comment:

```rust
    /// Written by an update snapshot, and by nothing else. A restart is the
    /// app coming back mid-read rather than a new launch, so it restores
    /// whatever the reopen setting says about ordinary ones — the reader
    /// pressed Update, not Quit.
    ///
    /// Missing means true: 1.6.2 and earlier wrote this file for restarts
    /// only, and had no field to say so.
    #[serde(default = "yes")]
    pub restart: bool,
```

Replace the `version` field comment's last clause so it matches the new rule:

```rust
    /// The version being installed. Only that version's first launch is the
    /// restart this was written for: on Windows the process is gone before the
    /// installer has done anything, so a cancelled install leaves this file
    /// behind, and whatever launches next must not treat it as a restart —
    /// see `read_from`. Means nothing on an ordinary session.
    pub version: String,
```

Replace `read_from`:

```rust
/// A restart file is only a restart for the build it was written for. Found by
/// any other — the install was cancelled, or a different build came later — it
/// is still what the reader last had open, so it is demoted to an ordinary
/// session rather than dropped, and rewritten so it stays one. An ordinary
/// session is read whatever wrote it.
fn read_from(path: &Path, current: &str) -> Option<Session> {
    let mut session: Session =
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    if session.restart && session.version != current {
        session.restart = false;
        session.argv.clear();
        config::write_json(path, &session);
    }
    Some(session)
}
```

Update the doc comment on `read` — first sentence becomes: `/// What the previous process left behind.` (the rest stays).

Update the doc comment on `discard`, whose second half no longer describes what happens to a leftover snapshot:

```rust
/// Used up, or thrown away: a restored session must not come back on every
/// launch after this one. A snapshot whose install never happened is not this
/// function's business — `read_from` demotes it.
```

- [ ] **Step 3b: A failed install puts the ordinary session back at once (`update.rs:135-143`)**

The snapshot overwrote the ordinary session, and the failure path only deletes. Until some window next reports, there is no session on disk at all. Add one call after `installing` is cleared (it must come after: `remember` refuses to write while `installing` is set):

```rust
    update.install(bytes).map_err(|e| {
        // The restart is off, so the file it was for goes, and the ordinary
        // saves the snapshot had stopped start again — beginning with one
        // now, since the snapshot wrote over the last of them.
        crate::session::discard();
        app.state::<crate::AppState>()
            .installing
            .store(false, std::sync::atomic::Ordering::SeqCst);
        // This is an async command, so not the main thread — and ordinary
        // saves only happen there (Global Constraints).
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || crate::session::remember(&handle));
        e.to_string()
    })?;
```

- [ ] **Step 4: Run, expect pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked session::`
Expected: all `session::tests` PASS.

- [ ] **Step 5: Gates, commit**

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings
git add src-tauri/src/session.rs src-tauri/src/update.rs
git commit -m "Keep the session across a cancelled or manual update"
```

---

### Task 2: The `reopen` value — validated, cached, and written on the way back from off (#6, #9)

**Files:**
- Modify: `src-tauri/src/config.rs:79-84` (`load`), tests
- Modify: `src-tauri/src/main.rs:60-` (`AppState`), `:903-915` (`set_reopen`), `:942-945` (`picker_dir`), `:1248` (setup)
- Modify: `src-tauri/src/session.rs:196-199` (`remember`)

**Interfaces:**
- Produces: `config::reopen_mode(raw: &str) -> &'static str` (one of `"restore" | "ask" | "off"`); `AppState.reopen: Mutex<String>`, filled in `setup` and by `set_reopen`. Task 3 relies on `session::remember(&AppHandle)` being cheap — it runs at every close and again after every grace. It is still main-thread only (Global Constraints).

- [ ] **Step 1: Failing test in `config.rs` tests**

```rust
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
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked config::` → FAIL to compile (`reopen_mode` not found).

- [ ] **Step 2: Implement in `config.rs`**

Below `DEFAULT_REOPEN`:

```rust
/// The reopen setting as one of the three values the app acts on. The file is
/// the user's to edit, and every reader of this compares strings.
pub fn reopen_mode(raw: &str) -> &'static str {
    match raw {
        "restore" => "restore",
        "off" => "off",
        _ => DEFAULT_REOPEN,
    }
}
```

Replace `load`:

```rust
pub fn load() -> Config {
    let mut cfg: Config = std::fs::read_to_string(file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    cfg.reopen = reopen_mode(&cfg.reopen).to_string();
    cfg
}
```

Run the config tests → PASS.

- [ ] **Step 3: Cache the mode in `AppState` (`main.rs`)**

Add to `struct AppState`, after `offered`:

```rust
    /// The reopen setting, kept here because `session::remember` asks on every
    /// report — every scroll-stop in every window — and the config is a file.
    /// Filled in `setup`, kept current by `set_reopen`.
    reopen: Mutex<String>,
```

In `setup`, replace `let reopen = config::load().reopen;` with:

```rust
            let reopen = config::load().reopen;
            *state.reopen.lock().unwrap() = reopen.clone();
```

In `session.rs`, `remember` — replace the first four lines of the body:

```rust
    let state = app.state::<AppState>();
    if *state.reopen.lock().unwrap() == "off" {
        return;
    }
```

(and delete the now-duplicate `let state = app.state::<AppState>();` that followed). If `config` is no longer used for `load` in `session.rs` it is still used for `dir`/`write_json`; leave the import.

- [ ] **Step 4: `set_reopen` validates, updates the cache, and writes**

```rust
#[tauri::command]
fn set_reopen(app: AppHandle, state: State<AppState>, mode: String) {
    let mode = config::reopen_mode(&mode).to_string();
    // Turning it off is meant to be felt now rather than at the next launch:
    // the file on disk is exactly what the reader has just said not to keep,
    // and so is the offer any window is still holding out.
    if mode == "off" {
        session::discard();
        *state.offered.lock().unwrap() = None;
    }
    let mut cfg = config::load();
    cfg.reopen = mode.clone();
    config::save(&cfg);
    *state.reopen.lock().unwrap() = mode;
    // Turning it back on has to be felt now too. Nothing was written while it
    // was off, and the next report may never come — the reader can quit from
    // here without touching a tab.
    session::remember(&app);
}
```

- [ ] **Step 5: `picker_dir` off the main thread**

```rust
/// Where a picker should open, given what the window has on screen.
///
/// Async only to get off the main thread: `start_dir` stats a folder that may
/// be on a share that has gone away, and a synchronous command would hold
/// every window's event loop for as long as the share takes to answer.
#[tauri::command(async)]
fn picker_dir(dir: String) -> String {
    config::start_dir(&dir, &config::load().last_folder)
}
```

- [ ] **Step 6: Gates, commit**

Run all four gates (Global Constraints). Expected: PASS.

```bash
git add src-tauri/src/config.rs src-tauri/src/main.rs src-tauri/src/session.rs
git commit -m "Take the reopen setting at its word, and at once"
```

---

### Task 3: A window closed on purpose stays closed (#4, close half)

**Design: the closed chain** (agreed with the t4-git-ui session, which landed the same rule set in its commit `3fcf6f5`). A window that closes is not forgotten at once: its tabs and frame move to a *chain* of recent closes, and every save during the grace writes the open windows **plus** the chain. When `CLOSE_GRACE` passes with no further close, the chain is dropped and the next save holds only what is open. A quit — every window going within the grace of the one before — ends with all of them in the chain, and all of them on disk.

Why not a counter and a single delayed save (this task's first draft): with nothing written at close, any report from a surviving window inside the grace rewrote the file without the closed one. Whether a window came back then depended on whether another window happened to report — and a live reload makes a window report with nobody touching it.

**Files:**
- Modify: `src-tauri/src/session.rs` — `CLOSE_GRACE`, `chain_expired`, `restorable`, `note_frame`, `window_closed`, `remember`, `save`, tests
- Modify: `src-tauri/src/main.rs` — `AppState` (two fields), `on_window_event` (`CloseRequested` arm, `Destroyed` arm at `:1221-1239`), `set_reopen` (from Task 2)

**Interfaces:**
- Consumes: `session::remember(&AppHandle)` and `AppState.reopen` from Task 2.
- Produces:
  - `AppState.closed: Mutex<Vec<(Instant, session::WindowSession)>>` — the chain, oldest close first
  - `AppState.frames: Mutex<HashMap<String, session::Frame>>` — each window's last known frame, by label
  - `session::CLOSE_GRACE: Duration`
  - `session::note_frame(app: &AppHandle, label: &str)`
  - `session::window_closed(app: &AppHandle, label: &str)`
  - `save` falls back to the cached frame when the live window gives none. Task 6 relies on this: a minimized window keeps its place without the frontend having to know.

- [ ] **Step 1: Failing tests (`session.rs` tests)**

```rust
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
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked session::` → FAIL to compile (`chain_expired`, `restorable`, `CLOSE_GRACE` not found).

- [ ] **Step 2: State (`main.rs`, `struct AppState`, after `installing`)**

`main.rs` imports nothing from `std::time` yet: add `use std::time::Instant;` after the `std::sync` imports.

```rust
    /// Windows that closed within the grace of one another, oldest first, as
    /// they stood when they went. Written out with the open ones until the
    /// grace is over — see `session::window_closed`.
    closed: Mutex<Vec<(Instant, session::WindowSession)>>,
    /// Where each window last stood. A window that has closed cannot be asked,
    /// and one that is minimized gives no answer worth keeping.
    frames: Mutex<HashMap<String, session::Frame>>,
```

- [ ] **Step 3: The pure half (`session.rs`)**

Below `REPORT_TIMEOUT`:

```rust
/// How long a closed window stays in the saved session. Quitting is every
/// window closing one after another, and each close restarts this clock — so
/// a quit ends with all of them still written down, and a window closed on
/// purpose while the app carries on drops out once this has passed.
pub const CLOSE_GRACE: Duration = Duration::from_secs(4);

/// How far past the grace the follow-up save runs, so that it lands on the far
/// side of `chain_expired`'s comparison rather than on it.
const GRACE_MARGIN: Duration = Duration::from_millis(50);
```

Below `reported`:

```rust
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
```

Run the session tests → PASS.

- [ ] **Step 4: `save` and `remember` (`session.rs`)**

`save` — the frame falls back to the cache and refreshes it, and an ordinary save appends the chain. Replace from `let windows = windows.into_iter().map(...)` to the end of the function:

```rust
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
```

`remember` — expire the chain first, and "nothing to write" now means nothing open *and* nothing in the chain. Replace the `nothing_open` block:

```rust
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
```

Its doc comment's second paragraph becomes:

```rust
/// Nothing is written when nothing is open and no window has just closed: the
/// reader closing their last tab must leave the waiting session where it is
/// rather than replacing it with the nothing that follows it.
```

and replace its first paragraph's "Called whenever a window reports a change, because nothing fires as the app quits" with:

```rust
/// Called whenever a window reports a change, when one closes, and once more
/// when the grace after a close is over — nothing fires as the app quits, so
/// the last write before the end is what comes back.
```

- [ ] **Step 5: `note_frame` and `window_closed` (`session.rs`, below `remember`)**

```rust
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
        // Back on the main thread for the write itself — see "Ordinary saves
        // run on the main thread". Every one of these does the same thing, so
        // a run of closes leaves nothing to cancel.
        let app = handle.clone();
        let _ = handle.run_on_main_thread(move || remember(&app));
    });
}
```

(`chain_expired(.., true)`: the closing window has a tab — the early return above guarantees it — so "something is open" holds by construction.)

- [ ] **Step 6: Window events and `set_reopen` (`main.rs`)**

In `on_window_event`, add an arm above `Destroyed`:

```rust
                // The last moment the window can say where it stands; by
                // `Destroyed` it is gone, and the chain needs its frame.
                WindowEvent::CloseRequested { .. } => {
                    session::note_frame(window.app_handle(), window.label());
                }
```

In the `Destroyed` arm, replace `state.sessions.lock().unwrap().remove(label);` with the call below, **keeping it where that line was** (before `focus_order` is trimmed, after the watches and `boot` cleanup), and replace the whole "Nothing is written here…" comment:

```rust
                    // Out of `sessions` and into the chain of recent closes,
                    // and written down with it still in — see `window_closed`.
                    session::window_closed(window.app_handle(), label);
```

`session::reported(&state, label);` stays as it is.

In `set_reopen`'s `if mode == "off"` block (Task 2), also clear the chain:

```rust
        state.closed.lock().unwrap().clear();
```

- [ ] **Step 7: Gates**

Run all four gates. Expected: PASS.

**Cases.** Expected behaviour, for the implementer and the reviewer to check against. `{…}` is what `session.json` holds.

| # | Sequence | At the close | After the grace | Next launch |
|---|---|---|---|---|
| 1 | Close B, keep using A | {A, B} | {A} | A only |
| 2 | Close B, close A under 4s later (quit by hand) | {A, B}, then {A, B} again with A's latest tabs | Process has gone | A + B, with B last in the file — in front, as far as boot order allows (each restored window raises itself when it finishes loading; that predates this plan) |
| 3 | Close B, wait over 4s, close A | {A, B} → {A} at 4s; A's close drops the old chain and writes {A} | — | A only |
| 4 | Close C, B, A, each under 4s apart | {A, B, C} each time | Process has gone | All 3, in the order they stood |
| 5 | Close C, 1s later close B, keep A | {A, B, C} both times | {A}, 4s after **B**'s close | A only |
| 6 | macOS Cmd+Q | Every window joins the chain as it goes | Process has gone | All |
| 7 | Close B (has tabs); A is an empty window | {B} | Still {B}: nothing open has a tab, so the chain waits | B comes back |
| 7b | …then open a document in A, 4s or more after the close | — | {A} at A's next save | A only |
| 8 | Close B, then A reports inside the grace (scroll, tab change, live reload) | {A, B} — A's report rewrites it **with B still in** | {A} | A only; A + B if the app goes inside the grace |
| 8b | Close B, close a tab in A, close A, all inside 4s | {A, B} with A's new tabs | Process has gone | A (new tabs) + B |
| 9 | Close B, open a new window C inside the grace | {A, C, B} | {A, C} | A + C |
| 10 | `reopen = off` | Nothing chained, nothing written | — | Empty |
| 11 | Update installing | Chained, but `remember` returns early; the snapshot leaves the chain out | — | Update restore, without B |
| 12 | Close B, process killed inside the grace | {A, B} | — | A + B. Accepted: inside the grace a close is not yet known to be deliberate. |

- Row 3 is the heuristic's known ceiling: a quit by hand with more than 4s between closes reads as a deliberate close. Tunable through `CLOSE_GRACE`; raising it delays row 1 by the same amount.
- Row 7 is the standing rule that closing the last thing open leaves the waiting session alone, now said once in `chain_expired`.
- Row 8 is what the chain is for. In the first draft A's report dropped B at once.
- Accepted: a window whose last tab was dragged out may close before its "now empty" report lands, and joins the chain still holding that tab. The tab is then in the file twice for up to 4s, and restored twice only if the app goes inside them.

- [ ] **Step 8: Verify in the running app (drive-app skill)**

Set `"reopen": "restore"` in `%APPDATA%\t4-markdown-viewer\config.json` (back the file up first).

Everything here can be driven from `main`'s page, so no second CDP target is needed (pick `main` with `CDP_PAGE` — see Execution, Phase 0a):
- open a window: `invoke("open_window", { path: "F:/…/README.md" })`; an empty one: `invoke("open_window", {})`
- close another window: `window.__TAURI__.window.Window.getByLabel("w2").then((w) => w.close())` (`allow-close` is already granted)
- close `main` last, when a check calls for it: `appWindow.close()` — the process goes with it, so relaunch for the next check
- labels count up (`w1`, `w2`, …) for the life of the process; read them with `window.__TAURI__.window.getAllWindows().then((ws) => ws.map((w) => w.label))`
- force a report from `main`: `reportSession()`

**Start every check clean.** In `restore` mode each launch brings back whatever the check before it left in `session.json`. Delete that file before every launch here (the backup was taken in Phase 0b), and launch with no file argument, so `main` always starts empty.

1. **Row 1.** Launch, open `README.md` in `main` (`openTab(path)`); open a second window on another file. Close it. Read `session.json` at once → **two** windows. Wait 5s, read again → **one**.
2. **Row 2.** Open the second window again. Close it, then `main`, within 2s. Read `session.json` → **two** windows, the second window's entry **last**, and its `frame` present.
3. **Row 5.** Three windows, each with a document. Close the third, close the second 1s later. Read at once → **three**. Wait 5s → **one**.
4. **Row 7.** Leave `main` empty and open a second window on a document, so the window that closes is not the one being driven. Close the second window, wait 5s. Read `session.json` → **one** window: the closed one.
5. **Row 8.** Two windows with documents. Close the second; inside 1s run `reportSession()` in `main`; read `session.json` at once → **two** windows. Wait 5s → **one**.
6. **Row 9.** Two windows with documents. Close the second, open a new one within 3s. Read at once → **three**. Wait 5s → **two**: the first and the new one.

Not driven: row 6 needs macOS; rows 10 and 11 are early returns; rows 3, 4, 8b and 12 are the rows above again with a later close, more windows, or a kill.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/main.rs src-tauri/src/session.rs
git commit -m "Leave a window closed on purpose out of the next launch"
```

---

### Task 4: A file opened at launch stays in front of the restored windows (#7)

**Files:**
- Modify: `src-tauri/src/session.rs` — new `surface`, `same_path`, tests
- Modify: `src-tauri/src/main.rs` — setup (`:1262-1287`), `restore_session` (`:1038-1061`), `session_payload` (`:1065-1072`), its two callers in `restore_offered_session` (`:933`, `:938`)
- Modify: `src/app.js:2605-2611` (end of `main()`)

**Interfaces:**
- Produces: `session::surface(windows: &mut Vec<WindowSession>, path: &str) -> bool`; `restore_session(app, windows, held: bool)`; `session_payload(w, behind_main: bool)`; payload field `"behind": "main"` read by the frontend.

- [ ] **Step 1: Failing test (`session.rs` tests)**

```rust
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
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked session::` → FAIL to compile.

- [ ] **Step 2: Implement in `session.rs`** (below `impl Frame`, above `WindowSession` is fine; keep with the other free functions if preferred)

```rust
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
```

Run the session tests → PASS.

- [ ] **Step 3: `session_payload` and `restore_session` (`main.rs`)**

```rust
/// What a restored window opens with. `maximized` is left to the frontend to
/// act on once it has shown the window — see `session::Frame::apply`. `behind`
/// names the window to hand the front back to once this one is up: each
/// window raises itself as it finishes booting, and one restored beside a file
/// the reader just opened would otherwise bury it.
fn session_payload(w: &session::WindowSession, behind_main: bool) -> Value {
    let mut payload = json!({
        "kind": "session",
        "tabs": w.open.tabs,
        "active": w.open.active,
        "maximized": w.frame.as_ref().is_some_and(|f| f.maximized),
    });
    if behind_main {
        payload["behind"] = json!("main");
    }
    payload
}
```

`restore_session` gains a `held` flag; only *spawned* windows stand behind, never `main` itself. Whether they do is decided by what happened to `main`, not by argv — macOS hands a double-clicked file over as `RunEvent::Opened`, never as an argument, and `open_path` has claimed `main` for it before `setup` runs. A failed claim is the one signal every platform gives:

```rust
fn restore_session(app: &AppHandle, windows: Vec<session::WindowSession>, held: bool) {
    let state = app.state::<AppState>();
    let mut windows = windows.into_iter();
    let first = windows.next();

    // `main` is the file the reader opened in two cases: a saved window that
    // already had it was surfaced into first place (`held`), or the file got
    // there before this did and the claim below fails. Either way the rest
    // stand behind it.
    let claimed = first
        .as_ref()
        .is_some_and(|w| claim_pending(&state, "main", session_payload(w, false)));
    let behind_main = held || (first.is_some() && !claimed);

    let spawn = |w: session::WindowSession| {
        let pending = session_payload(&w, behind_main);
        spawn_window(
            app,
            Some(pending),
            w.frame.map_or(Placement::Default, Placement::Frame),
            None,
        );
    };

    if let Some(w) = first {
        if claimed {
            if let (Some(frame), Some(main)) = (&w.frame, app.get_webview_window("main")) {
                frame.apply(&main);
            }
        } else {
            spawn(w);
        }
    }
    windows.for_each(spawn);
}
```

Add one sentence to its doc comment: `/// When \`main\` is showing a file the reader opened — its own window, or the saved one that already had it — every other window stands behind it.`

Known gap, accepted: on macOS the *duplicate* half of #7 is not fixed. The file has claimed `main` before `setup` can look for it in the session, and un-claiming it is not worth the machinery. The focus half is fixed everywhere.

In `restore_offered_session`, both calls become `session_payload(&w, false)` / `session_payload(&first, false)`.

- [ ] **Step 4: Setup (`main.rs`)** — replace from `let session = saved.filter(...)` through the `if let Some(s) = session { ... }` block:

```rust
            let mut session = saved.filter(|_| restart || reopen != "off");

            // (existing `echoed` comment and computation stay exactly as they are)
            let echoed = session
                .as_ref()
                .is_some_and(|s| !s.argv.is_empty() && s.argv[..] == args[1..]);

            // Windows and Linux never fire RunEvent::Opened; a cold
            // file-association open arrives as argv. `main` exists by now but
            // its webview does not, so this stashes rather than emits.
            let file = file_from_args(&args).filter(|_| !echoed);
            // A file the windows about to come back already have open is
            // brought forward there instead of being opened a second time.
            // Compared in the form a tab records: canonical, without the UNC
            // prefix — see `load_file`.
            let held = !offering
                && match (&file, session.as_mut()) {
                    (Some(path), Some(s)) => {
                        let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
                        session::surface(&mut s.windows, &strip_unc(&path))
                    }
                    _ => false,
                };
            if let Some(path) = file.as_ref().filter(|_| !held) {
                open_path(app.handle(), path);
            }

            if let Some(s) = session {
                if offering {
                    offer_session(&state, s);
                } else {
                    restore_session(app.handle(), s.windows, held);
                }
            }
```

- [ ] **Step 5: Frontend hands the front back (`app.js`, end of `main()`)**

Replace the `setFocus` line and its comment:

```js
  // Showing does not raise a window whose process is in the background, and a
  // window created for a file the user just opened has to land in front. One
  // restored *beside* such a file says so in `behind`, and raises that window
  // instead of itself — whichever of the two finishes booting last, the file
  // the reader asked for ends up on top.
  const front = pending?.behind
    ? await window.__TAURI__.window.Window.getByLabel(pending.behind)
    : null;
  await (front ?? appWindow).setFocus().catch(() => {});
```

- [ ] **Step 6: Gates**

Run all four gates. Expected: PASS.

- [ ] **Step 7: Verify in the running app (drive-app skill)**

"On top" is read from the OS, not guessed from a screenshot: the native title is `<file> — Markdown Viewer` (set by `updateChrome`), so ask PowerShell for the foreground window's title via `GetForegroundWindow` + `GetWindowText` a few seconds after launch, once every window has booted. Windows can refuse the foreground to an app launched from a background shell, in which case that title is the terminal's or the editor's: fall back to the z-order among the app's own windows (`EnumWindows` lists top-level windows front to back; keep the ones owned by the debug PID), and if that is ambiguous too, report the check as inconclusive rather than passed or failed.

With `"reopen": "restore"` and a saved session of two windows (neither showing `CHANGELOG.md`, one showing `README.md`):
1. Launch the debug exe with `CHANGELOG.md` as its argument → three windows, `CHANGELOG.md`'s on top and focused.
2. Launch with `README.md` as its argument → two windows, no duplicate tab, the `README.md` window on top with that tab active.
3. If `setFocus` on another window's handle is denied by the capability (console shows a permission error), add nothing to the capability — instead report back: the fallback is a two-line Rust command `raise(label)` calling the existing `focus_window`.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/session.rs src-tauri/src/main.rs src/app.js
git commit -m "Keep the file you opened in front of the windows that come back"
```

---

### Task 5: Scroll is only banked for the document on screen (#2)

**Files:**
- Modify: `src/app.js:201` (beside `renderToken`), `:562-620` (`showActive`), `:2482-2497` (scroll listener)

**Interfaces:**
- Produces: module-level `let shownToken`. No other task reads it.

- [ ] **Step 1: Reproduce first (drive-app skill)**

`app.js` is a plain script, so its functions are page globals. Open two long documents in two tabs; scroll tab A to ~1500px, tab B stays at 0. With tab A active, run in the page as **one** `eval` (the second statement fires the scroll event inside the gap the bug needs — `activateTab` has switched `activeId` and is parked on `load_file`):

```js
(async () => {
  const switching = activateTab(tabs[1].id);
  window.dispatchEvent(new Event("scroll"));
  await switching;
  await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  return window.scrollY;
})()
```

Expected before the fix: ~1500 (B opened at A's offset). This is the failing check. Tabs are activated on pointer-up through the drag code, not on `click`, which is why this calls `activateTab` rather than clicking the strip.

- [ ] **Step 2: Implement**

Beside `let renderToken = 0;`:

```js
/**
 * The render whose document is on screen. Behind `renderToken` from the moment
 * a switch begins until what it asked for is shown — and for that long the
 * page still belongs to the entry being left, not the active one.
 */
let shownToken = 0;
```

In `showActive`, after the `try/catch` and before `syncWatch();`:

```js
  shownToken = token;
```

(Every superseded render has already returned above; an error panel counts as shown.)

In the scroll listener, guard the body and extend the comment:

```js
  // ...existing comment stays, then add:
  // Not while a switch is loading, though: the active entry has already moved
  // on and the old document is still what is scrolling, so banking now would
  // hand the new entry the old one's offset.
  window.addEventListener(
    "scroll",
    () => {
      if (shownToken !== renderToken) return;
      rememberScroll();
      scheduleReport();
    },
    { passive: true },
  );
```

Every path that changes the active entry and then loads (`openTab`, `activateTab`, `removeTab`, `reopenClosed`, `adoptTab`, `loadPath`, `go`, `refresh`) calls `showActive` in the same tick, and `showActive` bumps `renderToken` before its first `await` — so there is no tick in which the entry has moved and the guard is still open. Same-document moves (`go` between anchors) do not load and are not guarded, which is right: the document on screen *is* theirs.

- [ ] **Step 3: Re-run Step 1's check**

Expected: `window.scrollY` is 0 (B's own position). Also confirm ordinary behaviour: scroll B to 800, switch to A and back → B returns to 800.

- [ ] **Step 4: Gate, commit**

Run: `node --check src/app.js` → no output.

```bash
git add src/app.js
git commit -m "Bank the reading position for the page that is actually showing"
```

---

### Task 6: Tab changes are written at once; moving, resizing and focus are written at all (#3, #4 frame and focus half)

**Files:**
- Modify: `src/app.js:622-639` (`updateChrome`), `:737-761` (`reportSession`, `REPORT_DELAY`, `scheduleReport`), end of `main()` (after the focus lines from Task 4)

**Interfaces:**
- Consumes: `reportSession()`, `scheduleReport()` (existing).

Why not simply report on every `updateChrome`: a live reload ends in `showActive` → `updateChrome` too, and the watcher's debounce is 150ms, so a file being rewritten continuously would mean six synchronous commands and six file writes a second. So: at once when it has been quiet, debounced when it has not.

- [ ] **Step 1: `reportSession` notes when it ran and cancels a pending debounce**

Move the `REPORT_DELAY` / `let reportTimer = null;` declarations *above* `reportSession`, add `lastReport`, and reword the comment:

```js
/** How long reports are held apart, so a run of changes costs one write. */
const REPORT_DELAY = 500;
let reportTimer = null;
let lastReport = 0;
```

```js
function reportSession() {
  clearTimeout(reportTimer);
  lastReport = Date.now();
  return invoke("set_session", {
    tabs: tabs.map(packTab),
    active: Math.max(0, tabs.findIndex((t) => t.id === activeId)),
  }).catch(console.error);
}
```

`scheduleReport` keeps its body; its comment becomes:

```js
/** Report a moment after things settle — for the changes that come in runs. */
```

Below it, new:

```js
/**
 * Report now if it has been quiet, and a moment after things settle if it has
 * not. Nothing fires as the last window goes, so a tab closed just before
 * quitting has to be on disk already or it comes back — but a document being
 * rewritten under the reader lands here several times a second, and that must
 * not become several writes a second.
 */
function reportSoon() {
  if (Date.now() - lastReport >= REPORT_DELAY) reportSession();
  else scheduleReport();
}
```

Residual, accepted: a tab change inside the 500ms after some other report is still debounced, and is lost if the app is killed inside that window.

- [ ] **Step 2: `updateChrome` uses it**

Replace its last statement (`scheduleReport();`) and extend the comment:

```js
  // The funnel every tab change already reaches: opening, activating, closing
  // and navigating all end up here, so the saved session follows them without
  // each of them having to remember to say so. Without delay when it can be:
  // the reader can quit inside one.
  reportSoon();
```

- [ ] **Step 3: Frame and focus changes schedule a report**

At the very end of `main()`, after the focus hand-back and before `checkUpdate`:

```js
  // Where the window stands, and which one was in front, are saved with the
  // tabs, and nothing else notices them changing. Maximize arrives as a
  // resize; so does minimize, and Rust answers that one with the frame it last
  // had. Registered only now: a report fired while the tabs are still being
  // restored would say this window has none, and `Frame::apply` resizes it
  // before they are.
  window.addEventListener("resize", scheduleReport);
  window.addEventListener("focus", scheduleReport);
  appWindow.onMoved(scheduleReport).catch(console.error);
```

Two things Task 3 made possible here. A minimized window needs no guard on this side: `save` falls back to the cached frame. And saving on focus is safe: the save that fires when a sibling closes and this window takes focus still carries the closed window, in the chain. (The `tauri://move` listener is inside `core:default`; no capability change.)

- [ ] **Step 4: Gate**

Run: `node --check src/app.js` → no output.

- [ ] **Step 5: Verify in the running app (drive-app skill)**

1. Open two tabs, wait 1s (so the last report is over 500ms old), then in one Bash call: `closeTab(tabs[1].id)` via `eval`, `sleep 0.1`, `taskkill //PID <pid> //F`. `session.json` → one tab. (The 100ms lets the command cross to Rust; the unfixed build, which waits 500ms before even sending it, still fails this.)
2. Relaunch. Move and resize with Win32 `MoveWindow` from PowerShell (`allow-set-size` is not granted — see the skill). Wait 1s, read `session.json` → `frame` has the new `x`/`y`/`width`/`height`. Maximize (`appWindow.maximize()`), wait 1s → `"maximized": true`.
3. Minimize (`ShowWindow` with `SW_MINIMIZE` from PowerShell), wait 1s → `frame` is **still the previous one**, not `null`.
4. Live reload does not flood: with a document open, rewrite the file ten times 200ms apart from Bash while counting `set_session` calls — wrap it first with `eval`: `window.__n = 0; const real = reportSession; reportSession = () => (window.__n++, real());`. Expected `__n` ≤ 5: `reportSoon` is a 500ms throttle, so a 2s burst gives at most two reports a second plus the trailing one (unthrottled it would be ten).
5. Focus order: two windows with documents. From `main`'s page, `getByLabel("w1").then((w) => w.setFocus())`, wait 1s, read `session.json` → `w1`'s window is **last**. Then `appWindow.setFocus()`, wait 1s → `main`'s is last. Windows can refuse focus to an app that was launched from a background shell; if the order never changes, check `document.hasFocus()` in each page and report the check as inconclusive rather than failed.
6. Console shows no permission error from `onMoved`.

- [ ] **Step 6: Commit**

```bash
git add src/app.js
git commit -m "Write the session down before there is time to quit"
```

---

### Task 7: Mac Ctrl+click is a right-click and nothing else (#10)

**Files:**
- Modify: `src/app.js:1090-1097` (`onTreeClick`), `:1131-1140` (`onTreeContextMenu`)

- [ ] **Step 1: `onTreeClick`** — replace the comment and the three-way branch:

```js
  // Ctrl+click opens beside the current document rather than in its place, and
  // Shift+click gives it a window of its own, as in a browser. Plain click
  // walks the active tab's history like a link. On the Mac the tab modifier is
  // Cmd alone: Ctrl+click there is the right-click that raises the menu, and a
  // webview that sends the click as well must not also open the file under it.
  if (isMac && event.ctrlKey) return;
  if (event.shiftKey) await invoke("open_window", { path });
  else if (event.metaKey || event.ctrlKey) await openTab(path);
  else await loadPath(path);
```

- [ ] **Step 2: `onTreeContextMenu`** — the pointer test learns about the Mac's Ctrl+click, which reports button 0 like the keyboard does. Append to the comment above it: `A Mac Ctrl+click reports no button either, but it does have a point.`

```js
  const fromKeyboard = event.button !== 2 && !(isMac && event.ctrlKey);
```

- [ ] **Step 3: Gate, commit**

Run: `node --check src/app.js` → no output. Not verifiable on this machine (Windows); on Windows confirm Ctrl+click still opens a new tab and right-click still places the menu at the pointer. Say in the hand-off that the macOS path is untested.

```bash
git add src/app.js
git commit -m "Let a Mac Ctrl+click raise the menu without opening the file"
```

---

### Task 8: The fade band ends where the text begins (#8)

**Files:**
- Modify: `src/base.css:298-312`, `:1429-1433`

- [ ] **Step 1: Band height follows the content's top padding**

In the comment, replace "This fades it into the page colour over the last 3.5rem." with:

```css
 * clean in half. This fades it into the page colour over the height of the
 * content's own top padding — so a document at rest starts exactly where the
 * band ends, and only text that has scrolled up into it is faded.
```

and the two declarations:

```css
  height: var(--content-pad);
  margin-bottom: calc(-1 * var(--content-pad));
```

- [ ] **Step 2: Anchor jumps land below the band**

```css
/* Anchor targets clear the sticky bar — and the fade band under it — instead
   of hiding behind them. `--bar-h` comes from measureBar(), since the bar
   grows when the tab strip shows. */
.markdown-body :target {
  scroll-margin-top: calc(var(--bar-h, 3rem) + var(--content-pad));
}
```

- [ ] **Step 3: Verify in the running app (drive-app skill)**

Screenshot, light and dark theme: (a) a document at `scrollY = 0` — first line at full contrast; (b) after clicking a TOC link — the target heading at full contrast, just under the band; (c) mid-scroll — text still fades into the bar with no visible bottom edge. Confirm `--content-pad` resolves on `main` (DevTools computed style of `main::before` height = 40px at default font size).

- [ ] **Step 4: Commit**

```bash
git add src/base.css
git commit -m "End the fade under the bar where the text begins"
```

---

### Task 9: Full gates and hand-off

- [ ] **Step 1:** Run all four gates from Global Constraints on the final tree. Paste the output.
- [ ] **Step 2:** Restore the backed-up `config.json` / `session.json`.
- [ ] **Step 3:** Report: what was verified by test, what by driving the app, and what could not be verified here (Task 7 on macOS; Task 4's focus order on macOS/Linux). Do not push.
