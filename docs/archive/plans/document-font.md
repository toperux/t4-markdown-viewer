# Document Font Implementation Plan

> **Status (2026-10-10):** executed in full and shipped in 1.8.0 as c197547, with Libron
> bumped to v0.31 in 7bccb0f. Paths and unticked boxes below are as they stood when it was
> written.

> **Status (2026-10-09):** draft, reworked for the owner's change of the same day: Libron is
> bundled and is the default document font. Owner rulings recorded: F1-F4, D1-D5, B1-B4 and
> R1-R5 (each the recommended option). Probes P1-P4 measured for installed fonts; P5 (the font
> inside the diagram frame) measured on Windows and Linux, the Mac left to Task 4 Step 2b; the
> bundled faces are checked in Task 1 Steps 2 and 2b. Not to be executed until the owner's go.

> **For agentic workers:** Execute per the **Execution** section below. Steps use checkbox
> (`- [ ]`) syntax for tracking.

**Goal:** documents read in Libron, a book font built into the app, in every theme. A
"Document font" choice in Settings changes it: to any font installed on the computer, or, when
cleared, to each theme's own font. Code keeps its monospace font.

**Architecture:**
- Libron v0.30's four WOFF2 files ship verbatim in `src/fonts/libron/`, declared with
  `@font-face` in `base.css`. They are embedded in the app like the rest of `src/`, and the
  CSP already allows fonts from the app itself and from `data:` (`font-src 'self' data:`,
  `tauri.conf.json:22`; the diagram frame inherits it).
  Nothing is installed on the system.
- A new config field, `doc_font`, defaults to `"Libron"` (B1), so a config written before this
  change reads as Libron too. Empty means the theme's font. One Rust function cleans it on save
  and on load (D1). A new command saves it and broadcasts it to every window, the saving one
  included, like `diagram_colours` (`main.rs:1216-1225`). Windows apply the broadcast, never
  the command's reply, so all end on the last save.
- The frontend sets it as an inline style on `#content`: `"<name>", <the theme's own list>`.
  The theme's list is read with the override off, so a font that isn't there falls back to the
  theme's font, not the browser's default. An inline style beats every theme rule short of
  `!important`, and the browser drops an invalid value whole.
- Before applying a font, the frontend waits for its faces to load (`document.fonts.load`).
- Diagrams are drawn by Mermaid in a sandboxed frame with no origin, which can't load the
  app's CSS (`app.js:407-428`). So the page reads the four WOFF2 files once and posts their
  bytes to each new frame, which adds them as `FontFace`s and loads them before it reports
  ready (R1). Without this, Mermaid would size every label for another font. Installed fonts
  need nothing: the frame sees system fonts.
- It is applied inside `applyTheme`, between the theme's swap and the diagram-look comparison,
  so diagrams (which take the document's font, `app.js:624`) redraw exactly when the look
  changes.

**Tech Stack:** Rust (config and one command), JavaScript, HTML, CSS, four WOFF2 files.
Walks with the `drive-app` skill on the Windows VM, the Linux VM and the Mac, never the owner's
PC.

**Spec:** the owner's requests of 2026-10-09: "is changing the font used easy? … I want to use
Libron as the font for the content", ruled as a Settings choice in its own plan; then "update
the font plan to bundle the font in the install and set the settings as default on with Libron
as default value".

## The font

- **Libron** ([nicoverbruggen/libron](https://github.com/nicoverbruggen/libron)), v0.30 of
  2026-10-06. A reworking of Readerly, itself from Newsreader, for reading. SIL Open Font
  License 1.1, with "Libron" a Reserved Font Name.
- Bundled: the release's `Libron_Web.zip` (B2), four WOFF2 files, 337 580 bytes in all:
  `Libron-Regular.woff2` (82 144), `-Bold` (83 280), `-Italic` (85 196), `-BoldItalic`
  (86 960). Family name "Libron".
- Licence obligations (OFL 1.1): the files ship unmodified, so the Reserved Font Name is kept
  as is; each copy carries the copyright notice and the licence. Hence
  `src/fonts/libron/OFL.txt` (the repository's `LICENSE` verbatim, which opens with the
  copyright lines) beside the files: it is embedded in the app with them, at
  `fonts/libron/OFL.txt` in the app's own origin (R2). `src-tauri/THIRD-PARTY-LICENSES.md`
  gets a section too, as for highlight.js and Mermaid; like theirs, it stays in the repository
  and isn't packaged (`bundle.resources` is `["themes/*"]`, `tauri.conf.json:53`).
- Updates (B4): pinned at v0.30, and the release skill's vendored-files check looks for a newer
  Libron before each release.

## Current behaviour (read 2026-10-09; line numbers as of 933896b)

- **Fonts in the document.**
  - Eleven themes set the document font on `.markdown-body` (e.g. `tufte.css:45`). The four
    Azure DevOps themes, the default `azure-devops-dark` among them, set none and use
    `base.css:1249-1254`. No bundled theme uses `!important` on a font, `#content`, or the
    `font:` shorthand.
  - What sets its own font inside `#content`, so keeps it:
    - `.markdown-body code` (`base.css:1395` and each theme), which covers code blocks, inline
      code and the JSON view (`<pre><code>`, `json.rs:379`; its fold and "more" buttons use
      `font: inherit`, `base.css:1465-1467`);
    - `p.mermaid-error` (`base.css:1452-1454`);
    - diagram SVGs, which carry their own scoped styles.
  - So an override on `#content` changes text, headings, tables and footnotes only, as F2 asks.
    (`kbd` and `sub` are stripped from documents, `themes/README.md:147-149`; footnote
    references render as `<sup>` and inherit the font.)
  - No `@font-face` exists in the app today; `src/` holds no fonts.
- **Diagrams.** `diagramLook` (`app.js:608`) reads `getComputedStyle(els.content).fontFamily`
  (`app.js:624`). The font is part of the look's key in both colour modes (`app.js:625`, and the
  palette at `app.js:690`, `726`), and Mermaid gets it in its config (`app.js:838`).
  - Mermaid runs in a hidden frame sandboxed to scripts alone, with no origin, built from a
    `srcdoc` that loads only `vendor/mermaid.min.js` and `diagram-frame.js`
    (`app.js:407-428`, `diagram-frame.js:1-32`). It measures label text there as it draws.
    The frame sees system fonts but not the page's `@font-face` faces, and a `url()` font in
    it would be a cross-origin fetch from a `null` origin (reasoned, not measured). Hence R1.
  - The frame says `{ ready: true }` once Mermaid has loaded (`diagram-frame.js:31`); the page
    then resolves the renderer (`app.js:476`). A frame that stops answering is replaced by a
    new one (`app.js:443-456`).
  - Diagrams never leave the app: the only clipboard write is the copy-section text
    (`app.js:3926`).
  - A stray `"` in a font name would make Mermaid's CSS and the browser disagree about where a
    string ends, and `stylesStayInside` (`app.js:527-535`, README:521-527) then refuses every
    drawing. Reasoned from those comments, not measured. Hence D1.
- **Themes.** `applyTheme` (`app.js:3571-3606`) awaits `read_theme`, then in one go reads the
  diagram look (`:3578`), swaps `els.themeStyle` (`:3579`), and compares the look again
  (`:3585`), redrawing diagrams if it changed. Its failure path applies the default theme
  (`:3600`). The `themes-changed` listener (`:4659-4662`) calls it again.
- **A shared setting, end to end** (`diagram_colours`):
  - `Config` field and default (`config.rs:52-87`; `#[serde(default)]` on the struct, so a
    missing field takes `Default`'s value), a cleaning function (`config.rs:45-50`) applied in
    `load()` (`config.rs:150`);
  - a command that cleans, saves and emits (`main.rs:1216-1225`), registered in
    `generate_handler!` (`main.rs:1796-1817`);
  - `get_settings` returns the whole config flattened (`main.rs:149`), so a new field needs
    nothing there;
  - boot reads it (`app.js:4618`, before `applyTheme` at `:4620`), each window listens
    (`app.js:4664`), and the control's handler sets and invokes (`app.js:4431-4438`);
  - no capability change: `core:event:allow-listen` is granted, and app commands need no entry.
- **Settings** (`index.html:160-223`): Theme, Opening a file, Reopening, Updates, in a
  `<form method="dialog">` whose Enter submits and closes it. The `select` is styled at
  `base.css:640-658`; there is no text-input rule. Settings opens from the gear
  (`app.js:4424`) and Ctrl+, (`app.js:4290`).
- **Vendored files**: highlight.js and Mermaid in `src/vendor/`, noted in
  `THIRD-PARTY-LICENSES.md` › Bundled at runtime, and checked for new releases by the release
  skill (`.claude/skills/release/SKILL.md:35-41`).

## Decisions (ruled by the owner 2026-10-09: the recommended option in each)

- **F1. The font is typed by name,** with a note saying whether it is found on this computer.
  No font list, no new dependency.
- **F2. It covers the document's text,** headings included. Code, the JSON view, diagram error
  lines and the app's chrome keep their fonts. Diagrams follow the document font.
- **F3. No size control.** The walk looks at how Libron sits at each theme's size. If it reads
  wrong, a size choice is a follow-up (Task 5 files the open item). Now that every reader gets
  Libron by default, the walk's look at sizes matters more.
- **F4. One font for every theme and window,** shared like the diagram colours.
- **D1. The name is cleaned** on save and on load: `" \ < > { } ; ,` and control characters
  removed, trimmed, capped at 100 characters. The field then shows the cleaned name. A typed
  list such as "Libron, Georgia" becomes one name, "Libron Georgia" (not found).
- **D2. Enter in the field saves and keeps Settings open,** so the note and the document
  behind show the result.
- **D3. The found/not-found note updates as you type;** the font is applied and saved on
  Enter, when the field loses focus, or when Settings closes (D4).
- **D4. Closing Settings saves a pending edit,** on every platform alike: one listener on the
  dialog's `close`.
- **D5. Closing the window with an unsaved edit loses it.** Closing the window fires no dialog
  `close`, and a save on the way out might not finish. Closed accepted limit (Task 5).

After the owner's change to a bundled default:

- **B1. Everyone gets Libron,** new installs and updates alike: a config without the field
  reads as `"Libron"`. Clearing the field still means the theme's font, and stays cleared. The
  release notes announce the new look.
- **B2. The WOFF2 web build** (337 KB), not the desktop TTFs (~900 KB).
- **B3. No "Get Libron" button.** The note says "Libron is built in." for Libron, and keeps
  the found/not-found check and the restart hint for any other font typed.
- **B4. Pinned at v0.30;** the release skill's vendored-files check covers Libron.

After review of the rework:

- **R1. The diagram frame gets the font's bytes** from the page and adds them as `FontFace`s
  before it reports ready. Not `url()` fonts in the frame (a likely CORS refusal), not
  `data:` URLs (~450 KB of text per frame), and diagrams still follow the document font (F2).
  P5 measured it on Windows and Linux; the Mac is checked in Task 4 Step 2b.
- **R2. `OFL.txt` embedded beside the fonts is the licence copy that ships;** the notice in
  `THIRD-PARTY-LICENSES.md` stays repository-only, as for highlight.js and Mermaid. A reasoned
  reading of the OFL, not legal advice.
- **R3. README screenshots are retaken in Libron,** the default a new user sees.
- **R4. A window that is still starting can miss a font change made in another window at that
  moment** (between `get_settings` and its listener), and shows the old font until the next
  change, as the diagram colours and open mode already can. Closed accepted limit (Task 5).
- **R5. A frame whose fonts take longer than 2 s draws in the fallback meanwhile** (ruled
  2026-10-09): those drawings are cached under Libron's look, so their labels stay
  slightly off until a theme or font change or a restart. The handover is expected to take
  milliseconds (reasoned); Task 4 Step 2 logs the real time. Closed accepted limit (Task 5).

## Global Constraints

- Libron v0.30 WOFF2 files ship byte for byte as released, with the OFL beside them. No other
  font, no new dependency, no new capability, no CSP change.
- Only Rust's `config::doc_font` decides what a saved name may contain. The frontend applies
  only names that came back from Rust (the event, `get_settings`).
- Empty means the theme's font, exactly as today.
- Same behaviour on WebView2, WKWebView and WebKitGTK.

## Review Focus

1. A name that isn't installed, misspelt, or with quotes, backslashes or `;` in it: the page
   keeps the theme's font (or uses the cleaned name), and every diagram still draws.
2. Switching theme, F8 or a theme file edited on disk, with a font set: the font stays and the
   colours change. Diagrams redraw once, in the right font.
3. Code blocks, inline code, tables and the JSON view keep their monospace font.
4. Diagrams redraw in the new font, in both diagram-colour modes. An open zoomed diagram
   closes.
5. Every window follows a change at once, including the field in another window's open
   Settings (its unsaved edit is overwritten: accepted). A new window and a relaunch start with
   it.
6. In the field: Enter saves and stays open; Tab or a click away saves; Esc and Done close
   Settings and save a pending edit (D4); closing the window doesn't (D5). Typing updates only
   the note.
7. The reading position after a font change, as after a theme change (WebKit has no scroll
   anchoring, so it may move on Linux and the Mac).
8. The note: "Libron is built in." for Libron in any letter case; it tells an installed font (a
   system font: Georgia on Windows and the Mac, DejaVu Serif on Linux) from a made-up name, on
   each engine.
9. The first launch after the update: a config without `doc_font` opens in Libron, and a
   document with diagrams draws them in Libron with labels that fit their boxes (the faces
   loaded in the page and in the diagram frame before the first drawing), with Libron not
   installed on the system. A replacement frame after a stuck render gets the faces too.
10. Libron installed on the system as well: the bundled faces still win (a family declared by
    `@font-face` hides installed fonts of that name), and nothing breaks.

## Probes (before execution)

Measured before the owner's change, with Libron installed on each system as any other font
would be. They still hold for installed fonts typed into the field. The bundled `@font-face`
faces are checked in Task 1 Steps 2 and 2b on all three engines.

- [x] **P1 — telling whether a font is installed,** on each engine, with Libron installed and
  with a made-up name: `document.fonts.check('16px "Libron"')`, and a width comparison (a
  string measured in `"Libron", <generic>` against plain `<generic>` on a canvas, then, after
  the Mac, in a hidden span).
  - **WebKitGTK 2.52.6 (Linux VM, Wayland, 2026-10-09):**
    - `document.fonts.check()` is useless: true for every name, a made-up one included.
    - The canvas width test works. With Libron, the width differed from all three generics
      (332.13 against monospace 269.72, serif 324.42, sans-serif 320.32). A made-up name and an
      uninstalled Georgia equal the fallback exactly.
    - Fontconfig's metric-compatible aliases count as found: "Times New Roman" read as present
      and rendered in Liberation Serif. Likely Arial and Courier New too (unmeasured). The text
      then really is drawn in that substitute, so "found" is fair.
    - So "found" = the width differs from at least one generic. A font identical in width to
      all three generics can't exist in practice, since they differ from each other.
    - Setup trap for walks: WebKit sees user fonts through `$XDG_DATA_HOME/fonts`. With
      drive-app's scratch `XDG_DATA_HOME`, Libron in `~/.local/share/fonts` was invisible
      until linked in. Normal users are unaffected. Goes into the drive-app skill's
      `linux.md` with the walks (Task 6).
  - **The Mac (in the app: the webdriver debug build of 353efd6, macOS 26.7.1, WebKit of
    Safari 27.0; Libron in `~/Library/Fonts`; 2026-10-09):**
    - `document.fonts.check()` is true for every name here too.
    - **The canvas can't see the per-user Libron:** it measured exactly like the missing font
      ("32px Libron" = "32px serif"). System fonts (Georgia, Times) it does see. Likely
      WebKit's fingerprinting protection (unverified).
    - **A hidden DOM span can:** Libron differed from all three generics, the made-up name from
      none. So the check measures a span, not a canvas (Task 4). Walked on all three as
      Review Focus 8.
  - **WebView2 (Windows VM, 1.7.4, Libron installed per user, 2026-10-09):** `check()` true
    for every name; the canvas and the span both see Libron and Georgia, and neither sees the
    made-up name. In an earlier run, "Times New Roman" measured equal to the generic serif
    there (it is the serif); its other comparisons weren't reported. So "found" must mean any
    one comparison differs, as the code has it.
- [x] **P2 — Libron renders** in each webview once installed: regular, italic, bold and bold
  italic, by screenshot.
  - **WebKitGTK 2.52.6 (Linux VM, 2026-10-09):** all four styles render as Libron's own faces.
    `font-synthesis: none` gave a pixel-identical screenshot, so nothing is synthesized. The
    release also has `Libron_R.zip` (a separately named variant), Kobo, CrossPoint and WOFF2
    builds. The app bundles the WOFF2 build (B2); these probes, with the TTFs installed, now
    stand for any other font a reader installs.
  - **The Mac (2026-10-09):** all four styles render as Libron's own faces (a drawn bold, a
    true cursive italic), from the per-user install.
  - **WebView2 (Windows VM, 2026-10-09), by CDP and screenshots:** CDP's platform fonts show
    Libron-Regular, -Bold, -Italic and -BoldItalic for the four styles, so nothing is
    synthesized; the screenshots show a drawn bold and a true cursive italic.
- [x] **P3 — a font installed while the app runs:** seen at once, after a reload, in a new
  window, or only after a restart? The hint says "restart" regardless (Linux and Windows
  measured); the Mac wasn't measured, and a restart is right there too.
  - **WebView2 (Windows VM, 2026-10-09):** installed while the app ran, Libron stayed unseen
    on the same page, after 6 s, and after `location.reload()`; after an app restart it was
    seen. A new window: not tried. Also: while the app runs, WebView2 holds the font files
    open, so uninstalling a font the app has used fails until it's closed. Worth one README
    line.
  - **WebKitGTK 2.52.6 (Linux VM, 2026-10-09):** fonts are read once per web process.
    - A page running when Libron was installed never saw it, not after 30 s, not after
      `location.reload()` (same web process).
    - A window opened afterwards saw it at once: each window has its own web process here.
    - After a restart, every window saw it.
    - So the hint needs "Restart the app after installing a font" on Linux at least. Not
      measured: a font folder that existed before launch, a system-wide install.
- [x] **P4 — how the font is installed:**
  - Windows: per user (the default "Install") and for all users.
    - **Measured 2026-10-09: a per-user install works** (P1, P2). All-users isn't needed, so it
      wasn't measured.
  - macOS: per user (`~/Library/Fonts`, Font Book's default) and `/Library/Fonts`.
    - **Measured 2026-10-09: a per-user install works for the page** (P2 rendered it); only the
      canvas is blind to it (P1), which the span check avoids. `/Library/Fonts` isn't needed,
      so it wasn't measured.
- [x] **P5 — a font from bytes inside the diagram frame** (R1; the Mac in Task 4 Step 2b),
  on each engine, Libron not installed: in the sandboxed `srcdoc` frame's own context,
  `new FontFace("ProbeLibron", <Libron-Regular.woff2 bytes>)` added and loaded; a span
  measured with it against `monospace`; the same from a `data:` URL (the fallback); and two
  `mermaid.render` calls of one flowchart, with and without the font, compared by `viewBox`.
  - **WebKitGTK 2.52.6 (Linux VM, Wayland, 2026-10-09): works.** Reached through
    tauri-driver's native frame switching (`about:srcdoc`, origin `null`, Mermaid present).
    - From bytes: `status` "loaded", the span used it (664.28 against monospace 539.44).
    - From a `data:` URL: the same.
    - Mermaid measured with it: `viewBox` 366.7×70 against 395.6×94 in monospace.
    - No CSP violation or error event in the frame.
  - **The Mac: not measured.** The in-app WebDriver plugin can't enter an opaque-origin frame
    (it reaches frames through `contentWindow`), and the CSP blocks injecting scripts. Left to
    Task 4 Step 2b: it draws a diagram on the Mac and compares its boxes.
  - **WebView2 (Windows VM, 1.7.4, 2026-10-09): works.** Reached through CDP's execution
    context for the `about:srcdoc` frame (origin `null`, Mermaid present).
    - From bytes and from a `data:` URL: both "loaded" and used (664.28 against 492.63).
    - Mermaid measured with it: `viewBox` 366.0 wide against 388.3 in monospace.
    - No console, CSP or exception events from the frame.

## Tasks

### Task 1: bundle Libron

**Files:** `src/fonts/libron/` (new), `src/base.css`, `src-tauri/THIRD-PARTY-LICENSES.md`.

- Download `Libron_Web.zip` from the v0.30 release
  (`gh release download v0.30 -R nicoverbruggen/libron -p Libron_Web.zip`) and check it:
  - sha256 `0d78f9367ee78ec31554947c80fb3f18e3d5924ce0866a058cbf1131ef934861` (GitHub's
    published digest, checked 2026-10-09).
  - The four files inside, by sha256:
    - `Libron-Regular.woff2` `e64c8420f5d21deb3fce8d45a6d9165827067de8c36262922ad5f56e6b0c37f4`
    - `Libron-Bold.woff2` `2c904e1af5a98879e7ea11089d7e1812e13574658849b9b1518d438f9379b75e`
    - `Libron-Italic.woff2` `4a663565fdd7ba2705ac9f2a0d112c92bf33010e701a40829ec8d93c1254ead0`
    - `Libron-BoldItalic.woff2`
      `5b635f6374388c29ef84fd712029a24e36c345fbbf7d0cfa6b350f8a216e7eee`
- Copy the four into `src/fonts/libron/` unchanged.
- `src/fonts/libron/OFL.txt`: the repository's `LICENSE` at the v0.30 tag
  (`gh api repos/nicoverbruggen/libron/contents/LICENSE?ref=v0.30`), verbatim, LF line
  endings in the repository: check the committed blob (`git show HEAD:src/fonts/libron/OFL.txt`
  piped to `tr -d -c '\r' | wc -c` gives 0), not the working copy, which autocrlf may turn to
  CRLF. It opens with the copyright lines and the Reserved Font Name.
- `base.css`, right after the header comment (before `:root`, `base.css:14`):

  ```css
  /*
   * Libron, the default document font (Settings › Document font), bundled from
   * github.com/nicoverbruggen/libron v0.30 under the SIL Open Font License,
   * src/fonts/libron/OFL.txt. Declared here, not in a theme, so every theme has it.
   */
  @font-face {
    font-family: "Libron";
    src: url("fonts/libron/Libron-Regular.woff2") format("woff2");
    font-weight: 400;
    font-style: normal;
  }
  @font-face {
    font-family: "Libron";
    src: url("fonts/libron/Libron-Bold.woff2") format("woff2");
    font-weight: 700;
    font-style: normal;
  }
  @font-face {
    font-family: "Libron";
    src: url("fonts/libron/Libron-Italic.woff2") format("woff2");
    font-weight: 400;
    font-style: italic;
  }
  @font-face {
    font-family: "Libron";
    src: url("fonts/libron/Libron-BoldItalic.woff2") format("woff2");
    font-weight: 700;
    font-style: italic;
  }
  ```

  - `font-display` is left at its default (`auto`): the faces are local, and the app waits for
    them before applying the font anyway (Task 3).
  - URLs are relative to `base.css`, which sits beside `fonts/` in `src/`.
- `THIRD-PARTY-LICENSES.md` › Bundled at runtime, after Mermaid:

  ```markdown
  ### Libron

  `src/fonts/libron/*.woff2` — v0.30 (the release's `Libron_Web.zip`), redistributed
  verbatim. SIL Open Font License 1.1, in full in `src/fonts/libron/OFL.txt`. Copyright (c)
  2026 Nico Verbruggen, with Reserved Font Name Libron; based on Readerly (c) 2026 Nico
  Verbruggen and Newsreader (c) 2020 The Newsreader Project Authors.
  <https://github.com/nicoverbruggen/libron>
  ```

- [ ] **Step 1:** the files, the OFL, the CSS and the notice.
- [ ] **Step 2: check** (the coder): the four sha256 sums match the list above. On the Windows
  VM, in the audit build, with Libron not installed on the system:
  `document.fonts.load('16px "Libron"')` resolves with one face, and the same for `bold`,
  `italic` and `italic bold`; `fetch("fonts/libron/Libron-Regular.woff2")` is ok with 82 144
  bytes (the frame's handover fetches through the app's protocol, a first for this app); the
  console shows no CSP error.
- [ ] **Step 2b: check** (the main session, through the Linux VM and Mac sessions): the same
  evals on each. This is the bundled font's probe: if any engine refuses it, stop and report.
- [ ] **Step 3: commit:** "Bundle Libron" (squashed later into the feature).

### Task 2: config and command

**Files:** `src-tauri/src/config.rs`, `src-tauri/src/main.rs`.

`config.rs`, beside `diagram_colours`:

```rust
/// The document font everyone starts on: Libron, bundled with the app (src/fonts/libron).
pub const DEFAULT_DOC_FONT: &str = "Libron";

/// The document font as a family name the page can quote safely: no character that would
/// end a CSS string or a declaration, or close a tag, survives (they would also stop every
/// diagram drawing; see `stylesStayInside` in app.js). Trimmed and capped. Empty means the
/// theme's own font.
pub fn doc_font(raw: &str) -> String {
    let kept: String = raw
        .chars()
        .filter(|c| !c.is_control() && !"\"\\<>{};,".contains(*c))
        .collect();
    kept.trim().chars().take(100).collect::<String>().trim_end().to_string()
}
```

- `Config`: `pub doc_font: String,` with a doc comment ("The font for the document's text, over
  every theme's; empty for the theme's own. See `DEFAULT_DOC_FONT` and `doc_font`."), default
  `DEFAULT_DOC_FONT.to_string()`. With `#[serde(default)]` on the struct, a config written
  before this field existed reads as Libron (B1).
- `load()`: `cfg.doc_font = doc_font(&cfg.doc_font);` after the `diagram_colours` line. An
  empty value stays empty: cleaning never brings the default back.
- Tests, beside `diagram_colours_accepts_only_the_two` (`config.rs:310`):

  ```rust
  #[test]
  fn doc_font_keeps_names_and_drops_what_could_break_out() {
      assert_eq!(doc_font("Libron"), "Libron");
      assert_eq!(doc_font("  Libre Baskerville  "), "Libre Baskerville");
      assert_eq!(doc_font("a\"b\\c;d{e}f<g>h,i"), "abcdefghi");
      assert_eq!(doc_font("x\ny\tz"), "xyz");
      assert_eq!(doc_font(""), "");
      assert_eq!(doc_font(&"é".repeat(150)).chars().count(), 100); // chars, not bytes
  }

  /// Every config written before the setting existed starts on Libron (B1); one that cleared
  /// it keeps the theme's font.
  #[test]
  fn doc_font_defaults_to_libron_and_keeps_a_cleared_one() {
      let old: Config = serde_json::from_str(r#"{"theme":"dracula"}"#).unwrap();
      assert_eq!(old.doc_font, "Libron");
      let cleared: Config = serde_json::from_str(r#"{"doc_font":""}"#).unwrap();
      assert_eq!(cleared.doc_font, "");
  }
  ```

`main.rs`, after `set_diagram_colours`, and in `generate_handler!`:

```rust
/// Broadcast, like the diagram colours: one reader, one reading font in every window. The
/// saving window takes the cleaned name from the event too, so windows saving at once all
/// end on the last save.
#[tauri::command]
fn set_doc_font(app: AppHandle, font: String) {
    let font = config::doc_font(&font);
    let mut cfg = config::load();
    cfg.doc_font = font.clone();
    config::save(&cfg);
    let _ = app.emit("doc-font-changed", font);
}
```

- [ ] **Step 1:** add the above.
- [ ] **Step 2:** `cargo fmt`, then `cargo test --manifest-path src-tauri/Cargo.toml`: both new
  tests pass, nothing else changes. Snapshot `%APPDATA%\t4-markdown-viewer` before and compare
  after (the test redirect, per the memory on reordered tests).
- [ ] **Step 3: commit:** "Document font: config and command".

### Task 3: applying it

**Files:** `src/app.js`, `src/diagram-frame.js`.

- `state` (`app.js:114` area): `docFont: "",`. It starts empty, so the boot call below always
  applies the saved font.
- A new section after "diagram colours" (`app.js:3033-3049`):

```js
/* ---------------- document font ---------------- */

/** The font that ships with the app (src/fonts/libron); see config::DEFAULT_DOC_FONT. */
const BUILT_IN_FONT = "Libron";

/**
 * Put the reader's font over the theme's (F2). Inline on #content: it beats every theme rule
 * short of !important, and the browser drops an invalid value whole. The theme's own list
 * follows, read with the override off, so a font this computer lacks falls back to the
 * theme's rather than the browser's default. `state.docFont` came from Rust, cleaned (D1).
 */
function applyDocFont() {
  els.content.style.fontFamily = "";
  if (!state.docFont) return;
  const theme = getComputedStyle(els.content).fontFamily;
  els.content.style.fontFamily = `"${state.docFont}", ${theme}`;
}

/**
 * Load a font's four faces before it is applied, so nothing (a diagram above all, which
 * measures its labels as it draws) is laid out in the fallback first. A name with no
 * @font-face, such as an installed font, resolves at once.
 */
function loadFontFaces(font) {
  const faces = ["", "bold ", "italic ", "italic bold "];
  return Promise.all(faces.map((f) => document.fonts.load(`${f}16px "${font}"`))).catch(() => {});
}

let docFontToken = 0;

/**
 * Reflect the font in Settings and on the page. Also called when another window changes it,
 * so it must not re-broadcast. Diagrams wear the document's font, so they redraw when their
 * look changes, as for the diagram colours. A later call wins over one still loading.
 */
async function showDocFont(font) {
  // Counted before the early return: a call that changes nothing still outranks an older one
  // still loading, so every window ends on the last save.
  const token = ++docFontToken;
  els.docFont.value = font;
  showDocFontNote(font);
  if (font === state.docFont) return;
  if (font) await loadFontFaces(font);
  if (token !== docFontToken) return;
  const drawn = !els.content.hidden && diagramFigures().length > 0;
  const before = drawn && diagramLook().key;
  state.docFont = font;
  applyDocFont();
  if (drawn && diagramLook().key !== before) {
    els.diagramDialog.close(); // the overlay's copy is of the old drawing
    redrawDiagrams().catch(console.error);
  }
}
```

- `applyTheme`: call `applyDocFont();` right after `els.themeStyle.textContent = css;`
  (`app.js:3579`), before the look is compared at `:3585`. That one place covers boot, F8, the
  picker, `themes-changed`, and the failure path's default theme.
- Boot, before `await applyTheme(settings.theme)` (`app.js:4620`):
  `await showDocFont(settings.doc_font ?? "");`. The faces are loaded before the first document
  renders, and the theme then applies the font. Rust has already cleaned the value.
- Listener, after `diagram-colours-changed` (`app.js:4664`):
  `await listen("doc-font-changed", (e) => showDocFont(e.payload).catch(console.error));`.
- **The diagram frame (R1).** `app.js`, beside `loadRenderer`:

  ```js
  /** The bundled faces, as bytes for the diagram frame, which can't load the app's CSS. */
  const BUILT_IN_FACES = [
    ["Libron-Regular", "400", "normal"],
    ["Libron-Bold", "700", "normal"],
    ["Libron-Italic", "400", "italic"],
    ["Libron-BoldItalic", "700", "italic"],
  ];
  let builtInFaceBytes = null;
  function builtInFaces() {
    builtInFaceBytes ??= Promise.all(
      BUILT_IN_FACES.map(async ([file, weight, style]) => {
        const r = await fetch(`fonts/libron/${file}.woff2`);
        if (!r.ok) throw new Error(`${file}: ${r.status}`);
        return { weight, style, data: await r.arrayBuffer() };
      }),
    ).catch((err) => {
      // No faces: diagrams draw in the fallback, as before this change.
      console.error("The bundled font did not load for diagrams:", err);
      return [];
    });
    return builtInFaceBytes;
  }
  ```

  In `loadRenderer`, the `ready` branch (`app.js:476`) no longer resolves at once. It marks
  the load settled (so the grace timers can't fail it), posts the faces, and resolves on the
  frame's `{ fontsReady: true }`, or after 2 s if that never comes, so a frame that can't add
  them still draws:

  ```js
  if (e.data.ready) {
    if (settled) return;
    settled = true;
    const go = () => resolve(render);
    const timer = setTimeout(go, 2000);
    fontsReady = () => (clearTimeout(timer), go());
    // Copied, not transferred: the same buffers go to every new frame, and a transferred one
    // would be detached for the next.
    builtInFaces().then((faces) => frame.contentWindow?.postMessage({ fonts: faces }, "*"));
    return;
  }
  if (e.data.fontsReady) return fontsReady?.();
  ```

  with `let fontsReady = null;` beside `settled`. Each new frame (a first load, or a
  replacement after `stop`) goes through `ready` again, so it gets the faces too. The bytes are
  fetched once per window (`connect-src` falls back to `default-src 'self'`).

  `diagram-frame.js`, a branch after line 10's guard and before line 11's destructure:

  ```js
  // The app's own fonts, as bytes: this frame has no origin and can't load its CSS. How
  // many loaded goes back, for the checks.
  if (Array.isArray(e.data.fonts)) {
    const faces = e.data.fonts.flatMap(({ weight, style, data }) => {
      if (!(data instanceof ArrayBuffer)) return [];
      const face = new FontFace("Libron", data, { weight: String(weight), style: String(style) });
      document.fonts.add(face);
      return [face.load().then(() => 1, () => 0)];
    });
    const loaded = (await Promise.all(faces)).reduce((a, b) => a + b, 0);
    parent.postMessage({ fontsReady: true, loaded }, "*");
    return;
  }
  ```

  If Task 4 Step 2b shows the Mac refuses fonts from bytes in the frame, the frame builds the
  `FontFace` from a `data:` URL instead (P5 measured both forms working on Windows and
  Linux); the rest is unchanged.
- **Tasks 3 and 4 are one change:** `showDocFont` uses Task 4's field and note, so both are
  implemented before the check.

### Task 4: Settings

**Files:** `src/index.html`, `src/base.css`, `src/app.js`.

**Markup**, a fieldset after Theme (`index.html:174`):

```html
<fieldset>
  <legend>Document font</legend>
  <p class="hint">
    The font for the document's text in every theme; code keeps its own. <b>Libron</b> is
    built in. Type another font's family name as installed on this computer, or clear the field
    for the theme's own font. Restart the app after installing a font.
  </p>
  <input id="doc-font" type="text" aria-label="Document font" placeholder="The theme's own font"
         spellcheck="false" autocomplete="off" autocorrect="off" autocapitalize="off" />
  <p id="doc-font-note" class="hint doc-font-note" role="status"></p>
</fieldset>
```

- "Restart the app after installing a font": P3 measured on Linux and Windows that a running
  window never sees a font installed after it started, a reload included. Kept whatever the
  Mac would show, since a restart is right everywhere. P4 measured a per-user install working
  on Windows and the Mac, so the hint says nothing about where to install.

**CSS** (`base.css`, after the `select` rules at `:640-658`):

- `#settings-dialog input[type="text"]`: the tree filter's text-field rules (`#tree-filter`,
  `base.css:455-477`: `appearance: none`, font, colour, background, border, radius, padding,
  and its `::placeholder` and `:focus-visible` rules), without its `display`, `width` and
  `margin`, plus the select's `min-width: 14rem` and `:hover`. Without these the field gets the
  browser's white box on dark themes and a grey placeholder on every theme.
- `#settings-dialog .doc-font-note { margin-top: 0.75rem; }`, like `.diagram-colours-hint`
  (`base.css:619-621`): a hint after a control, not a legend.

**`app.js`:**

- `els`: `docFont: document.getElementById("doc-font")`,
  `docFontNote: document.getElementById("doc-font-note")`.
- The note, in the "document font" section beside `BUILT_IN_FONT`:

  ```js
  // Font-name characters Rust would drop (config::doc_font), for the note before saving.
  // `char::is_control` covers U+0080-U+009F too.
  const FONT_NAME_DROPS = /[\u0000-\u001f\u007f-\u009f"\\<>{};,]/g;

  /**
   * Whether `name` is a font this computer has. `document.fonts.check` says yes to every name,
   * and on the Mac a canvas can't see fonts installed per user (both measured), so a hidden
   * span is measured instead: a missing font falls back to the generic, whose width it then
   * has exactly; an installed one differs from at least one of the three.
   */
  function fontInstalled(name) {
    const probe = document.createElement("span");
    probe.textContent = "mmmmmmmmmmlliWWQ@#0123456789";
    // A size in px: a bare `monospace` would otherwise drop to the browsers' 13px default.
    probe.style.cssText =
      "position:absolute;left:-10000px;top:0;visibility:hidden;white-space:nowrap;font-size:32px";
    document.body.append(probe);
    try {
      const width = (family) => {
        probe.style.fontFamily = family;
        return probe.getBoundingClientRect().width;
      };
      return ["monospace", "serif", "sans-serif"].some((g) => width(`"${name}", ${g}`) !== width(g));
    } finally {
      probe.remove();
    }
  }

  function showDocFontNote(raw) {
    const name = raw.replace(FONT_NAME_DROPS, "").trim();
    // CSS matches family names without regard to case, so "libron" is the built-in one too.
    els.docFontNote.textContent = !name
      ? "The theme's own font."
      : name.toLowerCase() === BUILT_IN_FONT.toLowerCase()
        ? `${BUILT_IN_FONT} is built in.`
        : fontInstalled(name)
          ? `${name} is on this computer.`
          : `${name} isn't on this computer, so the theme's font is used.`;
  }
  ```

  The built-in case is decided by name, not measured: a bundled face loads only when first
  used, so measuring it could read "not on this computer" before it has.
- Saving, in `main()` beside the diagram-colour radios (`app.js:4431`):

  ```js
  // Saved on Enter (D2), when the field is left, or when Settings closes (D4); the note
  // follows each keystroke (D3). The page changes when the broadcast comes back, in this
  // window as in every other.
  const saveDocFont = async () => {
    if (els.docFont.value === state.docFont) return; // already saved, or Enter then blur
    await invoke("set_doc_font", { font: els.docFont.value });
  };
  els.docFont.addEventListener("input", () => showDocFontNote(els.docFont.value));
  els.docFont.addEventListener("change", () => saveDocFont().catch(toast));
  els.settings.addEventListener("close", () => saveDocFont().catch(toast));
  els.docFont.addEventListener("keydown", (e) => {
    if (e.key !== "Enter" || e.isComposing || e.keyCode === 229) return;
    e.preventDefault(); // the form would submit and close Settings
    saveDocFont().catch(toast);
  });
  ```

- Both places that open Settings (`app.js:4290`, `4424`) call
  `showDocFontNote(els.docFont.value)` first. On Linux and Windows a running window never sees
  a font installed after it started (P3), but the Mac is unmeasured, and the check is cheap
  (owner's call).

- [ ] **Step 1 (Tasks 3 and 4):** implement both.
- [ ] **Step 2: check:** `node --check src/app.js` and `cargo test`. Build the audit app
  (drive-app skill) on the Windows VM. Setup: Libron not installed on the system (check the
  per-user Fonts folder and the HKCU Fonts values), and a config without `doc_font`. The audit
  build shares `%APPDATA%\t4-markdown-viewer` (`config.rs:111-115`), and any setting saved
  writes the whole config back, so delete the `doc_font` key from the live `config.json`
  (backed up first, per drive-app) before each first-launch check:
  - the first document opens in Libron (computed `font-family` starts with `"Libron"`); a page
    with a diagram draws it with labels inside their boxes, and the frame's `fontsReady` says
    `loaded: 4` (a page-side `message` listener records it, and the time from `ready` to
    `fontsReady`, logged for R5);
  - a replacement frame: remove it by CDP (`document.querySelector('iframe[sandbox]')
    .remove()`), then, on a page with a diagram and the diagrams following the theme's
    colours (the default), switch to a theme not yet shown this session, so the drawing isn't
    cached (`warmSources`, `app.js:798`); `render` finds the frame gone
    (`app.js:460-463`) and a new one comes up, whose `fontsReady` also says `loaded: 4`, with
    labels inside their boxes;
  - Settings shows "Libron" and "Libron is built in."; "libron" too;
  - type "Georgia": "Georgia is on this computer."; "Libr": not on this computer;
  - Enter: Settings stays open, the page changes, `config.json` has `"doc_font": "Georgia"`,
    and a second window follows;
  - `a"b;c` saves as `abc`, and `a'b(c)/*d*/` as typed; a page with a diagram still draws in
    both;
  - type a name and press Esc: Settings closes and the name is saved (D4);
  - after a theme switch with a font set, `els.content.style.fontFamily` isn't empty (the
    theme's list re-parsed);
  - a theme switch keeps the font; code blocks and a JSON file stay monospace;
  - clearing the field and pressing Tab brings the theme's font back; relaunch: still the
    theme's font (`"doc_font": ""` kept);
  - typing "Libron" again: Libron back.
- [ ] **Step 2b: check** (the main session, through the Mac session; P5 couldn't reach the
  Mac's frame): with Libron not installed, a document with a flowchart draws in Libron with
  every label inside its box, and the frame's `fontsReady` says `loaded: 4` (a page-side
  `message` listener records it). If the labels overflow, switch the frame to the `data:` URL
  form (Task 3) and check again; if that fails too, stop and report.
- [ ] **Step 3: commit:** "Document font: apply it, and choose it in Settings".

### Task 5: README, release skill and records

- README:
  - the Settings section gets "Document font": Libron is built in and the default; another
    installed font can be typed, or the field cleared for the theme's own; a newly installed
    font needs an app restart (P3); on Windows, a font the app has used can be uninstalled only
    after the app is closed (P3);
  - the diagram-colours passage (README:557-565) gains: diagrams follow the document font;
    with Mermaid's own colours, a diagram's own `fontFamily` still overrides it;
  - the licences paragraph (README:678) already points at `THIRD-PARTY-LICENSES.md`; check it
    still reads right with a font in it.
- Screenshots, retaken in Libron (R3) on the Windows VM in the walk, since the default look
  changes: `docs/screenshots/settings.png` (README:293), the opening one (README:13,
  `azure-devops.png`) and the theme gallery (README:400-402: azure-devops-dark, dracula-blue,
  tufte, azure-devops).
- `src-tauri/themes/README.md`: the document font (Libron by default) replaces a theme's
  `.markdown-body` `font-family`; only `!important` beats it, and a reader who clears the
  setting gets the theme's own.
- `.claude/skills/release/SKILL.md:35-41`: the vendored-files check names Libron too
  (`src/fonts/libron/`, its version line in `THIRD-PARTY-LICENSES.md`; a bump swaps the four
  files and `OFL.txt` together and keeps the family name).
- `docs/open-items.md` › Deferred (F3): "A size for the document font." Reopen if Libron or
  another chosen font reads too small or large at the themes' sizes.
- `docs/closed-items.md` › Accepted limits (D5): closing the window with an unsaved font edit
  in Settings loses it. "*Kept 2026-10-09, owner's call: closing the window fires no dialog
  `close`, and a save on the way out might not finish.*"
- `docs/closed-items.md` › Accepted limits (R4): a window still starting can miss a font change
  made in another window at that moment. "*Kept 2026-10-09, owner's call: as the diagram
  colours and open mode already can; it needs two windows and a save in the same moment.*"
- `docs/closed-items.md` › Accepted limits (R5): diagrams drawn while a frame's fonts take
  longer than 2 s keep fallback-sized labels until a theme or font change or a restart.
  "*Kept 2026-10-09, owner's call: the handover took <the time Task 4 Step 2 logged>.*"
- [ ] **Commit:** the READMEs, screenshots and skill ride with the feature; records are `docs:`.

### Task 6: walks on all three platforms

Via the Windows VM, the Linux VM and the Mac (never the owner's PC), first without Libron
installed on the system (each removed the probe's copy; check again: Windows per-user Fonts and
HKCU values, with the app closed since WebView2 holds the files open; Linux `fc-list`, the
scratch `XDG_DATA_HOME` link included; the Mac `~/Library/Fonts`), then (Review Focus 10) with
it:

- Review Focus 1-10 (for 9 on Linux, the frame's `fontsReady` says `loaded: 4`);
- how Libron reads in a light and a dark theme, and at Tufte's larger size (F3);
- the Linux trap from P1 goes into `.claude/skills/drive-app/linux.md`.

## Execution

Main session orchestrates. Task 1, Task 2, then Tasks 3 and 4 together, go to a `coder`
subagent (Opus), each committed on main after `cargo fmt`, `cargo test` and `node --check`.
The coder runs its checks on the Windows VM; the main session runs Task 1 Step 2b and Task 4
Step 2b through the Linux VM and Mac sessions. Task 5 is the main session's. Walks on the
Windows VM, the Linux VM and the Mac, then the change review loop (CLAUDE.md step 5).

At packaging: one feature commit, "Read in Libron, a built-in book font, or choose the
document's font in Settings" (the owner may retitle), plus `docs:` records. The release notes
should lead with the new default look (B1).
