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
- [ ] **A subframe navigating to the app's own page runs `stay`'s reload branch** (added
  2026-09-28, the Linux run). On WebKitGTK wry hands subframe navigations to `stay` with no
  way to tell them from the page's own (wry #1593; on macOS they reach the new-window handler
  instead, per the same issue), so a frame navigating itself to `tauri://localhost` queues a
  restore as a reload would. The only subframe is the sandboxed diagram frame, so this needs
  script running in it first — a mermaid bug past its sandbox. Worst case, plausibly, a reload
  loop. Fix: wry passing only main-frame navigations to the handler, as it does on WebView2, or
  telling it which frame (wry #1593, open). Reopen when wry ships that, or on any mermaid escape
  of its sandbox. A comment is drafted, not posted: `notes/wry-1593-comment.md`.
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
  *Added 2026-11-13 (`archive/plans/document-zoom.md`, triage): document zoom is unmeasured
  there too. It rests on CSS `zoom` on the document, on `cqw` inside it (asked of the engine
  at run time, `probeCqw`), and on `caretRangeFromPoint`; all were measured on macOS 26 only.
  The same check covers it: zoom `examples/kitchen-sink.md` to 50% and 300% with the keys and
  see the column stay centred and the top line stay put.*
- [ ] **`.jsonc` isn't associated with the app on macOS** (added 2026-09-28, the Mac run,
  measured on the installed 1.7.1). Finder's *Open With* on a `.jsonc` doesn't list the app, and
  macOS offers no app for it at all. The bundle's `T4MarkdownViewer.Json` type lists `json` and
  `jsonc` as extensions but claims `public.json` as its content type; `.jsonc` has no system
  type (it reads as a `dyn.` one), so LaunchServices binds only `public.json`. That macOS
  ignores the extension list once a content type is given is reasoned, not measured. `.json`
  works, and so does `.jsonc` on Windows, deb and rpm. Fix: declare a `.jsonc` type of the
  app's own in the bundle — unknown whether `tauri.conf.json`'s file associations can express
  one. Reopen when someone asks, or with the next file-association work.

The next two came out of the session-restore walks (`archive/plans/session-restore.md`,
2026-10-08); the first is how the plan's rule plays out, the second is Linux X11 only.

- [ ] **A window maximized right after it first opens un-maximizes to the default size** (added
  2026-10-08, the session-restore walks, Windows and the Mac). The normal rectangle comes only from
  a sample of the window un-maximized; a new window maximized within about a second, before its
  first report, has none, so after a restart it un-maximizes to 1100×860 at the screen's corner. As
  the plan's rule says (maximized with no old frame: `normal: false`). Likely fix (reasoned): record
  each window's starting frame when it is first shown. Reopen on a report, or with the next session
  or frame work.
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
- [ ] **A size for the document font** (added 2026-10-09, `archive/plans/document-font.md`, ruling F3).
  Settings › *Document font* picks the family only; the size stays the theme's. The walks
  (2026-10-09) measured Libron's x-height at 0.53 of the font size: about Azure DevOps' own
  (0.50 Segoe UI on Windows), but 15% above Tufte's Palatino (0.46), so Tufte reads visibly
  larger in Libron and its intro wraps a line sooner on the Mac. All three walkers found it
  readable; kept by the owner. A fix to start from: `font-size-adjust` on `#content` set to the
  theme font's own x-height ratio (support on WebKitGTK unverified). Reopen if Libron or another
  chosen font reads too small or large at the themes' sizes, or on a report about Tufte.
  The reading width (`archive/plans/reading-width.md`) is 75ch of the document font (100ch in a wide
  window), so a size change keeps the line length and moves the edge with it.
  *Kept deferred by the owner 2026-10-24 (`archive/plans/real-deferred-items.md`, Q-E1): no one
  has asked, and all three walkers found it readable.*
  *Kept deferred by the owner 2026-11-13 (`archive/plans/document-zoom.md`, triage): document
  zoom now lets a reader make everything bigger or smaller, in every document and theme at
  once, so a reader's own taste is covered. What is left here is a correction of one font
  against one theme. Same trigger.*
- [ ] **The JSON viewer's memory and layout cost per chunk** (added 2026-10-10, the find walks,
  `archive/plans/find-in-document.md` Q5). Each 512 KB chunk costs 400-500 MB of webview memory on
  Windows, with find open or closed; 17 chunks of a 9.75 MB file reached 9.7 GB on an 8 GB VM,
  which then thrashed. One fold takes 2.1-2.4 s to the next frame at 409 000 text nodes (Linux)
  and 6-7 s on the fully loaded file (2.28 M nodes, the Mac), and leaving that tab ~6 s; find
  closed or open alike. First step: measure which part of a chunk's rendered HTML dominates
  (fold buttons, spans per value). Reopen on a report of a JSON file exhausting memory or
  freezing, or with the next JSON viewer work.
  *Narrowed 2026-10-25 (`archive/plans/real-deferred-items.md`, Part D): measured, the sibling
  rule behind the fold's `…` stopped style sharing; with it gone and commas and colons bare, 4
  chunks take 27 % less memory on Windows (−47 % style objects). Fold time is unchanged. Kept
  open by the owner for the rest of the memory cost and the fold time, same trigger.*
  *Added 2026-11-13 (`archive/plans/document-zoom.md`, triage): a change of the document's
  zoom re-lays the whole page out, so on a large JSON tab one step takes 1.1 s at 151 000
  nodes and 2.3 s at 302 000 (Linux, measured); a wheel turn or a pinch of several steps
  queues that much per step. Leaving such a tab after its level changed blocked the page
  about 2.3 s at 302 000 nodes, against 0.65 s on 1.8.11 at 377 000 with no zoom (Linux,
  measured; the cause, tearing down the zoomed tree, is reasoned). Same trigger.*
- [ ] **On the Mac, find doesn't paint one match in a list item holding a nested list** (added
  2026-10-11, the Mac's reading-width walk). In `walk.md`'s "A long item: lorem…", which holds
  a nested list, the match is counted and stepped to ("1 of 8"), and the page scrolls to it,
  but neither the match nor the current highlight is coloured; the other matches paint. The
  build before the reading width does the same, and it stays with the item's width rule off.
  Windows and Linux counted and stepped but weren't checked for that highlight's pixels. Cause
  unknown. First step: reproduce on all three, and compare the match range's client rects with
  a painted one's. Reopen with the next find work, or on a report.
- [ ] **On the Mac, a clicked image-viewer button keeps a focus ring** (added 2026-10-11, the Mac's
  centred-column walk). After a click on the image viewer's + or Fit, the button shows the blue
  ring meant for keyboard focus, and it stays. Seen through the Mac's WebDriver clicks only, on
  the new build only; Windows and Linux didn't report it. The change touched that toolbar's
  spacing, not its focus styles. First step: a real mouse click on the Mac, on this build and on
  1.8.0. Reopen with the next image viewer work, or on a report.
- [ ] **Older commit messages carry Claude trailers and session links** (added 2026-10-13,
  `archive/plans/rewrite-commit-messages.md`). 279 commits end in `Co-Authored-By: Claude`,
  13 in a `Claude-Session:` link. A rewrite was planned and its drafts reviewed, but shelved:
  GitHub keeps `refs/pull/N/head` for every pull request, and those keep 244 of the 284 old
  commits fetchable by hash, all 13 session commits included, so a rewrite can't take them off
  GitHub. Only GitHub Support can, by deleting PRs #3–#11 and running garbage collection.
  New commits are covered by `CLAUDE.md` → "Commit messages" and by `attribution` turned off in
  the user settings. Reopen if the old messages must leave GitHub: start with the Support
  request, then the plan.
  *Kept deferred by the owner 2026-10-24 (`archive/plans/real-deferred-items.md`, Q-F1): the
  cost is lost PR pages and new hashes for every old commit.*
- [ ] **On Linux, a launch can fail with no crash report** (added 2026-10-20, the walks of the
  restore target and XWayland freeze fixes, Linux VM). Twice in ~58 walk launches: a debug build
  on native Wayland lost its WebDriver session at 0.4 s ("page crash or hang"; `session.json`
  stopped after its first write), and a locally built AppImage on Wayland, no driver, never
  started a web process or a window. Each rerun was fine; no `/var/crash` report either time.
  Not the XWayland freeze, which still shows a first paint. Cause unknown; the second launch's
  own output wasn't captured. Next Linux walks log the app's output and process state for every
  launch. Reopen on a report of the app not opening on Linux, or a third sighting in a walk.

- [ ] **On GNOME, a torn-off window opens with the pointer on its title bar, not its tabs**
  (added 2026-10-25, `archive/plans/real-deferred-items.md`, triage). The new window's outer
  frame goes to the drop point less 140×24 CSS px, and GNOME's title bar is 37 px tall, so the
  pointer ends 13 px above the page (measured once on the Linux VM, through `drop_tab` itself:
  its pointer input was down). The Linux side of the macOS 32 pt item fixed in the same plan. A
  fix steps back by the source window's title bar too, which the page would have to report.
  Reopen on a report, or once the Linux VM can walk real drags again.

- [ ] **A `link` on a sequence diagram's `actor`, `entity`, `control` or `boundary` stops the
  diagram drawing** (added 2026-10-25, `archive/plans/real-deferred-items.md`, triage). mermaid
  11.17.2 throws "Cannot read properties of null (reading 'class')" (Chromium; WebKit: "null is
  not an object (evaluating 'l.class')"), and the error shows in place of the diagram (measured
  on all three). The drawn-symbol types all fail (Windows VM, 2026-10-25); a `link` on a
  participant, database, queue or collections works. Not checked: newer mermaid, or an upstream
  report. Reopen at the next mermaid update, to recheck.

- [ ] **Several files opened at once on a cold start get a window each** (added 2026-11-10,
  `archive/plans/cargo-group-bump-pr23.md`, triage). Selecting four documents in Explorer and
  pressing Enter with the app not running starts four processes: the first becomes the app, and
  the other three hand their file over and exit. Each handed-over file opens in a window of its
  own, so four windows with one tab each (Windows VM: five runs on 793f82a's tree, three on
  1.8.10; one app process and no file lost every time). A booting window holds one waiting
  payload and the first file has it, so `open_path` (`main.rs:1520-1556`) gives a later file a
  new window: the rule `a_file_opened_while_main_boots_with_a_payload_gets_a_new_window` tests,
  written for a restored session or a torn-off tab. Reasoned, not measured: with the app
  already up, the same files open as tabs in the running window (`open_mode` other than
  `window`); and macOS and Linux do the same on a cold start. A fix lets a booting window hold
  a list of payloads, and the page open them all. Reopen on a report, or at the next change to
  how a booting window receives its documents.

- [ ] **On macOS, a pinch zooms a document in steps, not smoothly** (added 2026-11-13,
  `archive/plans/document-zoom.md`, the walk). A pinch walks the same 13 levels as the keys,
  one per 20 px of the pinch's wheel, each a full re-layout: 16 pinches by the owner's hand
  moved 3 to 6 levels each (100% to 150%, 110% to 300%), about one step fewer than the
  fingers asked for, and the owner found it "chunky or not smooth". Shipped as built by the
  owner's ruling, with a smooth pinch to follow the release. A way to start from: scale the
  page with the fingers during the pinch (a transform, no re-layout) and settle on a level
  when it ends. Unmeasured: blur or stutter on a large document, and the snap at the end
  unless a level off the ladder may be saved. The place-keeping of a wheel step
  (`zoomAnchor`) is replaced on that path, and with it the creep on empty space (*Accepted
  limits*). Reopen as the next change after the document-zoom release.

## First runs after the single-instance 2.5.5 bump

- [ ] **The restart after an update finds no single-instance lock** (added 2026-11-10,
  `archive/plans/cargo-group-bump-pr23.md`, triage). tauri-plugin-single-instance 2.5.5
  releases its lock in Tauri's `cleanup_before_exit`, no longer in its own `RunEvent::Exit`
  handler. After an update installs, the app restarts itself (`update.rs:197`); a lock still
  held then would make the new process hand over to the dying one and exit, and the app would
  not come back. Read in Tauri 2.12.5 (`app.rs:1449-1455`): the cleanup runs before the
  restart. It can't be walked without a newer release to install. Windows isn't in it: its
  updater exits the process itself (`updater.rs:1050`), and the mutex goes with the process.
  - [x] *macOS*: the first in-app update **from** a build carrying the bump (the release after
    the first one with it, since the outgoing version does the restart). The app comes back by
    itself. Walked 2026-11-13 on macOS 26.7.1, 1.8.11 → 1.9.1 (release run 43397401296): back
    by itself in 3 of 3 runs, 2.5–5.1 s after *Update now*, one new process, the tab restored.
    The lock's release was not observed, only that the new process stayed.
  - [ ] *The Linux AppImage*: the same. Walked 2026-11-13 on Ubuntu 26.04.1, 1.8.11 → 1.9.1:
    not proven. The app did not come back in 3 of 3 runs, for another reason, so the new
    process never reached the lock: the restart inherits the AppImage's `LD_LIBRARY_PATH`, and
    the new image's launcher dies in `/usr/bin/env` (`plans/appimage-child-environment.md`). Walk
    it again with the first update **from** a build carrying that fix.

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
- [ ] **Find with a real IME or a Cyrillic layout is unwalked** (added 2026-10-10, the find
  change review). No walk machine has an IME. Windows checked with synthetic key events that an
  Enter confirming IME input (`isComposing`, keyCode 229) doesn't step, and with raw key codes
  that a Cyrillic Ctrl+F opens find while Ctrl+Shift+F (А) opens nothing; WebKit (Linux, the Mac)
  ran neither.
  *Accepted 2026-10-10. Reopen on a report from an IME or Cyrillic user, or when a machine with an
  IME is at hand.*
- [ ] **Diagrams on Windows depend on turning `IsolateSandboxedIframes` off** (added 2026-10-10,
  `archive/plans/diagram-frame-isolation.md`, ruling D3). An isolated sandboxed frame's requests
  to the app's files go unanswered (WebView2 154), though wry asks for every request source
  (`COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL`). The fix keeps the frame un-isolated
  with a browser argument; a WebView2 that drops the feature name, or turns isolation on some
  other way, breaks diagrams again. Not reported upstream.
  *Accepted 2026-10-10. Reopen if diagrams break on Windows again, if a WebView2 release drops
  the feature name, or when wry or WebView2 serves such frames.*
- [ ] **Typing lags in text fields on a huge page** (added 2026-10-10, the find walks,
  `archive/plans/find-in-document.md` Q4). On a fully loaded 9.3 MB JSON (2.28 M text nodes), the Mac
  freezes ~0.4-0.6 s when a typed edit empties a text field or types into an empty one: the find
  field and the sidebar filter alike, not when code empties it, and not on a normal page. Not
  find's code, `:has()` rules, the placeholder, or style and layout visible to JavaScript (each
  removed and measured); likely WebKit's or AppKit's own work (unverified). On Windows every
  keystroke costs ~0.2 s on a 519 000-element page: ~70 ms for any text field, ~100 ms more in
  the find field (cause unmeasured). Linux: none at 409 000 nodes.
  *Accepted 2026-10-10. Reopen on a report of typing lag on large documents.*
- [ ] **Wide content nested in a list sits off-centre** (added 2026-10-11,
  `archive/plans/reading-width-centred.md` M1). A table, diagram or picture wider than the reading column
  is centred on its own box, so inside a list item or quote it is centred on that item, shifted by
  half the indents: three levels deep, a pane-wide table sat 42 px right of centre at 1600 px
  (Windows, Linux; the Mac 91 px left against 7 right at 1512). In a narrow window its box can
  pass the pane by a few px, which `overflow-x: clip` on the document cuts (2 px measured); the
  page doesn't scroll sideways. A fix to start from: JavaScript that measures each nested block's
  indent and shifts it back, again on resize.
  *Accepted 2026-10-11. Reopen on a report of nested wide content looking off-centre or cut, or
  with the next layout work.*
  *Added 2026-11-13 (`archive/plans/document-zoom.md`, triage): zoomed to 300% in a 700 px
  window with the sidebar open, a table nested in a list passed the pane's edge by 2 px and
  was cut the same way (Linux, measured). Same trigger.*
- [ ] **A command in an unlabelled code block scrolls instead of wrapping** (added 2026-10-11,
  `archive/plans/reading-width-centred.md` M2). Only blocks in a shell language (`sh`, `bash`, `shell`,
  `zsh`, `fish`, `console`, `shellsession`, `powershell`, `ps1`, `pwsh`, `cmd`, `bat`, `batch`)
  wrap with a hanging indent; a bare fence or ` ```text ` scrolls like program code. Guessing
  from the text (a `$ ` prompt) would misfire on some program code.
  *Accepted 2026-10-11. Reopen on a report of commands in unlabelled blocks scrolling, or a
  request to wrap them.*
- [ ] **A full re-layout may be slightly slower on Windows** (added 2026-10-11, the
  centred-column walks). Timing a full re-layout against 1.8.0: Linux and the Mac within noise
  (README x30 225 → 230 ms and 109 → 114 ms; a ~10 MB JSON's first chunk 1030 → 1075 ms and
  274 → 277 ms); Windows a little slower in both run orders (README x30 ~200 → ~225 ms, JSON
  ~685 → ~730 ms), with a spread inside each run as large as the gap. Fold and unfold
  unchanged. Not isolated: likely the split command blocks and the centred wide content.
  *Accepted 2026-10-11. Reopen on a report of slowness on large documents.*
- [ ] **A very long command block's cost is unmeasured** (added 2026-10-11, the centred-column
  triage). Each shell block is rebuilt after highlighting, one element per line and one per
  hyphen; the walks used blocks of a few lines. A long ` ```console ` log gets thousands of extra
  elements. A fix to start from: split only blocks under a line count, and let longer ones
  scroll.
  *Accepted 2026-10-11. Reopen on a report of a slow page with a long log, or with the next
  performance work.*
- [ ] **A table column's floor is judged by character count** (added 2026-10-11,
  `archive/plans/table-cap.md`). A column gets its 12ch floor when a cell has more than 14 characters,
  standing in for "would this column wrap". Characters about 2ch wide, as in Chinese, Japanese
  or Korean, which can also break between any two, are undercounted: a column of 8-character
  phrases (about 16ch) gets no floor and could squeeze to a character a line in a crowded
  table. Reasoned, not measured; Latin text is unaffected. A fix to start from: count wide
  characters double, or measure the columns' real widths.
  *Accepted 2026-10-11. Reopen on a report of a squeezed column in CJK or other non-Latin text,
  or a request to support such documents.*
- [ ] **A table written in HTML gets no column floors** (added 2026-10-11,
  `archive/plans/table-cap.md`, ruling R2). With no `<th>` header row the floor class lands on a `<td>`
  the CSS doesn't match, and a `colspan` makes the floor check count columns wrong. Such a
  table is still capped at 100ch, its columns shrinking to their longest word. Markdown tables
  always have a header row and no spans.
  *Accepted 2026-10-11. Reopen on a report of a hand-written HTML table with squeezed
  columns.*
- [ ] **A gantt in a restored tab on Linux is drawn at the default window width** (added
  2026-10-11, the launch-gantt fix `94bb3e8`). Diagrams in the documents a window opens at
  launch are drawn before it shows; while hidden, WebKitGTK gives the page no width, so the
  window's outer width stands in. That is still tauri.conf's 1100 when the session brings the
  window back at a saved size (measured: restored at 1500, gantt 1100 wide, centred in the
  pane). Not blank, and the same as any gantt after a resize, which is not redrawn at the new
  width. Windows and macOS unmeasured.
  *Accepted 2026-10-11. Reopen if gantts start redrawing on resize, or on a report of a narrow
  restored gantt.*
- [ ] **A wheel over the tab strip as a document restores can leave it a line or two off**
  (added 2026-10-18, the tab-strip walks, Windows). The first notch of a wheel gesture can't be
  cancelled, and when it lands in the same millisecond the hold starts (9 of 10 runs), it scrolls
  the page before the hold listens: the page ended 19-37 px past its spot. Later notches are
  handled.
  *Accepted 2026-10-18. Reopen on a report of a restored document drifting after a wheel over
  the tabs.*
- [ ] **The strip's wheel test is unwalked at real fractional display scaling** (added
  2026-10-18, the tab-strip walks). It allows 1 px of slack for rounding. A probe on Windows set
  the strip to fractional widths 0.2 px apart at device scales 1, 1.25 and 1.5: a strip that
  fits never read as over. But emulated scaling keeps whole-pixel layout, so a real 125 % or
  150 % display, whose viewport is fractional, wasn't tried.
  *Accepted 2026-10-18. Reopen on a report of a plain wheel over a strip that fits doing nothing
  on a scaled display.*
- [ ] **Right after a reload or a switch, a document's saved spot reads 0 for a few
  milliseconds** (added 2026-10-19, `archive/plans/restore-banks-its-target.md`, triage 1). After
  the swap the new page sits at its top until its restore's first frame, and the scroll listener
  banks 0 on the incoming entry: 6-21 ms on the Mac, in every live reload and switch back; no
  report carried it (`session.json` stayed at 20000). A quit inside that gap would save 0
  (reasoned). Likely as old as the restore. A fix would set the hold's target before the frame.
  *Accepted 2026-10-19. Reopen on a report of a document reopening at its top.*
- [ ] **The XWayland freeze workaround isn't applied on a native Xorg session, or under XWayland
  without Wayland markers** (added 2026-10-20, `archive/plans/xwayland-freeze.md`, triage 1).
  `WEBKIT_DISABLE_DMABUF_RENDERER=1` is set only for X11 on a Wayland desktop, told by
  `WAYLAND_DISPLAY` or `XDG_SESSION_TYPE=wayland` (owner's Q1, option b). A native Xorg session
  is unmeasured (no Xorg session on the VM); an AppImage started on the Wayland desktop with
  both markers removed ran under XWayland, wasn't detected, and froze (1 run). Normal desktop
  launches carry the markers (reasoned). Wider: set it for any X11 (Xorg users pay ~+7-27
  points of web CPU while scrolling), or ask the X server whether it is XWayland (before any
  thread: a bigger change).
  *Accepted 2026-10-20. Reopen on a report of the Linux app freezing after its first paint.*
- [ ] **With Narrator in scan mode, Enter can move keyboard focus out of the page** (added
  2026-10-24, `archive/plans/real-deferred-items.md`, triage). Enter on a control sometimes left
  Windows focus on WebView2's inner "Chrome Legacy Window", and no key reached the page until
  focus was put back. Measured on the Windows VM: the theme button 3 of 5, an in-page link 4 of
  5, a sequence-diagram actor 2 of 5, so not the actor code. In all 15 tries the control got
  focus once before Narrator's click when it jumped (9 of 9) and twice when it didn't (6 of 6);
  a race inside Narrator is reasoned from that, not proven. A real mouse click never did it, and
  `appWindow.setFocus()` didn't recover it.
  *Accepted 2026-10-24. Reopen on a report, or when a WebView2 or Windows update changes
  Narrator's activation.*
- [ ] **A tab torn off onto a second monitor with other scaling is walked on Windows only**
  (added 2026-10-25, `archive/plans/real-deferred-items.md`, triage). The new window is clamped
  at the size it takes on the target monitor (its size × the target's scale / its own), and
  stepped back from the pointer at the target's scale. Walked on the Windows VM across 100 % and
  150 % monitors in both directions; macOS runs the same math (reasoned), with no two-monitor
  setup to walk it on. On GNOME under XWayland the app sees every monitor at the same scale
  (measured at 100 % and 200 %), so there the math never applies; the Linux VM walked two
  monitors at those scales and the window landed inside the target's work area.
  *Accepted 2026-10-25. Reopen on a report of a torn-off window landing wrong on a second
  monitor on macOS or Linux.*
- [ ] **On Linux, a press on a wide diagram's scrollbar is only weakly shown not to open the
  full-window view** (added 2026-10-25, `archive/plans/real-deferred-items.md`, triage). GTK's
  overlay scrollbar takes no layout space, so the box's bottom edge can't be told from the bar
  by position. A real press at the bottom 4 px opened nothing, and a drag there scrolled the box
  (0 → 17 px), which a drag on the drawing doesn't do (reasoned: the press hit the bar). Windows
  and macOS pass. If wrong, the view opens and Escape closes it.
  *Accepted 2026-10-25. Reopen on a report of the full-window view opening from a scrollbar
  press.*
- [ ] **On Linux, Orca still reads a sequence diagram's actor names twice** (added 2026-10-25,
  `archive/plans/real-deferred-items.md`, triage). Each bottom actor box is `aria-hidden`, which
  fixes Narrator and VoiceOver (measured), but WebKitGTK builds the figure's text from all its
  drawn text regardless: Orca 50.2 still said "Bob Alice Alice Bob Hello Hi" (measured; the
  AT-SPI children were right). The same as before the fix, so nothing got worse.
  *Accepted 2026-10-25. Reopen on a WebKitGTK update that respects `aria-hidden` in a group's
  text, or on a report.*
- [ ] **On Linux, Orca doesn't say whose actor menu focus moved into** (added 2026-10-25,
  `archive/plans/real-deferred-items.md`, triage). Each menu is a group named "<actor>'s links";
  Narrator and VoiceOver say it on entry (measured), but Orca 50.2 says only "Home link"
  (measured; AT-SPI shows the named panel around the link). Orca seems not to announce a named
  panel on a focus change (reasoned).
  *Accepted 2026-10-25. Reopen on an Orca update that announces it, or on a report.*
- [ ] **On GNOME, a tab torn off onto a second monitor shows on the main one for a moment**
  (added 2026-10-25, `archive/plans/real-deferred-items.md`, triage). GNOME maps every new
  window on the main monitor whatever position was asked for; the app moves it again once the
  page shows it, about 16 ms after the map (measured roughly from X event times; no capture was
  fast enough to see whether it flashes).
  *Accepted 2026-10-25. Reopen on a report of a torn-off window flashing on the wrong monitor.*
- [ ] **After a tear-off, the source window once reported a 1102×862 page, not 1100×860** (added
  2026-10-25, `archive/plans/real-deferred-items.md`, triage). Seen once on the Windows VM in
  the two-monitor walk: `innerWidth`×`innerHeight` changed while its outer rect stayed 1116×899.
  Nothing in the tear-off change touches the source window; cause not traced.
  *Accepted 2026-10-25. Reopen on a report of a window changing size by itself.*
- [ ] **A zoom step with no text to hold lands a line or two off** (added 2026-11-13,
  `archive/plans/document-zoom.md`, triage). A change of the document's zoom keeps the line
  of text at the pointer (a wheel step) or at the top of the view (a key) where it was, to
  under 1 px (measured on all three). Where that point has no text, the page holds a point of
  the element there instead, and text re-wrapping around it moves the view: a step made from
  Settings (up to 4.2 px), under the tab switcher (up to 10.1 px over a tall table), on a
  heading's *Copy section* button (up to 6.6 px) or in a table's white space on Linux (up to
  6.4 px), all measured. The plan's bound is two lines of text.
  *Accepted 2026-11-13. Reopen on a report of the page jumping on a zoom step.*
- [ ] **On macOS and Linux, a burst of zoom steps on empty space creeps a few px** (added
  2026-11-13, `archive/plans/document-zoom.md`, the re-walk). With the pointer where there is
  no text (beside a heading's text, on its button), each step leaves the page about 0.4 px
  lower, always the same way: after four steps in and four out the heading ended 3 px
  (Linux) and 4 px (macOS) from its start, after eight and eight 2 to 5 px and 6 px; Windows
  ends at 0.00 (all measured). On text the same loss is made up by the next step (`zoomHeld`;
  worst 0.89 px over eight steps). Reasoned, not measured: WebKit rounds the page's scroll
  offset down, WebView2 to the nearest px. A fix to start from: round the scroll on that path.
  *Accepted 2026-11-13. Reopen with the smooth pinch on macOS (*Deferred*), or on a report.*
- [ ] **A tab not on screen comes back near its place, not on it, after a change of zoom**
  (added 2026-11-13, `archive/plans/document-zoom.md`, triage). A document that isn't shown
  isn't laid out, so its saved offset is multiplied by the ratio of the two levels, and text
  that re-wraps moves under it: 32 to 35 px in the walk (a hidden tab, a tab reopened with
  `Ctrl+Shift+T`, a document left for a picture tab), 16 to 254 px in the trials on long
  documents (measured).
  *Accepted 2026-11-13. Reopen on a report of a tab coming back at the wrong place after
  zooming.*
- [ ] **A change of zoom just as a document is shown can land it off** (added 2026-11-13,
  `archive/plans/document-zoom.md`, triage). A level that changes in the frame or two between
  a document being drawn and its place being restored restores the old level's offset: 24 ms
  after a tab switch the line ended 28.5 px off (Windows, measured); at 146 ms and later it
  held to 0.5 px (macOS). It needs a step from another window, or a key within a frame of the
  switch. Seen beside it, on Windows: switching back to a document with pictures at a zoomed
  level, the page read 891 px off at 23 ms and was exact from 102 ms on, about once in three
  runs, on the build before the walk's fixes too and not at 100%; whether it shows on screen
  is unmeasured.
  *Accepted 2026-11-13. Reopen on a report of a document landing off, or flashing another
  place, after a tab switch while zoomed.*
- [ ] **A session offered on the empty panel keeps the old zoom's places** (added 2026-11-13,
  `archive/plans/document-zoom.md`, triage; reasoned, not walked). The tabs behind *Reopen*
  are not open in any window, so a change of level before the offer is taken doesn't scale
  their saved offsets, and they come back as far off as the two levels differ.
  *Accepted 2026-11-13. Reopen on a report of reopened tabs landing off after zooming.*
- [ ] **On Linux, a tab reopened after zoom steps once read as at its top** (added 2026-11-13,
  `archive/plans/document-zoom.md`, the walk). In the first walk, 3 of 7 tries of
  `Ctrl+Shift+T` after three steps, with a 3 MB JSON tab on screen, read scroll offset 0
  with the saved offset untouched; 0 of 91 tries since, on 1.8.11 and on the walk's build.
  Shown since: after the key the page is blocked while the JSON tab is torn down (2.3 s at
  302 000 nodes), the restore comes one frame later, and a reading taken at 2.0 s falls just
  before it, 10 of 10; a second later the page is on its line. Not explained: the first
  walk's tab had 151 000 nodes, where the block is 0.45 s.
  *Accepted 2026-11-13. Reopen on a report of a reopened tab at its top.*
- [ ] **Document zoom is unwalked with some real input devices** (added 2026-11-13,
  `archive/plans/document-zoom.md`, triage). Not tried: a real mouse wheel on a scaled
  Windows display; a touchpad pinch on Windows and Linux, which zooms wherever the webview
  reports it as `Ctrl`+wheel; and on macOS, by hand, a pinch in the full-window diagram view
  and `Control`+wheel (both pass with scripted events; the owner's hand tried the document
  and a picture).
  *Accepted 2026-11-13. Reopen on a report, or when the hardware is at hand.*
- [ ] **A reader's own theme that turns the reading width off may get a limit back when
  zoomed out** (added 2026-11-13, `archive/plans/document-zoom.md`, triage; reasoned, not
  run). `themes/README.md` turns the measure off with `100vw`; on Windows and macOS `vw`
  scales inside the zoomed document, so below 100% the column may stop short of the pane. No
  bundled theme sets a measure.
  *Accepted 2026-11-13. Reopen on a report from someone with a theme of their own.*
- [ ] **A few diagram labels sit a little off at 300%** (added 2026-11-13,
  `archive/plans/document-zoom.md`, Task 1c and the walk). A label anchored at its left edge
  sits up to 7.5 px left of its place on Linux and 2 px on Windows; a pie's label 1.3 px up
  or down on Windows, others about 1 px on Linux (measured). At 100% every label is where
  1.8.11 put it.
  *Accepted 2026-11-13. Reopen at the next mermaid update, or on a report.*
