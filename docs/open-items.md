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

## First runs after Tauri 2.12

Each condition proves itself on an event that has not happened yet. Tick a sub-item when its
run passes, with the run id; when all are ticked, move the item to `closed-items.md`.

- [ ] **Tauri 2.12.1's crates have each run once for real** (added 2026-10-07,
  `archive/plans/tauri-2.12.md`).
  - [ ] *Updater 2.13.1's install path*: the first in-app update **from** a build carrying it —
    the release after the first one with tauri 2.12.1, since an update is installed by the
    outgoing version's updater. The installed copy is offered the new version, installs it and
    restarts into it, on Windows (NSIS) and macOS (the `.app.tar.gz`); the AppImage if walked.
    A failure strands that version's installs, with the manual download as the way out
    (t4-git-ui runs the same updater and accepted the same).

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
- [ ] **A macOS tab tear-off lands 32 pt high** (added 2026-09-28, the drive-app
  macOS run, `archive/plans/drive-app-macos.md`). On macOS `invoke("window_origin")` was measured
  to return the top of the window frame, not of the content: 32 pt higher, the title bar. The
  tear-off maps the cursor with it (`screenPoint` in `app.js`, then `drop_tab`'s
  `Placement::Cursor`), so the new window should land that much high. Cause unknown: tao
  0.35.3's `inner_position` reads as the content rect. Reopen when a macOS tear-off is seen
  landing high, or when `window_origin` is touched.
  *Seen 2026-10-07 on macOS 26.7.1, tauri 2.12.1 / tao 0.37.1 (`archive/plans/tauri-2.12.md`,
  walk W2): a real drag released at 200,140 pt opened the window's frame at 60,84 — 140 pt left and
  56 pt up of the cursor, 32 pt higher than the 140,24 the code intends (`drop_tab`,
  `main.rs:1149`). On Windows and Linux the same drop lands at exactly 140,24. That trigger
  fired; kept deferred by the owner 2026-10-07, now with the trigger *the tear-off placement
  work* (with *A torn-off window can land off-screen* and *A tab released level with the strip
  but outside the window reorders it*).*
- [ ] **On macOS Shift+Cmd+W closes a tab, not the window** (added 2026-09-28, the drive-app
  macOS run, measured). README gives `Shift+Cmd+W` as the shortcut for Close Window, and
  `macos_menu` binds it so. But the webview sees a real key before the menu, and `onKeydown`
  handles `w` with any Shift as close-tab with `preventDefault`, so the menu item never fires: a
  window with two tabs was left with one. Clicking **Window › Close Window** does close the
  window (measured), and README now says to use it; drop that sentence from README with the fix.
  Likely fix: let `onKeydown` skip Shift+Cmd+W on macOS so the menu gets it, checked with a real
  keystroke through `drive-app`'s `macos.md`. Reopen with the next macOS keyboard or menu work,
  or on a user report.
- [ ] **Diagrams are unproven on the macOS 13 floor** (added 2026-09-29, found after the Mermaid
  item closed). The app supports macOS 13 and up (`minimumSystemVersion` in
  `tauri.conf.json`), and the Mermaid item asked for a check on macOS 13's WKWebView, but it
  was closed on macOS 26.7 (`closed-items.md`). WKWebView uses the system WebKit, which Safari
  updates raise, so the worst case is a macOS 13 that never updated Safari past 16. If
  mermaid 11 or the sandboxed frame needs something newer, diagrams there stay as code blocks —
  reasoned from the versions; nothing is seen to fail. Check: install the release on macOS 13
  with Safari left at 16 (a UTM guest on Apple Silicon should do; untried) and open
  `examples/kitchen-sink.md` — the diagram draws, a click opens the overlay, and no second
  window opens. Reopen on a report from a macOS 13 user, before any
  mermaid bump (the release skill already holds 12.x back for this check), or when a macOS 13
  machine or VM is at hand.
- [ ] **`.jsonc` isn't associated with the app on macOS** (added 2026-09-28, the Mac run,
  measured on the installed 1.7.1). Finder's *Open With* on a `.jsonc` doesn't list the app, and
  macOS offers no app for it at all. The bundle's `T4MarkdownViewer.Json` type lists `json` and
  `jsonc` as extensions but claims `public.json` as its content type; `.jsonc` has no system
  type (it reads as a `dyn.` one), so LaunchServices binds only `public.json`. That macOS
  ignores the extension list once a content type is given is reasoned, not measured. `.json`
  works, and so does `.jsonc` on Windows, deb and rpm. Fix: declare a `.jsonc` type of the
  app's own in the bundle — unknown whether `tauri.conf.json`'s file associations can express
  one. Reopen when someone asks, or with the next file-association work.

The next eight came out of the Tauri 2.12 walks (`archive/plans/tauri-2.12.md`, 2026-10-07).
Each also shows on 1.7.2 or 1.7.1, so none comes from that change; each was deferred by the
owner at its triage.

- [ ] **A document with images comes back lower than it was left** (added 2026-10-07). A
  restore or Back sets the saved scroll one frame after render (`app.js:1413-1426`) without
  waiting for images, so an image above that spot that loads afterwards pushes the text down by
  its height. Measured: on Windows, README saved at 800 came back at about 1422 (the top
  screenshot's height), on the installed 1.7.1 (same Tauri as 1.7.2) and on tauri 2.12.1; on
  Linux, Back to README landed at 14079 for 15819. The Mac showed no drift (the image likely
  loaded first; reasoned). Likely fix:
  re-apply the saved scroll once images above it have loaded, or wait for them first; walk on
  all three, since timing differs per webview. Reopen with the session-restore plan, the next
  one now that the CLI 2.12.1 plan is done (with the next two items and *On GNOME, a restored
  window grows*).
- [ ] **On Windows, the last-used window doesn't come back in front** (added 2026-10-07). Two
  windows, the normal one used last, the other (`main`) maximized: after a restore `main` was
  in front, on the installed 1.7.1 and on 2.12.1. The Mac and Linux put the last-used one in
  front. Not investigated; a guess is that maximizing `main` after it shows also activates it,
  so it only happens with a maximized first window (untested — the first step is that probe).
  Reopen with the session-restore plan.
- [ ] **On GNOME, a restored window grows each launch** (added 2026-10-07, measured on Ubuntu
  26.04's GNOME Wayland). A normal window came back 52×89 px bigger each time (1100×860 →
  1152×949 → 1204×1038; 1.7.2: +52×99). `Frame::of` saves `inner_size()`, which there includes
  the GTK title bar and shadow, and `apply()` sets it back as the content size
  (`session.rs:106-128`). Windows and the Mac came back at exactly their size. Not measured:
  GNOME on X11, KDE, others. Likely fix: measure the decoration difference at runtime and save a
  size that round-trips. Reopen with the session-restore plan.
- [ ] **A torn-off window can land off-screen** (added 2026-10-07). On Windows and Linux a
  tab dropped away from every window opens one 140,24 px up-left of the cursor at full size,
  with no clamp (`drop_tab`, `main.rs:1149`): a drop at y=1100 on a 1440-high screen put the
  bottom at 1975 (measured, Windows). The title bar stays on screen then; a drop within 24 px of
  the top would put it above the screen (reasoned, untested). macOS clamps it itself. Likely
  fix: clamp to the work area of the monitor under the cursor, as `Frame::apply` checks
  monitors. Reopen with the tear-off placement work (with the macOS 32 pt item above and the
  next one), or on a report of a window landing off-screen.
- [ ] **A tab released level with the strip but outside the window reorders it** (added
  2026-10-07, seen on the Mac; the code is shared). `inStrip` (`app.js:3226-3229`) tests the
  pointer's height only, so a release at the strip's height far to the side reorders the tab to
  the end instead of tearing it off. Whether that was meant as a forgiving reorder is unknown.
  Likely fix: also require the pointer within the window's width plus slack. Reopen with the
  tear-off placement work.
- [ ] **On Linux, Ctrl+Shift+Tab doesn't go to the previous tab** (added 2026-10-07, seen
  with xdotool under Xvfb only). WebKitGTK reports the key as `Unidentified` (code `Tab`) and
  `onKeydown` matches `event.key === "Tab"` only (`app.js:4082`). Ctrl+Tab works; 1.7.2 the
  same. Whether a real keyboard does it too is unknown (Shift+Tab is the `ISO_Left_Tab` keysym
  on Linux, a guess at the cause). First step: press it on a real keyboard in the VM; likely fix
  one line, also matching `event.code === "Tab"`. Reopen with the next Linux walk, or on a
  report.
- [ ] **Ctrl/Cmd+Shift+T reopens a tab at the end of the strip** (added 2026-10-07, seen on
  the Mac; the code is shared). `reopenClosed` appends with `tabs.push` (`app.js:1924-1932`)
  and the closed-tab record keeps no position, where browsers put it back in its place. Likely
  fix: save the index on close and insert there. Reopen with the next tab-strip work, or on a
  report.
- [ ] **On macOS and Linux, some app text speaks Windows** (added 2026-10-07, the Mac's W12).
  The Open-mode hint says "double-click a `.md` file in Explorer" (`index.html:179`), and the
  Open button's tooltip says Ctrl+O on every platform (`index.html:30`, `app.js:2883`); macOS
  says Finder and Cmd. Likely fix: pick the words per platform. Reopen with the next UI-text or
  macOS keyboard work (the Shift+Cmd+W item), or on a report.

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
  before the menu. Cmd+Q reaches the menu (measured 2026-09-28 and 2026-09-29,
  `archive/plans/macos-quit-report.md`). Whether Cmd+H/M and the Edit menu's Cmd+A/C/V/X/Z do
  too is unknown; the page doesn't handle them today, so the menu probably still gets them.
  *Accepted 2026-09-28. Reopen with the deferred Shift+Cmd+W fix, which will test keys against
  the menu anyway.*
- [ ] **Un-maximizing a restored window loses where it was before the maximize** (added
  2026-10-07, the Tauri 2.12 walks). A window maximized at quit comes back maximized; un-maximized,
  it takes the default 1100×860 at the screen's corner (Windows: -8,-8; Mac: 0,34), not its
  earlier size and place. By design: `session.rs:113-118` keeps only a maximized frame's
  position, to pick the monitor, so un-maximizing doesn't leave a screen-sized window; the
  earlier rect is never saved. t4-git-ui saves it (its N6, about 100 lines).
  *Accepted 2026-10-07. Reopen on a report, or if the session-restore plan touches frames.*
- [ ] **Linux update checks off Debian are unwalked under updater 2.13** (added 2026-10-07,
  `archive/plans/tauri-2.12.md`). Updater 2.13 no longer points `SSL_CERT_FILE`/`SSL_CERT_DIR` at
  Debian's paths when they are unset (t4-git-ui's reading of the source). Debian and Ubuntu are
  unaffected (*Check now* passed on the Ubuntu VM); on Fedora, Arch and others the check might
  not verify GitHub's certificate, though rustls's platform verifier likely finds the system
  store (reasoned; nothing measured). The rpm means Fedora users exist.
  *Accepted 2026-10-07. Reopen on a report of a failing update check on a non-Debian
  distribution.*
- [ ] **Video and audio in documents are unwalked under tauri 2.12** (added 2026-10-07,
  `archive/plans/tauri-2.12.md`). Tauri 2.12 fixed malformed multi-range answers from the asset
  protocol (#15838), which media seeking can use; no fixture has video or audio, so none was
  played. An ordinary single-range request is not touched (reasoned from the release note).
  *Accepted 2026-10-07. Reopen on a report of video or audio in a document not playing or
  seeking.*
- [ ] **The AppImage fix is walked on Ubuntu 26.04 only** (added 2026-10-07,
  `archive/plans/tauri-cli-2.12.md`). The AppImage no longer bundles the build host's
  `libwayland-client` (linuxdeploy `07333c6`, and CI checks it stays out), which is what a recent
  Mesa failed on; other distributions with a recent Mesa (Fedora, Arch, …) are reasoned to be
  fixed the same way, not walked. The AppImage still bundles 22.04's `libwayland-cursor`, `-egl`
  and `-server`, harmless on 26.04 (measured) and shipped by t4-git-ui too.
  *Accepted 2026-10-07. Reopen on a report of the AppImage opening no window on any
  distribution.*
