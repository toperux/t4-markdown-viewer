# AppImage Child Environment Plan

> **Status (2026-11-14):** plan review done: six passes, the sixth found nothing to change. The
> owner's four rulings are in. Not approved; nothing is built.

**Goal:** on the Linux AppImage, the two programs the app starts get the host's environment, not
the bundle's, so that:

1. after an update installs, the app comes back by itself;
2. a link in a document opens in the browser.

Both fail today on Ubuntu 26.04, from one cause.

**Spec:** the owner's ruling of 2026-11-13 on the update walk's finding ("fix it as the next
change, before the Dependabot pull requests and the pinch"). The link fault was found while
measuring for this plan and is folded in, as the same cause produces it.

## Findings

Measured 2026-11-13 and 2026-11-14 by the Linux peer on Ubuntu 26.04.1 (Xvfb, X11, private D-Bus,
isolated XDG folders), on the released 1.8.11 and 1.9.1 AppImages, unless marked *read* (read in
source by the main session) or *reasoned*.

### The cause

- The AppImage's launcher is two programs. `AppRun` is a bash script linuxdeploy generates; its
  first line is `#! /usr/bin/env bash`. It sources `apprun-hooks/linuxdeploy-plugin-gtk.sh` and
  runs `AppRun.wrapped`, an ELF, which sets the environment and starts the app.
- `AppRun.wrapped` puts ten folders of the mounted image in front of `LD_LIBRARY_PATH`
  (`<mount>/usr/lib/`, `<mount>/usr/lib/x86_64-linux-gnu/`, … `<mount>/lib64/`). A value the user
  had set stays as the last entry; with none, the list ends in an empty entry.
- Everything the app starts inherits that. A host program started this way loads the bundle's
  libraries in place of the system's.
- The bundle carries `libsystemd.so.0` with version tags up to `LIBSYSTEMD_252` (it is built on
  the `ubuntu-22.04` runner). On Ubuntu 26.04, `/usr/bin/env`, `tr` and `basename` are the Rust
  coreutils and need `LIBSYSTEMD_254`. So with the app's environment:
  - `/usr/bin/env true` exits 1: ``version `LIBSYSTEMD_254' not found``;
  - `sh -c true` exits 0.

### Fault 1: the app does not come back after an update

- 1.8.11 → 1.9.1, three runs: the old process goes 3.9–10.5 s after *Update now*; no process and
  no window by 97 s. The file on disk is 1.9.1. Started by hand, it opens as 1.9.1 with the tab
  restored.
- stderr each run: `/usr/bin/env: <mount>/usr/lib/libsystemd.so.0: version 'LIBSYSTEMD_254' not
  found (required by /usr/bin/env)`.
- *Read:* Tauri 2.12.5 restarts with `Command::new(path).args(…).spawn()`, the environment
  untouched (`tauri/src/process.rs:83`). `path` is the AppImage file.
- So the failing step is the shebang of the **new** image's `AppRun`, run with the **old**
  image's `LD_LIBRARY_PATH`, while the old image is still mounted.
- The fix tried by hand, with a saved 1.8.11 environment and the 1.8.11 image still mounted:
  - unchanged: no process, the error above;
  - the old mount's entries removed from `LD_LIBRARY_PATH` (nothing was left, so it was unset):
    the window opens and the page renders;
  - `LD_LIBRARY_PATH` unset: the same.
- The stale variables left behind do no harm there: the new `AppRun.wrapped` overwrites every
  single-value variable and puts its own entries in front of every list.
- With the old mount gone, even the unchanged environment starts the app: the stale paths no
  longer exist. The restart is the one case where the old mount is alive.
- Tauri's single-instance lock is not involved: the new process never gets that far. So the
  *First runs* item in `open-items.md` is still unproven on Linux.

### Fault 2: a link opens nothing

- In the running 1.9.1 AppImage, a click on an `https` link opens nothing and shows no toast. A
  stub URL handler logged nothing.
- The app is left with a defunct child process. *Read:* that is not part of the fault. The `open`
  crate starts its launcher through a middle process that exits at once and that nothing waits
  for (`open-5.4.1/src/lib.rs:357-391`), so every link on every Linux install leaves one. It is
  also why there is no toast: the call succeeds once the launcher has started, whatever
  `xdg-open` then returns.
- By hand with the app's environment: `xdg-open https://…` exits 3, stderr `tr: …
  libsystemd.so.0: version LIBSYSTEMD_254 not found` and the same from `basename`; `gio open`
  exits 127 on the bundled glib.
- Controls: `xdg-open` from a plain shell exits 0 and the handler gets the URL. So does
  `xdg-open` with the app's environment once the mount's entries are out of `LD_LIBRARY_PATH`,
  everything else kept.
- *Read:* the page calls the opener plugin's `openUrl` (`app.js:5358`, and `:4608`, `:6259` for
  the release page). The plugin calls `open::that_detached`, which runs `xdg-open` as a plain
  `Command` with the process's environment (`open-5.4.1/src/lib.rs:278-291`, the version in
  `Cargo.lock`). A later version, 5.4.5, says in a comment that it leaves an AppImage's
  `LD_LIBRARY_PATH` alone on purpose, so no bump will fix this.
- *Read:* the app starts no other program. No `Command` exists in `src-tauri/src`; the dialog
  plugin uses GTK inside the process; single-instance uses D-Bus; the updater's helper programs
  (`pkexec`, `zenity`, …) are on the `.deb` and `.rpm` path, which an AppImage never takes.
- *Read:* showing a file in the file manager (`reveal_path`) talks to the file manager over D-Bus
  and starts no program, so it does not share the fault. Not measured.
- Not measured: a real browser (the stub kept the owner's Firefox profile out of it), any other
  distro, and since which version. *Reasoned:* any distro whose `env`, `tr` or `basename` links
  `libsystemd`; Ubuntu since it moved to the Rust coreutils.

### Why not clean the environment once, at startup

That would be a few lines and would cover every program the app ever starts. It breaks the app.

- On a normal start of 1.9.1 the app's working folder is `<mount>/usr` (`/proc/<pid>/cwd`), and its
  helpers' the same; the helpers are started by a path relative to it.
- The main binary has `RUNPATH $ORIGIN/../lib`, so it finds the bundle without the variable.
- WebKit's helper processes (`WebKitWebProcess`, `WebKitNetworkProcess`) have `RUNPATH $ORIGIN`,
  which is their own folder and does not hold `libwebkit2gtk`. Only `LD_LIBRARY_PATH` steers them
  to the bundle.
- 1.9.1's main binary started with the normal environment minus `LD_LIBRARY_PATH`: it starts, but
  both helpers load the **system** WebKitGTK (2.52.6), and no window is ever shown. With the
  variable kept, the same start renders normally.

So the environment is cleaned per spawn.

### What else the AppImage sets

`APPDIR`, `APPIMAGE`, `ARGV0`, `OWD`; `GTK_DATA_PREFIX`, `GTK_EXE_PREFIX`, `GTK_PATH`,
`GTK_IM_MODULE_FILE`, `GTK_THEME` (forced by the hook), `GDK_PIXBUF_MODULE_FILE`,
`GIO_MODULE_DIR`, `GI_TYPELIB_PATH`, `GSETTINGS_SCHEMA_DIR`, `GST_PLUGIN_SYSTEM_PATH`(`_1_0`),
`PERLLIB`, `PYTHONHOME`, `PYTHONPATH`, `PYTHONDONTWRITEBYTECODE`, `QT_PLUGIN_PATH`; and mount
entries in front of `PATH` and `XDG_DATA_DIRS`.

*Read:* the app adds two of its own at startup, inside an AppImage (`main.rs:1922-1937`):
`GDK_BACKEND=x11`, whatever it held, and `WEBKIT_DISABLE_DMABUF_RENDERER=1` on a Wayland desktop
unless the user had set it. Both reach the program a link opens and the restarted app. The
comment at `main.rs:1894` says the first is known: "The programs the app starts inherit it, as
they did under the old hook." *Reasoned, not measured:* a GTK browser started by a link on a
Wayland desktop runs under XWayland.

### What those do to a program a link opens (M7)

Host programs started by hand from the running 1.9.1's environment, in two variants, against a
plain shell as the control. **A:** only the mount's `LD_LIBRARY_PATH` entries removed (17
variables still name the mount). **B:** every mount entry removed from every variable.

| Program | A | B | B without `GTK_THEME` |
| --- | --- | --- | --- |
| `python3 -c "import sys"` | exits 1, `No module named 'encodings'` (`PYTHONHOME`) | as the control | as the control |
| `gsettings get …` | as the control | as the control | as the control |
| A GTK 3 window (gcr-viewer) | dark Adwaita, the control is light Yaru | the same as A, to the pixel | the control, to the pixel |
| A GTK 4 window (gnome-calculator) | Adwaita colours, the control is Yaru | the same as A, to the pixel | the control, to the pixel |

- `GTK_THEME` holds no mount path. The hook sets it whatever it was before, so the AppImage
  keeps no record of a value the user had.
- `xdg-open` on this desktop is a shell script that ends in the system's `gio open`; no Python or
  Perl in the chain. In A, `gio` ran with `GIO_MODULE_DIR` naming the bundle and still worked.
- Not measured: an input method (`GTK_IM_MODULE_FILE`), GStreamer, Perl and Qt programs, and the
  real browser. Firefox here is a snap and refuses to start from the peer's terminal, control
  included.

## Design

One rule: **a variable without the entries under `$APPDIR`**. Empty entries go with them (the
launcher's own artefact: an empty entry means "the current folder"). If nothing is left, the
variable is unset. A variable with no entry under `$APPDIR` is not touched at all.

On top of the rule, by name: `GTK_THEME`, `APPIMAGE`, `ARGV0` and `OWD` are dropped, and the two
the app sets itself at startup, `GDK_BACKEND` and `WEBKIT_DISABLE_DMABUF_RENDERER`, go back to
what they held before it did.

The app's own environment is never changed. The cleaned environment is put on the `Command` that
starts the other program, in both places: the restart and a link. Outside an AppImage (`APPDIR`
not set) nothing is cleaned and nothing changes.

The owner's four rulings of 2026-11-14 are in this design; *Rulings* at the end lists them with
what was turned down.

### The helpers (`src-tauri/src/main.rs`, Linux only)

```rust
/// `value`, a `:`-separated list, without the entries under `appdir`: `None`
/// when it has none (leave the variable alone), `Some(None)` when nothing else
/// is left (unset it).
#[cfg(target_os = "linux")]
fn outside(appdir: &Path, value: &OsStr) -> Option<Option<OsString>> {
    let entries: Vec<_> = std::env::split_paths(value).collect();
    if !entries.iter().any(|p| p.starts_with(appdir)) {
        return None;
    }
    let kept: Vec<_> = entries
        .into_iter()
        .filter(|p| !p.as_os_str().is_empty() && !p.starts_with(appdir))
        .collect();
    Some(if kept.is_empty() { None } else { std::env::join_paths(kept).ok() })
}

/// What to change in `vars`, the environment an AppImage gave this app, to
/// hand a program the host's: each name with its new value, `None` to unset.
/// `own` is what the app itself overwrote at startup, with what each held.
#[cfg(target_os = "linux")]
fn host_env(
    appdir: &Path,
    vars: impl Iterator<Item = (OsString, OsString)>,
    own: &[(&str, Option<OsString>)],
) -> Vec<(OsString, Option<OsString>)>

/// Give `cmd` the host's environment and a folder outside the mount to run in,
/// inside an AppImage; else leave it.
#[cfg(target_os = "linux")]
fn as_host(cmd: &mut Command)
```

- `host_env` gives, for each variable: `None` for `GTK_THEME`, `APPIMAGE`, `ARGV0` and `OWD`;
  `outside`'s answer where it has one; and for each name in `own`, the value it held before, or
  `None`.
- `as_host` reads `APPDIR`, `std::env::vars_os()` and the startup record, and calls `cmd.env` or
  `cmd.env_remove` for each change.
- **The working folder.** The app runs in `<mount>/usr` (measured; *Why not clean the
  environment once, at startup*), and a started program inherits that. A shell in the chain then
  exports `PWD` as that folder, the restarted runtime records it as `OWD`, and a browser is left
  in a folder that dies with the app (*reasoned*). So `as_host` also sets `cmd.current_dir`: the
  app's own `OWD`, the folder the user started it from, if it is still a folder; else `HOME`;
  else `/`.
- **The startup record:** `main()` already overwrites `GDK_BACKEND` and may set
  `WEBKIT_DISABLE_DMABUF_RENDERER` before any thread starts (`main.rs:1922-1937`). It now notes,
  in a `static OnceLock`, each name it wrote with the value read just before; the record is set
  at the end of that block, before `tauri::Builder`, so the "before any thread" rule holds. The
  block decides as before. `gdk_backend`'s comment (`main.rs:1894-1895`: "The programs the app
  starts inherit it, as they did under the old hook") stops being true and is reworded.
- `Path::starts_with` compares whole components, so `<mount>//usr/lib` is under `<mount>` and
  `<mount>-other` is not.
- A value that is not a list of folders (`DBUS_SESSION_BUS_ADDRESS=unix:path=/run/…`, an empty
  one) has no entry under the mount, so it is returned untouched, not split and joined.
- `APPDIR` itself goes, by the same rule. `APPIMAGE`, `ARGV0` and `OWD` name no mount path, but
  they tell whatever runs further down that it is this AppImage: a Tauri app started later from
  that browser would restart into this image (*read:* `process::current_binary` takes
  `APPIMAGE` first, `tauri-2.12.5/src/process.rs:48-53`). So they go too, for a link and for the
  restart. *Reasoned, not read:* the AppImage runtime sets all three itself for the new process,
  as it must on a first start from a file manager, which inherits none. V1 checks it.
- The `Exit` arm takes the image and the arguments from Tauri's `Env`, read at startup, and the
  app's own environment is never changed, so dropping them for a child costs the app nothing.

### Fault 1: the restart (`update.rs`, `main.rs`)

Inside an AppImage the app starts the new process itself. Elsewhere Tauri's restart is untouched.

*Read, Tauri 2.12.5:* `install_update` is an async command, so `app.restart()` does not spawn at
once. It sets a flag and asks the event loop to exit (`app.rs:606-628`). On the main thread the
`Exit` event then runs, in this order (`app.rs:1448-1454`): the plugins' exit hooks, the app's own
`.run` callback, `cleanup_before_exit()`, and, with the flag set, `process::restart`, which is
`Command::new(<the AppImage>).args(<the old arguments>).spawn()` and `exit(0)`
(`process.rs:48-88`).

- `update.rs`: a `static RESTARTING: AtomicBool`, set just before `app.restart()`. That call
  stays: it is what gets the main thread to the `Exit` event.
- `main.rs`, the `.run` callback (`:2148`), Linux only: on `RunEvent::Exit` with `RESTARTING` set
  and `app.env().appimage` present,
  1. `app.cleanup_before_exit()`, public, the call Tauri would make next; it drops the resources
     the single-instance lock is among;
  2. `Command::new(appimage)` with `app.env().args_os` minus the first, as Tauri builds it, then
     `as_host`, then `spawn`; a failure is printed to stderr;
  3. `std::process::exit(0)`.
  Tauri's own restart is never reached there. Outside an AppImage the arm does nothing and Tauri
  restarts as before.
- The new process gets the full cleaning, as a link's program does. Its own launcher then sets
  everything again, and its own `main()` sets the app's two variables again.
- This is a copy of six lines of Tauri (`app.rs:1452-1454`, `process.rs:83-88`) to keep in step
  with it on a Tauri bump.
- `.deb` and `.rpm` installs never restart (`installable()` is false) and have no `APPDIR`.

### Fault 2: links (`main.rs`, `Cargo.toml`, `app.js`, `capabilities/default.json`)

The plugin's command cannot be given another environment, so the page calls a command of the
app's own:

```rust
/// Open a web or mail link in the system's program for it. …
#[tauri::command(async)]
fn open_url(app: AppHandle, url: String) -> Result<(), String>
```

- **Which links:** the same four the plugin's default scope allows today, matched the same way
  (prefix, case-sensitive): `http://`, `https://`, `mailto:`, `tel:`. Anything else is refused
  with the error `Not a web or mail link: <url>`, which the page shows as a toast, where the
  plugin's refusal has its own wording today. A pure function, `opens(url) -> bool`.
- **Inside an AppImage** (`APPDIR` is set; behind `#[cfg(target_os = "linux")]`): each of
  `open::commands(&url)` in turn, the list the plugin itself walks (`xdg-open`, `gio open`,
  `gnome-open`, `kde-open`; `wslview` first under WSL; `open-5.4.1/src/unix.rs:23-50`). For each:
  `as_host`, all three stdio handles null, `process_group(0)`, `spawn`. The first that starts is
  the one; a thread waits for it, so it does not stay behind defunct. If none starts, the last
  error goes back to the page, which shows it as a toast.
  - `process_group(0)`: a browser the launcher starts is then not in the app's process group and
    does not die with a `Ctrl+C` in the terminal the app was started from. The plugin's own spawn
    detaches with `setsid`.
  - Where the launcher runs the browser in the foreground, the waiting thread idles for the
    browser's lifetime.
  - The launcher is looked up on the cleaned `PATH`. The mount's `usr/bin` holds only the app
    (measured), so nothing is lost by that.
- **Everywhere else** (Windows, macOS, a `.deb` or `.rpm` install, a debug build):
  `app.opener().open_url(url, None::<&str>)`, which is exactly what the plugin's command calls
  after its scope check.
- `Cargo.toml`: `open = "5"` under `[target.'cfg(target_os = "linux")'.dependencies]`, beside
  `gtk`, with a note that it is in the tree already through the opener plugin and this only makes
  it direct. `Cargo.lock` gains one `"open",` line in the app's own entry and no package (5.4.1
  satisfies `"5"`); it goes in the same commit, since every CI build is `--locked`. No feature is
  added, so nothing changes for the plugin's use of it.
- `app.js`: the three `openUrl(x)` calls become `invoke("open_url", { url: x })`; the
  `window.__TAURI__.opener` line goes.
- `capabilities/default.json`: `opener:allow-open-url`, `opener:allow-default-urls` and the
  `//urls` note go, since the page no longer calls the plugin. `reveal_path` uses the plugin's
  Rust API and needs no permission.

The URL cannot be read as an option by a launcher: it starts with one of the four prefixes.

### Not in this change

- `PYTHONDONTWRITEBYTECODE=1` still reaches the program a link opens: it names no mount path, and
  was not seen to change anything.
- A `GTK_THEME` the user had set before starting the AppImage is not put back; the launcher keeps
  no record of it. The program gets the desktop's theme.
- `XDG_DATA_DIRS` is left as the launcher's list minus the mount, which starts with a
  `/usr/share` the launcher added. If the user had none set, that is the value the program gets,
  where an unset one would have meant `/usr/local/share:/usr/share`.
- Outside an AppImage nothing is cleaned. One case exists there: a `.deb` or `.rpm` install
  started with the user's own `GDK_BACKEND=x11` on a Wayland desktop sets
  `WEBKIT_DISABLE_DMABUF_RENDERER=1`, and a link's program inherits it, as today.

## Tasks

Each task is one commit on `main`, local only.

**Task 1: the helpers and the restart.**
- `outside`, `host_env`, `as_host` and the startup record in `main.rs`; `RESTARTING` in
  `update.rs`; the `Exit` arm in the `.run` callback.
- Test (Linux): `outside` drops `<mount>/usr/lib/` and `<mount>//usr/lib`, keeps `/opt/userlib`
  and `<mount>-other/lib`; gives `Some(None)` for the launcher's own list with its trailing `:`;
  gives `None` (untouched) for `/usr/bin::/bin`, for an empty value and for
  `unix:path=/run/user/1000/bus`.
- Test (Linux): `host_env` on the measured environment in small: `PYTHONHOME=<mount>/usr/` and
  `APPDIR` unset, `PATH` and `XDG_DATA_DIRS` without their mount entries, `GTK_THEME`,
  `APPIMAGE`, `ARGV0` and `OWD` unset; `HOME` and an empty variable not in the result; with `own`
  holding
  `GDK_BACKEND` → `wayland` and `WEBKIT_DISABLE_DMABUF_RENDERER` → nothing, the first set to
  `wayland` and the second unset.
- Subject (a release-note line): `On Ubuntu 26.04, the AppImage comes back by itself after later
  updates`. "Later": the update **to** this release still does not come back (*Limits*).

**Task 2: links.**
- `open_url` and `opens` in `main.rs`, `open_url` registered in `invoke_handler`; `open` in
  `Cargo.toml` and `Cargo.lock`; the three call sites in `app.js`; the capability file.
- Test (all platforms): `opens` accepts the four prefixes and refuses `HTTP://x`, `file:///x`,
  `javascript:x`, `ftp://x`, `-x` and the empty string.
- Subject: `On Linux, links open from the AppImage on Ubuntu 26.04`. No "again": whether they
  ever did there is not known.

**Gates, each task:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, all with `--manifest-path src-tauri/Cargo.toml`. The Linux-only code and tests are
not compiled on this PC, so the Linux peer runs the same three before V0, at Task 1's commit and
at Task 2's, since each commit is meant to stand alone. The walk branch's workflow run (below)
compiles the head on `ubuntu-22.04`.

## Walk

On the VMs, never the host PC. The code reaches the peers on a pushed `walk/appimage-env` branch,
on the owner's go for that push.

### The walk image

Only a build below 1.9.1 takes the real 1.9.1 update, so the walk needs an AppImage of its own,
built as 1.9.0 (`--config '{"version":"1.9.0","bundle":{"createUpdaterArtifacts":false}}'`; the
updater compares the version in the config, and the signing key is not needed).

It cannot be built on the VM. The fault exists because the bundle carries the build machine's
`libsystemd`: the release is built on `ubuntu-22.04` (`release.yml:105`) and carries tags up to
`LIBSYSTEMD_252`. An image built on the 26.04 VM would carry 26.04's copy, the host's
`/usr/bin/env` would load it without complaint, and the walk would pass with or without the fix
(*reasoned*; the VM build itself was measured to work, 203 s). The VM's tauri-cli is also 2.11.5
with unpinned tools, where the release uses 2.12.1 and pinned ones.

So GitHub builds it. The walk branch carries one extra commit, never merged: a workflow file
`.github/workflows/walk-appimage.yml` that runs on a push to `walk/appimage-env`, on
`ubuntu-22.04`, with `permissions: contents: read`, no environment and no secrets. It is the
Linux leg of `release.yml` cut down: the build dependencies, the toolchain, tauri-cli 2.12.1,
*Pin the AppImage tools* as it stands, `cargo test --locked`, `cargo tauri build --bundles
appimage` with the config above, the release's guard that fails the run when the bundler
downloads a tool the pins do not cover (`release.yml:286-289`), and the image uploaded as an
artifact. The release's `libwayland-client` check reads the release's build matrix, so it is
done on the image in V0 instead. The peer fetches the image with `gh run download` if `gh` is
signed in on the VM; otherwise the owner hands it over.

### Checks

Before every update run, a fresh copy of the walk image: the update overwrites the file with
1.9.1.

| Check | Where | What | Pass |
| --- | --- | --- | --- |
| V0 | Linux | Control, on the walk image: Settings shows 1.9.0; `AppRun`'s first line; the highest `LIBSYSTEMD_` tag in its `usr/lib/libsystemd.so.0`; with the image mounted, `LD_LIBRARY_PATH=<mount>/usr/lib /usr/bin/env true`; and `find <mount> -name 'libwayland-client*'`. | 1.9.0; `#! /usr/bin/env bash`; 252; exit 1 with the `libsystemd` error; nothing found (an image carrying that library opens no window on 26.04, which would read as the fix failing). Shows the walk image has what makes the release fail. If not, the walk proves nothing: stop and report. |
| V1 | Linux | The walk image; one file open in a tab; take the 1.9.1 update. Three runs. | A window comes back by itself each time; one app process; the file on disk is 1.9.1; the tab is restored; no variable in the new process's environment (`/proc/<pid>/environ`) names the old mount, its `APPIMAGE` is the image file, and its `OWD` is the folder the image was first started from. This also proves the *First runs* item on Linux. |
| V2 | Linux | As V1, started from a shell with `LD_LIBRARY_PATH=/opt/userlib`. | Comes back as in V1; the new process's `LD_LIBRARY_PATH` is the new mount's entries followed by `/opt/userlib`. |
| V3 | Linux | The walk image, started with a dummy `WAYLAND_DISPLAY` so that the app sets its WebKit variable. First confirm the window still renders; if it does not, run without the dummy and report the WebKit half as not walked. Then click an `https` link and a `mailto` link, with a stub handler that logs its argument and its environment. Control: the same stub, started by `xdg-open` from the very shell that started the image. Once more with the shell holding `GDK_BACKEND=wayland,x11` and `LD_LIBRARY_PATH=/opt/userlib`. | The handler gets each URL. Its working folder (`pwd` in the stub) is the folder the image was started from, as the control's is. Its environment equals the control's, but for: `PYTHONDONTWRITEBYTECODE=1` (extra); `XDG_DATA_DIRS` (`/usr/share`, if the shell had none); `SHLVL` and `_`; and values that differ per start (`GIO_LAUNCHED_DESKTOP_FILE_PID`). Any other difference fails. In particular no variable names the mount, and `GTK_THEME`, `APPIMAGE`, `ARGV0`, `OWD` and `WEBKIT_DISABLE_DMABUF_RENDERER` are absent. `python3 -c "import sys"` run by the handler exits 0. No defunct child is left. |
| V4 | Linux | An `https` link with the real browser, without the dummy `WAYLAND_DISPLAY`, if the owner allows it on the VM. | The page opens. |
| V5 | Linux | The update dialog's release-notes button, in the walk image. | The handler gets the release page's URL. (The dialog's `Download…` call, `app.js:4608`, runs only where the install cannot update itself, so never in an AppImage; V7 covers that path.) |
| V6 | Linux | A link `ftp://example.com/x` in a document. | A toast reading `Not a web or mail link: ftp://example.com/x`, the new command's wording, so a page still calling the plugin fails this; nothing is started. |
| V7 | Linux | A debug build (no AppImage): an `https` link. | The handler gets the URL. |
| V8 | Linux | The walk image: a link to a file the viewer does not show (`notes.txt`). | Report only: does the file manager get the request over D-Bus. Unchanged code, never measured. |
| V10 | Linux | The walk image started from a terminal; a link opens a handler that stays running (`sleep 600` in the stub); then `Ctrl+C` in the terminal. Again, closing the terminal window instead. | The app ends; the handler's process is still running, both times. |
| V11 | Linux | The walk image started with `PATH=<folder>`, a scratch folder of symlinks to everything in `/usr/bin` except `xdg-open` (the image needs `bash` and its hook's tools from `PATH` to start at all, and `xdg-open` and `gio` share a folder); the stub's desktop file names it by full path; click an `https` link. | The handler gets the URL, through `gio open`; no variable in its environment names the mount. |
| V12 | Linux | As V11, the folder also without `gio`, `gnome-open` and `kde-open`; click an `https` link. | A toast with the error; nothing is started. |
| V9 | Windows, macOS | A debug build: an `https` link and a `mailto` link. | The browser and the mail program open, as on 1.9.1. |

## Limits to put to the owner at triage

- **The update from 1.9.1 still does not come back on Ubuntu 26.04.** The outgoing version does
  the restart, and 1.9.1 is already out. The first update that comes back is the one **from** the
  release carrying this fix. No fix is possible.
- **Links stay broken in an AppImage of 1.9.1 or older** on such a distro until it is updated.
- **Only Ubuntu 26.04 is walked.** Other distros are reasoned.
- **The restart inside an AppImage is the app's own copy of Tauri's**, to be read against Tauri's
  on a bump.
- **What taking back `GDK_BACKEND` and the WebKit variable gains on a Wayland desktop is
  reasoned.** The walk runs on X11 and shows only that a started program no longer has them.
- **`gnome-open`, `kde-open` and `wslview` are not walked** with the cleaned environment; `gio
  open` is (V11).
- **A start folder inside another AppImage's mount** is not recognised (review pass 5, finding 2,
  not taken): the image started from a program that itself runs inside its own AppImage mount
  gives an `OWD` there, and a link's program, or the restarted app, would run in a folder that
  dies with that other program. Rare; the check would be a guess at other mounts' names.
- **The four under *Not in this change***: `PYTHONDONTWRITEBYTECODE`, a `GTK_THEME` of the
  user's own, `XDG_DATA_DIRS` when the user had none, and installs that are not an AppImage.
- **The real browser is not walked** unless the owner allows it on the VM (V4).

## Rulings (the owner, 2026-11-14)

1. **How much to clean for a link:** every mount entry out of every variable, and `GTK_THEME`
   dropped. Turned down: the library path alone (M7's variant A: a Python program fails, GTK
   programs come up in the wrong theme), and every mount entry with `GTK_THEME` kept (variant B:
   the theme is still wrong).
2. **How the restart gets its environment:** the app starts the new process itself, with a
   cleaned `Command`. Turned down: `set_var` in the app's own environment just before
   `app.restart()`, or in the `Exit` arm. Both change the environment while other threads run,
   against the note at `main.rs:1920`, and that race cannot be walked.
3. **The app's own two variables:** a started program gets them as they were before the app set
   them. Turned down: leaving them, as the comment at `main.rs:1894` describes today.
4. **Which launchers get the cleaned environment:** the plugin's whole list, through
   `open::commands`, with `open` named in `Cargo.toml`. Turned down: `xdg-open` alone, falling
   back to the plugin and today's environment.

## Chosen without asking

- **The walk image is built by GitHub on `ubuntu-22.04`**, not on the VM (review pass 1,
  finding 1). A VM build cannot show the fault. The alternative, a 22.04 container on the VM,
  needs installs there.
- **V0 is a check of the walk image**, not a second build without the fix: the released 1.8.11
  already failed three times out of three, and V0 shows the walk image has the same ingredients.
- **`process_group(0)` on the link's spawn** (finding 3).
- **The two subjects reworded** (finding 10).
- **V11 added** with ruling 4, so that a launcher other than `xdg-open` is run once.
- **`APPIMAGE`, `ARGV0` and `OWD` are dropped too**, for a link and for the restart (review
  pass 3, finding 3, which the reviewer marked for the owner). Taken as the same principle the
  owner ruled in 1 and 3: a started program should not be told it is this AppImage. Keeping them
  would have been a limit to list; dropping them costs three names. The owner can reverse it at
  the go.
- **The restarted app gets the full cleaning**, not the library path alone: with ruling 2 it is
  the same call as a link's, and it is how the app starts from a desktop.
- Findings 6, 7, 8, 9 and 11: corrections to this plan's text and checks, taken as given.
- Review pass 2, all seven findings, none a decision: V11 and V12 given a setup that can be run;
  `gdk_backend`'s comment reworded; V3 run with a dummy `WAYLAND_DISPLAY` and a whole-environment
  comparison; the walk workflow keeps the release's download guard and gets read-only
  permissions; `open` declared for Linux only, the lock in the same commit; the gates run at each
  commit; V10 also closes the terminal; Task 1's subject cut to 70 characters.
- Review pass 3, findings 1, 2, 4, 5 and 6: V3's comparison given a like-for-like control and its
  expected differences written down beforehand; V4 run without the dummy display; V11's stub named
  by full path; the `libwayland-client` check moved into V0; V6 tied to the new command's wording.
- Review pass 4, its one finding: a started program is given a working folder outside the mount.
- Review pass 5, finding 1: the working-folder measurement is now in *Findings*.
