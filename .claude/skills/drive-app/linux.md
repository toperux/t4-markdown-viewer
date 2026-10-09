# Driving the app on Linux

WebKitGTK has no CDP, so on Linux the page is driven over WebDriver:
`tauri-driver` launches the debug build through `WebKitWebDriver`, and
`scripts/wd.mjs` talks to it. The app runs on a headless Xvfb display by
default, where `xdotool` reaches what WebDriver can't: the native GTK dialogs,
real OS keystrokes, window size. Section 3 of `SKILL.md` (page globals, the
visibility helper, forward-slash paths) applies here too, except that `eval`
takes one expression — see *3. Drive*.

Proven on Ubuntu 26.04 (GNOME, Wayland host) against 1.6.9.

## Prerequisites

The build dependencies in the README, then:

```bash
sudo apt install webkit2gtk-driver xvfb xdotool imagemagick
cargo install tauri-driver --locked
```

`xclip` too, only if a test needs the clipboard (see *Gotchas*). `wd.mjs` needs
Node 20 or later (`toReversed`, global `fetch`).

## Scripts

- `wd.mjs` — `node wd.mjs start <app> [args…]` | `eval "<expr>"` |
  `click "<css selector>" [Shift|Control|Alt]` | `key <Key>[+<Key>…]`
  (`key ArrowRight`, `key Control+o`) | `drag "x,y x,y …"` (viewport CSS px; one
  point is a click there) | `shot <out.png>` | `window [n]` (list handles, or
  switch to the nth) | `raw <METHOD> <path> [json]` (any other endpoint,
  relative to the session) | `stop`. The session id lives in
  `$TMPDIR/t4-wd-session`, so separate calls share one session. `WD_PORT` if
  tauri-driver isn't on 4444. A WebDriver error prints one line and exits 1.
- `xdialog.sh` — `<pid> <path>` | `<pid> --cancel` | `<pid> --dump`: answers
  the GTK Open / Select Folder dialog on display `:99`. `XDISPLAY` changes
  that, not `DISPLAY`, which the desktop session always sets.

## 1. Before launching

- **Rename the debug build**, as on Windows: single-instance keys on the
  identifier (over D-Bus here), and `/usr/bin/t4-markdown-viewer` may be
  running.

  ```bash
  TAURI_CONFIG='{"identifier":"com.montevirgen.t4-markdown-viewer.audit"}' \
    cargo build --manifest-path src-tauri/Cargo.toml
  ```

  Rebuild after every `src/` edit; frontend assets are embedded at build time.
  Themes are read from `target/debug/themes/` first, so after editing
  `src-tauri/themes/*.css` rebuild or copy them across.
- **Give it its own settings.** `config.json` and `session.json` live in
  `$XDG_CONFIG_HOME/t4-markdown-viewer`. Point the XDG dirs into the scratchpad
  and the user's `~/.config/t4-markdown-viewer` is never read or written, so
  nothing needs backing up or restoring:

  ```bash
  S=<scratchpad>/app; mkdir -p $S/config/t4-markdown-viewer $S/data $S/cache
  echo '{"reopen":"off"}' > $S/config/t4-markdown-viewer/config.json
  ```

  A consequence: the user's own themes (in their config folder) aren't there.
  Copy one into `$S/config/t4-markdown-viewer/themes/` if it's under test.
  Nor are the user's fonts: WebKit looks for them in `$XDG_DATA_HOME/fonts`, so
  a font in `~/.local/share/fonts` is invisible to the app until linked in
  (`ln -s ~/.local/share/fonts $S/data/fonts`). A check of whether a font is
  installed covers that link too (`fc-list` alone misses it).
- **Ports and display.** `ss -ltn | grep -E ':444[45]'` should be empty
  (tauri-driver on 4444, WebKitWebDriver on 4445; `--port` / `--native-port`
  move them). `ls /tmp/.X11-unix/` shows the displays taken; use `:99` unless
  `X99` is there.

## 2. Launch

Two helpers, then the session. Start the helpers with a plain `&` inside a
Bash call, not as the tool's background jobs: they outlive the call, and `&`
gives Xvfb's process ID. Env and shell variables do not carry between Bash
calls, so each command carries what it needs — set `S` again in every call
that uses it:

```bash
S=<scratchpad>/app
Xvfb :99 -screen 0 1600x1000x24 >/dev/null 2>&1 &
echo $! > $S/pids                                  # Xvfb's ID
DISPLAY=:99 GDK_BACKEND=x11 TAURI_WEBVIEW_AUTOMATION=true \
  XDG_CONFIG_HOME=$S/config XDG_DATA_HOME=$S/data XDG_CACHE_HOME=$S/cache \
  tauri-driver > $S/tauri-driver.log 2>&1 &
```

```bash
node .claude/skills/drive-app/scripts/wd.mjs start \
  "$PWD/src-tauri/target/debug/t4-markdown-viewer" "$PWD/examples/kitchen-sink.md"
```

**Record the drivers' IDs** once `start` returns, so clean-up kills only what
this run started. Take them from the ports, not from `$!`: after `setsid
tauri-driver &`, `$!` named a process already gone (without `setsid`,
untested), and WebKitWebDriver is started by tauri-driver, not by you. §1
checked both ports were free, so their owners now are this run's:

```bash
S=<scratchpad>/app
ss -ltnp | grep -E ':444[45] ' | grep -o 'pid=[0-9]*' | cut -d= -f2 >> $S/pids
```

Paths to the app and to files must be absolute: the app's working directory is
tauri-driver's, not yours. `start` can return before the app has booted (a first
`eval` once gave no tabs and a 0×0 window), so poll `eval "innerWidth"` until
it is non-zero, and when a file was named, `eval "tabs.length"` too — started
without one, the app stays on the empty screen with no tabs. Get the PID
(for `xdialog.sh`) with `pgrep -f '^[^ ]*target/debug/t4-markdown-viewer'` —
the anchor keeps it from matching the shell running the `pgrep`. Take a first
`shot` and read it. It renders without a GPU, so a blank frame means the page
never loaded.

## 3. Drive

As in `SKILL.md` §3, with `wd.mjs` in place of `cdp.mjs`:

- `click` is a real click (a user gesture). WebDriver refuses it when another
  element covers the target; `drag "x,y"` clicks at the point instead.
  `click "<sel>" Shift` opens a sidebar file in a new window, `Control` in a new
  tab.
- `key` takes no virtual-key code, and `+` joins a chord.
- `eval` takes one expression, not statements: it is placed inside `(…)`,
  since the page's CSP rules out `eval()`. Wrap statements in a function, e.g.
  the visibility helper as `eval "(() => { const v = …; return v('#sidebar') })()"`.
  It awaits a promise, and a thrown error comes back as `{"thrown": "…"}`.
- `drag` over `.tab` centres reorders tabs.

OS level, under Xvfb. There is no window manager: `windowfocus` works but
doesn't raise, `windowactivate` does nothing, and every window opens at 0,0 on
top of the last. Keys go through plain `xdotool key` after `windowfocus`
(`--window` sends synthetic events, which GTK ignores). Window titles are
`<file> — Markdown Viewer`, and `--all` matters: xdotool ORs `--pid` and
`--name` without it.

```bash
export DISPLAY=:99
W=$(xdotool search --all --onlyvisible --pid <pid> --name 'Markdown Viewer$' | head -1)
xdotool windowfocus --sync $W key ctrl+Tab               # a real keystroke
xdotool mousemove 100 200 click 1                        # a real click, screen px
xdotool mousemove 500 300 keydown ctrl click 4 keyup ctrl  # Ctrl+wheel up, one notch
xdotool windowraise $W; import -window $W out.png        # the whole window
xdotool windowsize $W 800 600
```

With several windows open, `head -1` picks any of them; put the file name in
`--name` to pick one. `import` of a covered window is black, hence the
`windowraise`. `windowsize` respects the minimum size (asking for 200×150 gives
420×320), so a minimum can be tested here.

Buttons 4 and 5 are the wheel, up and down, at the pointer; `keydown` /
`keyup` round them make a chord. One notch reached the page as a `wheel` with
`deltaY` -90 and `ctrlKey` true. With the window at 0,0 (no window manager),
page coordinates are screen coordinates: a `mousemove 500 300` read back in
the page as 500,300.

## 4. Native dialogs

`#empty-open-btn`, `#empty-folder-btn`, `#sidebar-name`, the bar's Open button
and Ctrl+O open a GTK file chooser in the app's process. WebDriver can't see it;
`xdialog.sh` finds it by its title (GTK's own "Open File" / "Select Folder"),
types the path into its location box (Ctrl+L) and presses Return.

```bash
node .claude/skills/drive-app/scripts/wd.mjs click "#open-btn"
.claude/skills/drive-app/scripts/xdialog.sh <pid> "$PWD/README.md"
```

- The file dialog closes after one call.
- The **folder picker needs two**: the first only completes the typed path in
  the location box, the second (`""`) selects it.
- `--cancel` presses Escape. `--dump` lists the app's visible windows.
- A screenshot of a dialog: `import -window $(DISPLAY=:99 xdotool
  search --onlyvisible --name '^Select Folder$') out.png`.
- The last line says whether the dialog closed. Confirm the outcome with
  `eval` (`tabs`, `state.folder`, `pickerOpen` back to `false`).

## 5. Clean up

1. `wd.mjs stop` ends the session and kills the app. The session survives it:
   `session.json` is rewritten half a second after each change settles while the
   app runs, not at exit, and a relaunch after `stop` brought back the tabs, the
   active one and the scroll position. So there's no need to close windows
   first — only to leave half a second after the last change.
2. `kill $(sort -u $S/pids); rm -f $S/pids` (with `S` set; `sort -u`
   because a process listening on two addresses is listed twice). The `rm`
   matters: §2 appends, and an ID left from an earlier run may belong to
   someone else's process by now. Give them a second to exit (`sleep 1`; the
   checks passed with it, and weren't tried without), then check nothing is
   left: `ss -ltn | grep -E ':444[45] '` and `ls /tmp/.X11-unix/ | grep X99`
   print nothing, and so does
   `pgrep -af '^[^ ]*(target/debug/t4-markdown-viewer|tauri-driver|WebKitWebDriver|Xvfb :99)'`.
   That `pgrep` only looks: whatever it lists is another session's or a leak.
   Report it; don't kill it.
3. Nothing to restore: the user's settings were never touched — unless the
   OS light/dark switch was flipped; then put their `gsettings` values back
   (see *The OS light/dark switch*).

## On the live Wayland desktop

When Wayland behaviour is what's under test, run the same tauri-driver command
without Xvfb, `DISPLAY=:99` or `GDK_BACKEND`, and ask the user first: the window
appears on their desktop. Record the drivers' IDs from their ports as in §2.
With no Xvfb nothing truncates `$S/pids`, so check it isn't there first. If it
is, an earlier run never finished clean-up: don't kill its IDs, which may be
reused by now — delete the file and look for leftovers with §5's `pgrep`.

The app runs as a native Wayland client, and `wd.mjs` works as before (`eval`,
`click`, `key`, `shot`, `window`). Nothing OS-level does: xdotool on the
desktop's `DISPLAY` (XWayland) finds none of the app's windows, so
`xdialog.sh` can't see the file dialog either. `wd.mjs stop` still closes the
app with a dialog open. Driving the dialog would take AT-SPI (`python3` with
`gi` `Atspi`, installed) — not written yet.

## The OS light/dark switch

A check that the page follows the OS ran on the live desktop (above);
whether a switch reaches the app under Xvfb wasn't tried. Switching changes
the look of the user's desktop: ask first, and note their values to put
back.

```bash
gsettings get org.gnome.desktop.interface color-scheme
gsettings get org.gnome.desktop.interface gtk-theme
```

- **Link the desktop's settings into the scratch config.** With
  `XDG_CONFIG_HOME` in the scratchpad, GTK looks for the desktop's settings
  there. Before launching, link in the three folders the 2026-09-28 run
  linked (with `S` set, as in §2):
  `for d in dconf gtk-3.0 gtk-4.0; do ln -s ~/.config/$d $S/config/$d; done`.
  That GTK misses the switch without them is that run's reasoning; it
  wasn't tried without.
- **GNOME has two switches, and the page is dark if either is.**
  `color-scheme` (Settings' Dark Style: `prefer-dark` / `default`) and
  `gtk-theme` (a dark one such as Ubuntu's default `Yaru-dark`, or
  `Adwaita-dark`). Light needs both light. So set one light and flip the
  other: under `Yaru-dark`, flipping `color-scheme` alone never reaches the
  page — GNOME's doing, not a failure.
- **Check the page saw it** before judging anything that follows the OS: an
  `eval` that adds a `change` listener to
  `matchMedia('(prefers-color-scheme: dark)')` and counts, then `matches`
  after each switch. No change event means the webview never saw it.
- **Put the user's values back** with `gsettings set` when done.

Measured 2026-09-28 on Ubuntu 26.04.1, GNOME 50.1, Wayland, WebKitGTK
2.52.6, with the Yaru and Adwaita pairs.

## Gotchas

- **Several windows.** `wd.mjs window` lists the handles and `window <n>`
  switches. The order is not the window order (a new window came first);
  `eval "appWindow.label"` says which one you're in.
- **A second instance.** One session drives the app it launched. Run the binary
  directly with the same `DISPLAY`, `GDK_BACKEND`, XDG dirs and an absolute
  path; it hands its argv to the running app, which opens a tab, and exits 0.
- **Dragging a tab into another window** is Windows-only in the app itself.
- **Clipboard.** Under Xvfb it's the X server's own selection, apart from the
  desktop's: seed it with `echo x | xclip -display :99 -selection clipboard -i`,
  read it with `xclip -display :99 -selection clipboard -o`. Writes need a real
  click: `wd.mjs click` on `button.copy-section` puts the section's Markdown
  there, while `.click()` inside `eval` is refused (`NotAllowedError`, shown as
  a toast) and leaves the clipboard alone.
- **A long `eval`.** The async script timeout is 30 s by default (`script
  timeout: …` after 30 s); raise it with `raw POST /timeouts '{"script":300000}'`.
  It lasts for the session.
- **Closing the last window from the page** (`eval "appWindow.close()"`) exits
  the app and kills the WebDriver session with it: later calls say
  `invalid session id`. `stop` still cleans up.
- **Driving WSL from Git Bash on Windows.** Git Bash rewrites arguments passed
  to `wsl.exe`: MSYS path conversion mangles anything with a `/`, and a `-c`
  string splits wrongly. Write each command to a `.sh` file and run it as
  `MSYS_NO_PATHCONV=1 wsl.exe -d Ubuntu -- bash /mnt/c/…/script.sh`; never
  pass the script inline.
