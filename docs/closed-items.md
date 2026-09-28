# Closed items — t4-markdown-viewer

Items ticked in `open-items.md`, moved here with their notes so that file lists only what is
still to do. Same sections as there; a newly closed item goes at the end of its section.

## Needs a decision

- [x] **Reply to and close issue #1, "Cannot open folder"** (macOS, opened 2026-09-10, no
  comments). The reporter wanted to open a whole folder from the start screen. The
  "Open a folder…" button beside "Open a file…" (10c7458) shipped in 1.5.8, and a5e5fef (1.5.9)
  points to the sidebar once a folder is open. The reply is public, so the owner posts it or
  approves it. Ask the reporter to confirm the Mac update item below too.
  *Closed 2026-09-11 with a comment that also asks about the Mac update.*
- [x] **Delete the stale branches.** None holds work missing from `main`; each was squashed in.
  *Deleted 2026-09-11. Tips, for the reflog: e2bcf54, 1158096, cab1798, 244463f, 03e8c27,
  97bfa6b.*
  - local: `backup-pre-fixup`, `backup-pre-plandoc`, `backup-pre-squash`,
    `backup-pre-theme-squash`, `session-restore`, `cross-platform`
  - remote: `origin/cross-platform`
- [x] **Frontend tests, or not.** `src/app.js` has no automated tests, and the UI is only ever
  checked by hand with the `drive-app` skill. Features with manual checks only: sidebar filter,
  update from Settings, sidebar folder switching, the folder picker, the empty-screen hints.
  A harness means adding Node tooling, which the repo has avoided so far.
  *Decided 2026-09-11: stay manual. `drive-app` is the check, and the repo stays free of Node
  tooling.*
- [x] **Dependabot security updates are on.** Round 2 left `automated-security-fixes` off on
  purpose, but it was enabled when checked on 2026-09-11. `ci.yml` runs only on pushes to
  `main`, so every Dependabot PR (security fix or version bump) goes unchecked until it's
  merged, and CI runs on `main` only after that. Options: keep it, turn it off, or have CI run on
  Dependabot's branches.
  *Settled by the code review (9910b77, 1.5.11): `ci.yml` now runs on `pull_request` too, so
  Dependabot PRs get the full matrix before merge. Security updates stay on. Proof: PR #3's
  checks ran (and failed, see Bugs).*
- [x] **A stale Reopen offer can outlive the session it describes** (added 2026-09-17, 1.6.3).
  With **Ask me each time**: ignore the offer, open a file, then close that tab. The empty
  screen comes back with the Reopen button still on it, and pressing it restores the session
  Rust is still holding in memory — even though `session.json` was overwritten by the file you
  opened seconds earlier. Arguably a courtesy rather than a fault: the stash is intact and
  nothing else can reach it. Options: leave it, or hide the button the first time this window
  opens anything.
  *Decided 2026-09-24: hide it. `show()` in `app.js` now hides the button whenever anything
  but the empty screen goes up. Driven on the debug build under `"reopen": "ask"`: the offer
  showed at launch ("Reopen 2 tabs in 1 window"), was gone once a file opened, and stayed gone
  after that tab closed; on a fresh launch, pressing it still restored both tabs.*
- [x] **Later does not cancel an update that has started** (added 2026-09-20, from #5 of
  `archive/reviews/code-review-2026-09-20.md`). Pressing **Update now** and then **Later** closes the
  dialog, but the download carries on and the app still restarts under the reader. Since
  769cd54 a reopened dialog says an install is under way; nothing makes Later mean it. Options:
  leave it (the reader did ask for the update), relabel the button to **Hide** once a download
  is running, or make it cancel — which needs the updater plugin's download to be abortable,
  unchecked.
  *Decided 2026-09-24: relabel. The button reads **Hide** from the moment an install starts,
  on a reopened dialog too, and **Later** again if it fails. Driven on the debug build with a
  faked `state.update` and a real `runUpdate()`, which Rust refused as there is nothing newer
  than 1.6.6: Later → Hide (Update now disabled) → Hide on reopen → Later after the failure.
  A review the same day found another window's already-open dialog kept Later and an enabled
  Update now; `setInstalling` now sets the flag and both buttons together, in every window, as
  the broadcast arrives. Driven with two windows: w1's open dialog went Later → Hide (Update
  now disabled) on main's `update-progress`, and back on `update-failed`.*
- [x] **A launch that dies mid-restore loses the whole session** (added 2026-09-21, found
  answering the crash-loop question below). `setup` discards `session.json`
  before it restores, and a window writes it again only once its document is laid out; a
  process that goes down in between leaves no file, so the next launch is the empty screen —
  the documents that were fine went with the one that was not. That is also exactly what stops
  a bad document coming back for ever. Options: leave it (one lost session for a guaranteed way
  out), or discard only once the first window has reported, and count failed restores so a
  second one in a row starts fresh.
  *Decided 2026-09-24: a third option — offer instead of restore. `setup` no longer discards a
  session it is about to put back; it rewrites it marked `restoring` (`session::begin_restore`),
  saves keep the mark while any restored window is still loading, and the file is consumed as
  before once the last one reports with nothing loading (`settled` on `set_session`). A launch
  that finds the mark offers the session with the Reopen button whatever the setting bar off
  (`session::offers`) — except one started on a file, which has no empty screen to offer on,
  so the file takes the session's place as under `ask` (the owner chose that over restoring
  beside the file, which would reload a crashing document on every double-click). The first drive found the mark cleared 0.5 s into a Reopen: the click's
  focus report landed mid-load, hence `settled`. Driven on the debug build under
  `"reopen": "restore"` with a 400,000-deep JSON as the active tab:*
  - *killed 3.2 s into the restore: the file was still there, marked, both tabs in it;*
  - *the relaunch showed "Reopen 2 tabs in 1 window" and loaded nothing;*
  - *Reopen pressed, killed 2 s in: still marked, offered again on the next launch;*
  - *Reopen pressed and left: the mark cleared at 6.7 s, once the document was on screen;*
  - *an ordinary restore of two light documents marked and cleared within 0.1 s, and the
    launch after it restored as usual.*

## Bugs

- [x] **#9 A JSON chunk ends on source bytes only, never on how much HTML it has become**
  (added 2026-09-20, `archive/reviews/code-review-2026-09-20.md`; `json.rs`, `emit` / `finish` /
  `more`). A 900 KB deeply nested file renders to 38 MB of HTML with 77k `more` buttons.
  A stopgap put a 64 MB ceiling on one render — a refusal, and a fallback to the first chunk for
  a re-render — which bounds the damage without fixing it: the 34–38 MB cases still go to the
  webview whole. The real fix is a second way to end a chunk, and it has to let a re-render
  stop at the same place, which a budget in source bytes cannot say.
  *Measured 2026-09-20 (smoke check Z, debug build):* `"[\n"`×150,000 + `1` + `",\n]"`×150,000,
  750 KB. `load_file` alone: 3.35 s, 33.9 MB of HTML, no `more` button. Opened as a tab: the
  page never answered CDP again — `shownToken === renderToken` and `document.title` both
  unanswered across three tries, more than 140 s in all — so the scroll test never ran. The
  host process still read `Responding: True` at a 51 MB working set, which is the host alone:
  the page lives in the `msedgewebview2` children, not measured. Killed by hand. Whether it
  would ever have finished is unknown. A 2 MB file of the same shape is refused with the error
  panel and the app stays responsive, so the ceiling itself works.
  *Planned to be parked; promoted by the owner 2026-09-21 because check Z hung. A constructed
  file, not one met in the wild — but the tab it makes is unusable, and a window is one page,
  so its other tabs go with it. Next step: a design for ending a chunk on output, then a plan.*
  *Fixed 2026-09-21 — d952d02, the stopgap squashed in with it
  (`archive/plans/json-fold-weight.md`) — and "a second way to end a
  chunk" turned out to be the wrong theory. Two spikes (tables in the plan) found that size is
  cheap and depth alone is cheap: what the webview pays for is lines × the `.fold-body` spans
  open around them, about half a second and 0.7 GB a million; 300,000 lines inside 256 spans
  never lay out, and `display: contents` does not help. So `json.rs` now adds that weight up
  as it renders, and past `MAX_FOLD_WEIGHT` (2 million) makes the render again with no fold
  markup: plain brackets, everything else as ever, cut in the same places. A `more` chunk
  weighs itself from its real depth in the document (`depth_at`). Measured on the debug build
  afterwards, as load / parse / attach + layout, renderer working set, all opening as a tab in
  under a second and answering at once:*
  - *the file above (750 KB, 150,000 deep): 17.9 MB of HTML, flat — 1.7 s / 0.3 s / 1.1 s,
    1.2 GB. Before: never answered.*
  - *50,000 deep: 0.6 s / 0.1 s / 0.4 s, 0.5 GB. Before: 77 s to parse alone.*
  - *the 2 MB, 400,000-deep file the ceiling used to refuse: now rendered, 47.6 MB flat —
    4.7 s / 0.8 s / 3.2 s, 2.5 GB.*
  - *the biggest flat first chunk a real file makes (`[1],` a line, 27 MB of HTML, 105,000
    folds kept): 2.7 s / 0.3 s / 1.8 s, 2.0 GB.*
  - *three documents built to sit just under the budget — 1,100 deep; 3,000 fold controls 256
    deep; 110,000 lines 16 deep — attach in 1.0 s, 1.0 s and 1.7 s (0.8–1.6 GB), so a fold
    control weighs no more than a line and the budget means what it says.*
  - *a 300-deep file keeps all 300 folds, a 2,000-deep one shows none and loses no text,
    folding and `more` work, and a re-render after a `more` stops where it did, folds intact.*
- [x] **Text contrast across the themes.** First seen as white text on the Dracula accent
  buttons (1.37–2.41). A full audit on 2026-09-11 (every visible text element, all 15 themes,
  8 UI states) found it went wider: accent-coloured text, opacity-dimmed hints, Solarized's UI
  text, the sidebar filter's placeholder, and several document colours.
  *Fixed on 2026-09-11. The owner chose to keep the upstream palettes faithful:*
  - *App chrome, every theme: three optional tokens in `base.css` (`--ui-on-accent`,
    `--ui-accent-text`, `--ui-fg-muted`), each set per theme only where the default failed.*
  - *Invented document colours: the `-blue` heading tint (#5a86e0 → #4d73c1), secondary text
    in dracula-blue and dracula-green, comments in dracula-blue, secondary text and syntax
    colours in sakura, comments in tufte.*
  - *Solarized's UI text colour (`--ui-fg`, chrome only, not the document) also moved, because
    the upstream base00/base0 fell short: light #657b83 → #576a71, dark #839496 → #93a1a1
    (Solarized base1).*
  - *Hover and selection no longer lay a grey fill under text, since the fill cut contrast
    below AA where a colour had little headroom. The active tab takes the page's colour and a
    soft glow in the text colour over its accent underline, and a hovered tab gets a faint
    underline; hovered sidebar rows, the folder name and Open-menu items get a thin accent
    outline instead. The tab close × uses the dimmed text colour instead of 55% opacity (which was
    about 2.1:1).*
  - *The `@supports not (color-mix)` fallback for macOS 10.15's WebKit is gone as of 2026-09-12:
    it set `color` and `opacity` on a hand-maintained list of elements, which clobbered the
    opaque `--ui-fg-muted` that Sakura and Solarized define and already missed `.tab-close`.
    `minimumSystemVersion` is 13.0 now, so every supported WebKit has `color-mix()`.*
  - *Left failing on purpose, because they're upstream colours:*
    - *Solarized's document and syntax colours (light body text is 4.13).*
    - *Solarized's dimmed text is nearly as strong as its normal UI text (4.61 vs 4.63),
      because nothing fits between 4.5 and the text colour.*
    - *Dracula's #6272a4 for comments, blockquotes and footnotes (3.03).*
    - *The Visual Studio Light+ syntax colours in azure-devops and azure-devops-blue (4.17–4.41
      on the #f4f4f4 code background).*
- [x] **The debug app ignored a normal close while Settings was open.** In the 2026-09-11 smoke
  run, `taskkill //PID` (WM_CLOSE) did nothing for 5+ s with the Settings dialog open, and
  `//F` worked. The installed 1.5.9 closed normally that day with no dialog open. Neither `src/`
  nor `src-tauri/src/` has a close-request handler. Unconfirmed. To reproduce: launch, open
  Settings, then close with the window's X and with `taskkill`. If the X fails too, it's a real
  bug.
  *Not reproduced on 2026-09-11. Three later normal closes with Settings open each exited
  within 1 s. One of them replayed the original run: native dialogs, folder switches, Check now.
  Reopen this if it happens again.*
- [x] **Dependabot's cargo PR #3 fails all three CI legs** (opened 2026-09-12, seen 2026-09-13).
  The group bumps seven crates; `comrak` 0.54 → 0.55 deprecates
  `Extension::tagfilter` (removed in 0.56), and clippy's `-D warnings` turns the deprecation
  at `render.rs:18` into an error. Needs a code change on top of the bump, not a merge. The
  other six (dialog, single-instance, opener, updater plugins, `dirs` 6 → 7, `windows-sys`
  0.59 → 0.61) are untested until comrak compiles.
  *Fixed 2026-09-24 in 61f05b0 (`archive/plans/cargo-group-bump.md`): the PR's Cargo files taken
  whole, `tagfilter` dropped, and `iframes_do_not_survive` widened to all nine tags it used to
  filter (`tagfiltered_tags_do_not_survive`). `tagfilter` was a no-op here — `render.unsafe_`
  is off, so raw HTML never reaches its filter; 12 documents rendered identically with it on
  and off. After the bump clippy's only error was that line, so `windows-sys` 0.61 and `dirs`
  7 compiled as they were; `dirs` 7's one behaviour change is `preference_dir` on Windows,
  which the app does not call. Renders of every `examples/*.md`, the README, both item files
  and an autolink edge-case sample were byte-identical before and after. Smoke pass on the
  debug build: boots themed; the saved theme is read back (same config dir); kitchen-sink
  renders; 18 recovered anchors, and `#f14` scrolls 0 → 6465; a task tick writes back; Check
  now says "You are up to date."; a tab dragged into another window's strip lands there
  (`w1` 2 tabs, `main` 1). PR #3 closes by itself once `main` is pushed. comrak 0.55 also
  fixes two autolink DoS issues (GHSA-xg9p-p4jc-c46g), and `autolink` is on. The updater's
  2.11 changes (a Windows installer spawn failure now errors) first run on the update after
  the one that installs this build.*
- [x] **A restored scroll position degraded once, unreproduced** (added 2026-09-17, 1.6.3).
  During the 1.6.3 drive run a tab recorded `scrollY: 1500`, and some minutes later the same
  entry read 500 with nothing having touched that window. Two later runs round-tripped 3000
  exactly, cold restore included, so it is not the obvious suspect (a render clamping the
  offset while the document is still short, which the scroll listener would then write back).
  No theory. Reopen with a reproduction; the cost if real is landing in the wrong place once,
  after which the wrong value sticks.
  *2026-09-20: #4 of the review is a mechanism that fits — `rememberScroll` banked the outgoing
  page's offset onto the incoming entry when a second tab switch landed before the first had
  loaded. Fixed in 0cbadf1. Leave open until a release has gone by without a recurrence.*
  *Closed 2026-09-25: two releases have gone by (1.6.7, 1.6.8), and the owner's in-app update
  brought tabs, windows and scroll positions back as they were. No recurrence seen. Reopen with
  a reproduction.*
- [x] **`toggle_task` can NUL-pad the file if an editor truncates it between the read and the
  seek** (an accepted limit from the 2026-09-12 review). The pre-fix behaviour
  (truncate-then-write) was worse.
  *Lifted 2026-09-25, at the owner's call: the only limit that could damage a document.
  `write_box` now reads the three bytes around the offset through the handle that writes, and
  writes only if they are still `[ ]`, `[x]` or `[X]`; otherwise the reader sees "changed on
  disk since it was read; try again". What is left is the instant between that check and the
  write. The new test was red first on the old logic — it wrote into a shortened file — and
  also covers a file rewritten so the offset holds other text.*
- [x] **`check_for_update` sends one request per window when several boot together; the
  negative cache only helps windows that open later** (an accepted limit from the 2026-09-12
  review).
  *Lifted 2026-09-25, at the owner's call — more common since 1.6.3, when every restored
  multi-window session started booting its windows together. The check now holds an async
  lock (`AppState.update_check`): the first window asks, the rest wait and read its cached
  answer. Counted with a temporary log on the debug build, booting a 3-window session: 3
  requests before, 1 after.*
- [x] **Two concurrent config saves are each atomic, but the pair is last-writer-wins** (an
  accepted limit from the 2026-09-12 review).
  *Retired 2026-09-25: it cannot happen inside the app. Every config writer is a synchronous
  command, and Tauri runs those on the main thread one at a time. Measured with a temporary
  probe holding each save open 300 ms, while two windows fired all five setters at once: every
  save on `ThreadId(1)` ("main"), never two at a time. `write_json`'s comment, which said two
  windows could be inside it at once, now gives the real reason for its unique temp name — an
  update snapshot writing from the install task beside the main thread's saves.*
- [x] **A link with a leading slash is resolved against the document's folder** (an accepted
  limit from the 2026-09-20 review, #8).
  *Lifted 2026-09-25 (320dcaa): `/docs/guide.md` and `/assets/logo.png` are taken from the root
  of the git repository the document sits in, as GitHub does — the nearest folder above holding
  `.git`, a worktree's `.git` file included, but never the home folder or a drive root. The
  asset scope grows to that root so such a picture loads. Outside a repository nothing changed.
  Driven on the debug build: a page two folders down in a scratch repository opened
  `/docs/guide.md` in place and showed `/assets/pic.png` (`naturalWidth` 128); the same page in
  a folder with no repository kept `repo: null` and resolved under its own folder.*
- [x] **Closing a window inside the 500 ms report wait loses the last of it** (an accepted limit
  from session restore, 1.6.3).
  *Lifted 2026-09-25 (21384f3) for a window closed by its X button, the macOS menu, or a tab
  dragged out of it: the page holds the close until its last report is sent, and Rust destroys
  the window anyway after 2 s (`CLOSE_WAIT`) if the page never answers. Measured on the debug
  build, scrolling to 3000 and posting `WM_CLOSE` to the window 50 ms later: `session.json`
  said `scrollY` 0 without the listener and 3000 with it, closing in about 110 ms. A page made to
  spin for 8 s still closed after 2073 ms. Dragging a window's only tab into another window
  still closed the empty window. A Cmd+Q, a kill or a shutdown still lose the wait; that
  remainder stays under Accepted limits.*
- [x] **#18, the superseded case: an `openFolder` overtaken by a newer one keeps its
  unverified pick** (an accepted limit from the 2026-09-20 review; the closed-mid-listing case
  was fixed in 74f2192).
  *Lifted 2026-09-25 (dd05778), at the owner's call. Two ways in: a tab switch overtakes a pick
  whose listing then fails, and the tab keeps the dead folder; or a second pick overtakes the
  first and fails, and falls back to the first — never verified — instead of the folder that
  worked. `listTree` now says whether its own listing failed even when a newer one has the
  tree, so an overtaken call puts its tab back, and each tab's folder from before the calls
  still out (`folderBefore`) is what a failed pick falls back to. Driven on the debug build with
  an unreachable share (fails after 21 s): both cases left the tab on the dead share before
  the fix and on the repo folder after; closing mid-listing still reverts with the filter
  blank, and a plain pick still lands canonical.*
- [x] **#23: a file-association open in the instant a window is closing is dropped** (an
  accepted limit from the 2026-09-20 review).
  *Lifted 2026-09-25 (d08e9d6), at the owner's call. The review's fix — drop the label from
  `focus_order` at `CloseRequested` — was turned down because `session::save` orders windows by
  it; `AppState.closing` holds the label instead, from `CloseRequested` to `Destroyed`, and
  `last_focused` skips it. The gap had not grown with 21384f3 (close request to destroyed:
  13.2 ms before, 12.0 ms after), but a page too busy to answer now holds it for `CLOSE_WAIT`,
  which made it reproducible: one window, its page spinning, `WM_CLOSE`, then a second launch
  naming a file 300 ms later. Before the fix the file went to the dying window and the app
  exited with it; after, it opened in a new window and the app stayed up. A plain second
  launch still lands as a tab in the open window.*

- [x] **A second instance aborts on a file name that is not Unicode** (added 2026-09-26,
  found by the rpm check). With the app already running, launching it on `caf\xe9.md`
  (terminal or `gio launch`) aborts the new process — exit 134, `panicked at
  std/src/env.rs:878`. `tauri-plugin-single-instance` 2.4.5 forwards argv with
  `std::env::args()` (`platform_impl/linux.rs:77`), which panics on non-UTF-8; the release
  profile is `panic = "abort"`. Our own `args_os()` fix (#6) covers only the first instance.
  The running window is untouched and the file would not have opened anyway (#25); what the
  user may see is a crash reporter (ABRT, apport). A UTF-8 name as second instance works.
  Windows (`windows.rs:91`, an unpaired surrogate) and macOS (`macos.rs:88`) share the call.
  Options: (A) in `main`, before the builder, on Unix re-exec the app with the arguments
  converted lossily (~10 lines), so the plugin never sees a non-UTF-8 argument; (B) report
  it upstream and wait; (C) accept it. Reproduce: two instances in WSL, second on
  `$'caf\xe9.md'`.
  *Fixed 2026-09-26, 009d8c8 (option A, `docs/archive/plans/second-instance-non-unicode.md`):
  on Linux `main` first re-executes itself in place (`exec`, same PID) with every argument made
  Unicode lossily, so the plugin never sees a non-UTF-8 one. Three unit tests (Linux only).
  Proved in WSL Ubuntu on a debug build, each launch with its own config and data folders:
  before, the second instance panicked (`env.rs:878`, exit 101 under debug's unwind); after, a
  first instance on `caf\xe9.md` kept its PID with `caf\xef\xbf\xbd.md` in
  `/proc/<pid>/cmdline` and opened a window; a second on it exited 0 with the first window
  untouched; a second on `second.md` opened it. The AppImage's own runtime is the one path not
  run; it is under *First runs* in `open-items.md`. Remove the workaround once
  `tauri-plugin-single-instance` reads `args_os()` — check on each bump.*

- [x] **A diagram's `themeVariables`/`themeCSS` could style the whole app, not just its own
  drawing** (found in this plan's review; live since Mermaid diagrams shipped, 1.7.0). mermaid
  scopes a diagram's styles itself, wrapping them in `#<id>{…}` through stylis — but stylis
  reads a quoted string across newlines and the browser ends it at the first `"`, so a lone `"`
  in any `themeVariables` value (or `fontFamily`) turned the rest of mermaid's stylesheet into
  top-level rules. Measured: `%%{init: {"themeVariables": {"lineColor": "red\""},
  "themeCSS": "body { background: red } #content { display: none }"}}%%` on a plain flowchart
  turned the whole app red and hid `#content` (45 unscoped rules).
  *Fixed c241696, 2026-09-27 (`archive/plans/mermaid-pipeline.md`): every drawing's `<style>` is
  parsed the way the page will parse it (a `<style media="not all">` probe) and the diagram is
  refused unless every rule is scoped under its own root id; `@keyframes` stay exempt (the app
  defines none). Verified with the payload above plus a `fontSize` variant and a front-matter
  form, each refused with "This diagram's styles reach outside it, so it isn't drawn." and the
  page unchanged; a balanced `themeCSS` still draws. A first pass missed escaped and quoted
  brackets in a selector (`.a\(`, `[title="("]`), fixed in the same commit's fixup round: 7
  direct-call cases and 105 section-checks across three full passes (before the check, after it,
  after the bracket fix) all held clean, with no diagram newly refused.*

## Deferred from the 2026-09-20 review

- [x] **Unchecked, from #2: can a file that kills the app bring it down again on every
  launch?** (added 2026-09-20; `session.rs` restore, `main.rs` `setup`). The review never
  traced whether `session.json` still names a document whose load aborted the process — if it
  does, `"reopen": "restore"` reopens it at startup and the app cannot be started without
  deleting the file by hand. d952d02 removes the one known way a document aborts the process
  (the unbounded reflow), so nothing is known to trigger this today; the question is whether
  the next such file would. A question, not a confirmed defect: the first step is to answer it
  — open a document, kill the process mid-session, relaunch with `"reopen": "restore"`, and
  read what comes back and what `session.json` held. One data point from the smoke pass:
  after the check Z kill, run with `"reopen": "off"`, `session.json` was not there at all.
  *Reopen when:* any document is found to crash or abort the app on load — #9's hang is the
  nearest thing so far, a hang rather than an abort — or session restore is next worked on.
  *Answered 2026-09-21: no, under `"reopen": "restore"`, measured on a debug build with a
  JSON file whose load takes 4.7 s and whose layout takes 3 more.*
  - *A tab is written to the session only after its document is laid out: `session.json`
    first named the file 9.8 s after `openTab`, the page having answered again at 9.0 s.
    Killed 1.5 s into the load, the file was never named and did not come back.*
  - *Killed once shown, it came back as the active tab — the restore doing its job.*
  - *Killed 2.5 s into that restore's load: `session.json` was already gone (`setup` discards
    it before restoring) and was not rewritten. The launch after that was the empty screen.
    So a document that kills or hangs the app on load costs one bad launch, not every one.*
  - *The price was that launch losing the whole session — its own item under* Needs a
    decision *above, fixed 2026-09-24. `"ask"` was not run; by the code the file stays, and
    so does the choice not to press Reopen.*
- [x] **#28 Anchor recovery misreads an `id=` inside another attribute's value** (added
  2026-09-20; `render.rs`, `attribute`). `<a href="p?a id=q" id="x"></a>` recovers no anchor,
  so `[…](#x)` links to nothing; every ordinary shape works. The fix is a scanner that skips
  quoted values. *Reopen when:* a document in the wild loses an anchor to it, or `attribute` is
  touched for any other reason.
  *Fixed 2026-09-24 in ff2eef6 (`archive/plans/anchor-attribute-scanner.md`), at the owner's
  call rather than a trigger. `attribute` now reads a tag as HTML does — a name, then an
  optional quoted or bare value skipped whole — and the first attribute of that name wins. It
  was worse than recorded: `<a title="the id=3 entry" id="x">` gave the wrong anchor, `#3`,
  not just none. Two deliberate changes ride along: a bare `id=a/b` is refused rather than cut
  to `a`, and an empty `id` falls back to `name`, as a browser does. Three tests, all red
  before; renders of every `examples/*.md` (the defect-96194 file's 18 raw anchors among them)
  and the README were byte-identical before and after.*

## Deferred from the 2026-09-25 review

- [x] **#37: the folder tree, menus and tab strip carry ARIA roles and a roving tabindex but no
  keyboard handling** (`docs/archive/reviews/code-review-2026-09-25.md`). No arrows/Enter/Space in the
  tree, no tab stop when no row is selected; arrow keys dismiss the menus; Enter/Delete do
  nothing on a focused tab. Reopen when keyboard or screen-reader use is asked for.
  *Done 2026-09-26, 70e935c: tree ↑/↓/←/→/Enter/Space and one tab stop, menus ↑/↓ and Esc/Tab
  back to their button, tabs ←/→/Enter/Space/Delete, and focus rings. Scope B (Home/End,
  type-ahead, `*`) not included.*
- [x] **#43: `write_json` renames without flushing**, so after a power loss NTFS can keep the
  rename but not the bytes, and `config.json` loads as defaults. Standard hazard, not
  reproduced. Reopen on a report of a session or settings lost after a crash or power cut.
  *Done 2026-09-26, folded into 7060f25: `write_json` now flushes before the rename.*
- [x] **#44: `core:default` grants more than the page uses** — tray, menus, and
  `image:allow-from-path`; `core:event:default` is listed twice. Matters only if script ever
  got into the page, which the CSP and `unsafe_` off prevent. Reopen with the next capability
  change.
  *Done 2026-09-26, a049765: the capability was trimmed to the 12 permissions the page
  actually uses.*
- [x] **#46: an action bump on the publish path first runs on a real tag.** Dependabot PRs only
  run `checks.yml`, and a dispatch skips `publish`. Reopen when a Dependabot bump touches
  `softprops/action-gh-release` or `download-artifact`.
  *Done in code 2026-09-26, 08606a8: a dry run now rehearses publish too — it drafts a release
  `dry-run-<run id>`, checks its assets against `dist/`, then deletes it. First real run is the
  pre-release dry run, tracked in `open-items.md`.*
- [x] **#47: each `more` click re-reflows a one-line JSON file, up to 8× its size, per click.**
  Memory, not just time. Reopen on an out-of-memory report; N concurrent clicks hold N copies
  at once.
  *Done 2026-09-26, folded into e904196: `json_region` calls are now serialized, so quick
  "more" clicks don't each hold a reflowed copy at once.*
- [x] **#48: window routing and session saving have no tests** — only their pure helpers do,
  because they need an `AppHandle`. Mock-runtime tests for window routing and session saving —
  do next, after the 2026-09-25 fixes.
  *Done 2026-09-26, a049765: an 18-test mock-runtime harness for routing and session saving,
  with a per-thread temp data folder under test. Not reachable under the mock: adoption via
  the Win32 hit-test, frame placement, and more than one spawned window per test.*
- [x] **The sidebar stays closed after a reload or a session restore, although each tab still
  holds its folder** — `followTab` returns while `state.folder === null`, and boot has no path
  that reopens it. Predates the 2026-09-25 fixes.
  *Done 2026-09-26, folded into 7060f25: the session now records whether each window's sidebar
  was open, and a restore or reload reopens it.*
- [x] **Task 7's watch-call skip** — checked live 2026-09-26, no change: works as intended.
- [x] **Window labels swapping after a restore** — checked 2026-09-26, no change: by design.
  Windows are saved least-recently-focused first and the first goes to `main`; labels
  themselves are not saved.

- [x] **Task 15's release-pipeline hardening is done in code (08606a8) but not yet run.** Owner
  steps, in order: (1) ~~add the four signing secrets to the `signing` environment~~ done
  2026-09-25, repo copies left in place until step 5; (2) clear the
  `v0-rust-build` caches; (3) push; (4) a `workflow_dispatch` dry run on `main` and check it;
  (5) delete the four repo secrets and dry-run again; (6) turn on "require actions pinned to a
  full-length commit SHA". With the dry run, also: extract its AppImage (`--appimage-extract`)
  and check `usr/lib` against THIRD-PARTY-LICENSES' "Linux AppImage" section, `libjbig`
  included; and check that the dry run also rehearses publish (#46) — the draft release
  `dry-run-<run id>` was created, its assets matched `dist/`, and it was deleted.
  *Steps 2–4 done 2026-09-26: caches cleared, 15a4a2a pushed, dry run 36186470162 green on
  every job. Its log: six AppImage tools pinned `OK`; `tauri-cli v2.11.4` installed from
  crates.io on all three legs; no download in the Linux Bundle and sign; updater signatures on
  all three; Windows setup, app and uninstaller signed by `F06C…8151`, macOS bundle, tarball and
  dmg by `53effb03…`; the draft took all 16 `dist/` files, the check passed and deleted it, and
  no `dry-run-*` release or tag is left. The AppImage (WSL): runtime `dd6cebe`, extracts, 164
  libraries in `usr/lib` with `libjbig.so.0` among them, `libjbig0` the only package whose
  copyright lists GPL alone, and the extracted `AppRun` opens a `.md`.*
  *Step 5 done 2026-09-26: the four repo secrets deleted (the environment keeps all six), and
  dry run 36217793093 passed every job on the environment alone: updater signatures on all
  three legs, Windows signed by `F06C…8151`, macOS by `53effb03…`, and the publish rehearsal
  drafted, checked and deleted `dry-run-36217793093`.*
  *Step 6 done 2026-09-26 by the owner: `actions/permissions` reports
  `sha_pinning_required: true`. Closed.*

- [x] **Anyone who can push to main can sign with the project's keys** (added 2026-09-26, from
  the step-5 dry run). The `signing` environment limited which refs could use the updater
  key, the macOS certificate and the Certum OTP seed, not who pushed to them.
  *Done 2026-09-26 at the owner's call rather than accepted: the environment now requires the
  owner's approval (self-review allowed, as the only reviewer), so every Release run, tag or
  dispatch, waits at the build legs until approved. The release skill says how.*

## Deferred

- [x] **A heading's id can collide with the app's own element ids** (added 2026-09-26, found
  during the Mermaid work). comrak slugs headings into plain ids, and `base.css` styles the
  app's chrome by id: `## Image` became `<h2 id="image">`, picked up the image panel's
  `#image { height: calc(100dvh - …) }`, and rendered a screen tall — every build since
  0ab46fd (2026-08-24). An in-page `#tree` link was left to the browser, which went to the
  first element of that name: the sidebar's.
  *Fixed cfd1e00, 2026-09-26 (`archive/plans/heading-id-collision.md`): headings and `<a id>`
  targets render behind `user-content-`, as on GitHub; `docTarget` adds the prefix at the
  jump, and looks a bare name (footnotes, GitHub-style links) up inside the document only. A
  dead link does nothing; `#top` and `#` still scroll to the top. A test fails if an app id
  ever starts with `user-content-`, `fn-` or `fnref-`.*
- [x] **Settings' Done button sits below the fold on a short window** (added 2026-09-26,
  review loop; older than it). At an 860 px window height the dialog scrolls and Done is out
  of sight; Escape still closes it. Fix: keep the dialog's footer in view (sticky footer, or
  a capped scrolling body). Reopen with the next change to Settings.
  *Fixed ce940ff, 2026-09-27 (`archive/plans/cheap-deferred-fixes.md`): `#settings-actions` is
  now sticky at the bottom of the dialog's own scroll box, and `scroll-padding-bottom` keeps
  keyboard focus clear of it. Measured on the debug build: at 860 px Done's bottom went from
  960 (below the dialog's 804) to 787 (in view); at the app's real minimum, 420×320 by
  `drag-corner.ps1`, 286 is in view of 303. Layout at 1080 is unchanged (dialog height 960),
  and a Tab walk through Settings never lands a control under the footer.*
- [x] **A Gantt `click … href` link does nothing** (added 2026-09-27, Plan A.1 review). Gantt
  wires a task's `click` line through `bindFunctions`, called on the SVG mermaid draws into the
  page — and the app has never called it, since diagrams first drew. It could not now anyway: a
  diagram draws inside its own frame and reaches the page only as an SVG string. Other types'
  links (`<a xlink:href>`, which mermaid writes straight into the SVG) still work.
  *Fixed 910546d, 2026-09-27 (`archive/plans/mermaid-follow-ups.md`): the frame sends back
  mermaid's own sanitised `{taskId → url}` map alongside the drawing, and the page wraps each
  linked task's bar and label in an SVG `<a xlink:href>` for every URL except `about:blank`
  (mermaid's mark for one it did not trust) — the app's existing link handler decides the rest,
  as for a flowchart link. A task mermaid marked `clickable` but left unlinked loses the class,
  so it no longer looks like a link.*
- [x] **An OS light/dark flip doesn't redraw diagrams under a see-through theme** (added
  2026-09-26, review loop). `diagramLook` reads the page background composited on the
  dialog's `Canvas`, which follows the OS when a theme sets `color-scheme: light dark` and
  leaves its body transparent. Only a theme change triggers a redraw, so such a theme's
  diagrams keep the old palette until the next open. No bundled theme is built that way.
  Fix: a `matchMedia("(prefers-color-scheme: dark)")` change listener that calls
  `redrawDiagrams()`. Reopen when a custom theme reports it.
  *Fixed e0dd026, 2026-09-27 (`archive/plans/cheap-deferred-fixes.md`): a
  `matchMedia("(prefers-color-scheme: dark)")` change listener in `main()` calls
  `redrawDiagrams()` and refreshes `--page-bg`, once a theme has set it. The listener also
  covers a theme that restyles its body under its own `prefers-color-scheme` rule, not only a
  see-through page. Verified with CDP-emulated light/dark flips under both shapes: diagrams
  redraw to the page's side, `--page-bg` follows, an open overlay closes, nothing happens on a
  picture tab or the empty screen, and an opaque theme that ignores the OS is untouched. The
  overlay closes at the switch itself, and a diagram clicked in a look the page no longer has
  is brought up to date instead of opening in the old one; the next click opens it.*
- [x] **Diagrams in the theme's own colours** (added 2026-09-26, `archive/plans/mermaid-diagrams.md`).
  Diagrams take mermaid's `default` or `dark` palette by the theme's side, and the theme's body
  font; a Dracula or Solarized document gets the same diagram colours as any other dark or light
  one. Fix: mermaid's `base` theme, with `themeVariables` read from the theme's computed styles
  (page background, text, code background, link colour), then a contrast check by eye across
  all bundled themes. Reopen when a bundled theme's diagrams clash visibly, or someone asks.
  *Fixed efc2b50, 2026-09-27 (`archive/plans/mermaid-pipeline.md`): a Settings choice, "Follow
  the theme" (default) or "Mermaid's own", in Settings → Theme, shared by every window. Follow
  the theme reads a palette from the page background, prose text, code background and link
  colour (`diagramLook`), rotates category colours from the link colour by hue, and locks a
  diagram's own `theme`/`themeVariables`/`darkMode`/`fontFamily`/`themeCSS` while its
  `classDef`/`style` lines and a type's own colour options still work, as chosen by the author.
  Verified across all 15 bundled themes: every palette colour a valid hex, `initialize` throws
  on none of them, and category colours hold their 4.5:1 (or 3:1) target — lowest measured on
  Solarized light (3.066 against text) and Solarized dark (3.006), the two themes whose own text
  is already under 5:1 on their page. Radar's nested rings are drawn at 0.06 opacity rather than
  mermaid's 0.3 (an owner ruling in the fix round, b6b2798): mermaid's five nested, filled rings
  at 0.3 stack into a solid band on both sides of the curve.*
- [x] **Some Mermaid ids are the document's own names, not namespaced** (added 2026-09-26,
  third review of the Mermaid work). Swimlane clusters and state groups take the name written
  in the diagram as their id (`attr("id", t.id)` in the bundle). So a state named `icons` wears
  the app's `#icons { display: none }` and vanishes, and `[x](#tree)` with no such heading but
  a state named `tree` finds it in `#content`, sets `location.hash = "tree"`, and the browser
  scrolls to the sidebar's `#tree` instead. Needs a group named exactly like an app id. Fix:
  at warm time, rename every id in a diagram's SVG under the diagram's own id, and rewrite
  its `url(#…)` and `href="#…"` references to match (the overlay's rewrite is the model).
  Reopen when a real diagram hits it.
  *Fixed d730f11, 2026-09-27 (`archive/plans/mermaid-pipeline.md`): every id inside a diagram's
  SVG is renamed under its own root id at draw time (`renameIds`), and every `url(#…)`,
  `href="#…"` (except a `click` link, which names a document target, not part of the drawing),
  aria id list, and `<style>` selector or `url(#…)` referencing it is rewritten to match.
  Verified on `all-types.md`'s three swimlane sections: their `icons` groups, hidden before by
  the app's own `#icons { display: none }`, now show inside the drawing; a flowchart's
  `click A href "#heading-b"` still jumps to the heading, and `[to tree](#tree)` against a
  diagram group named `tree` does nothing.*
- [x] **The diagram cache empties itself past 200** (added 2026-09-26, third review; the
  `ponytail:` note at `DIAGRAMS_KEPT`). A document with more than 200 distinct diagrams
  redraws all of them on every open, save and return, and documents in one window that
  together pass 200 evict each other's. Fix: keep the most recently used instead of clearing
  (about five lines on the `Map`'s insertion order). Reopen when such a document exists.
  *Fixed 404b7f9, 2026-09-27 (`archive/plans/mermaid-pipeline.md`): the cache keeps the 200 most
  recently used instead of clearing wholesale past the limit — a `Map` in least-recently-used
  order, a document's own drawings always moved to the newest end before the oldest elsewhere
  are evicted. Verified: 205 stale entries plus a one-diagram document trims to exactly 200,
  keeping the newest; a fully cached document revisited is marked used again; a document with
  more diagrams than the limit keeps every one of them while the oldest elsewhere are dropped.*
- [x] **The overlay's id rewrite can change a label's text** (added 2026-09-26, review loop).
  `openDiagram` rewrites the diagram's own id in the SVG string, so a label whose text contains
  `#Mermaid-N` or `"Mermaid-N` for that diagram's id reads differently in the overlay (the
  page copy is untouched). Fix: rewrite ids on the parsed SVG (attributes and its `<style>`)
  rather than the string; the unnamespaced-ids item above wants the same routine. Reopen with
  that item, or when a real label hits it.
  *Fixed d730f11, 2026-09-27 (`archive/plans/mermaid-pipeline.md`): the overlay now parses its
  copy into an SVG element and reuses `renameIds` — the routine the unnamespaced-ids fix above
  added — instead of a string `replaceAll`, renaming every id under a `-zoom` suffix and
  rewriting every reference along with it. Verified: a label naming the diagram's own id reads
  the same in the overlay as on the page, every id in the copy starts with the `-zoom` root, and
  arrowheads still resolve.*
- [x] **The foreignObject check runs after mermaid's temporary in-page render** (added
  2026-09-26, review loop). `mermaid.render` draws into a temporary element in the document
  before handing back the SVG string the backstop checks, so a drawing that is then refused
  had its filtered markup live for that moment. It matters only for an HTML-label route not
  yet known: the known ones are closed (math refused, HTML labels off) or accepted (event
  modeling). Fix: render in an isolated document, such as a sandboxed iframe. Reopen when a
  new HTML-label route is found, or on a mermaid major bump.
  *Fixed 7a63106, 2026-09-27 (`archive/plans/mermaid-pipeline.md`): mermaid now renders inside a
  sandboxed, origin-less `iframe` (`sandbox="allow-scripts"`, reached over `postMessage` through
  the new `src/diagram-frame.js`) rather than in a temporary element of the app's own document,
  so a drawing later refused — by this backstop, or the style check above — was never live in
  the page. Verified: `window.mermaid` is `undefined`, no `script[src*="mermaid"]` in the page,
  the frame's `contentDocument` is `null`, and a `MutationObserver` on the document sees no
  mutation while a diagram renders. A renderer that fails to load leaves blocks as code within
  about a second (or ~1.2 s for the bridge script specifically) and is rebuilt for the next
  document. Cost: about +15 ms on a window's first diagram document, not the ~200 ms estimated;
  later opens with new diagrams measured 13–18% faster than before the frame, since it has no
  app stylesheet to recalculate against.*
- [x] **The diagram overlay duplicates the image viewer's zoom and pan** (added 2026-09-26,
  third review). About 70 lines of `openDiagram`/`zoomDiagram`/`onDiagram*` mirrored
  `zoomTo`/`onImage*`, differing in the element and the state object, so a fix to one could
  miss the other.
  *Fixed 1658765, 2026-09-28 (`archive/plans/viewer-follow-ups.md`): both share one core now,
  `zoomAbout` and `dragPan`, parameterised on the view, the element and the state; wheel,
  double-click, toolbar and keys stay per viewer. Before/after measurements matched 290 of 290.*
- [x] **`docTarget`'s lookup order can differ from GitHub's** (added 2026-09-26, third
  review). It tried `user-content-<name>` across the whole page, then the bare name, then the
  name without its prefix, only those two inside the document, so `#user-content-x` could land
  on the wrong heading and a
  footnote link could land on a heading of the same name.
  *Fixed b5bb12b, 2026-09-28 (`archive/plans/viewer-follow-ups.md`): a name already carrying
  `user-content-` is looked up as written first, GitHub's own order, and a `fn-`/`fnref-` name
  tries the bare footnote in the document first.*
- [x] **Link clicks on a page are dropped while its refresh draws diagrams** (added
  2026-09-26, review loop). `onLinkClick` ignored every link click while
  `shownToken !== renderToken`, so during a save or a switch in flight the links on the page
  on screen went dead.
  *Fixed de4aebb, 2026-09-28 (`archive/plans/viewer-follow-ups.md`): a click now acts on the
  page on screen, and one that navigates outdates the pending render; a switch to another tab
  or document still leaves its links dead, as before.*
- [x] **A diagram or a picture opens only with a mouse** (added 2026-09-26, review of the
  Mermaid work). Neither `.mermaid-diagram` nor `img[data-file]` was focusable, so neither
  could be reached or opened from the keyboard.
  *Fixed 1b655d5, 2026-09-28 (`archive/plans/viewer-follow-ups.md`): both take `tabindex="0"`
  and a visible focus ring, and Enter or Space open them as a click would; closing the overlay
  or a save returns focus to the same one. Keeping focus on a link inside a diagram across a
  light/dark redraw, and keyboard access to sequence actor menus, stay open — see
  `docs/open-items.md`.*

## Needs a Mac, a Dependabot run, or an older build

- [x] **"Update available" in Settings (586560c).** Only the up-to-date path has been seen
  ("You are up to date." on 1.5.9). The other path needs an older build: install 1.5.8 under a
  spare Windows account (it installs per user), then Settings → Check now. Expect "Update…"
  beside the status, and the bar's Update button.
  *Checked on 2026-09-11 with a debug build temporarily reporting 1.5.8, so nothing was
  installed:*
  - *The launch check showed the bar's Update button.*
  - *Settings → Check now said "Version 1.5.9 is available." and showed "Update to 1.5.9…".*
  - *That button closed Settings and opened the update dialog: summary, release-notes link,
    restart warning, Update now. Later closed it.*
  - *Installing was not exercised.*
- [x] **Not driven in the 2026-09-20 smoke pass** (added 2026-09-21). These can be checked
  on this machine with the drive-app skill; the pass did not get to them.
  *All three driven 2026-09-21, `cdp.mjs` having learned `drag` and `key`:*
  - *#12: a, b, c dragged into b, c, a; `session.json` held b, c, a within 1.5 s with nothing
    else touched, and a kill and relaunch brought that order back.*
  - *Tear-off: a tab dragged 250 px down became window `w1` on its own; `main` kept the other
    two in order.*
  - *#16: at 3200 % each arrow press moved the picture 40 px on its own axis. Tab reaches
    `#image-view` (tenth stop). A click pans too, though it does not focus the panel — it
    does with `onImagePointerDown` taken off — since Chromium scrolls the clicked box
    anyway.*
  - *#12, tabs keep the order they were dragged into (0cbadf1):* drag a tab, then read the
    order `session.json` holds. Under `"reopen": "off"` that file has not been there to
    read, so this likely needs `"reopen": "restore"` — back the session up first. The
    skill's cross-window recipe covers the mouse events.
  - *Tab tear-off still works after 0cbadf1's `reportSoon()` in `onDragEnd`.*
  - *#16, arrow keys pan a zoomed picture (a7fc23e):* needs `Input.dispatchKeyEvent`, which
    `cdp.mjs` does not send yet.
- [x] **Dependabot `ci:` prefix (2bace27).** The release-notes filter (`release.yml`, "Build the
  release notes") drops `ci: Bump …`, checked locally; a plain `Bump …` still gets through.
  Confirm the next weekly Dependabot PR is titled `ci: …`.
  *Not yet seen. PR #2 ("Bump softprops/action-gh-release", 2026-09-10) predates 2bace27, so
  it's no evidence either way. The cargo entry's prefix does work: PR #3 (2026-09-12) is titled
  `chore: bump the cargo group …`. Wait for the next github-actions bump.*
  *Proven 2026-09-25: PR #5 (2026-09-24), the first github-actions bump since 2bace27, is
  titled `ci: Bump dtolnay/rust-toolchain …`.*
- [x] **An update restart still restores the session** (added 2026-09-17, 1.6.3). `snapshot`
  claims `session.json` for the restart and sets `AppState.installing`, which stands ordinary
  saves down until the install lands or fails, and the `restart` flag makes the session come
  back whatever **Reopening** says. All of it is verified by reading the code and by the
  `session.rs` tests; none of it has survived a real install, which needs a published release
  to update from. The next in-app update is the proof: tabs, windows and scroll positions
  should all return, even with **Start fresh** selected.
  *Proven 2026-09-25 by the owner's in-app update to 1.6.7 / 1.6.8: tabs, windows and scroll
  positions all came back, through the new restore mark (`040a866`) as well. Under
  **Restore**, not **Start fresh**: the `restart` flag is what overrides the setting, and it is
  the same flag either way.*
- [x] **2026-09-20 review fixes that nothing here could run** (added 2026-09-21). Each passed
  the gates and a code read; none has met the thing it fixes.
  - *#5, a second update refused (769cd54).* The `INSTALLING` guard and its `update-failed`
    broadcast only run during a real install. Only the page's own `installing` flag was
    checked, by eval. The next in-app update proves the ordinary path still installs; the
    refusal itself stays read-only unless Update now is pressed twice on that run.
  - *#6, a file name that is not Unicode (0d08e71).* Needs a Linux file manager handing over
    Latin-1 bytes; Windows cannot pass them and a test cannot inject argv. Proof: on a Linux
    install, open `$'caf\xe9.md'` from the file manager and the app starts.
  - *#27, a theme listed once (0b1c86c).* The test means something only where names ignore
    case. Windows passed locally; the macOS leg first runs it on the next push.
  - *#30 and #31, CI (47e8c0d).* The pinned `checkout` first runs on the next push; the
    pinned upload/download-artifact and the awk `lock=` line on the next release tag. The awk
    line was run once by hand against the real lock file and printed 1.6.5.
  - *#27, #30 and #31 proven 2026-09-21 by 1.6.6: the push's CI run passed on all three legs,
    macOS included; a `workflow_dispatch` packaging build passed before the tag; the v1.6.6
    Release run passed `version`, the checks, all three builds and `publish`. #5 and #6 are
    what keeps this open — 1.6.5 → 1.6.6 is the first in-app update to run #5's guard.*
  - *#5 proven as far as it can be 2026-09-25: the owner's in-app update to 1.6.7 / 1.6.8
    installed and restarted through the guard. The refusal itself stays read-only — nobody
    pressed Update now twice. #6 alone keeps this open.*
  *Retired 2026-09-25: four of five proven. #6 continues as its own item in `open-items.md`,
  "The app starts on a file name that is not Unicode".*
- [x] **The app starts on a file name that is not Unicode** (added 2026-09-25, split from the
  item above; review #6, 0d08e71). `setup` read argv with `std::env::args()`, which panics on
  a non-UTF-8 argument; it now reads `args_os()` lossily. The file itself does not open; that
  is #25, under *Accepted limits*.
  *Proven 2026-09-26 on the .deb from dry run 36186470162, in WSL Ubuntu 24.04 (WSLg): with
  `caf\xe9.md` on disk, both `t4-markdown-viewer <path>` and `gio launch` of the installed
  desktop file (`Exec=… %f`, the path a file manager takes) left the app running with no
  panic. That is the first instance; a *second* instance on such a name aborted in the
  single-instance plugin — a separate bug, under *Bugs*, fixed in 009d8c8.*

- [x] **The rpm has never been installed** (added 2026-09-26, owner kept it open rather than
  accept it). The .deb was checked in WSL Ubuntu on dry run 36186470162: MIME registration,
  `desktop-file-validate`, and a start on a non-Unicode file name. The rpm carries the same
  desktop and MIME files, but nothing has installed it: dependency names, the install
  scriptlets and the MIME cache refresh differ on Fedora. Proof: on a Fedora install (a WSL
  Fedora distro will do), `dnf install` the rpm from a dry run or release, then repeat the
  .deb checks — `xdg-mime query filetype x.jsonc` gives `application/json`, `gio mime
  application/json` lists the app, and `caf\xe9.md` starts it. Reopen sooner on an rpm bug
  report.
  *Proven 2026-09-26 in a throwaway WSL Fedora 44 (since unregistered), on dry run
  36186470162's rpm: `dnf install` resolved its two requires (`libwebkit2gtk-4.1.so.0`,
  `libgtk-3.so.0`) and the post-install scriptlet rebuilt the MIME and desktop caches;
  `desktop-file-validate` passes; `xdg-mime query filetype x.jsonc` gives `application/json`;
  `gio mime` lists the app as default for `application/json` and `text/markdown`; a `.md`
  opens; the app starts on `caf\xe9.md`. It also found the second-instance abort, now under
  *Bugs*.*

- [x] **`.json` / `.jsonc` file registration** (added 2026-09-12, see
  `archive/plans/json-viewer.md`). Only an installed build proves it: on Windows, Explorer's
  *Open with* on a `.json` lists the viewer and the Type column reads "JSON Document" (its own
  ProgID, `T4MarkdownViewer.Json`); on macOS, Finder's *Open With* offers it; on a deb/rpm
  install, `xdg-mime query filetype x.jsonc` gives `application/json` and *Open With* lists the
  app.
  *Windows proven 2026-09-13 on the installed 1.6.0: `HKCU\Software\Classes` has
  `T4MarkdownViewer.Json` ("JSON Document", icon, open command) and both `.json` and `.jsonc`
  list it under `OpenWithProgids`; `.md` still maps to `T4MarkdownViewer.Document`.*
  *deb proven 2026-09-26 on dry run 36186470162's .deb, in WSL Ubuntu 24.04:
  `xdg-mime query filetype x.jsonc` gives `application/json`, `gio mime application/json` and
  `text/markdown` list the app (and default to it), and `desktop-file-validate` passes. rpm
  proven the same way the same day on Fedora 44.*
  *macOS `.json` proven 2026-09-28 on the installed 1.7.1 (macOS 26.7): the bundle's
  `T4MarkdownViewer.Json` type claims `public.json`, LaunchServices lists the app for a
  `.json`, and Finder's *Open With* shows it. `.jsonc` does not bind on macOS — deferred in
  `open-items.md`, owner's call.*

- [x] **Mermaid diagrams on macOS and Linux** (added 2026-09-26, `archive/plans/mermaid-diagrams.md`).
  `drive-app` proves them on Windows (WebView2) only. On a Mac (WKWebView, macOS 13 floor)
  and a deb/rpm install (WebKitGTK), open `examples/kitchen-sink.md`: the diagram draws,
  clicking it opens the overlay, `Ctrl`+wheel (or a trackpad pinch) zooms about the cursor —
  `Cmd`+wheel doesn't, as in a picture tab (`onImageWheel`) and per the README — dragging pans,
  Escape comes back at the same scroll, and the light/dark toggle redraws it, as does an OS
  light/dark switch under a theme that follows the OS. No bundled theme does, so save one in
  the app's themes folder (`src-tauri/themes/README.md` says where) holding
  `:root { color-scheme: light dark } body { background: transparent }`, pick it, and switch
  the OS setting; then again with the body rule replaced by `body { background: #fff }` plus
  `@media (prefers-color-scheme: dark) { body { background: #111 } }`. The page itself must
  change side first — if it doesn't, the webview never saw the switch, which is not this
  check failing.
  *Linux is fully proven, 2026-09-28, on an Ubuntu 26.04 VM (GNOME 50, Wayland, WebKitGTK
  2.52.6), the debug build at e82fe38. Under Xvfb, with real input through xdotool: a click
  opens the overlay; Ctrl+wheel (`xdotool keydown ctrl click 4 keyup ctrl`) zooms about the
  cursor — held sideways at every level, and up and down once the drawing is taller than the
  view (while it is shorter it is centred, so there is nothing to hold it by); a drag of
  (-200, -100) pans exactly that; Escape comes back at the same scroll; the bar's light/dark
  toggle redraws the diagrams both ways. On the live desktop, both test themes changed side
  with the OS and redrew their diagrams at every switch. GNOME has two switches, and the
  webview is dark if either is: Settings' Dark Style (`color-scheme prefer-dark`) or a dark
  GTK theme (`gtk-theme Yaru-dark`, Ubuntu's default, or `Adwaita-dark`). So under a dark GTK
  theme, turning Dark Style off never reaches the app — GNOME's doing, not a failure here.
  macOS still needs a Mac.*
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
  *macOS run 2026-09-28: macOS 26.7 (arm64), WebKit 21624.5.1.11.3, main at 622f29b,
  `drive-app`'s `macos.md`.
  On a plain debug build, by the owner's eye (the feature build swaps WKWebView's UI delegate,
  which wry #1593 is about): the diagram draws, no second window opens, and the context menu's
  Reload comes back with both tabs and the diagram.
  On the feature build, with real input: 1 block, 1 figure, none left as code, no
  `window.mermaid`; a click opens the overlay; `Ctrl`+wheel and pinch zoom (0.057 to 8.69) —
  sideways the point held to 0.0004 of the width from scale 2 up, and drifted below that (max
  0.16 below scale 1, 0.044 between 1 and 2), where the drawing is centred or the scroll is at
  an edge (reasoned; passed by the owner); up and down it held to 0.025 once the drawing was
  taller than the view; a (-200, -100) drag moved `scrollLeft` exactly 200; Escape came back at
  scroll 2399; the bar's toggle redrew both ways (SVG hash and node fill). Both test themes
  changed side with the OS appearance and redrew at each switch. Keyboard: Tab rings the
  diagram and the picture, Enter and Space open both, Escape from a keyboard-opened overlay
  returns focus to the diagram. The feature build doesn't scroll from the keyboard at all (plain
  builds do, measured), so "a click, Escape, then Space scrolls the page" ran on a plain debug
  build, 2026-09-29: the owner clicked the diagram, Escape closed the overlay, and Space
  scrolled the page (the scroll bar's value 0.8588 → 1.0) without reopening it. macOS proven;
  the item is done.*
  *Closed on macOS 26.7, not the macOS 13 floor the item names: that check is deferred in
  `open-items.md` (2026-09-29, owner's call).*

- [x] **Mac keeps folder access across an update.** 1.5.8 was the first signed build and 1.5.9
  the first signed-to-signed update. CI proves both carry the cert (SHA-1 `53effb03…`). Only a
  Mac updating 1.5.8 → 1.5.9 without a new Documents/Desktop/Downloads prompt proves 929fe3a
  did its job. Any later signed-to-signed step is the same proof: every release since, through
  1.6.8, carries the same certificate.
  *Proven 2026-09-29 on macOS 26.7, the owner at the keys: the signed 1.7.0 in
  `/Applications`, its Documents grant reset (`tccutil reset SystemPolicyDocumentsFolder
  com.montevirgen.t4-markdown-viewer`), listed `~/Documents` in the sidebar on its own
  (Shift+Cmd+O, no Open panel) and macOS asked; the owner allowed it. A relaunch restored the
  sidebar there with no prompt. Then the in-app update to 1.7.1 (same certificate, SHA-1
  `53:EF:FB:03…`, updated in place) relaunched and restored the sidebar on `~/Documents` with
  no prompt: the grant held across the update. Two first attempts proved nothing and were
  redone: one ran an older copy in `~/Applications` whose version was unknown, and opened its
  files through Finder's *Open With* (a file the user opens that way likely skips the Documents
  check; reasoned); in the other, a file picked through the Open panel reopened on restore
  without the Documents check (plausibly the `com.apple.macl` mark the panel leaves;
  reasoned) — so a folder listing is the test, not a file opened by the user.*

## Housekeeping

- [x] **Docs folder layout.** Plans, the review and this tracker all sat in `plans/`.
  *Reorganised 2026-09-13: this file at `docs/`, finished plans in `archive/plans/`, the
  2026-09-12 review in `archive/reviews/`, screenshots unchanged.*
- [x] `archive/plans/ci-alignment.md` and `archive/plans/ci-alignment-round-2.md` still show
  unticked boxes,
  although that work shipped. Round 2's "delete `dependabot.yml`" was later reversed:
  Dependabot is back, and 2bace27 configures it. Tick or annotate them, or leave them as
  history.
  *Annotated on 2026-09-11 with a status line at the top of each.*
- [x] Code-scanning alerts weren't checked (the `gh` token lacks `admin:repo_hook`), and
  `cargo audit` isn't installed. Dependabot alerts (0 open) do cover `Cargo.lock`.
  *Checked on 2026-09-11. Code scanning isn't configured (default setup reports
  `not-configured`, and there's no CodeQL workflow), so there are no alerts to miss. Secret
  scanning and push protection are on. Dependabot alerts cover `Cargo.lock`, so `cargo audit`
  would only repeat them.*

## Accepted limits

Limits the owner has ruled to keep, moved here once ruled on. Still true of the app or its
tooling; listed so nobody rediscovers them.

- [x] **`session.json` is a record of a moment, not a log** (session restore, 1.6.3). Close one
  window and read on in another, and the closed one is gone from the record. Closing every
  window comes back whole.
  *Kept 2026-09-25: keeping closed windows in the record would change what closing means —
  the next launch would bring back windows the reader had closed on purpose.*
- [x] **#24: a failed folder watch drops the existing watcher** (2026-09-20 review). The next
  expand, collapse or tab switch asks again.
  *Kept 2026-09-25: `watch::watch` gives nothing back only when there is nothing to watch —
  where dropping the old watcher is right — or when every folder asked for failed. Then the
  old watcher covers either those same folders, as dead as the new one, or a folder no longer
  on show. Telling the two apart in `watch.rs` and in `set_watch`, which the file watcher
  shares, would buy nothing.*
- [x] **#25: a folder whose name is not valid UTF-8 comes back mangled** (2026-09-20 review)
  and then fails as "Not a folder". Linux only, and only through the picker: the tree hides
  such entries (`is_visible_entry`).
  *Kept 2026-09-25: the picker hands the name to the page as a string, so the bytes are gone
  before any of this app's code sees them. A fix means its own picker command and an encoded
  path type through all eleven commands that take a path — for an error row that loses
  nothing, on a platform nothing here can test.*
- [x] **A picture named by a full path outside any folder a tab has been opened from, or the
  repository it sits in, does not show** (#8, 2026-09-20 review): the asset protocol serves
  only those folders. A link to it works.
  *Kept 2026-09-25: since 320dcaa a leading slash is taken from the repository or the
  document's folder, so the only full path left is a Windows drive path — the limit is
  Windows-only. A page-side "allow this file" command would let the page widen its own scope
  and defeat it; the sound fix, Rust allowing each drive-path picture it renders, is ~25 lines
  and a change to what `render.rs` hands back, for a form documents rarely use.*
- [x] **A JSON render whose folding would stall the window is shown without fold controls**
  (#9, 2026-09-20 review) — nesting thousands deep, or a far-expanded document of very short
  lines on a re-render (`MAX_FOLD_WEIGHT` in `json.rs`). Everything else about it is as ever.
  *Kept 2026-09-25: it is the guard that ended #9's hang. A full fix is virtualized rendering.
  The middle ground — a first pass that finds the deepest level whose folds stay under the
  weight, and folds only down to it — is ~40 lines in `json.rs` and a fresh WebView2
  measurement, for documents this heavy are rare. Reopen if one turns up that needs folds.*
- [x] **A kill or a Windows shutdown loses the last 500 ms of changes** (session restore,
  1.6.3). A report waits 500 ms for things to settle — scrolling, moving or resizing a window,
  a tab change hard on another — and what is inside that wait lives only in the page.
  *Kept 2026-09-25. Closing a window waits for the report since 21384f3, but a kill sends
  nothing, and a shutdown or logoff reaches `tao` as `WM_ENDSESSION`, which it turns straight
  into `RunEvent::Exit` (`WM_QUERYENDSESSION` is left unhandled there on purpose), with the
  event loop gone before any page could be asked. macOS Cmd+Q is fixable and stays open under
  *Needs a Mac* in `open-items.md`.*
- [x] **#49: Add/Remove Programs shows the publisher as "montevirgen", not the signer's name**
  (2026-09-25 review). *Kept 2026-09-25: `bundle.publisher` also moves the NSIS registry key,
  losing the remembered install folder once and leaving the old key behind — worse than the
  cosmetic it would fix.*
- [x] **#50: a `//` comment in a JSONC file with bare-CR line endings runs to the end of the
  file** (2026-09-25 review) — colouring only, no text lost. *Kept 2026-09-25: old-Mac CR-only
  files are vanishingly rare.*
- [x] **A reload brings a window back as it last reported** (the navigation guard, 7060f25):
  a zoom or a folder expanded in the moment since is gone.
  *Kept 2026-09-26: the page reports within half a second of a change, and without the
  guard's recovery a reload lost every tab.*
- [x] **A document that crashes the page crashes it again on Refresh** (7060f25): the reload
  brings its tab back. Closing the window is the way out, as it always was.
  *Kept 2026-09-26: telling that document apart from the rest would mean a crash counter
  kept across reloads, for a case #51's 4 MB cap and the render bounds make rare.*
- [x] **A tab on a share that has gone away slows live reload in its window** (e904196). Each
  watch request re-checks every open file, and one on a dead host waits ~21 s whenever
  Windows' failure cache has lapsed; the other tabs' changes wait behind it. The window
  itself stays responsive, and requests a newer one replaced are skipped.
  *Kept 2026-09-26: watching per file, so a dead one cannot hold the rest, is a rewrite of
  `watch.rs` for a share that has already failed.*
- [x] **Each `more` click on a 32 MB one-line JSON file still takes ~0.3 s** (#47, e904196):
  the file is reflowed again per click, one click at a time.
  *Kept 2026-09-26: caching the reflow would hold up to 151 MB for as long as the tab is
  open; `json_region`'s `ponytail:` note says how, if the wait is ever felt.*
- [x] **The keyboard handling is the core keys only** (#37, 70e935c): no Home/End, no
  type-ahead and no `*` in the tree; focus is lost when the tab strip or the tree rebuilds for
  some other reason, such as Ctrl+Tab or a watcher re-listing; each tab's close button is a
  Tab stop of its own.
  *Kept 2026-09-26: every control works from the keyboard; the rest is polish to add when
  keyboard or screen-reader use asks for it.*
- [x] **Three routing paths have no tests** (#48, a049765): a tab dropped onto another window
  (the adopt branch hit-tests with Win32 `WindowFromPoint`), where a restored window is
  placed (the mock runtime reports no monitors), and a restore that builds several windows at
  once (the mock's window table is not thread-safe). Noted where the tests would go in
  `routing_tests.rs`.
  *Kept 2026-09-26: out of the mock runtime's reach; the drive-app skill covers them.*
- [x] **A session saved before 1.6.9 restores with every sidebar closed**: those files do not
  say whether it was open.
  *Kept 2026-09-26: the next save records it.*
- [x] **The uninstaller's "Delete the application data" was checked by hand** (04a3fdc), in
  Windows Sandbox — the silent uninstaller cannot tick it. The unticked and update cases ran
  scripted.
  *Kept 2026-09-26: re-check by hand if the NSIS hook changes.*
- [x] **A release tag's own path first runs on a real release** (08606a8). Dry runs dispatch
  from `main`, so they skip `checks`, publish only a draft, and never meet the `signing`
  environment's `v*` tag rule. A failure there shows after the tag is public.
  *Kept 2026-09-26: the fix is a re-run or a patch tag; a dispatch on a tag ref would still
  publish only a draft. The 1.6.9 run is the proof.*
- [x] **A second instance on Windows or macOS still aborts on an argument that is not Unicode**
  (009d8c8 fixed Linux only). `tauri-plugin-single-instance` 2.4.5 forwards argv with
  `std::env::args()` there too (`windows.rs:91`, `macos.rs:88`). Windows passes UTF-16, so only
  a name with an unpaired surrogate reaches it; macOS stores names as Unicode and Finder opens
  files through Apple Events, so only a terminal launch with made-up bytes does. The running
  window is untouched and the file could not open anyway (#25).
  *Kept 2026-09-26: no file manager on either can produce it, Windows has no `exec` to
  re-launch with, and it goes away with the Linux workaround once the plugin reads
  `args_os()`.*
- [x] **A second launch from a folder whose name is not Unicode loses its working directory**
  (found planning 009d8c8). `tauri-plugin-single-instance` sends
  `current_dir().to_str().unwrap_or_default()`, so the running app receives an empty cwd and
  resolves a *relative* argument against its own: the file is not found, or by coincidence a
  same-named file there opens. No crash.
  *Kept 2026-09-26: only a terminal hits it — file managers pass absolute paths (`%f`) — and
  re-executing cannot change what the plugin reads for the cwd. Reopen if the plugin starts
  sending the cwd as bytes, or on a report of the wrong file opening.*
- [x] **Event-modeling diagrams put attribute-free HTML on the page** (Mermaid, bf0a58b). Their
  data reaches the page as DOMPurify-filtered HTML in which a tag carrying any attribute is
  dropped, keeping its text, and no mermaid setting stops it, so they are exempt from the
  foreignObject check. Bare tags survive (`b`, `table`, `form`, `button`, `details`,
  `marquee`…); form submits are blocked in the page and the overlay. The README's Mermaid note
  says what survives.
  *Kept 2026-09-27: refusing the type loses it outright; what gets through is cosmetic.*
- [x] **Journey and venn diagrams are exempt from the foreignObject check by type, not by
  content** (Mermaid, bf0a58b). Measured text-only on mermaid 11.17.2; a later mermaid that
  put HTML in them would pass unchecked.
  *Kept 2026-09-27: a content check cannot tell mermaid's wrapper `<span style>` from a
  file's, and would let architecture's text icons back in. The release skill re-measures on
  every mermaid bump.*
- [x] **Architecture services with a text icon are refused** (Mermaid, bf0a58b):
  `service id "text"[title]`. mermaid writes that text as HTML, so a file's markup would
  become elements. Services with no icon or a named one draw.
  *Kept 2026-09-27: the refusal is the safety.*
- [x] **Any `$$…$$` in a diagram's source refuses it** (Mermaid, bf0a58b), even inside a `%%`
  comment. Math in a label would force HTML labels; the check reads the whole source.
  *Kept 2026-09-27: it errs on the safe side, and the error names what to remove.*
- [x] **A journey task name longer than two lines is clipped** (Mermaid): mermaid's task box is
  a fixed 50 px. mermaid.live draws it the same.
  *Kept 2026-09-27: mermaid's own rendering.*
- [x] **A diagram `click` URL written with an HTML entity becomes a local path** (Mermaid):
  `&#106;avascript:x` comes out as `xlink:href="&javascript:x"`, which reads as a relative
  file, so the click reveals a file that isn't there.
  *Kept 2026-09-27: harmless, and only a deliberately disguised URL writes it.*
- [x] **A JSON `more` chunk can land on no history entry** (Mermaid review loop; older than
  it). A chunk that arrives after Back or Forward to another entry on the same JSON file is
  spliced into the page, but its extent is recorded on neither entry, so the next re-render
  drops back to the shorter view; another click brings it back.
  *Kept 2026-09-27: near-unreachable. A JSON page has no links or ids of its own, so the only
  way to put two adjacent entries on one JSON file is a sidebar click on another spelling of
  the open file, such as a symlink to it. Fix, if it ever matters: record the extent on
  `shownEntry` when the chunk lands.*
- [x] **Input within a frame of a render can bank the page before its restore** (review of the
  Mermaid triage fixes; older than them). Back/Forward, or another page swapped in, before
  `renderDocument`'s restore frame runs saves the unrestored offset on the just-rendered entry.
  A cross-file anchor entry then counts as visited, and a return lands at that offset rather
  than on the heading.
  *Kept 2026-09-27: a ~16 ms window. Fix, if it ever matters: a restore-pending flag set in
  `renderDocument` and cleared in its frame, which `rememberScroll` respects.*
- [x] **A theme that animates its page background leaves the page colour behind** (review of
  the Settings footer and OS light/dark fixes; older than them). `applyTheme` reads the body
  background right after swapping the stylesheet, which under a `transition` is the start
  colour. `--page-bg` (the active tab, the top fade, the diagram overlay) keeps it until the
  next theme change or OS switch, and an overlay opened mid-transition can stay open after
  the side flips. Diagrams recover: a click on one in an outdated look redraws it.
  *Kept 2026-09-27: no bundled theme animates its background, and the themes README says not
  to. Fix, if it ever matters: a `transitionend` listener on body that re-reads both.*
- [x] **A Windows contrast theme doesn't redraw diagrams** (same review; older, unverified).
  `forced-colors` can change the page's colours without a `prefers-color-scheme` change, so
  no redraw fires; diagrams keep their palette until a click on one or the next render.
  *Kept 2026-09-27: niche, and a click or the next render recovers. Fix, if it ever matters:
  also listen to `matchMedia("(forced-colors: active)")`.*
- [x] **Tabbing onto Settings' Done may nudge the dialog's scroll** (same review;
  unconfirmed). Done sits in the pinned footer, inside the `scroll-padding-bottom` zone, so
  focusing it can scroll the dialog toward its end.
  *Kept 2026-09-27: harmless; the Tab walk and a click on Done both pass.*
- [x] **The diagram frame's answer is trusted, as mermaid's own in-page output was** (Mermaid
  pipeline, 7a63106). The frame's markup is trusted as mermaid's own: nothing checks that the
  string a render comes back with is really mermaid's SVG rather than something else the
  frame's script sent.
  *Kept 2026-09-27: the frame is sandboxed to scripts alone with no origin. Against one a bug
  or a compromise turned hostile, the page's CSP (`script-src 'self'`, no inline script) blocks
  script, and the page checks styles itself: the drawing's root must carry the id the page
  asked for, only that root is inserted, and every rule in it must be scoped under it. The CSP
  does not cover styles.*
- [x] **A swimlane group named like another figure's own root id keeps its raw id** (Mermaid
  pipeline, d730f11). `Mermaid-12` written as a swimlane group name inside the figure whose own
  root is `Mermaid-1` collides with it after renaming.
  *Kept 2026-09-27, owner's call: self-inflicted, and reachable only inside one document that
  names a group after another figure's internal id.*
- [x] **`@keyframes` in a diagram's `themeCSS` stay global** (Mermaid pipeline, c241696). The
  style check that refuses an unscoped rule exempts `CSSKeyframesRule`, since a keyframes name
  has no selector to scope.
  *Kept 2026-09-27: harmless here — the app defines no animations of its own for a diagram's
  keyframe names to collide with.*
- [x] **C4 relationship labels are mermaid's fixed `#444444` on every theme** (Mermaid pipeline,
  efc2b50). No `themeVariables` key reaches them; 1.5–1.9:1 contrast on a dark theme.
  *Kept 2026-09-27, owner's call, as on GitHub: mermaid has no key for this label to take.*
- [x] **Author-chosen diagram fills are not adjusted to the theme** (Mermaid pipeline, efc2b50).
  A diagram's own `classDef`/`style`/box colours always apply, in both colour modes, as on
  GitHub.
  *Kept 2026-09-27, owner's call: the author chose those colours for that diagram; adjusting
  them would be recolouring what the file itself wrote.*
- [x] **Both Solarized themes' category labels read at ~3:1** (Mermaid pipeline, efc2b50).
  Category colours target 4.5:1 against the text, or 3:1 on a theme whose own text is under
  5:1 on its page — both Solarized themes are 4.1–4.7 — since a higher bar left the colours,
  and the lines mermaid draws in them, barely apart from the page (1.04–1.11 measured).
  *Kept 2026-09-27, owner's call: distinct hues with ~3:1 labels, over pale, near-identical
  tints.*
- [x] **Radar rings are drawn at 0.06 opacity in the theme's colours, not mermaid's 0.3**
  (Mermaid pipeline, b6b2798). mermaid's `radar.graticuleOpacity` default is 0.3; its five
  nested, filled rings stack to about 83% and band solid on both sides of the curve under the
  theme's palette.
  *Kept 2026-09-27, owner's call: viewed on github-dark and github-light, 0.06 leaves the rings
  faint and the curve clear.*
- [x] **A sequence diagram's `link`/`links` on an `actor`/`boundary`/`control`/`entity`
  participant fails the whole diagram** (Mermaid follow-ups, c4b4b89). mermaid's own menu-drawing
  code errors on those participant kinds; the diagram is refused rather than only losing its
  links.
  *Kept 2026-09-27, owner's call: a mermaid bug — revisit on a mermaid version bump.*
- [x] **`#` in a sequence `link`/`links` line starts a comment** (Mermaid follow-ups, c4b4b89),
  anywhere in the link, so no link in a sequence menu can carry a `#` fragment (`notes.md#part`
  and `https://x/#a` are cut too). A relative file or a web address without a fragment works.
  *Kept 2026-09-27, owner's call: documented in README.md.*
- [x] **A `database`/`queue` participant's bottom box doesn't open its link menu** (Mermaid
  follow-ups, c4b4b89): only the top box and the lifeline are matched to a participant's name, and
  a database/queue's bottom box is drawn with no `rect.actor[name]` to match. Its top box, its
  lifeline and the full-window overlay all still work.
  *Kept 2026-09-27, owner's call.*
- [x] **`url(#…)` text inside a link's own URL is rewritten by `renameIds`** (Mermaid pipeline,
  d730f11): a diagram link whose URL contains `url(#id)` naming one of the drawing's ids has
  that id renamed with the drawing's.
  *Kept 2026-09-27, owner's call.*
- [x] **The diagram frame is trusted to send gantt links for any diagram** (Mermaid follow-ups,
  910546d): the page wraps whatever tasks the frame's answer names.
  *Kept 2026-09-27, owner's call: the same trust boundary as the SVG's own `<a>` links.*
- [x] **`renameIds` doesn't rewrite `aria-owns`/`aria-controls`** (Mermaid pipeline, d730f11):
  only `aria-labelledby` and `aria-describedby` are treated as id lists.
  *Kept 2026-09-27, owner's call: mermaid doesn't emit them.*
- [x] **The rightmost actor's link menu can be clipped at the viewBox edge** (Mermaid follow-ups,
  c4b4b89): it can run past the drawing's edge, where it is cut off.
  *Kept 2026-09-27, owner's call: mermaid's own layout.*
- [x] **A gantt task named with its diagram's own root-prefixed id keeps it** (Mermaid pipeline,
  d730f11): the draw-time rename skips ids already under the root, so a task named
  `Mermaid-3x0zoom` or `Mermaid-3x2` could clash with the overlay copy or a repeat.
  *Kept 2026-09-28, owner's call: the author would have to guess the app's id counter.*
- [x] **drive-app on macOS: `macdialog.sh` types ASCII paths only** (2026-09-28,
  `archive/plans/drive-app-macos.md`). It types the path with System Events `keystroke`, through
  the keyboard layout, so a character the layout lacks may be dropped.
  *Kept 2026-09-28, owner's call: the repo's paths are ASCII, and a walk that needs another path
  can open it with `eval "openTab(…)"` instead of the dialog.*
- [x] **drive-app on macOS: the clipboard backup keeps text only** (2026-09-28,
  `archive/plans/drive-app-macos.md`). `pbpaste` / `pbcopy` around a copy-button check lose an
  image, a file or rich text on the owner's clipboard.
  *Kept 2026-09-28, owner's call: `macos.md` says to ask first when the clipboard may hold one,
  and such checks are rare.*
- [x] **drive-app on macOS: WebKit's data goes to the real `~/Library`** (2026-09-28,
  `archive/plans/drive-app-macos.md`). The scratch `HOME` isolates the app's own settings, but
  WebKit ignores `HOME` and writes to `~/Library/WebKit/t4-markdown-viewer` and
  `~/Library/Caches/t4-markdown-viewer` (measured).
  *Kept 2026-09-28, owner's call: those folders belong to the unbundled dev binary, apart from
  the installed app's, and hold nothing the app relies on.*
- [x] **drive-app on macOS runs on the owner's desktop** (2026-09-28,
  `archive/plans/drive-app-macos.md`). There's no Xvfb: debug windows open on the real screen
  and take focus, real input needs the app in front, and the owner's own pointer reaches the
  page mid-walk (seen in an event log).
  *Kept 2026-09-28, owner's call: inherent to macOS; `macos.md` warns before a launch and asks
  the owner to keep off the trackpad during real-input checks.*
- [x] **The `webdriver` feature isn't built in CI** (2026-09-28,
  `archive/plans/drive-app-macos.md`). CI's clippy and tests run without it, so the gated plugin
  line in `main.rs` is never compiled there, and `Cargo.lock` carries the plugin's 21 packages
  (no existing version changed; the default build's graph is identical, measured).
  *Kept 2026-09-28, owner's call: a broken feature build shows at the first walk's build step,
  which is where it matters.*
- [x] **drive-app on macOS: `macpt.sh` assumes page zoom 1** (2026-09-28,
  `archive/plans/drive-app-macos.md`). It maps CSS px to screen points 1:1; a zoomed page would
  send every real click and drag off target.
  *Kept 2026-09-28, owner's call: nothing zooms the page today, and a wrong landing shows at
  once in the `eval` that checks each step.*
