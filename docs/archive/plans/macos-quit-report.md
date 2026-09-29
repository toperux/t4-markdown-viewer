# macOS Quit Keeps the Last Report Implementation Plan

> **Status (2026-09-29):** executed in full — ef15e73 (Task 1); the owner accepted Decision 3
> at triage, with no reopen trigger. Task 2 passed first time, with no retry: Q1 exited
> 337 ms after the scroll with 3000 kept, Q2 quit from the menu, and Q3 quit at once with the
> Open panel up.
> Results are in `../../closed-items.md` under *Needs a Mac, a Dependabot run, or an older
> build*. Deviations: the tests take "well under" to mean under `REPORT_TIMEOUT / 2`; in Q3 the
> panel opened on the README window, which was key once the app was brought to the front,
> rather than on the window just scrolled. Paths, item titles and unticked boxes below are as
> they stood when it was written.

> **For agentic workers:** Execute per the **Execution** section below. Steps use checkbox
> (`- [ ]`) syntax for tracking.

**Goal:** On macOS, Cmd+Q keeps what changed in the last 500 ms before it, as closing a window
already does.

**Architecture:** Replace the predefined Quit (`Item::quit`) in `macos_menu` with a Quit item of
our own on Cmd+Q. It works in three steps:

- It asks every window to report now and waits up to 1 s for the answers. This is the same ask
  and wait the update snapshot already makes (`session::snapshot`).
- Each answer writes `session.json` through the ordinary `set_session` → `remember` path.
- Then it calls `app.exit(0)`.

No window is closed one at a time.

**Tech Stack:** Rust + Tauri v2 (tauri 2.11.6, tauri-runtime-wry 2.11.4, tao 0.35.3), vanilla JS,
`drive-app` (`macos.md`) for the real keystroke.

**Spec:** `docs/open-items.md` → *Needs a Mac* → "Cmd+Q loses the last 500 ms of changes".

## Findings (read from the code and the pinned sources, 2026-09-29)

- **Why Cmd+Q loses it.** The chain runs like this:
  - `Item::quit` sends `terminate:` (muda 0.19.3, `macos/mod.rs:994`).
  - tao handles only `applicationWillTerminate` (`platform_impl/macos/app_delegate.rs:63`),
    and turns it into `RunEvent::Exit`.
  - Nothing asks the pages anything on the way out.

  The owner measured the loss on 2026-09-28; the details are in the open item.
- **The ask-and-wait exists.** `session::snapshot` (`session.rs:231`) does three things:
  - it puts every window whose webview has booted (`boot.ready`) into `awaiting`;
  - it emits `update-installing`;
  - it waits on `reported` for up to `REPORT_TIMEOUT` (1 s) until `awaiting` is empty.

  The page's listener (`app.js:4538`) calls `rememberScroll()` and `reportSession()`.
  `set_session` saves through `remember` before it calls `reported` (`main.rs:1041-1049`), so
  when the wait ends, every answer is already on disk. The 1.6.8 → 1.6.9 update restart ran this
  path (open items, "First runs"), but its answers weren't checked: only the install was
  recorded.
- **An ordinary report writes the file, with three exceptions.** `remember` skips the write
  when no window has a tab and no close is pending (`session.rs:299-301`), when `installing`
  is set during an update, and when the reopen setting is "Start fresh". A quit adds none of
  these, so each answer is saved as an ordinary, non-restart session.
- **`app.exit(0)` ends like the updater's restart.** In tauri 2.11.6, `AppHandle::exit`
  (`app.rs:574`) sends `RequestExit`. tauri-runtime-wry then fires `ExitRequested`, and with
  nothing preventing it (`lib.rs:4354-4366`), `ControlFlow::Exit` follows, then `RunEvent::Exit`,
  then the process exits (tao `event_loop.rs:202`). The app does not handle `ExitRequested`.
  Nothing else depends on `Item::quit`: the updater restarts with `app.restart()`
  (`update.rs:197`).
- **No window is destroyed on the way out** (read from tao 0.35.3; Q1 confirms it). On macOS,
  tao emits `Destroyed` only from `windowWillClose:` (`window_delegate.rs:333-341`).
  `ControlFlow::Exit` sends `NSApp stop:` (`app_state.rs:403-411`). Then `[NSApp run]`
  returns, `AppState::exit` sends `LoopDestroyed` and drops the handler
  (`app_state.rs:272-276`), and the process exits (`event_loop.rs:202`). None of these closes
  a window, so `session::window_closed` never runs and the saved order stays as the last
  report left it. The predefined Quit ends the same way, which fits the order kept on
  2026-09-28.
- **The saved order is the ordinary one.** A report saves through `save`, which lists windows
  least-recently-focused first (`session.rs:398`), exactly as any report during use does.

## Decisions

1. **Report, then exit, rather than closing windows one by one.** The owner chose this
   on 2026-09-29.
   - Review pass 1 found that closing the windows one by one had four problems:
     - a window created mid-quit left the app open;
     - a second Cmd+Q started closes in parallel;
     - a window could be destroyed with its Open panel still attached;
     - the teardown showed on screen.
   - With report-then-exit, none of these can happen.
2. **The wait stays at the update's 1 s (`REPORT_TIMEOUT`).** The owner chose this on
   2026-09-29, over the 2 s a close gets. A page busy for longer keeps its previous report. A
   window that has never reported keeps what it was created to open (`claim_in`,
   `main.rs:338-344`).
3. **A Quit from outside the app (the Dock, the app switcher, Activity Monitor,
   AppleScript), logout and shutdown stay as they are** (proposed; the owner rules at
   triage).
   - These send `terminate:`, and tao gives the app no way to hold it. Holding it would need our
     own `applicationShouldTerminate:` on tao's app delegate class, which is Objective-C runtime
     work outside tao. That isn't worth it for 500 ms.
   - They would join the closed accepted limit "A kill or a Windows shutdown loses the last
     500 ms" in `closed-items.md`, reworded to name them.
4. **The update path's event and listener stay as they are.** The quit gets its own event,
   `quit-requested`, and its own page listener. Renaming `update-installing` to a shared name
   would change the update path, which only a real release can test.
5. **The quit listener mirrors the close path** (owner, 2026-09-29, on review pass 2 #6). It
   calls only `reportSession()`, as `onCloseRequested` does, and is registered next to it
   (`app.js:4611`), once the window has booted. Why:
   - *Boot.* A window counts as ready as soon as it calls `take_pending` (`main.rs:530`), before
     its tabs are back (`app.js:4565ff`). If it answered then, it would report no tabs, and
     the save would drop it. Registered after boot, it doesn't answer. The quit waits out the
     1 s, and the window keeps what it was created to open. The move and resize listeners are
     registered late for the same reason (`app.js:4596`).
   - *Render.* `rememberScroll()` run between a render and its rAF `scrollTo`
     (`app.js:1410-1426`) would record the top over the saved spot. The scroll listener
     (`app.js:4416`) already records every scroll, so the quit doesn't need to.
   - Cost: Cmd+Q during boot takes the full 1 s. Boot can be long: a restore awaits
     `load_file` and `warmDiagrams`, and a saved sidebar awaits the folder listing
     (`app.js:2382`), which is slow on a slow share. If boot throws after `take_pending`,
     for example at `await appWindow.show()` (`app.js:4585`), the window never registers the
     listener, and every Cmd+Q waits 1 s for as long as it's open. The close listener has the
     same gap today. Accepted by the owner on 2026-09-29 (review pass 3 #4).
   - A scroll in the very last frame before Cmd+Q, one the scroll listener hasn't seen yet,
     is lost. That's at most one frame (`app.js:1659-1661`).
6. **Left as they are** (owner, 2026-09-29):
   - A quit during an update install can exit while the bundle is being replaced
     (`update.rs:182`). The predefined Quit had the same exposure.
   - With reopen "off", the quit still waits for answers that write nothing. The pages answer
     in milliseconds.
   - Some comments describe a quit as every window closing one after another:
     `session.rs:29-32`, `318-320`, `376-378`, and `routing_tests.rs:496`. That's true on
     Windows and Linux, and was already untrue for macOS Quit before this change.
7. **The menu text changes** from muda's default "Quit" to "Quit T4 Markdown Viewer" (owner,
   2026-09-29).

## Tasks

### Task 1: Our own Quit item

**Files:** `src-tauri/src/session.rs`, `src-tauri/src/main.rs`, `src/app.js`,
`src-tauri/src/routing_tests.rs`, `README.md`, `src/index.html`

- [ ] `session.rs`: move the middle of `snapshot` into
  `pub async fn gather<R: Runtime>(app: &AppHandle<R>, event: &str)`, generic so the mock
  runtime can test it. `REPORT_TIMEOUT` becomes `pub(crate)`, so the tests can name it. It
  covers filling `awaiting` from
  `boot.ready`, emitting `event`, and the `spawn_blocking` wait. `snapshot` keeps setting
  `installing` first, then calls `gather(app, "update-installing").await`, then saves as before.
  The doc comment on `gather` says it's shared by the update snapshot and the macOS Quit. The
  `REPORT_TIMEOUT` and `AppState::awaiting` comments now name both callers.
- [ ] `main.rs`: add a `QUIT` menu id next to `CLOSE_WINDOW`, under
  `#[cfg(target_os = "macos")]`. The `CLOSE_WINDOW` doc comment currently says "the one item
  that is not predefined"; change it to say both items are custom.
- [ ] `macos_menu`: replace `&Item::quit(app, None)?` with
  `&MenuItem::with_id(app, QUIT, "Quit T4 Markdown Viewer", true, Some("CmdOrCtrl+Q"))?`. The
  doc comment gains one line on why: the predefined Quit exits with no page asked, so its last
  report would be lost. It also says "So this is the default menu minus that collision"
  (`main.rs:1466-1472`). That was already inexact, as the README's count was, so reword it:
  for example, "the default's Edit menu, with Close Window and Quit as our own items".
- [ ] `on_menu_event` (main thread): on `QUIT`, `let app = app.clone();` then
  `tauri::async_runtime::spawn(async move { session::gather(&app, "quit-requested").await;
  app.exit(0); })`. It must not block, because the answers arrive as commands on the main
  thread. A second Cmd+Q during the wait starts a second gather that ends in the same exit; no
  guard is needed.
- [ ] `app.js`: next to `onCloseRequested` (`app.js:4611`), add
  `listen("quit-requested", () => reportSession()).catch(console.error);`. Its comment says:
  only the macOS Quit sends this; it's registered after boot for the same reason as the close
  listener (Decision 5); and a window that doesn't answer keeps its last report.
- [ ] Update the comments that call Cmd+Q unrecoverable: `set_session`'s doc
  (`main.rs:1020-1021`) and `app.js:1852` say "a Cmd+Q or a kill". Change both to "a Quit from
  outside the app (Dock, app switcher) or a kill".
- [ ] Update the comments that say nothing fires on quit, or that name only the snapshot:
  - `session.rs:12`, the module doc: "no hook that fires as the last window goes";
  - `session.rs:264-267`, `remember`: "nothing fires as the app quits";
  - `session.rs:362`, `reported`: "answered `snapshot`";
  - `main.rs:1016`, `set_session`: "once more when `update-installing` asks";
  - `main.rs:1044-1047`, inside `set_session`: "`reported` is what releases a waiting
    snapshot". Add that the macOS Quit exits as soon as the answers are in, so each answer
    must be on disk first.

  Each should say that the macOS Quit asks as well.
- [ ] Two tests for `gather` in `routing_tests.rs`, with the mock runtime and
  `tauri::async_runtime::block_on`:
  - *no ready window*: create `main_window(&app)` without calling `take_pending`. It returns
    at once (well under `REPORT_TIMEOUT`), and `awaiting` is empty, which proves a window
    that hasn't booted isn't waited on (Decision 5).
  - *a ready window answers*: use `remembering_app`, and mark `main` ready with
    `take_pending(app.state(), main.as_ref().window())`. Add a
    `listen_any("quit-requested", …)` whose handler owns a clone of `app.handle()` and of
    `main`, builds `tabs` inside itself (the handler is `Fn`), asserts that
    `handle.state::<AppState>().awaiting` contains `"main"`, and then calls
    `set_session(handle.state(), main.as_ref().window(), tabs, 0, false, false)`.
    The `report` helper borrows the `App`, so it can't be called from inside a `'static`
    handler. The mock's `emit` runs that handler on the test thread, the same thread
    `block_on` polls on, so the save lands in the test's own config folder.
    Then `block_on(gather(app.handle(), "quit-requested"))` must:
    - return well under `REPORT_TIMEOUT`;
    - leave the answer in `saved_paths()`;
    - leave `awaiting` empty.

    This catches three things:
    - `gather` not emitting its `event`, which hits the 1 s timeout;
    - `awaiting` filled after the emit, which fails the handler's assert;
    - `awaiting` never filled at all, which fails the handler's assert. Without it the real app
      would exit before any page answers.

    A mismatch between the name in `main.rs` and the one in `app.js` is caught only by Q1,
    because the wait runs to 1 s and the exit lands past 500 ms.
- [ ] README.md (owner, 2026-09-29):
  - Lines 253-257 say Close Window is "the one departure from the menu Tauri would build by
    default". That was already inaccurate: Tauri's default also has File, View and Help menus
    (`tauri-2.11.6/src/menu/menu.rs:182-240`). Drop the count and keep the reason for the
    Cmd+W binding. Then add: "Quit is replaced as well, so each window can send where it
    stands before the app exits." (owner, 2026-09-29)
  - "Picking up where you left off" (`README.md:345-349`): replace the paragraph with this
    text (owner, 2026-09-29):

    > Which windows are open, where they stand, what each has in its tabs and how far
    > down each document you are: all of it goes to `session.json`, beside
    > `config.json`, **as you work** rather than at quit. Closing a window, or
    > **Quit** from the menu on macOS, sends the last moment first. Anything that
    > gives the app no warning (a crash, a power cut, a `taskkill /F`, a shutdown, or
    > a Quit from the Dock or the app switcher) costs only what changed since things
    > last stood still for half a second.
- [ ] `src/index.html:193-194`, the Settings › Reopening hint, is the in-app twin of the README
  sentence (owner, 2026-09-29). "…so a crash or a power cut costs no more than a quit does."
  becomes "…so a crash or a power cut costs only the last moment."
- [ ] Gates, in `src-tauri`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`.
- [ ] Commit locally with a reader-facing subject and no prefix, because release notes are
  built from unprefixed subjects (`.claude/skills/release/SKILL.md:93-103`). For example:
  "Keep the last moment's reading when quitting with Cmd+Q on macOS".

### Task 2: Prove it on this Mac (`drive-app`, `macos.md`)

Setup:

- the debug build at the Task 1 commit, with `"reopen": "restore"`;
- two windows on documents long enough to scroll, reported and on disk at 1500 (back) and 2400
  (front). This is the same setup as the 2026-09-28 reproduction.
- every relaunch that checks the restore runs with no file argument. `macos.md:82` launches
  with `examples/kitchen-sink.md`, and a file given at launch is moved to first place in the
  restored session (`session::surface`, `session.rs:154-170`), which would skew the order
  and active-tab checks.

Before each run, check that `session.json` has `restoring: false`. A restore that hasn't
settled makes the next launch offer instead of restore.

If the first Cmd+Q after a WebDriver call doesn't quit, as happened once on 2026-09-28 (ruled
noise), record it and start over from a clean baseline. By then the ordinary report has
already saved the target, so first scroll to 2400, wait past 500 ms, and check that
`session.json` holds 2400; or use a new target, such as 3600, and check for that. A second key
press alone would land past 500 ms, and the ordinary report would save the scroll with or
without the fix.

- [ ] **Q1, the fix.** Bring the app to the front and check it (`front()` in `macos.md`
  includes a 0.3 s delay) before the scroll, so only the keystroke's `osascript` sits between
  the scroll and the key. Scroll the front window to 3000, then press a real Cmd+Q within 250 ms
  (System Events `keystroke "q" using {command down}`, with the app in front). Record the delay
  between the scroll and the key. Pass means:
  - the process exits within 500 ms of the scroll: poll the PID to time it. Past that, the
    ordinary report could have saved the scroll anyway;
  - `session.json` holds 3000 for the front window and 1500 for the back one;
  - the windows are saved in their old order (back window first), with `restart: false`;
  - the next launch comes back at 3000, with the windows in their old order.

  If the saved order differs from the order they stood in, which the source says can't happen
  (Findings, "No window is destroyed on the way out"), stop and bring it back to the owner.
- [ ] **Q2, the menu itself.** The item shows ⌘Q, and clicking it quits the app. Click it
  through System Events (`menu bar item 2 of menu bar 1`). An unbundled debug binary's app menu
  may show the executable's name rather than "T4 Markdown Viewer" (reasoned).
- [ ] **Q3, the Open panel up.** Scroll the front window to a new spot and wait past 500 ms.
  Open the Open panel on it (Cmd+O), then press Cmd+Q. Record which outcome happens:
  - the app quits at once;
  - it quits after about a 1 s pause, because the modal panel held back the answers;
  - the panel takes the key, and the app stays open. In that case, cancel the panel
    (`macdialog.sh <pid> --cancel`), press Cmd+Q again, and the app must quit.

  The panel is a sheet: tauri-plugin-dialog 2.7.3's `AsyncFileDialog` (`desktop.rs:148-149`)
  goes through rfd 0.16.0's `beginSheetModalForWindow` (`modal_future.rs:79-99`,
  `panel_ffi.rs:37-40`). It starts no run loop of its own, and a sheet doesn't disable the app
  menu. So the likeliest outcome is a prompt quit with the sheet still up. That is reasoned,
  not measured.

  Every clean outcome passes (owner, 2026-09-29), and the saved scroll is recorded. A hang
  (the app neither quits nor responds) or a crash dialog fails: stop and bring it to the
  owner.

### Task 3: Docs

- [ ] Move the open item to `closed-items.md`, under *Needs a Mac, a Dependabot run, or an
  older build*, with Q1–Q3's results and the commit. Rewrite its "Fix:" sentence
  (`open-items.md:39-41`), which describes the rejected close-every-window design, to say
  what was built: report, then exit. The `## Needs a Mac` heading is then empty: drop it
  (owner, 2026-09-29).
- [ ] Move this plan to `docs/archive/plans/`, add a status line at the top as the archived
  plans have, and cite that path in the closed item.
- [ ] Update `.claude/skills/drive-app/macos.md:228`. Cmd+Q already reached the menu in the
  2026-09-28 run, and Q1 measures it again. Say so there, and keep the other keys as unmeasured.
- [ ] In `open-items.md`, narrow the accepted limit *drive-app on macOS: other menu shortcuts vs
  the page are unmeasured* so it no longer includes Cmd+Q.
- [ ] In every case, replace the last sentence of the closed accepted limit "A kill or a Windows
  shutdown loses the last 500 ms" (`closed-items.md:847-848`). It says the macOS Cmd+Q fix
  is still open under *Needs a Mac*. Say instead that the menu Quit is fixed, and cite the
  closed item and the commit.
- [ ] If the owner agrees to Decision 3 at triage, also reword that limit to name a Quit from
  outside the app (Dock, app switcher) and macOS logout and shutdown.
- [ ] Commit locally with a `docs:` subject.

## Execution

- Task 1: `coder` agent (Opus). The main session reviews the diff and the gate output.
- Task 2: the main session drives the app through `drive-app`. Findings go straight to the
  owner.
- Task 3: the main session, as small doc edits.
- Then the change review loop (CLAUDE.md step 5).
