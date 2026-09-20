# Code Review 2026-09-20 Fixes Implementation Plan

> **Status (2026-09-21):** executed in full — commits d952d02 through 47e8c0d, since squashed by
> feature, so the per-task subjects below are no longer all in the log. Its smoke
> pass's check Z hung, so #9 was not parked as planned here: see `json-fold-weight.md`.
> Archived with the review (`../reviews/code-review-2026-09-20.md`); paths and unticked
> boxes below are as they stood when it was written.

> **For agentic workers:** Execute per the **Execution** section below — it says who runs each task and in what order, and it overrides the one-subagent-per-task default of superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 25 of the 31 findings from the 2026-09-20 full-codebase review, put a stopgap on one more (#9) while tracking its real fix and the other deferral (#28) plus one unanswered question, and write down the 4 it leaves.

**Architecture:** No new modules, no new dependencies. Rust fixes are local to `json.rs` (two algorithmic cliffs), `main.rs` (five command attributes, argv, one parse per open), `watch.rs` (a bounded drain), `update.rs` (a re-entrancy guard), `session.rs` (argv), `themes.rs` (a case-aware shadow check) and `render.rs` (page and title from one parse). Frontend fixes are small guards in `src/app.js` and one attribute in `src/index.html`. Two workflow files get commit pins and a sturdier version check. Pure Rust logic gets a unit test beside the existing ones; behaviour that only exists in a running window is checked once, at the end, by driving the debug build (`drive-app` skill).

**Tech Stack:** Rust + Tauri v2, vanilla JS (a plain script, no bundler, no frontend test harness).

**Spec:** `docs/reviews/code-review-2026-09-20.md`. Finding numbers (`#n`) below are that file's. Line numbers are as of `a033113` and drift as tasks land — **find code by function name, not by line.**

**Owner decisions.** Rows marked *decided* or *confirmed 2026-09-20* were put to the owner one by one; the rest are the review's *Proposed* word, which the owner has seen and not changed:

| Decision | Findings |
|---|---|
| fix | #1–#8, #10–#22 |
| fix now rather than defer — decided 2026-09-20 | #27 (Task 5a), #29 (Task 5b) |
| #8 fixes drive-letter paths only; a leading `/` keeps resolving against the document's folder — decided 2026-09-20 | #8 |
| watch flush cap 1 s; reflow limit 8× — decided 2026-09-20 | #7, #2 |
| fix on both sides: Rust guard (Task 5) plus a frontend `installing` flag (Task 9 Step 7) — decided 2026-09-20 | #5 |
| fix the closed-mid-listing case only; the superseded case is an accepted limit — decided 2026-09-20 | #18 |
| stopgap now (Task 1a: a 64 MB ceiling on one render's HTML), real fix still deferred and tracked — decided 2026-09-20 | #9 |
| park — deferred, tracked as unticked items in `docs/open-items.md` with a reopen trigger each (Task 11 Step 2a) | #9's real fix, #28, and the unchecked crash-loop question under #2 |
| fix rather than wontfix — decided 2026-09-20 | #30, #31 (Task 9a) |
| wontfix — confirmed 2026-09-20 | #23, #24, #25, #26 |

## Global Constraints

- Surgical changes only: every changed line traces to a finding. Match the surrounding comment style (full-sentence "why" comments) and naming.
- No new dependencies. No new files beyond the review and this plan.
- Gates, all run from the repo root, all must pass before each commit:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --check` (CI runs this on the Linux leg only, so a miss slips past a green Windows/macOS — run it every time)
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`
  - `node --check src/app.js`
- All commits go straight onto `main`; no feature branches. Commit locally only. **Never push.** Commit subjects are modest, reader-facing sentences with **no prefix** — `release.yml` builds the release notes from every subject that does not start with `ci:`, `docs:`, `release:` or `chore:`. Commits that are not user-facing take a prefix: `docs:` for the review, this plan and the close-out, `chore:` for Task 5b, `ci:` for Task 9a. Each task gives its exact subject. The owner squashes before release.
- End every commit message with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (or the executing model's own line).
- Bash on Windows: never `cd` in a compound command that writes; run from the repo root and use `--manifest-path`.
- **Ordinary session saves stay on the main thread.** Task 2 moves *read* commands off it; `set_session`, `watch_files` and `watch_folders` must stay plain `#[tauri::command]`.
- The debug build shares `%APPDATA%\t4-markdown-viewer\` (`config.json` and `session.json`) with the installed app. Phase 3 follows the drive-app skill's backup / `"reopen": "off"` / diff-before-restore rules.

## Execution

| Phase | Work | Who | Why |
|---|---|---|---|
| 0 | Commit the review and this plan (`docs:`). | main session | Git writes. |
| 0a | Baseline: the four gates on the untouched tree, raw output. If any fails, stop — that is a finding of its own, and no task should start on a red tree (a newer local clippy is the likely culprit). | `tester` | So a failure in Phase 1 is known to be the change's, and the build the `coder` needs is warm. |
| 1 | Tasks 1 → 1a → 2 → 3 → 4 → 5 → 5a → 5b, one commit each | one `coder` | All Rust. One agent builds once and reads `main.rs` once. 1a, 2 and 5b all edit `load_file`, in that order. |
| — | Review: read the eight diffs and the gate output | main session | |
| 2 | Tasks 6 → 7 → 8 → 9, one commit per task (Task 9: one per step), `node --check src/app.js` after each edit | a second, fresh `coder` | Tasks 7, 8 and 9 each run past the 20-line limit for main-session edits (Task 8 touches seven places). Fresh rather than continued: nothing the Rust agent read is of use here. |
| — | Review: read the diffs | main session | |
| 2a | Task 9a (CI): the edits | `scribe` | Mechanical YAML edits with every value written out. The main session runs Step 1's `gh` check first and Step 5's commit after, since `scribe` has no shell. |
| 3 | Task 10: one smoke pass in one launch session, then the four gates on the final tree | `tester` + drive-app skill | One build and one app session. Raw results only; no code changes. |
| — | Triage the smoke results; a failure goes back to the `coder` that made the change (continued with SendMessage), or is fixed directly if it is one file and ≤ 20 lines | main session | |
| 4 | Task 11: close-out docs | main session | |

Rules for the phases:
- **Serial, not parallel.** Tasks 1a, 2, 3 and 5b share `main.rs`, and Tasks 1 and 1a share `json.rs`; Tasks 6–9 share `app.js`; Task 9 Step 4 calls the `refresh` Task 6 changes, and Task 9 Step 7 listens for the event Task 5 emits.
- **`coder` gets its task text, not this whole file:** point it at the task's section by heading, plus Global Constraints. It runs the four gates for its own change and reports the output; nobody re-runs what it already ran.
- **Dispatching `tester`.** It has no Skill tool: tell it to `Read` `.claude/skills/drive-app/SKILL.md` first and follow it, including the `TAURI_CONFIG` identifier override and the rebuild-after-every-`src/`-edit rule. It reports raw results — returned values, screenshot paths — and draws no conclusions. It runs the four gates *after* the smoke pass, because a plain `cargo` build resets the identifier override.
- **No reviewer subagents.** Review is the main session reading the diff and the gate output.

## File Map

| File | Tasks | What changes |
|---|---|---|
| `src-tauri/src/json.rs` | 1, 1a | `more_after` replaces the `container_ends` + `has_content` probe in `emit`; `source` gives up past `REFLOW_GROWTH`; `MAX_HTML_BYTES` ceiling (`render_to` returns `Result`, `emit` splits into `emit` + `emit_within`); five tests |
| `src-tauri/src/main.rs` | 1a, 2, 3, 5b | `?` on `json::render_to`; `(async)` on five commands; `MAX_DOCUMENT_BYTES` comment; `args_os` in `setup`; `load_file` takes page and title from one call |
| `src-tauri/src/themes.rs` | 5a | `same_name` in `scan`'s shadow check; one filesystem test |
| `src-tauri/src/render.rs` | 5b | `render_titled` over one parse; `render` / `first_heading` become test-only wrappers |
| `src-tauri/src/session.rs` | 3 | `args_os` in `snapshot` |
| `src-tauri/src/watch.rs` | 4 | `drain` with a deadline; `MAX_BURST`; two tests |
| `src-tauri/src/update.rs` | 5 | `INSTALLING` guard; body moves to `install`; broadcasts `update-failed` (heard by Task 9 Step 7) |
| `src/app.js` | 6, 7, 8, 9 | guards and small fixes, listed per task |
| `src/index.html` | 9 | `tabindex="0"` on `#image-view` |
| `.github/workflows/release.yml`, `checks.yml` | 9a | six `actions/*` pins, two comments, the `lock=` line |
| `docs/reviews/code-review-2026-09-20.md`, `docs/open-items.md` | 11 | statuses; accepted limits |

---

### Task 1: JSON rendering cliffs (#1, #2)

**Files:**
- Modify: `src-tauri/src/json.rs` — constants near `LOOKAHEAD`, `source`, the `b','` arm of `emit`, a new helper beside `has_content`, tests in `mod tests`

- [ ] **Step 1: Write the failing tests** (append inside `mod tests`)

```rust
    /// `more_after` answers what `has_content(src, comma, innermost_end)` did,
    /// without finding the end.
    #[test]
    fn a_trailing_comma_has_nothing_after_it() {
        let array = [(b']', true)];
        assert!(!more_after("[1,\n]", 2, &array));
        assert!(more_after("[1, 2]", 2, &array));
        assert!(more_after("[1, // c\n]", 2, &array));
        // A closer that does not match closes nothing, so it is content.
        assert!(more_after("[1,}]", 2, &array));
        // Outside every container only the end of the text ends anything.
        assert!(!more_after("1,\n", 1, &[]));
        assert!(more_after("1, 2", 1, &[]));
    }

    /// A trailing comma before every closer, nested deep: no comma past the
    /// budget is a place to cut, and asking where every open container ends at
    /// each of them made the render quadratic in the depth — minutes for a file
    /// under a megabyte. Deep enough that the commas outrun `LOOKAHEAD`, which
    /// is where the repeated scans began.
    #[test]
    fn trailing_commas_nested_deep_render_in_linear_time() {
        let depth = 40_000;
        let src = format!("{}1{}", "[\n".repeat(depth), ",\n]".repeat(depth));
        let started = std::time::Instant::now();
        let html = emit(&src, 0, 0);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(3),
            "took {:?}",
            started.elapsed()
        );
        assert!(!html.contains("class=\"more\""));
    }

    /// Indentation grows with depth, so a file that is all nesting reflows to
    /// the square of its size. Past the limit it is shown as it was written.
    #[test]
    fn reflow_gives_up_on_a_file_that_is_all_nesting() {
        assert!(matches!(source(&"[".repeat(8192)), Cow::Borrowed(_)));
        // Ordinary nesting is nowhere near the limit.
        assert!(matches!(source("[[[[1]]]]"), Cow::Owned(_)));
    }
```

- [ ] **Step 2: Run them and see them not compile**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked json::`
Expected: does not compile — `cannot find function more_after`.

- [ ] **Step 3: Add `more_after`** directly above `has_content` — and nothing else yet, so the other two tests can be seen failing for the right reason.

```rust
/// Whether the comma at `comma` has a member after it, rather than the bracket
/// that closes the innermost open container — a JSONC trailing comma — or the
/// end of the text. The answer `has_content` gives for the stretch up to that
/// bracket, without looking for the bracket: `container_ends` scans to the end
/// of *every* open container, and a file with a trailing comma at each of many
/// levels asked it to at every one of them.
fn more_after(src: &str, comma: usize, stack: &[(u8, bool)]) -> bool {
    src.as_bytes()[comma + 1..]
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| !stack.last().is_some_and(|(closer, _)| b == closer))
}
```

If clippy (`-D warnings`) asks for `is_none_or` in place of `!….is_some_and(…)`, take its suggestion verbatim; the two are the same test.

- [ ] **Step 3a: See the real red.** `emit` and `source` are still unfixed.

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked json::`
Expected: `a_trailing_comma_has_nothing_after_it` PASS; `trailing_commas_nested_deep_render_in_linear_time` FAIL on the elapsed-time assert; `reflow_gives_up_on_a_file_that_is_all_nesting` FAIL.
**Report the elapsed time the timing test prints.** The 3-second bound was set by arithmetic, not by a run: the unfixed time was estimated at 10 s or more in a debug build. If the unfixed code comes in under 3 s the test proves nothing — stop and report rather than going on; the fix is to raise `depth`, not to tighten the bound.

- [ ] **Step 4: Use it in `emit`.** In the `b',' if token.start >= budget && token.start > 0` arm, replace

```rust
                    if token.start >= at {
                        let ends = container_ends(src, token.start, &stack);
                        let inner = ends.first().copied().flatten().unwrap_or(src.len());
                        // A remainder holding nothing but this comma and
                        // whitespace — a JSONC trailing comma, or the exact end
                        // of a container — is not worth a button, so carry on
                        // and trip at the next comma further out instead.
                        if has_content(src, token.start, inner) {
                            finish(&mut out, src, base, token.start, &ends, &stack);
                            return out;
                        }
                    }
```

with

```rust
                    // A remainder holding nothing but this comma and whitespace
                    // — a JSONC trailing comma, or the exact end of a container
                    // — is not worth a button, so carry on and trip at the next
                    // comma further out instead.
                    if token.start >= at && more_after(src, token.start, &stack) {
                        let ends = container_ends(src, token.start, &stack);
                        finish(&mut out, src, base, token.start, &ends, &stack);
                        return out;
                    }
```

`has_content` stays: `more` still uses it.

- [ ] **Step 5: Bound the reflow.** Below `const LOOKAHEAD`, add

```rust
/// How many times its own size the one-line reflow may grow a file to before it
/// is given up on. Indentation is two spaces a level at every bracket and comma,
/// so a file that is mostly nesting reflows to the square of its size — 8 KB of
/// `[` is 64 MB — and nothing else bounds that. Ordinary minified JSON roughly
/// doubles. The floor keeps a small file, which may fairly grow tenfold, out of it.
const REFLOW_GROWTH: usize = 8;
const REFLOW_FLOOR: usize = 1024 * 1024;
```

In `source`, replace

```rust
    let mut depth = 0usize;
    for token in Tokens::new(text, 0) {
        let piece = &text[token.start..token.end];
```

with

```rust
    let mut depth = 0usize;
    let limit = text.len().max(REFLOW_FLOOR) * REFLOW_GROWTH;
    for token in Tokens::new(text, 0) {
        // Shown as written instead: one long line, but one that opens.
        if out.len() > limit {
            return Cow::Borrowed(text);
        }
        let piece = &text[token.start..token.end];
```

- [ ] **Step 6: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked json::`
Expected: all `json::tests` PASS, the three new ones included, the timing test in well under a second.

- [ ] **Step 7: Gates, then commit**

```bash
git add src-tauri/src/json.rs
git commit -m "Open deeply nested JSON without stalling"
```

---

### Task 1a: A ceiling on the HTML one JSON render hands over (#9, stopgap)

Owner's decision, 2026-09-20. **A stopgap, not the fix:** #9 stays tracked in `open-items.md` (Task 11 Step 2a), because the real fix — a second way to end a chunk — is still a design job.

What this does and does not do, so nobody reads more into it:
- The budget counts *source* bytes, and HTML runs from about 3× the source (long strings) through 40× (short records like `{"a":1},`) to about 70× (`[[1],[1],…]`): a span per token, a fold control per bracket. These ratios are estimates from the markup `emit` writes, not measurements. Today nothing bounds what reaches the webview: an 8 MB `extent` of short records is several hundred MB of HTML.
- The ceiling is **64 MB**: above the most a *first chunk that found a comma to end on* can come to, so **no ordinary file is ever refused**. Counted from `emit`'s markup: a `[`…`]` pair is 178 bytes of HTML for 2 of source, the worst any token does, and such a chunk is at most `CHUNK_BYTES + LOOKAHEAD` = 576 KB of source — under 52 MB if it were brackets and nothing else, about 36 MB for `[[1],[1],…]`. A first chunk only passes 64 MB when no cut was possible — the pathological shapes.
- A *re-render* of a far-expanded document can pass it legitimately. That falls back to the first chunk alone: the reader loses what they had expanded, not the document. Only if the first chunk is itself too much is the document refused, and the message lands in the error panel `load_file` errors already use.
- Known and left: after a fallback the tab's `entry.extent` in `app.js` still holds the big number, so every refresh of that tab builds up to 64 MB, throws it away, and renders the first chunk — until the reader clicks a `more`, which recomputes `extent` from the buttons on the page. Bounded and rare; not worth a second return value.
- It does **not** stop the review's own examples (750 KB → 34 MB, 900 KB → 38 MB), which sit inside what a legitimate first chunk can reach. Whether the webview copes with 34 MB nested 150,000 deep is unmeasured; Task 10 check Z measures it, and what it shows decides whether #9's real fix is urgent.

**Files:**
- Modify: `src-tauri/src/json.rs` — one constant, `render_to`, `render_slice`, `emit` (split into `emit` + `emit_within`), three existing test call sites, two new tests
- Modify: `src-tauri/src/main.rs` — `load_file` (one `?`)

- [ ] **Step 1: Write the failing tests** (append inside `json.rs`'s `mod tests`). They pass a small ceiling of their own, so they cost nothing to run.

```rust
    /// Brackets nested deep with a trailing comma at every level: no comma is a
    /// place to cut, so the whole of it would be handed over.
    #[test]
    fn a_render_past_the_ceiling_is_refused() {
        let depth = 10_000;
        let src = format!("{}1{}", "[\n".repeat(depth), ",\n]".repeat(depth));
        assert!(render_within(&src, 0, 1024 * 1024).is_err());
        // The same document under a ceiling it fits is rendered as ever.
        assert!(render_within(&src, 0, usize::MAX).is_ok());
    }

    /// A reader who had expanded a big document gets the first chunk back, not
    /// an error, when the whole of what they had is too much.
    #[test]
    fn a_re_render_past_the_ceiling_falls_back_to_the_first_chunk() {
        // Long strings keep the markup small beside the text, so the sizes are
        // predictable: about 2.1 MB of source, whose first 512 KB renders to
        // well under the 3 MB ceiling and whose whole renders to well over it.
        let record = format!("{{\"a\":\"{}\"}},\n", "x".repeat(200));
        let src = format!("[{}{{\"a\":1}}]", record.repeat(10_000));
        let ceiling = 3 * 1024 * 1024;
        assert!(emit_within(&src, 0, usize::MAX, usize::MAX).len() > ceiling);
        let html = render_within(&src, usize::MAX, ceiling).unwrap();
        assert!(html.contains(r#"class="more""#));
        assert!(html.len() <= ceiling + 64);
    }
```

- [ ] **Step 2: Run them and see them not compile**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked json::`
Expected: does not compile — `cannot find function render_within`.

- [ ] **Step 3: The constant.** Below `const MAX_EXTENT`, add

```rust
/// The most HTML one render hands the webview. The budgets above count source
/// bytes, and HTML runs from a few times the source to seventy — a span per
/// token, a fold control per bracket — so without this nothing bounds what the
/// page is asked to swallow. Set above the most a first chunk that found a comma
/// to end on can come to — `CHUNK_BYTES + LOOKAHEAD` of nothing but brackets, at
/// 178 bytes of markup a pair, is under 52 MB — so that no ordinary file is
/// refused: only a re-render of a far-expanded one falls back, and only a
/// document with nowhere to cut is turned away.
// ponytail: a refusal, not a cut. Ending the chunk on output as well as on
// source is the real fix (review 2026-09-20, #9); it needs a re-render to stop
// at the same place, which a budget in source bytes cannot say.
const MAX_HTML_BYTES: usize = 64 * 1024 * 1024;

const TOO_MUCH: &str = "This is too much to show at once: rendered, it comes to more than 64 MB. \
    That usually means brackets nested thousands deep.";
```

- [ ] **Step 4: `emit` stops building what will be refused.** Rename the existing `fn emit(src: &str, base: usize, budget: usize) -> String` to

```rust
fn emit_within(src: &str, base: usize, budget: usize, ceiling: usize) -> String {
```

and directly above it add (the doc comment that was on `emit` stays with `emit_within`)

```rust
/// `emit_within` under the ceiling every real render has.
fn emit(src: &str, base: usize, budget: usize) -> String {
    emit_within(src, base, budget, MAX_HTML_BYTES)
}
```

Inside `emit_within`, make the first statement of the `for token in Tokens::new(src, 0)` loop

```rust
        // Past the ceiling the caller refuses this whatever else it holds, so
        // there is no point building — or allocating — the rest of it.
        if out.len() > ceiling {
            return out;
        }
```

Every existing `emit(…)` call, tests included, is untouched.

- [ ] **Step 5: `render_to` and `render_slice`.** Replace the whole of `render_to` (its doc comment stays, with one paragraph added at its end: `` Past `MAX_HTML_BYTES` a re-render falls back to the first chunk alone, and a first chunk that is still too much is refused. ``)

```rust
pub fn render_to(src: &str, extent: usize) -> String {
    let mut html = String::from("<pre><code>");
    html.push_str(&emit(src, 0, extent.clamp(CHUNK_BYTES, MAX_EXTENT)));
    html.push_str("</code></pre>");
    html
}
```

with

```rust
pub fn render_to(src: &str, extent: usize) -> Result<String, String> {
    render_within(src, extent, MAX_HTML_BYTES)
}

fn render_within(src: &str, extent: usize, ceiling: usize) -> Result<String, String> {
    let wanted = extent.clamp(CHUNK_BYTES, MAX_EXTENT);
    // What the reader had expanded, if that fits; failing that the first chunk
    // alone, which costs them their place in the document rather than the
    // document.
    let mut budgets = vec![wanted];
    if wanted > CHUNK_BYTES {
        budgets.push(CHUNK_BYTES);
    }
    for budget in budgets {
        let body = emit_within(src, 0, budget, ceiling);
        if body.len() <= ceiling {
            return Ok(format!("<pre><code>{body}</code></pre>"));
        }
    }
    Err(TOO_MUCH.to_string())
}
```

In `render_slice`, replace

```rust
    Ok(emit(slice, start, CHUNK_BYTES))
```

with

```rust
    let html = emit(slice, start, CHUNK_BYTES);
    if html.len() > MAX_HTML_BYTES {
        return Err(TOO_MUCH.to_string());
    }
    Ok(html)
```

(The frontend already shows a `json_region` error in place of the button.)

- [ ] **Step 6: Callers.** In `main.rs`, `load_file`, replace `json::render_to(&json::source(&text), extent.unwrap_or(0))` with `json::render_to(&json::source(&text), extent.unwrap_or(0))?`.

In `json.rs`'s tests: the `render` helper becomes `render_to(src, 0).unwrap()`; the two other direct calls — `let html = render_to(src, next);` and `render_to("[1]", usize::MAX).contains(…)` — each gain `.unwrap()`.

Run: `grep -n "render_to(" src-tauri/src/json.rs src-tauri/src/main.rs` and check every call either `?`s or `.unwrap()`s.

- [ ] **Step 7: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked json::`
Expected: all PASS, Task 1's three and these two included.

- [ ] **Step 8: Gates, then commit**

```bash
git add src-tauri/src/json.rs src-tauri/src/main.rs
git commit -m "Refuse a JSON render too big for the window to take"
```

---

### Task 2: Read commands off the main thread (#3)

**Files:**
- Modify: `src-tauri/src/main.rs` — the attributes on `load_file`, `section_source`, `json_region`, `load_asset`, `list_dir`; the doc comment on `MAX_DOCUMENT_BYTES`

No unit test: the attribute changes which thread Tauri calls the function on, which only a running app shows (Task 10, check A).

- [ ] **Step 1: Change five attributes.** On exactly these five functions, replace `#[tauri::command]` with `#[tauri::command(async)]`:

`load_file`, `section_source`, `json_region`, `load_asset`, `list_dir`.

Leave every other command as it is — in particular `toggle_task`, `watch_files`, `watch_folders`, `set_session`, `set_reopen`.

- [ ] **Step 2: Say why, once.** Above `load_file`'s attribute, extend its doc comment with a closing paragraph:

```rust
///
/// `(async)` here and on the other read-only commands — `load_asset`,
/// `section_source`, `json_region`, `list_dir` — only to get off the main
/// thread, as `picker_dir` does: a synchronous command runs on the event loop,
/// and a big file, a folder of ten thousand entries or a share that has gone
/// away would hold every window still for as long as it took. Not the watch
/// commands: being taken in turn on the main thread is what stops two quick
/// calls installing the older watcher last.
// ponytail: a slow call still occupies one of the async runtime's workers for
// as long as it takes; move the body into `spawn_blocking` if enough of them
// at once — a tree of folders on a dead share — ever starve the other async
// commands.
```

- [ ] **Step 3: Correct the `MAX_DOCUMENT_BYTES` comment.** Replace

```rust
/// rendering it all happen before the command returns — the window is frozen
/// for as long as that takes. Images are deliberately not capped: `load_asset`
```

with

```rust
/// rendering it all happen before the command returns — on a worker thread, so
/// nothing freezes, but the tab stands empty for as long as that takes and the
/// webview then has to swallow the result. Images are deliberately not capped: `load_asset`
```

then re-wrap the paragraph to the file's line width (`cargo fmt` does not re-wrap comments).

- [ ] **Step 4: Gates, then commit**

```bash
git add src-tauri/src/main.rs
git commit -m "Read files and folders off the main thread"
```

---

### Task 3: Start up on a non-UTF-8 argument (#6)

**Files:**
- Modify: `src-tauri/src/main.rs` — first line of the `.setup` closure
- Modify: `src-tauri/src/session.rs` — `snapshot`

No unit test: argv cannot be injected into a test process. Checked by reading.

- [ ] **Step 1: `main.rs`.** Replace

```rust
            let args: Vec<String> = std::env::args().collect();
```

with

```rust
            // `args()` panics on an argument that is not Unicode, and a panic
            // here is an abort with no window: a file manager handing over a
            // Latin-1 file name would simply fail to start the app. Lossy
            // instead — the mangled name is not a file, so it is ignored.
            let args: Vec<String> = std::env::args_os()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
```

- [ ] **Step 2: `session.rs`.** In `snapshot`, replace

```rust
    save(app, version, std::env::args().skip(1).collect(), true);
```

with

```rust
    // Lossy for the reason `setup` gives: `args()` panics on a name that is
    // not Unicode.
    let args = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    save(app, version, args, true);
```

- [ ] **Step 3: Gates, then commit**

```bash
git add src-tauri/src/main.rs src-tauri/src/session.rs
git commit -m "Start up when a file name is not valid Unicode"
```

---

### Task 4: Flush a burst that never pauses (#7)

**Files:**
- Modify: `src-tauri/src/watch.rs` — imports, a new constant, a new `drain`, the worker loop, a new `mod tests`

- [ ] **Step 1: Write the failing tests** (append to the end of the file)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// A log appended to faster than the quiet period never goes quiet, and
    /// the drain used to wait for quiet alone.
    #[test]
    fn a_burst_that_never_pauses_is_still_flushed() {
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for i in 0..200 {
                if tx.send(i).is_err() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        let started = Instant::now();
        let burst = drain(&rx, Duration::from_millis(50), Duration::from_millis(200));
        assert!(!burst.is_empty());
        assert!(
            started.elapsed() < Duration::from_millis(1000),
            "took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn quiet_ends_a_burst() {
        let (tx, rx) = channel();
        tx.send(1).unwrap();
        let burst = drain(&rx, Duration::from_millis(20), Duration::from_secs(5));
        assert_eq!(burst, vec![1]);
        drop(tx);
    }
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked watch::`
Expected: does not compile — `cannot find function drain`, `cannot find type Instant`.

- [ ] **Step 3: Implement.** Replace the two `use std::…` lines

```rust
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;
```

with

```rust
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};
```

Below `const DEBOUNCE`, add

```rust
/// However busy the folder, what has changed is reported at least this often.
/// Waiting for quiet alone means a log appended to faster than `DEBOUNCE`, or a
/// long copy into a folder on show, is never reported at all.
const MAX_BURST: Duration = Duration::from_secs(1);
```

Above `pub fn watch`, add

```rust
/// Everything that arrives until the channel has been quiet for `quiet`, or
/// until `longest` has passed, whichever comes first.
fn drain<T>(rx: &Receiver<T>, quiet: Duration, longest: Duration) -> Vec<T> {
    let deadline = Instant::now() + longest;
    let mut burst = Vec::new();
    while Instant::now() < deadline {
        match rx.recv_timeout(quiet) {
            Ok(ev) => burst.push(ev),
            // Quiet ends the burst, and so does a closed channel. Every
            // navigation rebuilds the watcher — once before the read, again
            // once the document's pictures are known — and returning on a
            // closed channel threw away hits already collected, so a save
            // landing between the two was neither read nor reported. The
            // burst is emitted instead; the outer `recv` then sees the closed
            // channel and the thread exits as before.
            Err(_) => break,
        }
    }
    burst
}
```

In the worker thread, replace

```rust
        // Drain the burst before reacting.
        loop {
            match rx.recv_timeout(DEBOUNCE) {
                Ok(ev) => collect(&ev, &mut hits),
                Err(RecvTimeoutError::Timeout) => break,
                // Every navigation rebuilds the watcher — once before the read,
                // again once the document's pictures are known — and returning
                // here threw away hits already collected, so a save landing
                // between the two was neither read nor reported. Break instead
                // and the burst is emitted; the outer `recv` then sees the
                // closed channel and the thread exits as before.
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
```

with

```rust
        // Drain the burst before reacting.
        for ev in drain(&rx, DEBOUNCE, MAX_BURST) {
            collect(&ev, &mut hits);
        }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked watch::`
Expected: both PASS.

- [ ] **Step 5: Gates, then commit**

```bash
git add src-tauri/src/watch.rs
git commit -m "Refresh a file that never stops being written"
```

---

### Task 5: One install at a time (#5)

**Files:**
- Modify: `src-tauri/src/update.rs` — the `atomic` import, a new static, `install_update` split in two

No unit test: the command needs a live `AppHandle` and a reachable update. Checked by reading; the second-click message is the existing `Update failed: …` path in `runUpdate`.

- [ ] **Step 1: Import.** Replace

```rust
use std::sync::atomic::{AtomicU64, Ordering};
```

with

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
```

- [ ] **Step 2: Guard the command.** Replace the two lines

```rust
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
```

(keeping the doc comment above them where it is) with

```rust
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    // One install, whoever asks. The button that started it is disabled, but
    // only in its own window and only until that dialog is reopened, and two
    // downloads ending in two installers over one directory is not a state to
    // find out about. Not `AppState::installing`: that stands ordinary session
    // saves down, which is right for the moment of the install and wrong for
    // the whole of a download.
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("An update is already being installed.".to_string());
    }
    let result = install(app.clone()).await;
    // Only ever reached by a failure: success does not return.
    INSTALLING.store(false, Ordering::SeqCst);
    // Every window has been counting along with the download and holding its
    // own Update button back; they need telling it is theirs to press again.
    let _ = app.emit("update-failed", ());
    result
}

static INSTALLING: AtomicBool = AtomicBool::new(false);

async fn install(app: AppHandle) -> Result<(), String> {
```

The body that follows is unchanged and now belongs to `install`.

- [ ] **Step 3: Gates, then commit**

```bash
git add src-tauri/src/update.rs
git commit -m "Refuse a second update while one is installing"
```

---

### Task 5a: A user theme shadows a bundled one the way the filesystem sees it (#27)

Owner's decision, 2026-09-20: fix now rather than defer.

**Files:**
- Modify: `src-tauri/src/themes.rs` — a helper above `scan`, the shadow check inside `scan`, one test and one test helper in `mod tests`

- [ ] **Step 1: Write the failing test** (append inside `mod tests`)

```rust
    /// A scratch directory of our own under the system's, emptied first.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("t4mv-themes-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `path_for` finds a theme through the filesystem, which on Windows and
    /// macOS does not tell `Solarized-Light` from `solarized-light`: the user's
    /// file answers to both names. So it has to shadow the bundled one in the
    /// list too, or the picker shows two rows that load the same file.
    #[test]
    fn a_user_theme_shadows_a_bundled_one_the_way_the_filesystem_sees_it() {
        let bundled = scratch("bundled");
        let user = scratch("user");
        std::fs::write(bundled.join("solarized-light.css"), "").unwrap();
        std::fs::write(user.join("Solarized-Light.css"), "").unwrap();

        let mut out = Vec::new();
        scan(&bundled, true, &mut out);
        scan(&user, false, &mut out);

        if cfg!(any(windows, target_os = "macos")) {
            assert_eq!(out.len(), 1);
            assert!(!out[0].builtin);
            assert_eq!(out[0].name, "Solarized-Light");
        } else {
            assert_eq!(out.len(), 2);
        }
        let _ = std::fs::remove_dir_all(&bundled);
        let _ = std::fs::remove_dir_all(&user);
    }
```

- [ ] **Step 2: Run it and see it fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked themes::`
Expected on Windows: FAIL — `left: 2, right: 1`. (On Linux it passes before and after; the Windows and macOS CI legs are what hold the fix.)

- [ ] **Step 3: Implement.** Directly above `fn scan`, add

```rust
/// Whether two theme names are one file to the filesystem `path_for` resolves
/// them through. ASCII is as far as this goes: theme files are named in it, and
/// matching a filesystem's full case folding is not worth a dependency.
fn same_name(a: &str, b: &str) -> bool {
    if cfg!(any(windows, target_os = "macos")) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}
```

In `scan`, replace

```rust
        // A user theme with the same stem shadows the bundled one.
        match out.iter().position(|t| t.name == info.name) {
```

with

```rust
        // A user theme with the same stem shadows the bundled one — the same
        // as the filesystem sees it, since that is who `path_for` asks.
        match out.iter().position(|t| same_name(&t.name, &info.name)) {
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked themes::`
Expected: all PASS.

- [ ] **Step 5: Gates, then commit**

```bash
git add src-tauri/src/themes.rs
git commit -m "List a theme once when only its capitals differ from a bundled one"
```

---

### Task 5b: Parse a markdown document once per open (#29)

Owner's decision, 2026-09-20: fix now rather than defer.

**Files:**
- Modify: `src-tauri/src/render.rs` — the `comrak::nodes` import, `render` and `first_heading` re-cut around one parse, one new test
- Modify: `src-tauri/src/main.rs` — `load_file`

`render` has 30-odd callers in its own tests and `first_heading` a dozen; they keep both names, as test-only wrappers, so no test changes.

- [ ] **Step 1: Write the failing test** (append inside `render.rs`'s `mod tests`)

```rust
    #[test]
    fn render_titled_gives_the_page_and_its_title_from_one_parse() {
        let (html, title) = render_titled("intro\n\n## Deeper\n");
        assert!(html.contains("<h2"));
        assert_eq!(title, Some("Deeper".into()));
        assert_eq!(render_titled("just text\n").1, None);
    }
```

- [ ] **Step 2: Run it and see it fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked render::`
Expected: does not compile — `cannot find function render_titled`.

- [ ] **Step 3: Re-cut `render.rs`.** Replace the import

```rust
use comrak::nodes::NodeValue;
```

with

```rust
use comrak::nodes::{AstNode, NodeValue};
```

Replace the signature and first three lines of `render`

```rust
pub fn render(md: &str) -> String {
    let o = options();
    let arena = Arena::new();
    let root = parse_document(&arena, md, &o);

    // Raw-HTML nodes in document order, which is the order comrak drops them.
```

with (the doc comment above `render` stays where it is, now on `render_titled`, with one sentence added at its end: `` The title comes back with it — see `title_of` — so the document is parsed once for both. ``)

```rust
pub fn render_titled(md: &str) -> (String, Option<String>) {
    let o = options();
    let arena = Arena::new();
    let root = parse_document(&arena, md, &o);
    (html_of(root, &o), title_of(root))
}

/// The HTML alone, which is all most of the tests below are about.
#[cfg(test)]
fn render(md: &str) -> String {
    render_titled(md).0
}

/// The title alone, likewise.
#[cfg(test)]
fn first_heading(md: &str) -> Option<String> {
    render_titled(md).1
}

fn html_of<'a>(root: &'a AstNode<'a>, o: &Options) -> String {
    // Raw-HTML nodes in document order, which is the order comrak drops them.
```

In what is now `html_of`'s body, replace `format_html(root, &o, &mut html)` with `format_html(root, o, &mut html)`. Nothing else in that body changes.

Replace the signature and first two lines of `first_heading`

```rust
pub fn first_heading(md: &str) -> Option<String> {
    let arena = Arena::new();
    let root = parse_document(&arena, md, &options());
    root.descendants().find_map(|node| {
```

with (its doc comment stays where it is, now on `title_of`)

```rust
fn title_of<'a>(root: &'a AstNode<'a>) -> Option<String> {
    root.descendants().find_map(|node| {
```

If the compiler asks for a lifetime on `Options` in `html_of`'s signature, write `o: &Options<'_>`.

- [ ] **Step 4: `main.rs`, `load_file`.** Replace

```rust
    let html = if as_json {
        json::render_to(&json::source(&text), extent.unwrap_or(0))?
    } else {
        render::render(&text)
    };
```

(the `?` is Task 1a's) with

```rust
    let (html, heading) = if as_json {
        (json::render_to(&json::source(&text), extent.unwrap_or(0))?, None)
    } else {
        // One parse for both: the title is a heading of the page just rendered.
        render::render_titled(&text)
    };
```

and replace

```rust
    let title = if as_json {
        file_name.clone()
    } else {
        render::first_heading(&text).unwrap_or_else(|| file_name.clone())
    };
```

with

```rust
    let title = heading.unwrap_or_else(|| file_name.clone());
```

The comment above it ("A JSON document has no headings to be titled by…") stays: `heading` is `None` for JSON, which is how that still holds.

- [ ] **Step 5: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked render::`
Expected: all PASS — the new one, and every existing `render` / `first_heading` test through the wrappers.

- [ ] **Step 6: Gates, then commit.** Not user-facing, so it takes a prefix and stays out of the release notes.

```bash
git add src-tauri/src/render.rs src-tauri/src/main.rs
git commit -m "chore: parse a markdown document once per open"
```

---

### Task 6: Bank scroll only for the page on screen (#4)

**Files:**
- Modify: `src/app.js` — `rememberScroll`, `refresh`

- [ ] **Step 1: `rememberScroll`.** Replace

```js
function rememberScroll() {
  const entry = currentEntry(activeTab());
  if (!entry) return;
```

with

```js
function rememberScroll() {
  // Not while a switch is loading: the active entry has already moved on and
  // the old document is still what is on screen, so banking now would hand
  // the new entry the old one's offset. Here rather than in each caller — a
  // second Ctrl+Tab before the first has loaded comes through `activateTab`,
  // not the scroll listener.
  if (shownToken !== renderToken) return;
  const entry = currentEntry(activeTab());
  if (!entry) return;
```

- [ ] **Step 2: `refresh`.** Replace

```js
  await showActive(window.scrollY);
}
```

(the last two lines of `refresh`) with

```js
  // Mid-switch the page on screen is the document being left, and its offset
  // is not this entry's to keep: the entry's own saved spot stands.
  await showActive(shownToken === renderToken ? window.scrollY : undefined);
}
```

- [ ] **Step 3: Check and commit**

Run: `node --check src/app.js` — expected: no output.

```bash
git add src/app.js
git commit -m "Keep each tab's place when switching tabs quickly"
```

---

### Task 7: Links and pictures named by full path (#8)

**Files:**
- Modify: `src/app.js` — a constant beside `HAS_SCHEME`, `resolvePath`, `isRelative` (renamed `isLocal`), its two callers

- [ ] **Step 1: Constant and decoder.** Directly below `const HAS_SCHEME = …;` add

```js
/**
 * A link that names a file by drive and path: `C:\notes\spec.md`, `D:/a/b.md`.
 *
 * Not a leading slash. `/docs/guide.md` is how a repository's README names a
 * file from the repository's root, and resolving it against the document's
 * folder — as every other link is — is what makes that work for a document
 * that sits at the root. Reading it as the filesystem's root instead would
 * break those links to fix ones almost nobody writes.
 *
 * Not a network path either — `\\host\share\x.png`, `//host/x.png` — and on
 * purpose: a picture is fetched without a click, and a document able to name a
 * server could have Windows offer the reader's credentials to it.
 */
const ABSOLUTE_PATH = /^[a-z]:[\\/]/i;
```

Directly above `function resolvePath`, add

```js
/** `href` with its percent-escapes undone; malformed ones are used as written. */
function unescapeHref(href) {
  try {
    return decodeURIComponent(href);
  } catch {
    return href;
  }
}
```

- [ ] **Step 2: `resolvePath`.** Replace

```js
  let decoded = rel;
  try {
    decoded = decodeURIComponent(rel);
  } catch {
    /* malformed escapes: use the raw text */
  }
  const joined = `${dir}/${decoded}`.replace(/\\/g, "/");
```

with

```js
  const decoded = unescapeHref(rel);
  // A full path stands on its own; only a relative one hangs off `dir`.
  const absolute = ABSOLUTE_PATH.test(decoded);
  const joined = (absolute ? decoded : `${dir}/${decoded}`).replace(/\\/g, "/");
```

and replace

```js
  const unc = dir.replace(/\\/g, "/").startsWith("//");
```

with

```js
  const unc = !absolute && dir.replace(/\\/g, "/").startsWith("//");
```

Update the function's doc comment to: `/** Resolve `rel` against `dir` — or on its own, if it is a full path — collapsing `.` and `..`. Forward-slash output. */`

- [ ] **Step 3: `isRelative` → `isLocal`.** Replace the whole function with

```js
/** Whether `href` names a file on this machine: relative to the document, or by its full path. */
function isLocal(href) {
  if (!href || href.startsWith("#")) return false;
  // Before the scheme test: `C:` reads as one.
  if (ABSOLUTE_PATH.test(unescapeHref(href))) return true;
  return !HAS_SCHEME.test(href) && !href.startsWith("//");
}
```

and rename both call sites (`if (!isRelative(raw)) return;` in the image rewrite, `if (isRelative(href)) {` in `onLinkClick`) to `isLocal`.

Run: `grep -n "isRelative" src/app.js` — expected: no matches.

- [ ] **Step 4: Check and commit**

Run: `node --check src/app.js` — expected: no output. Behaviour is checked in Task 10 (check C).

```bash
git add src/app.js
git commit -m "Follow links that name a file by its full path"
```

Known limits, stated in the close-out: a leading-slash path (`/home/u/spec.md`) is still resolved against the document's folder, on purpose — see the comment on `ABSOLUTE_PATH`. And a *picture* named by a full path outside the document's folder still does not show — the asset protocol only serves folders a document or picture tab has been opened from. A *link* to it works.

---

### Task 8: Sidebar choices belong to the tab on screen (#10, #17–#22)

**Files:**
- Modify: `src/app.js` — `followTab`, `renderTree`, `openFolder`, `setFolderSort`, `showFolder`, the two `els.treeFilter` handlers, one new helper

At rest `sidebarTab === activeTab()`; they differ only while a switch is loading, when `sidebarTab` is still the tab being left. The tree on show is still `sidebarTab`'s, so `syncFolderWatch` keeps writing `expanded` there — unchanged. What the *reader* chooses in that window is about the tab they are looking at arriving.

- [ ] **Step 1 (#10): `setFolderSort`.** Replace

```js
  if (sidebarTab) {
    sidebarTab.sort = sort;
    reportSoon();
  }
```

with

```js
  // The tab on screen, not `sidebarTab`: while a switch is loading that is
  // still the tab being left, and `followTab` would take the order back.
  const tab = activeTab();
  if (tab) {
    tab.sort = sort;
    reportSoon();
  }
```

- [ ] **Step 2 (#10): `openFolder`.** Replace

```js
  const owner = sidebarTab;
```

with

```js
  const owner = activeTab(); // not `sidebarTab`, which lags a loading switch
```

- [ ] **Step 3 (#10, #22): `showFolder`.** Replace

```js
  const tab = sidebarTab;
```

with

```js
  const tab = activeTab();
```

and replace

```js
    own ? { keep: tab.expanded, filter: tab.filter, picked: tab.picked } : { picked: !named },
```

with

```js
    // No filter to hand back: closing the sidebar blanked every tab's.
    own ? { keep: tab.expanded, picked: tab.picked } : { picked: !named },
```

- [ ] **Step 4 (#10, #21): the filter.** Directly above `function applyTreeFilter`, add

```js
/**
 * Bank the search on the tab on screen — while the sidebar shows that tab's
 * own folder, or the tab has none yet. Not while it is borrowing the
 * document's folder because its own will not list: nothing about a stand-in
 * is written down, the rule `syncFolderWatch` keeps for `expanded`.
 */
function rememberFilter(value) {
  const tab = activeTab();
  if (!tab || state.folder === null) return;
  if (!tab.folder || sameDir(tab.folder, state.folder)) tab.filter = value;
}
```

In the `els.treeFilter` `input` handler replace `if (sidebarTab) sidebarTab.filter = els.treeFilter.value;` with `rememberFilter(els.treeFilter.value);`.

In its `keydown` Escape branch replace `if (sidebarTab) sidebarTab.filter = "";` with `rememberFilter("");`.

- [ ] **Step 5 (#19, #20): `followTab`.** Replace

```js
  if (!tab) return; // empty window: the sidebar stays as it is
  const resort = tab.sort !== state.folderSort;
  state.folderSort = tab.sort;
  showSortMenu(false); // left open, it would go on ticking the last tab's order
  if (state.folder === null) return; // closed: showFolder picks it up later
  const folder = folderFor(tab);
  if (!folder) return; // a load that failed on a bare name: nowhere to go
```

with

```js
  if (!tab) return; // empty window: the sidebar stays as it is
  showSortMenu(false); // left open, it would go on ticking the last tab's order
  const folder = folderFor(tab);
  // A load that failed on a bare name: nowhere to go. Before the order is
  // taken, or the tree would stay in the old one with the menu ticking the new.
  if (!folder) return;
  const resort = tab.sort !== state.folderSort;
  state.folderSort = tab.sort;
  if (state.folder === null) return; // closed: showFolder picks it up later
```

and, in the same function, replace

```js
  if (tab.folder !== state.folder) {
    tab.folder = state.folder;
    reportSoon(); // `updateChrome` has already said its piece for this switch
  }
```

with

```js
  if (tab.folder !== state.folder) {
    tab.folder = state.folder;
    reportSoon(); // `updateChrome` has already said its piece for this switch
  }
  // `openFolder` is what usually says this, and it has not run: a tab opened
  // into an empty window reads it to learn whether this folder was chosen.
  state.folderPicked = own && tab.picked;
```

- [ ] **Step 6 (#17): `renderTree`.** Replace

```js
  for (const path of keep) rootKeep.add(path);
```

with

```js
  for (const path of keep) rootKeep.add(path);
  // After a listing that failed there are no rows left to read what was open
  // off, and a tree that came back shut would be written down as what the tab
  // remembers. The tab still knows: `syncFolderWatch` left it alone.
  if (
    els.sidebar.dataset.tree === "error" &&
    sidebarTab?.folder &&
    sameDir(sidebarTab.folder, dir)
  ) {
    for (const path of sidebarTab.expanded) rootKeep.add(path);
  }
```

- [ ] **Step 7 (#18): `openFolder`.** Replace

```js
  if (state.folder === null) return; // closed while the listing was out
```

with

```js
  if (state.folder === null) {
    // Closed while the listing was out. A folder that then failed to list was
    // never really picked, so the tab goes back to the one it had — unless
    // something newer has written it since. Not the filter: closing blanked it.
    if (owner && remember && owner.folder === path && els.sidebar.dataset.tree === "error") {
      Object.assign(owner, { folder: was, picked: wasPicked });
    }
    return;
  }
```

- [ ] **Step 8: Check and commit**

Run: `node --check src/app.js` — expected: no output.
Run: `grep -n "sidebarTab" src/app.js` and read each remaining use: the declaration, `followTab`, `openTab`, `syncFolderWatch`, `renderTree`. None of them is a *write of a reader's choice*.

```bash
git add src/app.js
git commit -m "Keep a sort, search or folder chosen while a tab is loading"
```

---

### Task 9: Small frontend fixes (#11–#16)

**Files:**
- Modify: `src/app.js` — `onKeydown`, `onDragEnd`, `onJsonClick`, `loadPath`, `showImage`
- Modify: `src/index.html` — `#image-view`

One commit per fix; `node --check src/app.js` before each.

- [ ] **Step 1 (#11): AltGr.** In `onKeydown`, replace

```js
  const ctrl = event.ctrlKey || event.metaKey;
  if (!ctrl) return;
```

with

```js
  const ctrl = event.ctrlKey || event.metaKey;
  // Windows reports AltGr as Ctrl+Alt, and on a German or Nordic keyboard
  // AltGr is how `[` and `]` are typed. Nothing below is a Ctrl+Alt shortcut.
  if (!ctrl || event.altKey) return;
```

```bash
git add src/app.js
git commit -m "Let AltGr type brackets instead of going back"
```

- [ ] **Step 2 (#12): tab order.** In `onDragEnd`, replace

```js
  if (!d.detached) {
    renderTabs(); // reorder is already applied; just drop the drag styling
    return;
  }
```

with

```js
  if (!d.detached) {
    renderTabs(); // reorder is already applied; just drop the drag styling
    reportSoon(); // the order is part of the session, and nothing else says it moved
    return;
  }
```

```bash
git add src/app.js
git commit -m "Remember the order tabs were dragged into"
```

- [ ] **Step 3 (#13): JSON `more`.** In `onJsonClick`, replace

```js
  const path = els.content.dataset.path;
  invoke("json_region", { path, start, end })
    .then((html) => {
```

with

```js
  const path = els.content.dataset.path;
  // Whose document this is, taken now: by the time the chunk is back, another
  // tab may be showing the same file.
  const owner = currentEntry(activeTab());
  invoke("json_region", { path, start, end })
    .then((html) => {
      // Re-rendered while the fetch was out: this button is no longer in the
      // page, and the new render has one of its own.
      if (!btn.isConnected) return;
```

and replace

```js
      const entry = currentEntry(activeTab());
      if (entry && els.content.dataset.path === path)
        entry.extent = loaded;
```

with

```js
      if (owner && owner === currentEntry(activeTab()) && els.content.dataset.path === path)
        owner.extent = loaded;
```

```bash
git add src/app.js
git commit -m "Keep how much of a JSON file is loaded to its own tab"
```

- [ ] **Step 4 (#14): a link to the open document.** In `loadPath`, replace

```js
    tab.index = tab.entries.length - 1;
    syncWatch();
  }
  await showActive(0);
}
```

with

```js
    tab.index = tab.entries.length - 1;
    syncWatch();
    await showActive(0);
    return;
  }
  // The document already on screen, named without a fragment. Read again where
  // it stands rather than from the top: no entry was pushed, so a jump would
  // leave no way back. Still a re-read, so clicking the open file in the tree
  // goes on retrying a load that failed.
  await refresh();
}
```

```bash
git add src/app.js
git commit -m "Stay in place when a link names the open document"
```

- [ ] **Step 5 (#15): zoom before the picture is in.** At the top of `showImage`, replace

```js
  renderTabs();

  els.imageEl.style.width = "";
```

with

```js
  renderTabs();

  // Until this one has decoded, `picture` would still describe the last, and
  // a zoom in that gap would be worked out from the wrong size and banked on
  // this entry.
  picture = null;
  els.imageEl.style.width = "";
```

```bash
git add src/app.js
git commit -m "Ignore a zoom pressed before the picture has loaded"
```

- [ ] **Step 6 (#16): keyboard panning.** In `src/index.html`, replace `<div id="image-view">` with `<div id="image-view" tabindex="0">`. If the tag carries other attributes, add `tabindex="0"` after `id` and leave the rest.

```bash
git add src/index.html
git commit -m "Pan a zoomed picture with the arrow keys"
```

- [ ] **Step 7 (#5, frontend half): one install, in every window's dialog.** Task 5's Rust guard makes a second install impossible; this makes the dialog say so instead of offering one. Owner's decision, 2026-09-20.

Directly above `function showUpdateDialog`, add

```js
/**
 * Whether an update is being downloaded or installed — by any window, since
 * the progress is broadcast. The dialog's own state is not enough: it is reset
 * every time the dialog opens, and another window's never knew.
 */
let installing = false;
```

In `showUpdateDialog`, replace

```js
  els.updateProgress.hidden = true;
  els.updateError.hidden = true;
  els.updateNow.disabled = false;
```

with

```js
  // An install already under way — begun here before the dialog was closed,
  // or in another window — is still under way: show it, and offer no second.
  els.updateProgress.hidden = !installing;
  if (installing && !els.updateProgress.textContent) els.updateProgress.textContent = "Downloading…";
  els.updateError.hidden = true;
  els.updateNow.disabled = installing;
```

In `runUpdate`, replace

```js
  els.updateNow.disabled = true;
  els.updateError.hidden = true;
```

with

```js
  installing = true;
  els.updateNow.disabled = true;
  els.updateError.hidden = true;
```

and in its `catch`, replace

```js
    console.error(err);
    els.updateProgress.hidden = true;
```

with

```js
    console.error(err);
    installing = false;
    els.updateProgress.hidden = true;
```

Where the events are subscribed, replace

```js
  await listen("update-progress", (e) => showUpdateProgress(e.payload));
```

with

```js
  await listen("update-progress", (e) => {
    installing = true; // another window's install is this window's too
    showUpdateProgress(e.payload);
  });
  // The window that asked hears of a failure from its own `invoke`; the rest
  // hear it here, and get their Update button back.
  await listen("update-failed", () => {
    installing = false;
  });
```

```bash
git add src/app.js
git commit -m "Show an update already under way instead of offering another"
```

---

### Task 9a: CI — pin every action, and read the lock version by block (#30, #31)

Owner's decision, 2026-09-20: fix both rather than leave them.

**Files:**
- Modify: `.github/workflows/release.yml` — four `uses:` lines, one comment, the `lock=` line
- Modify: `.github/workflows/checks.yml` — one `uses:` line, one comment

SHAs resolved with `gh api repos/<repo>/commits/<tag>` on 2026-09-20; each major tag sat on the patch release named in the trailing comment. Dependabot's `github-actions` entry already bumps SHA pins and rewrites that comment.

| Action | Was | Pin |
|---|---|---|
| `actions/checkout` | `@v7` | `@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1` |
| `actions/upload-artifact` | `@v7` | `@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1` |
| `actions/download-artifact` | `@v8` | `@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1` |

- [ ] **Step 1: Re-check the SHAs** — tags move, and this plan may be executed days after it was written.

Run, for each row: `gh api repos/actions/checkout/commits/v7 --jq .sha` (and `upload-artifact` `v7`, `download-artifact` `v8`).
Expected: the SHA in the table. If one differs, use what `gh` says now and find its patch version with `gh api "repos/<repo>/tags?per_page=30" --jq '.[] | select(.commit.sha=="<sha>") | .name'`.

- [ ] **Step 2: Pin.** Replace every `actions/checkout@v7` (three in `release.yml`, one in `checks.yml`), the one `actions/upload-artifact@v7` and the one `actions/download-artifact@v8` with the pinned form from the table, trailing comment included — the same shape as the `dtolnay/rust-toolchain` line.

Run: `grep -n "uses: actions/" .github/workflows/*.yml` — expected: six lines, every one with a 40-character SHA.

- [ ] **Step 3: Make the comments true.** In `release.yml` replace

```yaml
      # Third-party actions are pinned to a commit, not a tag: this job ends up
      # holding the signing key, and a tag or branch can be moved to code
      # nobody reviewed. Dependabot bumps the pins.
```

with

```yaml
      # Every action is pinned to a commit, not a tag — GitHub's own included:
      # this job ends up holding the signing key, and a tag or branch can be
      # moved to code nobody reviewed. Dependabot bumps the pins.
```

In `checks.yml` replace

```yaml
      # Third-party actions are pinned to a commit for the reason given in
      # release.yml. Dependabot bumps the pins.
```

with

```yaml
      # Every action is pinned to a commit for the reason given in release.yml.
      # Dependabot bumps the pins.
```

- [ ] **Step 4 (#31): read the lock version by block.** In `release.yml`'s version step, replace

```bash
          lock=$(grep -A1 '^name = "t4-markdown-viewer"$' src-tauri/Cargo.lock \
            | grep -m1 '^version = ' | cut -d'"' -f2)
```

with

```bash
          # The first `version` inside this package's own `[[package]]` block,
          # wherever in the block it sits: `grep -A1` assumed it is the line
          # right after `name`, which is the lockfile's habit, not its promise.
          lock=$(awk '/^\[\[package\]\]/ { hit = 0 }
                      /^name = "t4-markdown-viewer"$/ { hit = 1 }
                      hit && /^version = / { print; exit }' src-tauri/Cargo.lock \
            | cut -d'"' -f2)
```

Run from the repo root: `awk '/^\[\[package\]\]/ { hit = 0 } /^name = "t4-markdown-viewer"$/ { hit = 1 } hit && /^version = / { print; exit }' src-tauri/Cargo.lock | cut -d'"' -f2`
Expected: the version in `src-tauri/Cargo.toml` (`1.6.5` when this was written — checked).

- [ ] **Step 5: Commit.** Nothing here can be run locally: `ci.yml` exercises `checks.yml` on the next push, and the `lock=` line and the artifact pins are first exercised by the next release tag. Say so in the hand-off.

```bash
git add .github/workflows/release.yml .github/workflows/checks.yml
git commit -m "ci: pin GitHub's own actions to commits and read the lock version by block"
```

---

### Task 10: Smoke pass (tester + drive-app)

**Before anything else:** `Get-Process t4-markdown-viewer`. The debug build shares `config.json` and `session.json` with the owner's installed viewer, and this pass sets `"reopen": "off"` in that shared file. If the installed viewer is running, stop and ask the owner before going on — do not close it, and do not proceed around it.

One launch session on the final tree. Follow `.claude/skills/drive-app/SKILL.md` for the build (`TAURI_CONFIG` identifier override), the config/session backup, `"reopen": "off"`, the port, and the restore at the end. Fixtures go in the scratchpad, never the repo. Report raw values only.

- [ ] **Fixtures.** Create in the scratchpad:
  - `deep-trailing.jsonc` — `"[\n".repeat(150000) + "1" + ",\n]".repeat(150000)` (≈ 750 KB)
  - `deep-refused.jsonc` — `"[\n".repeat(400000) + "1" + ",\n]".repeat(400000)` (≈ 2 MB), for check Y
  - `deep-oneline.json` — `"[".repeat(8192)`
  - `links.md` — containing `[abs](<SCRATCH>/second.md)` with the scratchpad's real drive-letter, forward-slash path (`C:/Users/…/second.md`), `[self](links.md)`, and enough filler lines to scroll; plus `second.md`
  - `big.txt` — about 25 MB of lines
  - `tail.txt` — one line, appended to during check J
  - `long-a.md`, `long-b.md`, `long-c.md` — a heading and 400 short paragraphs each, for check D
- [ ] **A (#3).** Without putting `big.txt` on screen (25 MB of HTML would stall the webview itself and say nothing about Rust), `eval` with forward-slash paths:
  `(async () => { const t = performance.now(); const p = invoke("load_file", { path: BIG }); await invoke("list_dir", { path: DIR, sort: "name" }); const listed = performance.now() - t; await p; return [Math.round(listed), Math.round(performance.now() - t)]; })()`
  Report both numbers. Expect the first far below the second: the listing no longer queues behind the read. (On the unfixed build they are about equal.)
- [ ] **B (#1, #2).** **Do not open `deep-trailing.jsonc` as a tab here** — check Z does that, last. Task 1 makes Rust render it in linear time, but what comes back is about 34 MB of HTML nested 150,000 deep — that is #9, under Task 1a's ceiling on purpose — and putting it in the DOM may hang the webview and end the session. Time the command alone:
  `(async () => { const t = performance.now(); const d = await invoke("load_file", { path: P }); return [Math.round(performance.now() - t), d.html.length, d.html.includes('class="more"')]; })()`
  Report all three. Expect seconds, not minutes, and `false` for the last.
  For `deep-oneline.json` the same call, reporting `[ms, d.html.length, d.html.includes("\n")]` — expect a small length and `false` (shown as written, not reflowed). That one is safe to open as a tab afterwards; screenshot it.
- [ ] **C (#8).** `eval` each and report the value:
  - `resolvePath("C:/docs", "C:%5Cnotes%5Cspec.md")` → expect `C:/notes/spec.md`
  - `resolvePath("C:/docs", "C:/notes/../spec.md")` → expect `C:/spec.md`
  - `resolvePath("C:/proj", "/docs/a.md")` → expect `C:/proj/docs/a.md` (a leading slash still hangs off the document's folder — unchanged behaviour)
  - `resolvePath("C:/docs", "../a.md")` → expect `C:/a.md`
  - `resolvePath("C:/docs", "%5C%5Cevil%5Cshare%5Cx.png")` → expect `C:/docs/evil/share/x.png`
  - `[isLocal("C:%5Cnotes%5Cspec.md"), isLocal("/a/b.md"), isLocal("https://x"), isLocal("//host/x"), isLocal("mailto:a@b"), isLocal("#x")]` → expect `[true,true,false,false,false,false]`
  - Open `links.md`, `click` the `abs` link → `second.md` renders in the same tab.
- [ ] **D (#4).** Three tabs: `long-a.md`, `long-b.md`, `long-c.md`, in that order. Activate `tabs[1]`, scroll to 500, then activate `tabs[0]` and scroll to 3000, and wait for it to settle (`shownToken === renderToken`). Then `eval "activateTab(tabs[1].id); activateTab(tabs[2].id)"` in one call — the first is deliberately not awaited — wait for it to settle, and report `tabs.map(t => currentEntry(t).scrollY)`. Expect `[3000, 500, …]`: `tabs[1]` keeps its 500. (On the unfixed build it reads 3000.)
- [ ] **E (#14).** In `links.md` scrolled to 800, `click` the `self` link → `window.scrollY` still ≈ 800, `activeTab().entries.length` unchanged.
- [ ] **F (#12).** Needs a real pointer drag of a tab within the strip (`onDragEnd` is the path under test; splicing `tabs` by script is not). If `cdp.mjs` cannot produce one, report "not driven" — do not extend the script for it. If driven: after the drop, wait a second and report the tab order in `session.json` against `tabs.map(t => t.path)`.
- [ ] **G (#10).** With the sidebar open on a folder. Before the call, confirm and report that `tabs[0]` is not the active tab and `tabs[0].sort === "name"`. Then `eval "activateTab(tabs[0].id); setFolderSort('modified')"` in one call; after it settles report `[activeTab().sort, state.folderSort]` → expect both `modified`. This writes `folder_sort` to the shared `config.json`: `eval "setFolderSort('name')"` afterwards, and the skill's diff-before-restore covers the rest.
- [ ] **H (#16).** `cdp.mjs` cannot send real key presses, and a scripted `KeyboardEvent` does not scroll, so only the precondition is driven: open a picture, `eval "els.imageView.focus(); document.activeElement === els.imageView"` → expect `true`, and a screenshot to confirm no focus ring is drawn around the panel on a scripted focus. Arrow-key panning itself: report "not driven".
- [ ] **I (#11).** `eval` a dispatched `KeyboardEvent("keydown", {key: "[", ctrlKey: true, altKey: true, bubbles: true})` on `document` after following one link (so Back is possible); report `activeTab().index` before and after → expect unchanged. Then the same without `altKey` → expect it to drop by one (the shortcut itself still works).
- [ ] **J (#7).** Open `tail.txt`, then in the background run `node -e "const fs=require('fs');let i=0;const t=setInterval(()=>{fs.appendFileSync(P,'line\n');if(++i>=60)clearInterval(t)},100)"` (P = the file's path). Two seconds in, and again after it ends, report `els.content.textContent.split("line").length`. Expect the first number already above 1 (it refreshed while the writes were still coming) and the second at 61 or so.
- [ ] **K (#5, frontend flag).** No update is on offer, so the dialog is fed one: `eval "state.update = { version: '9.9.9', notes: '', installable: true, release_url: '' }; installing = true; showUpdateDialog(); [els.updateNow.disabled, els.updateProgress.hidden, els.updateProgress.textContent]"` → expect `[true, false, "Downloading…"]`. Then `eval "els.updateDialog.close(); installing = false; showUpdateDialog(); [els.updateNow.disabled, els.updateProgress.hidden]"` → expect `[false, true]`. Then `eval "els.updateDialog.close(); state.update = null"`. **Never click Update now.**
- [ ] **Regression sweep.** Task-list tick, copy-section button, JSON `more` on a large ordinary `.json`, sidebar expand/collapse + filter, tab tear-off to a new window. One screenshot each; read them. For Task 5b: open `examples/kitchen-sink.md` and report `activeTab().heading` → expect `Kitchen Sink`; open any `.json` and report it → expect the file's name (`load_file` titles JSON by file name, and still must).
- [ ] **Y (#9 stopgap).** A file the ceiling refuses: `deep-refused.jsonc` (226 bytes of HTML a level × 400,000 ≈ 90 MB). `(async () => { try { await invoke("load_file", { path: P }); return "rendered"; } catch (e) { return String(e); } })()` → expect the "too much to show at once" message. **If it returns `"rendered"`, report that and do not open the file** — the ceiling is not working and 90 MB would go to the webview. Otherwise open it as a tab and screenshot: expect the error panel with that message, and the app responsive (`eval "document.title"` answers).
- [ ] **Z (#9, measurement — last, it may end the session).** Every other check must be reported before this one starts. Open `deep-trailing.jsonc` (the 750 KB one, about 34 MB of HTML, under the ceiling on purpose) as a tab with `openTab`, and report: how long until `shownToken === renderToken`, whether `eval "document.title"` still answers afterwards, how long a `window.scrollTo(0, 5000)` takes to show in a screenshot, and the process's working set (`Get-Process t4-markdown-viewer | select WS`). If nothing answers for 60 s, report "hung", kill the process (`taskkill //F`), and say so. This is the number that says whether #9's real fix is urgent; it is not a pass/fail.
- [ ] **Gates.** After the app is closed and the backups restored, run all four gates and report the output.

---

### Task 11: Close-out

**Files:**
- Modify: `docs/reviews/code-review-2026-09-20.md`
- Modify: `docs/open-items.md`

- [ ] **Step 1: Review statuses.** Set each row's Status: `done` for everything fixed (with the commit hash), `park` for #9 (naming Task 1a's commit as the stopgap, and what check Z measured) and #28, `wontfix` for #23–#26, and anything Task 10 reported "not driven" says so in the status note at the top, in the style of the 2026-09-12 review.
- [ ] **Step 2a: `open-items.md`, a new `## Deferred from the 2026-09-20 review` section**, placed after `## Bugs`. These are real defects put off, not accepted: each is an unticked `- [ ]` item in the file's usual style (bold title, "added 2026-09-20", the review's `#n`, where it is, what it costs), ending with the trigger that reopens it:
  - **#9 A JSON chunk ends on source bytes only, never on how much HTML it has become** (`json.rs`, `emit` / `finish` / `more`). A 900 KB deeply nested file renders to 38 MB of HTML with 77k `more` buttons. Task 1a put a 64 MB ceiling on one render — a refusal, and a fallback to the first chunk for a re-render — which bounds the damage without fixing it: the 34–38 MB cases still go to the webview whole. The real fix is a second way to end a chunk, and it has to let a re-render stop at the same place, which a budget in source bytes cannot say. Record here what Task 10 check Z measured (time to render, whether the window stayed responsive, working set). *Reopen when:* check Z showed a hang or an unusable tab — then it is not deferred, raise it with the owner at hand-off — or a real-world JSON file, not a constructed one, produces a render the webview struggles with.
  - **#28 Anchor recovery misreads an `id=` inside another attribute's value** (`render.rs`, `attribute`). `<a href="p?a id=q" id="x"></a>` recovers no anchor. The fix is a scanner that skips quoted values. *Reopen when:* a document in the wild loses an anchor to it, or `attribute` is touched for any other reason.
  - **Unchecked, from #2: can a file that kills the app bring it down again on every launch?** (`session.rs` restore, `main.rs` `setup`). The review never traced whether `session.json` still names a document whose load aborted the process — if it does, `"reopen": "restore"` reopens it at startup and the app cannot be started without deleting the file by hand. Task 1 removes the one known way a document aborts the process (the unbounded reflow), so nothing is known to trigger this today; the question is whether the next such file would. A question, not a confirmed defect: the first step is to answer it — open a document, kill the process mid-session, relaunch with `"reopen": "restore"`, and read what comes back and what `session.json` held. *Reopen when:* any document is found to crash or abort the app on load, or session restore is next worked on.
- [ ] **Step 2b: `open-items.md`, Accepted limits.** Add a paragraph "Ruled on in the 2026-09-20 review (`reviews/code-review-2026-09-20.md`)" listing, one line each, what is *not* going to be fixed: #18's superseded case, #23 (a file-association open in the instant a window closes is dropped), #24, #25, and Task 7's two limits (a leading-slash link is resolved against the document's folder, not the filesystem root; a picture named by a full path outside any opened folder does not show). #26 is not a limit and stays in the review file only.
- [ ] **Step 2c: `open-items.md`, Needs a decision.** Add one unticked item, "**Later does not cancel an update that has started**" (added 2026-09-20, from #5 of the review): pressing **Update now** and then **Later** closes the dialog but the download carries on and the app still restarts under the reader. Task 9 Step 7 makes a reopened dialog say so; nothing makes Later mean it. Options: leave it (the reader did ask for the update), relabel the button to **Hide** once a download is running, or make it cancel — which needs the updater plugin's download to be abortable, unchecked. Also bump the file's "Last swept" line to 2026-09-20.
- [ ] **Step 3: `open-items.md`, Bugs.** Under "A restored scroll position degraded once, unreproduced", add: *"2026-09-20: #4 of the review is a mechanism that fits — `rememberScroll` banked the outgoing page's offset onto the incoming entry when a second tab switch landed before the first had loaded. Fixed in \<hash\>. Leave open until a release has gone by without a recurrence."*
- [ ] **Step 4: Commit**

```bash
git add docs/reviews/code-review-2026-09-20.md docs/open-items.md
git commit -m "docs: close out the 2026-09-20 review"
```

- [ ] **Step 5: Hand-off report** to the owner: commits made, gate output, smoke results (including anything not driven), and what is still unverified by construction — #5's Rust guard and its `update-failed` broadcast (need a real update), #6 (needs a non-UTF-8 argv on Linux), #27 on macOS (the test only bites on the Windows and macOS CI legs), and all of Task 9a (`checks.yml` is first exercised by the next push, the release parts by the next tag). When the review is fully closed, it and this plan move to `docs/archive/`.
