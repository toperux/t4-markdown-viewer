# Diagrams after unclean exits on Windows

> **Status (2026-10-10):** executed in full and shipped in 1.8.0 as 1af0fb9. Paths and
> unticked boxes below are as they stood when it was written.

> 2026-10-10, from the find change review's Windows walk (triage item 1: the owner ruled
> "investigate first"; find's packaging waits on this). D1-D3 ruled by the owner 2026-10-10
> (the recommended option in each); D4 is asked at packaging.

**Goal:** diagrams keep drawing on Windows whatever WebView2's crash streak, in every window.

**Architecture:** turn Chromium's `IsolateSandboxedIframes` off with WebView2 browser arguments,
in the config's `main` window and in the Rust builder for every other window, from one string.

**Tech stack:** Tauri 2.12.1, wry 0.57.0, WebView2 154.

## What the user sees

Every Mermaid diagram stays a code block with "The diagram renderer did not load." It lasts
until the app has exited cleanly enough times (how many is not known).

## What was measured (Windows VM, WebView2 154.0.4258.62, 2026-10-09/10)

- The diagram frame is a sandboxed `srcdoc` iframe (`sandbox="allow-scripts"`, origin null),
  made by `loadRenderer()` in `src/app.js`. It loads `vendor/mermaid.min.js` and
  `diagram-frame.js` from `http://tauri.localhost/`.
- When failing, the frame runs as its own process (its own CDP target, `about:srcdoc`) and both
  script loads fail with `net::ERR_CONNECTION_REFUSED` (type "Other", no CSP or CORS block).
  The page's own `fetch` of the same files still works (200).
- `--enable-features=IsolateSandboxedIframes` breaks the installed release 1.7.4 the same way;
  `--disable-features=IsolateSandboxedIframes` makes a failing build draw.
- The trigger is the crash streak (`variations_crash_streak` in the profile's
  `EBWebView\Variations` file, mirrored in `Local State`). Every unclean exit (crash, killed
  process) adds one. On one profile copy: streak 0-3 draws, 4-7 fails. The installed 1.7.4
  failed 4/4 at streak 9 on its own profile with no flag. The walks' forced kills raised the
  VM's streaks to 9 and 19-20.
- wry already registers its request filter with `COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL`
  (`wry-0.57.0/src/webview2/mod.rs:1013`), so iframe requests are meant to be served; the
  isolated sandboxed frame's are not.

Reasoned, not proven:

- From streak 4 the variations service falls back and drops Edge's server config, which keeps
  `IsolateSandboxedIframes` off for WebView2; Chromium's own default is on.
- The isolated frame's requests don't reach Tauri's `WebResourceRequested` handler (a WebView2
  limitation or bug).

Not known:

- How a real user's streak comes back down. On the VM, clean closes left it as it was; only an
  edit of the file reset it.
- Whether Edge will turn the feature on for WebView2 by server config one day, for everyone.
  `--disable-features` on the command line overrides field trials (Chromium's rule; not
  measured here).

## Rejected (measured on the VM)

- **Inline scripts in the `srcdoc`:** the frame inherits the page's CSP (`script-src 'self'`
  plus Tauri's hashes of `index.html`'s inline scripts), so an inline mermaid is blocked.
- **A `blob:` URL from the page:** the opaque-origin frame may not load it ("Not allowed to load
  local resource"), and `script-src` has no `blob:` anyway.

## Decisions (for the owner)

- **D1. The fix:** `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,IsolateSandboxedIframes`
  as WebView2 browser arguments. Setting arguments replaces wry's defaults (the first three,
  `wry-0.57.0/src/webview2/mod.rs:297`), so they are kept. Not set by wry: autoplay and proxy
  arguments, which this app doesn't use. Cost: sandboxed iframes share the page's process, as
  they did before; the frame is still sandboxed (origin null, scripts only). Recommended.
- **D2. Every window gets the same string.** WebView2 refuses a second environment with
  different options on one user data folder (measured: a second window failed with
  `HRESULT(0x8007139F)` when only `main` had the argument). One source: the config's `main`
  window; `spawn_window` reads it from `app.config()`. Recommended over a Rust constant in two
  places.
- **D3. Not reported upstream** for now: an isolated sandboxed iframe's requests to a custom
  protocol go unanswered, a WebView2 or wry bug. An open item records that the fix depends on
  the flag (Task 4).
- **D4. Release:** shipped 1.7.4 is affected on any Windows machine past 4 unclean exits in a
  row. Whether this goes out in its own patch release before find is the owner's call, asked
  at packaging.

## Global Constraints

- Windows only in effect; the builder method compiles on every platform (not `cfg`-gated in
  `tauri-2.12.1/src/webview/webview_window.rs:1080`) and is ignored off Windows.
- The three wry defaults stay in the string, exactly.
- `cargo fmt`, `cargo test`; LF line endings; commit straight to main; walks on the Windows VM
  only (never the owner's PC), closing the app gracefully, not with `taskkill /F`.

## Review Focus

1. A profile at crash streak 4 or more: diagrams draw in `main`.
2. A second window (Ctrl+N, a tear-off, a restored session's other windows) opens and draws at
   a high streak.
3. Streak 0: nothing changes. The app has no PDF links or webview downloads (the updater
   downloads in Rust), so the three defaults' features have nothing to touch either way; a walk
   of the usual pages (Markdown, JSON, a picture, links opening in the browser) is enough.
4. The browser command line carries one `--disable-features` with all four names, in every
   window.
5. Linux and the Mac: no change (the argument is ignored).

## Tasks

### Task 1: the arguments, in the config and in every window

**Files:** `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`.

- `tauri.conf.json` › `app.windows[0]`: add
  `"additionalBrowserArgs": "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,IsolateSandboxedIframes"`.
- `spawn_window` (`main.rs`, the `WebviewWindowBuilder::new` at about line 412): pass the same
  string, read once from the config:

  ```rust
  // The same WebView2 arguments as `main`: WebView2 refuses a second set of options on one
  // data folder, and the window would fail to open.
  let args = app
      .config()
      .app
      .windows
      .first()
      .and_then(|w| w.additional_browser_args.clone());
  ```

  and, on the builder, `if let Some(a) = &args { b = b.additional_browser_args(a) }` (rebinding
  the builder, as the chain allows).
- A unit test in `main.rs` (or `config.rs`'s test module) reading `tauri.conf.json` with
  `include_str!` and `serde_json`: the first window's `additionalBrowserArgs` contains each of
  `msWebOOUI`, `msPdfOOUI`, `msSmartScreenProtection` and `IsolateSandboxedIframes`, in one
  `--disable-features=`. It fails if someone drops a default or the fix.
- JSON takes no comment, so the test's doc comment says why the string is there.
- Routing tests (`routing_tests.rs`) build windows on the mock runtime, where the argument is
  stored and ignored; no change there.
- [ ] **Commit:** "Diagrams draw on Windows after the app has crashed or been killed several
  times" (a fix; the owner may retitle).

### Task 2: drive-app note

**Files:** `.claude/skills/drive-app/SKILL.md`.

- Under *Clean up*: close the app gracefully (`appWindow.close()` from CDP, or the window's
  ×) rather than `taskkill //F` where possible. Each forced kill adds to WebView2's crash
  streak, and from 4 in a row diagrams stop drawing (without this fix), so a walk can fail for
  a reason that isn't the change.
- Under *Other gotchas*: reading and resetting the streak (`EBWebView\Variations` and
  `Local State`, `variations_crash_streak`), with the app closed.
- [ ] **Commit:** `docs:`.

### Task 3: walk on the Windows VM

- The code reaches the VM on a throwaway walk branch (a push the owner approves at the time).
  The VM's auto mode refused a build with this argument as security-weakening during the probes;
  the brief carries the owner's go for it.
- Build the `.audit` debug app from that branch.
- On a copy of the `.audit` profile with `variations_crash_streak` set to 6 (in both
  `Variations` and `Local State`, app closed): diagrams draw in `main`; Ctrl+N opens a second
  window that draws; a session with two windows restores both, both drawing.
- At streak 0: the same, plus a JSON file, a picture and an external link behave as on 1.7.4.
- A tear-off (a tab dragged out of the strip) opens a window that draws: every extra window
  comes from `spawn_window` (five callers), so this covers restores and Ctrl+N too.
- The browser command line (from CDP `SystemInfo` or the process list) shows the one
  `--disable-features` list.
- The VM's own profiles: put the streak back as found.

### Task 4: records

- `docs/closed-items.md` › Bugs: the failure, the cause and the fix, with the measurements.
- `docs/open-items.md` › Accepted limits (D3): "Diagrams on Windows depend on turning
  `IsolateSandboxedIframes` off." An isolated sandboxed frame's requests to the app's files go
  unanswered (WebView2 154; wry already asks for every request source). *Accepted 2026-10-10.
  Reopen if diagrams break on Windows again, if a WebView2 release drops the feature name, or
  when wry or WebView2 serves such frames.*
- [ ] **Commit:** `docs:`.

## Execution

Main session orchestrates; Task 1 to a `coder` (Opus), gated by `cargo fmt` and `cargo test`;
Tasks 2 and 4 are the main session's; Task 3 on the Windows VM. Then the change review loop.
