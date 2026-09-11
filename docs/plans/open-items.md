# Open items — t4-markdown-viewer

## Context

Collected on 2026-09-11, after v1.5.9. Sources: a review of history since v1.5.0, CI and
GitHub state, and a smoke run of the debug build using the `drive-app` skill. Nothing here
blocks a release, and there are no user-facing changes since v1.5.9.

Tick items as they close. Move this file to `done/` once every box is ticked.

## Needs a decision

- [ ] **Reply to and close issue #1, "Cannot open folder"** (macOS, opened 2026-09-10, no
  comments). The reporter wanted to open a whole folder from the start screen. The
  "Open a folder…" button beside "Open a file…" (10c7458) shipped in 1.5.8, and a5e5fef (1.5.9)
  points to the sidebar once a folder is open. The reply is public, so the owner posts it or
  approves it. Ask the reporter to confirm the Mac update item below too.
- [ ] **Delete the stale branches.** None holds work missing from `main`; each was squashed in.
  - local: `backup-pre-fixup`, `backup-pre-plandoc`, `backup-pre-squash`,
    `backup-pre-theme-squash`, `session-restore`, `cross-platform`
  - remote: `origin/cross-platform`
- [ ] **Frontend tests, or not.** `src/app.js` has no automated tests, and the UI is only ever
  checked by hand with the `drive-app` skill. Features with manual checks only: sidebar filter,
  update from Settings, sidebar folder switching, the folder picker, the empty-screen hints.
  A harness means adding Node tooling, which the repo has avoided so far.

## Bugs

- [ ] **Accent buttons are hard to read in the Dracula themes.** Every accent button is `#fff`
  text on `var(--ui-accent)`: five rules in `src/base.css`, among them `#empty button`,
  `#bar #update-btn:hover`, `#settings-dialog #settings-update:hover` and
  `#update-actions #update-now`. Contrast measured on 2026-09-11, against WCAG AA's 4.5 for
  normal text:

  | Theme | Ratio |
  | --- | --- |
  | dracula-green | 1.37 |
  | dracula-blue | 2.30 |
  | dracula | 2.41 |
  | github-dark, github-dark-blue | 3.10 |
  | solarized-light, solarized-dark | 3.68 |
  | azure-devops (all four) | 4.53 |
  | github-light, github-light-blue | 5.19 |
  | sakura | 5.41 |
  | tufte | 8.42 |

  A fix probably needs a per-theme text colour for accent buttons, with `#fff` as the default.
  Recheck all five rules, not just the empty screen.
- [ ] **The debug app ignored a normal close while Settings was open.** In the 2026-09-11 smoke
  run, `taskkill //PID` (WM_CLOSE) did nothing for 5+ s with the Settings dialog open, and
  `//F` worked. The installed 1.5.9 closed normally that day with no dialog open. Neither `src/`
  nor `src-tauri/src/` has a close-request handler. Unconfirmed. To reproduce: launch, open
  Settings, then close with the window's X and with `taskkill`. If the X fails too, it's a real
  bug.

## Can't be checked from Windows yet

- [ ] **Mac keeps folder access across an update.** 1.5.8 was the first signed build and 1.5.9
  the first signed-to-signed update. CI proves both carry the cert (SHA-1 `53effb03…`). Only a
  Mac updating 1.5.8 → 1.5.9 without a new Documents/Desktop/Downloads prompt proves 929fe3a
  did its job.
- [ ] **Dependabot `ci:` prefix (2bace27).** The release-notes filter (`release.yml`, "Build the
  release notes") drops `ci: Bump …`, checked locally; a plain `Bump …` still gets through.
  Confirm the next weekly Dependabot PR is titled `ci: …`.
- [ ] **"Update available" in Settings (586560c).** Only the up-to-date path has been seen
  ("You are up to date." on 1.5.9). The other path needs an older build: install 1.5.8 under a
  spare Windows account (it installs per user), then Settings → Check now. Expect "Update…"
  beside the status, and the bar's Update button.

## Housekeeping

- [ ] `done/ci-alignment.md` and `done/ci-alignment-round-2.md` still show unticked boxes,
  although that work shipped. Round 2's "delete `dependabot.yml`" was later reversed:
  Dependabot is back, and 2bace27 configures it. Tick or annotate them, or leave them as
  history.
- [ ] Code-scanning alerts weren't checked (the `gh` token lacks `admin:repo_hook`), and
  `cargo audit` isn't installed. Dependabot alerts (0 open) do cover `Cargo.lock`.
