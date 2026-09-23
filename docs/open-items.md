# Open items — t4-markdown-viewer

## Context

Collected on 2026-09-11, after v1.5.9. Sources: a review of history since v1.5.0, CI and
GitHub state, and a smoke run of the debug build using the `drive-app` skill. Last swept on
2026-09-20, after v1.6.5 and the full review of that date (1.5.10 contrast, 1.5.11 review
fixes, 1.6.0 JSON viewer, 1.6.1–1.6.2 tab chrome, 1.6.3 session restore, 1.6.4–1.6.5 sidebar
sort). Nothing here blocks a release.

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
- [ ] **Dependabot's cargo PR #3 fails all three CI legs** (opened 2026-09-12, seen 2026-09-13).
  The group bumps seven crates; `comrak` 0.54 → 0.55 deprecates
  `Extension::tagfilter` (removed in 0.56), and clippy's `-D warnings` turns the deprecation
  at `render.rs:18` into an error. Needs a code change on top of the bump, not a merge. The
  other six (dialog, single-instance, opener, updater plugins, `dirs` 6 → 7, `windows-sys`
  0.59 → 0.61) are untested until comrak compiles.

## Deferred from the 2026-09-20 review

Real defects put off, not accepted (`archive/reviews/code-review-2026-09-20.md`). Each says what
reopens it. #9 was to sit here too; it went under *Bugs* instead, because the smoke pass hung,
and closed the next day (`closed-items.md`).

- [ ] **#28 Anchor recovery misreads an `id=` inside another attribute's value** (added
  2026-09-20; `render.rs`, `attribute`). `<a href="p?a id=q" id="x"></a>` recovers no anchor,
  so `[…](#x)` links to nothing; every ordinary shape works. The fix is a scanner that skips
  quoted values. *Reopen when:* a document in the wild loses an anchor to it, or `attribute` is
  touched for any other reason.

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
  - *#27, #30 and #31 proven 2026-09-21 by 1.6.6: the push's CI run passed on all three legs,
    macOS included; a `workflow_dispatch` packaging build passed before the tag; the v1.6.6
    Release run passed `version`, the checks, all three builds and `publish`. #5 and #6 are
    what keeps this open — 1.6.5 → 1.6.6 is the first in-app update to run #5's guard.*

## Accepted limits

Parked by ruling in the 2026-09-12 review (`archive/reviews/code-review-2026-09-12.md`,
*Implementation*). Not bugs to fix; here so nobody rediscovers them.

- `toggle_task` can NUL-pad the file if an editor truncates it between the read and the
  seek. The pre-fix behaviour (truncate-then-write) was worse.
- `check_for_update` sends one request per window when several boot together; the negative
  cache only helps windows that open later.
- Two concurrent config saves are each atomic, but the pair is last-writer-wins.

Session restore (1.6.3) adds two, each ruled on while planning it:

- A report waits 500 ms for things to settle — scrolling, moving or resizing a window, or a
  tab change hard on the heels of another — so quitting inside that wait loses the last of
  it. (In 1.6.3 a move or resize was not reported at all until the next tab change or scroll;
  1.6.4 reports them.)
- `session.json` is a record of a moment, not a log: close one window and read on in another,
  and the closed one is gone from the record. Closing every window comes back whole.

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
