# Full codebase review — 2026-09-20

> **Status (2026-09-21):** closed. Every item is `done`, `wontfix` or `park`: 26 findings are
> `done`, on `main` as the thirteen commits d952d02 through 47e8c0d (squashed by feature); #23–#26 are
> `wontfix`; #28 was parked, then fixed on 2026-09-24 (`ff2eef6`). #9 was to be parked
> behind its stopgap (`d952d02`), but the smoke
> pass's check Z — the 750 KB file from #1, 33.9 MB of HTML, under the ceiling on purpose —
> hung the webview for more than 140 s, so the owner promoted it and it was fixed the next day
> (`../plans/json-fold-weight.md`). Not driven in the app: #12 and the tab tear-off regression check
> (`cdp.mjs` cannot drag), and #16's arrow-key panning (it cannot send real key presses; the
> panel does take focus). Unverified by construction: #5's Rust guard and its `update-failed`
> broadcast (need a real update), #6 (needs a non-UTF-8 argv on Linux), #27 on macOS (the
> test bites on the Windows and macOS CI legs only), and #30–#31 (`checks.yml` is first
> exercised by the next push, the release parts by the next tag).

Reviewed at `a033113` (Release 1.6.5). Five parallel reviewers: Rust IPC / fs / config /
updater; Rust render / JSON / themes / session; frontend (`app.js`, `index.html`); CI /
packaging / dependencies; and an adversarial verifier for the ten findings `/code-review`
raised against e219293 ("Remember the sidebar's folder and sort for each tab").

How far each finding was checked:

- #1 and #2 were measured by the reviewer with out-of-tree harnesses (timings below) and
  then re-read by hand against `json.rs`.
- Everything marked *confirmed* was traced end to end in the code by its reviewer.
  *Plausible* means it depends on something that was not checked; the entry says what.
- The render/session reviewer also fuzzed `render.rs` (30k documents) and `json.rs` (40k
  documents, every `more` button followed): no panic, exact round trip.
- Nobody drove the app. Every frontend finding is from reading.

Status column: `open` / `fix` / `wontfix` / `park` / `done`. `park` is deferred, not
accepted: a parked finding becomes an unticked item in `../../open-items.md` with the condition
that reopens it. `wontfix` is final. Everything starts `open`; the
**Proposed** word at the head of each finding is the reviewer's recommendation, for the owner
to confirm or flip. The plan for the proposed fixes is
`../plans/code-review-2026-09-20-fixes.md`.

## High

| # | Status | Where | Finding |
|---|--------|-------|---------|
| 1 | done (`d952d02`) | `src-tauri/src/json.rs:406-422`, `:519` | **Proposed: fix.** **Trailing commas nested deep make the JSON render quadratic, and defeat chunking.** Past the chunk budget, a comma that fails `has_content` (a JSONC trailing comma right before its closer) carries on to the next comma — and each one calls `container_ends`, which scans forward until it has found a closer for *every* open container. N nested trailing commas cost O(depth²) and no cut is ever made. Measured (release): `[\n`×d + `1` + `,\n]`×d renders in 7.5 s at 750 KB and 270 s at 1.5 MB, with zero `more` buttons; the same size with a value after each comma takes 64 ms. **Fix:** the `has_content` question only needs the first non-whitespace byte after the comma (is it the innermost closer?), so answer it locally and call `container_ends` only in the branch that cuts. |
| 2 | done (`d952d02`) | `src-tauri/src/json.rs:280` (`indent`), `:201` (`source`) | **Proposed: fix.** **The one-line reflow is quadratic in nesting depth and unbounded.** `indent` writes `2 × depth` spaces at every opener, closer and comma, before `emit`'s budget can help. A single-line `[[[[…` measured 1 KB → 1 MB, 2 KB → 4 MB, 8 KB → 64 MB (exactly n²); `{"a":{"a":…` at depth 4000 is 24 KB → 32 MB; about 100 KB of that shape asks for ~10 GB and aborts the process. Ordinary minified JSON is fine (7.3 MB reflows 1.96× in 19 ms). **Fix:** give up on the reflow (`Cow::Borrowed(text)`) once the output passes 8× the input (floor 1 MB, so small files are never affected). **Not checked:** whether session restore reopens the file that killed the app, which would make this a crash loop. Deferred as a question of its own: tracked in `../../open-items.md` (plan Task 11 Step 2a). |

## Medium

| # | Status | Where | Finding |
|---|--------|-------|---------|
| 3 | done (`d4f11c1`) | `src-tauri/src/main.rs:492`, `:580`, `:598`, `:657`, `:732` | **Proposed: fix.** **Read commands run on the main thread and stall every window.** `load_file`, `section_source`, `json_region`, `load_asset` and `list_dir` are plain `#[tauri::command]`, which Tauri 2 runs inline on the event loop (`tauri-macros` defaults to `ExecutionContext::Blocking`; the repo says so itself at `picker_dir`). A 30 MB `.txt`, a 10k-entry folder (one `stat` each) or `canonicalize` on a dead UNC path holds up every window — and turns #1/#2 into app-wide freezes. Also the cost behind slow tab switches between folders (1+N `list_dir` calls). **Fix:** `#[tauri::command(async)]` on those five. **Not** `watch_files` / `watch_folders` (main-thread serialisation is what stops two rapid calls installing the older watcher last) and not `set_session` (`session::remember` is main-thread only). The `MAX_DOCUMENT_BYTES` comment ("the window is frozen") needs rewording with it. |
| 4 | done (`0cbadf1`) | `src/app.js:262` (`rememberScroll`), `:1597` (`refresh`) | **Proposed: fix.** **Scroll offset banked onto the wrong tab during a load.** `rememberScroll` writes `window.scrollY` onto `currentEntry(activeTab())` without checking that the page on screen belongs to that entry. The scroll listener got this guard in 1.6.3 (`shownToken !== renderToken`); the function itself did not. Ctrl+Tab A→B, then again →C before B's `load_file` resolves: `activateTab(C)` banks A's offset onto B, permanently. `refresh()` has the same hole: a `file-changed` or F5 mid-switch hands the incoming document the outgoing page's `window.scrollY`. **A plausible cause of the unreproduced "restored scroll position degraded once" item in `open-items.md`.** **Fix:** the same guard at the top of `rememberScroll`; `refresh` passes `window.scrollY` only when the page on screen is the entry's. |
| 5 | done (`769cd54`) | `src-tauri/src/update.rs:97`, `src/app.js:1695`, `:1715` | **Decided 2026-09-20: fix on both sides** — the Rust guard below, plus a frontend `installing` flag fed by the broadcast `update-progress` (and cleared by a new `update-failed` event), so a reopened dialog or another window shows the install under way instead of offering a second. **An update can be installed twice.** `install_update` has no in-flight guard and the "downloading" state is per-window DOM. Two windows each pressing **Update now**, or one window doing Update now → Later → reopen dialog (`showUpdateDialog` re-enables the button) → Update now, runs two `check` + `download` + `snapshot` + `install` sequences; on Windows, two NSIS runs over one install directory. *Plausible* as to damage: the updater plugin's temp-file naming was not traced. **Fix:** a dedicated `AtomicBool` around the command; a second call gets "An update is already being installed.", which the dialog already knows how to show. Not `AppState::installing` — setting that at entry would stand ordinary session saves down for the whole download. The frontend flag the reviewer also suggested is skipped: the Rust guard makes the second click harmless and truthful. |
| 6 | done (`0d08e71`) | `src-tauri/src/main.rs:1389`, `src-tauri/src/session.rs:236` | **Proposed: fix.** **A non-UTF-8 argument aborts startup silently.** `std::env::args()` panics on one, and `panic = "abort"` makes that an exit with no window and no message. Linux: a file manager opens `notes-caf\xe9.md` (Latin-1 bytes) via `Exec=… %f` and the app never starts. **Fix:** `args_os()` + `to_string_lossy()`; the mangled path then fails `file_from_args`' `is_file()` and is ignored. |
| 7 | done (`6cfb06a`) | `src-tauri/src/watch.rs:86-98` | **Proposed: fix.** **A file written faster than the debounce never refreshes.** The drain loop waits for 150 ms of quiet with no overall deadline. A `.txt` log appended every ~100 ms never emits `file-changed`; the sidebar stays stale for the whole of a large folder copy. **Fix:** cap the drain at one second, so a burst is flushed at least that often. |
| 8 | done (`ae50077`) | `src/app.js:163` (`isRelative`), `:131` (`resolvePath`), `:2276`, `:301` | **Proposed: fix.** **Absolute local paths in links and images never work.** `/home/u/spec.md` passes `isRelative` and is joined onto the document's folder ("Not a file" page, pushed onto history); `C:\notes\spec.md` matches `HAS_SCHEME` (`c:`) and goes to `openUrl`, which the capability rejects (toast). Same for `![](/abs/pic.png)` at `:301`. **Fix (drive-letter paths only):** recognise `C:\…` / `C:/…` before the scheme test and have `resolvePath` use it as it stands. **A leading slash is deliberately left as it is:** `/docs/guide.md` is how a repository's README names a file from the repository root, and today's behaviour — resolving it against the document's folder — is what makes those links work for a document at the root. Reading it as the filesystem root would break them to fix a form almost nobody writes. The reviewer's Linux example (`/home/u/spec.md`) therefore stays unfixed, as an accepted limit. **UNC (`\\host\…`, `//host/…`) stays excluded on purpose:** an image is fetched without a click, and a document that could name `\\evil\share\x.png` could make Windows offer the reader's NTLM hash to a server of the author's choosing. |
| 9 | done (`d952d02`) | `src-tauri/src/json.rs:304`, `:481` | **Fixed 2026-09-21, and not by bounding the output:** two spikes found the webview pays for lines × the fold spans open around them, not for size, so a render too heavy to fold is now made without fold markup (`MAX_FOLD_WEIGHT`; `../plans/json-fold-weight.md`). Check Z's file opens in about a second. *Before that —* **Decided 2026-09-21: the real fix is not parked.** Check Z opened #1's 750 KB file as a tab: Rust rendered it in 3.35 s to 33.9 MB of HTML, and the webview then answered nothing for more than 140 s and was killed. The stopgap below stands; the real fix is an open bug in `../../open-items.md`. *Before that —* **Decided 2026-09-20: stopgap now, real fix parked** (plan Task 1a). **JSON HTML output is not bounded, only source bytes are.** `finish` writes a `more` button per open container and every bracket gets a fold control, so a 900 KB deeply nested document renders to 38 MB of HTML with 77,544 buttons. Linear in Rust — but with #1–#3 fixed the cost moves to the webview, which is handed all of it; whether it copes is unmeasured (plan Task 10 check Z). HTML runs to roughly 40–70× the source for short tokens even in ordinary files, so an 8 MB `extent` can already be hundreds of MB. **Stopgap:** a 64 MB ceiling on one render — above the most a legitimate first chunk can reach (~36 MB), so no ordinary file is refused; a far-expanded re-render falls back to the first chunk; only a document with nowhere to cut is turned away, with an error page. It does not stop the 34–38 MB examples here. **Real fix:** end a chunk on output as well as on source, in a way a re-render can reproduce. |
| 10 | done (`74f2192`) | `src/app.js:1239`, `:1470`, `:2681`, `:2689` | **Proposed: fix.** **Sidebar choices made while a tab is loading land on the tab being left.** `sidebarTab` only changes in `followTab`, which runs at the *end* of `showActive` (`:664`), so for the whole load it is still the previous tab. A sort or filter change in that window is written to the old tab and then reverted by `followTab`; on a window's first tab `sidebarTab` is null, so the sort is written to config and then reverted for the window. The folder picker is only affected if the native dialog finishes inside the load window. **Fix:** those writes (and `openFolder`'s `owner`) go to `activeTab() ?? sidebarTab`. |

## Low

| # | Status | Where | Finding |
|---|--------|-------|---------|
| 11 | done (`89dc37f`) | `src/app.js:2560` | **Proposed: fix.** **AltGr triggers Ctrl shortcuts.** Windows reports AltGr as Ctrl+Alt, so on German / Nordic / Central-European layouts AltGr+8 (`[`) navigates Back and AltGr+9 (`]`) Forward instead of typing, filter box included. No shortcut uses Ctrl+Alt. **Fix:** `if (!ctrl \|\| event.altKey) return;`. |
| 12 | done (`0cbadf1`) | `src/app.js:1968` | **Proposed: fix.** **An in-strip tab reorder is never saved.** `reorderTo` and `onDragEnd`'s non-detached branch end at `renderTabs()`; nothing calls `reportSoon()`. Drag a tab, quit without touching anything else: the old order is restored and `active` can point at a different tab. **Fix:** `reportSoon()` in that branch. |
| 13 | done (`d952d02`) | `src/app.js:2383-2402` | **Proposed: fix.** **A JSON `more` result can be stamped on the wrong tab.** The guard is `els.content.dataset.path === path` — a path, not a tab. Same file in two tabs, click `more` in one, switch to the other before it resolves: the second tab gains an `extent` it never expanded. If the document re-rendered during the fetch, the detached button's `insertAdjacentHTML` throws. **Fix:** capture the entry before the invoke; bail if the button is no longer in the document. |
| 14 | done (`ae50077`) | `src/app.js:1498-1511` | **Proposed: fix.** **A link to the current document with no fragment jumps to the top with no way back.** It falls through both branches: no history entry, but `showActive(0)` re-renders at the top and the scroll listener then banks 0. **Fix:** re-render in place (`refresh()`) instead, which also keeps "click the open file in the tree to retry a failed load" working. |
| 15 | done (`a7fc23e`) | `src/app.js:549` | **Proposed: fix.** *Plausible.* **Zoom during an image's decode uses the previous picture.** `picture` still describes the old image until `decode()` resolves; Ctrl++ in that gap computes from the old `naturalW` and `rememberImage` writes the scale onto the new entry, which then opens at an arbitrary zoom instead of "fit". **Fix:** `picture = null` at the top of `showImage`. |
| 16 | done (`a7fc23e`) | `src/index.html:97` | **Proposed: fix.** *Plausible.* **A zoomed picture cannot be panned from the keyboard.** `#image-view` is not focusable and panning is pointer-only. Chromium's focusable-scroller behaviour may cover WebView2; WKWebView likely not. **Fix:** `tabindex="0"`. |
| 17 | done (`74f2192`) | `src/app.js:1040` (`renderTree`), callers `:1060`, `:1325` | **Proposed: fix.** **After a root listing error, the next good render comes back collapsed and that is persisted.** `resortTree` and `onFolderChanged` pass no `keep`; the error row left nothing open in the DOM; `syncFolderWatch` then writes `expanded = []` and reports. Realistic trigger: changing the sort while the error row shows. **Fix:** a root render that follows an error seeds `rootKeep` from the owning tab's `expanded`. |
| 18 | done (`74f2192`) | `src/app.js:1262` | **Decided 2026-09-20: fix the close case only.** **`openFolder` closed mid-listing keeps an unverified pick.** `folder`/`picked`/`filter` go onto the tab before the listing (`:1247`); the revert lives inside `if (state.folder === path)`. Pick an unreachable share, close the sidebar before it answers: the dead share is persisted as picked and the working folder is gone. The superseded-by-another-`openFolder` case is left alone: the newer call has already written the tab, and the older one cannot tell whether its own listing failed. |
| 19 | done (`74f2192`) | `src/app.js:1017` | **Proposed: fix.** **`state.folderPicked` goes stale on a same-folder tab switch.** Only `openFolder` writes it, and the same-folder branch of `followTab` never runs `openFolder`. Read only by `openTab` when `sidebarTab` is null: close every tab, open a file in the folder still on show, and the new tab inherits the wrong picked flag. **Fix:** `state.folderPicked = own && tab.picked;` in that branch. |
| 20 | done (`74f2192`) | `src/app.js:1001-1006` | **Proposed: fix.** **`state.folderSort` is committed before the no-folder early return.** A tab whose load failed on a bare filename (`folderFor` → `""`) leaves the tree in the old order while the state and the menu's tick claim the new one, and re-choosing it is a no-op (`setFolderSort`'s equality guard). **Fix:** name the folder and bail before taking the tab's sort. |
| 21 | done (`74f2192`) | `src/app.js:2681`, `:2689` | **Proposed: fix.** **A filter typed against a borrowed folder is saved onto the tab.** While the tab's own folder will not list, the sidebar shows the document's folder (`remember: false`, `:1289`), but the filter handlers write `sidebarTab.filter` regardless — `syncFolderWatch` guards `expanded` against exactly this. When the drive comes back the tab's own folder opens pre-narrowed. **Fix:** the same `sameDir(tab.folder, state.folder)` guard on both writes. |
| 22 | done (`74f2192`) | `src/app.js:2243` | **Proposed: fix.** **Dead option.** `showFolder` only runs with the sidebar closed, and `closeFolder` blanks every tab's filter, so `filter: tab.filter` is always `""`. Drop it. |
| 23 | wontfix | `src-tauri/src/main.rs:1367`, `:1121` | **Proposed: wontfix** (accepted limit). **A file-association open in the instant a window is closing is dropped.** Between `CloseRequested` and `Destroyed` the label is still in `focus_order` and `boot.ready`, so `open_path` emits to a dying window. The suggested fix (drop the label in `CloseRequested`) touches `focus_order`, which `session::save` reads to order the windows it writes — risk to the closed chain for a window milliseconds wide. |
| 24 | wontfix | `src-tauri/src/main.rs:636` (`set_watch`) | **Proposed: wontfix.** **A failed folder watch drops the existing watcher.** Keeping the old handle would not help — a watcher on a share that went away is dead either way — and the next `syncFolderWatch` (any expand, collapse or tab switch) asks again. |
| 25 | wontfix | `src-tauri/src/main.rs:786` | **Proposed: wontfix.** **A folder whose name is not valid UTF-8 comes back mangled** (`to_string_lossy`), and every call on it then fails as "Not a folder". Linux only, reachable only through the picker (`is_visible_entry` hides such entries in the tree), and the outcome is already an error row. |
| 26 | wontfix | `src-tauri/src/main.rs:1086` | **Proposed: wontfix.** **`reveal_path` takes any path.** Not exploitable: the frontend's `resolvePath` was traced and no escape found, and revealing is COM / NSWorkspace / DBus, never an exec. An `exists()` check would not narrow anything — showing an existing file in the file manager *is* the feature. |
| 27 | done (`0b1c86c`) | `src-tauri/src/themes.rs:261` | **Decided 2026-09-20: fix** (plan Task 5a, with a temp-directory test for `scan`). **Theme shadowing compares names case-sensitively**, but `path_for` resolves through the filesystem, which on Windows and macOS is not. `Solarized-Light.css` beside the bundled `solarized-light.css` gives two picker rows, a dead light/dark toggle, and the bundled file unreachable. Needs a user to create a case-variant of a bundled name. |
| 28 | done (`ff2eef6`) | `src-tauri/src/render.rs:47` | **Fixed 2026-09-24.** **Proposed: park.** **Anchor recovery misreads an `id=` inside another attribute's value.** `<a href="p?a id=q" id="x"></a>` recovers no anchor, so `[…](#x)` links to nothing. Every ordinary shape works. |
| 29 | done (`6cb91af`) | `src-tauri/src/main.rs:502-526` | **Decided 2026-09-20: fix** (plan Task 5b: `render_titled` over one parse). **Every markdown open parses the document twice** — `render::render` and then `render::first_heading`, each with its own arena. Pure cost, and off the main thread once #3 lands. |
| 30 | done (`47e8c0d`) | `.github/workflows/release.yml:30`, `:100`, `:280`, `:296`, `:300`, `checks.yml:33` | **Decided 2026-09-20: fix** (plan Task 9a). **`actions/checkout`, `upload-artifact`, `download-artifact` are pinned to tags.** Not against the workflow's rule as written — that (`checks.yml:42`) is "*third-party* actions are pinned to a commit", and these are GitHub's own — but the reason given for the rule (the job holds the signing key, and a tag can be moved) applies to them just the same. Pin all six uses and reword both comments to say every action. |
| 31 | done (`47e8c0d`) | `.github/workflows/release.yml:39` | **Decided 2026-09-20: fix** (plan Task 9a). **The `Cargo.lock` version grep uses `-A1`**, so a lockfile format that put a field between `name` and `version` would break it — loudly, with a mismatch error, before anything is published. Read the first `version` inside the package's own `[[package]]` block instead (`awk`; checked against the current lockfile, gives `1.6.5`). |

## Checked and clean

- **Security.** comrak `unsafe_` off drops raw HTML; `javascript:` / `vbscript:` / `file:` /
  entity-encoded and `data:` URLs come out with an empty `href`/`src`; fence info strings,
  alt/title text and slugs are entity-escaped; recovered anchors carry nothing but an
  `is_safe_id` name. JSON reaches the page only through `escape`, and no document text is put
  in an attribute. Theme CSS goes to `style.textContent`; `valid_name` leaves no traversal.
  CSP is `script-src 'self'` with no `unsafe-inline`; `asset:` appears only in `img-src` /
  `media-src`. Capabilities are tight and `openUrl` is scoped to http / https / mailto / tel.
  The updater has a pubkey and an HTTPS endpoint. No path from a viewed document to a
  privileged command.
- **Panics on external input.** None reachable besides #6. `render_slice` uses `get`,
  `render_to` clamps, `toggle_task` bounds-checks, comrak is iterative (100k-deep nesting
  renders in < 80 ms).
- **Locking, config and session.** No inversion, no lock across an `await` or an event-loop
  round trip. `write_json` is temp + rename; both loaders fall back cleanly. Session version
  gating, the closed chain and the install snapshot all read correctly.
- **Watcher lifetime.** New watcher registers before the old is dropped; `Destroyed` cleans
  both per-window handles; threads exit on channel close.
- **IPC contract.** Every `invoke()` name, argument and result field matches the Rust side, as
  do the event payloads.
- **Frontend hygiene.** Both `innerHTML` sinks take backend HTML; listeners are delegated on
  stable parents; `closedTabs` is capped at 20; index arithmetic in history, tab cycling and
  reorder is right.
- **CI and packaging.** `contents: read` by default, `publish` the only writer, no
  `pull_request_target`, no `${{ github.event.* }}` in a `run:`, no `curl | sh`, scoped temp
  keychain on macOS. Version gate, publish ordering, NSIS hooks, postinst/postrm, `.desktop`
  and the MIME XML all check out. `cargo tree -d` shows only ordinary ecosystem duplicates.
  `cargo audit` is not installed (already accepted in `open-items.md`).

## Rejected

- **`===` vs `sameDir` for folder identity** (`app.js:830`, `:835`, `:1008`, `:2240`; raised by
  `/code-review`). False. Both strings come from the same canonical assignment in
  `openFolder`, and `followTab` re-converges `tab.folder = state.folder` on every same-folder
  switch; `:1008` and `:2240` are identity tests on what `folderFor` returned, not path
  comparisons.
- **Settle tracking through three module globals** (`folderLoads`, `rootEpoch`, `rootKeep`).
  Maintainability only: every root-render caller ends in `syncFolderWatch` today.
- **`showFolder` "drifts" from `followTab`.** The differences (`record`, `picked: !named`)
  are an explicit action versus an automatic follow, and deliberate. Only the dead `filter`
  survives, as #22.

## Not implemented, so not reviewed

Find-in-page, a TOC / outline, print / export, and dropping a file onto the window (a dropped
file is inert, not unsafe).
