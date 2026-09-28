# Open items — t4-markdown-viewer

## Context

Collected on 2026-09-11, after v1.5.9. Sources: a review of history since v1.5.0, CI and
GitHub state, and a smoke run of the debug build using the `drive-app` skill. Swept after
v1.6.5 and the full review of 2026-09-20, again on 2026-09-25 after the full review of
2026-09-25 and v1.6.8, and again on 2026-09-26 after that review's follow-up round closed
#37, #43, #44, #46, #47, #48 and the sidebar-after-restore item; the release-pipeline item
closed the same day. Nothing here blocks a release.

Tick an item as it closes, then move it with its notes to `closed-items.md`, under the same
section, so this file lists only what is still to do. Finished plans and reviews live in
`archive/plans/` and `archive/reviews/`.

## Needs a decision

- [ ] **The `ubuntu-22.04` runner — one choice for all three t4 repos** (added 2026-09-16).
  `checks.yml` (matrix) and `release.yml` (the bundle leg) pin it on purpose: the `.deb` links
  against the builder's glibc, so building on the oldest supported runner reaches the most
  distros. GitHub deprecates the image on 2026-09-17, which is a label warning, not a break; the
  first hard failure is the brownout on 2027-03-23 (then -03-30, -04-06, -04-13, each
  14:00–00:00 UTC), and it's unsupported on 2027-04-17 (`actions/runner-images#14254`). Options,
  unchanged from `archive/plans/ci-alignment.md`: bump to `ubuntu-24.04` (raises the glibc floor
  and quietly drops older distros), keep 22.04 via `container: ubuntu:22.04`, or `cargo-zigbuild`.
  If the Linux leg moves into a container, check rustfmt is in the image — `Format` runs on that
  leg only. Listed here because both `ci-alignment.md` and `ci-alignment-round-2.md` deferred it
  and are now archived, so it was invisible. t4-git-ui carries the same item in its
  `docs/plans/open-items.md` §K.
  *Snoozed by the owner 2026-09-24 until 2026-12-23, three months before the first brownout:
  not to be raised or offered before then.*

## Needs a Mac or a Linux install

- [ ] **Mac keeps folder access across an update.** 1.5.8 was the first signed build and 1.5.9
  the first signed-to-signed update. CI proves both carry the cert (SHA-1 `53effb03…`). Only a
  Mac updating 1.5.8 → 1.5.9 without a new Documents/Desktop/Downloads prompt proves 929fe3a
  did its job. Any later signed-to-signed step is the same proof: every release since, through
  1.6.8, carries the same certificate.
- [ ] **`.json` / `.jsonc` file registration** (added 2026-09-12, see
  `archive/plans/json-viewer.md`). Only an installed build proves it: on Windows, Explorer's
  *Open with* on a `.json` lists the viewer and the Type column reads "JSON Document" (its own
  ProgID, `T4MarkdownViewer.Json`); on macOS, Finder's *Open With* offers it; on a deb/rpm
  install, `xdg-mime query filetype x.jsonc` gives `application/json` and *Open With* lists the
  app.
  *Windows proven 2026-09-13 on the installed 1.6.0: `HKCU\Software\Classes` has
  `T4MarkdownViewer.Json` ("JSON Document", icon, open command) and both `.json` and `.jsonc`
  list it under `OpenWithProgids`; `.md` still maps to `T4MarkdownViewer.Document`. macOS and
  deb/rpm still unchecked.*
  *deb proven 2026-09-26 on dry run 36186470162's .deb, in WSL Ubuntu 24.04:
  `xdg-mime query filetype x.jsonc` gives `application/json`, `gio mime application/json` and
  `text/markdown` list the app (and default to it), and `desktop-file-validate` passes. rpm
  proven the same way the same day on Fedora 44 (`closed-items.md`). macOS alone keeps this
  open.*
- [ ] **Cmd+Q loses the last 500 ms of changes** (added 2026-09-25, split from the session
  restore limit; the kill and shutdown half is kept in `closed-items.md`). A report waits
  500 ms for things to settle, and since 21384f3 closing a window waits for it — but the
  predefined Quit (`Item::quit` in `macos_menu`) ends in `applicationWillTerminate`, which `tao`
  turns straight into `RunEvent::Exit` with nothing to hold it. Fix: a Quit item of our own on
  Cmd+Q that closes every window, so each waits for its report and the app exits with the
  last; the session code already reads a quit as windows closing one after another. Dock →
  Quit and logout stay on the terminate path. Reopen on a Mac, and prove it there: scroll, then
  Cmd+Q within 500 ms, and the next launch comes back at the scroll, every window in order.
- [ ] **Mermaid diagrams on macOS and Linux** (added 2026-09-26, `archive/plans/mermaid-diagrams.md`).
  `drive-app` proves them on Windows (WebView2) only. On a Mac (WKWebView, macOS 13 floor)
  and a deb/rpm install (WebKitGTK), open `examples/kitchen-sink.md`: the diagram draws,
  clicking it opens the overlay, `Cmd`/`Ctrl`+wheel zooms about the cursor, dragging pans,
  Escape comes back at the same scroll, and the light/dark toggle redraws it, as does an OS
  light/dark switch under a theme that follows the OS. No bundled theme does, so save one in
  the app's themes folder (`src-tauri/themes/README.md` says where) holding
  `:root { color-scheme: light dark } body { background: transparent }`, pick it, and switch
  the OS setting; then again with the body rule replaced by `body { background: #fff }` plus
  `@media (prefers-color-scheme: dark) { body { background: #111 } }`. The page itself must
  change side first — if it doesn't, the webview never saw the switch, which is not this
  check failing. On GNOME, WebKitGTK may follow the GTK theme rather than the Dark Style
  switch (`gsettings set org.gnome.desktop.interface gtk-theme Adwaita-dark`; unverified).
  Headless Xvfb has no desktop setting to switch, so on Linux use a desktop session.
  *Linux no longer needs an install: since cd315c4, `drive-app` drives a debug build on
  WebKitGTK (`linux.md`); `wd.mjs` has no wheel command, so Ctrl+wheel would go through
  xdotool under Xvfb (`keydown ctrl click 4 keyup ctrl`, untried). macOS still needs a Mac.*
  *Diagrams draw in a sandboxed frame since Task 4 of `archive/plans/mermaid-pipeline.md`
  (2026-09-27), which the check above must also cover: on macOS 13 (WKWebView) and on Linux
  (WebKitGTK), the frame must still load and the diagram still draw, click and zoom as before.
  Windows is proven (`drive-app`, the debug build). Linux is too, since 2026-09-28 (WSL Ubuntu
  24.04, WebKitGTK 2.52.6, the debug build under Xvfb): Task 4 Step 5 (a) drew 35 of 35, (b)
  and (d) passed, and a gantt link, a sequence menu and the keyboard basics worked. Its first run
  drew nothing: wry hands WebKitGTK's subframe navigations to the app's `stay` guard, which
  refused the frame's `about:srcdoc`; `stay` now lets exactly that through (WebView2 never shows
  subframes to it). macOS still needs a Mac. There, per wry #1593, an iframe's navigation
  reaches the new-window handler rather than the navigation handler, so `stay` may never see the
  frame: check the frame loads and the diagrams draw, that nothing opens a window as it does,
  and that a reload (the context menu's Reload) comes back with every tab.*
  *The keyboard work of `archive/plans/viewer-follow-ups.md` (2026-09-28) was measured on
  WebView2 only. On each: Tab to a diagram and to a picture shows the ring; Enter and Space
  open them; a click, Escape, then Space scrolls the page; Escape from a keyboard-opened
  overlay returns focus to the diagram.*

## First runs after the 2026-09-26 changes

Each condition proves itself on an event that has not happened yet. Tick a sub-item when its
run passes, with the run id; when all are ticked, move the item to `closed-items.md`.

- [ ] **The changes of 2026-09-26 have each run once for real.**
  - [x] *SHA pinning required* (step 6): the next push to `main` — CI passes on all three legs.
    *CI run 36225138698 on 1e4d190, 2026-09-26: success.*
  - [ ] *Dependabot under SHA pinning*: its next weekly run (about 2026-10-02) — the
    "Dependabot Updates" job passes, and any PR it opens passes `checks.yml`.
  - [x] *The approval gate on `signing`*: the 1.6.9 tag — the run waits at the build legs,
    and after approval signs and publishes.
    *Release run 36225335885, 2026-09-26: all three legs `waiting` until approved; then every
    job passed, signed (`F06C…8151`, `53effb03…`, updater `.sig`s) and published v1.6.9.*
  - [x] *The 1.6.8 → 1.6.9 in-app update* (04a3fdc's timeouts and failure message ride in
    1.6.9): an installed 1.6.8 is offered 1.6.9 and installs it.
    *The owner's installed copy updated 2026-09-26: the exe reports 1.6.9 and is signed
    `F06C…8151` (Valid). The timeout and failure message stay unexercised: this update did not
    stall.*
  - [x] *The AppImage and 009d8c8's re-exec*: the next dry run or release — in WSL, a second
    instance of the AppImage on `$'caf\xe9.md'` exits 0 with the first window untouched.
    *The released 1.6.9 AppImage, 2026-09-26, mounted through FUSE in WSL Ubuntu: exit 0, no
    panic, the first window still on `first.md`.*

## Deferred

- [ ] **A diagram's links and actor menus lack full keyboard access** (added 2026-09-26,
  review of the Mermaid work; narrowed 2026-09-28, `archive/plans/viewer-follow-ups.md`). A
  diagram or picture is now focusable and opens with Enter or Space (Task 4). What's left:
  keyboard focus on a link inside a diagram is lost when a light/dark change redraws it, since
  `redrawDiagrams` replaces the SVG — fix: note the focused link's index among the figure's
  links before the swap, focus the same one after. And a sequence diagram's actor menus are
  mouse-only, with no keyboard way to open or step through them, and a redraw closes an open
  one. A keyboard-focused diagram that fails to draw in the new look is replaced by its code
  block, so focus is lost. Reopen with the next accessibility pass, or when someone asks.
- [ ] **A diagram render stuck in a loop still freezes its window** (added 2026-09-26, third
  review; widened to every platform 2026-09-27, `archive/plans/mermaid-follow-ups.md`, Task
  3). A frame that stops answering — navigating itself away, or simply gone quiet — is now
  caught: a 30 s per-render timeout removes it and rebuilds a fresh one for
  what draws next, on every platform. But a render that instead spins forever inside the frame
  is not caught the same way where the frame shares the page's own process: measured on
  Windows, WebView2 gives the sandboxed frame no `iframe` target of its own
  (`Target.getTargets` shows none), so a spinning render still freezes the whole window;
  WebKitGTK and WKWebView are not measured but likely share the same architecture. No known
  cause makes mermaid spin rather than error or hang quietly — `maxTextSize`/`maxEdges` bound
  the work, and this is not observed in practice. Reopen when a real diagram is found to freeze
  a window on any platform.
- [ ] **F5 doesn't retry a timed-out diagram** (added 2026-09-28, final review of
  `archive/plans/mermaid-follow-ups.md`). "The diagram took too long to draw." is cached, and
  survives F5, reopening and an unchanged save; only a restart, a theme or font change, or LRU
  eviction clears it. Measured 2026-09-28: minimised WebView2 doesn't throttle (rAF kept
  running, the 30 s timer fired at 30,002 ms, diagrams drew while minimised), so minimising
  alone won't cause one. On Windows the frame shares the page's process, so a render that
  finishes past 30 s may race the timer and cache the message anyway. Fix: have `refresh()`
  evict timeout entries for the active document. Reopen when someone hits a timed-out diagram
  they expected F5 to retry.
- [ ] **A document with a diagram has its HTML parsed twice** (added 2026-09-26, third
  review). `warmDiagrams` parses the whole rendered HTML into a `<template>` to find the
  sources, and `renderDocument` parses it again for the page. Documents without a Mermaid
  block skip it. Negligible on ordinary files; on a multi-MB one, plausibly tens of ms per open
  and per save (unmeasured). Fix: Rust returns the sources beside `html` on `Document`.
  Reopen when a large diagram document feels slow to update on save.
- [ ] **`click` tooltips never show** (added 2026-09-27, `archive/plans/mermaid-follow-ups.md`
  review). A flowchart/class/state `click A "tip"` third argument is a tooltip mermaid attaches
  in `bindFunctions`, called on the SVG mermaid draws into the page — and, since Task 4 of
  `archive/plans/mermaid-pipeline.md`, a diagram draws inside its own frame and reaches the page
  only as an SVG string, so nothing ever calls it. Reopen when someone uses one.
- [ ] **A renderer that fails to load is tried again for every document** (added 2026-09-28,
  the Linux run). A load failure is not cached, so while the frame cannot load, each document
  with a diagram waits out `loadRenderer`'s 15 s before showing its blocks as code. Fix: keep
  the failure for the window, cleared when a theme or font change redraws. Reopen when the
  frame is found failing to load anywhere in real use.
- [ ] **A subframe navigating to the app's own page runs `stay`'s reload branch** (added
  2026-09-28, the Linux run). On WebKitGTK wry hands subframe navigations to `stay` with no
  way to tell them from the page's own (wry #1593; on macOS they reach the new-window handler
  instead, per the same issue), so a frame navigating itself to `tauri://localhost` queues a
  restore as a reload would. The only subframe is the sandboxed diagram frame, so this needs
  script running in it first — a mermaid bug past its sandbox. Worst case, plausibly, a reload
  loop. Fix: wry passing only main-frame navigations to the handler, as it does on WebView2, or
  telling it which frame (wry #1593, open). Reopen when wry ships that, or on any mermaid escape
  of its sandbox. A comment is drafted, not posted: `notes/wry-1593-comment.md`.
- [ ] **A macOS tab tear-off probably lands about 32 pt high** (added 2026-09-28, the drive-app
  macOS run, `archive/plans/drive-app-macos.md`). On macOS `invoke("window_origin")` was measured
  to return the top of the window frame, not of the content: 32 pt higher, the title bar. The
  tear-off maps the cursor with it (`screenPoint` in `app.js`, then `drop_tab`'s
  `Placement::Cursor`), so the new window should land that much high. That is reasoned from the
  measurement; no tear-off was watched. Cause unknown: tao 0.35.3's `inner_position` reads as
  the content rect. Reopen when a macOS tear-off is seen landing high, or when `window_origin`
  is touched.
- [ ] **On macOS Shift+Cmd+W closes a tab, not the window** (added 2026-09-28, the drive-app
  macOS run, measured). README gives `Shift+Cmd+W` as the shortcut for Close Window, and
  `macos_menu` binds it so. But the webview sees a real key before the menu, and `onKeydown`
  handles `w` with any Shift as close-tab with `preventDefault`, so the menu item never fires: a
  window with two tabs was left with one. Clicking **Window › Close Window** does close the
  window (measured), and README now says to use it; drop that sentence from README with the fix.
  Likely fix: let `onKeydown` skip Shift+Cmd+W on macOS so the menu gets it, checked with a real
  keystroke through `drive-app`'s `macos.md`. Reopen with the next macOS keyboard or menu work,
  or on a user report.

## Accepted limits

Not bugs to fix; here so nobody rediscovers them. The three the 2026-09-12 review parked were
all lifted or retired on 2026-09-25 (`closed-items.md`). A limit the owner keeps with a reopen
trigger stays here until it reopens or lifts; one kept with no trigger moves to
`closed-items.md`, under *Accepted limits*. None is waiting for a ruling: each limit is ruled on
as it is found.

- [ ] **drive-app on macOS: a picker on a window not in front is untested** (added 2026-09-28,
  `archive/plans/drive-app-macos.md`). `macdialog.sh` finds the window holding the sheet and
  raises it before typing; only the case with that window already in front was measured. If
  the raise fails, the typed path could land in another window's page.
  *Accepted 2026-09-28. Reopen on the first walk that opens a picker from a window not in
  front.*
- [ ] **drive-app on macOS: no scripted check of the minimum window size** (added 2026-09-28,
  `archive/plans/drive-app-macos.md`). `wd.mjs raw POST /window/rect` ignores the minimum
  (measured: 200×150 physical px left 100×43 pt of content). Likely way in: a real corner drag
  with `macmouse.js drag`, as `drag-corner.ps1` does on Windows — untested.
  *Accepted 2026-09-28. Reopen when a change touches the minimum size or window sizing on
  macOS.*
- [ ] **drive-app on macOS: other menu shortcuts vs the page are unmeasured** (added
  2026-09-28, `archive/plans/drive-app-macos.md`). A real Shift+Cmd+W was seen reaching the page
  before the menu. Whether Cmd+Q/H/M and the Edit menu's Cmd+A/C/V/X/Z do too is unknown; the
  page doesn't handle them today, so the menu probably still gets them.
  *Accepted 2026-09-28. Reopen with the deferred Shift+Cmd+W fix, which will test keys against
  the menu anyway.*
