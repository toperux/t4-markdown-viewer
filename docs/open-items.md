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
  `main.rs:1170`). On Windows and Linux the same drop lands at exactly 140,24. That trigger
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

The next five came out of the Tauri 2.12 walks (`archive/plans/tauri-2.12.md`, 2026-10-07).
Each also shows on 1.7.2 or 1.7.1, so none comes from that change; each was deferred by the
owner at its triage.

- [ ] **A torn-off window can land off-screen** (added 2026-10-07). On Windows and Linux a
  tab dropped away from every window opens one 140,24 px up-left of the cursor at full size,
  with no clamp (`drop_tab`, `main.rs:1170`): a drop at y=1100 on a 1440-high screen put the
  bottom at 1975 (measured, Windows). The title bar stays on screen then; a drop within 24 px of
  the top would put it above the screen (reasoned, untested). macOS clamps it itself. Likely
  fix: clamp to the work area of the monitor under the cursor, as `Frame::apply` checks
  monitors. Reopen with the tear-off placement work (with the macOS 32 pt item above and the
  next one), or on a report of a window landing off-screen.
- [ ] **A tab released level with the strip but outside the window reorders it** (added
  2026-10-07, seen on the Mac; the code is shared). `inStrip` (`app.js:3378`) tests the
  pointer's height only, so a release at the strip's height far to the side reorders the tab to
  the end instead of tearing it off. Whether that was meant as a forgiving reorder is unknown.
  Likely fix: also require the pointer within the window's width plus slack. Reopen with the
  tear-off placement work.
- [ ] **On Linux, Ctrl+Shift+Tab doesn't go to the previous tab** (added 2026-10-07, seen
  with xdotool under Xvfb only). WebKitGTK reports the key as `Unidentified` (code `Tab`) and
  `onKeydown` matches `event.key === "Tab"` only (`app.js:4234`). Ctrl+Tab works; 1.7.2 the
  same. Whether a real keyboard does it too is unknown (Shift+Tab is the `ISO_Left_Tab` keysym
  on Linux, a guess at the cause). First step: press it on a real keyboard in the VM; likely fix
  one line, also matching `event.code === "Tab"`. Reopen with the next Linux walk, or on a
  report.
- [ ] **Ctrl/Cmd+Shift+T reopens a tab at the end of the strip** (added 2026-10-07, seen on
  the Mac; the code is shared). `reopenClosed` appends with `tabs.push` (`app.js:2066-2074`)
  and the closed-tab record keeps no position, where browsers put it back in its place. Likely
  fix: save the index on close and insert there. Reopen with the next tab-strip work, or on a
  report.
- [ ] **On macOS and Linux, some app text speaks Windows** (added 2026-10-07, the Mac's W12).
  The Open-mode hint says "double-click a `.md` file in Explorer" (`index.html:179`), and the
  Open button's tooltip says Ctrl+O on every platform (`index.html:30`, `app.js:2883`); macOS
  says Finder and Cmd. Likely fix: pick the words per platform. Reopen with the next UI-text or
  macOS keyboard work (the Shift+Cmd+W item), or on a report.

The next three came out of the session-restore walks (`archive/plans/session-restore.md`,
2026-10-08); the first is how the plan's rule plays out, the other two are Linux X11 only.

- [ ] **A window maximized right after it first opens un-maximizes to the default size** (added
  2026-10-08, the session-restore walks, Windows and the Mac). The normal rectangle comes only from
  a sample of the window un-maximized; a new window maximized within about a second, before its
  first report, has none, so after a restart it un-maximizes to 1100×860 at the screen's corner. As
  the plan's rule says (maximized with no old frame: `normal: false`). Likely fix (reasoned): record
  each window's starting frame when it is first shown. Reopen on a report, or with the next session
  or frame work.
- [ ] **On Linux X11, a long or image-heavy document can freeze the page** (added 2026-10-08, the
  session-restore walks on the Linux VM). Under XWayland (`GDK_BACKEND=x11`) the window resizes but
  the page stays a stale picture and stops answering; its web process waits on a futex and the UI
  loop is idle. 5 of 5 on *Reopen* with README, and 3 of 3 on a plain launch of a long noise-image
  document, on 1.7.3 (b5a8220) and every commit of the plan alike; Wayland 0 of 3, small documents
  0 of 3. `WEBKIT_DISABLE_DMABUF_RENDERER=1` avoided it (0 of 2), as did disabling compositing:
  likely WebKitGTK 2.52's DMA-BUF renderer under XWayland. Not measured: the AppImage (it forces X11
  but bundles 22.04's older WebKit), a native X11 session, the variable's cost. Likely fix: set it
  whenever the app runs on X11, beside `gdk_backend` in `main.rs`.
  *Deferred by the owner 2026-10-08. Reopen: next, as its own plan, which probes the AppImage
  first.*
- [ ] **On Linux X11, un-maximizing a restored window keeps its size but not its place** (added
  2026-10-08, the session-restore walks on the Linux VM). A 900×700 window at 300,200, maximized,
  came back maximized through five relaunches and un-maximized to 900×700 at 67,32, the work area's
  corner. Windows and the Mac return exactly; Wayland can't place windows. Better than 1.7.3, which
  gave the default size there. Guess (unmeasured): the position set on the hidden window doesn't
  survive its being mapped already maximized under Mutter. Reopen with the tear-off placement work,
  or on a report.
- [ ] **Settings' *Check now* may sit out of reach below the dialog** (added 2026-10-08, the 1.7.3 →
  1.7.4 update walk on the Windows VM, 1920×1200). With the update dialog showing, the VM opened
  Settings › *Check now*, but the button sat below the bottom edge of the Settings dialog and a
  click there registered nothing (an empty status line); the offer came from the startup check
  instead. Seen once, with 1.7.3. Not known: whether the Settings content scrolls there, whether the
  update dialog on top hid it, or whether a default-size window shows it fine. The Mac's *Check now*
  worked. First step: reproduce on the Windows VM with and without the update dialog open, at the
  window's default size. Reopen with the next UI work, or on a report.

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
- [ ] **Right after *Reopen*, `session.json` can list the windows in the wrong order for a moment**
  (added 2026-10-08, the session-restore walks, the Mac). A copy taken about 0.3 s after *Reopen*
  had the last-used window first (`session.json` lists windows back to front, so the last-used one
  belongs last); 2.5 s later it was right. Only a crash in that moment brings the wrong window to
  the front at the next launch. Cause not traced.
  *Accepted 2026-10-08. Reopen on a report.*
- [ ] **With three or more windows, only the last-used one comes back on top** (added 2026-10-08,
  `archive/plans/session-restore.md`, ruling R4). The windows behind it stack in the order they
  finish loading, which becomes their saved order at the next quit. A full fix chains the hand-off
  from window to window.
  *Accepted 2026-10-08. Reopen on a report.*
- [ ] **A restored scroll position can be off, or pull back once, in two rare cases** (added
  2026-10-08, `archive/plans/session-restore.md`, ruling R7; reasoned, not walked). A window that
  grows taller while the saved spot is near the end of the document clamps the scroll with the
  height unchanged, so the hold stops early and that document may land a little off. And an overlay
  scrollbar (macOS, some GTK themes) dragged while an image lands in the same frame is pulled back
  once: the scrollbar-press stop sees only classic scrollbars.
  *Accepted 2026-10-08. Reopen on a report of a restored document landing off, or of a scroll pulled
  back.*
- [ ] **A restored un-maximized window's size at 200 % on Wayland is unwalked** (added 2026-10-08,
  `archive/plans/session-restore.md`). The VM's screen was too small at 200 %, so GNOME maximized
  the window each time. At 100 % the size round-trips on Wayland and X11, and at 200 % on X11 the
  window's size matched GTK's size times the scale (893×531 × 2 = 1786×1062); on Wayland, where the
  fix applies, it was not checked at 200 %.
  *Accepted 2026-10-08. Reopen on a report of a restored window changing size on a scaled Linux
  display.*
- [ ] **Un-maximize across two monitors is unwalked** (added 2026-10-08, the session-restore walks).
  Which monitor a maximized window comes back on, and when its old size is kept (rulings R1, R5, R6
  in `archive/plans/session-restore.md`: kept while it is on the monitor the window is maximized on,
  judged by the monitor it overlaps most; always on Wayland), and a monitor unplugged since the last
  run, are covered by unit tests only: every VM and the Mac had one display.
  *Accepted 2026-10-08. Reopen when a second display is at hand, or on a report.*
