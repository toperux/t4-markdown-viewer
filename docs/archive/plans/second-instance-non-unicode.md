# Second Instance on a Non-Unicode Name Implementation Plan

> **Status (2026-09-26):** executed — 009d8c8. No deviations (`cargo fmt` re-wrapped two lines).
> Results are in `../../closed-items.md` under *Bugs*; the AppImage check is under *First runs*
> in `../../open-items.md`. Unticked boxes below are as they stood when it was written.

> **For agentic workers:** Execute per the **Execution** section below. Steps use checkbox
> (`- [ ]`) syntax for tracking.

**Goal:** A second launch on a file name that is not Unicode hands over to the running app
quietly instead of aborting.

**Architecture:** On Linux only, the first thing `main` does is check its arguments. If any is
not Unicode, it replaces itself (`exec`, same PID) with the same program and the arguments made
Unicode lossily — exactly what `setup` already does with them. The second process that the
single-instance plugin sees then has only Unicode arguments, so `std::env::args()` inside the
plugin cannot panic. A pure helper decides whether to re-exec and what with; it gets unit
tests.

**Tech Stack:** Rust + Tauri v2, stdlib only (`std::os::unix::process::CommandExt`). WSL Ubuntu
24.04 (Rust 1.98.1, `libwebkit2gtk-4.1-dev` installed) for the Linux gates and the check.

**Spec:** `docs/open-items.md` → *Needs a decision* → "A second instance aborts on a file name
that is not Unicode", option A.

## Findings (measured 2026-09-26, before this plan)

- **The abort is real and reproduced.** Fedora 44, the dry-run rpm (release profile,
  `panic = "abort"`): first instance on `first.md`, then `t4-markdown-viewer $'caf\xe9.md'` →
  `Aborted`, exit 134, `panicked at std/src/env.rs:878:51`. The first instance stayed up. A
  second instance on a UTF-8 name exited 0 and opened its file (control).
- **The cause is the plugin, not us.** `tauri-plugin-single-instance` 2.4.5 (locked),
  `src/platform_impl/linux.rs:77`: on `zbus::Error::NameTaken` it sends
  `std::env::args().collect::<Vec<String>>()` over D-Bus, and `Args::next` is
  `s.into_string().unwrap()` (`std/src/env.rs:878`). Our `setup` (`main.rs:1692`) and
  `session.rs:257` already read `args_os()` lossily; the first instance is fine.
- **The arguments cannot be fixed in place.** std snapshots argv at startup; there is no API
  to rewrite what `args()` returns. Replacing the process is the only way to hand the plugin
  different arguments without patching it.
- **A lossy argument is handled harmlessly on the receiving side.** `handle_second_instance`
  (`main.rs:1530`) → `file_from_args` finds no file at `caf\u{FFFD}.md` → focuses the last
  window. Same outcome as the first instance's "the mangled name is ignored" (#25 keeps the
  file itself unopenable).
- **Windows and macOS share the call** (`windows.rs:91`, `macos.rs:88`), but neither can hand
  over such a name in practice: Windows arguments are UTF-16 (only an unpaired surrogate
  fails), and macOS file systems store UTF-8 names and Finder opens files through Apple
  Events, not argv.
- **A non-Unicode working directory is a neighbouring, separate gap.** The plugin sends
  `current_dir().to_str().unwrap_or_default()`, so from such a folder the cwd arrives empty and
  a *relative* argument is resolved against the running app's own cwd — no crash, just the
  file not found (or, by coincidence, a same-named one opened). File managers pass absolute
  paths (`%f`), so only a terminal hits it, and `exec` cannot change it. Out of scope; offered
  to the owner at close-out with the Windows/macOS residue.
- **WSL Ubuntu holds state from the 2026-09-26 package checks**:
  `~/.config/t4-markdown-viewer/session.json` and
  `~/.local/share/com.montevirgen.t4-markdown-viewer/`. With the default `reopen`, a debug
  launch would offer or restore those windows and muddle the title checks. Task 2 runs with
  its own `XDG_CONFIG_HOME` and `XDG_DATA_HOME` (`dirs::config_dir` honours them) and leaves
  those folders — and `dev.topher.t4gitui`, the other app's — alone.

## Decisions (the owner can overrule at review)

- **Linux only** (`#[cfg(target_os = "linux")]`). Windows has no `exec`, and macOS re-exec
  inside a `.app` launched by LaunchServices is untested territory for a case macOS cannot
  produce from Finder. The Windows/macOS residue goes to *Accepted limits* at close-out, for
  the owner to rule on.
- **Every argument lossy, `argv[0]` included**, passed as `arg0`. The plugin collects all of
  them, so a non-Unicode install path (an AppImage in a Latin-1 folder) would panic just the
  same.
- **The program re-executed is `std::env::current_exe()`** (`/proc/self/exe`), not `argv[0]`,
  which may be a bare name or a symlink. In an AppImage it is the binary inside the mounted
  image; the mount lives as long as the AppImage runtime, which waits on this same PID.
- **If `current_exe` or `exec` fails, carry on as today.** `exec` returns only on failure; the
  worst case is the current behaviour, never a failed start.
- **No loop:** after the re-exec every argument is Unicode, so the check returns `None`.
- **Temporary by intent.** The comment names the plugin line it works around; once the plugin
  reads `args_os()`, the helper and its call go. The close-out records that as the reopen
  trigger, checked on each `tauri-plugin-single-instance` bump.

## Global Constraints

- Surgical changes only: one helper, one call, their tests. Match the surrounding comment
  style (full-sentence "why" comments) and naming. No new dependencies.
- Gates, from the repo root on Windows, all must pass before the commit:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`
  - `node --check src/app.js`
- **Plus the Linux gates in WSL**, because Windows never compiles `cfg(target_os = "linux")`
  code. Work on a clone in the WSL home, not `/mnt/f` (9p is slow and the Windows `target/`
  must not be touched):
  ```bash
  wsl -- bash -lc 'rm -rf ~/t4-wsl && git clone -q "/mnt/f/src/_ pet projects/t4-markdown-viewer" ~/t4-wsl'
  # then copy the edited main.rs over the clone's (the change is uncommitted at this point)
  wsl -- bash -lc 'cd ~/t4-wsl && cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked'
  ```
- Commit straight to `main`, locally. **Never push.** End the message with the executing
  model's `Co-Authored-By:` line.
- Bash on Windows: never `cd` in a compound command that writes; use `--manifest-path` /
  `git -C`. In WSL scripts, never `pkill -f t4-markdown-viewer`: the scratchpad path contains
  that string and the script kills itself. Use `pkill -x t4-markdown-vie` (the kernel's
  15-character comm name).
- Nothing here launches the app on Windows, so `%APPDATA%` is not at risk.

## Review Focus

1. **No behaviour change when every argument is Unicode** — the ordinary path. The helper
   returns `None`; `main` goes on with no `exec`. Task 1 test `all_unicode`.
2. **`argv[0]` preserved** (as its lossy form) and argument order kept. Task 1 test
   `latin1_argument`.
3. **An empty argv** (possible with `execve(path, NULL, …)`) does not panic. Task 1 test
   `empty`.
4. **Same PID after the re-exec**, so a launcher waiting on it (the AppImage runtime,
   `gio launch`) is not confused. Task 2 check 2.
5. **The call is the first statement of `main`**, before the builder: anything started before
   it (a thread, a D-Bus connection) would be lost or leaked by `exec`.

## Execution

One `coder` subagent (Opus) runs Tasks 1–2 and reports the gate output and the Task 2
transcript. The main session reviews the diff and the transcript, then commits and does
Task 3.

## Task 1: The helper, its call and tests

**Files:** `src-tauri/src/main.rs` only.

- [ ] **Step 1 — the helper.** Just above `fn main()`:

  ```rust
  /// A second launch hands its arguments to the running app through
  /// `tauri-plugin-single-instance`, which reads them with `std::env::args()`
  /// (2.4.5, `platform_impl/linux.rs`). That panics on one that is not Unicode —
  /// a Latin-1 file name from a file manager — and with `panic = "abort"` the
  /// launch dies before any of our code could catch it. The arguments cannot be
  /// changed in place, so start over as the same process with them made Unicode,
  /// lossily, which is how `setup` reads them anyway. `exec` returns only if it
  /// failed, and then this launch goes on exactly as before. Delete this once the
  /// plugin reads `args_os()`.
  #[cfg(target_os = "linux")]
  fn exec_with_unicode_args() {
      use std::os::unix::process::CommandExt;
      let Some(args) = lossy_args(std::env::args_os()) else {
          return;
      };
      let Ok(exe) = std::env::current_exe() else {
          return;
      };
      let _ = std::process::Command::new(exe)
          .arg0(&args[0])
          .args(&args[1..])
          .exec();
  }

  /// Every argument made Unicode, or `None` when they all are already.
  #[cfg(target_os = "linux")]
  fn lossy_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Option<Vec<String>> {
      let args: Vec<_> = args.into_iter().collect();
      if args.iter().all(|a| a.to_str().is_some()) {
          return None;
      }
      Some(args.iter().map(|a| a.to_string_lossy().into_owned()).collect())
  }
  ```

  `args[0]` is safe: `Some` means at least one argument exists.

- [ ] **Step 2 — the call.** First statement of `main`:

  ```rust
  fn main() {
      #[cfg(target_os = "linux")]
      exec_with_unicode_args();
      let builder = tauri::Builder::default()
  ```

- [ ] **Step 3 — tests.** In the existing `#[cfg(test)] mod tests` of `main.rs` (it has
  `use super::*`), three tests, each `#[cfg(target_os = "linux")]`, building non-Unicode input
  with `std::os::unix::ffi::OsStringExt::from_vec(b"caf\xe9.md".to_vec())`:
  - `lossy_args_leaves_unicode_alone`: `["/usr/bin/t4-markdown-viewer", "a.md"]` → `None`.
  - `lossy_args_mends_a_latin1_name`: `[exe, "x", caf\xe9.md]` →
    `Some([exe, "x", "caf\u{FFFD}.md"])` — order kept, Unicode arguments untouched.
  - `lossy_args_of_nothing`: `[]` → `None`.

- [ ] **Step 4 — gates.** Windows gates (they do not compile the new code; they prove nothing
  else moved), then the WSL gates. Both must pass. Report the WSL test lines for the three new
  tests by name.

## Task 2: Prove it in WSL

Build the debug binary in the WSL clone
(`cargo build --manifest-path src-tauri/Cargo.toml --locked`). Debug unwinds rather than
aborts, so the "before" shows a panic message and exit 101 instead of 134; the point is panic
versus no panic. Run as the default WSL user, every launch with
`GDK_BACKEND=x11 XDG_CONFIG_HOME=$T/cfg XDG_DATA_HOME=$T/data` where `T=$(mktemp -d)`, so no
earlier session comes back (see *Findings*); kill with `pkill -x t4-markdown-vie`.

- [ ] **Check 1 — before.** Order matters: the fresh clone is `HEAD` without the change, so
  build and run this *before* copying the edited `main.rs` in, then rebuild. First instance on
  `first.md`, second on `$'caf\xe9.md'` → panic in the second's stderr, non-zero exit. This
  proves the WSL setup reproduces the Fedora finding.
- [ ] **Check 2 — first instance on a non-Unicode name** (with the change): launch on
  `$'caf\xe9.md'` in the background, record `$!`. After 8 s: the process with that same PID is
  alive, and `/proc/$!/cmdline` holds `caf\xef\xbf\xbd.md` (U+FFFD in UTF-8) — proof the
  re-exec happened in place. A window exists (`xwininfo -root -tree`).
- [ ] **Check 3 — second instance on a non-Unicode name** (with the change): first instance on
  `first.md`; second on `$'caf\xe9.md'` → exit 0, no `panicked` in its stderr; the first is
  alive and its window title still names `first.md`.
- [ ] **Check 4 — control.** Second instance on `second.md` → exit 0, and the first window's
  title becomes `second.md — Markdown Viewer`.
- [ ] **Clean up:** kill the app, `rm -rf ~/t4-wsl`.

## Task 3: Close-out (main session)

- [ ] Commit the code, subject: `Stop a second launch aborting on a file name that is not
  Unicode`.
- [ ] `docs/open-items.md`: move the bug item to `closed-items.md` under *Bugs*, with the
  commit and the Task 2 results. Add to the *First runs* item a sub-item: *the AppImage*
  (release or dry run) — in WSL, a second instance of the AppImage on `$'caf\xe9.md'` exits 0.
  The AppImage wraps the binary in its own runtime, which Task 2 does not exercise.
- [ ] Ask the owner to rule, one at a time, on the Windows/macOS residue and the non-Unicode
  working directory (*Findings*) as accepted limits; record each ruling. Record the reopen
  trigger: remove the workaround when `tauri-plugin-single-instance` reads `args_os()`.
- [ ] Move this plan to `docs/archive/plans/` with a status line. Commit as `docs:`.
