# Dependabot PRs #24 and #26 Plan

> **Filed (2026-11-16):** executed in full on the owner's go. PR #26 walked on all three VMs
> (*Walk results*), then rebase-merged as `59d1bc7`, its tree identical to the walked `889628d`.
> PR #24 rebase-merged as `5346f6b`. The `checks.yml` runs on both passed on all three legs.
> The dry run 43843298027 passed: three legs built and signed, sixteen files uploaded to the
> draft by the new action, the draft checked and deleted. Triaged (*Triage*): four accepted
> limits with reopen triggers, in `open-items.md`. The tagged publish path then ran for real in
> the 1.9.2 release (run 43860956762): sixteen files on the release page.

**Goal:** land the two open Dependabot pull requests on `main`, so the next release ships with
them:

- **PR #24**, `ci:`: `softprops/action-gh-release` 3.0.3 → 3.0.4, the action that publishes a
  release.
- **PR #26**, `chore:`: four crates in `Cargo.lock`. `notify` 8.2.1 → 8.2.2,
  `tauri-plugin-opener` 2.7.2 → 2.7.3, `open` 5.4.1 → 5.4.5, `tauri-plugin-webdriver` 0.2.3 →
  0.2.4.

**Spec:** the owner's roadmap ("then the two Dependabot PRs"), and the release skill's checks
for a Dependabot pull request. PR #26 replaced PR #25 on 2026-11-16: the link fix (`2458584`)
made `open` a direct dependency, so Dependabot closed #25 (notify and the webdriver plugin) and
opened #26 with `open` and the opener plugin added. No security alert is attached to either, so
nothing forces the timing.

## Findings (2026-11-16)

Measured or read by the main session and the plan's reviewer, unless marked *reasoned*. `main`
is `431c726`.

### PR #24

- One commit, `424e002`, on an older `main` (`98919db`); `release.yml` has not changed since,
  so it rebases cleanly. It changes one line, `release.yml:560`, in the `publish` job: the pin
  `efb35369…` (v3.0.3) becomes `8a0cd651…` (v3.0.4).
- The new pin is the tip of the action's `master` (`compare/master...8a0cd651` says
  `identical`).
- Between the two pins: twelve dependency bumps and the commit `release 3.0.4 (#865)`. Files:
  `dist/index.js` (the built action), `package.json`, `package-lock.json`, `CHANGELOG.md` and
  the action's own workflow. `dist/index.js` is a bundle and was not read.
- One of the bumps is a major one in what the action runs: `glob` 13 → 14, the library that
  resolves `files: dist/*` (`release.yml:561`). So "every built file is attached" is the thing
  to see before a tag.
- Its checks passed, but they run `checks.yml` only. The `publish` job runs on a tag or on a
  dry run (`workflow_dispatch` on `main`), so the release skill asks for a dry run before the
  next tag. A dry run uses the `signing` environment, which the owner approves by hand. It
  rehearses a draft; the tagged, public path (`draft`, `tag_name`, `target_commitish`,
  `release.yml:563-565`) first runs on a real release, as for any bump of this action.

### PR #26

- One commit, `889628d`, on `431c726`. It changes `src-tauri/Cargo.lock` only (54 lines added,
  168 removed). Its checks build with `--locked` and passed on all three platforms.
- Every package in the new lockfile names `registry+https://github.com/rust-lang/crates.io-index`
  (568 of 568). The four `.crate` files, downloaded from crates.io, hash to the lockfile's
  checksums.
- Four packages change version. Eleven go away: `webview2-com` and `webview2-com-sys` 0.38.2,
  `windows` 0.61.3, `windows-core` 0.61.2 and seven more of that generation. Only the webdriver
  plugin 0.2.3 needed them; 0.2.4 uses the 0.62 generation the rest of the tree is on. The
  other changed lines are dependency names losing a version suffix that told two copies apart.
- **None of the eleven is in the shipped build** (`cargo tree --target all -e normal,build`
  without the `webdriver` feature, on `main`). So what ships changes in three crates: `notify`,
  `open` and `tauri-plugin-opener`.

### The bumps

Read as source diffs between the two versions.

| Bump | What changed upstream | Consequence here |
| --- | --- | --- |
| `notify` 8.2.1 → 8.2.2, **Windows** | Stopping a watch could re-arm a read on a handle that was already closed, and deadlock a thread that opened a folder just then, when Windows reused the handle (notify #1051). `stop_watch` now closes the handles only after `CancelIo` succeeds and the queued completion has run; if `CancelIo` fails it leaks the handle, a semaphore and the read buffer, and logs. Dropping a watcher now wakes its server thread before telling it to stop. `windows.rs` +218 −7, most of it two new tests | `watch.rs` builds one watcher for a window's open files and one for its sidebar folders, and drops the old one when the set changes: a tab opened or closed, a link followed or Back and Forward (once, or twice when the document's pictures differ, `watch.rs:36-41`), a folder expanded or collapsed (`app.js:3183-3184`), a window closed (`main.rs:2174-2177`). A switch between tabs leaves the file watcher alone, the set of files being the same (`app.js:2464-2467`), and rebuilds the folder watcher only when the other tab's folder is not the one on show (`followTab`, `app.js:2719-2750`). The deadlock would show as a folder that never lists or a document that never loads, not as a frozen window. Upstream needed twelve threads in a tight loop for 20 to 50 s to hit it, so no walk by hand reaches the race on either version: the walk is a smoke of the ordinary stop path. The app has no logger, so the leak's log line goes nowhere; the walk counts handles instead |
| `notify`, **macOS and Linux** | One format string in `inotify.rs`; `fsevent.rs` identical. Minimum Rust 1.85 → 1.88 | None in the app. CI builds on `stable`; a peer's own `rustc` must be 1.88 or newer to build the walk |
| `open` 5.4.1 → 5.4.5, **Linux** | `commands()` returns six launchers, not four: `kde-open6` and `kde-open5` are new, before `kde-open`. The first three (`xdg-open`, `gio open`, `gnome-open`) are built as before, arguments included. `that_detached` now watches each launcher for 250 ms | The app walks `commands()` itself (`open_url`) and never calls `that_detached` on Linux, so only the longer list reaches it: a Plasma 6 desktop without `xdg-open` gets a launcher it lacked. The first launcher that starts still wins; with none, the last one's error is shown. A comment in the new source says the crate leaves an AppImage's environment alone on purpose and points callers at `commands()`, which is what the app does |
| `open`, **macOS** | `that_detached` now runs `/usr/bin/open` and waits for it, where it started it through a middle process nothing waited for. Upstream: the old way left a defunct child per call. A failing `open` is now an error | Links on macOS go through the opener plugin's `open_url`, which calls `that_detached` (`open.rs:12`). So a link no longer leaves a defunct child (*reasoned*; the same fault the Linux triage fixed). And where `open` fails, the app now shows a toast, where it showed nothing: `Launcher "/usr/bin/open" "--" "<url>" failed with ExitStatus(unix_wait_status(256))` (worked out from the source, not seen). The app passes on only `http`, `https`, `mailto` and `tel` links, and a stock Mac has a program for each (FaceTime takes `tel:`; *from knowledge, not measured on the owner's Mac*), so this is likely out of a walk's reach |
| `open`, **Windows** | The PowerShell launcher's script changed. With the `shellexecute-on-windows` feature, `that_detached` is the Shell API call, untouched | The plugin turns that feature on (`tauri-plugin-opener` `Cargo.toml:68`), so the path a link takes is unchanged (*read*). One click in the walk confirms it |
| `tauri-plugin-opener` 2.7.2 → 2.7.3 | Its page script, which opens `target="_blank"` links and Ctrl- or Shift-clicked links in the browser, now also follows a `<base target="_blank">`. In Rust, a comment and the version | The page has no `<base>`, and a document cannot add one: raw HTML is dropped (`render.rs:292`). Showing a file in the file manager (`reveal_path`) uses another part of the plugin, unchanged. See *Found on the way* for the script itself |
| `tauri-plugin-webdriver` 0.2.3 → 0.2.4 | Built against the `windows` 0.62 crates; needs Tauri 2.12.1 or newer (its release note) | Behind the `webdriver` feature, which only drive-app on macOS builds. Never in a release. The macOS walk builds with it, which proves it compiles and drives |

### Found on the way, not from these PRs

The opener plugin's page script is on (`tauri_plugin_opener::init()`, `main.rs:2102`), and since
the link fix (`2458584`) the page has no permission for the command it calls
(`plugin:opener|open_url`). The script skips a click that is already cancelled, and the app
cancels every click on a web or mail link before it: in the document (`onLinkClick`,
`app.js:5333`, bound on the document's pane, which a click reaches before the window) and in
the full-window diagram view (`app.js:6317`). No other link exists in the app's own pages, and
the script does not run in the diagram frame.

One case gets through, on Windows only (*read, not measured*). A "back to top" link, `#` or
`#top` in a document with no heading of that name, is left to the webview (`app.js:5326`). On
Windows the page's address is `http://tauri.localhost`, so with Ctrl or Shift held the script
takes the click for a web link, cancels it and calls its command, which is refused: the click
does nothing. On 1.9.1 and older the command was allowed, so the same click opened the browser
at `http://tauri.localhost/#top`, a page that does not exist (*reasoned*). macOS and Linux use
`tauri://localhost`, which the script ignores.

Turning the script off is one line (`Builder::new().open_js_links_on_click(false)`), a code
change outside these PRs; what the webview then does with that click is not known. The walk
measures today's behaviour (P5). To put to the owner at triage.

## Global constraints

- No dependency change beyond the two PRs. No local edit to the Cargo files or the workflow:
  each PR's commit goes in as it is.
- No AI attribution. No push, merge, dry run or delete without the owner's go at that moment.
- Walks run on the peers (Windows VM, Linux VM, Mac), never on the host PC, from PR #26's own
  commit `889628d`, which is on origin. No `walk/*` branch is needed. Each peer works in a
  worktree under `.claude/worktrees/`, and backs up and restores `config.json` and
  `session.json` per the drive-app skill.
- Each peer first reads `rustc --version`. Older than 1.88: it stops and asks the owner there
  before updating anything.
- On the Mac, links are checked with Safari (through Choosy) and Apple Mail, never Edge.
- Dependabot's next weekly run is due about 2026-11-19. Its cargo group takes every crate
  (`patterns: ["*"]`), so a new version of any direct dependency by then rewrites PR #26's
  branch, and a 3.0.5 of the action rewrites PR #24's. The walked commit is then no longer what
  would merge.

## Task 1: walks of PR #26

Debug build of `889628d`.

**Windows** (the watcher's changed code; a smoke of the ordinary stop path, see *The bumps*):

- **P1, live reload:** a document open; edit and save the file on disk. The page shows the edit.
- **P2, stopping watches:** the sidebar on the repo's root with `docs` and `docs/fixtures`
  expanded (they stay so), `examples/kitchen-sink.md` open. Twenty rounds of:
  1. expand the `docs/fixtures/seq` row, then collapse it (a click each);
  2. open `docs/fixtures/b2-links.md` in a second tab (`openTab` through `eval`, or a click on
     its row with Ctrl held; a plain click would replace the first document);
  3. follow its `[doc](b2-other.md)` link (a click);
  4. go Back (a click on the Back button);
  5. close that tab.

  Pass: the expand lists the folder's two files, and each of steps 2 to 4 shows its document,
  within 2 s (polled through `eval`); after rounds 5, 10, 15 and 20, P1 passes on
  `kitchen-sink.md`. Read the process's thread count and `HandleCount` (`Get-Process`) before
  round 1 and after rounds 5, 10, 15 and 20, and report all five pairs. A count that rises at
  every reading is a finding.
- **P3, a second window:** open a second window with a document and a folder, then close it.
  The first window still answers, and P1 passes in it.
- **P4, quit:** quit with a folder and two documents open. The process is gone within 5 s.
- **P5, links:** an `https` link opens the browser; a `mailto` link reaches the mail program or
  Windows' chooser, as on 2026-11-14. Report only: in a scratch document with a `[top](#top)`
  link and no heading named "top", what a plain click, a Ctrl-click and a Shift-click on it do
  (scroll, nothing, a browser window, a new app window), on `889628d` and on a debug build of
  `c55db94` (1.9.1; same plugin, same early return, the permission still there). The two
  modified clicks are real input (CDP `Input.dispatchMouseEvent` with `modifiers` 2 and 8): a
  click made by script would not show what a user gets. Also report `location.origin`, which
  the case rests on (`http://tauri.localhost` expected).

**macOS** (`open`'s changed path, and the webdriver plugin):

- **P6, links:** an `https` link opens Safari, a `mailto` link a Mail draft.
- **P7, no child left:** after each, `ps -axo ppid,stat,comm` shows no defunct child of the app.
  Report the same for a debug build of `431c726`, where one per link is expected.
- **P8, a link nothing opens:** first find out, without clicking, whether a program takes
  `tel:` (NSWorkspace's `URLForApplicationToOpenURL`). If one does, report which and skip: the
  failing path then stays unwalked, a limit for triage. If none does, click a `tel:` link on
  `889628d` and on `431c726`, and report what the app shows on each, read at once (a toast
  hides after 4 s).
- **P9, live reload:** P1 once.

**Linux** (the longer launcher list), under Xvfb with the stub handler, as on 2026-11-16:

- **P10, links:** an `https` and a `mailto` link reach the stub; no defunct child of the app.
- **P11, no launcher:** the app started with `PATH` set to a folder without `xdg-open`, `gio`,
  `gnome-open` and every `kde-open*`. drive-app starts the binary itself, so it is handed a
  wrapper script (`#!/bin/sh`, then `PATH=<folder> exec <the binary's full path> "$@"`);
  *reasoned, not tried*: if it does not start that way, the peer reports how it did start it.
  An `https` link gives the toast `No program to open links could be started: No such file or
  directory (os error 2)`, and nothing starts. A toast hides after 4 s, so it is read at once.
  This also walks that wording, which was compiled but not walked.
- **P12, a new launcher:** as P11, the folder also holding a stub named `kde-open6` that logs
  its arguments to a file named by full path, using only what `/bin/sh` has built in. An
  `https` link reaches it as `-- <the URL>`.
- **P13, live reload:** P1 once.

Not walked: the Linux AppImage. It would need an image built by GitHub from a `walk/*` branch.
The code a link runs is the same in a debug build, the first three launchers are built exactly
as before, and the new ones come after them (*read*). A limit for triage.

## Task 2: merge PR #26

On every walk passing, and the owner's go. The go names the admin bypass: the ruleset on `main`
asks for an approving review, and the owner's account bypasses it, as for PR #23.

1. `git fetch`. `gh pr view 26 --json headRefOid` still says `889628d4…`, and
   `git rev-parse origin/main` is still `431c726`. Otherwise stop.
2. `gh pr merge 26 --rebase --admin --match-head-commit
   889628d4cc6bcd889148a1b52d7603473a22c923`, which keeps Dependabot's `chore:` subject.
3. `git fetch`. `git diff 889628d origin/main` prints nothing. The `checks.yml` run on `main`
   passes.
4. Local `main` carries this plan as a `docs:` commit, so it is rebased onto `origin/main`
   (`git rebase origin/main`), not pulled.

## Task 3: merge PR #24, then a dry run

On the owner's go for each step, the admin bypass named as in Task 2:

1. `git fetch`. `origin/main` is still PR #26's merge commit.
   `gh pr view 24 --json headRefOid` still says `424e0022…`, and the pin is still on the
   action's `master` (`gh api
   repos/softprops/action-gh-release/compare/master...8a0cd651e5b2365bd640854a3fa5dfc70e4875f3
   --jq .status` says `identical` or `behind`). Otherwise stop.
2. `gh pr merge 24 --rebase --admin --match-head-commit
   424e0022608ff4394a376a7a89ac0aee0e906478`. `git fetch`. `git show --stat origin/main` names
   `release.yml` only, one line. The `checks.yml` run on `main` passes. Local `main` is rebased
   as in Task 2.
3. A dry run: `workflow_dispatch` of the Release workflow on `main`. The owner approves the
   `signing` environment. Pass: every leg builds, the draft's assets match `dist/` (the check
   that covers the action's new `glob`), and the draft is deleted. It also packages PR #26's
   crates on all three platforms.
4. If the dry run fails in the publish steps: stop and report. Reverting PR #24 is the way
   back, on the owner's go.

A PR's branch stays on origin after the merge. Deleting each is its own ask.

## Task 4: close out

- Triage with the owner, one item at a time: any walk finding; the macOS toast for a link
  nothing opens (P8); the page script and P5's report (*Found on the way*); the AppImage not
  being walked; the watcher race being out of a walk's reach.
- Archive this plan to `docs/archive/plans/` with a status line giving the merge commits and
  the walks' results, plus the open-items entries the triage leaves. One `docs:` commit, local;
  pushed on the owner's go.

## Execution

The main session briefs the peers for Task 1 and runs Tasks 2 to 4. Serial. Stop and ask on:

- Either PR's head, or `main`, having moved.
- Any walk step failing, or behaving differently between runs.
- A check or the dry run failing.

## Walk results (2026-11-16)

Debug builds of `889628d`; `rustc` 1.99.0 on all three. All measured.

| Check | Result |
| --- | --- |
| P1 | Windows: pass, the edit shown in 192 ms, the revert in 182 ms. |
| P2 | Windows: pass, 20 of 20 rounds. The expand listed both files in 0 to 67 ms; the tab, the link and Back showed their documents in 0 to 66 ms; P1 after rounds 5, 10, 15 and 20 in 180 to 246 ms. Threads and handles before round 1 and after rounds 5, 10, 15, 20: 23/538, 23/536, 23/536, 23/536, 23/536. |
| P3 | Windows: pass. With the second window open 28 threads and 566 handles; after it closed 23 and 541, and P1 in the first window took 187 ms. |
| P4 | Windows: pass, the process gone 138 ms after quit. |
| P5 | Windows: pass. `https` opened Edge; `mailto` opened Windows' "Pick an app" chooser. The `#top` report: `location.origin` is `http://tauri.localhost` on both builds. A plain click scrolls to the top on both. Ctrl- and Shift-click on `889628d`: nothing visible, an uncaught "opener.open_url not allowed". On `c55db94` (1.9.1): each opens an Edge tab at `http://tauri.localhost/#top`, "can't reach this page". No new app window on either. |
| P6 | macOS: pass. `https` opened Safari through Choosy, `mailto` a Mail draft, both seen by the owner. |
| P7 | macOS: pass. `889628d` leaves no child after either link. `431c726` leaves one defunct child per link, still there 15 s later, gone with the app. |
| P8 | macOS: the click skipped as planned: Skype takes `tel:` on the owner's Mac. The failing path is unwalked. |
| P9 | macOS: pass, both ways of saving shown within 0.4 s. |
| P10 | Linux: pass. The stub got the `https` and the `mailto` URL; no defunct child. |
| P11 | Linux: pass. Started through the wrapper script; the toast reads `No program to open links could be started: No such file or directory (os error 2)`; nothing started. |
| P12 | Linux: pass. The `kde-open6` stub got `--` and the URL; no toast. |
| P13 | Linux: pass, both ways of saving shown within 3 s. |

The webdriver plugin 0.2.4 built and drove the app on macOS as 0.2.3 did.

## Triage (the owner, 2026-11-16)

All four accepted with a reopen trigger (`open-items.md`, *Accepted limits*):

1. **A Ctrl- or Shift-click on a "back to top" link does nothing on Windows** (*Found on the
   way*, P5). Reopen on a report, or at the next change to link handling. Turned down: fixing
   it now, and accepting it for good.
2. **On macOS a link the system cannot open shows a raw error** (P8, unwalked). Reopen on a
   report. Turned down: rewording it now, and accepting it for good.
3. **The Linux AppImage is unwalked with `open` 5.4.5.** Reopen on a report.
4. **The watcher's deadlock fix on Windows is out of a walk's reach.** Reopen on a report of a
   folder that never lists or a document that never loads.

Not an entry: the tagged, public publish path first runs on a real release, which
`closed-items.md` already holds for every release (*A release tag's own path first runs on a
real release*).

## Review

Pass 1 (a fresh reviewer, read-only), ten findings, none a decision for the owner before the
walks; all taken:

1. The merges need `--admin` and get `--match-head-commit` (Tasks 2 and 3).
2. The checks after a merge compare against `origin/main`, since local `main` carries the plan.
3. "The app cancels every link click" was wrong in one case on Windows (*Found on the way*);
   P5 measures it.
4. The `tel:` check likely cannot fail on a Mac; the expected toast is written down and the
   check asks before it clicks (P8).
5. The watcher walk cannot reach the fixed race and says so; it now times each step and counts
   threads and handles (P2). The bump's row names the two changes it had missed.
6. A tab switch drops no watcher; a followed link, Back and a closed window do (P2, P3).
7. PR #24 brings a major bump of `glob`; the dry run's asset check is the one that covers it.
   Twelve bumps, not eleven; line 560, not 557.
8. Dependabot can rewrite PR #26 for any direct dependency, and PR #24 too; Task 3 checks its
   head.
9. The new `kde-open6` launcher is walked once as a success (P12), and P11 says how `PATH` is
   set.
10. Each peer checks its `rustc` first.

Pass 2 (the same reviewer), five findings, none a decision for the owner; all taken:

1. P2 could not be run as written: a plain click in the sidebar replaces the open document.
   It now names its files and steps, and when the counts are read.
2. A tab switch does rebuild the folder watcher when the other tab's folder is not on show; a
   followed link rebuilds the file watcher once or twice (*The bumps*).
3. drive-app starts the binary itself on Linux, so P11 and P12 hand it a wrapper script.
4. P5's Ctrl- and Shift-clicks are real input, and the page's address is reported.
5. Task 3 checks that `origin/main` is still PR #26's merge commit.

Pass 3 (the same reviewer), one finding, taken: P2's folder row exists only once its two parent
folders are expanded, so the setup expands them.
