# IPC connect-src Plan

> **Status (2026-10-30):** executed in full: 5000811 adds the one `connect-src` line (gates
> passed, 215 tests). W1–W4 passed on Windows, macOS and Linux (Linux on headless Xvfb), and W5
> ran on Windows. W1: before the fix, calls went through the fallback on all three, marked
> `customProtocolIpcBlocked: true` on macOS and Linux. After it, every call is a fetch, to
> `http://ipc.localhost` on Windows and `ipc://localhost` on macOS and Linux, and none go
> through postMessage. Both planned signals needed adjusting:
> - WebKit's resource timing lists nothing at all.
> - The fallback isn't `window.ipc.postMessage`: it's `chrome.webview.postMessage` on Windows
>   and `window.webkit.messageHandlers.ipc.postMessage` on macOS and Linux, where `window.ipc`
>   is frozen.
>
> The walks counted those, and `fetch`. W2: no CSP error or fallback warning. W3: the four
> Libron faces load, and diagrams draw in them. W5: `load_file` of `closed-items.md` went from a
> median of 29.4–30.25 ms to 22.55–23.2 ms (debug builds). That's about 7 ms, too little to
> notice, so the owner kept the `chore:` subject. On macOS the settings button ignored the
> first click or two after a launch from a terminal, on both builds: a background window's
> activating click. The owner ruled no entry.

**Goal:** the page reaches the app over Tauri's IPC protocol, as Tauri intends, instead of
falling back to postMessage at every launch, and the launch console no longer shows the CSP
errors that fallback leaves.

**Spec:** the owner's ruling (2026-10-29) on the PR #18 Windows walk's console finding: fix it
now, separately from PR #18.

## Findings (2026-10-29, before this plan)

- **What happens (measured on Windows, PR #18 walk):** at launch the console shows "IPC custom
  protocol failed, Tauri will now use the postMessage interface instead (TypeError: Failed to
  fetch)" and two CSP errors: the fetch of `http://ipc.localhost/get_settings` is blocked, since
  `default-src 'self'` applies and `connect-src` isn't set. Every call after that goes through
  postMessage. Everything works.
- **Not new:** the same two lines are in earlier walks, on older tauri
  (`archive/plans/mermaid-follow-ups.md`, September 2026, ruled known noise then). The CSP has
  had no `connect-src` since the first commit (`d33ee3e`). Tauri 2.12.2 → 2.12.5 changes no CSP
  code (read in the diff).
- **Why:** Tauri doesn't add its IPC origins to the CSP itself. Its own examples set
  `connect-src ipc: http://ipc.localhost`. The IPC URL is `ipc://localhost` on macOS and Linux,
  and `http://ipc.localhost` on Windows (`useHttpsScheme` is unset, as `http://asset.localhost`
  in the same CSP shows).
- **`'self'` must stay:** the page fetches its own fonts (`app.js:664`, `fonts/libron/…`)
  through `connect-src`, which today falls back to `default-src 'self'`. A `connect-src` without
  `'self'` would block them.
- **Security:** no wider than today. The page's own scripts already make every IPC call, through
  postMessage. A document can't add script (`script-src 'self'`, comrak's `unsafe_` off), so no
  new caller gets in. Only the transport changes.
- **Not measured:** whether macOS and Linux show the same lines (their walk loggers started after
  launch; reasoned from the CSP, which is the same on all three), and whether the IPC protocol
  is faster for this app's large payloads (Task 2 measures it).

## Global constraints

- One line changed in `src-tauri/tauri.conf.json`. Nothing else.
- Gates, from the repo root, before the commit: `cargo fmt --manifest-path src-tauri/Cargo.toml
  --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked
  -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`,
  `node --check src/app.js`.
- Straight onto `main` after the PR #18 commit, committed locally; no push without the owner's go.
  No AI attribution.
- Walks run on the peers from a pushed `walk/*` branch, never on the host PC. Pushing it needs
  the owner's go at the time. Each peer backs up and restores `config.json` and `session.json`
  per the drive-app skill.

## Task 1: add connect-src

In `src-tauri/tauri.conf.json`, `app.security.csp`, after `default-src 'self';` add:

```
connect-src 'self' ipc: http://ipc.localhost;
```

Gates. Commit (subject: see Task 3).

## Task 2: walks

On a `walk/ipc` branch holding Task 1, debug build, all three platforms. Each step runs on
`main`'s build too (before) where it says so, to show the difference.

- **W1, the IPC path is used (before and after):** two signals, both read after launch, so no
  logger is needed at launch:
  - `performance.getEntriesByType("resource")`: entries whose name starts with `ipc://` or
    `http://ipc.localhost`, for example `…/get_settings`. Before: expected none. After:
    expected one per call. Unverified: whether WebKit lists custom-scheme fetches there.
  - Wrap `window.ipc.postMessage` (wry's postMessage channel) with a counter, then open
    Settings. Before: expected to count each call. After: expected zero. Unverified: whether
    Tauri looks the function up on each call or keeps its own reference.

  The before run calibrates them: a signal that doesn't differ before and after on a platform
  tells nothing there. Report both raw. Stop only if neither shows the change.
- **W2, no CSP errors (after):** on Windows, with the CDP logger attached from launch, the
  console shows no "IPC custom protocol failed" warning and no CSP error. On macOS and Linux,
  `securitypolicyviolation` events after launch: none.
- **W3, fonts still load (after):** `document.fonts` reports the four Libron faces loaded, and
  a diagram in `examples/kitchen-sink.md` draws with them (the diagram frame gets the font bytes
  from that fetch).
- **W4, the app works (after):** `examples/kitchen-sink.md` renders and a ticked task writes to
  the file (restore it after); an image opens through *Open file*; Settings › *Check now* reads
  "You are up to date."
- **W5, cost (Windows, before and after):** in the page, time
  `invoke("load_file", { path: <absolute path to docs/closed-items.md> })` ten times and report
  the median. That file renders to about 200 KB of HTML, the largest in the repo.

## Task 3: commit subject, and close out

The subject depends on W5, so it's set after the walks:

- If W5 shows a clear gain (median at least 20% lower), it's a release-note line:
  `Large documents open faster`, with the measured numbers in the body.
- Otherwise it's housekeeping: `chore: let the page reach the app over Tauri's IPC protocol`.

Amend Task 1's commit with the subject. Archive this plan to `docs/archive/plans/` with a status
line giving the commit and W1–W5's results, in a `docs:` commit, and an open-items entry if the
triage leaves anything.

## Execution

The main session runs Tasks 1 and 3 (one line, one commit) and sends Task 2 to the peers.
Serial. Stop and ask on:

- Neither W1 signal showing the change on a platform, or W2 showing any CSP error.
- W3 showing a font that didn't load.
- Any gate failing, or any smoke failure.
