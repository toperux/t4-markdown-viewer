# Driving the app on macOS

WKWebView has no WebDriver of its own (`safaridriver` drives Safari only, and
`tauri-driver` doesn't build for macOS), so on macOS the WebDriver server lives
**inside the app**: `tauri-plugin-webdriver`, compiled in only with the
`webdriver` Cargo feature. `scripts/wd.mjs` talks to it as it talks to
`tauri-driver` on Linux. Section 3 of `SKILL.md` (page globals, the visibility
helper, forward-slash paths) applies here too, and `eval` takes one expression,
as in `linux.md` *3. Drive*.

The plugin's input is **synthetic JS in the page**, not OS input, so real
clicks, drags and keys go through the OS: `macmouse.js` (CoreGraphics events)
and System Events. See *3. Drive*.

Proven on macOS 26.7 (Apple Silicon, one Retina display) against 1.7.1, from
iTerm2.

## Prerequisites

- Node 20 or later.
- Two one-time grants to the terminal that runs Claude Code: Accessibility
  (System Settings → Privacy & Security → Accessibility) and Automation of
  System Events (a prompt on first use). `macmouse.js`, `macdialog.sh` and real
  keys need them. `osascript -l JavaScript -e 'ObjC.import("ApplicationServices");
  $.AXIsProcessTrusted()'` says whether Accessibility is there.
- No Xvfb on macOS: **the window opens on the owner's desktop** and takes
  focus. Say so before the first launch. The owner's own mouse movements reach
  the page too (seen in an event log mid-run), so ask them to keep off the
  trackpad during a real-input check.

## Scripts

- `wd.mjs` — as in `linux.md`, with `WD_PORT=4445` on every call.
- `macmouse.js` — `osascript -l JavaScript macmouse.js click x,y [mods]` |
  `dblclick x,y` | `drag "x,y x,y …" [mods]`: real mouse events in screen
  points. `mods` is `cmd`, `shift`, `alt`, `ctrl` joined with `+`.
- `macpt.sh` — `macpt.sh "<css selector>"` (any quotes) prints the screen
  point of the centre of every match, `["x,y", …]`, for `macmouse.js`.
- `macdialog.sh` — `<pid> <path>` | `<pid> --cancel` | `<pid> --dump`: answers
  the Open / Select Folder panel.

## 1. Before launching

- **Rename the debug build and add the plugin:**

  ```bash
  TAURI_CONFIG='{"identifier":"com.montevirgen.t4-markdown-viewer.audit"}' \
    cargo build --manifest-path src-tauri/Cargo.toml --features webdriver
  ```

  The rename matters as on the other platforms: `/Applications/T4 Markdown
  Viewer.app` may be running, and single-instance keys its socket on the
  identifier (`/tmp/<identifier>_si.sock`). Rebuild after every `src/` edit.
  A plain `cargo build` at the end drops both the rename and the plugin.
- **Give it its own settings.** `config.json` and `session.json` live in
  `$HOME/Library/Application Support/t4-markdown-viewer`, and the app reads
  `$HOME`. Launch with `HOME` pointing into the scratchpad and the owner's own
  folder is never read or written (measured: contents and mtimes unchanged).

  ```bash
  S=<scratchpad>/app; mkdir -p $S/home
  ```

  - WebKit ignores `HOME`: its data goes to the real
    `~/Library/WebKit/t4-markdown-viewer` and `~/Library/Caches/t4-markdown-viewer`
    (measured). These are named after the unbundled binary, apart from the
    installed app's, and hold nothing the app relies on.
  - The owner's own themes aren't there; copy one into
    `$S/home/Library/Application Support/t4-markdown-viewer/themes/` if it's
    under test.
  - The dialogs' fallback start folder is the scratch home.
- **Port.** `lsof -nP -iTCP:4445 -sTCP:LISTEN` must be empty. A stale app
  holding it makes the new app's server fail silently, and every call then goes
  to the old app.

## 2. Launch

With the Bash tool's `run_in_background: true` (it ends with exit 143 when the
app is killed; that's expected), absolute paths:

```bash
HOME=$S/home "$PWD/src-tauri/target/debug/t4-markdown-viewer" "$PWD/examples/kitchen-sink.md"
```

Then, in a separate call:

```bash
W=.claude/skills/drive-app/scripts/wd.mjs; export WD_PORT=4445
for i in $(seq 1 30); do node $W start x 2>&1 | grep -q '^session' && break; sleep 1; done
for i in $(seq 1 30); do [[ $(node $W eval "typeof openTab") == '"function"' ]] && break; sleep 0.5; done
node $W eval "[innerWidth, tabs.length]"
pgrep -f '^[^ ]*target/debug/t4-markdown-viewer'     # the PID, for macdialog.sh
```

- The plugin ignores the capabilities, so `start x` just opens a session on
  the running app. It can attach before `app.js` has run (an `eval` once got
  `Can't find variable: openFolder`), hence the second loop. Until it listens,
  `start` says "tauri-driver is not listening"; that is `wd.mjs`'s wording.
- A relaunch needs a new `start`: the old session id gives `invalid session id`.
- The `pgrep` anchor keeps it from matching the shell running it.
- `shot` works; the PNG is in device pixels (2× on Retina). Read it.

## 3. Drive

`wd.mjs eval` is the workhorse, as on Linux. The rest of `wd.mjs` is
synthetic (read in the plugin's source):

- `click "<sel>"` is `scrollIntoView` + `el.click()` + `focus()`: no user
  gesture, no `pointerdown`, never refused when something covers the element.
  Fine for plain buttons (`#settings-btn`, `#open-btn`, `#sidebar-name`).
- `drag` sends plain `MouseEvent`s and ignores modifiers and `origin`. The
  app's drags listen for pointer events, so they don't react, and `click
  "<sel>" Shift` lands at the viewport's (0,0).
- `key` is an untrusted `KeyboardEvent`: the app's own shortcuts work
  (`key Meta+o`, `key Meta+,`; `ctrl` is `ctrlKey || metaKey`), but there are
  no default actions — Escape doesn't close a dialog, Tab doesn't move focus.

So `SKILL.md` §3's advice to use `click` rather than `el.click()` for a user
gesture doesn't hold here: both are `el.click()`. Use `macmouse.js`.

Everything that needs a real gesture goes through the OS. **Bring the app to
the front first, every time**, and check it's there: a click or a key goes to
whatever is in front — the owner's terminal or editor.

```bash
# Prints true only when the app really is in front; osascript exits 0 either way.
front() { osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $1) to true" \
  -e 'delay 0.3' -e "tell application \"System Events\" to get frontmost of (first process whose unix id is $1)"; }
M=.claude/skills/drive-app/scripts/macmouse.js; PT=.claude/skills/drive-app/scripts/macpt.sh
P=$($PT '.tree-row[data-path$="second.md"]'); A=$(echo "$P" | python3 -c "import json,sys;print(json.load(sys.stdin)[0])")
[[ $(front <pid>) == true ]] && osascript -l JavaScript $M click "$A" cmd
```

All of these were measured to work:

- **Real clicks** (`isTrusted`, `pointerdown` … `click`): a copy button fills
  the clipboard, which needs a user gesture; a Cmd+click on a
  `.tree-row[data-path]` opens a new tab.
- **Drags:** dragging one `.tab` centre past the other reorders `tabs`. The
  strip only shows with two tabs or more.
- **Double clicks:** `#image-view` flips `picture.fit`; `#diagram-view` (after
  `wd.mjs click "div.mermaid-diagram"`) takes `zoomed.scale` 1 → 2.5. Close
  the diagram with `eval "els.diagramDialog.close()"`: while it's open the page
  behind is inert and `onKeydown` ignores keys.
- **Real keys:** `osascript -e 'tell application "System Events" to key code
  53'` (Escape) closes the settings dialog; `keystroke "w" using {command
  down, shift down}` for chords. They don't scroll the page in a feature
  build; see *Gotchas*.

`macpt.sh` maps from `wd.mjs raw GET /window/rect` (the outer frame, physical
px) and `innerHeight`: the content starts a title bar (32 pt here) below the
frame's top. `invoke("window_origin")` gives the frame's top too, not the
content's, so don't map from it. It assumes page zoom 1. A point off the
window lands on whatever is there.

## 4. Native dialogs

`#open-btn`, `#sidebar-name`, `#empty-open-btn`, `#empty-folder-btn` and
Cmd+O open an `NSOpenPanel` as a sheet on the app's window. `macdialog.sh`
brings the app to the front, opens *Go to Folder* (Shift+Cmd+G), types the
path and presses Return twice.

```bash
node $W click "#open-btn"
.claude/skills/drive-app/scripts/macdialog.sh <pid> "$PWD/README.md"
```

- Both the file and the folder picker close after one call (measured).
- `keystroke` types through the keyboard layout, so only ASCII paths were
  measured; a character the layout lacks may be dropped.
- With several windows the sheet is on the window it was opened from; the
  script finds and raises that one. Only the case with that window already in
  front was measured.
- It exits 1, with a message, when no dialog is open or the app won't come to
  the front.
- For the folder picker with a tab open: `eval "state.folder"` must be `null`
  (`showFolder()` is a toggle), `eval "showFolder()"` shows the file's folder
  without a picker, then `click "#sidebar-name"` opens it. Pick a folder other
  than the one on show, so `state.folder` visibly changes.
- `--cancel` presses Escape. `--dump` lists the process's windows and which
  one has a sheet. The panel runs out of process: System Events sees the sheet
  but none of its buttons.
- The last line says whether the dialog closed. Confirm with `eval`
  (`tabs`, `state.folder`, `pickerOpen` back to `false`).

## 5. Clean up

1. `node $W stop` ends only the session; the app keeps running.
2. `kill <pid>`, then check `pgrep -lf '^[^ ]*target/debug/t4-markdown-viewer'`
   is empty and 4445 is free. BSD `pgrep -a` means "include ancestors", so
   `linux.md`'s `pgrep -af` matches the calling shell; use `-lf`.
3. `kill` leaves `/tmp/com_montevirgen_t4_markdown_viewer_audit_si.sock`; the
   next launch removes it.
4. A plain `cargo build`, to drop the rename and the plugin.
5. Nothing else to restore: the owner's settings were never touched.

## Gotchas

- **No keyboard scrolling in a feature build** (measured). Space, Down and
  PageDown sent through System Events reach the page — `keydown` fires, not
  `defaultPrevented` — but nothing scrolls. A plain build (§1's build
  without `--features webdriver`, started with §2's launch command) scrolls
  with the same keys, and with the owner's own. The cause is unknown; the
  plugin replacing the webview's UI delegate is a guess. So check anything
  that scrolls from the keyboard on a plain build. It has no WebDriver: the
  owner watches, and the page's scroll is read from its scroll bar through
  Accessibility:

  ```bash
  osascript -e 'tell application "System Events" to tell (first process whose unix id is <pid>) to get value of scroll bar 1 of scroll area 1 of group 1 of group 1 of window 1'
  ```

  The value is the fraction of the scroll range, 0 at the top and 1 at the
  bottom. Measured with one window, one tab and no sidebar, the app just made
  frontmost. `window 1` is the front window, a page too short to scroll
  probably has no scroll bar, and a sidebar or the tab strip may change the
  nesting (all reasoned). If the path fails, walk it down by role:
  `get {role, name} of every UI element of window 1`, then of each `group`,
  to the `AXScrollArea` holding the `AXWebArea` and the `AXScrollBar`.
- **Several windows.** Open one with `eval "invoke('open_window', {path:
  '<abs file>'})"`; synthetic Shift+click can't. The handles are the window
  labels (`main`, `w1`, …), and **their order changes from one call to the
  next**, so `window <n>` can land on a different window each time. Switch by
  label: `node $W raw POST /window '{"handle":"w1"}'`. Close one with `eval
  "(setTimeout(() => appWindow.close(), 100), 1)"` so the `eval` answers first.
- **Shift+Cmd+W closes a tab, not the window** (measured). The webview sees a
  real key before the menu, and `onKeydown` handles `w` with any Shift, so the
  menu's Close Window never fires. A real Cmd+Q does reach the menu's Quit
  (measured 2026-09-28 and 2026-09-29). Cmd+H, Cmd+M and the Edit menu's
  Cmd+A/C/V/X/Z are menu items too; whether the page sees them first is
  unmeasured.
- **Window size.** `raw POST /window/rect '{"width":…,"height":…}'` sets the
  outer size in physical px (on Retina, `width: 1200` is 600 pt), and it does
  **not** respect the minimum size: 200×150 left 100×43 pt of content. Test a
  minimum by hand, not through it.
- **Clipboard.** It's the owner's own. `pbpaste > backup` before and `pbcopy <
  backup` after — but that saves text only: an image or a file on the
  clipboard is lost. Ask first when the clipboard may hold one.
- **`eval` results** are converted to JSON natively; return plain values, not
  DOM nodes or functions.
- **A feature build loses** wry's file `<input>`, media permission and
  `window.open` handling (the plugin replaces the webview's UI delegate). The
  app uses none of them. Per wry #1593, an iframe's navigation on macOS
  reaches wry's new-window handler, and that handler may be part of what the
  plugin replaces (not checked). So a check of how the diagram frame loads, or
  that nothing opens a window, runs on a plain build by the owner's eye. The
  2026-09-28 frame check ran on a plain build for that reason, and passed;
  whether a feature build would differ was not measured.
- **Exposure.** While a feature build runs, anything local can drive it over
  `127.0.0.1:4445`, unauthenticated, as with the Windows CDP port.
