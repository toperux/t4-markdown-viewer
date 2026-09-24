# Open items — t4-markdown-viewer

## Context

Collected on 2026-09-11, after v1.5.9. Sources: a review of history since v1.5.0, CI and
GitHub state, and a smoke run of the debug build using the `drive-app` skill. Swept after
v1.6.5 and the full review of 2026-09-20, and again on 2026-09-25 after v1.6.8: what is left
needs a Mac, a Linux install or a date. Nothing here blocks a release.

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
- [ ] **The app starts on a file name that is not Unicode** (added 2026-09-25, split from the
  2026-09-20 review's "fixes that nothing here could run", now in `closed-items.md`; review
  #6, 0d08e71). `setup` read argv with `std::env::args()`, which panics on a non-UTF-8
  argument, and a panic there is an abort with no window; it now reads `args_os()` lossily, so
  the mangled name is simply ignored. Only a Linux file manager can hand such a name over —
  Windows and macOS pass Unicode, and a test cannot inject argv. Proof: on a deb or rpm
  install, `touch $'caf\xe9.md'`, open it from the file manager, and the app starts. The file
  itself does not open; that is #25, kept under *Accepted limits* in `closed-items.md`.
- [ ] **Cmd+Q loses the last 500 ms of changes** (added 2026-09-25, split from the session
  restore limit; the kill and shutdown half is kept in `closed-items.md`). A report waits
  500 ms for things to settle, and since 21384f3 closing a window waits for it — but the
  predefined Quit (`Item::quit` in `macos_menu`) ends in `applicationWillTerminate`, which `tao`
  turns straight into `RunEvent::Exit` with nothing to hold it. Fix: a Quit item of our own on
  Cmd+Q that closes every window, so each waits for its report and the app exits with the
  last; the session code already reads a quit as windows closing one after another. Dock →
  Quit and logout stay on the terminate path. Reopen on a Mac, and prove it there: scroll, then
  Cmd+Q within 500 ms, and the next launch comes back at the scroll, every window in order.

## Accepted limits

Not bugs to fix; here so nobody rediscovers them. The three the 2026-09-12 review parked were
all lifted or retired on 2026-09-25 (`closed-items.md`). A limit the owner rules to keep moves
to `closed-items.md`, under *Accepted limits*. None is waiting for a ruling: every limit was
ruled on 2026-09-25.
