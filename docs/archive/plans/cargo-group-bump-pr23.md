# Cargo Group Bump (Dependabot PR #23) Plan

> **Status (2026-11-10):** executed in full. PR #23 rebase-merged as 793f82a, its tree identical
> to the walked 5856ead; CI run 42828958546 on it passed on all three legs. P1-P3 passed on
> macOS (26.7.1, arm64), Linux (Ubuntu 26.04.1, WebKitGTK 2.52.6, headless Xvfb) and Windows
> (11 build 26300, WebView2 156); a second launch exits 0 in about 0.1 s on all three. P5 passed
> on Windows (0.3 s). P4, five runs: one app process and all four files open every time, none
> lost, but in four windows, not the one this plan expected. That is the app's own routing
> (`main.rs:1520-1556`), the same on 1.8.10 (three runs); the owner deferred it
> (`open-items.md`). The Windows race fix's 1 s retry was never seen to trigger.
> Every quit on Windows prints Chromium's "Failed to unregister class Chrome_WidgetWin_0. Error
> = 1412" to the debug console, on 1.8.10 too (3 of 3); the owner ruled no entry. The update
> restart is under *First runs* in `open-items.md`. Two errors in the first briefs:
> `examples/mermaid.md` doesn't exist (P2 and P4 below are corrected), and debug builds use the
> audit identifier, so the Mac's socket was `…_viewer_audit_si.sock`. The Linux workaround
> (*Found on the way*) gets its own plan next.

**Goal:** land Dependabot PR #23's bumps on `main` (tauri-plugin-dialog 2.8.3 → 2.8.4,
tauri-plugin-single-instance 2.5.4 → 2.5.5) so the next release ships with them.

**Spec:** PR #23 (`gh pr view 23`), opened 2026-11-05. The only open pull request; no security
alert is attached, so nothing forces the timing. The owner ruled on 2026-11-10 that it is walked
on all three platforms before it merges (Task 1).

## Findings (measured 2026-11-10, before this plan)

PR #23 is one commit, `5856ead`, on `31caae0` (Release 1.8.10), which is `main` today. It changes
`src-tauri/Cargo.lock` only (26 lines added, 98 removed). Its checks passed on Windows, macOS and
Linux, and they build with `--locked`, so the lockfile is consistent on all three.

Read in the diff: every changed line is a version, a checksum or a dependency-list entry. Every
package in the new lockfile names `registry+https://github.com/rust-lang/crates.io-index`. The
`.crate` files of dialog 2.8.4, single-instance 2.5.5 and rfd 0.17.2, downloaded from crates.io,
hash to the lockfile's checksums.

### Why the lockfile shrinks

Three packages change version: the two plugins, and `rfd` 0.16.0 → 0.17.2 (the file-dialog crate
under the dialog plugin). The rest of the removed lines are one duplicate going away:

- `rfd` 0.16 was the one crate that needed `windows-sys` 0.60. `rfd` 0.17 asks for 0.61.
- With it gone, 15 crates that accept a range (`errno`, `rustix`, `socket2`, `tempfile`,
  `dirs-sys`, `muda`, `rustls-platform-verifier`, `tray-icon`, `window-vibrancy`, `gio-sys`,
  `anstyle-query`, `anstyle-wincon`, `terminal_size`, `uds_windows`, `winapi-util`) move from
  `windows-sys` 0.60.2 to the 0.61.2 already in the tree. Their own versions don't change. Seven
  of their manifests were read and all allow 0.61.
- So `windows-sys` 0.60.2, `windows-targets` 0.53.5 and its eight per-target crates drop out.
- `iana-time-zone` moves from `windows-core` 0.61.2 to 0.62.2, also already in the tree.

`windows-sys` is declarations of Windows functions, compiled only on Windows. So on Windows,
those 15 crates are built against newer declarations; macOS and Linux don't compile it. Not
measured: why Dependabot's resolver consolidated them (a plain `cargo update -p` usually leaves
locked entries alone). The result is what its CI built and tested.

### The bumps

The upstream changes were read as source diffs: the 2.8.4 and 2.5.5 crates against the 2.8.3 and
2.5.4 in the local cargo registry, and `rfd` 0.17.2 against 0.16.0.

| Bump | What changed upstream | Consequence here |
|---|---|---|
| tauri-plugin-dialog 2.8.3 → 2.8.4 | Its manifest only: `rfd` 0.16 → 0.17, and the `xdg-portal` feature loses `rfd/tokio`. No source file changed | The app's two pickers, *Open file* and *Open folder* (`pickFile`, `pickFolder`, `app.js:4976` and `4994`), are the plugin's only use (`dialog:allow-open`). The plugin's default `gtk3` feature is on and `xdg-portal` is off |
| rfd 0.16.0 → 0.17.2, **Windows** | The async pickers were rewritten: `dialog_future.rs` and `thread_future.rs` are gone, and `pick_file_async` now runs the blocking picker on a spawned thread and sends the answer over a channel. The thread starts when the future is first polled, no longer when it's made | The plugin calls exactly these (`AsyncFileDialog`, `desktop.rs:149`). Same dialog, same COM setup, different plumbing. This is the one picker path whose code changed, so the Windows walk opens both pickers and cancels one |
| rfd, **macOS** | Additions only (a new "file or folder" picker). No existing line removed in `backend/macos/` | The plugin doesn't call the new picker. The walk still opens one |
| rfd, **Linux** | `backend/gtk3/` is identical. The portal backend was rewritten from `ashpd` to `libdbus` | The portal backend is off here. `libc` and `percent-encoding` become Linux dependencies of `rfd`; both were in the tree already |
| tauri-plugin-single-instance 2.5.4 → 2.5.5, **all platforms** | The lock that marks the running app (a named mutex and a hidden window on Windows, a D-Bus name on Linux, the socket `/tmp/com_montevirgen_t4_markdown_viewer_si.sock` on macOS) now lives in Tauri's resource table and is released in `cleanup_before_exit`, no longer by the plugin's own `RunEvent::Exit` handler | The plugin is how a second launch hands its file to the running app (`main.rs:1969`). Tauri 2.12.5 calls the plugins' Exit handlers, then the app's, then `cleanup_before_exit`, then restarts if asked (`app.rs:1449-1455`). The app's run callback handles only `RunEvent::Opened` (`main.rs:2181`), so the release moves by a few instructions. Quit (`app.exit(0)`, `main.rs:1703`) and closing the last window both go through it |
| single-instance, **Windows** | A second launch that finds the mutex but not yet the first launch's hidden window now looks again for up to 1 s (40 tries, 25 ms apart), then exits either way. Before, it carried on and started as a second full copy of the app | Reachable by opening several files at once from Explorer on a cold start: each is its own process. Before: a loser of the race could become a second app, with its own windows and its own writes to the session file. After: it waits, hands over its file and exits. If the 1 s runs out, its file is dropped with only a log line |
| single-instance | `deep-link` 2.6.2 → 2.6.3 | An optional dependency, off here. Not in the lockfile |

### The update restart, reasoned but not walkable

After an update installs, the app restarts itself (`update.rs:197`). The new process must find
no lock, or it would hand over to the dying one and exit, and the app would not come back.

- **macOS and the Linux AppImage:** `restart` from a command thread asks for an exit and
  restarts from the Exit event, after `cleanup_before_exit` has released the lock
  (`app.rs:1452-1454`). Read in Tauri's source; the plugin's new `instance_lock` test covers
  release-on-cleanup upstream.
- **Windows:** the updater starts the installer and calls `std::process::exit(0)`
  (`updater.rs:1050`), which skips the cleanup now as it did before. The mutex goes with the
  process.

Not measured: it needs a newer release to install. The first real run is the first update
*from* a build that has this bump. Raised at triage (Task 3).

### Found on the way, not from this PR

`main.rs:1879-1944` (`exec_with_unicode_args`, `lossy_args`, three tests) is a Linux workaround
for the plugin reading arguments with `std::env::args()`, which aborted a second launch on a file
name that isn't Unicode. Its comment says to delete it once the plugin reads `args_os()`, and the
closed item says to check on each bump. The plugin has read `args_os()` since 2.5.3, which
shipped in 1.8.6 (`828b63a`); 2.5.5 still does (`linux.rs:103`). The workaround only acts when an
argument isn't Unicode, so it costs nothing on a normal launch. Removing it is a code change with
its own walk, outside this PR. The owner ruled on 2026-11-10: it is removed in its own plan,
written after this one closes.

## Global constraints

- No dependency change beyond PR #23's. No local edit to the Cargo files: the PR's commit goes in
  as it is.
- No AI attribution. No push, merge, or delete without the owner's go at that moment.
- Walks run on the peers (Windows VM, Linux VM, Mac), never on the host PC, from the PR's own
  commit `5856ead`, which is already on origin. No `walk/*` branch is needed. Each peer works in a
  worktree under `.claude/worktrees/`, and backs up and restores `config.json` and `session.json`
  per the drive-app skill.
- Dependabot's next weekly run is due about 2026-11-12. If a new version of any crate is out by
  then it rewrites the PR's branch, and the walked commit is no longer what would merge.

## Task 1: walks

Debug build of `5856ead`, on all three platforms unless marked.

- **P1, pickers:** *Open file* shows the native picker at the expected folder; choosing
  `examples/kitchen-sink.md` opens it. *Open folder* shows the folder picker; choosing `examples`
  fills the sidebar. One picker cancelled: nothing opens and the window still answers (a tab
  switch works).
- **P2, second launch:** with the app running, start it again on `examples/widths.md`. The file
  opens as a tab in the running window, and the second process exits. On macOS, start the binary
  from a terminal (Finder opens don't go through the plugin).
- **P3, quit and come back:** quit (Ctrl+Q, or the menu on macOS). The process is gone within
  5 s with exit code 0. On macOS the socket file is gone too. Start the app again: it opens a
  window as the first instance. A smoke for the new release path, since a dead process frees a
  mutex and a D-Bus name whatever the plugin does.
- **P4, several files at once, Windows only:** from cold and with an empty session, start the
  app four times in one go, each on a different file: the three documents in `examples/` and
  `README.md` (`Start-Process` four times, no wait).
  After 5 s: one
  `t4-markdown-viewer.exe` process, one window, four tabs. Run it five times, since it's a race.
  A missing tab or a second app process is a finding.
- **P5, update check, Windows only:** Settings › *Check now* reads "You are up to date." The
  certificate check and the sockets under it are among the crates on newer declarations.

## Task 2: merge

On every walk passing, and the owner's go:

1. `gh pr view 23 --json headRefOid` still says `5856ead…`, and `git rev-parse origin/main` is
   still `31caae0`. Otherwise stop.
2. `gh pr merge 23 --rebase`, which keeps Dependabot's `chore:` subject, as the release skill
   says. `git pull --ff-only`.
3. `git diff 5856ead HEAD` prints nothing. The `checks.yml` run on `main` passes.
4. The PR's branch stays on origin after the merge (the repo doesn't delete merged branches).
   Deleting it is its own ask.

No local gates: the merged tree is the one the PR's checks ran on.

## Task 3: close out

- Triage with the owner, one item at a time: any walk finding, and the update restart that
  couldn't be walked.
- Archive this plan to `docs/archive/plans/` with a status line giving the merge commit and
  P1-P5's results, plus the open-items entries the triage leaves. One `docs:` commit, local;
  pushed on the owner's go.

## Execution

The main session briefs the peers for Task 1 and runs Tasks 2 and 3. Serial. Stop and ask on:

- The PR's head or `main` having moved.
- Any walk step failing, or behaving differently between runs.
- The post-merge checks failing.
