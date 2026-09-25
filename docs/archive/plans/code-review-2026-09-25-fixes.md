# Code Review 2026-09-25 Fixes Implementation Plan

**Status (2026-09-25):** executed — Tasks 1–16 committed, squashed into ca77ec0 … 486050b and the docs commit, smoke pass 11/11;
Task 15's owner steps and dry run pending (tracked in open-items).

> **For agentic workers:** Execute per the **Execution** section below — it says who runs each task, in what order, and where to stop for the owner. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix every finding of `docs/reviews/code-review-2026-09-25.md` that was ruled fix, with the revised fixes from its **Fix review** section, so the codebase is a clean baseline before the next features — without any fix introducing a new problem.

**Architecture:** Seventeen tasks, one commit each, grouped so that each commit is one coherent change a reader of the release notes would recognise. Order follows the code's dependencies: the page's navigation guard first, then task ticks (whose stamp other tasks rely on), rendering bounds, JSON, themes, the async move off the main thread, windows, tree, reading positions, updates, pointer capture, then vendored code, packaging, CI and docs.

**Tech Stack:** Rust + Tauri 2.11.6 (comrak 0.55.0, notify 8, tauri-plugin-updater), vanilla JS (`src/app.js`, no bundler, no frontend tests), NSIS, GitHub Actions. `drive-app` for anything only a running window shows.

**Spec:** `docs/reviews/code-review-2026-09-25.md` — the High/Medium/Low tables for *what*, the **Fix review** section for *how* (it supersedes the **Fix:** sketches in the rows), and **Decisions (owner, 2026-09-25)**.

## Findings this plan rests on (measured 2026-09-25)

- #1 and #2 driven on the debug build: an `href=""` link click and a real OS Ctrl+R with Settings open each reloaded the page (`tabs` → `[]`).
- #4: comrak `=0.55.0` with the repo's options — 2000×2000 cells (14 KB) → 160 MB HTML in 4.1 s. #5: `# a` × 20k → 10.6 s. #6: `T\n: d\n\n` × 20k → 2.9 s.
- #8: `std::fs::canonicalize` on a file of an unreachable host → 21.05 s first call, 61 µs second.
- #51: 4 MB of `*a* ` → 1.06 GB peak working set; 4 MB of plain words → 390 MB.
- The vetting harnesses: `openFolder` in node (#17: 9/9 with the revision), the render harness (2015 real `.md` files byte-identical, 800k fuzz documents clean), `onKeydown` over every reload spelling (#2).

## Global Constraints

- Surgical changes only. Match the surrounding comment style (full-sentence "why" comments) and naming.
- No new dependencies (Rust or JS).
- Run `cargo fmt --manifest-path src-tauri/Cargo.toml` after writing Rust: several snippets below are wider than rustfmt allows.
- Gates, all from the repo root, all must pass before every commit:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --check` (CI runs it on Linux only — run it every time)
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`
  - `node --check src/app.js`
- Commit straight onto `main`, locally. **Never push** — Task 15's dry run waits for the owner. User-facing subjects have no prefix; `ci:`, `chore:`, `docs:` otherwise. End every message with the executing model's `Co-Authored-By:` line.
- Bash on Windows: no `cd` in a compound command that writes; `--manifest-path` / `git -C`; `rtk proxy` for raw output.
- Driving: build with `TAURI_CONFIG='{"identifier":"com.montevirgen.t4-markdown-viewer.audit"}'`, CDP on 9223, back up and restore `config.json` and `session.json` (compare with `cmp`), wait for the app's exit by the child's `exitCode`, never `tasklist`. The owner's installed viewer stays open. CDP key events skip WebView2's accelerators; reload keys need a real OS keystroke (`scratchpad/sendkeys.ps1` pattern: `WScript.Shell` `AppActivate` + `SendKeys`).
- A drive step that fails means the task is not done: fix, re-gate, re-drive. Stop and ask the owner only where the task says so.

## Execution

| Phase | Tasks | Who |
|---|---|---|
| 0 | Commit this plan (`docs: plan the 2026-09-25 review fixes`) | main session |
| 1 | Tasks 1–14, 16 in order, each with its tests and drive steps | one `coder` subagent (Opus) per task; the main session reads its diff and gate output, then commits |
| 2 | Task 15 (CI) committed locally; its dry run waits for the owner's push and GitHub steps | `coder`, then owner |
| 3 | Task 17 smoke pass and close-out | main session |

Serial: a later task reads what an earlier one left. A `coder` brief is the task's text plus the Global Constraints; it reports gate output and every drive result, and stops at the task's stop points rather than choosing for the owner.

## File Map

| File | Tasks |
|---|---|
| `src/app.js` | 1, 2, 3, 5, 7, 9, 10, 11, 12 |
| `src-tauri/src/main.rs` | 1, 2, 4, 7, 8 |
| `src-tauri/src/render.rs` | 2, 4 |
| `src-tauri/src/json.rs` | 5 |
| `src-tauri/src/themes.rs` | 6 |
| `src-tauri/src/update.rs` | 11 |
| `src/base.css`, `src-tauri/themes/solarized-dark.css`, `src-tauri/themes/README.md`, `src-tauri/themes/_template.css` | 11 |
| `src/vendor/highlight.min.js` | 13 |
| `src-tauri/windows/hooks.nsi`, `src-tauri/tauri.conf.json` | 14 |
| `.github/workflows/release.yml`, `.claude/skills/release/SKILL.md` | 15 |
| `README.md`, `src-tauri/THIRD-PARTY-LICENSES.md` | 13, 16 |
| `.claude/skills/drive-app/SKILL.md`, `docs/*` | 17 |

---

### Task 1: A link or a key never reloads the page (#1, #2, #10)

**Files:** `src/app.js` (`onLinkClick` ~2405, `go` ~1656, `onKeydown` ~2680–2790), `src-tauri/src/main.rs` (builder, after the updater plugin)

- [ ] **Step 1: `onLinkClick`.** Replace its two lines `const href = a.getAttribute("href");` and `if (!href) return;` (~2418–2419) with:

```js
  const href = a.getAttribute("href");
  // Nothing to follow: an emptied link — comrak writes `href=""` for a
  // `file:`, `javascript:` or `data:` link — or a page that is not yet the
  // active tab's, a switch or a load still under way. Either way the webview
  // must not follow it on its own: following `""` reloads the page, and every
  // tab in the window goes with it.
  if (!href || shownToken !== renderToken) {
    event.preventDefault();
    return;
  }
```

- [ ] **Step 2: `go`.** The same-path shortcut only while the page on screen is the active tab's: `if (from && to && samePath(from.path, to.path) && shownToken === renderToken) {`.

- [ ] **Step 3: `onKeydown`.** Replace the F5 block and the modal guard (from `if (event.key === "F5") {` through `if (els.settings.open || els.updateDialog.open) return;`) with the block below, update the comment above it (Ctrl+R is now handled here, not "below"), delete the later `const ctrl = event.ctrlKey || event.metaKey;` and the `} else if (key === "r") { … refresh(); }` branch:

```js
  const ctrl = event.ctrlKey || event.metaKey;
  // `keyCode` too: WebView2's accelerator reads the virtual key, so Ctrl+К on
  // a Cyrillic layout reloads although `key` is "к". Not `code`: on Dvorak
  // `KeyR` is where P is.
  if (
    event.key === "F5" ||
    event.key === "BrowserRefresh" ||
    (ctrl && !event.altKey && (event.key.toLowerCase() === "r" || event.keyCode === 82))
  ) {
    event.preventDefault();
    refresh();
    return;
  }

  // Otherwise a dialog is modal: let it own the keyboard, Escape included —
  // but not WebView2's Back and Forward, which would walk the `#id` history
  // under it.
  if (els.settings.open || els.updateDialog.open) {
    if (event.altKey && (event.key === "ArrowLeft" || event.key === "ArrowRight")) event.preventDefault();
    return;
  }
```

- [ ] **Step 4: The guard.** In `main()`'s builder, directly after `.plugin(tauri_plugin_updater::Builder::new().build())` (single-instance stays first). A booted page never goes anywhere else; a reload of it comes back with the window's tabs. Measured on the debug build: after a renderer crash (CDP `Page.crash`) WebView2 shows "This page is having a problem" with a Refresh button, and that Refresh reaches this hook as a navigation to `http://tauri.localhost/` from a ready label — today it boots with 0 tabs.

```rust
        // A page that has booted never navigates anywhere else, and a reload of
        // it — an accelerator past the page's handler, the webview's own
        // context-menu Reload, the Refresh on its "page is having a problem"
        // screen after a crash — would boot on a pending slot `take_pending`
        // has drained, and every tab in the window would be gone. So a reload
        // is handed what the window last reported, as a restart is. Before
        // `take_pending` it is the window's own first load; the URL cannot
        // tell the two apart, so readiness does.
        .plugin(
            tauri::plugin::Builder::<tauri::Wry>::new("stay")
                .on_navigation(|webview, url| {
                    // `cargo tauri dev` serves the page itself and reloads it on every edit.
                    if cfg!(dev) && url.port().is_some() {
                        return true;
                    }
                    // WebKit asks about same-document `#id` jumps too; WebView2 does not.
                    if !cfg!(windows) && url.fragment().is_some() {
                        return true;
                    }
                    let state = webview.state::<AppState>();
                    let label = webview.label();
                    let mut boot = state.boot.lock().unwrap();
                    if !boot.ready.contains(label) {
                        return true;
                    }
                    // The app's own page and nothing else: `tauri://localhost/`
                    // on macOS and Linux, `http://tauri.localhost/` on Windows.
                    let own = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                        || url.host_str() == Some("tauri.localhost");
                    if !own || url.path() != "/" {
                        return false;
                    }
                    // Not through `claim_in`: that marks the window as restoring,
                    // which is the crash-loop guard for a restore from disk.
                    // Its report in `sessions` stands until the new page reports.
                    let open = state.sessions.lock().unwrap().get(label).cloned();
                    boot.ready.remove(label);
                    if let Some(open) = open {
                        let payload = json!({ "kind": "session", "tabs": open.tabs, "active": open.active });
                        boot.pending.insert(label.to_string(), payload);
                    }
                    true
                })
                .build(),
        )
```

Check before writing: `OpenTabs` is `Clone` and `sessions` is keyed by label (it is, per `claim_in`); the page's `pending?.kind === "session"` branch (~3126) calls nothing that assumes a restore from disk — e.g. a `session::restored` call is harmless here (`restoring.remove` finds nothing, so no `discard`), but read it. `maximized` is left out, so the page leaves the window's size alone.

- [ ] **Step 5: Gates.** Then drive (one launch, `"reopen": "off"`, a document with `[a](file:///C:/Windows/win.ini)`, `[b]()`, `[in](#x)` and a heading `# X`, a second document `b.md` with `# Deep` far down, linked as `[deep](b.md#deep)`; two tabs; `window.__m = 1` before each check):
  - click `a[href=""]` → `__m` survives, `tabs.length` unchanged;
  - Settings open, real OS Ctrl+R (`sendkeys.ps1 ^r`) → survives; Settings open, after an anchor jump, real Alt+← (`sendkeys.ps1 %{LEFT}` — CDP keys skip the accelerator) → `location.hash` unchanged;
  - CDP `location.reload()` → the page reloads (`__m` gone) and comes back with the same tabs, the same active tab, and each tab's Back history;
  - crash: CDP `Page.crash` (raw command; `scratchpad/crash/cdpraw.mjs`), then a real click on the error page's Refresh (`scratchpad/crash/click.ps1`, window rect from `winshot.ps1`) → the window comes back with its tabs (before this task: 0 tabs);
  - `location.href = "http://tauri.localhost/x"` and `location.href = "https://example.com/"` → refused, `__m` survives;
  - **anchors on Windows:** click `[in](#x)` → lands on `#x`, `location.hash === "#x"`; click `[deep](b.md#deep)` → `b.md` loads and lands on `#deep`. **If either fails**, remove the `!cfg!(windows) &&` condition (so the fragment exemption applies everywhere), rebuild, re-drive;
  - Ctrl+N opens a window that boots and shows its document; a tab dragged out tears off into a booting window (cross-window recipe);
  - a load delayed with the fetch patch (`window.fetch` wrapper delaying `/load_file`), `cycleTab(1)` unawaited, then click a relative link on the old page → the incoming tab's `entries` unchanged, no "Not a file".
  - Known cost, noted in the guard's comment only: a reload comes back as the window last reported, so what changed in the moment since (a zoom, a tree expanded) is gone; and a document that itself crashes the renderer crashes it again on Refresh — closing the window is the way out, as today.
- [ ] **Step 6: Commit** `Keep a window's tabs when a link or a key would reload the page`.

### Task 2: A task tick is refused when the file changed since it was shown (#3, #27)

**Files:** `src-tauri/src/main.rs` (`Document`, `load_file`, `toggle_task`, `section_source`, tests), `src-tauri/src/render.rs` (`toggle_task`), `src/app.js` (`renderDocument`, `onTaskToggle`, `onCopySection`)

- [ ] **Step 1: Failing tests.** One crate, one test binary: add the `render.rs` test first and run it (FAIL), then the `main.rs` one (compile error until Step 3). In `main.rs` `mod tests` (`toggle_task` is callable directly):

```rust
    /// A tick is sent against the page it was clicked on. Once the file has
    /// moved on, the line it names may hold a different task, so it is refused
    /// and nothing is written; quick ticks chain, each on the stamp the last left.
    #[test]
    fn a_tick_on_a_page_older_than_the_file_is_refused() {
        let dir = std::env::temp_dir().join(format!("t4-stamp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("t.md");
        let path = file.to_string_lossy().into_owned();
        std::fs::write(&file, "- [ ] a\n- [ ] b\n").unwrap();
        let s0 = stamp_of(&std::fs::read(&file).unwrap());

        let s1 = toggle_task(path.clone(), 1, true, s0.clone()).unwrap();
        assert_eq!(s1, stamp_of(&std::fs::read(&file).unwrap()));
        let s2 = toggle_task(path.clone(), 2, true, s1).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "- [x] a\n- [x] b\n");

        std::fs::write(&file, "new\n- [ ] a\n- [ ] b\n").unwrap();
        assert!(toggle_task(path.clone(), 2, true, s2).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "new\n- [ ] a\n- [ ] b\n");

        std::fs::write(&file, "\u{feff}- [ ] a\n").unwrap();
        let s = stamp_of(&std::fs::read(&file).unwrap());
        let next = toggle_task(path.clone(), 1, true, s).unwrap();
        assert_eq!(next, stamp_of(&std::fs::read(&file).unwrap()));
        std::fs::remove_dir_all(&dir).unwrap();
    }
```

and in `render.rs` `mod tests`:

```rust
    /// comrak reports the box of an item that opens with a link reference
    /// definition at the definition's label. The byte there must agree with
    /// the box comrak parsed, and be followed by `]`, or the tick is refused.
    #[test]
    fn toggle_task_refuses_a_reference_label_for_the_box() {
        assert!(toggle_task("- [x]: /u\n  [ ] a\n", 1).is_err());
        assert!(toggle_task("- [X]: /u\n  [ ] a\n", 1).is_err());
        assert!(toggle_task("- [xy]: /u\n  [x] a\n", 1).is_err());
        assert!(toggle_task("- [X] a\n", 1).is_ok());
    }
```

Run both: expect compile failure / FAIL.

- [ ] **Step 2: `stamp_of`, `Document.stamp`.** In `main.rs`:

```rust
/// What a page was rendered from, so a tick or a copy can tell the file has
/// moved on since. Hex, because a `u64` does not survive a JavaScript number.
/// `DefaultHasher::new()` is fixed-key SipHash, not `RandomState`'s random
/// keys, so the same bytes stamp the same in every call.
fn stamp_of(bytes: &[u8]) -> String {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    h.write(bytes);
    format!("{:016x}", h.finish())
}
```

`Document` gains `stamp: String` (doc comment: the raw bytes' stamp, BOM included); `load_file` sets `stamp: stamp_of(&bytes)` from the bytes as read, before `decode`.

- [ ] **Step 3: `toggle_task`.** New signature and body (still synchronous here; Task 7 moves it off the main thread). Keep the current comment "Errors end up on screen, so name the file the way the user knows it." above `shown`, and amend the doc comment's "…so nothing is updated here" (~610) to say it returns the file's new stamp:

```rust
#[tauri::command]
fn toggle_task(path: String, line: usize, checked: bool, stamp: String) -> Result<String, String> {
    let (path, _) = locate(path)?;
    let shown = strip_unc(&path);
    let mut bytes = std::fs::read(&path).map_err(|e| format!("{shown}: {e}"))?;
    // The line was read off a page. If the file is not what that page was
    // made from, the line may name a different task now.
    if stamp_of(&bytes) != stamp {
        return Err(format!("{shown} changed on disk since it was shown; showing it again."));
    }
    let at = {
        let (bom, body) = match bytes.strip_prefix(render::BOM) {
            Some(b) => (render::BOM, b),
            None => (&[][..], &bytes[..]),
        };
        let text = std::str::from_utf8(body)
            .map_err(|_| format!("{shown}: not UTF-8, leaving it alone"))?;
        // The offset is into the text after the BOM, and the file still has it.
        bom.len() + render::toggle_task(text, line)?
    };
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| format!("{shown}: {e}"))?;
    write_box(&mut file, at as u64, checked).map_err(|e| format!("{shown}: {e}"))?;
    // The page's next tick is made against this. Wrong only if someone wrote
    // between the read and the write — then that tick is refused: safe.
    bytes[at] = if checked { b'x' } else { b' ' };
    Ok(stamp_of(&bytes))
}
```

- [ ] **Step 4: `section_source`** takes `stamp: String` and refuses on a mismatch the same way, before parsing; delete its comment's "if it changed in between…/Guard it by sending the heading text along" sentences, which this closes.

- [ ] **Step 5: `render::toggle_task` (#27).** Carry the parsed symbol and require agreement and a closing bracket:

```rust
    let (want, symbol) = root
        .descendants()
        .find_map(|node| {
            let data = node.data.borrow();
            match &data.value {
                NodeValue::TaskItem(item) if data.sourcepos.start.line == line => {
                    Some((item.symbol.map_or(b' ', |c| c as u8), item.symbol_sourcepos.start))
                }
                _ => None,
            }
        })
        .ok_or_else(|| format!("Line {line} is not a task item"))?;

    // Columns are 1-based bytes; the symbol is one ASCII byte between the
    // brackets. It must be the box comrak parsed: an item that opens with a
    // link reference definition reports the definition's label instead.
    let bytes = md.as_bytes();
    line_start(md, symbol.line)
        .map(|start| start + symbol.column - 1)
        .filter(|&at| bytes.get(at) == Some(&want) && bytes.get(at + 1) == Some(&b']'))
        .ok_or_else(|| format!("Line {line} does not hold the box comrak saw"))
```

Run the Step 1 tests: PASS.

- [ ] **Step 6: The page.** In `renderDocument`, next to where `els.content.dataset.path` is set: `els.content.dataset.stamp = doc.stamp ?? "";` (every document gets one; only Markdown pages have ticks and copy-section to use it). Replace `onTaskToggle`:

```js
/**
 * Ticks go one at a time, each against the stamp the last one left: two quick
 * ones must not both claim the render's. A tick the file has moved on from is
 * refused; the page then shows the file as it is — on a share that sends no
 * change notices nothing else would.
 */
let ticking = Promise.resolve();

function onTaskToggle(event) {
  const box = event.target;
  const li = box.closest("li[data-sourcepos]");
  if (box.type !== "checkbox" || !li) return;
  const line = Number(li.dataset.sourcepos.split(":")[0]);
  // The tab's entry moves on as soon as a navigation starts, before the new
  // page is on screen; the line number belongs to the document still in the
  // DOM, so take the path — and the stamp — from there too.
  const path = els.content.dataset.path;
  const checked = box.checked;
  ticking = ticking.then(async () => {
    // Re-rendered while it waited: the new page already shows the file as it is.
    if (!box.isConnected) return toast("The file changed; tick it again.");
    try {
      const next = await invoke("toggle_task", { path, line, checked, stamp: els.content.dataset.stamp });
      if (box.isConnected) els.content.dataset.stamp = next;
    } catch (err) {
      toast(err);
      if (box.isConnected) {
        box.checked = !checked;
        refresh().catch(console.error);
      }
    }
  });
}
```

In `onCopySection`, pass `stamp: els.content.dataset.stamp` to `section_source`, and refresh only when the backend refused — not on a plain clipboard failure:

```js
    .catch((err) =>
      md.then(
        () => toast(err),
        (e) => {
          toast(e);
          refresh().catch(console.error);
        },
      ),
    )
```

- [ ] **Step 7: Gates.** Drive: a document with two tasks — `boxes[0].click(); boxes[1].click()` in one evaluate → after 500 ms both `[x]` on disk; then `invoke("watch_files", { paths: [] })` (no notices), insert a line at the top of the file from the shell, click a box → a toast, the page refreshes showing the new line, no task changed on disk; copy-section on a heading still copies.
- [ ] **Step 8: Commit** `Refuse a task tick when the file has changed since it was shown`.

### Task 3: Focus stays on a task box ticked from the keyboard (#36)

**Files:** `src/app.js` (`renderDocument`)

- [ ] **Step 1:** At the top of `renderDocument`, before `els.content.innerHTML = doc.html`:

```js
  // A box ticked from the keyboard comes back as a new element when the
  // watcher re-renders; put focus back on it — but only if it is still the
  // same task, or the next Space ticks a neighbour. Only a keyboard focus
  // (`:focus-visible`): a clicked box is focused too, and bringing that back
  // would let a Space meant to scroll untick it. Focus in the sidebar or a
  // dialog stays where it is. Read before `dataset.path` is overwritten below.
  const f = document.activeElement;
  const ticked =
    f?.type === "checkbox" &&
    f.matches(":focus-visible") &&
    els.content.contains(f) &&
    doc.path === els.content.dataset.path
      ? f.closest("li[data-sourcepos]")
      : null;
  const refocus = ticked && { pos: ticked.dataset.sourcepos, text: ticked.textContent };
```

and after `show("content")` (before the scroll `requestAnimationFrame`):

```js
  if (refocus) {
    const again = els.content.querySelector(`li[data-sourcepos="${refocus.pos}"]`);
    if (again?.textContent === refocus.text)
      again.querySelector(':scope > input[type="checkbox"]')?.focus({ preventScroll: true });
  }
```

- [ ] **Step 2: Gates.** Drive: scroll mid-page so a box is in view, reach it with real Tab presses (`cdp key Tab 9` until it is focused — a scripted `focus()` may not count as `:focus-visible`), `cdp key " " 32`, wait 600 ms → `document.activeElement` is the checkbox in the same `li`, `scrollY` unchanged; tick another box with `cdp click` → after the re-render `document.activeElement === document.body`; insert a line above externally and wait → focus on `body`, not a neighbour; focus the tree filter, tick via eval → focus stays on the filter.
- [ ] **Step 3: Commit** `Keep focus on a task box ticked from the keyboard`.

### Task 4: Oversized or pathological Markdown renders without hanging or crashing (#51, #52, #4, #5, #6, #7, #28)

**Files:** `src-tauri/src/main.rs` (size caps, `load_file`, `toggle_task`, `section_source`), `src-tauri/src/render.rs`

- [ ] **Step 1: Caps (#51, #52).** Replace `MAX_DOCUMENT_BYTES` with two caps and give `check_size` the limit:

```rust
/// How big a JSON document `load_file` will read — see `json.rs`, which shows
/// it a chunk at a time. Images are deliberately not capped: `load_asset`
/// hands the webview a path rather than bytes, so a 40 MB photo costs nothing.
const MAX_JSON_BYTES: u64 = 32 * 1024 * 1024;

/// How big a document parsed as Markdown may be, `.txt` included. comrak's
/// tree costs hundreds of bytes per input byte on inline-heavy text — 4 MB of
/// `*a* ` peaked at about 1 GB — so this bounds a render at a size a machine
/// can afford, where 32 MB could take many gigabytes and the app with it.
const MAX_MARKDOWN_BYTES: u64 = 4 * 1024 * 1024;
```

`check_size(path, limit)` compares with `limit`; the file's size gets one decimal (`{:.1} MB`), the limit stays whole, so a 4.3 MB file reads "4.3 MB, and the limit is 4 MB" rather than "4 MB". (Just over 4 MiB still reads "4.0 MB" — acceptable. No test or JS reads this text.) Callers: `load_file` (`MAX_JSON_BYTES` when `is_json`, else `MAX_MARKDOWN_BYTES` — compute `is_json` before the check), `json_region` (`MAX_JSON_BYTES`), and — new, #52 — `toggle_task` and `section_source` (`MAX_MARKDOWN_BYTES`), each right after `locate`. Update `check_size`'s doc comment.

- [ ] **Step 2: Failing tests** in `render.rs` `mod tests` (they assume Steps 3–6; every direct `render_titled` call gains `.unwrap()` — the `render` helper and `render_titled_gives_the_page_and_its_title_from_one_parse`, twice; compare `scratchpad/vet-render/src/new.rs`, whose tests pass):

```rust
    #[test]
    fn tables_comrak_cannot_afford_read_as_text() {
        let bomb = |cols: usize, rows: usize| {
            format!("{}|\n{}|\n{}\n- [ ] t\n\n# H\n", "|a".repeat(cols), "|-".repeat(cols), "|x\n".repeat(rows))
        };
        let md = bomb(20_000, 20_000);
        assert!(!render(&md).contains("<table"));
        let line = 20_004;
        assert_eq!(flip(&md, line, true).unwrap(), md.replace("- [ ] t", "- [x] t"));
        assert_eq!(section(&md, line + 2).unwrap(), "# H\n");
        assert!(render(&bomb(3, 2)).contains("<table"));
        assert!(table_cost(&bomb(1_000, 499)).0 <= MAX_TABLE_PADDING);
        assert!(table_cost(&bomb(1_000, 501)).0 > MAX_TABLE_PADDING);
        let wide = format!("{}|\n{}|\n{}|\n", "|a".repeat(30_000), "|-".repeat(30_000), "|x".repeat(30_000));
        assert!(!render(&wide).contains("<table"));
    }

    #[test]
    fn table_cost_never_undercounts_padding() {
        for md in [
            "|a|b|c|\n|-|-|-|\nx\n|y\n",
            "|a|b|c|\r\n|-|-|-|\r\nx\r\n",
            "|a|b|c|\r|-|-|-|\rx\r",
            "> |a|b|c|\n> |-|-|-|\n> x\n",
            "- i\n\n  |a|b|c|\n  |-|-|-|\n  x\n",
            "a|b|c\n-\x0b|-|-\n\\|\\|x\n\\\\|\\\\|x\n",
        ] {
            let arena = Arena::new();
            let root = parse_document(&arena, md, &options(md));
            let padded = root
                .descendants()
                .filter(|n| {
                    let d = n.data.borrow();
                    matches!(d.value, NodeValue::TableCell)
                        && n.first_child().is_none()
                        && d.sourcepos.start.column == d.sourcepos.end.column
                })
                .count();
            assert!(padded > 0, "{md:?} is not a padded table");
            assert!(table_cost(md).0 >= padded, "{md:?}: {padded} padded, charged {}", table_cost(md).0);
        }
        assert_eq!(table_cost("| a | b |\n|---|---|\n| 1 | 2 |\n").0, 0);
    }

    #[test]
    fn repeated_headings_get_comraks_ids_quickly() {
        let md = "# a\n# a-2\n# a\n## A\n# a\n\n# *Emph* & `code`\nSetext\nheading\n===\n";
        assert_eq!(render(md), comrak::markdown_to_html(md, &options(md)));
        assert_eq!(render(KITCHEN_SINK), comrak::markdown_to_html(KITCHEN_SINK, &options(KITCHEN_SINK)));
        assert_eq!(render(EXAMPLE), comrak::markdown_to_html(EXAMPLE, &options(EXAMPLE)));
        let html = render(md);
        for id in ["\"a\"", "\"a-2\"", "\"a-1\"", "\"a-3\""] {
            assert!(html.contains(&format!("id={id}")), "{id}: {html}");
        }
        let t = std::time::Instant::now();
        let html = render(&"# a\n".repeat(20_000));
        assert!(html.contains("id=\"a-19999\""));
        assert!(t.elapsed().as_secs() < 5, "{:?}", t.elapsed());
    }

    #[test]
    fn a_description_list_past_the_limit_reads_as_text() {
        assert!(render("T\n: d\n").contains("<dl"));
        let md = "T\n: d\n\n".repeat(MAX_DETAILS + 1);
        assert!(!render(&md).contains("<dl"));
    }

    #[test]
    fn markdown_past_the_ceiling_is_refused() {
        let md = format!("{}\n\n[^1]: x\n", "[^1]".repeat(300_000));
        assert_eq!(render_titled(&md).unwrap_err(), TOO_MUCH);
    }

    #[test]
    fn first_heading_reads_a_line_break_as_a_space() {
        assert_eq!(first_heading("Release notes\nfor 2.0\n===\n"), Some("Release notes for 2.0".into()));
        assert_eq!(first_heading("a\\\nb\n===\n"), Some("a b".into()));
    }
```

`KITCHEN_SINK` is the existing inline const in `mod tests` — leave it as it is (`gfm_constructs_render` depends on its line numbers). Add beside it `const EXAMPLE: &str = include_str!("../../examples/kitchen-sink.md");` (path verified). Run: FAIL to compile.

- [ ] **Step 3: `options(md)` (#4, #6).** `options` takes the text it will parse, and every caller — `render_titled`, `section`, `toggle_task`, the anchor test (`options("")`) — passes the same text it parses, so a tick's line is matched against the parse that rendered it:

```rust
/// Past these, a document is parsed without tables or description lists and
/// those lines read as text — see `table_cost` and `details`. About a third of
/// a second each at the limit.
const MAX_TABLE_PADDING: usize = 500_000;
const MAX_TABLE_WORK: usize = 1_000_000_000;
const MAX_DETAILS: usize = 5_000;
```

In `options`: `let (padding, work) = table_cost(md); o.extension.table = padding <= MAX_TABLE_PADDING && work <= MAX_TABLE_WORK;` and `o.extension.description_lists = details(md) <= MAX_DETAILS;`. Add:

```rust
/// Lines as comrak counts them: `\n`, `\r\n` or a bare `\r` ends one.
fn lines(md: &str) -> impl Iterator<Item = &str> {
    md.split('\n').flat_map(|l| l.strip_suffix('\r').unwrap_or(l).split('\r'))
}

/// What comrak's tables would cost, at most: the empty cells it pads short
/// rows out with, and the square of each row's width, since it finds every
/// cell's column by walking the row from its start. A table runs from its
/// delimiter row to the next blank line and is as wide as that row, so each
/// line is charged the widest delimiter row seen since the last blank line.
/// Quote markers and indentation come off first; a line of `-|: ` that is not
/// a delimiter row only overcharges, never under.
fn table_cost(md: &str) -> (usize, usize) {
    let (mut padding, mut work, mut width) = (0usize, 0usize, 0usize);
    for line in lines(md) {
        if line.bytes().all(|b| matches!(b, b' ' | b'\t')) {
            width = 0;
            continue;
        }
        let body = line.trim_start_matches([' ', '\t', '>']);
        let cells = row_cells(body);
        if body.contains('-')
            && body.bytes().all(|b| matches!(b, b'|' | b'-' | b':' | b' ' | b'\t' | b'\x0b' | b'\x0c'))
        {
            width = width.max(cells);
        }
        padding += width.saturating_sub(cells);
        work += width * width;
    }
    (padding, work)
}

/// The fewest cells a table row makes of `line`: one after each `|` but a
/// leading one, and one more for anything after the last. Exact for a
/// delimiter row. comrak's cell scanner takes the longest match, so a `|`
/// straight after a `\` — `\\|` included — may be part of a cell and is not
/// counted, which only undercounts cells.
fn row_cells(line: &str) -> usize {
    let b = line.as_bytes();
    let pipes = (0..b.len()).filter(|&i| b[i] == b'|' && (i == 0 || b[i - 1] != b'\\')).count();
    let lead = usize::from(line.starts_with('|'));
    let tail = usize::from(!line.trim_end_matches([' ', '\t', '\x0b', '\x0c']).ends_with('|'));
    (pipes + tail).saturating_sub(lead).max(1)
}

/// Lines that could open a description's details: `:` or `~` then a space or
/// tab, once quote markers and indentation are off.
fn details(md: &str) -> usize {
    lines(md)
        .filter(|l| {
            let body = l.trim_start_matches([' ', '\t', '>']).as_bytes();
            matches!(body, [b':' | b'~', b' ' | b'\t', ..])
        })
        .count()
}
```

(`work` cannot overflow on 64-bit within 4 MB of input; add a `saturating_mul`/`saturating_add` anyway if clippy or review prefers.)

- [ ] **Step 4: Heading ids in linear time (#5).** A `HeadingAdapter` that writes comrak's exact markup with a per-slug next-suffix memo:

The file's imports (lines 1–2 now) become — `format_html` goes, or clippy `-D warnings` fails on the unused import:

```rust
use comrak::adapters::{HeadingAdapter, HeadingMeta};
use comrak::nodes::{AstNode, NodeValue, Sourcepos};
use comrak::options::Plugins;
use comrak::{format_html_with_plugins, parse_document, Anchorizer, Arena, Options};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Mutex;
```

```rust

/// comrak's heading ids, made in linear time. Its own anchorizer tries `-1`,
/// `-2`, … from 1 again for every repeat of a slug, so 20,000 identical
/// headings took 12 s; this remembers where each slug got to. Every suffix
/// below that was taken then and still is — nothing is ever given back — so
/// the ids are the ones comrak would have given.
#[derive(Default)]
struct HeadingIds(Mutex<Ids>);

#[derive(Default)]
struct Ids {
    taken: HashSet<String>,
    next: HashMap<String, usize>,
    open: String,
}

impl HeadingAdapter for HeadingIds {
    fn enter(&self, out: &mut dyn fmt::Write, heading: &HeadingMeta, sourcepos: Option<Sourcepos>) -> fmt::Result {
        let ids = &mut *self.0.lock().unwrap();
        // A fresh anchorizer has nothing to avoid, so this is the bare slug.
        let slug = Anchorizer::new().anchorize(&heading.content);
        let mut n = ids.next.get(&slug).copied().unwrap_or(0);
        let id = loop {
            let id = if n == 0 { slug.clone() } else { format!("{slug}-{n}") };
            if !ids.taken.contains(&id) {
                break id;
            }
            n += 1;
        };
        ids.next.insert(slug, n);
        ids.taken.insert(id.clone());
        write!(out, "<h{} id=\"{id}\"", heading.level)?;
        if let Some(sp) = sourcepos.filter(|sp| sp.start.line > 0) {
            write!(out, " data-sourcepos=\"{sp}\"")?;
        }
        out.write_str(">")?;
        ids.open = id;
        Ok(())
    }

    fn exit(&self, out: &mut dyn fmt::Write, heading: &HeadingMeta) -> fmt::Result {
        let id = &self.0.lock().unwrap().open;
        write!(out, "<a href=\"#{id}\" aria-label=\"Link to heading '")?;
        comrak::html::escape(out, &heading.content)?;
        out.write_str("'\" data-heading-content=\"")?;
        comrak::html::escape(out, &heading.content)?;
        // comrak's adapter path skips the newline its own path writes after `</hN>`.
        writeln!(out, "\" class=\"anchor\"></a></h{}>", heading.level)
    }
}
```

The `repeated_headings_get_comraks_ids_quickly` test's identity checks against `markdown_to_html` are what prove the markup is comrak's; they also catch drift on a comrak bump.

- [ ] **Step 5: A ceiling on the HTML (#7).**

```rust
/// The most HTML one Markdown render hands the webview — the same ceiling JSON has.
pub const MAX_HTML_BYTES: usize = 64 * 1024 * 1024;
pub const TOO_MUCH: &str = "This document is too much to show: rendered, it comes to more than 64 MB.";

/// A `String` that refuses to grow past the ceiling: comrak passes the error
/// up rather than building the rest.
struct Capped(String);

impl fmt::Write for Capped {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.0.len() + s.len() > MAX_HTML_BYTES {
            return Err(fmt::Error);
        }
        self.0.push_str(s);
        Ok(())
    }
}
```

`html_of(root, &o) -> Result<String, String>`: set up `HeadingIds` in `Plugins` (`plugins.render.heading_adapter = Some(&ids)`), format with `format_html_with_plugins(root, o, &mut html, &plugins).map_err(|_| TOO_MUCH.to_string())?` into `Capped`, run anchor recovery unchanged on `html.0`, then `if out.len() > MAX_HTML_BYTES { return Err(TOO_MUCH.to_string()); }` — recovery can grow it. `render_titled(md) -> Result<(String, Option<String>), String>` builds `options(md)` once and uses it for parse and format. In `main.rs`, `load_file` uses `render::render_titled(&text)?`.

- [ ] **Step 6: Title (#28).** In `title_of`, replace the manual loop with `let text = node.collect_text(); let text = text.trim();`, keeping the comment's point about alt text (comrak's `collect_text` includes it and turns soft and hard breaks into spaces).

- [ ] **Step 7: Gates.** Run the Step 2 tests: PASS, and every existing test. Harness check (the Findings drivers, rebuilt against the new `render.rs` or via `cargo test --release` timings): the 20000×20000 bomb, `# a` × 20k, `T\n: d\n\n` × 20k each under a second. Byte-identity: render every `.md` in the repo and `examples/` with the old and the new code (a throwaway test or scratch binary) → HTML identical except titles fixed by #28. Drive: open `examples/kitchen-sink.md` (or the largest example) → renders, anchors and copy-section work; a `.md` of exactly 5 MiB (5,242,880 bytes) → "…5.0 MB, and the limit is 4 MB."; a 5 MiB `.json` still opens.
- [ ] **Step 8: Commit** `Render oversized or pathological Markdown without hanging or crashing`.

### Task 5: A big JSON document is shown flat before it is refused, and keeps what was expanded (#29, #30, #31)

**Files:** `src-tauri/src/json.rs` (`emit_within`, `TOO_MUCH`, tests), `src/app.js` (`more` handler ~2554)

- [ ] **Step 1: Test** in `json.rs`: `assert!(render_within(&"[1]\n".repeat(393_000), 0, MAX_HTML_BYTES).is_ok_and(|h| !h.contains("class=\"fold")));` (adapt to `render_within`'s real signature and constant names). Run: FAIL.
- [ ] **Step 2: #29.** `emit_as(src, base, budget, ceiling, Some(depth)).filter(|h| h.len() <= ceiling).or_else(|| emit_as(src, base, budget, ceiling, None)).unwrap_or_default()`; update `emit_within`'s doc comment (a folded render over the ceiling is tried flat).
- [ ] **Step 3: #30.** `TOO_MUCH` becomes "This is too much to show at once: rendered, it comes to more than 64 MB with nowhere to split it — values with no comma between them, one very long value, or brackets nested thousands deep."
- [ ] **Step 4: #31.** Seed the `reduce` with `Number.MAX_SAFE_INTEGER` instead of `end`, and rewrite the comment above it: with no button left the whole document is loaded, and `render_within` clamps the extent to `MAX_EXTENT`; never `Infinity`, which `JSON.stringify` turns into `null` — extent 0, chunk one. Leave `end` if still used; delete it if not.
- [ ] **Step 5: Gates.** Drive: a 2–8 MB JSON file whose first cut leaves at least two `more` buttons; click the outer (last) one, then the inner → `currentEntry(activeTab()).extent === Number.MAX_SAFE_INTEGER`; `await refresh()` → no `button.more` left.
- [ ] **Step 6: Commit** `Show a big JSON document flat before refusing it, and keep what was expanded`.

### Task 6: A user theme is read safely, whatever its text (#32, #33)

**Files:** `src-tauri/src/themes.rs` (~156, `read` ~328, `scan`)

- [ ] **Step 1: Test:**

```rust
    #[test]
    fn color_scheme_scan_is_bounded_and_char_safe() {
        assert_eq!(mode_from_css(&format!(":root {{ color-scheme: {} dark }}", "é".repeat(42))), Some(Mode::Dark));
        assert_eq!(mode_from_css(&format!(":root {{ color-scheme:\n{}dark; }}", " ".repeat(90))), Some(Mode::Dark));
        let t = std::time::Instant::now();
        mode_from_css(&format!(":root{{{}", "color-scheme:".repeat(5_000)));
        assert!(t.elapsed().as_secs() < 2, "{:?}", t.elapsed());
    }

    /// A theme saved in a code page, or with a BOM, is still read — its mode
    /// from its own declaration, not guessed from its name.
    #[test]
    fn a_theme_that_is_not_utf8_is_still_scanned() {
        let dir = scratch("latin1");
        std::fs::write(dir.join("plain.css"), b"\xef\xbb\xbf:root{color-scheme:dark}/* \xa9 */").unwrap();
        let mut out = Vec::new();
        scan(&dir, false, &mut out);
        assert_eq!(out[0].mode, Mode::Dark);
    }
```

(`scratch` is the existing helper at ~690; match its real signature.) Run: FAIL — the timing assertion (the quadratic scan: ~9 s at 5,000 in a debug build, ~25 ms after; do not raise the count, 40,000 took ~10 min) and the scan test (`read_to_string` fails today, so the mode falls back to the name: `Light`). The first two assertions already pass; they guard the fix's slicing, which a byte-based cut would break.
- [ ] **Step 2: #32.** Replace the value slice with:

```rust
                // Bounded, by characters: a value is a word or two, and slicing
                // bytes could cut a character in half. `list_themes` runs on the
                // main thread with `panic = "abort"`.
                let value = value.trim_start();
                let value = &value[..value.char_indices().nth(64).map_or(value.len(), |(i, _)| i)];
                let value = &value[..value.find([';', '}']).unwrap_or(value.len())];
```

- [ ] **Step 3: #33.** In both `read` and `scan`, read bytes and decode like a document: `std::fs::read(&path).map(|b| crate::render::decode(&b))` (keep each one's error handling); the comment says why — a theme saved in a code page still applies, and a BOM would otherwise reach `<style>` as part of the first selector.
- [ ] **Step 4: Gates.** Drive: copy a theme into the themes folder with a Latin-1 `©` in a comment and a UTF-8 BOM at the start → it lists, applies, and its first rule takes effect (compare a computed colour it sets).
- [ ] **Step 5: Commit** `Apply any user theme safely, whatever its encoding`.

### Task 7: Every window stays responsive when a network share goes away (#8)

**Files:** `src-tauri/src/main.rs` (`watch_files`, `watch_folders`, `set_watch`, `toggle_task`, `reveal_path`, the single-instance callback, `load_file`'s doc comment), `src/app.js` (`syncWatch`, `syncFolderWatch`)

- [ ] **Step 1: Async.** (Their `State`, `Window` and `&str` parameters are fine: the reference rule applies to `async fn`, not to `(async)` on a plain fn.) `#[tauri::command(async)]` on `watch_files`, `watch_folders`, `toggle_task`, `reveal_path`. `set_watch` takes the `AppHandle` and installs nothing for a window that is gone:

```rust
fn set_watch(
    app: &AppHandle,
    watches: &Mutex<HashMap<String, watch::Handle>>,
    label: String,
    handle: Option<watch::Handle>,
) {
    let mut watches = watches.lock().unwrap();
    // Built off the main thread, so the window may have gone meanwhile. Tauri
    // drops it from its own store before our `Destroyed` handler clears this
    // map under this lock, so nothing is installed that will not be cleared.
    if app.get_webview_window(&label).is_none() {
        return;
    }
    match handle {
        Some(h) => {
            watches.insert(label, h);
        }
        None => {
            watches.remove(&label);
        }
    }
}
```

`watch_files` / `watch_folders` clone `app` before moving it into `watch::watch` and pass `&app` to `set_watch`.

- [ ] **Step 2: The page keeps its watch calls in order.** In `syncWatch`: `watchCall = watchCall.then(() => invoke("watch_files", { paths }).catch(console.error));` with `let watchCall = Promise.resolve();` beside it and a comment: off the main thread, two calls could finish out of order and the older would win; chained, they cannot. Same for `syncFolderWatch` with `folderWatchCall`.
- [ ] **Step 3: A second launch naming a file on a dead share.** The single-instance callback runs on the main thread and `file_from_args` stats the path: `let app = app.clone(); std::thread::spawn(move || handle_second_instance(&app, argv));` (after Task 8's `resolve_args` if that has landed; order does not matter otherwise). `open_path` takes its locks itself and its window calls dispatch to the event loop.
- [ ] **Step 4: Comments.** Rewrite `load_file`'s doc comment where it says the watch commands stay synchronous for ordering: ordering is now the page's chain; the ponytail note about async workers stays. Also `spawn_window`'s doc (~334–338), which says the single-instance hook runs on the event loop: no longer (and on macOS it never did — the plugin calls back from a tokio task).
- [ ] **Step 5: Gates.** Drive (a fresh unreachable host each run, e.g. `\\10.9.<n>.1\s`): from window A `invoke("reveal_path", { path: "\\\\10.9.x.1\\s\\x.bat" })` unawaited, then time `invoke("window_origin")` from window B → milliseconds (before: ~21 s); `openTab("\\\\10.9.y.1\\s\\a.md")` then time a Ctrl+Tab → responsive; switch tabs rapidly 10×, edit the active file externally → it live-reloads; close a window with a watch call on a dead share in flight, wait 25 s → no error, the app still closes cleanly at the end.
- [ ] **Step 6: Commit** `Keep every window responsive when a network share goes away`.

### Task 8: Files, restores and dropped tabs go only to windows that can take them (#22, #23, #24, #25, #26)

**Files:** `src-tauri/src/main.rs`

- [ ] **Step 1: Test (#23):**

```rust
    /// A second launch's relative path is read from where that launch was
    /// started; an absolute one and a flag are unchanged.
    #[test]
    fn second_instance_args_resolve_against_their_cwd() {
        let dir = std::env::temp_dir().join(format!("t4-cwd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("n.md"), "# n\n").unwrap();
        let cwd = dir.to_string_lossy().into_owned();
        let args = resolve_args(vec!["exe".into(), "n.md".into()], &cwd);
        assert_eq!(file_from_args(&args), Some(dir.join("n.md")));
        let abs = dir.join("n.md").to_string_lossy().into_owned();
        assert_eq!(resolve_args(vec![abs.clone()], "C:\\elsewhere"), vec![abs]);
        assert_eq!(file_from_args(&resolve_args(vec!["exe".into(), "--flag".into()], &cwd)), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
```

(Adapt `file_from_args`' real return type.) Run: FAIL.
- [ ] **Step 2: #23.**

```rust
/// A second launch's arguments, read from where it was started rather than
/// from where this process was: `t4-markdown-viewer notes.md` typed in a
/// terminal means the `notes.md` in that terminal's folder. An absolute path
/// replaces the base; a flag or a URL is still no file.
fn resolve_args(argv: Vec<String>, cwd: &str) -> Vec<String> {
    argv.into_iter()
        .map(|a| Path::new(cwd).join(a).to_string_lossy().into_owned())
        .collect()
}
```

and the callback: `init(|app, argv, cwd| … handle_second_instance(app, resolve_args(argv, &cwd)) …)` (inside Task 7's thread if it has landed).
- [ ] **Step 3: #24.** At the top of `focus_window`:

```rust
    // Still booting: its page shows and raises the window itself once it has
    // painted, and showing it now would flash an unthemed window. Readiness,
    // not visibility: on macOS a minimized window reports not visible, and
    // must still be brought back.
    if !app.state::<AppState>().boot.lock().unwrap().ready.contains(label) {
        return;
    }
```

(Both `open_path` branches drop `boot` before calling — check, or this deadlocks.)
- [ ] **Step 4: #25.** In `window_at`, after the handle match and before reading the position: `if app.state::<AppState>().closing.lock().unwrap().contains(&label) { return None; }` with the comment: on its way out, as `last_focused` treats it — a tab handed over now would go with it; `None` makes the drop a tear-off.
- [ ] **Step 5: #22.** In `restore_offered_session`, next to the `restoring` insert: `state.sessions.lock().unwrap().insert(window.label().to_string(), first.open.clone());` with the comment: the window's tabs are in the record from now, as `claim_in` does for a pending restore, so a save before it reports keeps them.
- [ ] **Step 6: #26.** `.min_inner_size(420.0, 320.0)` after `.inner_size(…)` in `spawn_window`'s builder, commented: the same minimum `main` has from the config; a saved smaller frame is clamped to it.
- [ ] **Step 7: Gates.** Drive: with the app running, run the debug exe from another folder with a relative `notes.md` that exists there → it opens as a tab; minimize the window, second instance with a file → restored and in front; a cold launch with a file → no white flash (log `w.is_visible()` in `take_pending` temporarily: `false`; remove the log); (#25) hold B in `closing` by stalling its session report — a second `onCloseRequested` listener does not, each listener destroys on its own. In B wrap `window.chrome.webview.postMessage` — `__TAURI_INTERNALS__.invoke` is non-writable, and the CSP blocks Tauri's IPC `fetch`, so every call goes through postMessage: `const o = chrome.webview.postMessage.bind(chrome.webview); chrome.webview.postMessage = (m) => { if (!String(typeof m === "string" ? m : JSON.stringify(m)).includes("set_session")) o(m); };` then close B; within `CLOSE_WAIT` (2 s) drop a tab from A over B → `"detached"` (before the fix: `"adopted"` — confirm that on the pre-fix build first, or the check proves nothing); Ctrl+N then shrink the window with Win32 `MoveWindow` (`set-size` is not granted) → inner size ≥ 420×320; (#22) "ask" mode with two saved windows: stub `set_session` in `main` the same way before pressing Reopen, press it, poll `session.json` until it changes (that is `w1`'s report) → it holds `main`'s tabs as well as `w1`'s (before the fix: only `w1`'s).
- [ ] **Step 8: Commit** `Tighten how windows open, restore and take tabs`.

### Task 9: The folder tree stays current, and a failed folder pick is undone (#9, #17)

**Files:** `src/app.js` (`listTree`, `expandRow`, `openFolder`)

- [ ] **Step 1: #9.** In `listTree`, `if (signature === treeListings.get(ul)) return "same";` and extend its JSDoc (`"same"` when the listing had not changed; `false` when it failed). `expandRow`:

```js
async function expandRow(row, open, keep) {
  row.parentElement.setAttribute("aria-expanded", String(open));
  const children = row.nextElementSibling;
  children.hidden = !open;
  if (!open) return;
  // Unchanged here, but nothing watched the folders left open inside while
  // this one was shut: ask each again. A rebuild re-lists them itself.
  if ((await renderTree(children, row.dataset.path, keep)) !== "same") return;
  const inner = children.querySelectorAll(':scope > li[aria-expanded="true"] > .tree-row');
  await Promise.all([...inner].map((r) => expandRow(r, true)));
}
```

- [ ] **Step 2: #17.** In `openFolder`: the `before` record gains `n: 0` (calls made for this tab while any is out; the last one answers for it); `const n = writes ? ++before.n : 0;` before `before.out++`; `const latest = writes && before.n === n;` **after** the listing's try/finally, next to `overtaken` (~1366) — declared before the `await` it is always true and fixes nothing (the vetted harness has it there, `openfolder.js:21`). The overtaken revert requires `latest`; the closed branch becomes:

```js
  if (state.folder === null) {
    // Closed while the listing was out. A folder whose own listing failed was
    // never really picked, so the tab goes back to the one it had — unless a
    // newer call has answered for it since. Not the filter: closing blanked it.
    if (latest && listed === false && owner.folder === path) {
      Object.assign(owner, { folder: was, picked: wasPicked });
      reportSoon();
    }
    return;
  }
```

and the `set_last_folder` condition gains `&& listed !== false`.
- [ ] **Step 3: Gates.** Node: the vetting harness `scratchpad/vet-fe-ui/openfolder.js` string-patches the *old* code into variants and throws on patched code, so copy it and cut `variants` down to `{ current: base }`, then `node openfolder.js src/app.js` → 9/9 under `current`. (If it is gone, re-create its nine cases: #17 as reported, the reverse, newer-call-succeeded closed and overtaken, no `set_last_folder` for a dead call, closed-mid-listing keeps the filter blank, plain pick lands canonical and is recorded, same-tab double pick, the unplugged-own-folder fallback.) Drive (#9): folder `F/A/B/x.md`; expand A, B; collapse A; create `F/A/B/new.md`; `new MutationObserver(…).observe(aUl, { childList: true })` — no `subtree`, B's own rebuild is below it; expand A → B lists `new.md`, no records on A's `ul`; repeat with no change → no records. Drive (#17): pick `//10.x.y.1/share`, Ctrl+Tab, close the sidebar, wait ~22 s → the first tab's `folder` is its old one.
- [ ] **Step 4: Commit** `Keep the folder tree current and undo a failed folder pick`.

### Task 10: Reading positions survive anchors, failed reloads and restores (#18, #19, #20, #21)

**Files:** `src/app.js` (`renderDocument`, `showActive`, `loadPath`, `pushAnchorEntry`, `refresh`, `rememberScroll`, `rememberImage`, `restoreTabs`, `main()`)

- [ ] **Step 1: #18.** `renderDocument`: `if (hash && scrollY == null && jumpToAnchor(hash)) {` and `window.scrollTo(0, scrollY ?? 0);`, the comment "`scrollY` is null exactly when nothing has been recorded yet"; `showActive`: `renderDocument(doc, scrollY ?? entry.scrollY, entry.hash);`; `loadPath`: push `{ path, scrollY: null, hash: id }` and `await showActive();`; `pushAnchorEntry`: push `scrollY: null`.
- [ ] **Step 2: #19.** `rememberScroll`'s first line: `if (shownToken !== renderToken || !els.error.hidden) return;`; `refresh`'s last line: `await showActive(shownToken === renderToken && els.error.hidden ? window.scrollY : undefined);` with the comment extended: nor while the error panel is up, whose scroll position is 0.
- [ ] **Step 3: #20.** `rememberImage`'s first line: `if (shownToken !== renderToken) return;` (the same reason `rememberScroll` gives).
- [ ] **Step 4: #21.**

```js
async function restoreTabs(list, active) {
  const rebuilt = list.map(rebuildTab);
  // A tab that got here first — a file opened or a tab dropped once
  // `take_pending` had answered — stays, and stays in front: the reader has
  // just asked for it.
  const early = tabs.length > 0;
  tabs = [...rebuilt.filter(Boolean), ...tabs];
  syncWatch();
  // The early tab is already on screen: showing it again would load it twice
  // and put it back at the last scroll position banked, not where it is.
  if (early) return updateChrome();
  // `active` counts the list as it was saved …  (keep the existing comment)
  activeId = (rebuilt[active] ?? tabs[0])?.id ?? null;
  await showActive();
}
```

(Check `updateChrome` redraws the tab strip; if not, call `renderTabs()` there too.)

and `main()`'s final `} else {` becomes `} else if (!tabs.length) {`, commented: a file that arrived and rendered first must not be hidden behind the empty screen.
- [ ] **Step 5: Gates.** Drive: `[deep](b.md#deep)` → lands on `#deep`; then `scrollTo(0, 0)`, a frame, `await refresh()` → `scrollY` stays 0; long document at 3000, rename it, `await refresh()` (error panel) → entry `scrollY` still 3000; rename back, `await refresh()` → ≈ 3000; two image tabs with `load_asset` delayed, `cycleTab(1)` then `zoomBy(2)` → the new entry's `scale` is null and it fits; one tab open, `await restoreTabs([{ path: "<other>.md" }], 0)` → two tabs, `activeId` unchanged.
- [ ] **Step 6: Commit** `Keep reading positions through anchors, failed reloads and restores`.

### Task 11: A stalled update times out, and every window shows why an update failed (#15, #34, #38)

**Files:** `src-tauri/src/update.rs`, `src/app.js` (`update-failed`, `update-progress` listeners), `src/base.css`, `src-tauri/themes/solarized-dark.css`, `src-tauri/themes/README.md`, `src-tauri/themes/_template.css`

- [ ] **Step 1: #15.** Add `use std::time::Duration;` (not imported in `update.rs` today). One helper used by `check_for_update` and `install` in place of `app.updater()` (updater 2.12.0 has `configure_client`; the closure's `reqwest::ClientBuilder` type infers — the plugin does not re-export it):

```rust
/// The updater, with a limit on how long any one wait may take. Per read, not
/// in total: a stalled check or download fails instead of holding the update
/// lock — and every later check — for good, while a slow link that keeps
/// delivering bytes still finishes.
fn updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    app.updater_builder()
        .configure_client(|c| c.connect_timeout(Duration::from_secs(15)).read_timeout(Duration::from_secs(30)))
        .build()
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: #34.**

```js
  await listen("update-failed", () => {
    setInstalling(false);
    els.updateProgress.hidden = true;
    els.updateProgress.textContent = "";
    // The window that asked has its own reason from `invoke`, which may have
    // landed first; every other window says only that it failed.
    if (els.updateError.hidden) {
      els.updateError.textContent = "The update failed.";
      els.updateError.hidden = false;
    }
  });
```

and in the `update-progress` listener, before showing it: `els.updateProgress.hidden = false; els.updateError.hidden = true;` — a dialog already open in another window when the install began otherwise shows no progress, and keeps an earlier failure's reason on screen, which the `hidden` guard above would then never replace. Emits arrive in order, so no progress lands after `update-failed`.
- [ ] **Step 3: #38.** `base.css` `:root`, under the "Optional" comment (~19): `--ui-error: color-mix(in srgb, #d13438 60%, var(--ui-fg));`; the existing `#update-error { color: #d13438; … }` (~785) takes `var(--ui-error)` — no second rule. `solarized-dark.css` `:root`: `--ui-error: #ff7b72;` (its own foreground is too dim for any mix to reach AA: 3.0:1; this is 5.16:1). Document the optional token: `themes/README.md` ("Three more are optional" ~80 → four, and a line in its example block ~86–90); `_template.css` a commented `/* --ui-error: … */` in its optional list (~33–37). Check contrast on every theme's `--ui-bg` (the vetting figures: all ≥ 4.8 except Solarized dark, 5.2 with the override) — a node script over the theme files, or computed styles in the app.
- [ ] **Step 4: Gates.** Drive: two windows; in window 2 `state.update = { version: "9.9.9", notes: "", installable: true }; showUpdateDialog(); setInstalling(true); els.updateProgress.hidden = false; showUpdateProgress(42);` then `window.__TAURI__.event.emit("update-failed")` → progress hidden, "The update failed." shown, Update now enabled; in window 1 set a detailed error first, emit → the detailed text stays. Computed colour of `#update-error` on three themes including Solarized dark.
- [ ] **Step 5: Commit** `Time out a stalled update and show why it failed in every window`.

### Task 12: A tab drag or a picture pan ends when the pointer is lost (#35)

**Files:** `src/app.js` (`onDragMove`, `onImagePointerMove`, wiring ~2980–2990)

- [ ] **Step 1: Check first.** Drive: start a tab drag (press on a tab, move past the threshold, hold), force a `renderTabs()` reflow under a still cursor, and log every `pointermove` with its `buttons`. **If any arrives with `buttons === 0` while the button is held, stop and ask the owner** — the `buttons` check would cancel real drags; ship only the `lostpointercapture` half.
- [ ] **Step 2:** `onDragMove`, after its `pointerId` check: `if (event.buttons === 0) return onDragCancel(); // a release the page never heard (Alt+Tab, a UAC prompt)`; `onImagePointerMove`: `if (event.buttons === 0) return onImagePointerUp(event);`. Wiring:

```js
  // Only the element that holds the capture: a touch captures the tab or the
  // picture itself first, and handing that over to the bar fires this on the
  // child — which bubbles here and would cancel every touch drag.
  els.bar.addEventListener("lostpointercapture", (e) => e.target === els.bar && onDragCancel());
  els.imageView.addEventListener("lostpointercapture", (e) => e.target === els.imageView && onImagePointerUp(e));
```

`onDragCancel` (~2172) never released the capture — fine while only `pointercancel` called it, as the browser releases then; on the new `buttons === 0` path no `pointerup` follows, and the bar would keep the mouse. After its `drag = null`, add `try { els.bar.releasePointerCapture(d.pointerId); } catch {}` as `onDragEnd` does (~2107–2111).

- [ ] **Step 3: Gates.** Drive: a scripted drag with a `mouseMoved` carrying `buttons: 0` after the threshold → `drag === null`, tab back at its start, `els.bar.hasPointerCapture(1) === false`, and a click on a link in the page then follows it; press and move on a tab, then `els.bar.releasePointerCapture(drag.pointerId)` → `drag === null` next tick; `Input.dispatchTouchEvent` touchStart + touchMove on a tab → `drag` still set; an ordinary `drag` reorder still lands.
- [ ] **Step 4: Commit** `Cancel a tab drag or picture pan when the pointer is lost`.

### Task 13: highlight.js 11.12.0 (#45)

**Files:** `src/vendor/highlight.min.js`, `src-tauri/THIRD-PARTY-LICENSES.md`

- [ ] **Step 1:** Confirm the vendored file is byte-identical to `https://cdn.jsdelivr.net/npm/@highlightjs/cdn-assets@11.11.1/highlight.min.js`; download `…@11.12.0/highlight.min.js` (129,254 bytes per the vetting) over it; check its header names 11.12.0. Update the version line in THIRD-PARTY-LICENSES.
- [ ] **Step 2: Gates.** Drive: `examples/kitchen-sink.md` (or the example with the most code blocks) in three themes → code blocks highlighted, `hljs-*` spans present, no console error.
- [ ] **Step 3: Commit** `Update highlight.js to 11.12.0, which fixes a hang on some C and C++ code` — no prefix: a hang a reader can hit is a fix to shipped behaviour, and a `chore:` subject is dropped from the release notes.

### Task 14: The uninstaller removes the app's own data when asked; Linux files the app under Utility (#39, #40)

**Files:** `src-tauri/windows/hooks.nsi`, `src-tauri/tauri.conf.json`

- [ ] **Step 1: #39.** At the end of `NSIS_HOOK_POSTUNINSTALL`, after the registry lines:

```nsis
  ; Tauri's "Delete application data" removes $APPDATA\<identifier> and
  ; $LOCALAPPDATA\<identifier> only; the app keeps its own files - settings,
  ; the session, user themes - in config.rs::dir(). Same guard as Tauri's:
  ; ticked, and not an update.
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RmDir /r "$APPDATA\t4-markdown-viewer"
  ${EndIf}
```

- [ ] **Step 2: #40.** `"category": "Utility",` in `bundle`.
- [ ] **Step 3: Gates.** **Never install on the host:** the install dir, uninstall key and running-app check are keyed on the product name, not the identifier, so a local install lands on the owner's viewer, and its uninstall deletes the `.md`/`.json` associations (`hooks.nsi:78-88`). Build locally — `cargo tauri build --bundles nsis --no-sign` from `src-tauri` (`--no-sign`: off CI the updater key is absent and `createUpdaterArtifacts` errors; install `tauri-cli 2.11.4 --locked` if `cargo tauri` is missing). Then test in **Windows Sandbox** (`C:\Windows\System32\WindowsSandbox.exe`, present): a `.wsb` in the scratchpad mapping the bundle folder and a results folder, with a `LogonCommand` running a script that, for each case, seeds `%APPDATA%\t4-markdown-viewer\config.json` (no need to launch the app) and writes `Test-Path` results to the results folder:
  - install `/S`, `uninstall.exe /S` (unticked) → folder kept;
  - install `/S`, then the same installer `/P /UPDATE` over it → kept;
  - install, uninstall with the box **ticked** → gone. The silent uninstaller cannot tick it: **stop and ask the owner** to run this one case by hand in the open sandbox (the script installs and leaves the uninstaller's path on the desktop), then read the result.
  The sandbox is discarded on close. `.deb` category: checked in Task 15's dry run.
- [ ] **Step 4: Commit** `Remove the app's own data on uninstall when asked, and file it under Utility on Linux`.

### Task 15: CI builds with pinned tools and keeps the signing keys out of the compile (#11, #12, #13, #16)

**Files:** `.github/workflows/release.yml`, `.claude/skills/release/SKILL.md`

- [ ] **Step 1: #12.** Delete the `cargo-bins/cargo-binstall` step; the install step becomes:

```yaml
      - name: Install the Tauri CLI
        # Built from crates.io with the CLI's own lockfile, so every crate is
        # checksum-verified. rust-cache keeps ~/.cargo/bin, so this is a no-op
        # until the toolchain or this version changes (then ~10 min). Pinned
        # exactly: this job holds the signing key. Bump deliberately with Tauri.
        run: cargo install tauri-cli --version 2.11.4 --locked
```

- [ ] **Step 2: #13 + #16's env gating.** Replace "Build the packages" with two steps:

```yaml
      # Compile with no secrets in env: every build.rs and proc-macro runs here.
      - name: Build the app
        working-directory: src-tauri
        run: >
          cargo tauri build --no-bundle
          ${{ matrix.target && format('--target {0}', matrix.target) || '' }}
          -- --locked

      # `cargo tauri bundle` compiles nothing; it packages, signs, and writes
      # the updater .sig files. Missing the updater key is an error here.
      - name: Bundle and sign
        working-directory: src-tauri
        shell: bash
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
          APPLE_SIGNING_IDENTITY: T4 Apps Self-Signed Code Signing
          CERTUM_EMAIL: ${{ runner.os == 'Windows' && secrets.CERTUM_EMAIL || '' }}
          CERTUM_OTP: ${{ runner.os == 'Windows' && secrets.CERTUM_OTP || '' }}
        run: |
          cargo tauri bundle --bundles ${{ matrix.bundles }} \
            ${{ matrix.target && format('--target {0}', matrix.target) || '' }} \
            ${{ env.SIGNING_ARGS }} 2>&1 | tee "$RUNNER_TEMP/bundle.log"
          # Every AppImage tool is pinned in "Pin the AppImage tools".
          if [ "$RUNNER_OS" = Linux ] && grep -q Downloading "$RUNNER_TEMP/bundle.log"; then
            echo "bundler downloaded a tool the pins do not cover" >&2; exit 1
          fi
```

Keep the old step's comments that still hold (why each secret exists, the macOS self-signed note), and correct the claim that the build "still succeeds" without the key — `bundle` errors. Move "Import the macOS signing certificate" (~156–176) from before the compile to just before "Bundle and sign": where it is, the certificate sits in an unlocked keychain open to `codesign` through every build script and proc-macro.
- [ ] **Step 3: #11.** Between "Build the app" and "Bundle and sign":

```yaml
      # The AppImage bundler downloads its tools from moving URLs (master,
      # `continuous`) and runs them with the updater key in env, but skips any
      # file already in its cache dir. Seed pinned, hashed copies there — and
      # the runtime appimagetool would otherwise fetch from `continuous`, which
      # becomes the head of every AppImage shipped. Names are tauri-bundler
      # 2.9.4's (tauri-cli 2.11.4); "Bundle and sign" fails if a CLI bump makes
      # the bundler itself download a tool. A fetch by appimagetool would not
      # show in that log — the runtime pin is checked by the dry run instead.
      - name: Pin the AppImage tools
        if: runner.os == 'Linux'
        shell: bash
        run: |
          set -euo pipefail
          dir="${XDG_CACHE_HOME:-$HOME/.cache}/tauri"
          mkdir -p "$dir"
          fetch() {
            curl -fsSL --retry 3 -o "$dir/$1" "$2"
            echo "$3  $dir/$1" | sha256sum -c -
            chmod 770 "$dir/$1"
          }
          rel=https://github.com
          raw=https://raw.githubusercontent.com
          fetch AppRun-x86_64 $rel/tauri-apps/binary-releases/releases/download/apprun-old/AppRun-x86_64 f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f
          fetch linuxdeploy-x86_64.AppImage $rel/tauri-apps/binary-releases/releases/download/linuxdeploy/linuxdeploy-x86_64.AppImage e762bea85c8eb0d4b3508d46e5c1f037f717d0f9303ae3b4aafc8b04991fa1ef
          fetch linuxdeploy-plugin-gtk.sh $raw/tauri-apps/linuxdeploy-plugin-gtk/dda522bce37387f1b853d9095713bfaa924c8423/linuxdeploy-plugin-gtk.sh 7804c9eef13e59bf2783aad9882ef9db8f3f3f9e8d631874b1d348d550a3693f
          fetch linuxdeploy-plugin-gstreamer.sh $raw/tauri-apps/linuxdeploy-plugin-gstreamer/2a2e67491c32995a3f279ad0ecbe77abd512b42a/linuxdeploy-plugin-gstreamer.sh c107b49d84edbffc6ab226ed1007e0626a4f7aa2c3a36b7782bef62351d49e94
          fetch linuxdeploy-plugin-appimage.AppImage $rel/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/1-alpha-20250213-1/linuxdeploy-plugin-appimage-x86_64.AppImage 992d502a248e14ab185448ddf6f6e7d25558cb84d4623c354c3af350c25fccb3
          fetch runtime-x86_64 $rel/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64 2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
          echo "LDAI_RUNTIME_FILE=$dir/runtime-x86_64" >> "$GITHUB_ENV"
```

Before committing, re-download each of the six locally and confirm its sha256 matches (the hashes were computed on 2026-09-25; a mismatch now means an upstream re-upload — stop and ask).
- [ ] **Step 4: #16.** Job level: `environment: signing` for every leg (replace `environment: ${{ matrix.environment }}`, drop the Windows matrix entry's `environment`), and update the comment that explains the environment.
- [ ] **Step 5: Release skill.** `.claude/skills/release/SKILL.md` "Signing": every secret now lives in the `signing` environment and every leg uses it; the step names are "Build the app" + "Bundle and sign"; without the key the bundle step errors; run a dry run after any tauri-cli bump (the AppImage pins); a line for #46 (if `release.yml` changed since the last tag, read the bumped action's changelog first); a line for #45 (check highlight.js for a new release).
- [ ] **Step 6: Gates.** `actionlint` if installed, else a YAML parse (`node -e` with a YAML-free check is not possible — use `python -c "import yaml,sys;yaml.safe_load(open(sys.argv[1]))"`). Commit `ci: pin the AppImage tools, build the Tauri CLI from crates.io, and keep signing keys out of the compile`.
- [ ] **Step 7: STOP — owner steps and dry run.** Report to the owner, in order: (1) add the four secrets to the `signing` environment (`gh secret set TAURI_SIGNING_PRIVATE_KEY --env signing -R toperux/t4-markdown-viewer < <file>` and likewise `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`); (2) clear the `v0-rust-build` caches (`gh cache list -R … --key v0-rust-build`, `gh cache delete <id>`); (3) push when they choose; (4) a `workflow_dispatch` dry run on `main`. Expected: every leg shows `Installed package tauri-cli v2.11.4`; the pin step shows six `OK`; the bundle log has no `Downloading`; `Finished N updater signature(s)`; the thumbprint and macOS fingerprint checks pass; staging finds every `.sig`; the `.deb` has `Categories=Utility;`; the AppImage, run on Linux or WSL, reports runtime commit `dd6cebe` (`--appimage-version`) and opens a `.md`. Then (5) delete the four repo secrets and dry-run again; (6) turn on "require actions pinned to a full-length commit SHA". A second dry run shows the CLI "already installed".

### Task 16: README and licences (#14, #41, #42)

**Files:** `README.md`, `src-tauri/THIRD-PARTY-LICENSES.md`

- [ ] **Step 1: #14.** `README.md:171` → `xdg-mime default "T4 Markdown Viewer.desktop" text/markdown`.
- [ ] **Step 2: #41.** The `.desktop` paragraph and Layout lines as the review's #41 row gives them. The paragraph runs to README:499, not 498: replace from 493 through "covers both." (~497), keep the MimeType sentence (still true) and re-wrap it — do not leave the "explicitly — it is never inferred…" fragment dangling.
- [ ] **Step 3: #42.** Regenerate with the vetted script `scratchpad/vet-ci/lic.py` over `cargo metadata --format-version 1 --locked --filter-platform x86_64-pc-windows-msvc --manifest-path src-tauri/Cargo.toml`, non-dev edges (it prints 349 on `vet-ci/meta-x86_64-pc-windows-msvc.json`). Then:
  - lines 3–5 ("Everything it depends on is permissively licensed… Nothing here is GPL…") go entirely; new header: "No Rust crate here is GPL, LGPL, AGPL or SSPL. The Linux AppImage also bundles system libraries; see below.";
  - the count (line ~36) 306 → 349; "Apache-2.0 only" gains `sync_wrapper` (2); `ring` is `Apache-2.0 AND ISC` (ISC is new); re-derive the MPL list and the Direct dependencies line;
  - "Regenerating" (~73–79) describes the `cargo metadata` + non-dev-edge method, not cargo-about.

  New section (the owner's decision: a written offer):

> ## Linux AppImage
> The AppImage also carries shared libraries that linuxdeploy copies from the Ubuntu 22.04 build machine — GTK 3, WebKitGTK, GLib, GStreamer, libsoup and what they link to (the full list is `usr/lib` after `--appimage-extract`). They are unmodified Ubuntu binaries under their own licences: mostly LGPL-2.1-or-later, some MIT/BSD/ICU, and `libjbig` (through libtiff) under GPL-2.0-or-later. The app loads them dynamically; any of them can be replaced by extracting the AppImage and running `AppRun` from the extracted tree. Their source is the matching Ubuntu 22.04 (jammy) source package (`apt-get source <package>`), and the author will provide it on request for three years after each release. The .deb and .rpm bundle none of these.

Also the highlight.js version line if Task 13 did not.
- [ ] **Step 4:** No code gates change; run them anyway. Commit `docs: correct the Linux default-app command and the layout, and complete the third-party licences`.

### Task 17: Smoke pass and close-out

- [ ] **Step 1: Smoke.** One debug launch, `"reopen": "restore"` with a two-window session: both windows come back; tabs, Back/Forward, anchors in-page and cross-file; sidebar open, expand, filter, sort; a task tick; copy-section; a JSON file with `more`; an image tab zoom; Settings open and closed; theme switch; close a window and relaunch → it comes back. Any failure → back to its task.
- [ ] **Step 2: drive-app skill.** Gotchas: a reload (CDP `location.reload()`) now comes back with the window's last-reported tabs — relaunch for a clean page; `Page.crash` plus a real click on Refresh drives the crash screen (`cdpraw.mjs`, `click.ps1`, `winshot.ps1` — copy them into the skill's `scripts/`); stubbing or delaying one command to test a race: wrap `window.chrome.webview.postMessage` and hold back messages naming the command (`__TAURI_INTERNALS__.invoke` is non-writable, and the CSP blocks the IPC `fetch`, so Tauri falls back to postMessage), e.g. a `set_session` that never answers holds a window in `closing`; real OS keystrokes for reload and Back accelerators (`sendkeys.ps1`); never install a local NSIS build on the host — Windows Sandbox; Git Bash collapses a leading `\\` in an argument, so launch a second instance with a UNC path from a `.ps1` (`scratchpad/task7/second.ps1`); Win32 `MoveWindow` ignores a window's minimum size — test a minimum with a real corner drag (`scratchpad/task8/drag.ps1`), and fix the skill's "Window size" note to say so; a repeat call to a dead host is not reliably fast — Windows' failure cache expires, so use a fresh IP per probe.
- [ ] **Step 3: Review file.** Every fixed finding `done (<hash>)`; #37, #43 (trigger: "a session or settings lost after a crash or power cut"), #44, #46 (trigger: "a Dependabot bump touches `softprops/action-gh-release` or `download-artifact`"), #47, #48 (build after this batch) `park`; #49, #50 `wontfix`; a status block at the top like 2026-09-20's, noting the hashes are refreshed after the owner's squash. Move it to `docs/archive/reviews/` and change its `../open-items.md` / `../closed-items.md` links (lines ~25, 26, 199) to `../../`.
- [ ] **Step 4: open-items / closed-items.** `open-items.md`: a new section "Deferred from the 2026-09-25 review" (mirroring `closed-items`' section of that name, ~269) with an unticked item per park and its trigger, and one for #48 ("Mock-runtime tests for window routing and session saving — do next, after the 2026-09-25 fixes"); Task 15's dry run and owner steps if not yet done; update the Context paragraph (~4–5), which says what is left needs a Mac, a Linux install or a date. `closed-items.md` *Accepted limits*: #49 and #50 with a "Kept" note each.
- [ ] **Step 5:** `git mv` this plan to `docs/archive/plans/` with a status line; commit `docs: close the 2026-09-25 review`.
