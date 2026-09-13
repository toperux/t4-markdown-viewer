# Open items — t4-markdown-viewer

## Context

Collected on 2026-09-11, after v1.5.9. Sources: a review of history since v1.5.0, CI and
GitHub state, and a smoke run of the debug build using the `drive-app` skill. Last swept on
2026-09-13, after v1.6.0 (1.5.10 contrast, 1.5.11 review fixes, 1.6.0 JSON viewer). Nothing
here blocks a release.

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

## Bugs

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

## Accepted limits

Parked by ruling in the 2026-09-12 review (`archive/reviews/code-review-2026-09-12.md`,
*Implementation*). Not bugs to fix; here so nobody rediscovers them.

- `toggle_task` can NUL-pad the file if an editor truncates it between the read and the
  seek. The pre-fix behaviour (truncate-then-write) was worse.
- `check_for_update` sends one request per window when several boot together; the negative
  cache only helps windows that open later.
- Two concurrent config saves are each atomic, but the pair is last-writer-wins.

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
