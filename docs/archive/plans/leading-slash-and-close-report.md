# Leading-Slash Links and the Close Report Implementation Plan

> **Status (2026-09-25):** executed in full — 320dcaa (A), 21384f3 (B). One deviation, in Task 3
> only: the scratchpad's `probe-close.mjs` checked liveness with `tasklist | find`, which
> reported the app gone while it was still running, so B1 and B2 were rerun with a probe that
> watches the child's exit and posts `WM_CLOSE` to the window alone. Results are in
> `../../closed-items.md` under *Bugs*; paths and unticked boxes below are as they stood when it
> was written.

> **For agentic workers:** Execute per the **Execution** section below — it says who runs each task and in what order, and it overrides the one-subagent-per-task default of superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lift two accepted limits: a link or picture written `/docs/guide.md` is taken from the repository's root, and a window closed within the 500 ms report wait keeps its last scroll, move or tab change.

**Architecture:** Two independent tasks. **A:** `load_file` finds the repository a document sits in (the nearest folder above holding `.git`), returns it as `repo`, and adds it to the asset scope; `resolvePath` resolves a leading `/` against it. **B:** the page registers `onCloseRequested`, so Tauri holds each close until the page has sent its pending report; Rust destroys the window anyway if the page does not answer within `CLOSE_WAIT`.

**Tech Stack:** Rust + Tauri v2, vanilla JS (no bundler, no frontend test harness), `drive-app` for what only a running window shows.

**Spec:** `docs/open-items.md` → *Accepted limits*: "A link with a leading slash … is resolved against the document's folder" (#8 of the 2026-09-20 review) and "A report waits 500 ms for things to settle … quitting inside that wait loses the last of it" (session restore, 1.6.3).

## Findings (measured 2026-09-25, before this plan)

- **B is real.** Debug build, `"reopen": "restore"`, one window on `README.md`: `scrollTo(0, 3000)`, then a normal close (`WM_CLOSE`, what the X button sends) 50 ms later. The window was gone in 504 ms and `session.json` still said `scrollY: 0`. `scratchpad/probe-close.mjs` reproduces it; Task 3 reuses it.
- **B's mechanism exists.** Tauri 2.11.6 `manager/window.rs:171`: on `CloseRequested`, `if window.has_js_listener("tauri://close-requested") { api.prevent_close() }`, then emits the event. The JS API's `onCloseRequested` awaits its handler and then calls `destroy()`, which needs the `core:window:allow-destroy` permission (`capabilities/default.json` has `allow-close` but not `allow-destroy`).
- **B needs a way out.** A page that cannot answer — a document still laying out, the #9 kind of stall — would otherwise leave a window the X button cannot close. Hence the Rust-side `CLOSE_WAIT`. `Window::destroy` (tauri-runtime-wry 2.11.4, `lib.rs:2283`) posts `Destroy` for the window's own `window_id` through the event-loop proxy, so it is safe off the main thread, and `on_window_close` (`lib.rs:4469`) ignores an id already gone: the fallback cannot touch a later window, and a window the page already closed costs nothing.
- **A's pictures need the scope.** `load_file` (`main.rs:549`) allows the asset protocol only the document's own folder, recursively. A `/assets/logo.png` resolved to a repository root *above* the document would still not load unless that root is allowed too.
- **A's current rule is deliberate and documented** at `ABSOLUTE_PATH` (`app.js:120-133`): a leading slash is resolved against the document's folder, which is right only for a document at the repository's root. Nothing in the README states it. `packTab` does not carry `dir`, so a new `root` field is recomputed on every load and never persisted.

## Decisions (made in this plan; the owner can overrule at review)

- **Repository root = the nearest folder at or above the document with a `.git` entry**, a folder or a file (worktrees and submodules write `.git` as a file). No `.git` anywhere up the tree: behaviour stays exactly as today.
- **Never the home folder or a filesystem root.** A dotfiles repository in the home folder would otherwise make every document under it resolve `/` from the home folder, and put the whole home folder in the asset scope.
- **The asset scope grows to the repository root.** Every picture in the repository becomes loadable by a document in it. Nothing carries it off the machine: comrak's `unsafe_` is off, so a document brings no script of its own (`script-src 'self'`), and `connect-src` falls back to `default-src 'self'`. That is the same argument the existing scope comment makes for the document's folder.
- **`CLOSE_WAIT` = 2 s.** Long enough for a report round trip on a busy machine, short enough that a hung window still closes promptly.
- **Only `WindowEvent::CloseRequested` closes are covered.** An app-level exit (macOS Cmd+Q), a process kill or an OS shutdown still loses whatever was inside the 500 ms wait. That residue stays an accepted limit, reworded.

## Global Constraints

- Surgical changes only. Match the surrounding comment style (full-sentence "why" comments) and naming.
- No new dependencies.
- Gates, all run from the repo root, all must pass before each commit:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --check` (CI runs this on the Linux leg only — run it every time)
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`
  - `node --check src/app.js`
- All commits go straight onto `main`; no feature branches. Commit locally only. **Never push.** User-facing subjects have no prefix; `docs:` for the close-out.
- End every commit message with the executing model's `Co-Authored-By:` line.
- Bash on Windows: never `cd` in a compound command that writes; use `--manifest-path` / `git -C`. Prefix `rtk proxy` for raw output.
- The debug build shares `%APPDATA%\t4-markdown-viewer\` with the owner's installed viewer, which may be running: never close it. Back up `config.json` and `session.json` before any launch and restore them after, as the probes do.

## Review Focus

1. **A document not in a repository:** `/x.md` must resolve exactly as today (the document's folder). Task 1 Step 1 (`None` cases) and Task 3 A3.
2. **A repository in the home folder** (dotfiles): no root, no widened scope. Task 1 Step 1 (`at_home`).
3. **A worktree or submodule**, where `.git` is a file: still a repository. Task 1 Step 1 (`worktree`).
4. **A page that never answers the close:** the window must still close, within `CLOSE_WAIT`. Task 3 B2.
5. **An empty window closing**, such as the "Ask" screen before anything opens. It now reports no tabs on the way out, and that must not replace the session waiting to be offered. Checked in the code: `remember` (`session.rs:287`) writes nothing when no window has a tab and nothing has just closed.
6. **The tab-drag hand-off** (`onDragEnd` reports, then `appWindow.close()`): the close handler reports a second time; it must still close promptly and not write the moved tab back. Task 3 B3.

## Execution

| Phase | Work | Who |
|---|---|---|
| 0 | Commit this plan (`docs: plan leading-slash links and the close report`). | main session |
| 1 | Task 1 (A), one commit | main session |
| 2 | Task 2 (B), one commit | main session |
| 3 | Task 3: drive both on the debug build | main session (it has the probes and the drive-app skill loaded) |
| 4 | Task 4: close-out docs | main session |

Serial. Stop and ask the owner only if: B1 still records `scrollY: 0` after Task 2; B2's window is still alive after 5 s; A1/A2 show the picture not loading after Task 1.

## File Map

| File | Task | What changes |
|---|---|---|
| `src-tauri/src/main.rs` | 1, 2 | `repo_root` + test; `Document.root`; scope on the root (1). `CLOSE_WAIT` and the fallback in `CloseRequested` (2) |
| `src/app.js` | 1, 2 | `resolvePath(dir, rel, repo)`, `ABSOLUTE_PATH` comment, `resolveMedia` and link click pass the repository, `tab.repo` (1). `onCloseRequested` (2) |
| `README.md` | 1 | One paragraph on the `/` rule |
| `src-tauri/capabilities/default.json` | 2 | `core:window:allow-destroy` |
| `docs/open-items.md`, `docs/closed-items.md` | 4 | Two limits lifted; the picture limit and the 500 ms limit reworded to what is left |
| `docs/plans/…` → `docs/archive/plans/` | 4 | This plan, archived |

---

### Task 1: A leading slash is taken from the repository's root

**Files:** `src-tauri/src/main.rs` (`Document` at 150-161, `load_file` at 523-566, tests), `src/app.js` (120-133, 155-183, 330-346, 447, 679, 2392)

**Interfaces:**
- Produces: `fn repo_root(dir: &Path, home: Option<&Path>) -> Option<PathBuf>`; `Document.repo: Option<String>` (JSON `null` when absent); `resolvePath(dir, rel, repo)` — `repo` optional, falsy means today's behaviour.
- Naming: `repo`, not `root` — in `app.js` a root is already the sidebar's (`tab.folder`, `resolveMedia(root, …)`, `renderTree`).

- [ ] **Step 1: Failing test** (append inside `main.rs`'s `mod tests`):

```rust
    /// A leading slash is named from the repository: the nearest folder at or
    /// above the document holding `.git`, as a folder or as a worktree's file —
    /// and never the home folder, where a dotfiles repository would otherwise
    /// claim every document under it.
    #[test]
    fn repo_root_is_the_nearest_folder_with_git() {
        let base = std::env::temp_dir().join(format!("t4-repo-root-{}", std::process::id()));
        let docs = base.join("repo").join("docs").join("sub");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::create_dir_all(base.join("repo").join(".git")).unwrap();
        std::fs::create_dir_all(base.join("plain").join("docs")).unwrap();
        std::fs::create_dir_all(base.join("wt").join("docs")).unwrap();
        std::fs::write(base.join("wt").join(".git"), "gitdir: elsewhere\n").unwrap();

        // `base` stands in for the home folder, so nothing above it is searched.
        let found = repo_root(&docs, Some(&base));
        let plain = repo_root(&base.join("plain").join("docs"), Some(&base));
        let worktree = repo_root(&base.join("wt").join("docs"), Some(&base));
        let at_home = repo_root(&docs, Some(&base.join("repo")));
        std::fs::remove_dir_all(&base).unwrap();

        assert_eq!(found, Some(base.join("repo")));
        assert_eq!(plain, None);
        assert_eq!(worktree, Some(base.join("wt")));
        assert_eq!(at_home, None);
    }
```

Run: `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked repo_root`
Expected: does not compile (`repo_root` not found).

- [ ] **Step 2: `repo_root`** — add directly above `load_file`'s doc comment:

```rust
/// The repository a document sits in: the nearest folder at or above `dir`
/// holding a `.git` — a folder, or the file a worktree or submodule has. A link
/// or picture written `/docs/guide.md` is named from there, as GitHub renders
/// it. `None` outside a repository, and never `home` or a filesystem root: a
/// dotfiles repository in the home folder would otherwise hand every document
/// under it the whole home folder, asset scope included.
fn repo_root(dir: &Path, home: Option<&Path>) -> Option<PathBuf> {
    for ancestor in dir.ancestors() {
        if Some(ancestor) == home || ancestor.parent().is_none() {
            return None;
        }
        if ancestor.join(".git").exists() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}
```

Run the Step 1 test. Expected: PASS.

- [ ] **Step 3: `Document.repo` and the scope.** In `struct Document`, after `dir: String,` add:

```rust
    /// The repository the document sits in, for links and pictures written
    /// from its root (`/docs/guide.md`) — see `repo_root`. `null` outside one.
    repo: Option<String>,
```

In `load_file`, replace the scope comment and call (currently lines 543-549) with:

```rust
    // Let the webview load images and other assets sitting next to the document.
    // Recursive on purpose: documents reference `images/foo.png`, and the scope
    // is the only thing that lets those load. A document in a repository gets
    // the repository too, since a picture written `/assets/logo.png` is named
    // from its root. The cost is that the whole subtree stays readable to the
    // webview for the rest of the session; comrak's `unsafe_` being off and the
    // CSP are what keep that from mattering, since no document can talk the
    // webview into fetching anything from it.
    app.asset_protocol_scope().allow_directory(&dir, true).ok();
    // Canonical like `dir`, so the two compare.
    let home = dirs::home_dir().and_then(|h| std::fs::canonicalize(h).ok());
    let repo = repo_root(&dir, home.as_deref());
    if let Some(repo) = &repo {
        app.asset_protocol_scope().allow_directory(repo, true).ok();
    }
```

and in the `Ok(Document { … })`, after `dir: strip_unc(&dir),` add `repo: repo.as_deref().map(strip_unc),`.

- [ ] **Step 4: The page resolves from the root.** In `src/app.js`:

  1. Replace the `ABSOLUTE_PATH` doc comment's second paragraph (from `Not a leading slash.` through `almost nobody writes.`) with:

     ```js
      * Not a leading slash. `/docs/guide.md` is how a repository's README names a
      * file from the repository's root, so `resolvePath` takes it from there — the
      * `repo` Rust found for the document — or, outside a repository, from the
      * document's folder. Reading it as the filesystem's root instead would break
      * those links to fix ones almost nobody writes.
     ```

  2. Replace `resolvePath`'s doc comment and its first four statements (through `const unc = …;`) with:

     ```js
     /**
      * Resolve `rel` against `dir` — or on its own, if it is a full path — collapsing `.` and `..`. Forward-slash output.
      * A leading slash is taken from `repo`, the repository the document sits in, when it has one.
      */
     function resolvePath(dir, rel, repo) {
       const decoded = unescapeHref(rel);
       // A full path stands on its own; only a relative one hangs off a folder.
       const absolute = ABSOLUTE_PATH.test(decoded);
       const base = repo && /^\/(?!\/)/.test(decoded) ? repo : dir;
       const joined = (absolute ? decoded : `${base}/${decoded}`).replace(/\\/g, "/");
       // A network path is `//server/share/...`: both leading slashes belong to the
       // root, and the server and share are part of it too — `..` climbing past them
       // would leave a path no longer pointing at any machine. Only the base can say
       // so; the joined string starts `//` for the POSIX root as well, since the
       // separator lands right behind its own leading slash.
       const unc = !absolute && base.replace(/\\/g, "/").startsWith("//");
     ```

  3. `resolveMedia(root, dir)` → `resolveMedia(root, dir, repo)`, its `resolvePath(dir, raw)` → `resolvePath(dir, raw, repo)`, and in `renderDocument` change `resolveMedia(els.content, doc.dir);` to `resolveMedia(els.content, doc.dir, doc.repo);`.
  4. In `showActive`, after `tab.dir = doc.dir;` add `tab.repo = doc.repo;`. (The image branch needs none: a picture has no links to resolve.)
  5. In the link click handler, change `resolvePath(activeTab()?.dir ?? "", pathPart)` to `resolvePath(activeTab()?.dir ?? "", pathPart, activeTab()?.repo)`.

- [ ] **Step 5: README.** The rule changes what a reader sees and is written down nowhere. After the paragraph at `README.md:256-260` ("Relative `.md` links load in place …"), add:

```markdown
A link or picture starting with `/` — `/docs/guide.md`, `/assets/logo.png` — is
taken from the root of the git repository the document sits in, as on GitHub.
Outside a repository it is taken from the document's own folder.
```

- [ ] **Step 6: Gates**, then commit:

```bash
git -C . add src-tauri/src/main.rs src/app.js README.md
git -C . commit -m "Take a link that starts with / from the repository's root" -m "Co-Authored-By: <executing model line>"
```

### Task 2: A close waits for the last report

**Files:** `src/app.js` (after `appWindow.onMoved(…)`, ~line 3119), `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs` (`CloseRequested` arm ~1430, a constant)

- [ ] **Step 1: The page reports before it goes.** In `app.js`, directly after `appWindow.onMoved(scheduleReport).catch(console.error);` add:

```js
  // A close waits for this window's last report. Anything still inside
  // REPORT_DELAY — a scroll, a move, a tab change hard on another — would
  // otherwise go with the window. Tauri holds the close while a listener is
  // registered and destroys the window once it resolves; Rust closes it
  // anyway if the page cannot answer (`CLOSE_WAIT`). Registered with the
  // others, after boot: a window still restoring has nothing to add.
  appWindow.onCloseRequested(() => reportSession()).catch(console.error);
```

- [ ] **Step 2: Permission.** In `capabilities/default.json`, after `"core:window:allow-close",` add `"core:window:allow-destroy",`. `onCloseRequested` closes the window with `destroy()`.

- [ ] **Step 3: The way out.** In `main.rs`, between `MAX_DOCUMENT_BYTES` and `struct Boot`'s doc comment, add:

```rust
/// How long a close waits for the page before the window goes anyway. The page
/// holds each close to send its last report (`onCloseRequested` in app.js); one
/// too busy to answer — a document still laying out — must not leave a window
/// the X button cannot close.
const CLOSE_WAIT: Duration = Duration::from_secs(2);
```

and change `use std::time::Instant;` to `use std::time::{Duration, Instant};`. Then replace the `CloseRequested` arm with:

```rust
                // The last moment the window can say where it stands; by
                // `Destroyed` it is gone, and the chain needs its frame.
                WindowEvent::CloseRequested { .. } => {
                    session::note_frame(window.app_handle(), window.label());
                    // The page holds the close to send its last report and
                    // then destroys the window itself; this is for a page
                    // that never does. The handle names this window, not its
                    // label, so one already gone is left alone.
                    let window = window.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(CLOSE_WAIT);
                        let _ = window.destroy();
                    });
                }
```

- [ ] **Step 4: Two comments this makes wrong.** Each says a closing window sends nothing. That is now true only of a quit that skips the windows (macOS Cmd+Q), a kill or a crash.
  - `app.js:853` (`reportSoon`): change "Nothing fires as the last window goes, so a tab closed just before quitting has to be on disk already or it comes back" to "Nothing fires on a Cmd+Q or a kill, so a tab closed just before one has to be on disk already or it comes back".
  - `main.rs:886-888` (`set_session`): change "there is no event for the last window going, so the file has to be current before it does" to "a close waits for the page's last report, but a Cmd+Q or a kill waits for nothing, so the file has to be current before either".

  Leave `session.rs:254` alone: "nothing fires as the app quits" is about the app, and it still holds.

- [ ] **Step 5: Gates**, then commit:

```bash
git -C . add src/app.js src-tauri/capabilities/default.json src-tauri/src/main.rs
git -C . commit -m "Keep a window's last change when it closes straight after it" -m "Co-Authored-By: <executing model line>"
```

### Task 3: Drive both on the debug build

Build once with the `TAURI_CONFIG` identifier override (drive-app skill). Leave the installed viewer running.

- [ ] **A1 — a link from the root.** In the scratchpad, build `repo/.git/` (an empty folder), `repo/assets/pic.png` (copy any PNG from `examples/img/`), `repo/docs/guide.md` (`# Guide`), and `repo/docs/sub/page.md` containing `[guide](/docs/guide.md)` and `![pic](/assets/pic.png)`. Launch with `"reopen": "off"` (backups first), `openTab` on `page.md`. Report `eval "[resolvePath(activeTab().dir, '/docs/guide.md', activeTab().repo), activeTab().repo]"`, then `click` the link and report `activeTab().label`. Expected: `repo` is the scratch `repo` folder; the label becomes `guide.md`.
- [ ] **A2 — a picture from the root.** Back on `page.md`: report `document.querySelector('#content img').naturalWidth`. Expected: greater than 0.
- [ ] **A3 — outside a repository, as before.** Copy `page.md` to a scratch folder with no `.git` above it (under the scratchpad, whose parents have none — check `ls`), open it, and report the same `resolvePath` eval. Expected: `repo` is `null` and `/docs/guide.md` resolves under the document's own folder.
- [ ] **B1 — the lost scroll comes back.** `node <scratchpad>/probe-close.mjs fixed`. Expected: `saved scrollY=3000`.
- [ ] **B2 — a hung page still closes.** `node <scratchpad>/probe-close.mjs hung hang`. Expected: `still alive: false`, closed in roughly 2000–2500 ms.
- [ ] **B3 — the tab hand-off.** With two windows (`invoke("open_window", {path})`), drag `main`'s only tab into `w1` (drive-app cross-window recipe). Expected: `main` closes within ~1 s of the drop, and a moment later `session.json` holds one window with the tab.
- [ ] **Restore** `config.json` and `session.json` from the backups and `cmp` them.

### Task 4: Close out

- [ ] **Step 1:** In `open-items.md` *Accepted limits*: remove the leading-slash bullet; reword the picture bullet to "A picture named by a full path outside any folder a tab has been opened from, or the repository it sits in, does not show …"; reword the 500 ms bullet to what is left: "An app-level exit (macOS Cmd+Q), a process kill or a shutdown still loses whatever was inside the 500 ms report wait; closing a window no longer does (<Task 2 hash>)."
- [ ] **Step 2:** Add both lifted limits to `closed-items.md`, *Bugs*, ticked, each with its commit hash and Task 3's values.
- [ ] **Step 3:** `git mv` this plan to `docs/archive/plans/` with a status line under the title, and commit: `docs: close the leading-slash and close-report limits`.
