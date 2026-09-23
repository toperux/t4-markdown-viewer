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
  - *The price is under* Needs a decision *in `open-items.md`: that launch loses the whole
    session. `"ask"` was not run; by the code the file stays, and so does the choice not to
    press Reopen.*

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
