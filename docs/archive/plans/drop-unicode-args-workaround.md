# Drop the Linux Non-Unicode Arguments Workaround Plan

> **Status (2026-11-10):** executed in full: 9761b62, 72 lines deleted from `main.rs` and none
> added. Windows gates passed (215 tests, as before). On the Linux VM (Ubuntu 26.04.1, headless
> Xvfb): clippy clean, 217 tests (three fewer than `main`), and U1-U3 passed with no panic; the
> second launches exited 0 in about 0.1 s. Tabs were judged from window titles and screenshots,
> since WebDriver can't pass a non-Unicode argument. Triage: no first-run check for the
> AppImage, and no entry for such a file being ignored without a message (existing behaviour,
> closed item #25); both are noted on the bug's entry in `closed-items.md`.

**Goal:** remove the Linux startup step that restarts the app in place when a command-line
argument isn't Unicode. The single-instance plugin made it unnecessary.

**Spec:** the owner's ruling of 2026-11-10 (`archive/plans/cargo-group-bump-pr23.md`, *Found on
the way*): removed in its own plan, after PR #23.

## Findings (measured 2026-11-10, before this plan)

### What the workaround is

- `exec_with_unicode_args` (`main.rs:1879-1901`), its helper `lossy_args` (`main.rs:1932-1944`),
  the call in `main` (`main.rs:1964`) and three Linux-only tests (`main.rs:2215-2246`). Nothing
  else names them: not the docs outside the archive, the skills or the workflows.
- Added in 009d8c8 (1.6.9). tauri-plugin-single-instance 2.4.5 read a second launch's arguments
  with `std::env::args()`, which panics on one that isn't Unicode, such as a Latin-1 file name.
  The release profile aborts on a panic, so the second launch died (exit 134) and a crash
  reporter could show.
- It only acts when an argument isn't Unicode: `lossy_args` returns `None` otherwise, and `main`
  goes on with no `exec`.
- Its own comment: "Delete this once the plugin reads `args_os()`."

### Why it is no longer needed

- The plugin reads `args_os()` and converts each argument lossily since 2.5.3 (its changelog,
  #3644: "A second instance no longer panics when a command-line argument is not valid
  Unicode"). 2.5.3 shipped in 1.8.6 (`828b63a`).
- `main` locks 2.5.5 today (`793f82a`). Read in its source: `linux.rs:103-105`, `macos.rs:89`,
  `windows.rs:126`.
- No crate on the startup path uses the panicking `std::env::args()`. Counted in the locked
  versions' sources: tauri 2.12.5, tauri-runtime 2.12.2, tauri-runtime-wry 2.12.3, tauri-utils
  2.10.2, wry 0.57.0, tao 0.37.1, and the updater 2.14.1, opener 2.7.2, dialog 2.8.4 and
  single-instance 2.5.5 plugins: zero uses each.
- The app's own readers are lossy already: `setup` (`main.rs:2072`) and the session restore
  (`session.rs:405`).

### What changes for the user

Reasoned from the code; Task 2 measures it.

- **A normal launch:** nothing. The workaround doesn't act.
- **A first launch on a non-Unicode name:** the same outcome, by a shorter path. The app opens a
  window without the file, since the mangled name matches no file (closed item #25). It no
  longer restarts itself first, so `/proc/<pid>/cmdline` keeps the original bytes.
- **A second launch on such a name:** the same outcome. It exits 0 and the running window comes
  forward. The plugin now does the conversion the workaround did.
- **Windows and macOS:** no change. The code is Linux-only.

## Global constraints

- Deletions only, in `src-tauri/src/main.rs`. No new code, no dependency change.
- Gates on Windows before the commit, from the repo root: `cargo fmt --manifest-path
  src-tauri/Cargo.toml --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace
  --all-targets --locked -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml
  --workspace --locked`. `app.js` isn't touched, so no `node --check`.
- Windows never compiles `cfg(target_os = "linux")` code, so its gates prove only that nothing
  else moved. The Linux gates run on the Linux VM, from the walk branch (Task 2).
- Straight onto `main`, committed locally. No push, branch or delete without the owner's go at
  that moment. No AI attribution.
- The walk runs on the Linux VM (`md-viewer-lin`) from a pushed `walk/drop-exec` branch, never
  on the host PC or its WSL. Each launch gets its own `XDG_CONFIG_HOME` and `XDG_DATA_HOME`, so
  the owner's settings and session are never read or written.

## Task 1: delete the workaround

1. Remove from `main.rs`: `exec_with_unicode_args` with its comment, `lossy_args` with its
   comment, the call in `main`, and the tests `lossy_args_leaves_unicode_alone`,
   `lossy_args_mends_a_latin1_name` and `lossy_args_of_nothing`.
2. `git diff --stat` shows `main.rs` only, with no added line.
3. Windows gates. Expect the same test count as before the change (the three tests never
   compiled on Windows).
4. Commit: `chore: drop the Linux re-exec the single-instance plugin made unneeded`.

## Task 2: Linux gates and walk

On `walk/drop-exec`, on the Linux VM.

**Gates:** clippy and the tests, with the commands above. Expect three fewer tests than `main`
on Linux, and none named `lossy_args_*`. CI doesn't run on a walk branch (`ci.yml` runs on
pushes to `main` and on pull requests), so if the VM has no clippy, the fallback is a manual CI
run on the branch (`gh workflow run ci.yml --ref walk/drop-exec`), on the owner's go.

**Walk:** the debug build, with the audit identifier the drive-app skill's `linux.md` gives it,
so an installed copy can't take the second launches. A debug build unwinds where a release
build aborts, so a panic shows as a `panicked at` line on stderr and exit code 101. In a
scratch folder holding `first.md`,
`second.md` and a file named with the bytes `caf\xe9.md`:

- **U1, first launch on the non-Unicode name:** `t4-markdown-viewer $'caf\xe9.md'`. The app
  starts and opens a window, with nothing on stderr about a panic. `/proc/<pid>/cmdline` holds
  the bytes `caf\xe9.md` as typed, which proves no restart happened. The window shows no
  document.
- **U2, second launch on the non-Unicode name:** with the app running on `first.md`, launch
  again on `$'caf\xe9.md'`. The second process exits 0 with no `panicked` on stderr. The first
  process is alive and its window still shows `first.md`.
- **U3, control:** a second launch on `second.md` exits 0, and the file opens as a tab in the
  running window.

## Task 3: close out

- Triage with the owner, one item at a time: any finding, and whether the AppImage gets a
  first-run check at the next release (its own runtime sits between the launch and the app, and
  the walk doesn't run it).
- `docs/closed-items.md`: a dated note on the bug's entry ("A second instance aborts on a file
  name that is not Unicode") giving the removal commit and U1-U3's results, and one on the
  Windows and macOS entry beside it, which the plugin's fix also closes.
- Archive this plan to `docs/archive/plans/` with a status line. One `docs:` commit, local.
- Asked separately, each at its moment: pushing `main`, and deleting `walk/drop-exec`.

## Execution

The main session does Task 1 (deletions in one file) and Task 3, and briefs the Linux VM for
Task 2. Serial. Stop and ask on:

- Any gate failing, on Windows or Linux, or clippy reporting anything new.
- A `panicked` line or a non-zero exit in any walk step.
- A walk step behaving differently from *What changes for the user*.
