# Mermaid Pipeline Implementation Plan (Plan A of two)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A diagram can never style anything outside itself; diagrams draw in an isolated frame, carry only their own namespaced ids, survive a busy cache, and take the theme's colours — with a Settings choice to keep mermaid's own.

**Architecture:** All in the diagram draw path of `src/app.js`, plus one new bridge script and a small Rust setting. After every draw the page parses the drawing's `<style>` and refuses the diagram unless every rule is scoped under its own root id. Mermaid moves out of the page into a hidden `sandbox="allow-scripts"` iframe (opaque origin) that renders on request over `postMessage`; the page keeps its `$$` refusal, foreignObject backstop and the new style check on the returned string. Every drawing's ids are renamed under its root id at draw time by one DOM routine, which the overlay reuses. The drawing cache evicts least-recently-used entries. The look key grows to include a palette read from hidden probe elements, which drives mermaid's `base` theme when "Follow the theme" is on.

**Tech Stack:** plain JS (`src/app.js`, no bundler, no JS tests), mermaid 11.17.2 IIFE (`src/vendor/mermaid.min.js`), Rust (Tauri 2, `src-tauri/src/config.rs`, `main.rs`). The `drive-app` skill (CDP against the debug build) is the only frontend check.

**Spec:** the owner's rulings, 2026-09-27, on `docs/open-items.md` → `## Deferred`, and on this plan's review:
- *Diagrams in the theme's own colours* — do now, **plus a Settings option: "Follow the theme" (default) or "Mermaid's own", in Settings → Theme, shared by every window (broadcast).** With "Follow the theme" a diagram's own `theme`/`themeVariables`/`darkMode`/`fontFamily`/`themeCSS` are locked out; with "Mermaid's own" they work as today (as on GitHub). `classDef`/`style`, and per-type colour options (`c4.*_bg_color`, `journey.actorColours`, `sankey.linkColor`/`nodeColors`, `railroad.*`), work in both — the author chose those for that diagram.
- **"Follow the theme" fully:** `darkMode` inside `themeVariables`; explicit colour series (pie, git, cScale, journey fills, venn) rotated from the link colour at the theme's lightness; explicit values for the parts mermaid's `base` theme hard-codes light (gantt, architecture, event modeling).
- **Diagram styles that escape their drawing** (found in review; live in 1.7.0) — fix with a page-side check that refuses the diagram unless every style rule is scoped under its own id. Covers both modes; "Mermaid's own" keeps per-diagram `themeVariables`/`themeCSS`.
- *Some Mermaid ids are the document's own names, not namespaced* and *The overlay's id rewrite can change a label's text* — do now, one routine.
- *The diagram cache empties itself past 200* — do now (least recently used).
- *The foreignObject check runs after mermaid's temporary in-page render* — do now, **V2: mermaid inside an `allow-scripts`-only iframe (opaque origin), reached by `postMessage` through a bridge file.**
- **The frame's answer is trusted as mermaid's output is today**; the page's CSP (no inline script) is the backstop against a compromised frame. Accepted limit.

**Measured (2026-09-27; spike on the Windows debug build, review in headless Edge on the vendored bundle):**
- Today every `mermaid.render` builds the unsanitised drawing live as `div#d<id>` under `<body>`. `render(id, src, iframeBody)` crashes 7 of 8 types.
- Mermaid in a `srcdoc` iframe: CSP allows it (the frame inherits `script-src 'self'` and the base URL); output identical; 0 main-document mutations; render time unchanged (~70 ms for 8 types); frame load ~215 ms, once. With `sandbox="allow-scripts"` alone: `origin` is `"null"`, `parent.document` throws, inline script is blocked, `vendor/mermaid.min.js` still loads, Tauri `invoke` never reaches Rust. **Windows WebView2 only** — WebKitGTK and WKWebView untested.
- **CSS escape:** mermaid wraps its styles and `themeCSS` in `#<id>{…}` through stylis, which drops `@import`/`@font-face` and prefixes every selector — but stylis runs quoted strings across newlines and the browser ends them at one. A lone `"` in any `themeVariables` value (or `fontFamily`) turns the rest of mermaid's stylesheet into top-level rules: `%%{init: {"themeVariables": {"lineColor": "red\""}, "themeCSS": "body { background: red } #content { display: none }"}}%%` on `graph TD; A-->B` turned the app red and hid `#content` (45 unscoped rules). With the five keys in `secure`, zero escape. `@keyframes` stay global by design (the app defines none).
- Mermaid's own emitted styles never name an id other than `#<root>` (and, in the neo look, `url(#<root>-gradient)`); every rule it emits starts with `#<root>` followed by a space or `>`, or is exactly `#<root>`.
- Ids not starting with the render id: sequence `actor0…`, `root-0`, `actor-man-*`, `actor{N}_popup`; sankey `node-N`, `linearGradient-N` (via `stroke="url(#…)"`); architecture `IconifyId…`; treemap `clip-*` (via `clip-path`); `chart-title-<id>`/`chart-desc-<id>` (via aria); and **raw group names (`icons`, `tree`) only under `swimlane-beta` or front matter `layout: swimlane`** — the default layout already prefixes them. References emitted: `url(#…)` in `marker-*`/`stroke`/`clip-path`, aria lists, `<a xlink:href>` (diagram `click` links). Never `<use>`, `mask`, `filter`, `for`.
- `initialize` before every render is safe (restarts from defaults; icon packs survive); ~0.45 ms.
- Theme: `initialize` throws on `oklch()`, `lab()`, `hwb()`, `color()`, `color-mix()`, `var()`. Top-level `darkMode` is ignored — only `themeVariables.darkMode` counts. Colour series derived from a near-grey `primaryColor` collapse to greys (adjacent pie slices 1.2–1.4 contrast; dark `cScale`/venn black). `base` hard-codes `doneTaskBkgColor: lightgrey`, `altSectionBkgColor: white`, `excludeBkgColor: #eeeeee`, `taskTextClickableColor: #003163`, `archGroupBorderColor: #000`, `emUiFill: white`, `emSwimlaneBackgroundOdd: rgb(250,250,250)`; on dark `branchLabelColor` becomes black. 15 bundled themes give 13 distinct palettes but only 5 of today's look keys.
- Gantt sizes itself from the width of the element mermaid draws in — today the page's `<body>`.

## Rulings (plan review, 2026-09-27)

- Renderer failure fails fast: the frame's `load` event plus 1 s, 15 s backstop; retried on the next document, as today.
- The frame is sized to the page body's width before each render, so width-sensitive types (gantt) lay out as today.
- Radios stay in the Theme fieldset (owner's choice).
- Commit subjects: the style check (Task 1), ids (Task 3) and colours (Task 5) are plain; the cache (Task 2) and the frame (Task 4) are `chore:` — neither changes what a reader sees.
- Linux gates Task 4's commit; macOS goes into the existing "Mermaid diagrams on macOS and Linux" item.
- Diagram styles are parsed in a `<style media="not all">` — exactly the page's parser, applying nothing, measured 0.16 ms. (Not for old macOS: mermaid itself needs `new CSSStyleSheet()`, so a webview without it draws no diagrams either way.)
- No `contain: paint`: mermaid turns a rule on the diagram itself into `#id #id`, which matches nothing, and measured, no scoped rule (fixed position, huge transforms) leaves the figure.
- Series colours reach 4.5:1 against the text, or 3:1 on a theme whose own text is under 5:1 on its page (both Solarized themes), each hue stepped toward the page's side. The owner chose distinct hues with ~3:1 labels over pale, near-identical tints; a target tied to the theme's own contrast (~3.7) left category lines 1.04–1.11 from the page, so 3:1 it is.
- C4 relationship labels stay mermaid's fixed `#444444` (no theme key; 1.5–1.9:1 on dark), and author-chosen fills stay the author's — as on GitHub. Accepted limits (owner).
- Venn sets are mid-tone (60 % / 45 % lightness): mermaid writes a set's label in its own colour ±30, never the text colour.
- A swimlane group named like another figure's root (`Mermaid-12` in `Mermaid-1`) keeps its raw id: self-inflicted, one document. Accepted limit.

## Outcome (2026-09-27)

Done, one commit per task: c241696 (Task 1, the style check; fix round squashed in as
5850d37), 404b7f9 (Task 2, `chore:` the LRU cache), d730f11 (Task 3, ids), 7a63106 (Task 4, the
frame; `chore:`) plus 91bb286 (Task 4, `ci:` syntax-check `diagram-frame.js`), efc2b50 (Task 5,
theme colours and the Settings choice; fix round squashed in as b6b2798). All on `main`, none
pushed.

**Rulings made against the plan, in order** (full text in the ledger, `progress.md`):

- Task 1: fixed a real bypass in `selectors()`'s splitter — an escaped or quoted bracket
  (`.a\(`, `[title="("]`) never closed its depth, so a selector past one read as one scoped
  string instead of two. The spec (a diagram's styles never reach the page) outranks the
  plan's own code here; cost if wrong was a few lines.
- Task 3: committed as is, with the three swimlane sections' `icons` groups visible but
  clipped — mermaid measured them while the app's own `#icons { display: none }` still applied
  during its in-page render. Task 4 (render in the frame, with no app CSS reaching it) removes
  the cause, and its own verification confirms the groups lie inside the svg viewBox, not
  clipped; the interim state lived only between two local commits.
- Task 5: radar's `graticuleOpacity` is 0.06, not mermaid's own 0.3 — its five nested, filled
  rings at 0.3 stack to about 83% and band solid on both sides of the curve; 0.06, viewed on
  github-dark and github-light, leaves the rings faint and the curve clear. Cost if wrong: one
  number.

**Linux is not proven.** Task 4 Step 5 (g) — the frame on WebKitGTK — was deferred: WSL lacked
the packages the check needs sudo for. The owner chose to commit Task 4 and run Linux later; it
must pass before the owner pushes (see the amended item in `open-items.md`, under *Needs a Mac
or a Linux install*): `sudo apt install webkitgtk-webdriver xvfb imagemagick`,
`cargo install tauri-driver --locked`, then `.claude/skills/drive-app/linux.md`'s run of Task 4
Step 5 (a), (b) and (d) on the debug build. Windows (WebView2) is proven throughout; macOS
(WKWebView) still needs a Mac, as before.

**Found while running it:**

- Between Task 1 and Task 4, an escaping diagram was still momentarily live. Task 1's check
  refuses a diagram only after reading the SVG string mermaid hands back, but mermaid itself
  built that string by rendering into a temporary element of the app's own document until Task
  4 moved rendering into the isolated frame — so a diagram later refused, for reaching outside
  itself (Task 1) or for an id colliding with the app's own (Task 3), had its raw, unrefused
  markup transiently live in the real page in between. Task 4 closes that gap for good; it is
  also why the plan's own foreignObject-backstop item stayed open until now.
- The plan's Task 4 Step 5 (c) foreignObject example was wrong: `architecture-beta` /
  `service t(server)["<b>x</b>"]` has no foreignObject and draws as before — the iconText form,
  `service t "<b>x</b>"[T]`, is what actually trips the refusal. Checked against HEAD's own
  render path for parity: the plan's literal example gives `fo=0` there too, so nothing
  regressed — the example just never reached what it meant to test.
- Task 5 Step 3's palette key list assumed `emSwimlaneBackgroundEven`; the 11.17.2 bundle has
  no such key, so it is left unset.

**Measured:**

- Baseline (Task 1 Step 1, HEAD): 35 diagram sections × 2 themes (github-dark, github-light),
  0 refused. The style check held 105/105 across the review's verification passes of those 35
  sections, no diagram newly refused, plus 7 direct-call cases for the splitter fix
  (escaped/quoted brackets, nested `:is()`, a raw comma).
- The frame's failure paths (Task 4 Step 5 (d)): the renderer script missing fails in 20 ms
  (nothing had loaded yet); the bridge script missing fails in 1204 ms (the frame's `load`
  event plus the 1 s grace); either way the blocks show as code and the frame is rebuilt for the
  next document.
- A window's first diagram document costs about +15 ms for the frame, not the ~200 ms the
  spike estimated; later documents with new diagrams measured 13–18% faster than before the
  frame, since it has no app stylesheet to recalculate against.
- The 15 bundled themes (`azure-devops`, `azure-devops-dark`, `azure-devops-blue`,
  `azure-devops-dark-blue`, `github-light`, `github-dark`, `github-light-blue`,
  `github-dark-blue`, `solarized-light`, `solarized-dark`, `dracula`, `dracula-blue`,
  `dracula-green`, `sakura`, `tufte`) all pass `initialize` with the derived palette; the lowest
  category-to-text contrasts are Solarized light (3.066, against the 3:1 target its own
  sub-5:1 text earns) and Solarized dark (3.006).
- `cargo test --manifest-path src-tauri/Cargo.toml`: 190 passed, 0 failed.

**Deferred minors, for the final whole-branch review** (full list in `progress.md`): a
comment's wording and a UTF-16 index loop (Task 1); the overlay test order, `url(#…)`
rewriting in `aria-label`/`data-*`, an ASCII-only id regex, a comment fragment, and duplicate
ids from the same source used twice in one document (Task 3); the frame's message listener
registered before `append` (apply before the Linux run above), an empty error string read as
success, and a comment fragment (Task 4); two module-level bindings both named `probe` twenty-
two lines apart (Task 5). None blocked a task.

**Deferred, tracked** in `open-items.md`: the render-never-settles item, rewritten with the
frame's own gap — a lost frame after `ready` has no per-render timeout; and a new item, a
Gantt `click … href` link doing nothing (`bindFunctions` is never called, since a diagram now
reaches the page only as an SVG string). **Accepted, tracked** in `closed-items.md`: the six
limits ruled on above, plus the frame's answer being trusted as mermaid's in-page output was.

## Global Constraints

- mermaid stays **11.17.2**; no new dependencies.
- Commit straight to `main`; never push (the owner pushes).
- Commit trailer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Rust: `cargo fmt` and `cargo test --manifest-path src-tauri/Cargo.toml` pass before any commit that touches Rust.
- `node --check src/app.js` (and `src/diagram-frame.js` once it exists) before every commit.
- Driving the app: `.claude/skills/drive-app/SKILL.md`. **The owner sets `"reopen": "off"` in `%APPDATA%\t4-markdown-viewer\config.json` by hand before execution and back afterwards; no agent writes `config.json`** (the sandbox refuses it). Each task backs up and restores `session.json` only, kills only the debug build it launched (by PID; the owner's viewer shares the process name), re-reads PID and window handle after any relaunch, and ends with a plain `cargo build`. Any Settings choice a check changes in the app is set back through the app before cleanup.
- Pass paths to `openTab` with forward slashes. `await applyTheme(…)`.
- Fixture: `docs/fixtures/all-types.md` holds one source per mermaid 11.17.2 diagram type, a swimlane case, and a `## Heading B`. Copy it next to other scratch docs when driving the app.
- Line numbers below are the **pre-change** file's; find code by the content quoted.
- Comments say why, in full sentences, like the surrounding code.

## Review Focus

1. **Every diagram type still draws as before** — geometry and colours — through the style check, the frame and the renaming → baseline in Task 1 Step 1, compared in Task 1 Step 5, Task 3 Step 4 (a), Task 4 Step 5 (a).
2. **A diagram's styles never reach the page**, in either colour mode → Task 1 Step 5 (b), Task 5 Step 6 (d).
3. **Nothing mermaid draws is ever live in the app's document**, and the page no longer has `window.mermaid` → Task 4 Step 5 (b).
4. **A diagram link to a document heading still works**, and a link to a name that is only a diagram group does nothing → Task 3 Step 4 (b).
5. **Switching between two dark themes redraws the diagrams**, which today's key misses → Task 5 Step 6 (b).
6. **The renderer failing to load** leaves blocks as code within about a second, and the next document tries again → Task 4 Step 5 (d).
7. **The frame works on WebKitGTK** → Task 4 Step 5 (g).

---

### Task 1: Refuse a diagram whose styles reach outside it

**Files:**
- Modify: `src/app.js` — new `stylesStayInside` after `TEXT_ONLY_FO` (`app.js:436`); `warmSources` (after the foreignObject backstop, `app.js:547-549`).

**Interfaces:**
- Produces: `stylesStayInside(root: SVGSVGElement): boolean` — true when every rule of every `<style>` in `root` is scoped under `#<root.id>`. In `warmSources`, the successful branch now parses the SVG into `const t` (a `<template>`) and `const root` (its `svg`) before `done = …`; Task 3 extends exactly that block.

- [x] **Step 1: Baseline** (per `drive-app`, on the build at HEAD before any change). Gantt takes its width from the page on screen when it draws, so this and every later comparison opens `all-types.md` at the same window size, from the same diagram-free page (`docs/fixtures/blank.md`, one paragraph). Open `all-types.md` under `github-dark` and under `github-light`. For each figure record, to `scratchpad/baseline.json`: its section heading, `svg.getAttribute('viewBox')`, and the computed `fill` and `stroke` of the first `.node rect, .node path, .node polygon, rect, path` inside it and of the first `.edgePath path, .flowchart-link, path.relation, line` (whichever exists); and for each block left as code, its error text. Screenshot each figure (`scratchpad/base-<theme>-<section>.png`). This baseline is compared after Tasks 1, 3 and 4.

- [x] **Step 2: Add the check** after `TEXT_ONLY_FO`:

```js
/**
 * Whether every rule in a drawing's styles is scoped under its root, as
 * mermaid means them to be. mermaid scopes them itself, but a stray `"` in a
 * diagram's own theme values makes its CSS compiler and the browser disagree
 * about where a string ends, and the rest of its stylesheet then applies to
 * the whole app. Parsed the way the page will parse it. Keyframes are global
 * by nature, and harmless: the app has none of its own.
 */
function stylesStayInside(root) {
  const scope = `#${root.id}`;
  // Split on top-level commas only: `:is(a, b)` is one selector.
  const selectors = (text) => {
    const out = [""];
    let depth = 0;
    for (const c of text) {
      if (c === "," && !depth) out.push("");
      else out[out.length - 1] += c;
      if (c === "(" || c === "[") depth++;
      if (c === ")" || c === "]") depth--;
    }
    return out.map((s) => s.trim());
  };
  // The root itself, or inside it — not its siblings (`~`, `+`), and not `#Mermaid-10` for `#Mermaid-1`.
  const scoped = (s) => s.startsWith(scope) && /^(\s*>|\s+[^\s~+]|$)/.test(s.slice(scope.length));
  const ok = (rules) =>
    [...rules].every((r) =>
      r instanceof CSSKeyframesRule ||
      // A nested rule only comes from the escape: mermaid's compiler flattens.
      (r instanceof CSSStyleRule ? selectors(r.selectorText).every(scoped) && !r.cssRules?.length
        : r instanceof CSSGroupingRule && ok(r.cssRules)));
  // Parsed the way the page will parse it, by a `<style>` that matches no
  // media, so it applies to nothing while it is read.
  const probe = document.createElement("style");
  probe.media = "not all";
  try {
    return [...root.querySelectorAll("style")].every((style) => {
      probe.textContent = style.textContent;
      document.head.append(probe);
      return ok(probe.sheet.cssRules);
    });
  } finally {
    probe.remove();
  }
}
```

- [x] **Step 3: Call it** in `warmSources`. Replace `        done = { svg };` (right after the foreignObject `throw`) with:

```js
        const t = document.createElement("template");
        t.innerHTML = svg;
        const root = t.content.querySelector("svg");
        if (!stylesStayInside(root))
          throw new Error("This diagram's styles reach outside it, so it isn't drawn.");
        done = { svg };
```

(`<template>` content is inert: its `<style>` applies to nothing while it is checked.)

- [x] **Step 4:** `node --check src/app.js`; `cargo build`.

- [x] **Step 5: Verify** (per `drive-app`):
  - (a) `all-types.md` under both themes: every figure's record equals the baseline (no type newly refused; viewBox, fill, stroke equal).
  - (b) `docs/fixtures/escape.md`, a paragraph first, then three diagrams: `graph TD; A-->B` with `%%{init: {"themeVariables": {"lineColor": "red\""}, "themeCSS": "body { background: red } #content { display: none }"}}%%`; the same with `"fontSize": "16px\""` instead of `lineColor`; and one with front matter `config: themeVariables: fontSize: '16px"'`. Each stays as code with the new message; `getComputedStyle(document.body).backgroundColor` and `#content`'s `display` are unchanged; `getComputedStyle(document.querySelector('#content p')).marginTop` equals the same read on a diagram-free document with a paragraph, opened just before.
  - (c) A balanced `themeCSS` that mermaid scopes (`%%{init: {"themeCSS": ".node rect { fill: red }"}}%%`) still draws, with its node red.
  - (d) No console errors besides the expected.

- [x] **Step 6: Commit** `src/app.js` — subject `Refuse a diagram whose styles would reach outside it`.

---

### Task 2: Keep the most recently used diagrams

**Files:**
- Modify: `src/app.js:417-425` (cache comment and `DIAGRAMS_KEPT`), `src/app.js:492-500` (`warmSources`, `todo` and the clear)

**Interfaces:**
- Produces: `diagrams` stays a `Map` of key → `{ svg } | { error }`, now in least-recently-used order; nothing outside `warmSources` changes.

- [x] **Step 1: Replace the cache comment and constant** (`app.js:417-425`):

```js
/**
 * Finished diagrams, `{ svg }` or `{ error }`, by look and source. Filled
 * before a document is painted, so `renderDiagrams` swaps them in without
 * waiting — which is what lets the scroll restore land on a page that already
 * has its full height. A save that leaves a diagram alone redraws it for free.
 * Kept in the order last used, oldest first, so what goes when it is full is
 * what the window has shown least recently.
 */
const diagrams = new Map();
const DIAGRAMS_KEPT = 200;
```

- [x] **Step 2: Replace the wholesale clear** in `warmSources`. Make `let todo` a `const`, and delete:

```js
    // Past the limit, start over — this document included, or the diagrams it
    // already had would be dropped and shown as code.
    if (diagrams.size + todo.length > DIAGRAMS_KEPT) {
      diagrams.clear();
      todo = [...sources];
    }
```

Right after `const todo = …` and **before** `if (!todo.length) return;` (so a fully cached document is still marked as used), insert:

```js
    // This document's drawings become the newest, so the oldest are other
    // documents'; drop those to make room, never this one's — a document with
    // more diagrams than the limit keeps them all.
    const mine = new Set();
    for (const s of sources) {
      const key = diagramKey(look, s);
      mine.add(key);
      const done = diagrams.get(key);
      if (done) diagrams.delete(key), diagrams.set(key, done);
    }
    for (const key of diagrams.keys()) {
      if (diagrams.size + todo.length <= DIAGRAMS_KEPT || mine.has(key)) break;
      diagrams.delete(key);
    }
```

- [x] **Step 3: Verify** (`node --check`; then per `drive-app`):
  - (a) Fresh window. `for (let i = 0; i < 205; i++) diagrams.set('x' + i, { error: 'x' })`; open `examples/kitchen-sink.md` (one diagram): it draws; `diagrams.size === 200`; `!diagrams.has('x5') && diagrams.has('x6')`.
  - (b) `diagrams.set('y', { error: 'y' })`; open another tab and come back: `[...diagrams.keys()].at(-1)` is kitchen-sink's key; `diagrams.size === 200`.
  - (c) The case the move-to-end exists for. `diagrams.clear()`; open `docs/fixtures/three.md` (3 diagrams, its 3 keys now oldest); add 198 dummies (size 201 — over, as a later insert would find it); edit `three.md` on disk so one diagram's source changes and wait for the refresh: the 2 unchanged keys are still present, the new one drew, `diagrams.size === 200`, and the last 3 keys are `three.md`'s.

- [x] **Step 4: Commit** `src/app.js` — subject `chore: keep the most recently used diagrams instead of emptying the cache`.

---

### Task 3: Namespace every id inside a diagram

**Files:**
- Modify: `src/app.js` — new `renameIds` after `diagramKey`; `warmSources` (the block Task 1 added); `openDiagram` (`app.js:718-724`, the overlay copy).

**Interfaces:**
- Consumes: Task 1's `t`/`root` in `warmSources`.
- Produces: `renameIds(svg: SVGSVGElement, rename: (id: string) => string | null): void` — renames every non-empty id `rename` maps (the root's too), and every reference to a renamed id: `url(#id)` / `url("#id")` in any attribute value, `href`/`xlink:href` of `#id` on any element **except `<a>`** (a diagram's `click` link names a document target), the `aria-labelledby`/`aria-describedby` lists, and `#id` in selectors and `url(#id)` in the drawing's `<style>`. Text content and style declarations are never touched. One pass per string, so a new name is never renamed again.
- Every cached `svg` string is namespaced: every id in it starts with the root's id (`Mermaid-<n>`).

- [x] **Step 1: Add `renameIds`** after `diagramKey`:

```js
/**
 * Rename ids inside a drawing, and every reference to them: `url(#…)` in any
 * attribute, `href="#…"` on anything but a link — a diagram's `click` link
 * names a place in the document, not a part of the drawing — the aria id
 * lists, and selectors and `url(#…)` in its own `<style>`. Text is left
 * alone, so a label that mentions an id reads the same; so are declarations,
 * where `#aaa` is a colour.
 */
function renameIds(svg, rename) {
  const renamed = new Map();
  for (const el of [svg, ...svg.querySelectorAll("[id]")]) {
    if (!el.id) continue;
    const to = rename(el.id);
    if (to && to !== el.id) renamed.set(el.id, to), (el.id = to);
  }
  if (!renamed.size) return;
  const url = /url\((['"]?)#([^'")]+)\1\)/g;
  const toUrl = (m, q, id) => (renamed.has(id) ? `url(${q}#${renamed.get(id)}${q})` : m);
  for (const el of [svg, ...svg.querySelectorAll("*")]) {
    for (const attr of [...el.attributes]) {
      let value = attr.value.replace(url, toUrl);
      if (attr.localName === "href" && el.localName !== "a" && renamed.has(value.slice(1)) && value.startsWith("#"))
        value = `#${renamed.get(value.slice(1))}`;
      if (attr.name === "aria-labelledby" || attr.name === "aria-describedby")
        value = value.split(/\s+/).map((id) => renamed.get(id) ?? id).join(" ");
      if (value !== attr.value) attr.value = value;
    }
  }
  const hash = (m, id) => (renamed.has(id) ? `#${renamed.get(id)}` : m);
  for (const style of svg.querySelectorAll("style"))
    style.textContent = style.textContent
      .replace(/[^{}]+(?=\{)/g, (selector) => selector.replace(/#([\w-]+)/g, hash))
      .replace(url, toUrl);
}
```

- [x] **Step 2: Namespace at draw time.** In `warmSources`, in Task 1's block, replace `        done = { svg };` with:

```js
        // Every id under the drawing's own, so a group named like part of the
        // app — `icons`, `tree` — can neither wear the app's styles nor be
        // what a `#tree` link finds.
        renameIds(root, (id) => (id.startsWith(root.id) ? null : `${root.id}-${id}`));
        done = { svg: t.innerHTML };
```

- [x] **Step 3: The overlay copy** — in `openDiagram`, replace the comment above the `ids` regex, the `ids` regex, the `els.diagramView.innerHTML = …replace(…)` line and `const svg = els.diagramView.querySelector("svg");` with:

```js
  // Its own ids: the copy's styles and arrowheads then point at itself rather
  // than at the original, which a re-render can take away while this is open.
  // Every id in a drawing starts with its root's (`renameIds` at draw time).
  const t = document.createElement("template");
  t.innerHTML = fig.innerHTML;
  const svg = t.content.querySelector("svg");
  const root = original.id;
  renameIds(svg, (id) => (id.startsWith(root) ? `${root}-zoom${id.slice(root.length)}` : null));
  els.diagramView.replaceChildren(t.content);
```

- [x] **Step 4: Verify** (per `drive-app`). `docs/fixtures/ids.md`, in this order: (1) a flowchart whose label names its own id, first so it is `Mermaid-1` in a fresh window: `graph TD; L["see #Mermaid-1 and #quot;Mermaid-1#quot;"] --> M`; (2) the `flowSwimlaneLayout` section of `all-types.md` (front matter `layout: swimlane`, groups `icons` and `tree`); (3) `graph TD; A --> B` with `click A href "#heading-b"`; then `[to tree](#tree)` as a paragraph and `## Heading B`. **First, on the build before this task**, confirm the test bites: `document.querySelectorAll('#content [id="icons"]').length === 1` and the swimlane group is hidden (`getBoundingClientRect().height === 0`); the overlay of figure (1) shows `Mermaid-1-zoom` in a label. Then, after:
  - (a) Every figure draws; `document.querySelectorAll('[id="icons"]').length === 1` and `…[id="tree"]…` `=== 1` (the app's own); the `icons` group is visible; every id inside every figure starts with its svg's id; every `url(#…)` in every figure's attributes resolves (`document.getElementById(id)` non-null); `all-types.md` matches the baseline under both themes, except that the three swimlane sections' `icons` groups now show (the fix).
  - (b) Clicking `[to tree](#tree)` does nothing (no scroll, `location.hash` empty); clicking node `A` jumps to `## Heading B`.
  - (c) Overlay on figure (1): `[...copy.querySelectorAll('text')].map((t) => t.textContent).join('\n')` equals the same on the page's svg (and contains `Mermaid-1`, not `Mermaid-1-zoom`); every id in the copy starts with `Mermaid-1-zoom`; arrowheads resolve in the copy. Overlay on the swimlane figure: its groups keep their colours (screenshot vs page).

- [x] **Step 5: Commit** `src/app.js` — subject `Keep a diagram's own names from colliding with the app's`.

---

### Task 4: Draw diagrams in an isolated frame

**Files:**
- Create: `src/diagram-frame.js` (the bridge, runs inside the frame)
- Modify: `src/app.js:393-413` (`loadMermaid` → `loadRenderer`), `src/app.js:427` (queue comment), `warmSources` (`app.js:494-541`: initialize/render calls, the yield comment at 527-529)
- Modify: `.github/workflows/checks.yml:64` (syntax-check the new file)

**Interfaces:**
- Consumes: Task 1's check and Task 3's `renameIds` — unchanged in `warmSources`.
- Produces: `loadRenderer(): Promise<(config: object, id: string, source: string) => Promise<{ svg: string, diagramType: string } | { error: string }>>`. Rejects with `Error("The diagram renderer did not load.")` if the frame reports failure, or has loaded without saying it is ready 1 s later, or 15 s pass; a rejected load is forgotten so the next document tries again.

- [x] **Step 1: Create the bridge** `src/diagram-frame.js`:

```js
/*
 * Runs inside the diagram frame: a sandboxed iframe with scripts but no origin
 * of its own, so nothing mermaid draws or runs here can reach the app's page
 * or Tauri. The page sends one diagram at a time and gets back mermaid's SVG
 * string, or why it failed.
 */
if (typeof mermaid === "undefined") parent.postMessage({ failed: true }, "*");
else {
  addEventListener("message", async (e) => {
    if (e.source !== parent || !e.data) return;
    const { n, config, id, source, width } = e.data;
    // The width mermaid lays out in — gantt fills it — as the page's was.
    document.body.style.width = `${width}px`;
    try {
      mermaid.initialize(config);
      const { svg, diagramType } = await mermaid.render(id, source);
      parent.postMessage({ n, svg, diagramType }, "*");
    } catch (err) {
      parent.postMessage({ n, error: String(err?.message ?? err) }, "*");
    }
  });
  parent.postMessage({ ready: true }, "*");
}
```

- [x] **Step 2: Replace `loadMermaid`** (`app.js:393-413`, its comment included) with:

```js
/**
 * The diagram renderer: mermaid, in a hidden frame of its own. It is 3.5 MB of
 * script, so it loads the first time a document has a diagram. And it runs in
 * a frame sandboxed to scripts alone, with no origin: mermaid builds each
 * drawing live in its document before handing back the string, so the
 * unsanitised drawing is never in the app's page, and nothing it runs can
 * reach the page or Tauri. The frame must be laid out, not hidden, for mermaid
 * to measure text.
 */
let rendererLoad = null;
function loadRenderer() {
  rendererLoad ??= new Promise((resolve, reject) => {
    const frame = document.createElement("iframe");
    frame.setAttribute("sandbox", "allow-scripts");
    frame.setAttribute("aria-hidden", "true");
    frame.inert = true; // never focused or clicked into; its scripts still run
    frame.style.cssText = "position:fixed;left:-20000px;top:0;width:1200px;height:1200px;border:0";
    frame.srcdoc =
      '<!doctype html><html><head><script src="vendor/mermaid.min.js"></script>' +
      '<script src="diagram-frame.js"></script></head><body style="margin:0"></body></html>';
    let n = 0;
    let settled = false;
    const waiting = new Map();
    const fail = () => {
      if (settled) return;
      settled = true;
      removeEventListener("message", onMessage);
      frame.remove();
      rendererLoad = null; // the next document tries again
      reject(new Error("The diagram renderer did not load."));
    };
    // The frame's scripts have run by its load event; a bridge that never
    // loaded — or a webview that refused it — says nothing, so give it a
    // moment past that rather than holding the document up for the backstop.
    frame.addEventListener("load", () => setTimeout(fail, 1000));
    setTimeout(fail, 15000);
    const render = (config, id, source) =>
      new Promise((settle) => {
        waiting.set(++n, settle);
        // The width goes with the request: the frame may run in a process of
        // its own, where a resize could land after the render.
        frame.contentWindow.postMessage({ n, config, id, source, width: document.body.offsetWidth }, "*");
      });
    function onMessage(e) {
      if (e.source !== frame.contentWindow || typeof e.data !== "object" || !e.data) return;
      if (e.data.failed) return fail();
      if (e.data.ready) return settled || ((settled = true), resolve(render));
      const settle = waiting.get(e.data.n);
      if (!settle) return;
      waiting.delete(e.data.n);
      // Strings only: the frame runs whatever the file hands mermaid.
      const { svg, diagramType, error } = e.data;
      settle(typeof svg === "string" ? { svg, diagramType: String(diagramType) } : { error: String(error ?? "The diagram did not draw.") });
    }
    addEventListener("message", onMessage);
    document.body.append(frame);
  });
  return rendererLoad;
}
```

- [x] **Step 3: Use it in `warmSources`.**
  - Replace `const mermaid = await loadMermaid();` with `const render = await loadRenderer();`.
  - Replace the `mermaid.initialize({ … });` statement by building the same object into `const config = { … };` (every key and comment kept; Task 5 changes the colour keys).
  - Inside the loop replace `const { svg, diagramType } = await mermaid.render(\`Mermaid-${++diagramId}\`, source);` with:

```js
        const drawn = await render(config, `Mermaid-${++diagramId}`, source);
        if (drawn.error) throw new Error(drawn.error);
        const { svg, diagramType } = drawn;
```

  - The queue comment (`app.js:427`) becomes: `/** One warm at a time: the frame renders one diagram at a time, and a warm's look must not change under it. */`
  - The yield comment (`app.js:527-528`) becomes: `// Let a click that switches away run between diagrams, rather than after the last.`
  - A rejected `loadRenderer()` still propagates out of `run` to the queue's `.catch(console.error)`, leaving the blocks as code (`warmSources`' "Never rejects" holds).

- [x] **Step 4:** `node --check src/app.js`; `node --check src/diagram-frame.js`; `grep -n "loadMermaid\|mermaidLoad\|window.mermaid" src/app.js` finds nothing; `cargo build` (assets embed at build time; `frontendDist` is `../src`, so the new file ships).

- [x] **Step 5: Verify** (per `drive-app`):
  - (a) `all-types.md` under both themes matches the baseline (every type: same drawing — viewBox, fill, stroke — or the same refusal message). Gantt's viewBox equal is the width ruling holding; if it differs, report the two widths.
  - (b) Isolation: `typeof window.mermaid === "undefined"`; no `script[src*="mermaid"]` in the page; the frame exists with `contentDocument === null`. Then, with a `MutationObserver` on `document.documentElement` (subtree, childList, attributes excluded), `const src = \`graph TD; Q${Date.now()}-->R\`; await warmSources(new Set([src]), () => false)`; `observer.takeRecords().length === 0` and `diagrams.get(diagramKey(diagramLook(), src)).svg` is a string.
  - (c) The `$$` refusal, the foreignObject backstop (`architecture-beta` / `service t(server)["<b>x</b>"]`) and Task 1's escape doc refuse as before.
  - (d) Renderer failure, both paths. Fresh window, before any diagram: `window.__d = Object.getOwnPropertyDescriptor(HTMLIFrameElement.prototype, 'srcdoc'); Object.defineProperty(HTMLIFrameElement.prototype, 'srcdoc', { ...__d, set(v) { __d.set.call(this, v.replace(window.__from, 'missing.js')); } })`.
    - `__from = 'vendor/mermaid.min.js'`: open a diagram document → it shows within ~1 s with its blocks as code; `rendererLoad === null`; no frame in the page.
    - `__from = 'diagram-frame.js'`: open another → the same, within ~1.5 s (load + grace), not 15 s.
    - Restore (`Object.defineProperty(HTMLIFrameElement.prototype, 'srcdoc', __d)`); open a third → draws. One `console.error` per failure is expected; nothing else.
  - (e) Timing: first diagram document open vs the baseline build (expect ~+200 ms once per window); later opens unchanged.
  - (f) No console errors, `unhandledrejection` or `securitypolicyviolation`.
  - (g) **Gate: Linux.** Per `.claude/skills/drive-app/linux.md`, run (a) for `all-types.md` under one theme, (b), and (d)'s first path on WebKitGTK. If the frame's scripts are refused there, or a slow-but-working load is failed by the 1 s grace (WebKit firing `load` for the initial `about:blank` first), stop and report — that is a plan-level problem, not a fix to guess.

- [x] **Step 6: Commit** `src/app.js`, `src/diagram-frame.js` — subject `chore: draw diagrams in an isolated frame, so nothing mermaid builds is ever live in the page`.

- [x] **Step 7: CI.** In `.github/workflows/checks.yml`, after `node --check ../src/app.js` (line 64), add a **separate** `node --check ../src/diagram-frame.js` command — `node --check a.js b.js` checks only `a.js`. Commit — subject `ci: syntax-check the diagram frame's script`.

---

### Task 5: Diagrams in the theme's colours, with a Settings choice

**Files:**
- Modify: `src-tauri/src/config.rs` (constant, normaliser, field, default, `load`, tests), `src-tauri/src/main.rs` (`set_diagram_colours` command, registration)
- Modify: `src/index.html` (radios in the Theme fieldset), `src/app.js` (`els`, `state`, `pixel` comment, `diagramLook`, `warmSources` config, `showDiagramColours`, radio listeners, boot, listener)

**Interfaces:**
- Consumes: everything above, unchanged.
- Produces: `diagramLook(): { dark: boolean, fontFamily: string, palette: object | null, key: string }` — `palette` is mermaid `themeVariables` (every colour `#rrggbb`, including inside its `packet` and `xyChart` objects; `xyChart.plotColorPalette` is a comma list of them; plus `fontFamily` and `darkMode`) when `state.diagramColours === "theme"`, else `null`; `key` changes whenever mode, side, font or palette does. Rust: `config::diagram_colours(raw: &str) -> &'static str` (`"mermaid"` or `"theme"`), `Config.diagram_colours: String` (default `"theme"`), command `set_diagram_colours(app, colours: String)` emitting `"diagram-colours-changed"` with the normalised value.

- [x] **Step 1: Rust setting.** In `config.rs`, following `folder_sort`'s pattern exactly:
  - `pub const DEFAULT_DIAGRAM_COLOURS: &str = "theme";` with a doc comment (`"theme"` draws diagrams in colours read from the page; `"mermaid"` in mermaid's own default or dark palette, letting a diagram pick its own theme).
  - `pub fn diagram_colours(raw: &str) -> &'static str { match raw { "mermaid" => "mermaid", _ => DEFAULT_DIAGRAM_COLOURS } }`.
  - `pub diagram_colours: String` on `Config`, with a doc comment; `DEFAULT_DIAGRAM_COLOURS.to_string()` in `Default`; normalised in `load()` next to `folder_sort`.
  - Tests in the file's style: `config_without_diagram_colours_still_loads` (an old JSON → `"theme"`) and `diagram_colours_accepts_only_the_two` (`"mermaid"`, `"theme"`, `"Mermaid"`→`"theme"`, `""`→`"theme"`).

  In `main.rs`, after `set_open_mode`:

```rust
/// Broadcast, like the open mode: every window's diagrams should agree about
/// whose colours they wear.
#[tauri::command]
fn set_diagram_colours(app: AppHandle, colours: String) {
    let colours = config::diagram_colours(&colours).to_string();
    let mut cfg = config::load();
    cfg.diagram_colours = colours.clone();
    config::save(&cfg);
    let _ = app.emit("diagram-colours-changed", colours);
}
```

  Register it in `invoke_handler` next to `set_open_mode`. `cargo fmt`, `cargo test`.

- [x] **Step 2: Settings radios** in `index.html`, inside the Theme fieldset after `<select id="theme-picker" …>`:

```html
          <p class="hint diagram-colours-hint">Diagrams draw in the theme's colours, or in Mermaid's own light or dark ones.</p>
          <label><input type="radio" name="diagram-colours" value="theme" /> Diagrams follow the theme</label>
          <label><input type="radio" name="diagram-colours" value="mermaid" /> Diagrams in Mermaid's own colours</label>
```

  and in `base.css`, next to `#settings-dialog .hint` (`base.css:613`): `#settings-dialog .diagram-colours-hint { margin-top: 0.75rem; }` — the hint follows the picker, not a legend.

- [x] **Step 3: Palette in the look.** In `app.js`:
  - `els.diagramColourRadios: document.querySelectorAll('#settings-dialog input[name="diagram-colours"]')`; `state.diagramColours: "theme"`.
  - The `pixel` comment (`app.js:438-442`) becomes: `One pixel to paint colours on and read back. A computed colour can be any CSS form — \`oklch(…)\`, \`color(srgb 0.97 …)\` — whose numbers are not 0–255 channels, and mermaid takes none of those; a canvas hands back sRGB bytes for all.`
  - After `let pixel = null;` (before `diagramLook`'s comment), declare:

```js
/** Hidden stand-ins for prose, code and a link, styled by the theme like the page's own. */
let probe = null;
```

  - Extend `diagramLook`'s comment with: `With the theme's colours, also the palette: the page's text, code background and link colour, as the theme styles a document.` and replace its body with:

```js
function diagramLook() {
  pixel ??= document.createElement("canvas").getContext("2d", { willReadFrequently: true });
  // A colour as sRGB bytes, painted over `under` — any CSS form, see-through or not.
  const bytes = (css, under) => {
    pixel.fillStyle = under;
    pixel.fillRect(0, 0, 1, 1);
    pixel.fillStyle = css;
    pixel.fillRect(0, 0, 1, 1);
    return [...pixel.getImageData(0, 0, 1, 1).data.slice(0, 3)];
  };
  // On the page's own canvas colour first: that is what the webview paints
  // behind a page with no background — dark under a dark `color-scheme` — and
  // a see-through colour read on its own comes back black. The overlay's
  // background colour is exactly that system `Canvas`.
  const bg = bytes(getComputedStyle(document.body).backgroundColor, getComputedStyle(els.diagramDialog).backgroundColor);
  const dark = 0.299 * bg[0] + 0.587 * bg[1] + 0.114 * bg[2] < 128;
  const fontFamily = getComputedStyle(els.content).fontFamily;
  if (state.diagramColours !== "theme") return { dark, fontFamily, palette: null, key: `m\0${dark}\0${fontFamily}` };
  if (!probe) {
    probe = document.createElement("div");
    probe.className = "markdown-body";
    probe.setAttribute("aria-hidden", "true");
    probe.style.cssText = "position:absolute;left:-10000px;top:0;visibility:hidden";
    probe.innerHTML = '<p>x</p><pre><code>x</code></pre><p><a href="#">x</a></p>';
    document.body.append(probe);
  }
  const hexOf = (c) => `#${c.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
  const page = hexOf(bg);
  const over = (el, prop) => bytes(getComputedStyle(el)[prop], page);
  const fg = over(probe.querySelector("p"), "color");
  const code = over(probe.querySelector("pre"), "backgroundColor");
  const accent = over(probe.querySelector("a"), "color");
  const mix = (a, b, t) => hexOf(a.map((v, i) => Math.round(v * t + b[i] * (1 - t))));
  const lum = (c) =>
    c.map((v) => ((v /= 255) <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)).reduce((s, v, i) => s + v * [0.2126, 0.7152, 0.0722][i], 0);
  const contrast = (a, b) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05);
  const hue = ([r, g, b]) => {
    const max = Math.max(r, g, b), d = max - Math.min(r, g, b);
    if (!d) return 210;
    const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
    return Math.round(h * 60);
  };
  // Categories — pie slices, git branches, mindmap and timeline sections,
  // event-modeling boxes, chart bars — need colours that differ: hues stepped
  // by the golden angle from the link colour's. Each is taken toward the page's
  // side until the text reads on it at 4.5:1 — or 3:1 on a theme whose own
  // text is under 5:1 on its page (Solarized's is 4.1–4.7), where a higher
  // bar would leave the colours, and the lines mermaid draws in them, barely
  // apart from the page.
  const target = contrast(fg, bg) >= 5 ? 4.5 : 3;
  const hueAt = (i) => (hue(accent) + i * 137.5) % 360;
  const cats = Array.from({ length: 12 }, (_, i) => {
    for (let l = dark ? 32 : 82; ; l += dark ? -2 : 2) {
      const c = bytes(`hsl(${hueAt(i)} ${dark ? 40 : 60}% ${l}%)`, page);
      if (contrast(fg, c) >= target || l <= 0 || l >= 100) return hexOf(c);
    }
  });
  // Venn writes a set's label in the set's own colour, 30 lighter or darker,
  // so its sets sit mid-way rather than behind the text.
  const venn = Array.from({ length: 8 }, (_, i) => hexOf(bytes(`hsl(${hueAt(i)} 60% ${dark ? 60 : 45}%)`, page)));
  const each = (prefix, colours, from = 0) => Object.fromEntries(colours.map((c, i) => [`${prefix}${i + from}`, c]));
  // Measured across the bundled themes (2026-09-27): their own border colours
  // are too faint to outline a node, so borders and lines are the text mixed
  // into the page; a code background as faint as the page gets the same.
  const surface = contrast(code, bg) >= 1.05 ? hexOf(code) : mix(fg, bg, 0.08);
  const border = mix(fg, bg, 0.6);
  const text = hexOf(fg);
  const link = hexOf(accent);
  const faint = mix(fg, bg, 0.04);
  const palette = {
    // Inside the variables: mermaid reads the side from here, not the top level.
    darkMode: dark, background: page, fontFamily,
    primaryColor: surface, mainBkg: surface, primaryTextColor: text, textColor: text, titleColor: text,
    primaryBorderColor: border, nodeBorder: border, clusterBorder: border,
    secondaryColor: mix(accent, bg, 0.18), secondaryTextColor: text, secondaryBorderColor: border,
    tertiaryColor: faint, tertiaryTextColor: text, tertiaryBorderColor: border,
    clusterBkg: faint, lineColor: mix(fg, bg, 0.75), edgeLabelBackground: page,
    noteBkgColor: mix(accent, bg, 0.14), noteTextColor: text, noteBorderColor: mix(accent, bg, 0.6),
    ...each("pie", cats, 1), pieSectionTextColor: text, pieTitleTextColor: text, pieLegendTextColor: text,
    pieStrokeColor: border, pieOuterStrokeColor: border,
    ...each("git", cats.slice(0, 8)), ...each("gitBranchLabel", Array(8).fill(text)),
    ...each("cScale", cats), ...each("cScaleLabel", Array(12).fill(text)),
    ...each("fillType", cats.slice(0, 8)), ...each("venn", venn, 1),
    // What mermaid's base theme fixes at light values whatever the side.
    doneTaskBkgColor: mix(fg, bg, 0.2), doneTaskBorderColor: border, altSectionBkgColor: faint,
    excludeBkgColor: mix(fg, bg, 0.08), taskTextClickableColor: link, vertLineColor: link,
    archGroupBorderColor: border, faceColor: surface,
    emUiFill: surface, emSwimlaneBackgroundOdd: faint, emSwimlaneBackgroundStroke: border, emUiStroke: border,
    emProcessorFill: cats[0], emReadModelFill: cats[1], emCommandFill: cats[2], emEventFill: cats[3],
    emProcessorStroke: border, emReadModelStroke: border, emCommandStroke: border, emEventStroke: border,
    packet: { startByteColor: text, endByteColor: text, labelColor: text, titleColor: text, blockStrokeColor: border, blockFillColor: surface },
    // Whole, not just the palette: mermaid puts a nested object given here in
    // place of the one it derives, rather than merging the two.
    xyChart: {
      backgroundColor: page, titleColor: text, dataLabelColor: text, legendTextColor: text,
      xAxisTitleColor: text, xAxisLabelColor: text, xAxisTickColor: text, xAxisLineColor: text,
      yAxisTitleColor: text, yAxisLabelColor: text, yAxisTickColor: text, yAxisLineColor: text,
      plotColorPalette: cats.slice(0, 10).join(","),
    },
  };
  return { dark, fontFamily, palette, key: `t\0${JSON.stringify(palette)}` };
}
```

  Every name above was confirmed read by the 11.17.2 bundle in review (headless Edge, 2026-09-27; `emSwimlaneBackgroundEven` does not exist). The `xyChart` keys are exactly the ones `base` derives (`this.xyChart={backgroundColor:…,plotColorPalette:…}` in the bundle).

- [x] **Step 4: The config.** In `warmSources`' `config`, replace `theme: look.dark ? "dark" : "default",` and `fontFamily: look.fontFamily,` with:

```js
      fontFamily: look.fontFamily,
      // The theme's colours, or mermaid's own light or dark. With the theme's,
      // a diagram cannot re-colour itself wholesale — its own `theme`,
      // `themeVariables` and the like are locked below — though `classDef`,
      // `style` and a type's own colour options still colour it; with
      // mermaid's, it may, as on GitHub.
      ...(look.palette ? { theme: "base", themeVariables: look.palette } : { theme: look.dark ? "dark" : "default" }),
```

  and in `secure`, after `"flowchart",`: `...(look.palette ? ["theme", "themeVariables", "darkMode", "fontFamily", "themeCSS"] : []),`. Update the `secure` comment: "…plus, with the theme's colours, the five that would re-colour a diagram wholesale."

- [x] **Step 5: Setting plumbing** in `app.js`, after the open-mode functions:

```js
/* ---------------- diagram colours ---------------- */

/**
 * Reflect the choice and redraw what it changes. Also called when another
 * window changes it, so it must not re-broadcast.
 */
function showDiagramColours(colours) {
  for (const radio of els.diagramColourRadios) radio.checked = radio.value === colours;
  if (colours === state.diagramColours) return;
  // Blocks left as code count too: a diagram refused in one mode can draw in the other.
  const drawn = !els.content.hidden && (diagramFigures().length > 0 || els.content.querySelector(MERMAID_BLOCK));
  state.diagramColours = colours;
  if (!drawn) return;
  // The overlay's copy is of the old drawing.
  els.diagramDialog.close();
  redrawDiagrams().catch(console.error);
}
```

  Radio listeners next to `els.modeRadios`': `if (radio.checked) { showDiagramColours(radio.value); invoke("set_diagram_colours", { colours: radio.value }).catch(console.error); }`. At boot, right after `showReopen(…)` (before `applyTheme`, and before any document can open): `showDiagramColours(settings.diagram_colours === "mermaid" ? "mermaid" : "theme");`. And `await listen("diagram-colours-changed", (e) => showDiagramColours(e.payload));` next to `open-mode-changed`.

- [x] **Step 6: Verify** (per `drive-app`; `node --check`, `cargo test` done):
  - (a) Default: a fresh launch reads `state.diagramColours === "theme"`; "Diagrams follow the theme" is checked. Under **each of the 15 bundled themes**: kitchen-sink's first flowchart node's computed `fill`, put through the same canvas, equals `palette.mainBkg`; every string value except `fontFamily` and `plotColorPalette`, and every value in `packet`/`xyChart` except `plotColorPalette`, matches `/^#[0-9a-f]{6}$/`, and `plotColorPalette` is ten of them joined by commas; for every colour `c` in `pie*`, `git*`, `cScale*`, `fillType*`, `emProcessorFill`, `emReadModelFill`, `emCommandFill`, `emEventFill` (not `venn*`: never behind text), `contrast(text, c) >= (contrast(text, page) >= 5 ? 4.5 : 3) - 0.01` (compute in the page from the palette). Report the lowest ratio per theme, and for both Solarized themes the lowest `contrast(page, c)`.
  - (b) `await applyTheme('dracula')`, then `await applyTheme('github-dark')`: the figure redraws (new svg node; `diagramBlocks.get(fig).look` changes).
  - (c) Screenshots of `all-types.md` in `github-dark`, `dracula`, `github-light` and `solarized-light`, saved as `scratchpad/theme-<theme>-<section>.png`. The controller views every one: no text unreadable on its fill, slices/branches/sections tell apart, no near-white box on a dark page (gantt done and alternate sections, architecture groups, event modeling lanes and boxes, packet blocks, xychart bars, journey faces, ER rows). Known and accepted: C4 relationship labels (mermaid's fixed `#444444`), author-chosen `classDef`/`style`/`box` colours, both Solarized themes' category labels at ~3:1. Any other failure is fixed by an explicit value in the palette's hard-coded group, and re-shot.
  - (d) Locking and escape, with Task 1's `escape.md` plus: a diagram with `%%{init: {"themeVariables": {"mainBkg": "#ff0000"}}}%%`, and one with `classDef red fill:#f00` + `class A red`.
    - "Follow the theme": the `mainBkg` diagram is not red; the `classDef` node is red; the `escape.md` diagrams **draw** (their `themeVariables`/`themeCSS` are dropped) and the page is unchanged.
    - "Mermaid's own" (through Settings): the `mainBkg` diagram is red; the `classDef` node is red; the `escape.md` diagrams are refused with Task 1's message and the page is unchanged.
    - Back to "Follow the theme" with `escape.md` open: its blocks, left as code, now draw (the switch redraws a page with only refused blocks).
  - (e) Mode switch: choosing "Mermaid's own" redraws to mermaid's dark/default (node fill no longer the palette's); a second window (Ctrl+N, a diagram document) follows via the broadcast; `config.json` reads `"diagram_colours": "mermaid"` (read only). Set it back to "Diagrams follow the theme" through the app; both windows follow; config reads `"theme"`.
  - (f) The OS light/dark listener, the overlay closing on a theme change, and the click-redraw in `openDiagram` still work (flip with a see-through test theme appended to `els.themeStyle`; theme change with the overlay open; drift `document.body.style.background`, then click twice).
  - (g) No console errors; `initialize` never throws in any bundled theme (open kitchen-sink under each of the 15).

- [x] **Step 7: Commit** Rust + `index.html` + `base.css` + `app.js` — subject `Draw diagrams in the theme's colours, with a Settings choice to keep Mermaid's own`.

---

### Task 6: Docs

**Files:**
- Modify: `README.md` (Mermaid feature bullet, design notes, the `src/` layout block at ~582-586), `src-tauri/themes/README.md`, `docs/open-items.md`, `docs/closed-items.md`
- Move: `docs/plans/mermaid-pipeline.md` → `docs/archive/plans/mermaid-pipeline.md` (plain `mv`), with an `## Outcome` section.

- [x] **Step 1: README.** The Mermaid feature bullet mentions themed colours and the Settings choice. The design notes: mermaid runs in a sandboxed, origin-less frame (why: it builds each drawing live in its document before returning the string); every drawing's styles must stay scoped under it or it is refused (why: the quote escape); ids are namespaced under the drawing's own; the look key includes the palette; with "Follow the theme" a diagram's `theme`/`themeVariables`/`darkMode`/`fontFamily`/`themeCSS` are locked, with "Mermaid's own" not; `classDef`/`style` and a type's own colour options always work. Re-check the "What a file can style" paragraph and correct it. Add `diagram-frame.js` to the `src/` layout block.
- [x] **Step 2: Themes README.** One paragraph: under "Follow the theme", diagrams take their colours from the page background, the prose text colour, the code-block background and the link colour, as the theme styles `.markdown-body`; category colours are rotated from the link colour; keep text readable on the code background.
- [x] **Step 3: Items.**
  - Move the five Deferred entries (theme colours; unnamespaced ids; the overlay id rewrite; the cache; the foreignObject temporary render) to closed-items `## Deferred`, ticked, each `*Fixed <hash>, <date> (\`archive/plans/mermaid-pipeline.md\`): …*`.
  - Add a closed entry under closed-items `## Bugs` for the style escape: found in this plan's review, live in 1.7.0, fixed by Task 1's commit.
  - Rewrite "A Mermaid render that never settles" (`open-items.md:130`): with the frame, a stuck render can be abandoned by removing and rebuilding the frame on a timeout; a frame lost after `ready` would leave renders waiting. Keep it deferred; update its fix and reopen trigger.
  - New open item under `## Deferred`: *Gantt `click … href` links do nothing* — mermaid wires them through `bindFunctions`, which the app never calls (found in this review). Reopen when someone uses one.
  - Amend the existing "Mermaid diagrams on macOS and Linux" item (`open-items.md:64-80`, under *Needs a Mac or a Linux install*): diagrams now draw in a sandboxed frame, which must be checked on macOS 13 (WKWebView); Linux proven by Task 4 Step 5 (g) (record the date and what ran). No new First-runs entry.
  - Accepted limits: the frame's answer is trusted (CSP backstop); a swimlane group named like another figure's root keeps its raw id; `@keyframes` in a diagram's `themeCSS` stay global; C4 relationship labels are mermaid's fixed `#444444` on every theme; author-chosen diagram fills are not adjusted to the theme; both Solarized themes' category labels read at ~3:1.
  - `grep -rn "loadMermaid\|empties itself\|temporary render\|temporary in-page" docs README.md --exclude-dir=archive --exclude=closed-items.md --exclude=mermaid-pipeline.md` finds nothing stale.
- [x] **Step 4:** Tick this plan's boxes, add `## Outcome` (commits, measurements, the bundle names found in Task 5 Step 3, anything found; and that between Tasks 1 and 4 an escaping `<style>` was still live during mermaid's in-page render — the reason Task 4 exists), `mv` it to `docs/archive/plans/`.
- [x] **Step 5: Commit** `docs: close the Mermaid pipeline items, and file their plan`.
