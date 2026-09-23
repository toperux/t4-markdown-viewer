# Anchor Attribute Scanner (#28) Implementation Plan

> **For agentic workers:** Execute per the **Execution** section below — it says who runs each task and in what order, and it overrides the one-subagent-per-task default of superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close #28 of the 2026-09-20 review: an `id=` inside another attribute's value no longer loses or replaces a recovered anchor.

**Architecture:** `attribute()` in `render.rs` stops searching the attribute text for the key and instead reads it the way HTML does: a name, then an optional `=value` (double-quoted, single-quoted or bare), each value skipped whole. The first attribute *named* `key` wins. Its one caller, `anchor_target`, is unchanged, and `is_safe_id` stays the last gate on the value.

**Tech Stack:** Rust (std only), comrak's raw-HTML nodes as input.

**Spec:** `docs/open-items.md` → *Deferred from the 2026-09-20 review* → "#28 Anchor recovery misreads an `id=` inside another attribute's value"; `docs/archive/reviews/code-review-2026-09-20.md`, row 28.

## Findings (measured 2026-09-24, before this plan)

`attribute()` (`render.rs:46`) lowercases the text, `find`s the key, and accepts the first hit that has whitespace before it and `=` after it. It does not know where a quoted value ends. Traced and confirmed by running the Task 1 tests on the current code:

| Attribute text | Current result | Effect |
|---|---|---|
| ` href="p?a id=q" id="x"` | `q"` (fails `is_safe_id`) | no anchor: `[…](#x)` goes nowhere |
| ` title="the id=3 entry" id="x"` | `3` (passes) | **wrong anchor**: `#3` exists, `#x` does not |
| ` id="x"title="y"` (key `title`) | `None` | an attribute right after a quote is missed |
| ` id="x` (unclosed) | `"x` (fails) | no anchor, as it should be |

Not a safety issue: any value must pass `is_safe_id` (letters, digits, `-_.:`, at most 128) and goes into a tag generated from scratch.

A dry run of this plan's code, since reverted: all three new tests failed on the current code for the reasons above and passed with the new scanner; all 151 tests passed; clippy was clean. Renders of every `examples/*.md` (including the defect-96194 file's 18 raw anchors), the README and an autolink sample were byte-identical before and after.

Two deliberate behaviour changes beyond #28, both measured with a probe on the old and new code and both pinned by Task 1's tests:

- **A bare value ends at whitespace or `>` only**, as in HTML, no longer also at `/`. `<a id=a/b>` used to recover the anchor `a`; it now reads `a/b`, which `is_safe_id` refuses. The document named `a/b`, not `a`. A trailing `/` of a self-closing tag is already stripped by `anchor_target` before `attribute` sees it (`<a id=x />` still gives `x`).
- **An empty `id` falls back to `name`.** `<a id="" name="n">` used to recover nothing: the old `attribute` returned `Some("")`, so `anchor_target` never asked for `name`, and `is_safe_id` refused the empty string. Now an empty value counts as no value, and the anchor is `n`, which is what a browser does with that tag. A value-less `<a id name="n">` gave `n` before and still does.

## Global Constraints

- Surgical changes only: `attribute()` and new tests in `render.rs`. Match the surrounding comment style (full-sentence "why" comments).
- No new dependencies.
- Gates, all run from the repo root, all must pass before each commit:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --check` (CI runs this on the Linux leg only — run it every time)
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`
  - `node --check src/app.js`
- All commits go straight onto `main`; no feature branches. Commit locally only. **Never push.** A user-facing commit subject is a modest sentence with **no prefix**; `docs:` keeps the close-out out of the release notes.
- End every commit message with the executing model's `Co-Authored-By:` line.
- Bash on Windows: never `cd` in a compound command that writes; run from the repo root and use `--manifest-path` / `git -C`. A hook condenses shell output through `rtk`; prefix `rtk proxy` for raw output.

## Review Focus

Inputs a person's document may hold that the scanner must read as HTML does, most likely first. Each is pinned by a Task 1 test:

1. **An `id=` inside a `title`/`href` value, before the real `id`**: the bug itself, both the missing-anchor and the wrong-anchor forms.
2. **A bare attribute** (`<a download id="x">`): a name with no `=` must not swallow the next attribute.
3. **Upper case and spaces around `=`** (`ID = 'x'`): both worked before and must still work.
4. **Non-ASCII text** in another attribute's value (`title="café id=q"`) or at the start of an attribute name (`é=1`): must neither panic nor match. The name's first character is sliced by its UTF-8 length (`first`); a byte-1 slice would panic there.
5. **Two `id`s**: the first wins, as in HTML, and as before. And an empty `id` beside a `name`: the `name` is the anchor.

## Execution

| Phase | Work | Who |
|---|---|---|
| 0 | Commit this plan (`docs: plan the anchor attribute scanner for #28`). | main session |
| 1 | Task 1, one commit | main session (one function and three tests, all given below) |
| 2 | Task 2: close-out docs | main session |

Serial. No smoke pass: anchor recovery is pure Rust with no UI of its own, the end-to-end test goes through `render`, and Task 1 Step 5's render comparison covers the real documents. Stop and ask the owner only if a render comparison differs, or an existing anchor test fails.

## File Map

| File | Task | What changes |
|---|---|---|
| `src-tauri/src/render.rs` | 1 | `attribute()` rewritten as a scanner with a doc comment; three tests |
| `docs/open-items.md`, `docs/closed-items.md` | 2 | #28 moved to closed |
| `docs/archive/reviews/code-review-2026-09-20.md` | 2 | Row 28's status `park` → `done (<hash>)` |
| `docs/plans/anchor-attribute-scanner.md` → `docs/archive/plans/` | 2 | This plan, archived with a status line |

---

### Task 1: Read attributes the way HTML does

**Files:**
- Modify: `src-tauri/src/render.rs`: `attribute()` (currently lines 46-82); tests inside `mod tests`

**Interfaces:**
- Consumes: nothing new.
- Produces: `fn attribute<'a>(attrs: &'a str, key: &str) -> Option<&'a str>`, the same signature. `key` is lower-case ASCII (`"id"`, `"name"`); the match ignores ASCII case.

- [ ] **Step 1: Write the failing tests.** In `render.rs`'s `mod tests`, directly above `anchor_recovery_carries_nothing_but_the_name` (and its doc comment), add:

```rust
    /// Attributes are read one after another, each value skipped whole, so a
    /// `key=` inside some other attribute's value is text, not the attribute.
    #[test]
    fn attribute_skips_a_key_inside_another_value() {
        assert_eq!(attribute(r#" href="p?a id=q" id="x""#, "id"), Some("x"));
        assert_eq!(attribute(r#" title="the id=3 entry" id="x""#, "id"), Some("x"));
        assert_eq!(attribute(" title='a id=b' id=x", "id"), Some("x"));
        assert_eq!(attribute(r#" title="café id=q" id="x""#, "id"), Some("x"));
        assert_eq!(attribute(r#" title="x id=q""#, "id"), None);
        // A name may start with a character wider than a byte.
        assert_eq!(attribute(" é=1 id=x", "id"), Some("x"));
    }

    /// The rest of how HTML spells an attribute.
    #[test]
    fn attribute_reads_a_tag_the_way_html_does() {
        assert_eq!(attribute(r#" id="x""#, "id"), Some("x"));
        assert_eq!(attribute(" ID = 'x' ", "id"), Some("x"));
        assert_eq!(attribute(" id=f5", "id"), Some("f5"));
        // A bare attribute has no value to skip.
        assert_eq!(attribute(r#" download id="x""#, "id"), Some("x"));
        // The first of two wins, as in HTML.
        assert_eq!(attribute(r#" id="a" id="b""#, "id"), Some("a"));
        // A name that merely ends in the key is another attribute.
        assert_eq!(attribute(r#" data-id="q" id="x""#, "id"), Some("x"));
        // No whitespace is needed after a quoted value.
        assert_eq!(attribute(r#" id="x"title="y""#, "title"), Some("y"));
        // An unclosed quote runs to the end of the tag.
        assert_eq!(attribute(r#" id="x"#, "id"), None);
        assert_eq!(attribute(" id", "id"), None);
        assert_eq!(attribute("", "id"), None);
        // A bare value runs to whitespace, slashes included; `is_safe_id`
        // then refuses it rather than an anchor being cut from its front.
        assert_eq!(attribute(" id=a/b", "id"), Some("a/b"));
        // An empty value is no value, so the caller can go on to `name`.
        assert_eq!(attribute(r#" id="" name="n""#, "id"), None);
    }

    /// End to end: the anchor the document meant, not one made from the text
    /// of another attribute.
    #[test]
    fn an_id_inside_another_attribute_does_not_steal_the_anchor() {
        let html = render("<a title=\"the id=3 entry\" id=\"x\"></a>t\n\nSee [x](#x).\n");
        assert!(html.contains("<span id=\"x\"></span>"), "{html}");
        assert!(!html.contains("id=\"3\""), "{html}");
        let html = render("<a href=\"p?a id=q\" id=\"x\"></a>t\n");
        assert!(html.contains("<span id=\"x\"></span>"), "{html}");
        // An empty id leaves the name to do the job, as a browser would.
        let html = render("<a id=\"\" name=\"n\"></a>t\n");
        assert!(html.contains("<span id=\"n\"></span>"), "{html}");
    }
```

- [ ] **Step 2: See them fail.**
Run: `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked render::`
Expected: the three new tests FAIL, and every other `render::` test passes. `attribute_skips_a_key_inside_another_value` fails with `left: Some("q\"")`, `right: Some("x")`; `attribute_reads_a_tag_the_way_html_does` with `left: None`, `right: Some("y")` (the `title` after a quote); the end-to-end test on its first assertion.

- [ ] **Step 3: Record the "before" renders.** Append this throwaway test inside `mod tests` (Step 5 deletes it):

```rust
    // THROWAWAY — deleted before commit.
    #[test]
    #[ignore]
    fn dump_renders() {
        let out = std::path::PathBuf::from(std::env::var("T4_DUMP").unwrap());
        std::fs::create_dir_all(&out).unwrap();
        let mut docs: Vec<std::path::PathBuf> = std::fs::read_dir("../examples")
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "md"))
            .collect();
        docs.push("../README.md".into());
        for p in docs {
            let html = render(&std::fs::read_to_string(&p).unwrap());
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            std::fs::write(out.join(format!("{name}.html")), html).unwrap();
        }
    }
```

Run, with `<S>` the session's scratchpad directory (absolute, forward slashes):
`T4_DUMP=<S>/anchors-before rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked dump_renders -- --ignored`
Expected: PASS; one `.html` per example file plus `README.md.html`.

- [ ] **Step 4: Replace `attribute()`.** Replace the whole function (from `fn attribute<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {` through its closing `}`, just above `/// The name a bare`) with:

```rust
/// The value of the attribute named `key`, in any case, in a tag's attribute
/// text. Read the way HTML reads it — a name, then an optional `=value`, quoted
/// or bare, skipped whole — so a `key=` inside another attribute's value is
/// never taken for the attribute itself. The first attribute of that name wins.
fn attribute<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    let mut rest = attrs;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        let first = rest.chars().next()?.len_utf8();
        // A name runs to whitespace, `=`, `/` or `>`, but always takes its
        // first character, so a stray `=` cannot stall the scan.
        let name_end = rest[first..]
            .find(|c: char| c.is_whitespace() || matches!(c, '=' | '/' | '>'))
            .map_or(rest.len(), |i| i + first);
        let name = &rest[..name_end];
        rest = rest[name_end..].trim_start();

        let mut value = None;
        if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            match after.chars().next() {
                Some(quote @ ('"' | '\'')) => {
                    // An unclosed quote runs to the end of the tag, so nothing
                    // after it is an attribute.
                    let end = after[1..].find(quote)?;
                    value = Some(&after[1..1 + end]);
                    rest = &after[2 + end..];
                }
                _ => {
                    // HTML lets the value go unquoted, in which case it runs to
                    // the first whitespace or to the tag itself. `is_safe_id`
                    // still has the last word on what such a value may contain.
                    let end = after
                        .find(|c: char| c.is_whitespace() || c == '>')
                        .unwrap_or(after.len());
                    value = Some(&after[..end]);
                    rest = &after[end..];
                }
            }
        }
        if name.eq_ignore_ascii_case(key) {
            return value.filter(|v| !v.is_empty());
        }
    }
}
```

Run: `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked render::`
Expected: all `render::` tests PASS, the three new ones included.

- [ ] **Step 5: Record the "after" renders and compare.**
Run: `T4_DUMP=<S>/anchors-after rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked dump_renders -- --ignored`
Then: `rtk proxy diff -r <S>/anchors-before <S>/anchors-after && echo IDENTICAL`
Expected: `IDENTICAL`. Any difference: stop and ask, with the diff.
Then delete the `dump_renders` test (the whole `// THROWAWAY` block).

- [ ] **Step 6: Gates.** Run `cargo fmt --manifest-path src-tauri/Cargo.toml` first (it wraps one long `assert_eq!` in Step 1's tests), then all four gates from Global Constraints. Expected: all pass, 151 tests (148 + 3).

- [ ] **Step 7: Commit.**

```bash
git -C . add src-tauri/src/render.rs
git -C . commit -m "Keep an id inside another attribute from taking an anchor's place" -m "Co-Authored-By: <executing model line>"
```

### Task 2: Close out

**Files:** `docs/open-items.md`, `docs/closed-items.md`, `docs/archive/reviews/code-review-2026-09-20.md`, this plan

- [ ] **Step 1:** Move "#28 Anchor recovery misreads an `id=` inside another attribute's value" from `open-items.md` (*Deferred from the 2026-09-20 review*) to the end of the same section in `closed-items.md`, ticked, with an italic closing note: the commit hash, the wrong-anchor case (`title="the id=3 entry"` gave `#3`), the two behaviour changes (`id=a/b` refused rather than cut to `a`; an empty `id` falls back to `name`), and the render-comparison result. If *Deferred from the 2026-09-20 review* in `open-items.md` is then empty, remove its heading and intro paragraph.
- [ ] **Step 2:** In `docs/archive/reviews/code-review-2026-09-20.md`, row 28: Status `park` → `done (<hash>)`, and put `**Fixed 2026-09-24.**` in front of `**Proposed: park.**`. In the status block at the top (a `> ` blockquote, wrapped: the phrase runs from line 5 onto line 6), change "#28 is `park`, tracked with the crash-loop question from #2 in `../../open-items.md`" to "#28 was parked, then fixed on 2026-09-24 (`<hash>`)", and re-wrap that paragraph with a `> ` on every line.
- [ ] **Step 3:** `git mv docs/plans/anchor-attribute-scanner.md docs/archive/plans/anchor-attribute-scanner.md`, and add under its title: `> **Status (<date>):** executed in full — <hash>; paths and unticked boxes below are as they stood when it was written.` Note any deviation in the same line.
- [ ] **Step 4:** Commit: `docs: close #28, the anchor attribute scanner`.
