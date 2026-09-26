# Heading-Id Collision Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** No id a document gives can take on the app's own styles or be found in place of the app's elements, and every in-document link still lands where it did on GitHub.

**Architecture:** Rust renders heading ids and recovered `<a id>` anchor spans with the prefix `user-content-` (GitHub's), so none can equal an app id. Links inside documents keep the names as written. The frontend adds the prefix at the moment of the jump, in `docTarget`, which `jumpToAnchor` and `pushAnchorEntry` both use. Those two are already the funnel for in-page links, same-file links, cross-file `a.md#x` links and restored sessions. In-page `#x` clicks stop being left to the browser, which jumps to the first element of that name, and the app's chrome comes before `#content`. comrak's footnote ids (`fn-*`, `fnref-*`) stay bare. A Rust test pins that no app id can ever match them or the prefix.

**Tech Stack:** Rust (`src-tauri/src/render.rs`, comrak 0.55 heading adapter), plain JS (`src/app.js`).

**Spec:** the open-items entry "A heading's id can collide with the app's own element ids" (`docs/open-items.md`, Deferred), plus these decisions, taken with the owner on 2026-09-26:
1. Prefix document ids, rather than renaming ~70 app ids or renaming in the DOM after each render.
2. The prefix is `user-content-`. A link already written against GitHub's ids (`#user-content-setup`) also works.
3. Footnote ids stay bare, and a test guards against app ids that could match them.
4. A link to an id the document doesn't have does nothing: no jump and no history entry.

## Outcome (2026-09-26)

Done: cfd1e00; 188 tests pass, and checks A–I passed in the app. Found while running it:

- The plan's Step 2 command needs `--` before the two filters: cargo takes one filter before it.
- A copied section shows no toast; the button gains `copied` for 1.2 s. Check H read the class.
- `sendkeys.ps1` sent Alt+←/→ to another window when Windows' foreground lock refused the switch. Alt+←/→ went in as CDP key events instead, which the app's `onKeydown` handles. The gotcha is now in the drive-app skill.
- The README's new sentences were moved after the `javascript:` sentence, so its "instead" reads right.

## Global Constraints

- The prefix is exactly `user-content-`. Rust and JS each hold it once, as a named constant.
- Links in rendered HTML keep the document's own names. The heading's `<a class="anchor" href="#slug">` stays unprefixed, and so does comrak's `href="#fn-…"`.
- Saved sessions keep working unchanged. `entry.hash` holds the name as linked, and the prefix is added only at jump time.
- `:target` has to keep matching the heading, since that is what applies `scroll-margin-top` under the sticky bar (`base.css` `.markdown-body :target`). So the jump still sets `location.hash`, to the element's real (prefixed) id.
- Our heading markup stays byte for byte comrak's. `repeated_headings_get_comraks_ids_quickly` asserts `render(x) == comrak::markdown_to_html(x, &options(x))`. So comrak's own `header_id_prefix` is set to the same constant: comrak prefixes the id and, with `header_id_prefix_in_href` off, keeps the href bare. That is exactly the shape wanted.
- `[Back to top](#top)` and a bare `#` keep working: when no document element matches and the name is empty or `top`, the browser's default scroll-to-top is left alone.
- Accepted: with both `## Toast` and `## User content toast` in one document, `#user-content-toast` lands on the second heading, where GitHub would pick the first. That needs contrived headings, and plain `#toast` is unaffected.
- Mermaid's `Mermaid-N` ids are untouched. They are capitalised, so they can never be a slug, and they are app-generated.
- Rust changes need `cargo fmt` before committing. CI's Format step runs on the Linux leg only.
- The commit goes straight to `main` and is never pushed. The fix commit has no prefix, since it fixes shipped behaviour. The docs commits take `docs:`.

## Review Focus

1. **A heading named like an app element:** `## Image`, `## Icons`, `## Tree`, `## Toast`, `## Error`. Each renders at its normal size and stays visible. (Step 6, checks A and B.)
2. **A link whose name is also an app id,** such as `[x](#tree)` with the sidebar open. It lands on the document's heading and never scrolls or focuses the sidebar. (Check C.)
3. **Footnotes:** the reference jumps to the note and the back-link returns. Neither is prefixed. (Check D.)
4. **Cross-file and restored jumps:** `other.md#tree` lands on the heading when the file opens. Back and Forward between anchor entries return to the same spots. (Checks E and F.)
5. **A dead link** (`#nowhere`, or an app-only name like `#sidebar` with no such heading): no scroll and no history entry. `#top` and `#` still scroll to the top, as before. (Checks G and I.)

---

### Task 1: Prefix document ids, and jump through the prefix

**Files:**
- Modify: `src-tauri/src/render.rs`: `options()`'s `header_id_prefix` and its comment (~lines 40–41), `HeadingIds::enter` (~line 161), the anchor-span writer in `html_of` (~line 350), the doc comment above `HeadingIds` (~line 125), and the tests that assert heading or anchor ids (~lines 553, 587, 596, 604, 656–670, 1163–1168, and any others `grep -n 'id=' render.rs` finds in `mod tests`)
- Modify: `src/app.js`: a new `DOC_ID_PREFIX` / `docTarget` next to `decodeId` (~line 2076), `jumpToAnchor` (~2091), `pushAnchorEntry` (~2109), and the `#` branch of `onLinkClick` (~2900)
- Modify: `README.md`, the "With one exception: explicit anchor targets" design note (~line 454)
- Test: `src-tauri/src/render.rs` `mod tests`

**Interfaces:**
- Rust: `const DOC_ID_PREFIX: &str = "user-content-";` (module-private in render.rs)
- JS: `const DOC_ID_PREFIX = "user-content-";` and `docTarget(id: string): Element | null`

- [x] **Step 1: Write the failing Rust tests**

In `mod tests`, add:

```rust
#[test]
fn document_ids_carry_the_prefix() {
    // A heading and an anchor target both come out prefixed, so neither can
    // equal an id of the app's own; the heading's link keeps the bare name,
    // which is what documents write and what the page maps at the jump.
    let html = render("## Image\n\n<a id=\"tree\"></a>text\n\nSee [x](#image).\n");
    assert!(html.contains(r#"<h2 id="user-content-image""#), "{html}");
    assert!(html.contains(r##"href="#image""##), "{html}");
    assert!(html.contains(r#"<span id="user-content-tree"></span>"#), "{html}");
    assert!(!html.contains(r#"id="image""#), "{html}");
    assert!(!html.contains(r#"id="tree""#), "{html}");
}

#[test]
fn no_app_id_can_match_a_document_id() {
    // Document ids are `user-content-…`, plus comrak's footnote `fn-…` and
    // `fnref-…`, which stay bare. The page looks both up, so no element of
    // the app's own may be named that way — or a jump could land on it.
    let page = include_str!("../../src/index.html");
    let ids: Vec<&str> = page
        .split(" id=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect();
    assert!(ids.len() > 50, "found only {} ids in index.html", ids.len());
    for id in ids {
        for reserved in ["user-content-", "fn-", "fnref-"] {
            assert!(!id.starts_with(reserved), "app id {id:?} starts with {reserved:?}");
        }
    }
}
```

- [x] **Step 2: Run them to see the first fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml document_ids no_app_id`
Expected: `document_ids_carry_the_prefix` FAILS (it finds `<h2 id="image"`), and `no_app_id_can_match_a_document_id` PASSES. The second is a guard for later edits, not a red test.

- [x] **Step 3: Implement the Rust side**

Near the top of render.rs, after `OMITTED`:

```rust
/// What every id a document gives — a heading's, an `<a id>` target's — is
/// rendered behind, as GitHub does. Without it `## Image` would be
/// `<h2 id="image">` and wear the app's own `#image` rules, and a `#tree`
/// link could land in the sidebar. Links keep the bare name; the page adds
/// this at the jump. comrak's footnote ids stay bare: see
/// `no_app_id_can_match_a_document_id`.
const DOC_ID_PREFIX: &str = "user-content-";
```

In `options()`, replace the empty prefix and its comment:

```rust
    // comrak's own heading ids go behind the same prefix our `HeadingIds`
    // writes, so the two render byte for byte alike (see
    // `repeated_headings_get_comraks_ids_quickly`). The href stays the bare
    // slug: `header_id_prefix_in_href` is off, and links are written bare.
    o.extension.header_id_prefix = Some(DOC_ID_PREFIX.into());
```

In `HeadingIds::enter`, change the write, and only that. Slug dedup, `taken` and `open` stay on bare slugs, so `exit`'s `href="#{id}"` stays bare, just as comrak's does:

```rust
        write!(out, "<h{} id=\"{DOC_ID_PREFIX}{id}\"", heading.level)?;
```

In `html_of`'s anchor loop:

```rust
        if let Some(id) = target {
            out.push_str("<span id=\"");
            out.push_str(DOC_ID_PREFIX);
            out.push_str(id);
            out.push_str("\"></span>");
        }
```

Update the `HeadingIds` doc comment's last sentence ("the ids are the ones comrak would have given") so it says that the slugs are comrak's, rendered behind `DOC_ID_PREFIX`.

- [x] **Step 4: Update the existing tests**

Every assertion in `mod tests` that a heading id or anchor-span id is present gets the prefix: `id="some-section"` → `id="user-content-some-section"`, `<span id="f5">` → `<span id="user-content-f5">`, the `a`, `a-1`, `a-2`, `a-3`, `a-19999` loop, and so on. Keep the *negative* assertions meaningful too: `!html.contains("id=\"3\"")` becomes `!html.contains("id=\"user-content-3\"")`. Tests about `attribute(...)` parsing are unaffected; leave them alone. `href="#…"` assertions stay bare.

`repeated_headings_get_comraks_ids_quickly`'s three `assert_eq!(render(…), comrak::markdown_to_html(…))` lines must pass **unchanged**. They are the proof that our ids and markup still match comrak's. If they fail, the prefix differs between `options()` and the adapter; don't edit the assertions.

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all pass (186 existing + 2 new = 188).

- [x] **Step 5: Implement the frontend side**

In `src/app.js`, directly after `decodeId`:

```js
/**
 * What Rust renders a document's own ids behind — its headings' and its
 * `<a id>` targets' — so none can take on the app's (`## Image` would wear the
 * image panel's `#image` rules). Links name them as written; the prefix goes on
 * here, at the jump.
 */
const DOC_ID_PREFIX = "user-content-";

/**
 * The element in the document that `id` names, or null. Prefixed first — no
 * app id can carry the prefix, which a test pins — then as written, which is
 * how footnotes are named and how a link copied from GitHub
 * (`#user-content-x`) already reads. The bare name is looked for inside the
 * document only: the app's own elements come first in the page, and a link
 * must never land on one.
 */
function docTarget(id) {
  if (!id) return null;
  return (
    document.getElementById(DOC_ID_PREFIX + id) ??
    els.content.querySelector(`#${CSS.escape(id)}`)
  );
}
```

`jumpToAnchor`: look up through `docTarget`, and set the hash to the element's real id, so `:target` matches it:

```js
function jumpToAnchor(id) {
  const target = docTarget(id);
  if (!target) return false;
  // Assigning the fragment the URL already carries is not a change, so the
  // browser does nothing and a second click on the same link goes nowhere.
  // Dropping it first makes every jump a jump, at no cost in history.
  clearHash();
  location.hash = target.id;
  return true;
}
```

`pushAnchorEntry`: its existence check becomes `if (!docTarget(id)) return;`. The entry still records `hash: id`, the name as linked.

`onLinkClick`, the `#` branch: the page now jumps itself. Rewrite the comment too, since the browser no longer performs the jump:

```js
  // In-page anchor: recorded, then jumped to here rather than by the browser,
  // which would go to the first element of that name — and the app's own come
  // before the document's. Except `#` and `#top` naming nothing in the
  // document: those are "back to top", which the browser does itself.
  if (href.startsWith("#")) {
    const id = decodeId(href.slice(1));
    if (!docTarget(id) && /^(top)?$/i.test(id)) return;
    event.preventDefault();
    pushAnchorEntry(id);
    jumpToAnchor(id);
    return;
  }
```

`pushAnchorEntry` decodes its argument again. Decoding an already-decoded id is what `loadPath`'s same-file branch does today too, so leave it as is.

Also update `pushAnchorEntry`'s doc comment. It says "The browser is left to perform the jump itself", which is no longer true. It should say that the jump is `jumpToAnchor`'s, run right after, and that setting `location.hash` is what keeps `:target` and its offset working.

- [x] **Step 6: Verify in the app (drive-app skill)**

`cargo fmt --manifest-path src-tauri/Cargo.toml`, then build per the skill, protecting `%APPDATA%`. Fixture `ids.md` in the scratchpad:

````markdown
# Ids

[img](#image) · [icons](#icons) · [tree](#tree) · [gh](#user-content-toast) · [dead](#nowhere) · [app](#sidebar) · [empty](#) · note[^n]

## Image
## Icons
## Tree
## Toast
## Error
## User content setup

[setup](#user-content-setup)

<!-- 60 filler paragraphs so every heading can reach the top -->

[Back to top](#top)

[^n]: The note.
````

Plus `ids-other.md` holding `[to tree](ids.md#tree)`.

With the sidebar open (so `#tree` and `#sidebar` exist in the app):

- **A. Sizes:** each h2's `getBoundingClientRect().height` is under 60 px. Before the fix, `## Image` was 823.
- **B. Visible:** `## Icons` computes `display: block`, not `none`.
- **C. Links land on the document.** For each of `img`, `icons`, `tree` and `gh`: click it. The target heading's rect top ≈ `--bar-h` + `--content-pad` (±2 px), `document.querySelector(":target")` is that heading (and inside `#content`), and the tab's `index` went up by 1. For `tree`, the sidebar's `scrollTop` and `document.activeElement` are unchanged. `gh` exercises the fallback: `user-content-user-content-toast` misses, then the bare `user-content-toast` inside `#content` hits `## Toast`. `setup` lands on `## User content setup` directly, since its prefixed lookup `user-content-user-content-setup` is that heading's id.
- **D. Footnote:** `note` jumps to `#fn-n`, and its back-link returns to `#fnref-n`.
- **E. Cross-file:** from `ids-other.md`, `to tree` opens `ids.md` with `## Tree` at the top.
- **F. Back/Forward:** after C, Alt+Left steps back through each anchor entry to its recorded `scrollY` (±1 px), and Alt+Right forward again.
- **G. Dead links:** `dead` and `app` change neither `scrollY` nor `index`, and the sidebar is untouched.
- **I. Back to top:** scrolled down, `Back to top` (`#top`) and `empty` (`#`) each bring `scrollY` to 0, as on the parent commit's build. Neither throws: no console error, and in particular `CSS.escape("")` is never reached.
- **H. Nothing else moved:** `examples/kitchen-sink.md` shows its `## Image` section at normal size with the picture under it (screenshot). Its heading copy buttons still copy (click one, and the toast confirms). A Mermaid diagram there still opens its overlay.

- [x] **Step 7: README**

In the "With one exception: explicit anchor targets" note (~line 454), after the sentence ending "…so nothing in the source can break out of the attribute.", add:

```markdown
Those targets and every heading are given ids behind `user-content-`, as
GitHub does, so none can take on the app's own ids — `## Image` would
otherwise be styled as the image panel — and the page adds the prefix when a
link is followed, so `[x](#image)` and GitHub-style `#user-content-image`
links both land.
```

- [x] **Step 8: Commit**

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
git -C "F:/src/_ pet projects/t4-markdown-viewer" add src-tauri/src/render.rs src/app.js README.md
git -C "F:/src/_ pet projects/t4-markdown-viewer" commit -m "Stop a heading named like part of the app taking on its styles or its links"
```
The message ends with a blank line then `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

---

### Finish: close the item and file the plan

- Tick the open-items entry "A heading's id can collide with the app's own element ids". Add a line: `*Fixed <hash>, 2026-09-26: document ids render behind \`user-content-\`; the page adds it at the jump.*` Move the entry to `docs/closed-items.md` under a `## Deferred` heading, creating the heading if there is none.
- Tick this plan's boxes, add a short Outcome section (the commit and anything found), and move the plan to `docs/archive/plans/heading-id-collision.md`.
- Commit both: `docs: close the heading-id collision and file its plan`.
