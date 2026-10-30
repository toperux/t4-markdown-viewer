# Dependabot #19, #21, #22 and Three Deferred Fixes Plan

> **Status (2026-10-30):** executed in full. #19 rebase-merged (152950d); #21 and #22 taken as
> 64287a2, the render byte-identical on the 8 docs and the snippet. The fixes: 705149a (drop
> caret), a5bb89c (F5 after a timeout), 98c31d0 (renderer failure kept for the window). W1 found a
> bug older than the bumps, fixed as ba7bfb5: `base.css` set every cell `text-align: left`, beating
> the `align` attribute a `:-:` or `-:` column gives its cells. Triage: the squared-width table
> limit went (cd2f764; drawing is linear in cells now, 0.45 s per million, and the padding limit
> stays), and the `zip` advisories closed by taking tauri-plugin-updater 2.14.1 (6611f91), which
> pins zip 9; zip is a Windows-only dependency of the updater. W1-W6 passed on Windows, macOS and
> Linux (headless Xvfb); W6 re-run on 6611f91 passed on all three (the check answers in 0.25-0.35 s).

**Goal:** land the three open Dependabot PRs and fix three deferred items that each have a known
fix, so the next release carries them.

**Spec:** the owner's scope ruling of 2026-10-30 ("A"): the dependency PRs, the three small
fixes below, and a re-measurement of the table limit that comrak 0.58's fix bears on.

## Findings (measured 2026-10-30, before this plan)

### The PRs

All three are based on `373ff8d`, which is `main` today, so their files can be taken as they
are. Their checks passed.

| PR | Bump | What changed upstream | Consequence here |
|---|---|---|---|
| #19 | `Swatinem/rust-cache` 2.9.2 → 2.9.3 | New pin `d1527c8` is `behind` the action's `master`. Between the pins: Dependabot bumps of its own actions, and "stop cleanup timestamp pruning after the first entry" (#377) | `checks.yml` and `release.yml`, one line each. The build legs only, not the publish job, so the release skill asks for no dry run |
| #21 | `rustls` 0.23.45 → 0.23.46 | Patch release | `Cargo.lock` only. The updater plugin's TLS, through `reqwest` |
| #22 | `comrak` 0.57 → 0.59 | **0.58, security:** the HTML renderer looked up each table cell's alignment by walking its earlier siblings, so a row cost the square of its width; a cursor (`TableCellIndexer`) makes it linear. Also Obsidian callouts (`ofm_callouts`) and a parse cancel callback, both off unless asked. **0.59:** `wikilinks_ignore_embeds`, an option on wikilinks, which are off here | `render.rs`'s `options()` turns on neither extension, so output should be unchanged (Task 1's render diff measures it). The quadratic 0.58 removes is what `MAX_TABLE_WORK` (`render.rs:31`, Σ width² per row) guards against, so that limit can be re-measured. `MAX_TABLE_PADDING` guards comrak padding rows to the header width (the 61 GB case); 0.58 doesn't touch that |
| #22 | `notify` 8.2.0 → 8.2.1 | The macOS watcher drops `fsevent-sys` for `objc2-core-services`; FSEvents startup errors now come back from `watch` instead of going unseen; the Linux watcher cleans up deleted paths the kernel already dropped; MSRV 1.85 | File watching is what reloads a document when it changes on disk (`watch.rs`). `watch.rs:81` already skips a directory whose `watch` fails, so a newly surfaced error costs that directory's reload, as before. Local rustc 1.99 and CI's `stable` clear the floor. Task 3 walks live reload on all three platforms |

Lockfile side effects of #22: `fsevent-sys` drops out, `objc2-core-services` 0.3.2 comes in
(`registry+https://github.com/rust-lang/crates.io-index`).

**Out of scope, raised at triage:** Dependabot's security run for `zip` (40945407514) failed.
`zip` 4.6.1 comes only through `tauri-plugin-updater`, which pins `zip = "4"`, and the
advisories cover every version below 9.0.0, so Dependabot can't move it. It waits on an updater
release on `zip` 9. Reasoned from the pin and the advisory ranges; the run's own error isn't in
the public log.

### The fixes

| Item (`docs/open-items.md`) | Where | Fix |
|---|---|---|
| Closing the dragged tab mid-drag may leave another window's drop caret lit | `onDragEnd`, `app.js:4690-4691`: `if (!tab) return;`. Reached only once the drag is detached, and after `drag_over` probes may have lit a caret in another window (`crossWindowDrag`: on in Windows builds, off elsewhere, `main.rs:1312`) | Send `drag_cancel` on that return, as `onDragCancel` does (`app.js:4750`). Rust's `drag_cancel` clears the carets (`main.rs:1234`) |
| F5 doesn't retry a timed-out diagram | `loadRenderer`'s `stop` caches `{ error: "It took too long." }` for every diagram the frame owed (`app.js:590`, via `app.js:1127`). `refresh()` (`app.js:3489`, what F5 runs) reloads the document, and `warmSources` skips every cached key | In `refresh()`, before `showActive()`, delete the cache entries of the blocks on the page whose error is the timeout, under the current look. The blocks are still there: a failed diagram keeps its `pre > code.language-mermaid` (`renderDiagrams`, `app.js:1375`). The message becomes a named constant so both sites agree |
| A renderer that fails to load is tried again for every document | `fail` sets `rendererLoad = null` (`app.js:576`), so the next document with a diagram builds a new frame and waits again: 1 s after the frame's `load` if its bridge says nothing, 15 s if `load` never fires | A `rendererFailed` flag set in `fail`; `loadRenderer` rejects at once while it's set; `redrawDiagrams` (the theme, colour and font path, which already redraws from the page's sources) clears it first. `warmSources` already treats a rejection as "leave the blocks as code" (`app.js:1052-1054`) |

Not measured: whether Ctrl+W closes the dragged tab mid-drag. The handler (`app.js:5658`) has
no drag guard and closes the *active* tab, so the dragged tab must be the active one; whether
the keydown arrives while the bar holds pointer capture is reasoned, not seen. If it doesn't
happen, the first item is unreachable and closes as such, and 2a stays as a guard.

## Global constraints

- No dependency change beyond the PRs'. Never a bare `cargo update`; `rustls` is moved with
  `cargo update -p rustls --precise 0.23.46`, and the result is checked against #21's diff.
- Gates, from the repo root, before each commit: `cargo fmt --manifest-path src-tauri/Cargo.toml
  --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked
  -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`,
  `node --check src/app.js`.
- Straight onto `main`, committed locally; no push without the owner's go. No AI attribution.
  Dependabot closes #21 and #22 by itself once `main` has the versions.
- Walks run on the peers from a pushed `walk/deps3` branch, never on the host PC. Pushing it
  needs the owner's go at the time. Each peer backs up and restores `config.json` and
  `session.json` per the drive-app skill.
- `app.js` has no test harness, so each fix leaves a walk step behind instead.

## Task 0: merge #19

`gh pr merge 19 --rebase` keeps Dependabot's `ci:` subject, as the release skill says. It's a
change to `main` on GitHub, so it needs the owner's go at that moment. Then `git pull --ff-only`.

## Task 1: take #21 and #22, with a render diff and the table timing

1. **Before renders.** The throwaway `#[ignore]` dump test from PR #18's plan
   (`archive/plans/cargo-group-bump-pr18.md`, Task 1), with its snippet extended by:
   - `> [!NOTE]\n> text` and `> [!note]- Title\n> text` (callouts, both syntaxes).
   - A table with `:-:` and `-:` alignments (the code 0.58 changed).
   - `[[page]]`, `![[page]]` (0.59's option).

   Plus a timing print for two full tables (every row as wide as the header, so
   `table_cost`'s `padding` is 0 and `work` is width² × rows): 1,000 columns × 1,000 rows
   (10⁹, right at `MAX_TABLE_WORK`, so it renders as a table) and 3,000 × 300 (2.7 × 10⁹, over
   it, so its lines read as text today). Both stay under the 64 MB render cap (about 30 bytes a
   cell). Each timed once after a warm-up. Not the existing test's `bomb`: its rows are one cell
   wide, so it trips `MAX_TABLE_PADDING` first. Run into `<scratchpad>/deps3/before`; note the
   times.
2. **Take #22's files.** `git fetch origin dependabot/cargo/src-tauri/cargo-118229b611`; check
   `git diff --stat 373ff8d FETCH_HEAD` lists exactly the two Cargo files, then
   `git checkout FETCH_HEAD -- src-tauri/Cargo.toml src-tauri/Cargo.lock`. Then
   `cargo update -p rustls --precise 0.23.46 --manifest-path src-tauri/Cargo.toml`, and check
   `git diff` of `Cargo.lock` against #22's diff plus #21's: nothing else moved.
3. **After renders**, into `<scratchpad>/deps3/after`. Expect every file byte-identical. Any
   difference: stop and ask. Note the two table times again; and a third, with
   `MAX_TABLE_WORK` raised 10× in the working copy so the 3,000-wide bomb renders as a table,
   to see what the limit would cost if lifted. Revert that.
4. Delete the throwaway test. Gates; the same test count as before. Commit:
   `chore: bump comrak, notify and rustls from Dependabot PRs #21 and #22`.
5. The table times go to triage as a decision: keep `MAX_TABLE_WORK`, raise it, or drop the
   `work` term. Nothing changes in this task.

## Task 2: the three fixes, one commit each

**2a, drop caret.** `app.js:4690-4691`:

```js
  const tab = tabs.find((t) => t.id === d.id);
  if (!tab) {
    // Closed mid-drag: nothing to drop, but another window may still show
    // the caret the probes lit.
    d.ipc = d.ipc.then(() => invoke("drag_cancel")).catch(() => {});
    return;
  }
```

Commit: `A tab closed mid-drag no longer leaves another window's drop caret lit`.

**2b, F5 retries a timeout.** `const TIMED_OUT = "It took too long.";` beside `DIAGRAMS_KEPT`
(`app.js:687`), used at `app.js:590`. In `refresh()`, before `showActive()`:

```js
  // A diagram that took too long is cached as failed, so a reload alone would
  // show the same reason; F5 is the reader asking for another try.
  const look = diagramLook();
  for (const code of els.content.querySelectorAll(MERMAID_BLOCK)) {
    const key = diagramKey(look, code.textContent);
    if (diagrams.get(key)?.error === TIMED_OUT) diagrams.delete(key);
  }
```

Commit: `F5 tries again a diagram that took too long to draw`.

**2c, a renderer that won't load.** `let rendererFailed = false;` beside `rendererLoad`
(`app.js:551`). `loadRenderer` starts with
`if (rendererFailed) return Promise.reject(new Error("The diagram renderer did not load."));`.
`fail` sets `rendererFailed = true` before `reject`. `redrawDiagrams` sets
`rendererFailed = false` before `warmSources`, with a comment: a new look is the one moment to
try the frame again. Update `loadRenderer`'s doc comment (`app.js:542-550`) with one sentence on
the flag.

Commit: `A document opens without a wait once the diagram renderer has failed to load`.

Gates after each; `node --check` is the one that sees `app.js`.

## Task 3: walks

On `walk/deps3`, debug builds. BEFORE = `main` at `373ff8d` where a step says so.

- **W1, renders (all three):** `examples/kitchen-sink.md` draws its tables (the `:-:` column
  centred), code, footnotes, task list and diagram.
- **W2, live reload (all three; notify):** with kitchen-sink open, append a line to the file
  on disk: the page shows it within about a second. Delete the file and write it back with the
  line removed: the page follows. On Linux this is the deleted-path fix; on macOS the new
  watcher.
- **W3, drop caret (Windows only, BEFORE and AFTER):** window A with two tabs, window B beside
  it. Drag A's active tab out of the strip until it detaches, hold it over B's tab strip until
  B's drop caret shows, press Ctrl+W, release over B. BEFORE: expected B's caret stays. AFTER:
  it clears. If Ctrl+W does nothing mid-drag, say so: the item closes as unreachable and 2a
  stays as a guard. CDP input is fine (trusted events).
- **W4, F5 after a timeout (all three, AFTER):** with kitchen-sink drawn, in the page's console
  put the timeout into the cache for its diagram and repaint:
  `diagrams.set(diagramKey(diagramLook(), document.querySelector(".mermaid-diagram") ? diagramBlocks.get(document.querySelector(".mermaid-diagram")).pre.textContent : document.querySelector("pre > code.language-mermaid").textContent), { error: "It took too long." })`,
  then F5: the block reads "Diagram didn't draw: It took too long." (the stale entry is painted
  on this first reload, since the page had a figure, not a block, when `refresh` looked). F5
  again: the diagram draws. BEFORE, for the record: the second F5 shows the same reason.
  `app.js` is a classic script, so its top-level names are reachable from the console.
- **W5, renderer failure (Windows, BEFORE and AFTER):** with CDP, `Network.setBlockedURLs`
  on `*diagram-frame.js`, then open kitchen-sink: its diagram stays a code block after about a
  second. Open `examples/widths.md` (it has diagrams too). BEFORE: another frame is created
  (count `iframe[sandbox]` insertions, or time to paint) and the wait repeats. AFTER: no new
  frame, the blocks show at once. Then lift the block and pick a different diagram colour in
  Settings: AFTER, a frame is created again (the look change cleared the flag) and the diagrams
  draw.
- **W6, update check (one platform):** Settings › *Check now* reads "You are up to date."
  (rustls is the updater's TLS).

Linux runs on headless Xvfb if the VM's pointer is still down; W3 and W5 are Windows-only
anyway.

## Task 4: close out

Move the three items to `docs/closed-items.md` with "Fixed <date>" notes and the commits
(or "Closed <date>, unreachable" for the caret item if W3 says so). Add what triage leaves:
the `zip` advisory, the table-limit ruling. Archive this plan with a status line. In a `docs:`
commit: `docs: plan and record the Dependabot bumps and the three deferred fixes`.

## Execution

The main session runs Tasks 0, 1, 2 and 4, and sends Task 3 to the peers. Serial. Stop and ask
on:

- A PR touching more than its listed files, or `main`'s Cargo files having changed since
  `373ff8d`.
- Any difference in the render diff.
- Any gate failing, or clippy reporting anything new.
- Any walk failure, or W3 finding Ctrl+W unreachable (a closing ruling, not a fix).
