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

- [ ] **Diagrams in the theme's own colours** (added 2026-09-26, `archive/plans/mermaid-diagrams.md`).
  Diagrams take mermaid's `default` or `dark` palette by the theme's side, and the theme's body
  font; a Dracula or Solarized document gets the same diagram colours as any other dark or light
  one. Fix: mermaid's `base` theme, with `themeVariables` read from the theme's computed styles
  (page background, text, code background, link colour), then a contrast check by eye across
  all bundled themes. Reopen when a bundled theme's diagrams clash visibly, or someone asks.
- [ ] **Open a diagram or a picture from the keyboard** (added 2026-09-26, review of the Mermaid work).
  `.mermaid-diagram` and `img[data-file]` open their full-window view on a click only; neither
  is focusable. Fix: `tabindex="0"` plus `role="button"` and Enter/Space on both, and a visible
  focus ring. Reopen with the next accessibility pass, or when someone asks.
  *Same pass (review loop, 2026-09-26): keyboard focus on a link inside a diagram is lost when a
  light/dark change redraws it, since `redrawDiagrams` replaces the SVG. Fix: note the focused
  link's index among the figure's links before the swap, focus the same one after.*
- [ ] **Some Mermaid ids are the document's own names, not namespaced** (added 2026-09-26,
  third review of the Mermaid work). Swimlane clusters and state groups take the name written
  in the diagram as their id (`attr("id", t.id)` in the bundle). So a state named `icons` wears
  the app's `#icons { display: none }` and vanishes, and `[x](#tree)` with no such heading but
  a state named `tree` finds it in `#content`, sets `location.hash = "tree"`, and the browser
  scrolls to the sidebar's `#tree` instead. Needs a group named exactly like an app id. Fix:
  at warm time, rename every id in a diagram's SVG under the diagram's own id, and rewrite
  its `url(#…)` and `href="#…"` references to match (the overlay's rewrite is the model).
  Reopen when a real diagram hits it.
- [ ] **A Mermaid render that never settles would stall its window's diagrams** (added
  2026-09-26, third review). `showActive` awaits `warmDiagrams`, and renders queue one at a
  time per window, so one render that never settled would keep that window on the old page
  and every later diagram document waiting, until a reload. No known cause: the bundle waits
  on no network or fonts (its `fetch(` hits are KaTeX's parser), errors reject and are shown,
  and `maxTextSize`/`maxEdges` bound the work. Its timers and animation frames pause while the
  window is minimised, which delays but ends. A timeout would let an abandoned render overlap
  the next, the theme race the queue exists to prevent, and cannot stop a runaway loop.
  Reopen when a diagram document is reported never to show.
- [ ] **A document with a diagram has its HTML parsed twice** (added 2026-09-26, third
  review). `warmDiagrams` parses the whole rendered HTML into a `<template>` to find the
  sources, and `renderDocument` parses it again for the page. Documents without a Mermaid
  block skip it. Negligible on ordinary files; on a multi-MB one, plausibly tens of ms per open
  and per save (unmeasured). Fix: Rust returns the sources beside `html` on `Document`.
  Reopen when a large diagram document feels slow to update on save.
- [ ] **The diagram cache empties itself past 200** (added 2026-09-26, third review; the
  `ponytail:` note at `DIAGRAMS_KEPT`). A document with more than 200 distinct diagrams
  redraws all of them on every open, save and return, and documents in one window that
  together pass 200 evict each other's. Fix: keep the most recently used instead of clearing
  (about five lines on the `Map`'s insertion order). Reopen when such a document exists.
- [ ] **The diagram overlay duplicates the image viewer's zoom and pan** (added 2026-09-26,
  third review). About 70 lines of `openDiagram`/`zoomDiagram`/`onDiagram*` mirror
  `zoomTo`/`onImage*`, differing in the element and the state object, so a fix to one can miss
  the other (the close-mid-drag cursor was one). Kept apart on purpose when the overlay was
  planned, so shipped image-viewer code stayed untouched. Fix: parameterise the viewer
  functions on the view and its state, then re-check the image tab end to end (zoom, pan,
  history restore, resize). Reopen when either viewer next needs a behaviour change.
- [ ] **`docTarget`'s lookup order can differ from GitHub's** (added 2026-09-26, third
  review). It tries `user-content-<name>`, then the bare name, then the name without its
  prefix, all inside the document. With `## X` and `## User content X` both present,
  `#user-content-x` lands on the second, where GitHub picks the first. And a heading named
  `fn 1` (id `user-content-fn-1`) takes comrak's own footnote link `#fn-1` ahead of footnote 1;
  before the heading-id fix both carried `fn-1` and the first in the page won, so that
  ambiguity is older. Fix: for `fn-`/`fnref-` names, try the bare footnote first. Reopen when a
  real document hits either.
- [ ] **Link clicks on a page are dropped while its refresh draws diagrams** (added 2026-09-26,
  review loop). `onLinkClick` ignores link clicks while `shownToken !== renderToken`, so during
  a save or a switch in flight the links on the page on screen go dead (pictures and diagrams
  still open). The gate is older than Mermaid; diagrams
  stretch it from a few ms to seconds on a first load, or when many diagrams changed. Fix:
  let a click act on the page on screen and outdate the pending render. Reopen if someone
  notices dead links just after a save.
- [ ] **The overlay's id rewrite can change a label's text** (added 2026-09-26, review loop).
  `openDiagram` rewrites the diagram's own id in the SVG string, so a label whose text contains
  `#Mermaid-N` or `"Mermaid-N` for that diagram's id reads differently in the overlay (the
  page copy is untouched). Fix: rewrite ids on the parsed SVG (attributes and its `<style>`)
  rather than the string; the unnamespaced-ids item above wants the same routine. Reopen with
  that item, or when a real label hits it.
- [ ] **The foreignObject check runs after mermaid's temporary in-page render** (added
  2026-09-26, review loop). `mermaid.render` draws into a temporary element in the document
  before handing back the SVG string the backstop checks, so a drawing that is then refused
  had its filtered markup live for that moment. It matters only for an HTML-label route not
  yet known: the known ones are closed (math refused, HTML labels off) or accepted (event
  modeling). Fix: render in an isolated document, such as a sandboxed iframe. Reopen when a
  new HTML-label route is found, or on a mermaid major bump.

## Accepted limits

Not bugs to fix; here so nobody rediscovers them. The three the 2026-09-12 review parked were
all lifted or retired on 2026-09-25 (`closed-items.md`). A limit the owner rules to keep moves
to `closed-items.md`, under *Accepted limits*. None is waiting for a ruling: each limit is
ruled on as it is found.
