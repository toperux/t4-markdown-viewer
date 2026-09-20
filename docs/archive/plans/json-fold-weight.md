# JSON Fold Weight Budget Implementation Plan

> **Status (2026-09-21):** executed in full — squashed into d952d02 with #1, #2 and the #9
> stopgap; what Task 2
> measured is under #9 in `../../open-items.md`. One deviation: `emit` needed
> `#[cfg(test)]`, its last caller outside the tests having moved to `emit_within`. Archived
> with the review (`../reviews/code-review-2026-09-20.md`); paths and unticked boxes below
> are as they stood when it was written.

> **For agentic workers:** Execute per the **Execution** section below — it says who runs each task and in what order, and it overrides the one-subagent-per-task default of superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close #9 of the 2026-09-20 review: a JSON file nested thousands deep opens instead of hanging the window.

**Architecture:** `json.rs` adds up what a render's fold markup will cost the webview — the number of `.fold-body` spans open, summed over every newline and every fold control — and past `MAX_FOLD_WEIGHT` makes the render again with no fold markup at all: plain brackets, everything else as ever. A chunk fetched by a `more` button starts its count from how deep it sits in the document, learned from one tokenizer pass over the source before it. Nothing in `app.js`, `main.rs`, the `data-range` contract or the `extent` mechanism changes: where a chunk is cut never depended on fold markup, so a re-render stops where it did.

**Tech Stack:** Rust (`src-tauri/src/json.rs`), its unit tests, and one drive-app pass on the debug build.

**Spec:** #9 in `docs/reviews/code-review-2026-09-20.md`, its entry under *Bugs* in `docs/open-items.md`, and the **Design** section below, which is where the two spikes' numbers are. Line numbers are as of the tree just before this fix — **find code by name, not by line.**

## Design

### Spike 1 — real files through the app (debug build, WebView2, 2026-09-21)

| Fixture | HTML | Elements | Nesting | Parse | Attach + layout | Renderer WS |
|---|---|---|---|---|---|---|
| `[[1],[1],…]` × 60,000 (one chunk) | 9.4 MB | 210,000 | 2 | 126 ms | 0.8 s | 0.6–1.5 GB |
| `[\n`×d `1` `,\n]`×d, d = 500 | 0.1 MB | 2,500 | 500 | 6 ms | 0.3 s | 0.2 GB |
| d = 2,000 | 0.45 MB | 10,000 | 2,000 | 60 ms | 2.4 s | 2.1 GB |
| d = 10,000 | 2.3 MB | 50,000 | 10,000 | 1.7 s | 15.6 s | 5.2 GB |
| d = 50,000 | 11 MB | 250,000 | 50,000 | 77 s | not run | — |
| d = 150,000 (Task 10 check Z) | 33.9 MB | — | 150,000 | never answered, > 140 s | | |

Size is cheap. The first reading was "depth is quadratic"; spike 2 says what it is quadratic *in*.

### Spike 2 — synthetic markup: D nested `.fold-body` spans around L one-span lines

| D | L | Parse | Attach + layout | Renderer WS |
|---|---|---|---|---|
| 0 | 300,000 | 0.23 s | 0.7 s | 0.6 GB |
| 16 | 300,000 | 0.25 s | 2.4 s | 3.8 GB |
| 64 | 300,000 | 0.38 s | 7.9 s | 7.7 GB |
| 64, `.fold-body { display: contents }` | 300,000 | 0.37 s | 7.7 s | 8.0 GB |
| 256 | 20,000 | 0.06 s | 2.6 s | 3.3 GB |
| 256 | 300,000 | 0.87 s | no answer in 120 s | — |
| 256, `display: contents` | 300,000 | 0.81 s | no answer in 120 s | — |
| 256, a fold control on every line | 100,000 | 0.58 s | no answer in 120 s | — |
| 600 asked for | 1,000 | 11 ms | 1.2 s (510 levels made: the parser's own limit) | 0.4 GB |

Layout costs about half a second and 0.7 GB per million of **lines × spans open around them**; spike 1's deep files fit the same line. Depth alone is cheap, size alone is cheap, and CSS does not help. A depth cap of 256 — the first design — leaves check Z's file at 77 million and still hung.

Two things the spikes did **not** settle, both handed to Task 2:
- *What a fold control costs.* The row with a control on every line (256 × 100,000 ≈ 26 million, or 51 million counting the controls as lines) did not answer in two minutes, where the line model says 12–25 s. A control may weigh several lines. The weight counts it as one; checks E1–E3 measure documents built to sit just under the budget, one of them control-heavy, and say whether that is good enough.
- *Parse time* was superlinear in depth in spike 1 (77 s at 50,000 deep) and is not modelled on its own: a folding render gives up at a depth of a thousand or so, where parse measured in milliseconds, and a flat one has no depth.

### Decisions (owner, 2026-09-21)

| Decision | |
|---|---|
| Bound the measured cost directly: a weight of (spans open) summed over newlines and fold controls, `MAX_FOLD_WEIGHT = 2_000_000` — about a second and 1.5 GB. | approved |
| Past it the whole render is made without fold markup. All or nothing: a line pays for every span open around it whether or not more are opened after, so "stop opening new folds" saves nothing. | approved |
| A `more` chunk counts from its real depth in the document (`depth_at`, one tokenizer pass per click). Stateless: no frontend change. | approved |
| No depth cap, no output budget. `MAX_HTML_BYTES` stays as the backstop; only its `ponytail:` note is reworded. | approved |

What the reader gives up: fold controls, for the whole of a render whose folding would stall the window — nesting thousands deep, or (on a re-render) a far-expanded document of very short lines, which keeps its place and loses its folds. An ordinary chunk is nowhere near: indentation makes deep lines long, so `CHUNK_BYTES` of pretty-printed JSON weighs a few hundred thousand.

Known and left: the 2 MB, 400,000-deep file Task 10 check Y used was refused at 90 MB of HTML; without fold markup it comes to about 48 MB and is rendered. Flat, so spike 2 says seconds — Task 2 measures it (Z4).

Not yet measured, measured in Task 2 (Z3): the biggest *flat* first chunk a real file can produce — a file already on many lines (`[1],\n` repeated), about 28 MB of HTML, weight about 210,000, so it keeps its folds. Linear extrapolation says about 3 s. If it stalls, an output budget is back on the table — stop and raise it.

## Global Constraints

- Surgical changes only. Match the surrounding comment style (full-sentence "why" comments) and naming.
- No new dependencies. No new files beyond this plan.
- Gates, all run from the repo root, all must pass before each commit:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --check` (CI runs this on the Linux leg only — run it every time)
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`
  - `node --check src/app.js`
- All commits go straight onto `main`; no feature branches. Commit locally only. **Never push.** A user-facing commit subject is a modest sentence with **no prefix** — `release.yml` builds the release notes from every subject that does not start with `ci:`, `docs:`, `release:` or `chore:`. Each task gives its exact subject.
- End every commit message with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (or the executing model's own line).
- Bash on Windows: never `cd` in a compound command that writes; run from the repo root and use `--manifest-path`. A hook condenses shell output through `rtk`; prefix `rtk proxy` for raw output.
- The debug build shares `%APPDATA%\t4-markdown-viewer\` (`config.json` and `session.json`) with the installed app. Task 2 follows the drive-app skill's backup / `"reopen": "off"` / diff-before-restore rules.

## Execution

| Phase | Work | Who |
|---|---|---|
| 0 | Commit this plan (`docs: plan the JSON fold weight budget`). No baseline run: the tree was green at `47e8c0d` and only docs have changed since. | main session |
| 1 | Task 1, one commit | one `coder` — give it the Task 1 section by heading plus Global Constraints |
| — | Review: read the diff and the gate output | main session |
| 2 | Task 2: smoke pass, then the four gates | `tester` — it has no Skill tool: tell it to `Read` `.claude/skills/drive-app/SKILL.md` first and follow it. Raw results only. |
| — | Triage | main session |
| 3 | Task 3: close-out docs | main session |

Serial. No reviewer subagents. Stop and ask the owner only on: the installed viewer running at the start of Task 2; Z1 or Z2 not answering after the fix (the design is wrong — do not tune the constant); Z3 not answering within 60 s (an output budget is back on the table); any of E1–E3 over 5 s (the budget or the weight of a control is off — a one-line change, but the owner's number); a smoke failure that cannot be triaged.

## File Map

| File | Task | What changes |
|---|---|---|
| `src-tauri/src/json.rs` | 1 | `MAX_FOLD_WEIGHT`; `depth_at`; `emit_within` becomes a two-try wrapper over `emit_as`, which keeps the weight; `render_slice` passes `depth_at(src, start)`; the `MAX_HTML_BYTES` note; five tests |
| `docs/open-items.md`, `docs/reviews/code-review-2026-09-20.md` | 3 | #9 ticked with Task 2's numbers; status `done` |

---

### Task 1: A render too heavy to fold is made flat

**Files:**
- Modify: `src-tauri/src/json.rs` — a constant below `MAX_HTML_BYTES` / `TOO_MUCH`, the note on `MAX_HTML_BYTES`, `render_within`, `render_slice`, `emit`, `emit_within` (split into `emit_within` + `emit_as`), a new `depth_at` above `container_ends`, one existing test call site, five new tests

How the weight of a test document is reckoned, for anyone checking the numbers below: `"[\n"`×d + `"1"` + `"\n]"`×d opens its k-th bracket with k−1 spans open (the control: +k−1) and then a newline inside k spans (+k); on the way back each newline is inside k spans. About 1.5 d². d = 500 weighs about 375,000; d = 2,000 about 6 million.

- [ ] **Step 1: Write the failing tests** (append inside `mod tests`)

```rust
    /// Nested deep enough that folding it would stall the window, a document
    /// is shown without folds — and every byte of it is still on the page.
    #[test]
    fn a_document_too_heavy_to_fold_is_shown_flat() {
        let depth = 2000;
        let src = format!("{}1{}", "[\n".repeat(depth), "\n]".repeat(depth));
        let html = emit(&src, 0, usize::MAX);
        assert!(!html.contains("fold"), "fold markup in a flat render");
        assert!(balanced(&html));
        assert_eq!(strip(&html), escaped(&src));
    }

    /// Deep is not heavy: five hundred levels is a few hundred thousand, and
    /// folds all the way down.
    #[test]
    fn a_deep_document_that_is_not_heavy_still_folds() {
        let depth = 500;
        let src = format!("{}1{}", "[\n".repeat(depth), "\n]".repeat(depth));
        let html = emit(&src, 0, usize::MAX);
        assert_eq!(html.matches(r#"class="fold-body""#).count(), depth);
    }

    /// A chunk is spliced in where its `more` button was, inside every span
    /// open there, so its lines weigh what that depth makes them weigh.
    #[test]
    fn a_slice_weighs_its_lines_by_its_depth_in_the_document() {
        let members = ",\n[2]".repeat(1000);
        let deep = format!("{}1{members}{}", "[\n".repeat(3000), "\n]".repeat(3000));
        let start = deep.find(',').unwrap();
        let chunk = render_slice(&deep, start, start + members.len()).unwrap();
        assert!(!chunk.contains("fold"), "a heavy chunk folded");
        assert_eq!(strip(&chunk), members);
        // The same members one level down weigh two thousand, and fold.
        let shallow = format!("[1{members}]");
        let chunk = render_slice(&shallow, 2, 2 + members.len()).unwrap();
        assert_eq!(chunk.matches(r#"class="fold-body""#).count(), 1000);
    }

    /// `depth_at` has to agree with the emitter about what is open, so it
    /// keeps the emitter's rules: a closer that does not match closes
    /// nothing, and a bracket inside a string is text.
    #[test]
    fn depth_is_counted_by_the_emitters_own_rules() {
        assert_eq!(depth_at("[[1, 2]]", 0), 0);
        assert_eq!(depth_at("[[1, 2]]", 3), 2);
        assert_eq!(depth_at("[[1], 2]", 4), 1);
        assert_eq!(depth_at("[}, \"]\", 1]", 9), 1);
    }
```

- [ ] **Step 2: Run them and see them not compile**

Run: `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked json::`
Expected: does not compile — `cannot find function depth_at`.

- [ ] **Step 3: The constant and `depth_at`** — and nothing else yet, so the tests can be seen failing for the right reason.

Below `const TOO_MUCH`, add

```rust
/// What folding may cost one render before the render is made without it.
///
/// A fold is a `.fold-body` span around everything its container holds, so a
/// line nested twenty deep sits inside twenty of them — and that, not the size
/// of the markup or its depth alone, is what a webview pays for. Measured in
/// WebView2 (2026-09-21): laying a document out takes about half a second and
/// 0.7 GB for every million of lines × spans open around them. 300,000 lines
/// inside none lay out in 0.7 s; inside 16, 2.4 s and 3.8 GB; inside 64, 8 s
/// and 8 GB; inside 256 the window never answers again. `display: contents`
/// changes none of it.
///
/// So the weight is the number of spans open, summed over every newline — a
/// line — and every fold control, which is a box of its own inside the same
/// spans. Past this a render starts again with no folds at all: plain
/// brackets, everything else as ever. All or nothing, because a line pays for
/// every span open around it whether or not more are opened after.
///
/// An ordinary chunk comes nowhere near. Indentation makes deep lines long, so
/// `CHUNK_BYTES` of pretty-printed JSON weighs a few hundred thousand. What
/// passes this is nesting thousands deep, or a far-expanded document of very
/// short lines on a re-render — which keeps its place and loses its folds.
const MAX_FOLD_WEIGHT: usize = 2_000_000;
```

Directly above `fn container_ends`, add

```rust
/// How many containers are open at `at`, counted from the top of the document
/// by the emitter's own rules — a closer that does not match what is open
/// closes nothing. What a chunk needs to know to weigh itself: it is rendered
/// on its own, but spliced in that deep, inside every span open there.
// ponytail: a pass over everything before the chunk on every click, beside a
// reflow of the whole file that `json_region` already pays for; carry the depth
// on the button if either is ever felt.
fn depth_at(src: &str, at: usize) -> usize {
    let mut open: Vec<u8> = Vec::new();
    for token in Tokens::new(src, 0) {
        if token.start >= at {
            break;
        }
        if token.kind != Kind::Punct {
            continue;
        }
        match src.as_bytes()[token.start] {
            b'{' => open.push(b'}'),
            b'[' => open.push(b']'),
            byte @ (b'}' | b']') if open.last() == Some(&byte) => {
                open.pop();
            }
            _ => {}
        }
    }
    open.len()
}
```

- [ ] **Step 3a: See the real red.**

Run: `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked json::`
Expected: `depth_is_counted_by_the_emitters_own_rules` and `a_deep_document_that_is_not_heavy_still_folds` PASS (the second is a guard: it must still pass at the end); `a_document_too_heavy_to_fold_is_shown_flat` FAIL on "fold markup in a flat render"; `a_slice_weighs_its_lines_by_its_depth_in_the_document` FAIL on "a heavy chunk folded". (An unused-constant warning is fine here.)

- [ ] **Step 4: `emit_within` becomes two tries over `emit_as`.** Rename the existing function: replace

```rust
fn emit_within(src: &str, base: usize, budget: usize, ceiling: usize) -> String {
```

with

```rust
fn emit_as(
    src: &str,
    base: usize,
    budget: usize,
    ceiling: usize,
    folding: Option<usize>,
) -> Option<String> {
```

Its doc comment stays with it, with this paragraph added at the end:

```rust
///
/// `folding` is how many `.fold-body` spans are already open where `src` will
/// sit, or `None` for a render with no fold markup at all. A folding render
/// gives up — `None` — once it has cost more than `MAX_FOLD_WEIGHT`; one
/// without never does.
```

Directly above it, add the new `emit_within`:

```rust
/// Render `src` folded if the webview can afford that, and flat if not.
/// `depth` is how many containers are already open where `src` begins — none
/// for a document, `depth_at` for a chunk of one. Where the chunk is cut does
/// not depend on which: the budget and the commas are the same either way, so
/// a re-render stops where it did.
fn emit_within(src: &str, base: usize, budget: usize, ceiling: usize, depth: usize) -> String {
    emit_as(src, base, budget, ceiling, Some(depth))
        .or_else(|| emit_as(src, base, budget, ceiling, None))
        .unwrap_or_default()
}
```

Inside `emit_as`:

After `let mut at_line_start = true;` add

```rust
    // How many `.fold-body` spans are open around what is being written, those
    // of the document this is a chunk of included, and what that has come to:
    // see `MAX_FOLD_WEIGHT`.
    let mut open = folding.unwrap_or(0);
    let mut weight = 0usize;
```

Replace the head of the token loop

```rust
        if out.len() > ceiling {
            return out;
        }
        let text = &src[token.start..token.end];
```

with

```rust
        if out.len() > ceiling {
            return Some(out);
        }
        if folding.is_some() && weight > MAX_FOLD_WEIGHT {
            return None;
        }
        let text = &src[token.start..token.end];
        if open > 0 {
            weight += open * text.bytes().filter(|b| *b == b'\n').count();
        }
```

(the comment above the ceiling check stays where it is). In the `byte @ (b'{' | b'[')` arm, replace

```rust
                    let folds = has_body(src, token.start, closer);
```

with

```rust
                    let folds = folding.is_some() && has_body(src, token.start, closer);
```

and replace

```rust
                    if folds {
                        out.push_str(r#"<span class="fold-body">"#);
                    }
                    stack.push((closer, folds));
```

with

```rust
                    if folds {
                        out.push_str(r#"<span class="fold-body">"#);
                        // The control is paid for by what was open around it.
                        weight += open;
                        open += 1;
                    }
                    stack.push((closer, folds));
```

In the closer arm, replace

```rust
                    let (_, folds) = stack.pop().unwrap();
                    if folds {
                        out.push_str("</span>");
                    }
```

with

```rust
                    let (_, folds) = stack.pop().unwrap();
                    if folds {
                        out.push_str("</span>");
                        open -= 1;
                    }
```

In the comma arm, replace `return out;` (after `finish(…)`) with `return Some(out);`, and the function's last line `out` with `Some(out)`.

Known and left: a folding try that passes the *ceiling* before the weight comes back as it is and is refused, without a flat try that might have fitted. Only a light document of more than 64 MB gets there, which no first chunk can be (52 MB at most), and a re-render that big falls back to the first chunk anyway.

- [ ] **Step 5: The three callers.**

`emit` — replace the function and its doc comment with

```rust
/// `emit_within` under the ceiling every real render has, from the top of a
/// document.
fn emit(src: &str, base: usize, budget: usize) -> String {
    emit_within(src, base, budget, MAX_HTML_BYTES, 0)
}
```

`render_within` — replace `emit_within(src, 0, budget, ceiling)` with `emit_within(src, 0, budget, ceiling, 0)`.

`render_slice` — replace

```rust
    let html = emit(slice, start, CHUNK_BYTES);
```

with

```rust
    let html = emit_within(
        slice,
        start,
        CHUNK_BYTES,
        MAX_HTML_BYTES,
        depth_at(src, start),
    );
```

(let `cargo fmt` lay it out), and add a paragraph to the end of `render_slice`'s doc comment:

```rust
///
/// Rendered on its own, but not as if it were the top of a document: it is
/// spliced in where the button was, so it weighs its folds from that depth —
/// see `depth_at` and `MAX_FOLD_WEIGHT`.
```

In the tests, `a_re_render_past_the_ceiling_falls_back_to_the_first_chunk` calls `emit_within(&src, 0, usize::MAX, usize::MAX)`: add a fifth argument, `0`.

Run: `grep -n "emit_within(\|emit_as(" src-tauri/src/json.rs` and check every call has five arguments.

- [ ] **Step 6: A guard for the claim the design rests on** (append inside `mod tests`). It passes as soon as it is written — it is there so that nobody later makes a cut depend on fold markup.

```rust
    /// A re-render has to stop where the render before it did, whichever of
    /// the two was folded: cuts, ranges and text are the same either way.
    #[test]
    fn a_flat_render_cuts_where_a_folded_one_does() {
        let src = "[[1,2,3],[4,5,6],[7,8,9]]";
        let folded = emit_as(src, 0, 4, usize::MAX, Some(0)).unwrap();
        let flat = emit_as(src, 0, 4, usize::MAX, None).unwrap();
        assert!(folded.contains("fold-body") && !flat.contains("fold"));
        assert!(first_range(&folded).is_some());
        let ranges = |html: &str| -> Vec<String> {
            html.match_indices("data-range=\"")
                .map(|(at, open)| {
                    let rest = &html[at + open.len()..];
                    rest[..rest.find('"').unwrap()].to_string()
                })
                .collect()
        };
        assert_eq!(ranges(&folded), ranges(&flat));
        assert_eq!(strip(&folded), strip(&flat));
    }
```

- [ ] **Step 7: Make the `MAX_HTML_BYTES` note true.** Replace

```rust
// ponytail: a refusal, not a cut. Ending the chunk on output as well as on
// source is the real fix (review 2026-09-20, #9); it needs a re-render to stop
// at the same place, which a budget in source bytes cannot say.
```

with

```rust
// ponytail: a refusal, not a cut, and a backstop only. What hung the window in
// #9 of the 2026-09-20 review was lines inside nested fold spans, not how much
// markup there was — see `MAX_FOLD_WEIGHT`. End a chunk on output as well as on
// source if a real file ever proves size alone too much; a re-render would
// then have to stop at the same place, which a budget in source bytes cannot
// say.
```

The doc comment above it and `TOO_MUCH` stay as they are.

- [ ] **Step 8: Run the tests**

Run: `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked json::`
Expected: all `json::tests` PASS. Two existing tests render documents heavy enough to come out flat now, and neither looks at fold markup, so both should still pass: `trailing_commas_nested_deep_render_in_linear_time` (no `more` button, under 3 s) and `a_render_past_the_ceiling_is_refused` (a 10,000-deep document must still pass 1 MB flat: 119 bytes a level, 1.19 MB). If either fails, stop and report; do not change their numbers unasked.

- [ ] **Step 9: Gates, then commit**

```bash
git add src-tauri/src/json.rs
git commit -m "Open JSON nested thousands deep without hanging the window"
```

---

### Task 2: Smoke pass (tester + drive-app)

**Before anything else:** `powershell -NoProfile -Command "Get-Process t4-markdown-viewer -ErrorAction SilentlyContinue"`. If the installed viewer is running, stop and ask the owner — do not close it, do not proceed around it.

Follow `.claude/skills/drive-app/SKILL.md` for the build (`TAURI_CONFIG` identifier override — rebuild, `src-tauri` has changed), the config/session backup, `"reopen": "off"`, the port, and the restore. Fixtures and screenshots go in the scratchpad, never the repo. Keep a running notes file there and append each result as it comes, so they survive a hang. A fresh launch for each of checks Z1–Z4; every `eval` gets 60 s, and one that does not answer is reported as "NO ANSWER", with `Get-Process t4-markdown-viewer,msedgewebview2 | Select-Object Name,Id,WS,CPU`, and the debug process killed (`taskkill //F //PID`). Raw values only.

- [ ] **Fixtures** (`node -e`, absolute scratch paths):
  - `deep-150k.jsonc` — `"[\n".repeat(150000) + "1" + ",\n]".repeat(150000)` (750 KB; check Z's file)
  - `deep-50k.jsonc` — the same with 50,000
  - `deep-400k.jsonc` — the same with 400,000 (2 MB; check Y's file)
  - `flat-lines.json` — `"[\n" + "[1],\n".repeat(200000) + "[1]\n]"` (1 MB on many lines, so it is not reflowed)
  - `records.json` — `JSON.stringify(Array.from({ length: 60000 }, (_, i) => ({ id: i, name: "item " + i, tags: ["a", "b"] })), null, 2)` (about 5.5 MB, a dozen chunks; for R3 — two chunks of `flat-lines.json` come to 55 MB of HTML, close enough to the 64 MB ceiling for a re-render to fall back and fail R3 for the wrong reason)
  - `edge-z.json` — `"[\n".repeat(1100) + "1" + "\n]".repeat(1100)` (weight about 1.8 million: just under the budget)
  - `edge-controls.json` — `"[\n".repeat(256) + "[1],\n".repeat(3000) + "[1]" + "\n]".repeat(256)` (about 1.6 million, nearly all of it controls and lines 256 deep)
  - `edge-lines.json` — `"[\n".repeat(16) + "1,\n".repeat(110000) + "1" + "\n]".repeat(16)` (about 1.8 million, all of it lines 16 deep; 330 KB, so one chunk)
  - `deep-300.json` — `"[\n".repeat(300) + "1" + "\n]".repeat(300)`
  - `deep-2000.json` — the same with 2,000
  - `nested.json` — `JSON.stringify({ a: { b: [1, 2, { c: [3, 4] }] }, d: [5] }, null, 2)`
- [ ] **The staged probe**, used by Z1–Z4. Separate `eval`s, in order, so a partial result survives (`P` = the fixture's forward-slash path):
  1. `(async () => { const t = performance.now(); try { const d = await invoke("load_file", { path: P }); window.__html = d.html; return [Math.round(performance.now() - t), d.html.length, (d.html.match(/class="more"/g) || []).length, (d.html.match(/class="fold-body"/g) || []).length]; } catch (e) { return String(e); } })()`
  2. `(() => { const t = performance.now(); const el = document.createElement("div"); el.innerHTML = window.__html; window.__el = el; return [Math.round(performance.now() - t), el.querySelectorAll("*").length]; })()`
  3. `(() => { const t = performance.now(); document.body.appendChild(window.__el); const h = window.__el.offsetHeight; return [Math.round(performance.now() - t), h]; })()`
  4. `(() => { window.__el.remove(); window.__el = null; window.__html = null; return true; })()`
  5. Open `P` as a tab the way the skill does, poll `shownToken === renderToken` once a second, and report: ms until true; whether `eval "document.title"` answers and in how long; `(() => { const t = performance.now(); window.scrollTo(0, 5000); return [Math.round(performance.now() - t), window.scrollY]; })()`; a screenshot; and the `Get-Process` line above.
- [ ] **Z1 — `deep-150k.jsonc`.** Expect stage 1's last number (fold bodies) to be `0`, and every stage to answer. This is the check that hung for more than 140 s. **No answer is a stop-and-ask.**
- [ ] **Z2 — `deep-50k.jsonc`.** Expect `0` fold bodies. Stage 2 alone took 77 s before the fix. **No answer is a stop-and-ask.**
- [ ] **Z3 — `flat-lines.json`.** The biggest flat first chunk a real file makes; expect about 28 MB of HTML, one `more` button, and about 105,000 fold bodies — it is light, so it keeps its folds. **No answer within 60 s is a stop-and-ask, not a triage.**
- [ ] **Z4 — `deep-400k.jsonc`.** Was refused; expect it rendered now, flat, at about 48 MB. If stage 1 returns the "too much to show at once" string instead, report that and skip the other stages. A measurement, not a pass or fail.
- [ ] **E1–E3 — the budget at its edge.** The staged probe (a fresh launch each) on `edge-z.json`, `edge-controls.json` and `edge-lines.json`: the heaviest documents that still fold, one of each kind. Expect stage 1's fold-body count to be `1100`, `3257` and `16` — a `0` means the fixture tripped the budget and measures nothing; report it and go on. The numbers that matter are stage 3's milliseconds and the renderer's working set: the budget was set to mean about a second and 1.5 GB. **Any stage over 5 s, or no answer, is a stop-and-ask**: the constant is too high or a fold control weighs more than a line (see Design), and which of the three it was says which.
- [ ] **R1 — light folds, heavy does not.** Open `deep-300.json`: `eval "[document.querySelectorAll('#content .fold').length, document.querySelectorAll('#content .fold-body').length]"` → expect `[300, 300]`. Open `deep-2000.json`: the same → expect `[0, 0]`, and `eval "document.querySelector('#content').textContent.length"` → expect 8001, the file's own length (nothing lost); report the number whatever it is. A screenshot of each.
- [ ] **R2 — folding still works.** Open `nested.json`; `click` the first `.fold` control; report `document.querySelector('#content .fold').getAttribute('aria-expanded')` → expect `"false"`; screenshot; click it again → `"true"`.
- [ ] **R3 — `more` still works, and a re-render stops where it did.** Open `records.json` as a tab, `click` the `more` button, wait for it to go, and report `[document.querySelectorAll('#content .more').length, currentEntry(activeTab()).extent, document.querySelectorAll('#content .fold').length > 0]` — expect one button, an `extent` above 524288, and `true`. Then `eval "refresh()"`, let it settle, and report the same triple → expect the same triple.
- [ ] **Restore, then gates.** Close the app, diff-before-restore per the skill, confirm both files `cmp`-identical to the backups, confirm `git status --short` is clean, then run the four gates and report their output.

---

### Task 3: Close-out

**Files:**
- Modify: `docs/open-items.md`
- Modify: `docs/reviews/code-review-2026-09-20.md`

- [ ] **Step 1: `open-items.md`.** Tick the #9 item under *Bugs* (`- [x]`) and append a closing note in the file's italic style: what was wrong (lines inside nested fold spans, not size and not depth alone — cite the two spike tables in this plan), the fix and its commit hash, and Task 2's Z1–Z4 numbers. Correct the item's own account where the spikes overtook it: "The real fix is a second way to end a chunk" was the theory; say so rather than delete it. Add to *Accepted limits*, under the 2026-09-20 paragraph: "A JSON render whose folding would stall the window — nesting thousands deep, or a far-expanded document of very short lines on a re-render — is shown without fold controls (`MAX_FOLD_WEIGHT` in `json.rs`). Everything else about it is as ever."
- [ ] **Step 2: The review.** #9's Status becomes `done (\`d952d02\`, \`<Task 1's hash>\`)`; prefix its finding with one sentence giving the outcome and pointing at this plan. Rewrite the status note at the top: every item `done`, `wontfix` or `park`; keep the not-driven and unverified-by-construction lists as they are.
- [ ] **Step 3: Commit**

```bash
git add docs/open-items.md docs/reviews/code-review-2026-09-20.md
git commit -m "docs: close #9 of the 2026-09-20 review"
```

- [ ] **Step 4: Hand-off report** to the owner: commits, gate output, Z1–Z4 and R1–R3 raw values, anything not driven. Two questions for them: whether `README.md`'s "every `{…}` and `[…]` foldable" wants a clause about the limit (left alone here: no real file meets it); and whether the review and both plans now move to `docs/archive/` — #28 and the crash-loop question live on in `open-items.md`, which is what the 2026-09-12 review's closing did with its parked limits.
