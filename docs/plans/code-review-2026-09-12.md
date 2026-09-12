# Full codebase review — 2026-09-12

Reviewed at `b9c7fc6` (Release 1.5.10). Four parallel reviewers (Rust IPC/backend,
renderer/themes/updater, frontend, packaging/CI) plus a diff review of the last
two commits. High and most medium findings were re-verified by hand against the
code; the renderer findings were verified by running comrak 0.54 out of tree.

**Caveat:** #1 is confirmed from the opener plugin's source, not by running the app.

Status column: `open` / `fix` / `wontfix` / `done`. Fill in as we go.

## High

| # | Status | Where | Finding |
|---|--------|-------|---------|
| 1 | done | `src-tauri/capabilities/default.json:16` | **Decided:** add `opener:allow-default-urls`; replace the `openUrl(convertFileSrc(target))` branch at `app.js:1734` with a Rust command that **reveals** the file in the file manager (`reveal_item_in_dir`), not one that launches it (see audit A1); toast `openUrl` failures instead of `console.error`. **External links do nothing.** Capability grants `opener:allow-open-url` but no URL scope. The plugin's `Scope::is_url_allowed` returns false on an empty allow list, so every `openUrl` rejects with `ForbiddenUrl`; `.catch(console.error)` swallows it. Affects document links (`app.js:1739`), the release-notes button (`app.js:1231`, `2101`), and relative non-md/non-image links (`app.js:1734`). **Fix:** add `opener:allow-default-urls` (http/https/mailto/tel). |
| 2 | done | `src/app.js:115` | **Decided:** patch `resolvePath` in JS (detect `//` prefix, keep both slashes, floor `..` at server/share); verify manually via drive-app with a `\\localhost\c$\...` doc. **UNC paths break images and cross-links.** `resolvePath` keeps only one leading empty segment, so `\\server\share\notes` + `img.png` → `/server/share/notes/img.png`. `..` can also pop `server` off the root. **Fix:** detect a `//` prefix before splitting, re-emit both slashes on join, floor `..` at server/share. |
| 3 | done | `src/app.js:730` | **Decided:** skip `showActive()` when the removed tab wasn't active. **Closing a background tab reloads the active doc and loses its scroll.** `removeTab` skips `rememberScroll` when the closed tab isn't active, then calls `showActive()` which re-invokes `load_file` and restores the stale `entry.scrollY`. **Fix:** if the removed tab isn't active, only `syncWatch()`, `renderTabs()`, `updateChrome()`. |

## Medium

| # | Status | Where | Finding |
|---|--------|-------|---------|
| 4 | done | `src-tauri/src/main.rs:431`, `config.rs:60` | **Decided:** `.md` → in-place 1-byte write at the box offset (`render::toggle_task` returns the offset); config/session → temp + `rename` in `write_json`. **Non-atomic writes.** `toggle_task` and `write_json` use `fs::write` (truncate then write). Disk full / kill mid-write leaves the user's markdown, `config.json`, or `session.json` truncated; `config::load` silently falls back to defaults. **Fix:** write to a sibling temp file, then `fs::rename`. |
| 5 | done | `src-tauri/src/main.rs:687`, `src/app.js:2211` | **Decided:** adopt handler computes the slot from `e.payload.x`. **Cross-window drop always lands at the end.** `drop_tab` calls `clear_drag` (emits `tab-drag-out` → `setCaret(-1)`) before emitting `tab-adopt`, whose handler reads `dropCaret`. The `x` in the adopt payload is never read. **Fix:** in the adopt handler use `tabs.length > 1 ? insertionIndex(e.payload.x) : tabs.length`. |
| 6 | done | `src-tauri/src/main.rs:778` | **Decided (revised in audit):** put `ready` and `pending` behind one mutex so the check-then-claim in `open_path` and the mark-then-take in `take_pending` are each atomic. Swapping the two lines in `take_pending` does not close the race (see audit A2). **Lost file open race.** `open_path` reads `ready` and then claims `pending` under separate locks. If `take_pending` runs between (insert `ready`, remove `pending` → None), the payload lands in a slot nobody drains: window focuses showing "empty", file never opens. **Fix:** have `take_pending` remove `pending` first and insert `ready` second. |
| 7 | done | `src-tauri/src/main.rs:274` | **Decided:** optimistic + rollback — on build failure Rust emits `tab-spawn-failed` with the packed tab to the source window, which re-adds it and toasts. **Torn-off tab silently lost on window build failure.** `spawn_window` returns the label synchronously; `drop_tab` reports `detached` and the frontend removes the tab. If `build()` fails on the worker thread the only signal is `eprintln!` (invisible in a `windows_subsystem = "windows"` build). **Fix:** emit a failure event back to the source window so it keeps the tab. |
| 8 | done | `src-tauri/src/render.rs:121` | **Decided:** skip `Image` subtrees when collecting targets; add regression test. **Anchor recovery desyncs on raw HTML in image alt text.** `targets` counts every raw-HTML node, but comrak renders image alt children as plain text and emits no `<!-- raw HTML omitted -->` for them. One such image shifts every later anchor onto the wrong placeholder and drops the last ones. Verified: `![a <a id="ghost"></a> b](i.png)` + `<b>x</b>` + `<a id="real"></a>` → `ghost` lands on the `<b>`, `real` is lost. **Fix:** don't descend into `NodeValue::Image` when collecting targets; add a regression test. |
| 9 | done | `src-tauri/src/themes.rs:233`, `:286`, `src/app.js:1570` | **Decided:** allow dots (`path_for` rejects only `/`, `\`, `:`, empty; also hide `_`-prefixed); `applyTheme` toasts on failure and applies `DEFAULT_THEME` without persisting (expose default via `get_settings`); `selectTheme` persists only on success. **Dotted theme names permanently unstyle the app.** `scan` lists `my.theme.css` via `file_stem`; `path_for` rejects any name containing `.`. Selecting it persists the name, `read_theme` fails every launch, `applyTheme` only `console.error`s, and the window renders with base.css only. No fallback to `DEFAULT_THEME`. **Fix:** apply the same guard in `scan`; on `applyTheme` failure fall back to the default theme and toast. (Also: `path_for` still reads `_template`, which `scan` hides.) |
| 10 | done | `src/app.js:1485` | **Decided:** `rememberScroll()` at the top of `onDragEnd`. **Tab dragged to another window arrives at the top.** `onDragEnd` serialises with `packTab` before `removeTab`'s `rememberScroll` runs. **Fix:** `rememberScroll()` at the top of `onDragEnd`. |
| 11 | done | `src/app.js:563` | **Decided:** always bump for images; drop the `isWatched` condition. **Image freshness check is dead.** `!isWatched(asset.path)` can never be true because every caller runs `syncWatch()` (which watches every tab's current entry) before `showActive`. Reopening an image that changed while unwatched shows the cached bytes. **Fix:** capture watched state before `syncWatch` mutates it. |
| 12 | done | `.github/workflows/release.yml:35` | **Decided:** version step also parses the package entry in `Cargo.lock`. **Version gate ignores `Cargo.lock`.** Only Cargo.toml is compared to the tag; the build runs `--locked`, so a stale lock fails all three legs after the tag is public. **Fix:** also parse the package entry in `Cargo.lock`. |
| 13 | done | `.github/dependabot.yml`, `checks.yml`, `ci.yml` | **Decided:** dependabot `cargo` entry, weekly, grouped, `chore:` prefix; `node --check src/app.js` on the Linux leg of `checks.yml`; `pull_request` trigger in `ci.yml`. **CI gaps.** No `cargo` ecosystem in dependabot (Rust tree never bumped, no audit). Frontend (`app.js`) never linted or even `node --check`ed. No `pull_request` trigger, so PRs merge unchecked. |

## Low

| # | Status | Where | Finding |
|---|--------|-------|---------|
| 14 | wontfix | `src-tauri/src/main.rs:393`, `:503` | **Decided:** keep recursive (subfolder images need it; CSP blocks the exploit path); add a comment saying so. Asset scope widened with `allow_directory(dir, true)` (recursive), never revoked. Opening `~/notes.md` grants the whole home dir for the session. Consider `false`. |
| 15 | done | `src-tauri/src/main.rs:267` | `Placement::Cursor` subtracts 140/24 from physical pixels; off by the scale factor on HiDPI. |
| 16 | done | `src-tauri/src/update.rs:78` | Only a positive check result is cached, so "no update" hits GitHub once per window; no in-flight dedupe. |
| 17 | wontfix | `src-tauri/src/update.rs:98` | **Decided:** a newer signed release installing is acceptable. `install_update` re-fetches the manifest; may install a different version than the dialog showed. `installable()` not re-checked. |
| 18 | done | `src-tauri/src/main.rs:387` | **Decided:** 32 MB ceiling checked in `locate()`. No file size cap; sidebar lists `.txt`, so a multi-GB log reads, decodes and renders synchronously. |
| 19 | done | `src-tauri/src/render.rs:288` | `first_heading` hand-rolls a scanner and disagrees with comrak: `#hashtag` (no space), indented code `    # x`, `- item\n---` (thematic break), mismatched fence markers. Use the already-parsed tree. |
| 20 | done | `src-tauri/src/render.rs:62` | `attribute` only accepts quoted values; `<a id=f5></a>` yields no anchor. |
| 21 | wontfix | `src-tauri/src/themes.rs:105`, `:135` | **Decided:** add a comment naming the limit. `strip_comments` and the brace counter ignore string literals (`content: "/*"`, `--x: "}"`). Only affects light/dark classification. |
| 22 | done | `src/app.js:802` | `restoreTabs` indexes the filtered array with the pre-filter `active`; a failed rebuild activates the wrong tab. |
| 23 | done | `src/app.js:1389` | **Decided:** keep the in-flight `drag_over` promise and await it before `drag_cancel`. `drag_over` fire-and-forget can land after `drag_cancel`; other window keeps a stuck caret. |
| 24 | done | `src/app.js:1505` | `onDragCancel` doesn't undo the in-strip reorder `reorderTo` already applied. |
| 25 | done | `src/app.js:1106` | Re-clicking the same `#hash` link pushes a history entry but `location.hash = same` doesn't scroll. |
| 26 | done | `src/app.js:2135`, `1616`, `1625`, `2106` | Unhandled rejections: `onKeydown` (async, no try/catch), bare `selectTheme()` calls. |
| 27 | done | `src/app.js:641` | **Decided:** attributes only — `aria-selected` + `tabindex` on the active tab. Tabs have `role="tab"` but no `aria-selected` / `tabindex`; strip unreachable by keyboard. |
| 28 | done | `src/app.js:870`, `:1000` | **Decided:** attributes only — move `aria-expanded` to the `treeitem`, add `aria-selected`, `tabindex` on the selected row. Tree `aria-expanded` sits on the inner div, not the `li[role="treeitem"]`; no `aria-selected`; nothing focusable. |
| 29 | done | `src-tauri/linux/main.desktop:12` | **Decided:** `%f`; single-instance hook turns each launch into a tab. `Exec … %F` promises multiple files; `file_from_args` takes the first only. Use `%f`. |
| 30 | done | `src-tauri/tauri.conf.json:75` | deb/rpm postinst updates mime/desktop DBs; no post-remove counterpart. |
| 31 | done | `src-tauri/THIRD-PARTY-LICENSES.md:57` | Direct-dependency list omits `tauri-plugin-updater`. |
| 32 | done | `src/base.css:809` | `#tabs .tab-close:hover` keeps a `rgba(128,128,128,0.3)` fill; × fails AA (~3.4:1) on both Solarized themes. Drop the fill like the other hovers. |
| 33 | done | `src/base.css:1288` | **Decided:** bump `minimumSystemVersion` to 13.0 and delete the `@supports not (color-mix)` block entirely (covers #34). `@supports not (color-mix)` fallback sets `color`/`opacity` on elements, clobbering the opaque `--ui-fg-muted` literals Sakura/Solarized define (macOS 10.15 / Safari 15). Redefine the token in `:root` inside the block instead; also removes the hand-maintained selector list, which already misses `#tabs .tab-close`. |
| 34 | done | `src/base.css:776` | **Decided:** resolved by #33 (fallback block removed). `.tab:hover { border-bottom-color: color-mix(…) }` is invalid-at-computed-value on engines without color-mix → falls to `currentColor`, louder than the active tab. Add `.tab:hover { border-bottom-color: transparent }` to the fallback block. |
| 35 | done | `src/base.css:780` | `.tab.active { background: var(--page-bg) }` has no fallback; if `applyTheme` fails, `--page-bg` is never set and the active tab loses its fill for the session. Use `var(--page-bg, rgba(128,128,128,0.22))`. |
| 36 | done | `src-tauri/themes/README.md:74` | Active tab now depends on the theme painting `body { background }` distinct from `--ui-bg`; the theme contract doesn't say so and `_template.css` doesn't require it. |
| 37 | done | `src/base.css:417` | `.tree-row:hover` accent ring also hits the non-clickable `.tree-row.error`. Use `:not(.error)`. |

## Clean (checked, nothing to report)

- `watch.rs`: handle/thread lifetimes, replace-before-drop, disconnected-break.
- `session.rs`: version gating, single consumption, condvar timeout, `awaiting` cleared on destroy. No lock-order inversion anywhere in `AppState`.
- `hwnd_at` / `window_at` unsafe: null check, physical coords, consistent scale.
- Panic surface: every `usize` from the webview is bounds-checked; slices land on ASCII bytes.
- XSS: comrak `unsafe_` off, `tagfilter` on, anchors rebuilt only through `is_safe_id`; CSP `script-src 'self'`. No path from a viewed `.md` to IPC.
- `toggle_task` encoding: BOM, CRLF, bare `\r`, non-UTF-8 refusal all correct.
- `path_for` traversal: `..`, `/`, `\`, `:` rejected.
- `hooks.nsi`: register/unregister symmetric, paths quoted, `SHELL_CONTEXT` matches `currentUser`.
- Updater plumbing: pubkey, HTTPS endpoint, `.sig` staging, `latest.json` keys match `installable()`.
- README/SECURITY claims match code.

## Audit of the decided fixes

Second pass over the decisions, looking for regressions the fix itself would cause.

| # | Ref | Note |
|---|-----|------|
| A1 | #1 | **Launching relative links is a code-execution path.** A Rust `open_path` command that opens any path with its default app means `[notes](../tools/setup.bat)` (or `.exe`, `.lnk`, `.ps1`) runs on one click, from a document the user did not write. Today that branch is dead, so this would be a new exposure. Revised: reveal the file in the file manager instead (`app.opener().reveal_item_in_dir`), which is what a viewer should do with a file it cannot show. |
| A2 | #6 | **Reviewer's fix does not close the race.** `open_path` does (A) read `ready`, (B) claim `pending`; `take_pending` does (D) insert `ready`, (C) remove `pending`. The losing interleave is A, D, C, B. Swapping to C-then-D gives A, C, D, B: still lost. Only making A+B atomic against C+D fixes it: one mutex around both maps (`Mutex<Boot { ready, pending }>`). No lock order to get wrong because there is only one lock. |
| A3 | #18 | **Cap must not apply to images.** `locate()` is shared with `load_asset`; a 32 MB cap there refuses large PNG/SVG files. Put the check in `load_file` only (`toggle_task`/`section_source` re-read the same file the user already has open). |
| A4 | #16 | **`force` must bypass the negative cache.** Once "no update" is cached, the Settings "Check now" button (`force = true`) must still hit the network; only the automatic per-window check should short-circuit. |
| A5 | #28 | **CSS keys the chevron on the div.** `base.css:450` is `.tree-row[aria-expanded="true"] .tree-twist`. Moving `aria-expanded` to the `li[role=treeitem]` breaks the twist unless that selector moves with it (`li[aria-expanded="true"] > .tree-row .tree-twist`). |
| A6 | #25 | **Keep the hash, don't switch to `scrollIntoView`.** `base.css:1280` styles `.markdown-body :target`, which only matches via `location.hash`. The fix stays as "clear the fragment with `replaceState`, then assign `location.hash`" so `:target` keeps working. |
| A7 | #23 | **Serialise, don't just await once.** `onDragMove` is a sync pointer handler; awaiting one probe inside it reorders later moves. Chain every drag IPC through one promise (`drag.ipc = drag.ipc.then(() => invoke(...))`) so `drag_over`, `drag_cancel`, `drop_tab` always land in issue order. |
| A8 | #8 | **Add an unreferenced-footnote test.** A raw-HTML node inside a footnote definition that is never referenced is not rendered either, so it emits no placeholder. Same class of desync as image alt text; the target-collection walk should skip `FootnoteDefinition` nodes that comrak dropped, or the test should prove comrak still emits them. |
| A9 | #29 | **`%f` yields windows, not tabs, during boot.** The second instance arrives while the first is still booting; `open_path`'s not-ready branch finds `pending` already claimed by argv and spawns a new window. Multi-select "Open With" on Linux gives N windows. Acceptable for now; if it grates, the alternative in #29 (open every argv match as a tab) is the fix. |
| A10 | #7 | **Worker thread needs the source label.** `spawn_window` doesn't know which window to send `tab-spawn-failed` to; `drop_tab` has to pass `window.label()` through. Failures on the `open_path` / restore-session spawn paths have no tab to roll back and stay `eprintln!`-only. |
| A11 | #3, #7, #10, #24 | All touch `removeTab` / `onDragEnd` / `onDragCancel`. Land them in one commit so the drag flow is reviewed as a whole. |
| A12 | #33 | Only `tauri.conf.json:70` names 10.15; README and the release skill don't, so the bump is a one-line config change plus the CSS deletion. |

Verified clean: #5's `x` is window-local logical px from `window_at`, matching `insertionIndex`'s `clientX`; #4's watcher filter counts `Modify`, so an in-place byte write still re-renders; #11's extra bump only affects `?v=` on an already-local URL.

## Decisions summary

All 37 items decided 2026-09-12. `wontfix`: #14, #17, #21 (each gets a comment naming the limit). Everything else `done`.

## Implementation

Landed on branch `review-fixes`, seven commits `57ec447..7224c0e`, 88 Rust tests passing, `node --check` clean. Smoke-tested in the debug app: external links, reveal-in-Explorer, task-box byte write, background-tab close, dotted theme name, missing-theme fallback, 32 MB cap — all pass. Not driven (CDP cannot cross windows): cross-window drop position (#5), tear-off rollback (#7), tear-off placement on HiDPI (#15); UNC share unreachable on the test machine (#2, traced by hand instead).

Final review parked two things by ruling: `toggle_task` can NUL-pad if an editor truncates the file in the microseconds between read and seek (pre-change behaviour was worse); `check_for_update` still issues one request per window when N windows boot together (the negative cache only helps later windows). Two concurrent config saves are atomic each but last-writer-wins as a whole; noted for open-items.

## Suggested order

1. #1, #2, #3 — each ≤10 lines, user-visible.
2. #4, #5, #6, #7, #10, #11 — data loss, lost-tab/lost-open, scroll/stale-image paths.
3. #8, #9, #19, #20 — renderer and theme correctness, with regression tests.
4. #12, #13, #29, #30, #31 — release/CI/packaging.
5. #15, #16, #18, #22–#28 — small frontend/backend fixes.
6. #32, #33/#34, #35, #36, #37 — CSS + theme contract, one commit.
