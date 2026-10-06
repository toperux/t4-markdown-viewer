# Tauri 2.12.1 Implementation Plan

> **Status (2026-10-07):** executing. Task 1 done (b4407b7, lock hash `ca17bb28…0292`); the
> change review found nothing. Task 2 passed on Linux (198 tests) and the Mac (195; Clippy
> clean, so 2.12.1 fixes PR #9's failure). W1–W11 passed on all three platforms, with no
> regression from the new crates. Task 4 done. Triage done (owner, 2026-10-07): 14 items, all
> pre-existing — seven deferred to `open-items.md` (three to the session-restore plan, next;
> two to the tear-off placement work, with the macOS 32 pt item, whose trigger fired and which
> stays deferred), drive-app's `sendkeys.ps1` guard fixed (1f10e5f), updater 2.13.1's install
> made a First runs check, three open accepted limits, one closed. Next: Task 5.

> **For agentic workers:** Execute per the **Execution** section below. Steps use checkbox
> (`- [ ]`) syntax for tracking.

**Goal:** Move the app to tauri 2.12.1 and the plugin releases that go with it, replacing
Dependabot PR #9, whose tauri 2.12.0 fails to compile the tests on macOS.

**Architecture:** No source change is expected. One `cargo update` of the Tauri crates rewrites
`src-tauri/Cargo.lock`; `Cargo.toml` already allows every new version (`"2"`, and
`"2.5.4"` for opener). The Tauri CLI that packages the release stays at 2.11.4, so the bundler,
the AppImage tool pins and the installer templates don't change. A release dry run proves that
CLI packages the new crates, and walks on all three platforms cover what the crates change
under the app.

**Tech Stack:** Rust + Tauri v2: tauri 2.11.6 → 2.12.1, tao 0.35.3 → 0.37.1, wry 0.55.1 →
0.57.0. `drive-app` for the walks.

**Spec:** the owner's go of 2026-10-06 on the proposal: take tauri 2.12.1 and the plugin crates,
keep tauri-cli 2.11.4 and prove it with a release dry run, walk on all three platforms; the CLI
bump is a separate, later decision.

**Prior art:** t4-git-ui did the same move on 2026-10-04/05,
`../t4-git-ui/docs/archive/plans/2026-10-04-tauri-2.12-plan.md` (its commits `44c9da5`, the
lock, and `5204425`, npm and the CLI). Its findings about tauri, tao and wry are folded in below
where they touch this app. Its npm and CLI work doesn't apply here: this app has no
`package.json`, and the CLI stays where it is.

## Why not merge PR #9

PR #9 (Dependabot, 2026-10-01) moves tauri to **2.12.0** with 5 other crates. Its CI fails on
the macOS leg only (run 36911229040):

```
error[E0046]: not all trait items implemented, missing: `transparent`
  --> tauri-2.12.0/src/test/mock_runtime.rs:354:1
```

- `tauri::test`'s mock window builder in 2.12.0 lacks `transparent` on macOS. The app compiles
  that mock because its dev-dependencies turn on tauri's `test` feature (`Cargo.toml:33`), for
  `routing_tests.rs`. t4-git-ui doesn't, which is why its own 2.12 PR was green.
- tauri 2.12.1 fixes it: its notes say "the `macos-private-api` feature flag … is no longer
  required to use transparency or fullscreen on macOS" (#16166), and its
  `src/test/mock_runtime.rs:438` has `fn transparent` with no platform condition (measured in
  the crate source). That 2.12.1 compiles on macOS is reasoned from that; Task 2's gates on
  the Mac measure it, before any push.
- Dependabot proposed 2.12.0 because it is what the PR was opened against; 2.12.1 came out on
  crates.io on 2026-10-01, the same day.

## What moves (measured with `cargo update --dry-run`, 2026-10-06)

The targeted update (Decision 1) moves:

| Crate | Now | After |
|---|---|---|
| `tauri` | 2.11.6 | 2.12.1 |
| `tauri-build`, `-codegen`, `-macros`, `-plugin` | 2.6.3 | 2.7.1 |
| `tauri-runtime`, `tauri-runtime-wry` | 2.11.3, 2.11.4 | 2.12.1 |
| `tauri-utils` | 2.9.3 | 2.10.1 |
| `tauri-plugin-dialog` | 2.7.3 | 2.8.1 |
| `tauri-plugin-fs` (a dependency of dialog) | 2.5.2 | 2.6.0 |
| `tauri-plugin-opener` | 2.5.5 | 2.7.0 |
| `tauri-plugin-single-instance` | 2.4.5 | 2.5.2 |
| `tauri-plugin-updater` | 2.12.0 | 2.13.1 |
| `tao` | 0.35.3 | 0.37.1 |
| `wry` | 0.55.1 | 0.57.0 |
| `muda`, `tray-icon` | 0.19.3, 0.24.2 | 0.20.0, 0.25.1 |

Plus, pulled in by those: `brotli` 9, `cargo_toml` 1, `html5ever`/`markup5ever` 0.39,
`json-patch` 4, `urlpattern` 0.6, `window-vibrancy` 0.8, `windows` 0.62 and `webview2-com`
0.39 (newly in the tree), and the removal of `dirs` 6, `toml` 0.9, the `unic-*` crates and
`winnow`: 59 entries in all. The app's own `dirs` 7 is unchanged.

## What changes under the app

From the tauri, tao, wry and plugin release notes, and t4-git-ui's reading of the plugin
sources. Each item names where this app meets it, and which walk (Task 3) covers it.

**tauri 2.12.0 / 2.12.1:**
- **Monitor queries now run on the main thread** (#15630). `Frame::apply`
  (`session.rs:122-124`) calls `available_monitors` from the thread `spawn_window` starts
  (`main.rs:393`), so the call now waits for the event loop. Nothing on the main thread waits
  for that thread, so no deadlock (reasoned). Walk W1.
- **Listeners bound to a window are removed when it is destroyed** (#15617, #15604), and
  channel payloads are bound to their webview (a security fix). The Rust side registers no
  listeners outside the tests. The pages listen with `listen` (the `Any` target) for the
  broadcasts (`open-mode-changed`, `diagram-colours-changed`, `themes-changed`;
  `app.js:4507-4512`) and with a window-addressed helper (`app.js:4473-4480`) for the tab-drag
  events Rust sends with `emit_to` (`main.rs:518, 1100-1135`). Closing one window must leave the
  others hearing both. Walk W3.
- **`asset://` files load off the event loop** (#16050), and multi-range responses are fixed
  (#15838). Every image in a document goes through `convertFileSrc` (`app.js:339`) and the
  asset protocol (`tauri.conf.json:22-24`). Walk W7. No fixture has video or audio, so the range
  fix is not walked.
- **`WindowEvent::Focused` is no longer sent while dragging a window on Windows** (#15625). The
  app keeps window order from `Focused(true)` (`main.rs:1727`, `touch_focus`), which the session
  saves least-recently-focused first. Walk W1 checks the order after a restore.
- **MSRV 1.90.** Local Rust is 1.99; CI uses `stable`. The Linux VM's and the Mac's toolchains
  are checked before their builds (Task 2).
- `windows` 0.62 drops Windows 7. Tauri 2 already needs Windows 10 for WebView2, so nothing
  changes (reasoned).
- Not used by the app: app-bound domains, `appDirectoriesOverride`, the JS `exit`,
  `set_fullscreen_on_monitor`, Liquid Glass, the ACL deny fix (the app has no deny
  permissions).

**tao 0.36 / 0.37:**
- **Windows: a hidden window's maximize is delayed, and its size and position reads change**
  (0.37.0 #1306). The app builds every window hidden (`main.rs:400`), sets its position and size
  while hidden (`Frame::apply`), and leaves maximizing to the page after it shows
  (`session.rs:115-118`). Walk W1 restores one maximized and one normal window.
- **Windows: `WM_ENDSESSION` now exits the process** (0.37.0 #1157). A shutdown already ended
  the app with no page asked (the closed accepted limit *A kill, a shutdown or a Quit from
  outside the app loses the last 500 ms*); `session.json` is written as the reader works, so
  nothing new is lost (reasoned). Not walked: a shutdown can't be scripted on the walking
  machines.
- **Windows: keyboard and IME internals refactored** (0.36.0 #1238, #1215), "no behaviour
  changes" per the notes. Walk W9 runs the app's shortcuts.
- **Windows: visibility flicker fixes** (0.37.1 #1341).
- **Linux: thread-safety fixes for min/max size** (0.36.0 #1173); Wayland decoration fixes.
  The app sets a minimum size (`main.rs:399`).
- **macOS:** no change listed that the app touches. Deferred item *A macOS tab tear-off
  probably lands about 32 pt high* depends on tao's `inner_position`; walk W2 on the Mac
  records where a tear-off lands, which may settle it either way.

**wry 0.56 / 0.57:** IPC handler no longer panics on an odd document URL; `window.ipc` is
injected only into webviews built with an IPC handler (Tauri always sets one); a Windows
teardown crash fix. Wry #1593 (deferred item *A subframe navigating to the app's own page runs
`stay`'s reload branch*) is still open (checked 2026-10-06), so that item is unchanged. The
diagram frame is walked in W8.

**Plugins** (t4-git-ui's reading of the sources, 2026-10-05):
- **updater 2.13.1:** on Linux, `check()` no longer sets `SSL_CERT_FILE`/`SSL_CERT_DIR` to
  Debian paths when they are unset. Debian-family hosts are unaffected; other distributions are
  not walked. The install path only runs for real at the next release's update (Risks). Walk W11
  checks for updates.
- **single-instance 2.5.2, dialog 2.8.1, opener 2.7.0:** formatting and refactors; no API or
  permission change the app uses. Walks W4, W5, W6.
- **single-instance still forwards argv with `std::env::args()`** in 2.5.2
  (`platform_impl/linux.rs:77`, `macos.rs:88`, `windows.rs:91`; measured in the crate source).
  `closed-items.md:290-291` asks for this check on each bump: 009d8c8's Linux re-exec, which
  makes every argument Unicode before the plugin sees it, stays. W4's Linux row runs it.
- **muda 0.20 / tray-icon 0.25:** the macOS menu bar is built with muda (`macos_menu`). Walk W10.

## Decisions

1. **A targeted update, not a full `cargo update`** (owner, 2026-10-06). Targeted moves the Tauri crates and what they pull in (the table above). A full
   `cargo update` also moves about 70 unrelated crates (`tokio`, `rustls`, `hyper`, `reqwest`,
   `open` 5.4.4, `thiserror` 2.0.21, …), measured by `cargo update --dry-run`. Targeted keeps
   the change to what the walks cover, and matches what Dependabot proposed; the rest can be its
   own small change.
2. **The Tauri CLI stays at 2.11.4** (owner, 2026-10-06). Moving it brings bundler 2.10.1 and
   the changes t4-git-ui had to rule on (a renamed linuxdeploy, the AppImage no longer forcing
   X11, bundled GIO modules, the NSIS running-app check). `release.yml:139` and *Pin the AppImage
   tools* are untouched. That CLI 2.11.4 packages tauri 2.12.1 crates is not known; the dry run
   (Task 5) measures it. If it fails, stop and bring it back to the owner.
3. **PR #9 is closed, not merged.** Dependabot closes its own PR as "no longer updatable" once
   `main` has the versions (t4-git-ui's #20 went that way). If it hasn't by the time this is
   pushed, close it by hand with a comment naming the commit (asked first, like any action on
   GitHub).
4. **The commit is `chore:`**, so it stays off the release page (the `dependabot.yml` comment
   gives the same reason). Whether a release follows, and what titles it, is asked after triage.
5. **The *First runs* check *Dependabot under SHA pinning* is ticked** (owner, 2026-10-06,
   option (a)). Its pass condition is "the Dependabot Updates job passes, and any PR it opens
   passes `checks.yml`". The job passed on 2026-10-01 (runs 36910987673 and 36910984666). PR
   #9's CI (run 36911229040) passed in full on Windows and Linux; on macOS every pinned action
   ran and the *Clippy* step (`--all-targets`, so it compiles the tests) failed on tauri
   2.12.0's own bug, not on the pinning (measured in the run's jobs and log). Options: (a) tick it
   with that note, since what it checks (SHA pinning) worked; (b) leave it for the next
   Dependabot PR to pass outright.

## Tasks

### Task 1: Take the crates

**Files:**
- Modify: `src-tauri/Cargo.lock` (only)

- [ ] **Step 1: Update**

```sh
cargo update --manifest-path src-tauri/Cargo.toml -p tauri -p tauri-build -p tauri-plugin-dialog -p tauri-plugin-single-instance -p tauri-plugin-opener -p tauri-plugin-updater
```

Expected: the moves in *What moves*, with `tauri v2.11.6 -> v2.12.1`. A different set means
crates.io moved since 2026-10-06: compare, and bring any new major or minor move to the owner
before going on.

- [ ] **Step 2: Gates** (`run-cargo-fmt-before-committing`: CI formats on Linux only)

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: no diff, no warning, 195 passed. A deprecation warning from the new crates fails
clippy here: fix it in the same commit, the smallest change that clears it, and say so in the
commit body.

- [ ] **Step 3: Check the diff is the lock alone**

```sh
git status --short
```

Expected: ` M src-tauri/Cargo.lock` (plus any file Step 2 had to fix).

- [ ] **Step 4: Record the lock's hash** for the Linux and Mac sessions to match (Task 2), with
  carriage returns stripped so a checkout's line endings can't differ:

```sh
tr -d '\r' < src-tauri/Cargo.lock | sha256sum
```

- [ ] **Step 5: Commit locally**

```
chore: take Tauri 2.12.1 and its plugins

Replaces Dependabot #9, whose tauri 2.12.0 fails to compile the tests on
macOS: its mock window builder lacks `transparent` there (fixed in 2.12.1,
#16166). tauri 2.12.1, tauri-build 2.7.1, dialog 2.8.1, opener 2.7.0,
single-instance 2.5.2, updater 2.13.1; transitively tao 0.37.1 and
wry 0.57.0. The CLI stays at 2.11.4.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

### Task 2: The same lock on Linux and the Mac

Nothing is pushed for this: each machine makes the lock itself. Through Remote Control
sessions (`remote-platform-walks`), a self-contained brief each.

- [ ] **Step 1:** `rustc --version` ≥ 1.90 (tauri 2.12's MSRV); update the toolchain if not.
- [ ] **Step 2:** `git pull` on `main`, which must be at the same commit as this machine's
  `main` before Task 1 (`git rev-parse HEAD`, compared). Then Task 1's Step 1 command.
- [ ] **Step 3:** Task 1's Step 4 hash command. Expected: the hash Task 1 recorded. A different
  hash means crates.io moved in between, or the base differs: stop and report, don't walk.
- [ ] **Step 4:** Task 1's Step 2 gates. Expected: no diff, no warning, no test failed (the
  count differs by platform: some tests are Linux-only). On the Mac this is the proof,
  before any push, that 2.12.1 fixes PR #9's failure (*Clippy* compiles the tests).
- [ ] **Step 5:** after the walks, discard the local lock and any applied fix patch
  (`git status --short` first: only those files may show; then `git restore .`), so the
  machine's `main` is clean for the pull after the push.

### Task 3: Walks

W1–W11 run on debug builds before the push: Windows on this machine with `drive-app` (Task 1's
commit); Linux on the VM and macOS on the Mac on Task 2's lock, through the same Remote Control
sessions (the Mac's built with `--features webdriver` for `drive-app`). W12 runs on the dry
run's builds after the push (Task 5). Each machine's `session.json` and `config.json` backed up
before and restored after.

| Walk | What | Pass | Where |
|---|---|---|---|
| W1 | Settings › Reopening on *Restore*. Two windows: one maximized, one normal at a set size and place, each with tabs scrolled down. Close the app the ordinary way (each window closed; Cmd+Q on the Mac), relaunch. | Each back where it stood, the maximized one maximized, un-maximizing it gives a normal size, scroll kept, window order kept. | all 3 |
| W2 | Tear a tab off into a new window; drag a tab into another window. | New window under the cursor; the tab moves. On the Mac, record how far from the cursor the new window lands (deferred 32 pt item). | all 3 |
| W3 | Three windows, `examples/kitchen-sink.md` open in two; close the third. In one remaining window, switch Settings between *Diagrams follow the theme* and *Diagrams in Mermaid's own colours* (`index.html:172-173`; broadcast by `set_diagram_colours`, `main.rs:1197-1204`); then drag a tab between the two remaining windows. | The other window redraws its diagram in the new colours; the drag shows its drop marker and the tab moves (`emit_to`). | all 3 |
| W4 | With the app open, open a `.md` from the file manager / a second launch. | It opens in the running app; no second process stays. Linux: a second instance on `$'caf\xe9.md'` exits 0 with the first window untouched (009d8c8's re-exec). | all 3 |
| W5 | Open (Ctrl/Cmd+O) and the sidebar's Select Folder. | Both dialogs open, the pick opens. | all 3 |
| W6 | Click an `https://` link in a document. | The browser opens it; the app stays. | all 3 |
| W7 | `examples/kitchen-sink.md`: the image `img/icon.png`; a document with a missing image path. | The image shows; the missing one shows broken, the window stays responsive. | all 3 |
| W8 | `examples/kitchen-sink.md`'s diagram: draws, a click opens the overlay; F8 redraws it. | Drawn, overlay opens, redraw in the new theme, no second window. | all 3 |
| W9 | README's shortcut table (`README.md:229-241`): Ctrl/Cmd+O, +T, +N, +W, +Shift+T, +Tab, +Shift+Tab, +Shift+O, +Shift+F, +, (Settings), F5, F8 and Shift+F8, Alt+← / →, and Ctrl/Cmd++ / − / 0 on an open picture. | Each does what the table says. | all 3 |
| W10 | macOS menu: Cmd+Q right after a scroll (1.7.2's fix), Edit › Copy of selected text, Window › Close Window. | Quit keeps the scroll; copy reaches the clipboard; the window closes. | Mac |
| W11 | Settings › Updates › *Check now* (`index.html:212-213`). | It answers: up to date on 1.7.2; no error. | all 3 |
| W12 | After the push: the dry run's build, installed (Windows installer, Linux AppImage, macOS `.dmg`): launch, open a file. Linux: also W4's `$'caf\xe9.md'` second instance on the AppImage. | Starts, opens, no crash; the AppImage's second instance exits 0. | all 3 |

W12 replaces the installed app with an unreleased build that also calls itself 1.7.2, so the
next real release still updates it. On every machine that is the owner's installed copy: asked
before installing on each.

Every finding goes straight to the owner, as it is found.

### Task 4: Records

- [ ] **Step 1:** `docs/open-items.md` → *First runs* → *Dependabot under SHA pinning*: tick it
  (Decision 5), noting the Updates runs 36910987673 and 36910984666, and that PR #9's CI
  (36911229040) failed on macOS on tauri 2.12.0's mock, not on the pinning. Every sub-item is
  then ticked, so move the whole item to `docs/closed-items.md`, at the end of *Needs a Mac, a
  Dependabot run, or an older build* (closed-items has no *First runs* section; that one holds
  the earlier run checks), and drop the emptied *First runs* section from `open-items.md`.
- [ ] **Step 2:** If W2 on the Mac settles the 32 pt tear-off item, update or close it with the
  measurement. If anything else in the walks bears on a deferred item, note it there.
- [ ] **Step 3:** Commit locally as `docs:`.

### Task 5: Push, dry run, W12

After triage. Each push and each dispatch is asked for first (`CLAUDE.md`, step 7).

- [ ] **Step 1: Push `main`** (Task 1's and Task 4's commits, and any fix). Expected: CI green
  on all three legs.
- [ ] **Step 2: Dispatch the Release workflow on `main`** (`workflow_dispatch`; the `release`
  skill's *Checking the packaging without burning a version*); the owner approves `signing`, or
  says to approve it by `gh api` (the skill's *Signing*). Expected: every leg builds and signs
  with CLI 2.11.4, the publish rehearsal matches `dist/`, and the `dry-run-*` draft is deleted.
  Keep the run id; its installer, AppImage and `.dmg` are what W12 installs.
- [ ] **Step 3:** if any leg fails in *Build the app* or *Bundle and sign*, stop: that is
  Decision 2's unknown, and goes back to the owner.
- [ ] **Step 4: W12** on all three, each install asked for first. A finding goes back through
  fix → change review → triage before any release.
- [ ] **Step 5:** File this plan under `docs/archive/plans/` with its status line filled in
  (the dry run's id, W12's results); commit locally as `docs:`, and ask before pushing it.

## Verify

- Task 1's gates on Windows and Task 2's on Linux and the Mac, before any push; Task 5's CI on
  all three; the dry run green with CLI 2.11.4.
- The walks, each row passed or brought to the owner.
- `tauri-plugin-webdriver` (optional, `drive-app` on macOS) builds against tauri 2.12.1: the
  Mac's debug build for W1–W11 is built with `--features webdriver`, so the walk proves it.

## Risks

- A window, restore or keyboard regression from tao 0.37 that only a walk shows: W1, W2, W9.
- CLI 2.11.4 failing to package 2.12.1 crates: the dry run catches it before any tag.
- **Updater 2.13.1's install path is unproven until the next release** after this lands: a
  broken one would strand every install on that version, with the manual download as the way
  out. t4-git-ui accepted the same (its D6 (a)). Proposed for acceptance at triage, with the
  reopen trigger *the first in-app update from a build carrying updater 2.13.1*.
- Linux update checks on non-Debian distributions (updater 2.13's TLS roots), which no walk
  covers. Proposed for acceptance at triage, trigger *a report of a failing update check on a
  non-Debian distribution*.
- Video and audio range requests (#15838) unwalked: no fixture has either.

## Execution

In this order, on `main` (`commit-straight-to-main`):

1. Task 1, in the main session (one lock file and three commands; no `coder` needed).
2. Task 3's W1–W11 on Windows, in the main session through `drive-app`.
3. Task 2 and Task 3's W1–W11 on Linux and the Mac, through Remote Control sessions, findings
   relayed to the owner. Can run alongside step 2.
4. The change review loop on Task 1's commit (`CLAUDE.md` step 5). A fix is a new local commit,
   through fix → change review, and the walks it touches run again; it reaches the Linux and
   Mac sessions as a `git format-patch` file in the brief, applied over Task 2's lock.
5. Task 4, then triage, one item at a time (`CLAUDE.md` step 6).
6. Task 5, after triage, as `CLAUDE.md` step 7 orders a push. The one exception is W12: the dry
   run dispatches only from `main` (the `signing` environment), so the installed builds can only
   be walked after the push. Its findings get their own fix → review → triage round.
7. The release question.
