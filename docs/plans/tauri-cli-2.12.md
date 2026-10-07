# Tauri CLI 2.12.1 Implementation Plan

> **Status (2026-10-07):** in the plan review loop; Decisions 1–4 ruled by the owner as
> recommended. Nothing executed.

> **For agentic workers:** Execute per the **Execution** section below. Steps use checkbox
> (`- [ ]`) syntax for tracking.

**Goal:** The Linux AppImage starts on Ubuntu 26.04 and other distributions with a recent Mesa,
by moving the release's Tauri CLI from 2.11.4 to 2.12.1.

**Architecture:** tauri-cli 2.12.1 locks tauri-bundler 2.10.1, whose linuxdeploy (`07333c6`)
leaves `libwayland-client` out of the AppImage by itself. The release workflow moves to that CLI
and re-pins the AppImage tools; a CI check fails the build if `libwayland-client` ever comes
back. The new bundler's gtk hook no longer forces X11, so the app does it itself inside an
AppImage, as the old hook did. Nothing changes for Windows and macOS apart from the bundler's
own changes, which a dry run and a smoke walk cover.

**Tech Stack:** GitHub Actions (`release.yml`), tauri-cli 2.12.1 / tauri-bundler 2.10.1, Rust
(`main.rs`), `drive-app`.

**Spec:** open-items › Deferred › *The AppImage can't start on Ubuntu 26.04* — ruled by the
owner 2026-10-07: fix next, through the tauri-cli 2.12.1 bump, ahead of the session-restore
plan. `archive/plans/tauri-2.12.md`, Decision 2, deferred the bump.

**Prior art:** t4-git-ui made this move on 2026-10-05:
`../t4-git-ui/docs/archive/plans/2026-10-04-tauri-2.12-plan.md` (its *The bundler* section,
and rulings N1, N2, N4), commits `ee39631` (the first, repack fix) and `308c378` (the CLI bump
that replaced it). Its findings below were measured in the bundler and linuxdeploy sources;
where this app differs, it says so.

## The problem (measured 2026-10-07, `archive/plans/tauri-2.12.md` W12)

On the Linux VM (Ubuntu 26.04, GNOME 50 Wayland, Mesa 26.0.8, libegl1 1.7.0), the AppImage's
web process aborts with `Could not create default EGL display: EGL_BAD_PARAMETER`, and no
window opens: the dry run's AppImage and the released 1.7.2 alike. The release builds on
`ubuntu-22.04` (`release.yml:105`), and linuxdeploy bundles that runner's `libwayland-client`
(1.20). The host's Mesa `libEGL_mesa` uses symbols 1.20 lacks (t4-git-ui's diagnosis,
`ee39631`). An extracted copy with the bundled libwayland libraries moved out rendered and
passed the smoke rows.

## What tauri-cli 2.12.1 changes (t4-git-ui's reading of bundler 2.9.4 → 2.10.1)

- **linuxdeploy moves to a fixed build**, `07333c6`, fetched as
  `linuxdeploy-07333c6-x86_64.AppImage` from
  `…/binary-releases/releases/download/linuxdeploy-07333c6/linuxdeploy-x86_64.AppImage`, sha256
  `36a2d7e274d12e1050d0e9ecfe11d339ed54720b2bec464c286d53f8b07f5c62` (measured by t4-git-ui on
  two downloads, equal to GitHub's asset digest). Its exclude list adds `libwayland-client.so.0`
  and `libpipewire-0.3.so.0`. This is the fix. t4-git-ui's AppImage built this way passed its
  walk on the same Ubuntu 26.04 VM (its group BO 10).
- **The gtk and gstreamer plugin scripts are embedded in the bundler** and rewritten into the
  tool cache whenever the cached copy differs. Our pinned downloads of them
  (`release.yml:222-223`) would be overwritten every run, so they go; the CLI's crates.io
  checksum (`--locked`) pins them instead.
- **AppRun** (`apprun-old`) and **linuxdeploy-plugin-appimage** (pinned to `1-alpha-20250213-1`)
  are unchanged; the runtime (`LDAI_RUNTIME_FILE`) is ours and unchanged.
- **The embedded gtk hook no longer exports `GDK_BACKEND=x11`.** The old hook (`dda522b`) did,
  for a Wayland crash (tauri#8541). Without it, on a Wayland desktop the AppImage runs native
  Wayland, where the compositor ignores the app's window positions: a restored window's frame
  (`Frame::apply`, `session.rs`) and the tear-off placement (`drop_tab`, `main.rs:1149`). The
  debug build already runs native Wayland on the VM, and its W1 there could not check positions
  (measured). Decision 1.
- **GIO modules.** The old hook copied only `libgiognutls.so` and set `GIO_EXTRA_MODULES`
  (added to the host's); the new one copies the build host's whole `giomoduledir` (libproxy,
  dconf, gvfs from 22.04) and sets `GIO_MODULE_DIR`, which *replaces* the host's. Reasoned
  risks: GSettings now read through the bundled dconf module (Decision 2), and the file
  chooser's mounts and `gio open` run on 22.04's modules. Every process the app starts inherits
  it: the browser a link opens (`tauri-plugin-opener` → `xdg-open`) and the file manager
  *Reveal* opens (`reveal_path`, `main.rs:1308`; reached by a link to a file that is neither a
  document nor a picture). Walk W4 checks both.
- **`bundleXdgOpen` is gone.** The app never set it; links already open through the host's
  `xdg-open`.
- **The NSIS installer's running-app check now uses Restart Manager** (`CheckIfAppIsRunning`
  takes a full path). t4-git-ui had to fix its own hook; ours (`src-tauri/windows/hooks.nsi`)
  only registers file types and never calls it (measured: no `CheckIfAppIsRunning` in it), so
  nothing to change. The new check runs when an installer meets a running copy, which the
  Windows in-app update always does: walk W2.
- **The CLI's crate/npm version check** (tauri-cli 2.12.1 errors when `tauri` and
  `@tauri-apps/api` differ in minor) has nothing to compare: the app has no `package.json`
  (reasoned; the dry run shows it).
- The rest of the bundler diff is formatting (deb, rpm, macOS, `windows/sign.rs`), plus an
  opt-in VC-runtime bundling the app doesn't set.

## Decisions

1. **Keep X11 inside the AppImage** (owner, 2026-10-07). In `main()`,
   before anything else, set `GDK_BACKEND=x11` when the `APPIMAGE` variable is set (the
   AppImage runtime sets it; `update.rs:35-41` already uses it), whatever it held, as the old
   hook's export did. Without it, a restored window and a torn-off one land wherever the
   compositor puts them on Wayland. Every process the app starts inherits `x11`, as it does
   today under the old hook. t4-git-ui ruled the same (its N1), and its later fix gave the
   user's value back to the programs it starts (its N8); this app starts only `xdg-open` and the
   file manager, through the opener plugin, whose environment it doesn't control, so they keep
   `x11`, as now. Ruled so.
   - Alternative: leave Wayland native. Costs: the positions above are ignored on Wayland (and
     the deferred tear-off placement item gets harder to judge); gains: native Wayland
     rendering, and the browser a link opens runs as the user set.
2. **The file chooser's start folder under the bundled dconf module** (owner, 2026-10-07:
   option (a), measure first). With dconf now loaded, GTK reads the user's own GSettings: if
   `org.gtk.Settings.FileChooser startup-mode` is `cwd`, a picker with no start folder opens in
   the AppImage's read-only mount, where AppRun leaves the working folder (t4-git-ui measured
   this on the same VM, its N4). Here the Open and folder pickers pass a start folder whenever
   one is known (`app.js:3552, 3568`, `defaultPath: dir || undefined`), so only a first pick
   with none is exposed. t4-git-ui fixed it with `GSETTINGS_BACKEND=memory` inside an AppImage,
   which also hands `memory` to every program the app starts (it then restored theirs; this app
   can't, see Decision 1). Options:
   - (a) **Measure first** (ruled): walk W3 opens the pickers with no start folder on the
     dry run's AppImage, with `startup-mode` at `cwd` and at its default; set
     `GSETTINGS_BACKEND=memory` only if a picker is seen opening in the mount, through fix →
     review → triage.
   - (b) Set `GSETTINGS_BACKEND=memory` now, as t4-git-ui did. Cost: the browser and file
     manager the app starts get `memory` too, ignoring the user's GSettings (unmeasured what that
     changes for them).
3. **A CI check that `libwayland-client` stays out** (owner, 2026-10-07): right after *Bundle and sign* on
   Linux, fail if `T4 Markdown Viewer.AppDir` (the bundler leaves it beside the image) is missing
   or holds a `libwayland-client*`, as t4-git-ui's *Check the AppImage has no libwayland-client*
   does. A later CLI or linuxdeploy that bundles it again would ship a broken AppImage silently;
   nothing else would notice before a user. Ruled so.
4. **The commits** (owner, 2026-10-07). The workflow and `main.rs` change is one commit, user-facing, titled for the
   release page: `The AppImage starts on Ubuntu 26.04 and other distributions with a recent
   Mesa` (no prefix, so it becomes 1.7.3's title — `release-title-most-significant`). The
   release skill and docs edits are `docs:`.

## Tasks

### Task 1: The bump and the X11 setting

**Files:**
- Modify: `.github/workflows/release.yml` (*Install the Tauri CLI* `:134-139`, *Pin the
  AppImage tools* with its comment `:197-226`, a new step right after *Bundle and sign*
  `:259-288`)
- Modify: `src-tauri/src/main.rs` (`main()`, `:1669`; a test beside `lossy_args`'s)
- Modify: `.claude/skills/release/SKILL.md` (`:152-157`, and *When it goes wrong* if it has one)

- [ ] **Step 1: The CLI.** `cargo install tauri-cli --version 2.12.1 --locked`. The comment
  above keeps its reasons ("checksum-verified", "pinned exactly", "bump deliberately with
  Tauri").

- [ ] **Step 2: The pins.** In *Pin the AppImage tools*:
  - replace the `linuxdeploy-x86_64.AppImage` line with
    `fetch linuxdeploy-07333c6-x86_64.AppImage $rel/tauri-apps/binary-releases/releases/download/linuxdeploy-07333c6/linuxdeploy-x86_64.AppImage 36a2d7e274d12e1050d0e9ecfe11d339ed54720b2bec464c286d53f8b07f5c62`;
  - drop the `linuxdeploy-plugin-gtk.sh` and `linuxdeploy-plugin-gstreamer.sh` lines, and the
    now-unused `raw=` variable;
  - the comment names tauri-bundler 2.10.1 / tauri-cli 2.12.1, says linuxdeploy comes from a
    fixed build, and that the plugin scripts are embedded and pinned by the CLI's checksum (as
    t4-git-ui's `release.yml:226-236` words it).
  The *Bundle and sign* guard (`grep -q Downloading`, `:286`) stays: it fails the run if the new
  bundler downloads anything the pins miss.

- [ ] **Step 3: The check** (Decision 3), after *Bundle and sign*:

```yaml
      # linuxdeploy used to bundle this runner's libwayland-client (1.20), and a
      # newer host's Mesa EGL uses symbols 1.20 lacks: EGL_BAD_PARAMETER and no
      # window (1.7.2 on Ubuntu 26.04). linuxdeploy 07333c6 (tauri-bundler
      # 2.10.1) leaves it out itself; this fails the run if a later CLI or
      # linuxdeploy bundles it again. It checks the AppDir the bundler squashed,
      # which it leaves beside the image ("Clear stale bundle output" makes it
      # fresh); a missing AppDir fails too, rather than pass on nothing.
      - name: Check the AppImage has no libwayland-client
        if: runner.os == 'Linux'
        shell: bash
        run: |
          set -euo pipefail
          appdir='${{ matrix.bundle_dir }}/appimage/T4 Markdown Viewer.AppDir'
          [ -d "$appdir" ] || { echo "no $appdir: the bundler no longer leaves it" >&2; exit 1; }
          found=$(find "$appdir" -name 'libwayland-client*')
          [ -z "$found" ] || { echo "libwayland-client is back in the AppImage:" >&2; echo "$found" >&2; exit 1; }
```

- [ ] **Step 4: X11** (Decision 1). At the top of `main()`, before `exec_with_unicode_args()`
  (the re-exec keeps the environment, and nothing has started a thread yet):

```rust
fn main() {
    // Before anything starts a thread: `set_var` must not race another
    // thread's read. A block, since `#[cfg]` can't sit on an `if`.
    #[cfg(target_os = "linux")]
    {
        if let Some(backend) = gdk_backend(std::env::var_os("APPIMAGE").is_some()) {
            std::env::set_var("GDK_BACKEND", backend);
        }
        exec_with_unicode_args();
    }
```

```rust
/// The `GDK_BACKEND` to force: `x11` inside an AppImage, whatever the variable
/// held, else none. linuxdeploy's gtk hook used to export it (tauri#8541, a
/// crash on Wayland), and native Wayland ignores the app's window positions —
/// a restored frame, a torn-off tab's window; the hook embedded in
/// tauri-bundler 2.10 no longer does. The programs the app starts inherit it,
/// as they did under the old hook.
#[cfg(target_os = "linux")]
fn gdk_backend(in_appimage: bool) -> Option<&'static str> {
    in_appimage.then_some("x11")
}
```

  and a test beside `lossy_args`'s (Linux only, as those are):

```rust
    #[test]
    fn gdk_backend_is_x11_only_inside_an_appimage() {
        assert_eq!(gdk_backend(true), Some("x11"));
        assert_eq!(gdk_backend(false), None);
    }
```

  The edition is 2021 (`Cargo.toml:4`), so `set_var` needs no `unsafe`.

- [ ] **Step 5: The release skill.** `SKILL.md:152-157` names tauri-bundler 2.10.1, four pinned
  tools (AppRun, linuxdeploy `07333c6`, the appimage plugin, the runtime), and that the gtk and
  gstreamer scripts ride inside the bundler. Add, where the skill lists failures: *Check the
  AppImage has no libwayland-client* failing means a CLI or linuxdeploy change bundled it again
  (no window on a recent Mesa) — find which, and don't ship; the AppDir missing means the
  bundler stopped leaving it — change the check, don't drop it.

- [ ] **Step 6: Gates and commit.**

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

  Expected on Windows: no diff, no warning, 195 passed (the new test is Linux-only). Commit
  locally, two commits (Decision 4): the workflow and `main.rs` as `The AppImage starts on
  Ubuntu 26.04 and other distributions with a recent Mesa`, body naming the CLI bump, the pins,
  the check and X11; then the release skill as `docs: the release skill names tauri-bundler
  2.10.1's AppImage tools and the libwayland-client check`.

### Task 2: Linux gates

Through the Linux VM's Remote Control session, with Task 1's two commits as `git format-patch`
files in the brief (nothing is pushed yet): apply them on a clean `main` at the same commit, then
Task 1's gates. Expected: 199 passed (198 + the new one), no warning.

### Task 3: Push and dry run

After the change review (*Execution*). Each push and dispatch is asked for first.

- [ ] **Step 1:** Push `main`. CI green on all three.
- [ ] **Step 2:** Dispatch the Release workflow on `main` (`release` skill, *Checking the
  packaging without burning a version*); `signing` approved by the owner or on their word by
  `gh api`. Expected: every leg green; the *Bundle and sign* log shows no `Downloading`; the new
  check passes; the draft is deleted. A failure in *Bundle and sign* or the check: stop and
  bring it to the owner.

### Task 4: Walks on the dry run's builds

Each install asked for first, as in `archive/plans/tauri-2.12.md` W12.

| Walk | What | Pass | Where |
|---|---|---|---|
| W1 | The AppImage on the live GNOME Wayland desktop, the case that failed: launch, open `examples/kitchen-sink.md`. Also its AppDir listing: no `libwayland-client`. | A window opens; the document, its image and its diagram render; Settings says 1.7.2; *Check now* answers. | Linux VM |
| W2 | Windows: the dry run's installer run **while the installed app is open with two windows**, with the arguments the updater passes (`/P /UPDATE /R`: passive mode, `tauri.conf.json:37`; tauri-plugin-updater 2.13.1 `updater.rs:985-1000`, `config.rs:41-49`). The real updater exits the app right after starting the installer; leaving it open is the harder case for the new Restart Manager check. Then relaunch. | The installer closes the running app without hanging or a prompt it can't answer, installs, restarts it, and the restart restores both windows (session kept). | Windows |
| W3 | Pickers in the AppImage with no start folder: a fresh scratch config, Ctrl+O and the folder picker, once with `startup-mode` as the VM has it (`gsettings get org.gtk.Settings.FileChooser startup-mode`, recorded) and once at the other of `cwd` / `recent`. Changing it is a change to the owner's desktop settings: asked first, restored after. | Record where each opens. Inside the AppImage's mount (`/tmp/.mount_*`) is the Decision 2 finding. | Linux VM |
| W4 | In the AppImage: click an `https` link; click a link to a file that is neither a document nor a picture (it reveals the file in the file manager, `app.js:3708-3712`; e.g. a scratch `.md` holding `[x](tool.sh)` beside a `tool.sh`); a second instance on `$'caf\xe9.md'`. | The browser opens the link; the file manager shows `tool.sh` (not run); the second instance exits 0 with the first window untouched. Note any GIO warnings in the app's output. | Linux VM |
| W5 | In the AppImage on the live desktop: `GDK_BACKEND` as the app sees it, and a session restore of two windows at set places. | The WebKit web process the app starts reports `GDK_BACKEND=x11` (its `/proc/<pid>/environ`; the app's own shows only what it started with, not a later `set_var`), and the app's windows are X11 clients (`xlsclients` or `xwininfo -root -tree` lists them); the windows come back where they stood (positions readable under XWayland). | Linux VM |
| W6 | macOS: the dry run's `.dmg` app over `/Applications`; launch, open `kitchen-sink.md`, *Check now*. | Starts, renders, up to date; same self-signed cert. | Mac |

### Task 5: Records

- [ ] open-items: *The AppImage can't start on Ubuntu 26.04* ticked and moved to `closed-items.md`
  (*Bugs*) with W1's result; Decision 2's outcome recorded; any walk finding through triage.
- [ ] This plan filed under `archive/plans/` with its status; commit as `docs:`; ask before
  pushing.

## Verify

- Task 1's gates on Windows, Task 2's on Linux; CI on the push; the dry run green with the new
  check passing and no bundler download.
- W1–W6, each passed or brought to the owner.

## Risks

- **The fix reaches new downloads only:** an AppImage that can't start can't update itself.
  1.7.3's release notes could tell AppImage users to download it again (t4-git-ui's release body
  did); asked at the release.
- **Bundled 22.04 GIO modules** replacing the host's (reasoned): GSettings, mounts in the
  picker, `gio open` in the programs the app starts. W3 and W4.
- **Restart Manager in the Windows installer:** a running copy is now closed through Restart
  Manager; the in-app update always meets one. W2.
- **Other distributions** with a new Mesa are reasoned to be fixed the same way; only Ubuntu
  26.04 is walked.

## Execution

In this order, on `main`:

1. Task 1, in the main session (one workflow, one function, one skill section: no `coder`).
2. Task 2, through the Linux Remote Control session.
3. The change review loop (`CLAUDE.md` step 5).
4. Task 3: the push and the dry run, each asked first. The AppImage can only be walked from the
   dry run (a local AppImage build would bundle the building desktop's own libraries, not the
   runner's), so the push comes before the walks; that is why triage follows them.
5. Task 4, then triage, one item at a time; a fix goes through fix → review → triage.
6. Task 5, then the release question (1.7.3, held for this: `hold-1-7-3-for-appimage-fix`).
