# drive-app on macOS

**Goal:** the `drive-app` skill can launch the debug build on macOS and drive it as it does on Linux: `eval`,
clicks, keys, drags, `shot`, several windows, and the native Open / Select Folder dialogs.

**Branch:** `drive-app-macos`.

## Why this needs an app change

- Windows drives the page over CDP (WebView2). Linux drives it over WebDriver: `tauri-driver` →
  `WebKitWebDriver`.
- macOS's webview is WKWebView. Apple ships no WebDriver for it: `safaridriver` drives Safari only.
  `tauri-driver` builds for Windows and Linux only: on anything else its `main` prints "not supported on this
  platform" (read in its 3.0.0-alpha.1 source). Safari's Web Inspector can attach to a debug build by hand, but
  it has no scriptable interface.
- So the only way to run JS in the page from a script is a WebDriver server **inside the app**. The Tauri docs'
  WebdriverIO page says its embedded server is how it supports macOS. That's from the docs; the repo hasn't
  tested it.

## Approach

Owner's decisions, 2026-09-28:

- The in-app plugin, behind a Cargo feature. Rejected: the macOS-only danielraffel plugin, and an OS-only skill
  with no `eval`.
- Real OS mouse input from a small script of our own, because the plugin's input is not real (below).
- The skill launches the app itself. Rejected: the plugin's `tauri-webdriver` CLI.

### The plugin

`tauri-plugin-webdriver` (Choochmeque, MIT, 0.2.3, updated 2026-09-01, ~129k downloads). Read in its source
(0.2.3):

- **Startup.** It starts an HTTP WebDriver server in `setup` on every launch, with no env gate, on
  `127.0.0.1:4445` (`TAURI_WEBDRIVER_PORT` moves it).
  - If a stale app still holds that port, the new app's server thread panics silently, and every call goes to
    the old app. Hence the port check before launch.
- **Permissions.** It needs none. It defines no commands and no permissions, and all its JS goes through native
  `evaluateJavaScript` / `callAsyncJavaScript`, which the page's CSP doesn't cover. `capabilities/default.json`
  stays as it is.
- **Sessions.** It ignores the capabilities sent at session start. So `WD_PORT=4445 wd.mjs start x` attaches to
  the running app, and `wd.mjs` needs no change.
- **Endpoints.** It routes every endpoint `wd.mjs` uses, with standard element references.
- **Windows.** Each window is its own handle. Handles are window labels in arbitrary order.
- **`eval` results.** They are converted to JSON natively. Returning a DOM node or a function probably errors
  or comes back `null`: reasoned from WebKit's `callAsyncJavaScript`, not measured. Return plain values.
- **Its input is synthetic JS in the page**, not OS input:
  - `click <sel>`: `scrollIntoView` + `el.click()` + `focus()`. Not a user gesture, no `pointerdown`, never
    refused when something covers the element, and it scrolls the page.
  - `/actions` pointer events: plain `MouseEvent`s at `elementFromPoint`. No `PointerEvent`s, so the app's
    drags (tab reorder, sidebar resize, diagram / picture pan) don't react. No `click`, so `drag "x,y"` clicks
    nothing. It ignores `origin` and modifier keys, so `click "<sel>" Shift` lands at the viewport's (0,0).
  - `key`: an untrusted `KeyboardEvent` on `document.activeElement`. Shortcuts the app handles in `onKeydown`
    work (`ctrl = ctrlKey || metaKey`, so `Meta+o` and `Control+o` both do). There are no default actions, so
    Escape won't close a `showModal()` dialog, Tab won't move focus, and the native menu never sees the key.
  - Copy buttons probably fail without a user gesture. That is reasoned from WebKit's rules, not measured.
- **Delegate.** It replaces wry's `WKUIDelegate`, so a feature build loses file `<input>`, media permission,
  and `window.open` / `_blank` handling. The app uses none of them (grepped).
- **Compatibility.** It needs tauri ≥2.10, the tauri-plugin build crate ≥2.5.3 and Rust 1.90. The lock has
  tauri 2.11.6 and tauri-plugin 2.6.3, and rustc here is 1.98.
- **Exposure.** While a feature build runs, any local process can drive it over `127.0.0.1:4445`,
  unauthenticated. That is the same exposure as the Windows CDP port and Linux's `tauri-driver`.

### The gate

A Cargo feature, `webdriver`, not the plugin README's `[target.'cfg(debug_assertions)'.dependencies]`:

- Cargo matches that table against `rustc --print cfg`, which lists `debug_assertions` unless given `-O`
  (checked).
- It is passed no profile flags, so the release build would compile the plugin too, and only the `#[cfg]` on
  the registration would keep it out. That part is reasoned, not measured.

A plain `cargo build`, `cargo test` and CI's release build (`cargo tauri build -- --locked`, no features) never
contain the plugin. Measured in a copy of `src-tauri`: with the dependency added, the default build's graph
(`cargo tree -e normal,build`) is byte-identical on aarch64-apple-darwin, x86_64-linux-gnu and
x86_64-windows-msvc.

### Real input

A JXA script (`osascript -l JavaScript`) posts real mouse events through CoreGraphics (`CGEventCreateMouseEvent`
+ `CGEventPost`), with modifier flags and a click count. This is the counterpart of `click.ps1` and
`xdotool click`.

- Checked here: JXA reaches those functions and constants, and an event can be created.
- Unchecked: posting one, which needs Accessibility.
- If JXA can't post, the fallback is `brew install cliclick`, which needs an owner decision at that point.

Real keys go through System Events (`keystroke` / `key code`). Native dialogs go through System Events too.

## Changes

### App (only in the `webdriver` feature)

1. `src-tauri/Cargo.toml`:

   ```toml
   [features]
   # drive-app on macOS: an in-app WebDriver server. Never in a release build.
   webdriver = ["dep:tauri-plugin-webdriver"]

   [dependencies]
   tauri-plugin-webdriver = { version = "0.2", optional = true }
   ```

   **`Cargo.lock` changes, and is committed with the change**, since CI builds with `--locked`. Measured in a
   copy:
   - 21 packages are added, among them glib 0.22 beside the existing 0.18.5.
   - No existing package's version is removed or changed.
   - About 35 existing entries' dependency lists change: `glib` becomes `glib 0.18.5`, and features are merged
     into objc2-app-kit, objc2-web-kit, tokio, tracing and tower.

   CI's `clippy` and `test` run without the feature, so the one gated line below is never linted there.
2. `src-tauri/src/main.rs`, in `main()`, next to the `#[cfg(target_os = "macos")] let builder =
   builder.menu(…)` step:

   ```rust
   #[cfg(feature = "webdriver")]
   let builder = builder.plugin(tauri_plugin_webdriver::init());
   ```

### Skill

3. New `.claude/skills/drive-app/scripts/macmouse.js`. Run it with `osascript -l JavaScript macmouse.js
   <verb> …`, in screen points:
   - `click x,y [mods]`
   - `dblclick x,y`
   - `drag "x,y x,y …" [mods]`: press at the first point, move through the rest, release at the last.

   `mods` is a `+`-joined list of `cmd`, `shift`, `alt` and `ctrl`. The event types matter. This is reasoned from
   AppKit / WebKit, not measured; V5 checks it:
   - `drag`:
     - a `mouseMoved` to the first point, then `leftMouseDown`;
     - interpolated `leftMouseDragged` steps with a few ms between them, then `leftMouseUp`.
     - A `mouseMoved` while the button is down would read `buttons === 0` in the page, and `onDragMove` cancels
       on that (`app.js`, `if (event.buttons === 0) return onDragCancel()`).
   - `dblclick`: two down/up pairs inside the double-click interval, with `kCGMouseEventClickState` set to 1
     and then 2 on both the down and the up. WebKit takes `detail` and `dblclick` from the click count.
   - Modifiers: `CGEventSetFlags` on every event of the gesture.

   **Page → screen mapping.** `macos.md` gives one `eval`: `invoke("window_origin")` (a page global, registered
   on every platform) returns the window's inner position in physical px and its scale. Screen point =
   `pos / scale + css px`, at zoom 1.
   - Read in tao 0.35.3's macOS source: the position is the content rect below the title bar, flipped to a
     top-left origin on the main display, times `backingScaleFactor`, and `scale_factor` is that same factor.
     So `pos / scale` is global points, and it holds across displays of different scales, since both values
     come from the same window.
   - Reasoned: that CGEvent uses the same global point space, and that CSS px = points at zoom 1.
4. New `.claude/skills/drive-app/scripts/macdialog.sh`: `<pid> <path>` | `<pid> --cancel` | `<pid> --dump`,
   the macOS counterpart of `xdialog.sh`. It uses System Events on the process with that unix id. The panel is a
   sheet on the app's window (dialog plugin `set_parent` → rfd `beginSheetModalForWindow`).
   - `<path>`: Cmd+Shift+G opens *Go to Folder*; type the path and press Return, then Return again to press
     Open.
   - `--cancel`: Escape.
   - `--dump`: the process's windows and sheets, and the dialog's buttons.
   - The last line says whether the dialog closed.

   How many Returns the file and the folder pickers each need, and whether System Events can see the panel's
   buttons at all, is to be measured (V4).
5. New `.claude/skills/drive-app/macos.md`, laid out like `linux.md` (prerequisites, scripts, sections 1–5,
   gotchas). What it covers:
   - **Prerequisites.** Node 20+ (22 is installed). Two one-time grants to the terminal that runs Claude Code,
     both needed by `macmouse.js` and `macdialog.sh` / real keys:
     - Accessibility, in System Settings → Privacy & Security → Accessibility.
     - Automation, the "… wants to control System Events" prompt on first use.
   - **Build.** `TAURI_CONFIG='{"identifier":"com.montevirgen.t4-markdown-viewer.audit"}' cargo build
     --manifest-path src-tauri/Cargo.toml --features webdriver`.
     - The rename is still needed: `/Applications/T4 Markdown Viewer.app` is installed, and single-instance on
       macOS keys its socket on the identifier (`/tmp/<identifier>_si.sock`).
     - A plain `cargo build` at the end drops both the rename and the plugin.
   - **Settings.** `config::dir()` is `dirs::config_dir()`, which on macOS is `$HOME/Library/Application
     Support`. `dirs-sys` 0.5 reads `$HOME` before the password database (read in its source). So launching
     with `HOME=$S/home` (created first) keeps the user's `~/Library/Application Support/t4-markdown-viewer`
     untouched, as the XDG dirs do on Linux. Side effects:
     - WebKit's own data probably still goes to the real `~/Library/WebKit/…` and `~/Library/Caches/…`:
       AppKit's home lookups ignore `$HOME`. The app keeps nothing there that matters, since it uses no
       `localStorage`. That is reasoned, not measured.
     - The dialogs' fallback start folder (`config::start_dir`) is the scratch home.
     - `repo_root`, which stops its climb at the home folder, climbs to `/` instead. Harmless: there's no `.git`
       above the user's home.
     - The user's own themes aren't there; copy one in if it's under test.
   - **Ports.** `lsof -nP -iTCP:4445 -sTCP:LISTEN` must be empty; see the stale-app trap above.
   - **Launch.** Start the binary in the background, with its env on the same command and absolute paths:

     ```bash
     S=<scratchpad>/app; mkdir -p $S/home
     HOME=$S/home "$PWD/src-tauri/target/debug/t4-markdown-viewer" "$PWD/examples/kitchen-sink.md"
     ```

     - Launch with the Bash tool's `run_in_background: true`, not a trailing `&`: the call would wait on the
       inherited stdout. That is reasoned, and matches SKILL.md's Windows step.
     - Then poll `WD_PORT=4445 wd.mjs start x` until it prints `session`. The plugin takes a moment to listen.
       Until then `start` fails with "tauri-driver is not listening on …": `wd.mjs`'s wording, which is fine
       here.
     - Then poll `eval "innerWidth"` and, if a file was named, `eval "tabs.length"`, as on Linux.
     - A relaunch needs a new `start`: the old id in `$TMPDIR/t4-wd-session` gives `invalid session id`.
     - Get the PID with `pgrep -f '^[^ ]*target/debug/t4-markdown-viewer'`; the anchor keeps it from matching
       the shell running it.
     - A second window: `eval "invoke('open_window', {path: '<abs file>'})"`. Synthetic Shift+click can't open
       one.
     - Every `wd.mjs` call needs `WD_PORT=4445`.
   - **No headless display.** macOS has no Xvfb, so the window opens on the owner's desktop and can take focus.
     Say so before the first launch.
   - **Drive.** As `linux.md` §3, with these differences:
     - `wd.mjs click`, `drag` and `key` are synthetic (see *The plugin* above). Use them for plain clicks and
       for the app's own shortcuts only.
     - Use `macmouse.js` for anything that needs a gesture, a pointer event, a modifier or a double click.
     - Use System Events keys for Escape on a dialog, Tab and menu keys.
     - The app's shortcuts use Cmd (`key Meta+o`). A Ctrl+click on a tree row is ignored on macOS (`app.js`,
       `isMac && event.ctrlKey`), and Cmd+click opens a new tab.
   - **OS keystrokes.** `osascript` System Events `keystroke` / `key code`, after setting the app's process
     frontmost.
     - Check `frontmost` before typing, and stop if the app isn't in front, or the keys land in the owner's
       editor, the same risk as Windows' `AppActivate`.
     - **The same before every `macmouse.js` call.** A raw binary started from a terminal can open behind it.
       A first click on an inactive window probably only activates it, since WKWebView doesn't accept the first
       mouse (reasoned). And a covering window takes the click.
     - Real keys pass through the app's menu first (`macos_menu` in `main.rs`). This is reasoned from AppKit and
       from `main.rs`'s own comment on Cmd+W; V5 checks it:
       - Shift+Cmd+W is the native Close Window. The page also handles Shift+`w` as close-tab, so if the page
         saw it first it would close a tab.
       - Cmd+Q quits, Cmd+H hides, Opt+Cmd+H hides the others, Cmd+M minimizes and Ctrl+Cmd+F goes full screen.
       - Cmd+A/C/V/X/Z and Shift+Cmd+Z go through the Edit menu.
       - Cmd+O, Cmd+W and Cmd+T are not in the menu, so they reach the page.
   - **Window size.** `wd.mjs raw POST /window/rect '{"width":…,"height":…}'` sets the **outer** size
     (the title bar included) in **physical** px: on Retina, `width: 1200` is 600 pt. Whether it honours the
     minimum size is to be measured.
   - **Clipboard.** `pbcopy` / `pbpaste`. This is the owner's own clipboard: save it first and put it back.
   - **Clean up.**
     - `wd.mjs stop` ends only the session.
     - `kill <pid>`, then check that `pgrep -lf '^[^ ]*target/debug/t4-markdown-viewer'` is empty and that
       4445 is free.
     - Warn: BSD `pgrep -a` means "include ancestors", so `linux.md`'s `pgrep -af` would match the calling
       shell. Use `-lf`.
     - Then a plain `cargo build`.
     - `kill` leaves `/tmp/com_montevirgen_t4_markdown_viewer_audit_si.sock` behind. That is harmless: the next
       launch removes it when it can't connect.
6. `SKILL.md`: the `description` and the opening paragraph name macOS (WKWebView / in-app WebDriver) and point
   to `macos.md` for steps 1, 2, 4 and 5, as they do for `linux.md`. Nothing else in `SKILL.md` changes.

### Not in scope

- A whole-window screenshot (title bar included). `wd.mjs shot` covers the page. `screencapture -l <window id>`
  would need Screen Recording permission and a window-id lookup. Add it when a check needs the title bar.
- Real mouse wheel events.
- Switching Linux or Windows to the plugin.
- Cross-window tab drag, which is Windows-only in the app.

## Verification (the execute step, on this Mac)

Each result goes into `macos.md` as measured, or the step is changed to match what happened. The owner grants
Accessibility and Automation before V2.

- **V1. Build.**
  - A build with `--features webdriver` compiles.
  - A plain `cargo build` and `cargo test` still pass.
  - Without the feature, `cargo tree -i tauri-plugin-webdriver` answers "did not match any packages".
  - In `git diff Cargo.lock`, no existing package's version is removed or changed.
- **V2. WebDriver.** Launch `examples/kitchen-sink.md`, then check:
  - `eval "tabs.length"` is 1.
  - `wd.mjs click "#settings-btn"` opens settings (`els.settings.open` true); close it with `eval
    "els.settings.close()"`.
  - `key Meta+o` opens the Open dialog (`pickerOpen` true). Bring the app to the front, check `frontmost`, and
    close the dialog at once with a System Events Escape (`key code 53`). `pickerOpen` is false again.
  - `shot` works (read the PNG and note its pixel scale).
  - With a second window open (`invoke('open_window', …)`), `window` switches between them. Close it again
    with `eval "(setTimeout(() => appWindow.close(), 100), 1)"` from that window, then `window 0`. The defer
    lets the `eval` answer before the window is gone.
  - `stop`, then a second `start`, attaches again.
- **V3. Settings isolation.** After a run, `$S/home/Library/Application Support/t4-markdown-viewer/` has the
  files, and the real folder's contents and mtimes are unchanged.
- **V4. Dialogs.**
  - **File.** `click "#open-btn"` (the bar's button; `#empty-open-btn` is hidden once a tab is open), then
    `macdialog.sh` opens the file.
  - **Folder.** Check `eval "state.folder"` is `null` first: `showFolder()` is a toggle. `eval "showFolder()"`
    shows the file's folder without a picker. Then `click "#sidebar-name"` opens the folder picker, and
    `macdialog.sh` picks `<abs>/examples/img`, a folder other than the one on show, so `state.folder` changes to
    it.
  - `--cancel` leaves `pickerOpen` false.
- **V5. Real input.** Each check brings the app to the front first.
  - **Tab drag.** `eval "openTab('<abs>/examples/second.md')"`; with one tab the strip is hidden. Drag one
    `.tab` centre past the other with `macmouse.js drag`. `tabs.map(t => t.path)` comes back reversed.
  - **Cmd+click.** `eval "openFolder('<abs>/examples')"`, then `macmouse.js click … cmd` on
    `.tree-row[data-path$="second.md"]`. `tabs.length` goes up by 1, and the old tab stays.
  - **Copy.** Back to kitchen-sink first: `eval "activateTab(tabs.find(t =>
    t.path.endsWith('kitchen-sink.md')).id)"`. Save the owner's clipboard with `pbpaste`, then `macmouse.js
    click` on the first `h1 button.copy-section`. `pbpaste` starts with that heading's Markdown. Restore the
    clipboard.
  - **Double click.** Optionally first, while kitchen-sink is active: open its Mermaid diagram (`wd.mjs click
    "div.mermaid-diagram"`), then `macmouse.js dblclick` on `#diagram-view`, and `zoomed.scale` goes 1 → 2.5.
    Close it with `eval "els.diagramDialog.close()"` and check `els.diagramDialog.open` is false: while it's
    open the page behind is inert and `onKeydown` ignores keys. Then `eval
    "openTab('<abs>/examples/img/icon.png')"` for the picture tab. Read `eval "picture.fit"` (true), then
    `macmouse.js dblclick` on `#image-view`: it flips to false.
  - **Escape.** `key Meta+,` opens settings (the page handles it), then a System Events `key code 53`. Expect
    `els.settings.open` to be false.
  - **Menu.** Open a second window with `invoke('open_window', …)` and give the front one ≥2 tabs. A real
    Shift+Cmd+W should drop `window` handles from 2 to 1, with `tabs` in the remaining window unchanged. A
    closed tab would leave both handles. Then `window 0`, since the session may point at the gone window.
- **V6. Clean up.** No debug-app process is left, 4445 is free, and the last `cargo build` is plain.
