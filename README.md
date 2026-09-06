# T4 Markdown Viewer

[![CI](https://github.com/toperux/t4-markdown-viewer/actions/workflows/ci.yml/badge.svg)](https://github.com/toperux/t4-markdown-viewer/actions/workflows/ci.yml)

A small, themeable Markdown viewer for Windows, macOS and Linux. Double-click a
`.md` file and read it. No editing, no bundled browser.

Built on [Tauri v2](https://v2.tauri.app) — a Rust shell around the webview the
operating system already ships (WebView2 on Windows, WKWebView on macOS,
WebKitGTK on Linux). The whole app is a single ~6 MB executable; a comparable
Electron build starts around 150 MB.

![Azure DevOps theme](docs/screenshots/azure-devops.png)

## Features

- **Open by double-click.** `.md`, `.markdown`, `.mdown`, `.mkd`, `.mdtext`
  are registered by the installer.
- **Open a folder.** The folder icon beside Back/Forward shows the current
  file's folder as a tree in a sidebar (or asks for one when nothing is open),
  and hides it again on a second press;
  clicking a file there opens it in the current tab, so Back walks through
  what you have read. `Ctrl`+click or middle-click opens it in a new tab. The
  tree follows the folder — files added, renamed or removed show up on their
  own. The box above the tree narrows it to names containing what you type,
  across every folder it has listed so far.
- **Tabs or windows.** Read several documents at once, and choose per taste
  whether an opened file lands in a new tab or its own window. Tabs drag to
  reorder, and out onto the desktop to become their own window. Dragging one
  *into* another window is Windows-only — see [Platforms](#platforms).
- **Themes are CSS files.** Fifteen bundled, and you can drop your own into a
  folder. The default is Azure DevOps Dark.
- **Live reload.** Edit in another editor; the view updates on save and keeps
  your scroll position.
- **Tick task lists.** Click a checkbox and the `[ ]` in the file flips with it.
- **Diagrams you can actually read.** A link to an SVG or an image opens it in a
  tab of its own, with zoom and pan — a wide ERD is unreadable at column width.
  Clicking a picture embedded in a document opens the same view.
- **Syntax highlighting** for fenced code blocks.
- **GFM**: tables, task lists, footnotes, strikethrough, autolinks,
  definition lists.

## Platforms

| | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Open by double-click | ✅ | ✅ | ✅ deb / rpm |
| Tabs, tear-off, live reload, themes | ✅ | ✅ | ✅ |
| Drag a tab **into another window** | ✅ | — | — |
| Updates itself | ✅ | ✅ | ✅ AppImage |
| Signed installer | — | — | n/a |

Nothing is code-signed. On Windows that means a SmartScreen prompt; on macOS it
means a quarantine flag to clear. Both are one-time, and both are described
below.

The one real feature gap is dragging a tab from one window into another. It
needs to know which window the compositor is drawing under the cursor, which on
Windows is a `WindowFromPoint` call and elsewhere is either awkward (macOS, X11)
or deliberately impossible — **Wayland hides the global pointer position by
design**. Rather than work on two platforms out of three, the app doesn't offer
it off Windows: no drop caret appears, and releasing a dragged tab tears it off
into its own window, which you can then read side by side. Reordering tabs
within a window works everywhere.

## Installing

Every release ships a `.sha256` sidecar next to each file, so you can check you
got what CI built: `sha256sum -c <file>.sha256` on Linux, `shasum -a 256 -c` on
macOS, `Get-FileHash '.\<file>' -Algorithm SHA256` on Windows.

## Updating

From 1.2.0 the app looks after this itself. On launch it asks GitHub once
whether there is a newer release; if there is, an **Update** button appears in
the toolbar. Clicking through it downloads the new version, installs it, and
restarts with your windows and tabs where they were. Nothing is downloaded
before you say so.

Turn the check off in **Settings → Updates**; **Check now** there works either
way, shows the version you are on, and offers the update right there when it
finds one.

Two things it deliberately does not do. It never installs silently — an unsigned
app that swaps itself out behind your back has earned every bit of suspicion
that follows. And it does not touch a `.deb` or `.rpm` install: those belong to
your package manager, so the app points you at the download page instead. An
AppImage updates in place like Windows and macOS.

Downloads are verified against a signing key held outside this repository —
separate from code signing, which the project still does not do (see below).
A release built without that key ships no signatures, and the app refuses it.

Anything installed before 1.2.0 predates all of this and has to be replaced by
hand once; updates run themselves from there.

### Windows

Download `T4-Markdown-Viewer_<version>_x64-setup.exe` from
[Releases](https://github.com/toperux/t4-markdown-viewer/releases) — or build it
yourself, in which case it lands in `src-tauri/target/release/bundle/nsis/`
under Tauri's own name, `T4 Markdown Viewer_<version>_x64-setup.exe`.

It is a **per-user** install — no admin prompt — landing in
`%LOCALAPPDATA%\T4 Markdown Viewer`. Uninstall from Add/Remove Programs.

**The installer is not code-signed**, so Windows SmartScreen will show
"Windows protected your PC" the first few times anyone runs it: click **More
info** → **Run anyway**. A certificate is the only thing that removes that
prompt, and it is not worth several hundred dollars a year for this.

### macOS

Download `T4-Markdown-Viewer_<version>_universal.dmg` — one image for both
Apple Silicon and Intel — and drag the app to Applications. Then clear the
quarantine flag:

```sh
xattr -dr com.apple.quarantine "/Applications/T4 Markdown Viewer.app"
```

Without that, Gatekeeper reports the app as *damaged* rather than merely
unsigned, which is its usual response to an unsigned download. Right-click →
**Open** works too, if you prefer the prompt to the command.

Finder offers the app under **Open With** straight away. To make it the default,
select a `.md` file → **Get Info** → *Open with* → pick it → **Change All…**.

### Linux

Pick the packaging you prefer:

```sh
sudo apt install ./T4-Markdown-Viewer_<version>_amd64.deb    # Debian, Ubuntu
sudo dnf install ./T4-Markdown-Viewer_<version>_x86_64.rpm   # Fedora, RHEL
chmod +x T4-Markdown-Viewer_<version>_x86_64.AppImage        # anywhere else
```

The deb and rpm register the file association and rebuild the MIME and desktop
caches, so `.md` opens on double-click and the app appears under *Open With*.
Set it as the default with:

```sh
xdg-mime default t4-markdown-viewer.desktop text/markdown
```

**An AppImage is not installed into the system MIME database**, so it will not
pick up file associations on its own — that needs
[AppImageLauncher](https://github.com/TheAssassin/AppImageLauncher) or a
hand-installed `.desktop` file. Passing a path on the command line always works:

```sh
./T4-Markdown-Viewer_<version>_x86_64.AppImage notes.md
```

### Making it the default for `.md` on Windows

**The installer cannot do this, and neither can any other installer.** Windows
10/11 protect the current default in

```
HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.md\UserChoice
```

with a per-user hash that Explorer validates. Anything that writes there without
the correct hash is ignored or reset. Setting a default is deliberately a
user gesture. Note this outranks `HKCU\Software\Classes\.md`, so if any app has
ever claimed `.md`, installing this one changes nothing on its own.

What the installer *does* do is make sure the app is actually offered:

- `Software\Classes\Applications\t4-markdown-viewer.exe` with `FriendlyAppName`,
  `DefaultIcon` and `SupportedTypes` — populates "Open with"
- `.<ext>\OpenWithProgids` → `T4MarkdownViewer.Document` for all five extensions
- `Software\T4MarkdownViewer\Capabilities` + a `RegisteredApplications` entry —
  makes it appear in Settings → Default apps

Then pick one:

**From Explorer** — right-click any `.md` → **Open with** → **Choose another
app** → **T4 Markdown Viewer** → tick **Always use this app to open .md files**.

**From Settings** — Settings → Apps → Default apps → search *T4 Markdown
Viewer* → click the `.md` tile → select it.

Repeat per extension if you want `.markdown`, `.mdown`, `.mkd`, `.mdtext` too;
Windows tracks each one separately.

## Keyboard

On macOS, read `Cmd` for every `Ctrl` below.

| Key | Action |
| --- | --- |
| `Alt+←` / `Alt+→`, or `Ctrl+[` / `Ctrl+]` | Back / forward through visited documents |
| `Ctrl+O` | Open a file, wherever Settings says |
| `Ctrl+Shift+O` | Show or hide the sidebar with the current file's folder (a picker when nothing is open) |
| `Ctrl+Shift+F` | Filter the sidebar tree (opens it first when it is closed) |
| `Ctrl+T` | Open a file in a new **tab** |
| `Ctrl+N` | Open a file in a new **window** |
| `Ctrl+W` | Close the current tab |
| `Ctrl+Shift+T` | Reopen the last closed tab |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab |
| `F5` / `Ctrl+R` | Re-read the current file, keeping your scroll position |
| `F8` | Next theme (`Shift+F8` for previous) |
| `Ctrl++` / `Ctrl+-` / `Ctrl+0` | Zoom in / out / fit — while a picture is open |

Mouse thumb buttons work for back/forward, and middle-click closes a tab.

In a picture tab, `Ctrl`+wheel zooms about the cursor, dragging pans, and
double-clicking zooms in on what you pointed at and then back out. Plain and
`Shift`+wheel scroll, as anywhere else. Zoom and position are kept per tab, so
switching away and back returns to the same place.

macOS keeps `Cmd+W` for closing a **tab**, as browsers do, and moves closing the
window to `Shift+Cmd+W`. That is the one departure from the menu Tauri would
build by default, which binds `Cmd+W` to the window and would leave no way to
close a tab from the keyboard.

Theme cycling used to be `Ctrl+T`; it moved to `F8` so the tab shortcuts could
follow the conventions every browser and editor already uses.

### History

Relative `.md` links load in place and build a back/forward stack, as in a
browser. **History is per tab**, so Back in one tab can never walk into a
document you opened in another. Each entry remembers its scroll position, so
going Back returns you to the spot you left rather than the top of the page.
Navigating after going back discards the forward branch.

In-page `#anchor` links get a history entry too, so Back returns to the line you
clicked the link from rather than skipping the whole document. Stepping between
two anchors in one file is a scroll, not a reload — no flash, no re-highlight. A
link pointing at an id that does not exist scrolls nowhere and costs no entry.

A link carrying both — `notes.md#fc-29` — does both: the file loads and the view
lands on that section rather than at the top, and Back comes back to the link.
Cross-referencing documents lean on this heavily, and they tend to spell the
name even when the target is the file already open; that case is treated as the
in-page jump it really is, not a reload of the page you are on.

## Tabs and windows

![Settings](docs/screenshots/settings.png)

The **gear** opens Settings, where a pair of radio buttons decides where a file
opens: in a new tab, or in a new window. That covers every way a document
arrives without you saying otherwise — double-clicking it in your file manager,
`Ctrl+O`, or the **Open** button. It is shared by every open window, because a
double-click has to land *somewhere* and per-window disagreement about that is
unguessable.

To go against the default just this once, use the **arrow beside Open** — it
offers *Open in new tab* and *Open in new window* directly, and changes nothing
about the saved setting. `Ctrl+T` and `Ctrl+N` are the same two overrides from
the keyboard.

Everything runs in one process no matter how many windows are open, so a second
window costs a webview rather than a whole second copy of the app.

### Dragging tabs

The tab strip appears once a window holds two documents. From there a tab can be

- dragged left and right to reorder,
- dropped onto another viewer window to move it there — that window shows a
  caret where it would land, and takes focus once you let go (**Windows only**),
- or dropped anywhere else, including this window's own text, to tear it out
  into a new window at the cursor.

Once a tab leaves the strip it vacates its slot and a chip follows the cursor in
its place. A tab carries its history and scroll positions with it.

**The drop target is whatever window you can see** under the cursor, decided by
real z-order rather than a guess. Releasing over your own window tears the tab
off even if another window happens to be buried underneath — you could not have
aimed at something invisible, so it does not count as a target.

On macOS and Linux there is no drop target: no caret appears on any window, and
every release tears the tab off into its own window. See
[Platforms](#platforms) for why.

With a single document the strip is hidden, so the **file path in the toolbar
becomes the drag handle**. Tearing off there would just recreate the window you
already have, but on Windows dropping it onto another window merges the two,
leaving this one empty.

Closing the last tab leaves the window empty rather than destroying it; a window
disappearing from under you is a worse surprise than an empty one.

## Themes

Bundled: `azure-devops`, `azure-devops-dark` (default), `azure-devops-blue`,
`azure-devops-dark-blue`, `github-light`, `github-dark`, `github-dark-blue`,
`github-light-blue`, `solarized-light`, `solarized-dark`, `dracula`,
`dracula-green`, `dracula-blue`, `sakura`, `tufte`.

The same page in four of them — a theme owns every colour, the type scale and
the code chrome, so they are not recolourings of one look:

| | |
| --- | --- |
| ![azure-devops-dark](docs/screenshots/azure-devops-dark.png) | ![dracula-blue](docs/screenshots/dracula-blue.png) |
| `azure-devops-dark` | `dracula-blue` |
| ![tufte](docs/screenshots/tufte.png) | ![azure-devops](docs/screenshots/azure-devops.png) |
| `tufte` | `azure-devops` |

To write your own, copy `themes/_template.css` into

```
%APPDATA%\t4-markdown-viewer\themes\                        Windows
~/Library/Application Support/t4-markdown-viewer/themes/    macOS
~/.config/t4-markdown-viewer/themes/                        Linux
```

It shows up in the Settings list immediately and re-applies every time you save
it. A file there shadows a bundled theme with the same name. Full contract in
[`src-tauri/themes/README.md`](src-tauri/themes/README.md).

Themes are picked from the dropdown in **Settings** (the gear), and applied the
moment you select one — the document behind the dialog is the preview, which is
why that dialog's backdrop is barely tinted. `F8` cycles without opening
Settings, and keeps working while it is open.

## Building

Requires only a Rust toolchain — there is no Node.js build step. The frontend
is plain HTML/CSS/JS served straight out of `src/`.

On Linux, the webview and its dependencies are needed first:

```sh
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

macOS needs `xcode-select --install`, and nothing else.

```sh
cargo install tauri-cli          # once
cd src-tauri
cargo test                       # renderer + path handling
cargo tauri dev                  # run
cargo tauri build                # packages in target/release/bundle
```

`cargo tauri build` produces whatever the host can make: an NSIS installer on
Windows, `.app` and `.dmg` on macOS, `.deb`/`.rpm`/`.AppImage` on Linux. The
target list in `tauri.conf.json` names all of them, and the bundler skips the
ones that do not apply. For a universal macOS binary:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo tauri build --target universal-apple-darwin
```

To open a file during development, pass it to the built binary directly:

```sh
./target/debug/t4-markdown-viewer ../examples/kitchen-sink.md   # .exe on Windows
```

`examples/kitchen-sink.md` exercises every construct the renderer supports —
use it when checking a theme.

## Design notes

**Raw HTML is stripped, not rendered.** The app opens arbitrary files off disk,
so `<script>`, `<iframe>`, inline event handlers and `style` attributes never
reach the webview. This also means benign inline HTML — `<kbd>`, `<sub>`,
`<br>` — does not render. That is a deliberate trade, not a bug.

**With one exception: explicit anchor targets.** `<a id="f12"></a>`,
`<a name="…">` and `<span id="…">` are extremely common in hand-written
Markdown — they are how you point a `[F12](#f12)` link at something that is not
a heading — and stripping them silently breaks every such link in a document.
So the renderer walks comrak's AST for those tags, and substitutes the
`<!-- raw HTML omitted -->` placeholders they leave behind with a generated
`<span id="…"></span>`. Nothing but the identifier survives: no other
attributes, no text, and ids outside `[A-Za-z0-9._:-]` are refused, so nothing
in the source can break out of the attribute. Enabling comrak's `unsafe` option
instead would have been one line, but it also switches raw URLs back on — the
`javascript:` hole this app most needs closed.

**Highlighting runs in the webview,** using a vendored highlight.js rather than
Rust's `syntect`. `syntect` bakes colors into inline `style` attributes, which
CSS themes cannot override — that would defeat the point of CSS theming. The
cost is that a very large, code-heavy document highlights on the UI thread.

Blocks with no declared language are *not* auto-detected. Detection is often
wrong and costs real time; they get the theme's plain code background instead.

**Every platform delivers a double-clicked file differently.** All three routes
converge on one function, `open_path`, so the tab-or-window setting is obeyed
identically however the file arrived:

- **Windows and Linux** hand it over as a command-line argument in a brand-new
  process. `tauri-plugin-single-instance` forwards that `argv` to the running
  instance; cold starts read it in `setup()`.
- **macOS** never uses `argv` for this. Launch Services reuses the running app
  and sends an Apple Event, surfacing as `RunEvent::Opened` — for warm opens
  too, so the single-instance hook never sees them.

A window whose webview has not yet asked for its startup payload has no event
listener either, so `open_path` stashes the path for it to collect rather than
emitting into the void. That is the normal case on a cold start, where the OS
hands over the file before the webview exists.

**Tauri's Linux `.desktop` template omits the `%F` field code**, so the desktop
environment would launch the app with no path at all and land it on the empty
state. `src-tauri/linux/main.desktop` is a copy of that template with `%F`
added, wired up through `bundle.linux.deb.desktopTemplate`; the AppImage
bundler reuses the deb's data directory, so one file covers both. The `MimeType`
key is likewise only written when `fileAssociations[].mimeType` is set
explicitly — it is never inferred from the extensions.

**Dragging is pointer capture, not HTML5 drag-and-drop.** DnD cannot cross a
webview boundary and every window here is its own webview. Instead the bar
captures the pointer, and Rust answers "which of my windows is under this screen
point" with `WindowFromPoint` — the compositor's own answer, since Tauri exposes
no z-order and window rectangles alone cannot tell which one is on top. A tab is
plain JSON — path plus history — so moving one between windows is a hand-off
through Rust rather than any kind of webview state migration.

That question has no portable answer, so off Windows `window_at` returns
`None` and the frontend is told, through `get_settings`, not to offer the
affordance at all. Wayland goes further and will not say where a window *is*
either, so `window_origin` reports an inexact origin there and the frontend
falls back to the pointer's own screen coordinates — a torn-off window lands
approximately rather than exactly, which beats the drag doing nothing.

**Screen coordinates come from the window, not the pointer event.**
`screenX * devicePixelRatio` assumes a single scale factor for the whole
desktop and lands in the wrong place across monitors set to different scaling.
Rust reports the window's own physical origin and scale at drag start, and the
frontend maps `clientX/clientY` through those.

Three things about that were only discoverable by hitting them:

- **New windows need to be in a capability.** `capabilities/default.json` lists
  the windows its permissions apply to. With only `main` there, a runtime-created
  window is denied `event.listen` and `window.show` — so it boots half-way and
  then hangs, invisible, with no error anywhere. Hence the `w*` entry.
- **`listen()` defaults to the `Any` target,** which also receives events Rust
  addressed to a *different* window via `emit_to`. Left alone, one warm-opened
  file appears in every window and one dropped tab is adopted by all of them.
  Per-window events are listened for with an explicit `AnyLabel` target.
- **`WebviewWindowBuilder::build()` deadlocks on the event loop thread.** It
  waits for the loop to construct the webview, and both callers — a synchronous
  command and the single-instance hook — already run *on* that loop. Windows are
  therefore built on a worker thread.

## Layout

```
src/                  frontend — no bundler, no npm
  index.html
  app.js              tabs, per-tab history, dragging, settings dialog
  base.css            structure only; declares no document colors
  vendor/             highlight.js
src-tauri/
  src/
    main.rs           windows, file-open routing, drag hit-testing, commands
    render.rs         comrak: Markdown -> HTML
    themes.rs         theme discovery
    watch.rs          debounced per-window file + theme watching
    config.rs         persisted settings
  capabilities/       permission scope — must cover runtime windows
  themes/             bundled theme catalog
  windows/            NSIS installer hooks
  linux/              .desktop template, MIME package, post-install script
examples/             kitchen-sink fixture
```

## License

MIT — see [`LICENSE`](LICENSE).

Every dependency is permissively licensed and compatible with that; nothing in
the tree is GPL, LGPL, AGPL or SSPL. Five transitive crates are MPL-2.0, whose
file-level copyleft expressly allows combining them into an MIT-licensed larger
work. Per-license breakdown and attribution in
[`src-tauri/THIRD-PARTY-LICENSES.md`](src-tauri/THIRD-PARTY-LICENSES.md).
