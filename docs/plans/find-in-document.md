# Find in Document Implementation Plan

> **Status (2026-10-09):** draft, review passes 1-7 applied; every probe measured. Owner
> rulings recorded: D1-D6, A-E and G after pass 1, R1-R6 after pass 2, R7-R8, F and H after
> pass 3, R9 after pass 4 (each the recommended option; D5 refined, R7 noted). Not to be
> executed until the owner's go.

> **For agentic workers:** Execute per the **Execution** section below. Steps use checkbox
> (`- [ ]`) syntax for tracking. Read **Shared state** before any task: every task uses it.

**Goal:** Ctrl/Cmd+F finds text in the document on screen, on all three platforms. A small bar
takes the query, every match is highlighted, the current one stands out, and a count reads
"3 of 12". Next and previous move between matches, and Esc closes the bar.

**Architecture:** frontend only (`src/app.js`, `src/index.html`, `src/base.css`,
`src-tauri/themes/README.md`, `src-tauri/themes/_template.css`, `README.md`).
- An index of `els.content`'s searchable text is built once per change of the document: the
  raw text of its text nodes joined into one string, with `"\0"` at every block boundary, and
  each node's start offset in it.
- A search is a case-insensitive regular expression over that string. Its matches map back to
  `StaticRange`s by binary search over the node starts, so a match can cross element
  boundaries ("foo **bar**").
- Matches are painted with the CSS Custom Highlight API, without touching the DOM.
- One `MutationObserver` on `els.content` marks the index stale whenever the document changes
  while the bar is open.
- No Rust change, no new dependency.

**Tech Stack:** JavaScript (plain script, no bundler), CSS. Walks with the `drive-app` skill
(`.claude/skills/drive-app/SKILL.md`) on the Windows VM, the Linux VM and the Mac, never the
owner's PC.

**Spec:** the owner's request of 2026-10-09: "can we add a search text option?", narrowed to
find in the current document (not folder-wide search).

## Current behaviour (read 2026-10-09; line numbers as of 353efd6)

- There is no text search. Ctrl+Shift+F (`app.js:4266-4271`) focuses the sidebar's tree
  filter, which matches file names only.
- `onKeydown` (`app.js:4103`, bubbling on `document`, `app.js:4555`) doesn't handle Ctrl+F,
  F3 or Ctrl+G.
  - **Windows (measured, P2):** each of the three opens WebView2's own find bar today.
  - **macOS:** the app menu has no Find item (`main.rs:1592-1605`); Cmd+F's effect is measured
    by P2 on the Mac.
  - **Linux:** unmeasured; WebKitGTK has no built-in find bar.
- **Raw HTML never reaches the page:** `render.unsafe_` is off (`render.rs:296`), so there is
  no `<details>`, `<style>` or `<script>` in a document.
- The document is `<article id="content">` (`index.html:91`).
  - **Replaced in full** by `renderDocument`'s `innerHTML` (`app.js:1380`). Everything that
    shows another document goes through it: a tab switch, Back/Forward to another file, F5 and
    the live reload of a changed file (`refresh` → `showActive` → `renderDocument`).
  - **Changed in place** by:
    - a JSON fold toggling `hidden` on its body (`app.js:3962`);
    - a JSON "more" button splicing in the next chunk (`app.js:3986-3987`);
    - `redrawDiagrams` (`app.js:1078-1120`), which swaps figures and `<pre>`s, run on a theme
      change, F8, the diagram-colours setting, the OS dark-mode flip, and a click on a diagram
      drawn in an outdated look;
    - `themeProbe`'s brief append and remove (`app.js:642-650`), on every `diagramLook()`
      with diagrams on the page.
  - **Hidden, not emptied**, by `show()` (`app.js:1308-1312`) when a picture, error or empty
    screen takes over. The last document's DOM stays in place.
  - Same-document Back/Forward and anchor jumps only scroll.
- **Text that isn't the reader's text:**
  - `button.more` says "… 2.4 MB more" (`json.rs:630-636`);
  - a diagram figure (`div.mermaid-diagram`, `app.js:1029`) holds only mermaid's SVG, with its
    `<style>` text and actor popup menus hidden by `display="none"` (`app.js:999`);
  - copy-section buttons hold only an SVG and an aria-label, and heading anchors are empty.
- **Code blocks:** `highlight()` (`app.js:390-403`) splits only blocks that declare a language
  into highlight.js spans. A plain block's text sits directly in its `<code>`.
- **The scroll hold** (`holdScroll`, `app.js:1451`), started after a restore
  (`app.js:1426`), puts the saved position back at every height change for up to 60 s. Its
  scroll listener lets go only when the height is unchanged (`app.js:1511`), and its keydown
  stop skips inputs.
- **Modal dialogs** (Settings, the update dialog, the diagram dialog) own the keyboard
  (`app.js:4167-4173`). The guard returns without `preventDefault`. Only the reload keys are
  handled before it (`app.js:4153-4165`), with `ctrl && !event.altKey` against AltGr and a
  `keyCode` fallback for non-Latin layouts.
- **Themes** define only the `--ui-*` chrome colours (`_template.css:28-38`, among them
  `--ui-accent` and the optional `--ui-on-accent`, which `base.css:21` defaults).
  `base.css:4-7` keeps base colours out of `.markdown-body`.
- **Layout:**
  - `#bar` is the top bar (`index.html:12`; `user-select: none` with its `-webkit-` twin at
    `base.css:54-55`).
  - The window's minimum width is 420 px (`tauri.conf.json:15`), and the sidebar takes 16rem
    (`base.css:354`).
  - An element shown with `display: flex` needs its own `[hidden] { display: none; }` rule
    (`base.css:138-141`, `875-877`).
  - The toast sits at the bottom centre (`base.css:995-1001`).

## Decisions (ruled by the owner 2026-10-09: the recommended option in each)

From the first draft:

- **D1. Painting: the CSS Custom Highlight API.** No DOM change, so nothing that reads the
  document (copy section, task checkboxes, `data-sourcepos`, diagrams) sees a difference.
  Measured on WebView2, WebKitGTK 2.52.6, the AppImage's WebKitGTK 2.50.4 and the Mac's WebKit
  (P1). Engines without it: F.
- **D2. A slim bar under the top bar, at the right.** It takes no space in the bar's row.
- **D3. Visible text only.** Collapsed JSON folds, unloaded JSON chunks and diagrams are not
  searched. (The clause on `<details>` was dropped by R1.)
- **D4. Case-insensitive plain text.** Any run of whitespace in the query matches any run of
  whitespace in the text, so a phrase that wraps in the source still matches. No toggles.
- **D5. One bar per window.** While it's open, the query re-runs on whatever document is shown.
  Refined after pass 1: after the document changes, the count and highlights update, with no
  current match and no scroll, so the app's own scroll restore stands (R3 keeps the current
  match when the text didn't change). The next Enter or F3 picks from the screen as it is
  then. Typing still moves.
- **D6. Keys.**
  - Ctrl/Cmd+F opens the bar, or focuses and selects its text if it's already open.
  - Enter, F3 and Ctrl/Cmd+G go to the next match; Shift with any of them to the previous.
  - Esc closes the bar and gives focus back (only when it was in the bar, R7).
  - The browser defaults of Ctrl+F, F3 and Ctrl+G are always suppressed.

After review pass 1:

- **A. Colours:** two optional variables, `--find-match` and `--find-current`. `base.css`
  defaults them from `--ui-accent`; the current match's text uses `--ui-on-accent`. A theme
  may override them. Documented in `themes/README.md`. This is a written exception to
  `base.css`'s "no colours in `.markdown-body`" rule. Every theme's contrast is measured in the
  walks; a theme that fails gets its own values.
- **B. Block boundaries are hard breaks.** Nothing matches across a heading, paragraph, list
  item, table cell, `<br>` or other block boundary. Inline formatting (bold, links, code spans,
  highlight.js spans) joins.
- **C. On a picture, error or empty screen,** Ctrl+F opens the bar (or keeps it) showing "No
  matches", and keeps the query. It re-runs when a document is back.
- **D. F3 or Ctrl+G with the bar closed** opens it with this window's last query and moves to
  the next match (Shift: previous). With no query yet, it only opens the bar.
- **E. Diagrams are not searched.**
- **F. No fallback painting** (ruled after pass 3). Every engine measured has the API. Only a
  macOS 13 or 14 Mac that never took Safari 17.2+ lacks it (from knowledge, not measured). There
  the bar still counts, steps and scrolls to each match, with nothing painted. Closed accepted
  limit (Task 5).
- **G. Option+Left/Right on macOS** stays Back/Forward in the bar, as in the tree filter today.
  An open item is filed to revisit both fields together (Task 5).

After review pass 2:

- **R1. No `<details>` handling.** Raw HTML never reaches a page, so find never changes the DOM
  at all.
- **R2. On a narrow window the bar may cover the sidebar's header** while it's open (estimated
  370 px wide; overlap below about 640 px with the sidebar open). Accepted; Esc or × removes
  it. A screenshot at 420 px in the walk.
- **R3. A rebuild that leaves the text identical keeps the current match** (same start offset),
  repainted, with no scroll. That covers a theme switch or F8 with diagrams on the page, a click
  on an outdated diagram, and `themeProbe`. A real text change still follows D5.
- **R4. A partly loaded JSON says so:** while a `button.more` is in the page, the count gets
  " (loaded part)", with a `title` saying only the loaded part is searched and "… more" at the
  end loads the rest.
- **R5. App-written text in `#content` is searched:** a diagram's error line, a footnote's "↩"
  and a footnote number next to its word ("word1"; "word" still matches). Closed accepted limit,
  recorded in `docs/closed-items.md` (Task 5).
- **R6. On macOS, Control+F opens find too,** in text fields included, where it would move the
  caret one character. As Control+N, Control+W and Control+O already do. Closed accepted limit
  (Task 5).

After review pass 3:

- **H. The cap is 1 000** ("1,000+"). P3: 10 000 highlights paint once in ~0.06 / ~0.2 s on
  WebView2 (README ×30 / a JSON chunk), ~0.3 / ~0.9 s on WebKitGTK and ~0.35 / ~1.2 s on the
  Mac; a one-letter query hits the cap. A tenth of that is expected at 1 000 (reasoned: cost
  taken to grow with the count); the walk measures it.
- **R7. Esc closes the bar from anywhere** while it's open. Focus goes back only if it was in
  the bar, so Esc in the document doesn't move focus. One exception, accepted after pass 4: in
  the tree filter, Esc keeps clearing the filter (its own handler stops the key), and a second
  Esc closes the bar.
- **R8. A fully loaded big JSON re-indexes slowly:** with every chunk of a 9.75 MB JSON loaded
  (2.3 M text nodes), the index takes 0.8-1.9 s on WebKitGTK (P3b), so each fold or "more"
  freezes the window about 1 s while the bar is open. Open accepted limit, reopened on a report
  of find freezing on a large JSON (Task 5).

After review pass 4:

- **R9. Enter always moves.** If Enter arrives within 150 ms of a keystroke and the pending
  search keeps the current match, Enter then steps on from it, as it would after a pause.

## Global Constraints

- Frontend only; no new dependency, no Rust change, no new Tauri permission.
- Same behaviour on WebView2, WKWebView (macOS 13 floor) and WebKitGTK.
- Find never modifies the document's DOM.
- Typing stays responsive on a large document:
  - the index is built at most once per document change, never per keystroke;
  - a search runs 150 ms after the last keystroke;
  - matches are capped at `FIND_CAP`, 1 000 (H).
- Every theme: highlights readable in light and dark (A).
- The bar's text is never searched, and nothing outside `#content` is.

## Review Focus

1. **A huge document:** an ~8 MB JSON (all chunks loaded, P3b) and a long Markdown file.
   Typing never freezes the window; the cap and the debounce hold.
2. **A match that crosses formatting,** like "foo **bar**", a link's text, or a highlight.js
   span boundary. It is found and painted as one match. A match across two table cells or a
   heading and its paragraph is not found (B).
3. **The document changes while the bar is open:** a tab switch, F5, a live reload, a picture
   tab ("No matches", C), a JSON fold or "more", a theme switch on a page with diagrams (R3:
   current kept). The count is right afterwards, and no stale highlight is painted. Enter right
   after a change (inside the 150 ms) uses the new page.
4. **Keys:**
   - with the bar focused, Ctrl+W, Ctrl+Tab and Alt+Left still work, and Enter moves;
   - Esc closes the bar from the input, its buttons or the document (after any open tree, sort
     or open menu, or actor popup, each of which takes Esc first); focus moves back only from
     the bar (R7); Esc in the tree filter clears it first, and a second Esc closes the bar;
   - IME: the Enter that confirms CJK input doesn't move;
   - Ctrl+F with a dialog open shows no WebView2 find bar and doesn't open ours;
   - AltGr+F and AltGr+G (on Windows, `[` and `]` on a Hungarian layout) still type in the
     input and the tree filter;
   - Ctrl+Shift+F on a Cyrillic layout doesn't open find. (It doesn't open the tree filter
     either, today: that branch tests `key` only.)
5. **The scroll hold after a restore:** a jump to a match is never pulled back.
6. **A match far right in a wide code block or table** (with and without a language) scrolls
   into view.
7. **The narrowest window:** 420 px with the sidebar open. The bar is usable (R2).

## Probes (before execution)

- [x] **P1 — the Highlight API per engine:** `typeof CSS.highlights` and a painted test range.
  - **WebView2 (Windows VM, 1.7.4, 2026-10-09): supported.** `CSS.highlights` is an object,
    `Highlight` a function, `::highlight()` supported. A 5-character range painted exactly, and
    a range from paragraph text into the following `<code>` painted as one continuous run.
  - **WebKitGTK 2.52.6 (Linux VM, debug build of 353efd6, Wayland, 2026-10-09): supported.**
    The same three answers. Both ranges painted, including the one into `<code>`.
  - **The 1.7.4 AppImage's bundled WebKitGTK 2.50.4 (X11, 2026-10-09): supported.** The same
    three answers. Painting was seen with `WEBKIT_DISABLE_DMABUF_RENDERER=1`. With the default
    renderer, X11 captures are stale for every change, so painting couldn't be captured
    there.
  - **The Mac's WebKit (macOS 26.7.1, Safari 27.0, debug build of 353efd6, 2026-10-09):
    supported.** The same three answers; a 5-character range and a range from text into an
    `<a>` both painted. macOS 13 with Safari 17.2+ updates has the API; only a never-updated
    macOS 13 or 14 lacks it (from knowledge, not measured; F).
- [x] **P2 — Ctrl+F today on Windows.**
  - **Measured 2026-10-09 (Windows VM, real keystrokes).** Ctrl+F, F3 and Ctrl+G **each** open
    WebView2's own find bar today. A capture-phase keydown listener sees all three first, and
    `preventDefault` stops the bar for each.
  - **The Mac (2026-10-09, real keys):** Cmd+F, Cmd+G and Shift+Cmd+G reach the page with
    `metaKey`, not prevented, and do nothing visible. The app menu has no Find item.
- [x] **P1b — paint details, all three engines:**
  - does `var(--ui-accent)` resolve inside `::highlight()`;
  - does `priority` decide which of two overlapping highlights shows;
  - is a range across highlight.js spans painted.
  - **WebKitGTK 2.52.6 (Linux VM, Wayland, 2026-10-09): all three yes.**
    - `var(--ui-accent)` and `var(--ui-on-accent, #fff)` resolve and paint exactly (pixels
      #0078D4 and #FFFFFF).
    - The higher `priority` wins, and changing it repaints live. At equal priority the
      highlight registered later wins.
    - Ranges across highlight.js spans paint continuously.
  - **WebView2 (Windows VM, 1.7.4, 2026-10-09): all three yes**, by pixel histograms:
    `var()` resolves (#0078d4 background, #ffffff glyphs); the higher `priority` shows and
    repaints live; a range from an hljs span into the next text node paints continuously.
  - **The Mac (2026-10-09): all three yes.** The `var()` colour matches a control box painted
    with the same variable; priority wins and repaints live; hljs spans paint continuously.
- [x] **P3 — cost, all three engines:** the text walk, a search, building 10 000 ranges, their
  first paint, a scroll frame and the clear, on an ~8 MB JSON and README repeated 30 times.
  Sets `FIND_CAP` and tells whether the index build needs splitting across frames.
  - **WebKitGTK 2.52.6 (Linux VM, Wayland, 2026-10-09)**, cold / warm run, ms:

    | Page | text nodes | chars | walk | search | 10 000 ranges | paint | scroll frame |
    |---|---|---|---|---|---|---|---|
    | README ×30 (1 MB) | 36 450 | 1.0 M | 11 / 5 | 119 / 45 | 6 / 8 | 314 / 288 | 43 / 30 |
    | 9.75 MB JSON, first chunk | 125 326 | 0.5 M | 337 / 23 | 9 / 4 | 11 / 6 | 1001 / 877 | 30 / 88 |

    - The JSON page holds only its first 512 KB chunk until "more" is pressed, so the page's
      text is bounded per chunk, not by the file.
    - The first paint of 10 000 highlights is the only large cost: ~0.3 s on README ×30, ~0.9 s
      on the JSON chunk. Registration and paint weren't separated. Scrolling afterwards stays
      normal (30-88 ms frames), and the page stayed responsive.
    - The walk and the search are cheap enough to run in one go; no splitting needed.
  - **WebView2 (Windows VM, 1.7.4, 2026-10-09)**, same pages, cold / warm, ms:

    | Page | walk | search | 10 000 ranges | paint | scroll frame |
    |---|---|---|---|---|---|
    | README ×30 | 3 / 2 | 3 / 7 | 3 / 2 | 61 / 54 | 45 / 40 |
    | 9.75 MB JSON, first chunk | 27 / 13 | 4 / 4 | 12 / 7 | 253 / 187 | 25 / 19 |

    - With 10 000 highlights left on, an `eval` round trip took 1 ms and a 2000 px scroll
      13 ms.
  - **The Mac (macOS 26.7.1, 2026-10-09)**, same pages, cold / warm, ms:

    | Page | walk | search | 10 000 ranges | paint | scroll frame |
    |---|---|---|---|---|---|
    | README ×30 | 3 / 2 | 5 / 5 | 3 / 3 | 332 / 407 | 116 / 299 |
    | 9.75 MB JSON, first chunk | 6 / 4 | 2 / 2 | 4 / 5 | 1155 / 1285 | 232 / 34 |

    - Responsive afterwards (an `eval` round trip 85-91 ms); clearing took ~33 ms.
  - `new Highlight(staticRange)` paints on all three: P3 painted `StaticRange`s, and its
    screenshots show them.
- [x] **P3b — the JSON with every chunk loaded** (Linux VM, the slowest engine): the walk with
  the plan's filter, a phrase search and a one-letter search.
  - **WebKitGTK 2.52.6 (Linux VM, Wayland, 2026-10-09):** 18 "more" clicks loaded the whole
    9.75 MB file: 2 280 002 text nodes, 9.7 M characters.
    - The walk: 1871 ms cold, 813 ms warm.
    - The searches: 1-3 ms each to 10 000 matches (the cap before H), a phrase and one letter
      alike.
    - The window stayed responsive (an `eval` round trip 119 ms; scrolls 19-32 ms after the
      first full layout, which took 4.4 s on its own, without find).
    - So with find open on a fully loaded big JSON, each fold toggle or "more" costs a ~1 s
      rebuild on Linux (WebView2 was ~10× faster on the walk in P3; not measured here).
      Loading such a file took the prober minutes of clicks. Ruled: R8.

## Shared state and functions

Each task's implementer sees one task; these names are fixed here so the tasks fit together.
Task 1 declares all the state, next to the other module-level `let`s in `app.js`:

```js
// Find in document (docs/plans/find-in-document.md).
const FIND_CAP = 1000; // ruling H: 10 000 took up to 1.3 s to paint (P3)
let findIndex = null; // { text, starts, nodes } of #content, or null while the bar is closed
let findStale = false; // #content changed since findIndex was built
let findHits = []; // StaticRange[], in document order
let findHitStarts = []; // each hit's start offset in findIndex.text
let findMore = false; // the search stopped at FIND_CAP
let findCurrent = -1; // index into findHits, or -1 for none
let findTimer = 0; // the typing debounce
let findRefreshTimer = 0; // the mutation debounce
let findObserver = null; // watches #content while the bar is open
let findReturn = null; // what had focus before the bar opened
let scrollHold = null; // the running holdScroll's AbortController
```

| Function | Task | Does |
|---|---|---|
| `buildFindIndex(root)` | 1 | `{ text, starts, nodes }` |
| `findMatches(index, query, cap)` | 1 | `{ ranges, hitStarts, more }` |
| `openFind()`, `closeFind()` | 2 | show or hide the bar, focus |
| `findAgain(step)` | 2 | F3 / Ctrl+G: open if closed, then `findStep` if there is a query |
| `runFind(mode, step)` | 3 (stub in 2) | search, paint, count; `mode` is `"type"` or `"mutate"` |
| `findStep(step)` | 3 (stub in 2) | next (1) or previous (−1) match |
| `paintFind()`, `paintCurrent()`, `showFindCount()`, `scrollToHit(range)` | 3 | |
| `freshFindIndex()` | 3 | rebuilds the index if missing or stale; true if the text changed |
| `firstShownHit(step)` | 3 | index of the first hit on screen (the last, for −1) |
| `findMutated()` | 4 | the observer's callback |

## Tasks

### Task 1: the index and the matcher

**Files:** `src/app.js`: the shared state above, and a new section beside the other document
helpers.

```js
// Block boundaries: no match crosses one (ruling B).
const FIND_BLOCKS = new Set(["P", "LI", "TD", "TH", "DT", "DD", "H1", "H2", "H3", "H4", "H5",
  "H6", "PRE", "BLOCKQUOTE", "DIV", "TABLE", "TR", "UL", "OL", "DL", "FIGURE", "FIGCAPTION", "HR",
  "BR", "SECTION"]);

/*
 * The searchable text of `root`: its text nodes joined, with "\0" where a block starts so no
 * match crosses one. "\0" is safe: the HTML parser drops it from text, and `\s` doesn't match
 * it. Buttons (the JSON "… more", copy section), SVG (diagrams, icons) and anything `hidden`
 * (a folded JSON body) are left out. Tags and attributes only: computed style is too slow on a
 * JSON page, and `checkVisibility` needs Safari 17.4.
 */
function buildFindIndex(root) {
  const parts = [], starts = [], nodes = [];
  let length = 0;
  if (!root.hidden) {
    const skip = (n) =>
      n.nodeType === 1 && (n.tagName === "BUTTON" || n.tagName === "svg" || n.hasAttribute("hidden"))
        ? NodeFilter.FILTER_REJECT
        : NodeFilter.FILTER_ACCEPT;
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, skip);
    for (let n; (n = walker.nextNode()); ) {
      if (n.nodeType === 1) {
        if (FIND_BLOCKS.has(n.tagName)) parts.push("\0"), length++;
      } else if (n.data) {
        starts.push(length);
        nodes.push(n);
        parts.push(n.data);
        length += n.data.length;
      }
    }
  }
  return { text: parts.join(""), starts, nodes };
}

/** The query as a pattern: case-insensitive, any whitespace run matching any other (D4). */
function findPattern(query) {
  // Not just whitespace or NULs, which would leave an empty pattern that matches nothing
  // forever.
  if (!/[^\s\0]/.test(query)) return null;
  const literal = (s) => s.replace(/[.*+?^${}()|[\]\\/]/g, "\\$&");
  return new RegExp(query.replaceAll("\0", "").split(/\s+/).map(literal).join("\\s+"), "giu");
}

/** Up to `cap` matches of `query` in `index`, as ranges in document order. */
function findMatches(index, query, cap) {
  const ranges = [], hitStarts = [];
  const re = findPattern(query);
  if (!re) return { ranges, hitStarts, more: false };
  // The node holding offset `i` of the text: the last whose start is at or before it.
  const nodeAt = (i) => {
    let lo = 0, hi = index.starts.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (index.starts[mid] <= i) lo = mid;
      else hi = mid - 1;
    }
    return lo;
  };
  for (let m; (m = re.exec(index.text)); ) {
    if (ranges.length === cap) return { ranges, hitStarts, more: true };
    const a = nodeAt(m.index), b = nodeAt(m.index + m[0].length - 1);
    ranges.push(new StaticRange({
      startContainer: index.nodes[a], startOffset: m.index - index.starts[a],
      endContainer: index.nodes[b], endOffset: m.index + m[0].length - index.starts[b],
    }));
    hitStarts.push(m.index);
  }
  return { ranges, hitStarts, more: false };
}
```

Notes for the implementer:

- `toLowerCase` isn't used: the `iu` flags fold case, astral scripts and ς/σ included, and
  offsets stay the raw text's, so "İstanbul" needs no special mapping.
- A match never contains `"\0"` (the query can't, and `\s` doesn't match it), so its first and
  last characters are both inside text nodes.
- Text after a nested block inside the same parent (a list item's text after its sub-list)
  would join the sub-list's last text. comrak puts only whitespace there; accepted.

- [ ] **Step 1:** add the state and the three functions.
- [ ] **Step 2: check.** `node --check src/app.js`. Build the audit app (drive-app skill) on
  the Windows VM and `eval` against fixtures written into a detached `div`. The expected
  results:

  | Fixture (HTML) | Query | Expected |
  |---|---|---|
  | `<p>foo <strong>bar</strong></p>` | `foo bar` | 1 match; a live range from it reads `"foo bar"` |
  | `<h2>Setup</h2><p>Install</p>` | `setup install` | 0 |
  | `<table><tr><td>a</td><td>b</td></tr></table>` | `ab` | 0 |
  | `<p>İstanbul</p>` | `stan` | 1, reading `"stan"` |
  | `<p>a  \n  b</p>` | `a b` | 1 |
  | `<p>a<br>b</p>` | `a b` | 0 |
  | `<p>x</p><button>2.4 MB more</button>` | `more` | 0 |
  | `<div hidden><p>gone</p></div>` | `gone` | 0 |
  | `<p>a.b</p><p>axb</p>` | `a.b` | 1 (no regex meaning) |
  | `<p>` + `"e".repeat(2000)` + `</p>` | `e` | 1 000 ranges, `more: true` |
  | anything | `"   "` | 0, `more: false` |
  | anything | `"\0\0"` | 0, `more: false`, returns at once |

  Also time `buildFindIndex(els.content)` and `findMatches(…, "e", FIND_CAP)` on README ×30
  and report them.

  The fixtures are built with `document.createElement("div")` and `innerHTML`, never
  `els.content`.
- [ ] **Step 3: commit:** "Find: index and matcher" (squashed later into the feature commit).

### Task 2: the bar and its keys

**Files:** `src/index.html`, `src/base.css`, `src/app.js` (`els`, `onKeydown`, the bar's
listeners).

**Consumes:** the shared state (Task 1). **Adds stubs** for Task 3, so this task runs on its
own: `function runFind(mode) {}` and `function findStep(step) {}`.

**Markup:** the last child of `#bar` (`index.html:12`), after `#tabs` (`index.html:67`). The
buttons follow the bar's `class="nav"` buttons with inline SVG (`index.html:15-20`):

```html
<div id="find" role="search" hidden>
  <input id="find-input" type="text" aria-label="Find in document" placeholder="Find"
         spellcheck="false" autocomplete="off" />
  <span id="find-count" role="status"></span>
  <button id="find-prev" class="nav" type="button" aria-label="Previous match"
          title="Previous match (Shift+Enter)"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 10 L8 5 L13 10" /></svg></button>
  <button id="find-next" class="nav" type="button" aria-label="Next match"
          title="Next match (Enter)"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 6 L8 11 L13 6" /></svg></button>
  <button id="find-close" class="nav" type="button" aria-label="Close find"
          title="Close (Esc)"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4 L12 12 M12 4 L4 12" /></svg></button>
</div>
```

- `type="text"`, not `search`: Chromium clears a search field on Escape.

**CSS (`base.css`, beside the `#bar` rules):**

```css
/* Find in document: hangs under the bar at the right, taking no room in it. #bar is sticky,
   so it is the containing block. Over the sidebar's header on a narrow window (ruling R2). */
#find {
  position: absolute;
  top: 100%;
  right: 1rem;
  max-width: calc(100vw - 2rem);
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.375rem;
  background: var(--ui-bg);
  color: var(--ui-fg);
  border: 1px solid var(--ui-border);
  border-top: 0;
  border-radius: 0 0 6px 6px;
}
#find[hidden] {
  display: none;
}
#find-input {
  flex: 1 1 12rem;
  min-width: 0;
  -webkit-user-select: text;
  user-select: text;
}
#find-count {
  min-width: 7ch;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  color: var(--ui-fg-muted);
}
```

The input takes the tree filter's look, but not its layout (`#tree-filter`'s `display: block`,
`width` and `margin`, `base.css:455-467`, would break the flex row). Add to `#find-input`:
`appearance: none; padding: 0.125rem 0.25rem; font: inherit; color: inherit; background:
transparent; border: 1px solid var(--ui-border); border-radius: 3px;`. `font: inherit` matters:
inputs don't inherit the font. Then copy the filter's `::placeholder` and `:focus-visible` rules
(`base.css:470-477`) for `#find-input`.

**Keys in `onKeydown`:**

1. **Suppress,** right after the reload block (`app.js:4153-4165`), before the modal guard. It
   doesn't return, so under a dialog the guard still swallows the key, and otherwise the
   branches below act on it:

   ```js
   // WebView2 opens its own find bar on each of these (measured). The modifier is checked
   // as the reload's is: AltGr arrives as Ctrl+Alt, and AltGr+F types "[" on a Hungarian
   // layout. `keyCode` for non-Latin layouts, as above.
   // `key` can be missing (an autofill keydown), hence the `?.`.
   const k = event.key?.toLowerCase();
   if (
     event.key === "F3" ||
     event.keyCode === 114 ||
     (ctrl && !event.altKey && (k === "f" || k === "g" || event.keyCode === 70 || event.keyCode === 71))
   )
     event.preventDefault();
   ```

2. **Escape,** right after `if (event.key === "Escape" && closeActorMenus()) return;`
   (`app.js:4181`). It works from anywhere while the bar is open (R7):

   ```js
   if (event.key === "Escape" && !els.find.hidden && !event.isComposing && event.keyCode !== 229) {
     closeFind();
     return;
   }
   ```

3. **F3,** just before `if (!ctrl || event.altKey) return;`:

   ```js
   if (event.key === "F3" || event.keyCode === 114) {
     findAgain(event.shiftKey ? -1 : 1);
     return;
   }
   ```

4. **Ctrl/Cmd+F and +G,** in the key switch, right after the `key === "f" && event.shiftKey`
   branch:

   ```js
   } else if ((key === "f" || event.keyCode === 70) && !event.shiftKey) {
     openFind();
   } else if (key === "g" || event.keyCode === 71) {
     findAgain(event.shiftKey ? -1 : 1);
   ```

   `!event.shiftKey`: on a Cyrillic layout Ctrl+Shift+А has `keyCode` 70 but `key` "А", and
   must not open find.

**The bar's own listeners,** in `main()` beside the tree filter's (`app.js:4349`):

```js
els.findInput.addEventListener("keydown", (event) => {
  // The Enter that confirms IME input; WebKit may send it with isComposing false.
  if (event.isComposing || event.keyCode === 229) return;
  if (event.key === "Enter") {
    event.preventDefault();
    event.stopPropagation();
    findStep(event.shiftKey ? -1 : 1);
  }
});
els.findPrev.addEventListener("click", () => findStep(-1));
els.findNext.addEventListener("click", () => findStep(1));
els.findClose.addEventListener("click", closeFind);
```

**Opening and closing:**

```js
function openFind() {
  if (els.find.hidden) {
    // An item of the open-menu is about to be hidden with it; go back to its button instead.
    findReturn = els.openMenu.contains(document.activeElement) ? els.openMore : document.activeElement;
    els.find.hidden = false;
    runFind("mutate"); // a kept query recounts on this page, without moving
  }
  showOpenMenu(false);
  els.findInput.focus();
  els.findInput.select();
}

function closeFind() {
  // Esc works from anywhere (R7); only focus that was in the bar is moved.
  const inBar = els.find.contains(document.activeElement);
  els.find.hidden = true;
  clearTimeout(findTimer);
  clearTimeout(findRefreshTimer);
  findTimer = findRefreshTimer = 0;
  findObserver?.disconnect();
  CSS.highlights?.delete("find");
  CSS.highlights?.delete("find-current");
  findIndex = null;
  findHits = [];
  findHitStarts = [];
  findCurrent = -1;
  // The query stays in the input, for F3 and Ctrl+G (ruling D).
  if (inBar) {
    els.findInput.blur();
    // Without scrolling: the reader stays at the match they walked to.
    if (findReturn?.isConnected && findReturn.getClientRects().length)
      findReturn.focus({ preventScroll: true });
  }
  findReturn = null;
}

function findAgain(step) {
  if (els.find.hidden) openFind();
  if (/[^\s\0]/.test(els.findInput.value)) findStep(step);
}
```

- [ ] **Step 1:** markup, CSS, `els` entries (`find`, `findInput`, `findCount`, `findPrev`,
  `findNext`, `findClose`), the keys, the listeners, `openFind`, `closeFind`, `findAgain`, and
  the two stubs.
- [ ] **Step 2: check:** `node --check src/app.js`. On the Windows VM, with real keystrokes
  (`sendkeys.ps1`, after checking `document.hasFocus()`):
  - Ctrl+F opens our bar and no WebView2 bar; F3 and Ctrl+G with the bar closed open ours;
  - with Settings open, Ctrl+F, F3 and Ctrl+G open neither bar;
  - Esc from the input and from the close button closes it, and focus goes back, with no
    scroll; Esc after clicking into the document closes it and leaves focus there;
  - Ctrl+W in the input closes the tab;
  - Ctrl+Shift+F still focuses the tree filter;
  - at 420 px with the sidebar open, a screenshot (R2).
- [ ] **Step 3: commit:** "Find: the bar and its keys".

### Task 3: searching, painting and moving

**Files:** `src/app.js`, `src/base.css`, `src-tauri/themes/README.md`,
`src-tauri/themes/_template.css`.

**Consumes:** Tasks 1 and 2. Replaces the two stubs.

**CSS (`base.css`),** right after the `:root { … }` block (`base.css:14-29`):

```css
/*
 * Find's highlights: the other exception to the rule at the top. They paint
 * inside .markdown-body, so they take the theme's accent, which every theme
 * sets. A theme may set --find-match and --find-current itself.
 */
:root {
  --find-match: color-mix(in srgb, var(--ui-accent) 35%, transparent);
  --find-current: var(--ui-accent);
}
::highlight(find) {
  background-color: var(--find-match);
}
::highlight(find-current) {
  background-color: var(--find-current);
  color: var(--ui-on-accent);
}
```

- `var()` resolves inside `::highlight()`: measured on all three (P1b).
- The header (`base.css:9-11`): "is the one exception" becomes "is one exception", and one
  more sentence follows its last: "Find's highlights are the other: they paint in the
  document, from `--ui-accent` (see `::highlight(find)`)."
- `themes/README.md`: add `--find-match` and `--find-current` as optional, in the `--ui-*`
  block (`README.md:85-95`), with their defaults. Its "Four more are optional" (line 85)
  becomes six.
- `_template.css`: after `--ui-error` (line 38), in the same commented style:

  ```css
    /* --find-match: rgba(0, 120, 212, 0.35); */ /* every find match */
    /* --find-current: #0078d4; */ /* the current find match; its text is --ui-on-accent */
  ```

**The scroll hold (`holdScroll`, `app.js:1451`):** after `const hold = new AbortController();`
add the following, so a jump to a match can let go of it:

```js
scrollHold = hold;
hold.signal.addEventListener("abort", () => {
  if (scrollHold === hold) scrollHold = null; // a newer hold may have taken its place
});
```

**The functions:**

```js
/** Rebuilds the index if it is missing or stale; true when the searchable text changed. */
function freshFindIndex() {
  if (findIndex && !findStale) return false;
  const old = findIndex?.text;
  findIndex = buildFindIndex(els.content);
  findStale = false;
  return findIndex.text !== old;
}

/*
 * Search the page for the query. "type": the query changed, so keep the current match if it
 * still starts in the same place, else take the first one on screen (the last, for `step` −1),
 * and bring it into view. "mutate": the page changed under the query, so only recount and
 * repaint (D5); the app's own scroll restore stands. Either way the current match is kept
 * only while the text is the same (R3): after a change its offset means something else.
 * Returns whether the current match was kept.
 */
function runFind(mode, step = 1) {
  const changed = freshFindIndex();
  const keep = findCurrent >= 0 && !changed ? findHitStarts[findCurrent] : -1;
  ({ ranges: findHits, hitStarts: findHitStarts, more: findMore } =
    findMatches(findIndex, els.findInput.value, FIND_CAP));
  findCurrent = keep < 0 ? -1 : findHitStarts.indexOf(keep);
  const kept = findCurrent >= 0;
  if (mode === "type" && !kept && findHits.length) findCurrent = firstShownHit(step);
  paintFind();
  showFindCount();
  if (mode === "type" && findCurrent >= 0) scrollToHit(findHits[findCurrent]);
  return kept;
}

function findStep(step) {
  // Enter straight after typing: the pending search is the move, unless it kept the current
  // match; then step on from it, as Enter would after a pause.
  if (findTimer) {
    clearTimeout(findTimer);
    findTimer = 0;
    if (!runFind("type", step)) return;
  }
  if (findStale) runFind("mutate"); // the page changed inside the last 150 ms
  if (!findHits.length) return;
  findCurrent =
    findCurrent < 0 ? firstShownHit(step) : (findCurrent + step + findHits.length) % findHits.length;
  paintCurrent();
  showFindCount();
  scrollToHit(findHits[findCurrent]);
}
```

- **`firstShownHit(step)`:** the first hit whose rect's bottom is below the find bar's bottom
  (`els.find.getBoundingClientRect().bottom`), for `step` 1, else 0. For `step` −1: the last hit
  whose top is above `innerHeight`, else the last. Binary search over `findHits`, which are in
  document order, so only a few rects are read.
- **A hit's rect:** a `StaticRange` has none, so make a live `Range` from it (`setStart`,
  `setEnd`) each time one is needed. It is safe: `findStale` is set synchronously by the
  observer (Task 4) in the microtask after a change, before any key or timer can run, and
  every path through `runFind` or `findStep` rebuilds first, so a hit never points at a
  removed node.
- **`scrollToHit(range)`:**
  1. If the hit is inside `pre` or `.table-scroll` (closest to `startContainer.parentElement`)
     and its rect lies outside that box's rect horizontally, set the box's `scrollLeft` to
     centre it. A plain code block's text sits directly in a wide `<code>`, so
     `scrollIntoView` on the parent wouldn't move it.
  2. Read the rect again. If its top is above the find bar's bottom, or its bottom below
     `innerHeight`: abort a running scroll hold (`scrollHold?.abort()`; it watches only the
     window, so step 1 doesn't need this), then
     `window.scrollBy(0, rect.top - (findBottom + innerHeight) / 2)`. A hit already on screen
     doesn't move the page.
- **`paintFind()`:** if `CSS.highlights` is missing, return (F). Else build `find` from every
  hit with `.add` in a loop (JavaScriptCore caps spread arguments near 65 000), then
  `paintCurrent()`. With no hits, `find` is set to an empty `Highlight`.
- **`paintCurrent()`:** if `CSS.highlights` is missing, return (F; `findStep` calls it
  directly). Else `find-current` gets the current hit alone, with `priority = 1`, or
  nothing when `findCurrent` is −1. It paints over `find` (measured on all three, P1b). `find`
  isn't rebuilt when only the current hit moves.
- **`showFindCount()`:** numbers through `toLocaleString()`:
  - an empty or whitespace-only query (`!/[^\s\0]/.test(value)`, as `findPattern`): "";
  - no hits: "No matches" (on a picture or error screen too: C);
  - a current hit: "3 of 12";
  - none current: "1 match" or "12 matches";
  - `findMore`: the total reads "1,000+" (in an English locale);
  - R4: while `!els.content.hidden && els.content.querySelector("button.more")`, append
    " (loaded part)" and set the count's `title` to "Only the loaded part of this file is
    searched. Load the rest with '… more' at its end." Clear the `title` otherwise.
- **Typing:** in `main()`, beside the bar's listeners (Task 2), the input's `input` event
  restarts a 150 ms `findTimer`, which resets `findTimer` to 0 and calls `runFind("type")`.

- [ ] **Step 1:** implement the above.
- [ ] **Step 2: check:** on the Windows VM, README:
  - "the": the count matches a hand count on a one-paragraph fixture;
  - Enter and Shift+Enter walk and wrap; F3 and Ctrl+G likewise;
  - Enter within 150 ms of typing lands on the first match on screen, not the second;
    Shift+Enter there lands on the last one on screen;
  - R9: on "foo" at match B, typing "d" (B still matches) and Enter within 150 ms goes to C,
    and Shift+Enter to A, as after a pause;
  - one letter on README ×30 and on the big JSON's first chunk: time from the keystroke to the
    paint, on each platform (H expects about a tenth of P3's);
  - a match far right in a wide plain code block, and in a wide table, scrolls them across;
  - a screenshot of the current match among the others, in a light and a dark theme.
- [ ] **Step 3: commit:** "Find: search, highlight and move between matches".

### Task 4: following the document

**Files:** `src/app.js` (`openFind` and a new `findMutated`).

```js
/*
 * #content changed while the bar is open: a new document, a reload, a JSON fold or "more", a
 * diagram redrawn, a picture or error screen hiding it. Mark the index stale at once (this runs
 * as a microtask, before the next key), and recount once things settle.
 */
function findMutated() {
  findStale = true;
  clearTimeout(findRefreshTimer);
  findRefreshTimer = setTimeout(() => {
    findRefreshTimer = 0;
    runFind("mutate");
  }, 150);
}
```

- In `openFind`, before `runFind`:
  `(findObserver ??= new MutationObserver(findMutated)).observe(els.content, { childList: true,
  subtree: true, attributes: true, attributeFilter: ["hidden"] });`
  - `hidden` on `#content` itself is reported, so `show()` hiding it for a picture is caught,
    and the index then comes out empty ("No matches", C).
  - `characterData` isn't needed: nothing edits a text node in place.
  - Nothing find does mutates `#content`, so it can't trigger itself.
  - `themeProbe` and a diagram redraw trigger it; R3 keeps the current match through them.
- `closeFind` already disconnects it (Task 2).

- [ ] **Step 1:** implement the above.
- [ ] **Step 2: check:** on the Windows VM, with the bar open on "the" and a current match:
  - another tab, then back: the count follows, with no current match and no scroll;
  - F5, and an edit to the file on disk (live reload);
  - a picture tab: "No matches"; back: recounted;
  - F8 on a page with a diagram: the current match is kept;
  - a large JSON: fold a block holding a match (the count drops), then "more" (it grows, with
    "(loaded part)" until the last chunk);
  - a session restore with the hold running: Enter jumps and stays.
- [ ] **Step 3: commit:** "Find: follow the document as it changes".

### Task 5: README and records

- README shortcut table, above `Ctrl+Shift+F` (`README.md:232`; the "Cmd for Ctrl" line at
  `README.md:225` covers the Mac):
  - `Ctrl+F` | Find in the document
  - `Enter` / `F3` / `Ctrl+G` (Shift for previous) | Next / previous match
- `docs/open-items.md` › Deferred (ruling G): "On macOS, Option+Left/Right in the tree filter
  and the find bar goes Back/Forward, not a word." Reopen on a Mac user's report, or with the
  next keyboard work.
- `docs/closed-items.md` › Accepted limits: R5 (app-written text is searched), R6 (Control+F
  on macOS) and F (no highlights on a Mac without Safari 17.2+), each "*Kept 2026-10-09,
  owner's call, no trigger.*"
- `docs/open-items.md` › Accepted limits: R8 (a fully loaded big JSON re-indexes in about 1 s
  on each fold or "more" while find is open). "*Accepted 2026-10-09. Reopen on a report of find
  freezing on a large JSON.*"
- Skipped findings from the change review go the usual way (CLAUDE.md step 6).
- [ ] **Commit:** the README row rides with the feature; the records are a `docs:` commit.

### Task 6: walks on all three platforms

Via the Windows VM, the Linux VM and the Mac (the drive-app skill; never the owner's PC):

- each Review Focus item;
- every theme's highlight contrast: loop `applyTheme` over `state.themes` (objects: use
  `t.name`), read the computed `--find-match`, `--find-current` and `--ui-on-accent` against the
  page background, and screenshot the darkest and lightest themes;
- Windows: Ctrl+F, F3 and Ctrl+G show only our bar; AltGr+F on a Hungarian layout types `[`;
- the Mac: Cmd+F, Cmd+G, Option+Arrow in the input (G);
- Linux: Wayland and X11.

## Execution

Main session orchestrates. Tasks 1-4 go to a `coder` subagent (Opus) one at a time, each
committed on main after `node --check` and `cargo test`. Task 5 is the main session's (small
doc edits). Walks run on the Windows VM, the Linux VM and the Mac, then the change review loop
(CLAUDE.md step 5). At packaging the commits are squashed into one feature commit, "Find text
in the document (Ctrl/Cmd+F)", plus `docs:` records.

## Change review fixes (triage 2026-10-10)

From the walks of e3cde03 on all three platforms and the owner's
triage. Already done and walked: the WebKit stale-paint fix (`b9989c9`; Linux and the Mac
passed). Recorded: the Ctrl+Tab-then-Enter race and the past-the-cap jump (closed accepted
limits), real IME and Cyrillic unwalked (open accepted limit). The diagram frame failure the
Windows walk found has its own plan, `plans/diagram-frame-isolation.md`.

### Rulings

- **T1. Solarized's matches** (text on a match 2.77 light / 2.89 dark, measured on all three;
  every other theme 5.09 or better). The owner's standing rule keeps upstream palettes faithful
  (`closed-items.md`, the 2026-09-11 contrast audit), and an app-invented colour is set per theme
  only where its default fails. Ruled: **the app picks the tint per theme**: the strongest
  accent tint up to today's 35% that keeps the theme's text at 3:1 or better on a match (the
  3:1 bar the owner set for diagram category labels on themes whose own text is under 5:1). No
  theme file changes; a theme that sets its own `--find-match` is left alone.
- **T2. Huge documents** (a fully loaded 9.75 MB JSON: re-indexing took 0.8-1.9 s per fold or
  "more" in probe P3b, 8-21 s on a memory-starved Linux VM). Ruled: **change find's design for
  huge documents**: above a size, a change inside the document no longer re-indexes on its own.
- **T3. The count touches ▲ at 420 px** (all three). Ruled: fix.
- **T4. Enter right after typing centres a match half under the bar** (Linux: a 390 px jump).
  Ruled: fix: a match counts as on screen only when it sits wholly below the bar.

### Decisions (ruled by the owner 2026-10-10: the proposed option in each)

- **Q1 (T2). The size:** an index of more than 250 000 text nodes. Measured sizes:
  README ×30 is 35 000 (index 8-39 ms), the big JSON's first chunk 116 000-137 000 (59-63 ms),
  the whole JSON 2.3-2.5 M (0.8 s to 26 s).
- **Q2 (T2). What the reader sees** once a change isn't followed: the count keeps
  its numbers and adds " (changed)", with the title "The page changed. Press Enter or type to
  count again."; Enter or typing re-indexes (one wait the reader asked for).
- **Q3 (T2). A new document** (a tab switch, a reload) always re-indexes, whatever the size of
  the old one; only changes deeper inside the page (a fold, "more", a diagram redrawn) wait.
- **Q4. The first-letter freeze** (the Mac: typing the first letter into the empty field, or
  clearing it, blocks ~0.6 s on the fully loaded JSON before find's code runs; cause
  unmeasured, `:has()` rules suspected): a probe on the Mac first (does the tree
  filter do the same; does it stop with the `:has()` rules removed in the page), plus a quick
  check whether Windows and Linux freeze at all, then a ruling on what it shows.
- **Q5. The JSON viewer's memory** (Windows: 400-500 MB per 512 KB chunk with find open or
  closed; 17 chunks reached 9.7 GB). Not find's: a deferred open item with a trigger, recorded
  with these fixes.

### Task 7: the per-theme match tint (T1)

**Files:** `src/app.js`.

- Lift `bytes`, `lum` and `contrast` out of `diagramLook` to module level, so both use them.
  `pixel` (the 1x1 canvas) is declared at module level but created inside `diagramLook`
  (`pixel ??= …`); that line moves into `bytes`, so either caller can be first.
- A new `findTint()` called from `applyTheme` right after `applyDocFont()`:

  ```js
  /*
   * Find's match tint: the theme's accent, as strong as it can be up to 35%
   * while the theme's text keeps 3:1 on it. A theme that sets --find-match
   * keeps its own.
   */
  function findTint() {
    const root = document.documentElement;
    root.style.removeProperty("--find-match");
    if (els.themeStyle.textContent.includes("--find-match")) return;
    const page = getComputedStyle(document.body).backgroundColor;
    const bg = bytes(page, "#fff");
    const text = bytes(getComputedStyle(els.content).color, page);
    const accent = bytes(getComputedStyle(root).getPropertyValue("--ui-accent"), page);
    for (let p = 35; p >= 10; p -= 5) {
      const tint = accent.map((v, i) => Math.round((v * p + bg[i] * (100 - p)) / 100));
      if (contrast(text, tint) >= 3) {
        if (p < 35) root.style.setProperty("--find-match", `color-mix(in srgb, var(--ui-accent) ${p}%, transparent)`);
        return;
      }
    }
    root.style.setProperty("--find-match", "color-mix(in srgb, var(--ui-accent) 10%, transparent)");
  }
  ```

  A theme's own value is found in its CSS text (`els.themeStyle` holds the loaded theme,
  bundled or the user's), not by comparing computed values: engines serialise a custom
  property's value differently. A theme that only mentions the name in a comment loses the
  adjustment, which is harmless (it keeps 35%).
- Expected (computed from the theme colours with findTint's rounding, not measured):
  Solarized light 25% (3.10), Solarized dark 30% (3.12; the first step to clear 3:1); every
  other theme stays 35%.
- Walk: Solarized light reads `--find-match` at 25% and dark at 30%, text on a match ≥ 3:1; every other
  theme unchanged; a user theme with its own `--find-match` keeps it; switching away from
  Solarized puts 35% back.

### Task 8: huge documents (T2)

**Files:** `src/app.js`.

- `findMutated(records)`: the observer passes its records. If any record is a `childList`
  change whose target is `els.content` itself (a new document) or the `hidden` attribute of
  `els.content`, or the last index has no more than `FIND_DEFER_NODES` (250 000, Q1) text
  nodes, keep
  today's behaviour. Otherwise set `findStale = true` and `findDeferred = true`, and don't
  start the timer.
- `showFindCount`: with `findDeferred`, append " (changed)" and the Q2 title.
- `runFind` clears `findDeferred` (it rebuilds through `freshFindIndex`). `findStep` already
  runs `runFind("mutate")` when stale, so Enter re-indexes; typing does too.
- `closeFind` clears `findDeferred`.
- Walk (Windows and Linux): the fully loaded JSON; fold and "more" with the bar open: no
  freeze, " (changed)" shows, Enter re-indexes once and counts right; a tab switch away from
  it re-indexes the new page at once; README ×30 behaves as before.

### Task 9: the 420 px count and the half-hidden match (T3, T4)

**Files:** `src/base.css`, `src/app.js`.

- `#find-count`: `flex: none;` so it never shrinks under its text (its `min-width: 7ch` let it
  shrink below a longer count, which then ran into the gap).
- `firstShownHit`, the forward branch: `hitRect(findHits[mid]).top >= top` in place of
  `.bottom > top`, so a match half under the bar isn't "on screen".
- Walk: a 420 px screenshot with "No matches" and "1 of 1,000+"; on README ×30 with a match
  half under the bar, Enter right after typing takes the next one, with no jump.

### Task 10: records and the probe

- Q4's Mac probe before Task 8 is walked.
- `docs/open-items.md` › Deferred (Q5): the JSON viewer's memory per chunk, with the Windows
  numbers. First step: measure which part of a chunk's rendered HTML dominates. Reopen on a
  report of a JSON file exhausting memory, or with the next JSON viewer work.
- R8's entry in `open-items.md` gets the new measurements, or closes if Task 8 lifts it.
- [ ] **Commit:** `docs:`.
