# Open items — t4-markdown-viewer

## Context

Collected on 2026-09-11, after v1.5.9. Sources: a review of history since v1.5.0, CI and
GitHub state, and a smoke run of the debug build using the `drive-app` skill. Last swept on
2026-09-20, after v1.6.5 and the full review of that date (1.5.10 contrast, 1.5.11 review
fixes, 1.6.0 JSON viewer, 1.6.1–1.6.2 tab chrome, 1.6.3 session restore, 1.6.4–1.6.5 sidebar
sort). Nothing here blocks a release.

Tick items as they close. Move this file to `archive/` once every box is ticked. Finished
plans and reviews live in `archive/plans/` and `archive/reviews/`.

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
- [ ] **A stale Reopen offer can outlive the session it describes** (added 2026-09-17, 1.6.3).
  With **Ask me each time**: ignore the offer, open a file, then close that tab. The empty
  screen comes back with the Reopen button still on it, and pressing it restores the session
  Rust is still holding in memory — even though `session.json` was overwritten by the file you
  opened seconds earlier. Arguably a courtesy rather than a fault: the stash is intact and
  nothing else can reach it. Options: leave it, or hide the button the first time this window
  opens anything.
- [ ] **Later does not cancel an update that has started** (added 2026-09-20, from #5 of
  `archive/reviews/code-review-2026-09-20.md`). Pressing **Update now** and then **Later** closes the
  dialog, but the download carries on and the app still restarts under the reader. Since
  769cd54 a reopened dialog says an install is under way; nothing makes Later mean it. Options:
  leave it (the reader did ask for the update), relabel the button to **Hide** once a download
  is running, or make it cancel — which needs the updater plugin's download to be abortable,
  unchecked.
- [ ] **A launch that dies mid-restore loses the whole session** (added 2026-09-21, found
  answering the crash-loop question below). `setup` discards `session.json` before it
  restores, and a window writes it again only once its document is laid out; a process that
  goes down in between leaves no file, so the next launch is the empty screen — the documents
  that were fine went with the one that was not. That is also exactly what stops a bad
  document coming back for ever. Options: leave it (one lost session for a guaranteed way out),
  or discard only once the first window has reported, and count failed restores so a second
  one in a row starts fresh.

## Bugs

- [ ] **A restored scroll position degraded once, unreproduced** (added 2026-09-17, 1.6.3).
  During the 1.6.3 drive run a tab recorded `scrollY: 1500`, and some minutes later the same
  entry read 500 with nothing having touched that window. Two later runs round-tripped 3000
  exactly, cold restore included, so it is not the obvious suspect (a render clamping the
  offset while the document is still short, which the scroll listener would then write back).
  No theory. Reopen with a reproduction; the cost if real is landing in the wrong place once,
  after which the wrong value sticks.
  *2026-09-20: #4 of the review is a mechanism that fits — `rememberScroll` banked the outgoing
  page's offset onto the incoming entry when a second tab switch landed before the first had
  loaded. Fixed in 0cbadf1. Leave open until a release has gone by without a recurrence.*
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
- [ ] **Dependabot's cargo PR #3 fails all three CI legs** (opened 2026-09-12, seen 2026-09-13).
  The group bumps seven crates; `comrak` 0.54 → 0.55 deprecates
  `Extension::tagfilter` (removed in 0.56), and clippy's `-D warnings` turns the deprecation
  at `render.rs:18` into an error. Needs a code change on top of the bump, not a merge. The
  other six (dialog, single-instance, opener, updater plugins, `dirs` 6 → 7, `windows-sys`
  0.59 → 0.61) are untested until comrak compiles.
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

Real defects put off, not accepted (`archive/reviews/code-review-2026-09-20.md`). Each says what
reopens it. #9 was to sit here too; it is under *Bugs* instead, because the smoke pass hung —
and closed there the next day.

- [ ] **#28 Anchor recovery misreads an `id=` inside another attribute's value** (added
  2026-09-20; `render.rs`, `attribute`). `<a href="p?a id=q" id="x"></a>` recovers no anchor,
  so `[…](#x)` links to nothing; every ordinary shape works. The fix is a scanner that skips
  quoted values. *Reopen when:* a document in the wild loses an anchor to it, or `attribute` is
  touched for any other reason.
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
  - *The price is under* Needs a decision*: that launch loses the whole session. `"ask"` was
    not run; by the code the file stays, and so does the choice not to press Reopen.*

## Needs a Mac, a Dependabot run, or an older build

- [ ] **Mac keeps folder access across an update.** 1.5.8 was the first signed build and 1.5.9
  the first signed-to-signed update. CI proves both carry the cert (SHA-1 `53effb03…`). Only a
  Mac updating 1.5.8 → 1.5.9 without a new Documents/Desktop/Downloads prompt proves 929fe3a
  did its job. Any later signed-to-signed step (1.5.10, 1.5.11, 1.6.0 are all signed) is the
  same proof.
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
- [ ] **Dependabot `ci:` prefix (2bace27).** The release-notes filter (`release.yml`, "Build the
  release notes") drops `ci: Bump …`, checked locally; a plain `Bump …` still gets through.
  Confirm the next weekly Dependabot PR is titled `ci: …`.
  *Not yet seen. PR #2 ("Bump softprops/action-gh-release", 2026-09-10) predates 2bace27, so
  it's no evidence either way. The cargo entry's prefix does work: PR #3 (2026-09-12) is titled
  `chore: bump the cargo group …`. Wait for the next github-actions bump.*
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
- [ ] **An update restart still restores the session** (added 2026-09-17, 1.6.3). `snapshot`
  claims `session.json` for the restart and sets `AppState.installing`, which stands ordinary
  saves down until the install lands or fails, and the `restart` flag makes the session come
  back whatever **Reopening** says. All of it is verified by reading the code and by the
  `session.rs` tests; none of it has survived a real install, which needs a published release
  to update from. The next in-app update is the proof: tabs, windows and scroll positions
  should all return, even with **Start fresh** selected.
- [ ] **2026-09-20 review fixes that nothing here could run** (added 2026-09-21). Each passed
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

## Accepted limits

Parked by ruling in the 2026-09-12 review (`archive/reviews/code-review-2026-09-12.md`,
*Implementation*). Not bugs to fix; here so nobody rediscovers them.

- `toggle_task` can NUL-pad the file if an editor truncates it between the read and the
  seek. The pre-fix behaviour (truncate-then-write) was worse.
- `check_for_update` sends one request per window when several boot together; the negative
  cache only helps windows that open later.
- Two concurrent config saves are each atomic, but the pair is last-writer-wins.

Session restore (1.6.3) adds four, each ruled on while planning it and stated in the README so
a reader meets them before they surprise anyone:

- A window's frame is only as fresh as its last report. Move a window and quit without
  touching a tab or scrolling, and it comes back where it was before the move.
- The report is debounced by 500 ms, so quitting inside that window loses the last change.
- `session.json` is a record of a moment, not a log: close one window and read on in another,
  and the closed one is gone from the record. Closing every window comes back whole.
- A manual reinstall of a *different* version drops the saved session once, because the file
  records the version that wrote it. An in-app update does not — its own snapshot rewrites the
  file with the version being installed.

Ruled on in the 2026-09-20 review (`archive/reviews/code-review-2026-09-20.md`):

- #18, the superseded case: an `openFolder` overtaken by a newer one keeps its unverified
  pick, because the newer call has already written the tab and the older cannot tell whether
  its own listing failed. The closed-mid-listing case is fixed (74f2192).
- #23: a file-association open in the instant a window is closing is dropped.
- #24: a failed folder watch drops the existing watcher; the next expand, collapse or tab
  switch asks again.
- #25: a folder whose name is not valid UTF-8 comes back mangled and then fails as "Not a
  folder". Linux only, and only through the picker.
- A link with a leading slash (`/docs/guide.md`, `/home/u/spec.md`) is resolved against the
  document's folder, not the filesystem root; only drive-letter paths stand on their own (#8).
- A picture named by a full path outside any folder a tab has been opened from does not show:
  the asset protocol serves only those folders. A link to it works (#8).
- A JSON render whose folding would stall the window — nesting thousands deep, or a
  far-expanded document of very short lines on a re-render — is shown without fold controls
  (`MAX_FOLD_WEIGHT` in `json.rs`). Everything else about it is as ever (#9).

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
