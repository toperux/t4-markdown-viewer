# Mermaid Follow-ups Implementation Plan (Plan A.1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **Owner rule: never decide on your own.** Any ruling, skipped finding, accepted limit or parked item goes to the owner before it is acted on — this overrides the skill's "rulings, not stalls".

**Goal:** Finish Plan A (`archive/plans/mermaid-pipeline.md`) with the follow-ups the owner chose on 2026-09-27: tidy names and comments, safer id renaming, a palette probe that sees `#content` rules, numbered ids for repeated diagrams, a per-render timeout, working gantt `click` links and sequence-diagram link menus.

**Architecture:** All in the diagram path of `src/app.js` and the bridge `src/diagram-frame.js`, plus two lines of `src/base.css` and docs. No Rust.

**Tech Stack:** plain JS (no bundler, no JS tests), mermaid 11.17.2 in a sandboxed frame. The `drive-app` skill is the only frontend check.

**Spec:** the owner's answers, 2026-09-27 (after Plan A, and on this plan's review round 1):
- Tidy: the style check's comment wording and long line; the draw-time rename comment and the frame's `inert` comment as full sentences; the href test reads in natural order; the module-level palette `probe` renamed `themeProbe`; the shadowed `id` parameter in the draw-time `renameIds` callback renamed; README.md (~524, ~530) re-wrapped; open-items' gantt item wrapped where it lands (Task 6).
- `url(#…)` rewriting skips human-readable attributes: `aria-*` (except `aria-labelledby`/`aria-describedby`, which are id lists), `data-*`, `title`.
- The palette probe is appended **inside `#content`** for the synchronous read and removed in the same tick, so a theme styling prose as `#content p` is seen.
- The same diagram twice in one document: the 2nd, 3rd… copy has its ids numbered **`<root>x2`, `<root>x3`** at insertion (mermaid never puts `x` right after the root, so no collision with e.g. a gantt task named `2`), reusing `renameIds`. Only the app's own figures.
- Per-render timeout **30 s**, message "The diagram took too long to draw." (cached). On timeout the frame is removed; **the rest of that document's diagrams go to a freshly built frame** and draw normally. A render sent to a frame that is gone rebuilds it the same way. WebKit (frame likely in the page's process, so a stuck render freezes the window and no timeout fires) → stays an open item.
- Gantt `click … href` links: the bridge sends mermaid's own sanitised `{taskId → url}` map back (gantts with `click` only); the page wraps each task's bar and label in an SVG `<a xlink:href>` for **every URL except `about:blank`** (mermaid's mark for an unsafe one) — the app's link handler decides the rest, as for a flowchart link. Gantt tasks mermaid marked `clickable` but left unlinked lose the class, so they don't look like links.
- Sequence `link`/`links` menus: reveal mermaid's own hidden popup when an actor's box (top and bottom) or lifeline is clicked; `database` and `queue` participants from the top box and lifeline only (their bottom box can't be matched). A click anywhere in the document, or Escape (when no dialog is open), closes it; a click on empty diagram area closes it and opens the full-window view as today. Its links go through the existing handler; hidden in the overlay copy; entries underlined.
- Recorded without code: click tooltips never show (open item); `link` on `actor`/`boundary`/`control`/`entity` crashes mermaid's menu drawing (accepted limit, wait for mermaid); `#` in a sequence link starts a comment (accepted limit, documented in README); `database`/`queue` bottom boxes don't open the menu (accepted limit).

**Measured (research and review, 2026-09-27, headless Edge on the vendored bundle with the app's CSP, mermaid in a sandboxed srcdoc frame; `scratchpad/research-a1.md`, `review-a1-mermaid.md`):**
- Gantt: tasks are `<rect id="Mermaid-N-t1" class="task … clickable">` and `<text id="Mermaid-N-t1-text" class="clickable taskText …">`; no URL in the SVG. `mermaid.mermaidAPI.getDiagramFromText(src)` returns a Promise; `.db.getLinks()` is a `Map`. For `click t1 href "#heading-b"`, `t2 "https://example.com"`, `t3 "notes.md"`, `t4,t5 "javascript:alert(1)"`, `t6 call fn()` the map is `[["t1","#heading-b"],["t2","https://example.com/"],["t3","notes.md"],["t4","about:blank"],["t5","about:blank"]]`. The extra parse leaves every SVG byte-identical. Wrapping keeps position, fill, z-order and the pointer cursor (mermaid's `#id .task.clickable`); the gantt stylesheet has no `>`, `+` or `~` rules.
- Render times (frame, under load): 600-node mindmap 5.8–5.9 s (worst), 440-node flowchart near `maxTextSize` 3.2–3.6 s. With sandboxed-iframe isolation (Edge default) a spinning frame doesn't lag the page, `frame.remove()` takes 0 ms, a new frame is ready in ~120 ms. Without isolation the page's thread freezes for good.
- On `window`, a capturing `message` listener registered **after** a bubbling one runs second and can't stop it; registered **first**, `stopImmediatePropagation` blocks the page's handler.
- Duplicates: Chromium draws every copy right. `numberRepeats` on three copies (one in a closed `<details>`): no duplicate ids, every `url()` resolves in its own copy; same after re-setting copies from the cache. With `-k` a gantt task named `2` collided (`Mermaid-N-2`) — hence `x`.
- Sequence menus: `<g id="actor{K}_popup" class="actorPopupMenu" display="none">`, a direct child of the root svg; `display` is the attribute (no CSS rule), so setting it to `block` shows the menu (0×0 → 150×80). Per link an `<a xlink:href>` around `<text class="actor">`. Participant top box: `g#root-{K}[data-et="participant"][data-id=<name>]` holding `rect.actor.actor-top[name]`; the lifeline `line#actor{K}.actor-line[name][data-et="life-line"][data-id=<name>]` sits in the actor's outer group; bottom box a plain `g` with `rect.actor.actor-bottom[name]`. `database`/`queue` are drawn with paths — no `rect.actor[name]` — but their top group and lifeline carry `data-et`/`data-id`. K is a global counter: match by name. `participant X@{ "type": "database" }` is the working syntax (bare `database X` is a parse error in 11.17.2).

## Outcome (2026-09-27)

Done, one commit per task: b7b344e (Task 1, tidy names, comments and wrapping; fix round
squashed in as 326a412), 58e53f4 (Task 2, `chore:` safer renaming, the `#content` probe and
numbered repeats; fix round squashed in as bd0f03a), 27d26e8 (Task 3, `chore:` the per-render
timeout; stop-comment fixup squashed in as 761b4bc), 910546d (Task 4, gantt `click` links; fix
round squashed in as 6e52d45), c4b4b89 (Task 5, sequence-diagram link menus). All on `main`,
none pushed.

**Owner's decisions during execution, in order** (full text in the ledger, `progress.md`):

- Task 1: the README re-wrap (~524–536) needed the owner to refill the paragraph at width 80,
  which the plan's own text could not settle unaided.
- Task 2: review round 1 raised six findings; the owner applied minors 1, 2, 3 and 6, and had 5
  redone (the probe appended straight into `#content`, no wrapper element). Minor 4, a second
  numbering pass being possible, was left — an efficiency nit, not a correctness one. The
  drive-app check's own script strips a raw `<details>` before it can be read; the owner accepted
  reading the script-moved copy instead of the plan's literal check.
- Task 3: Step 3 (c) measured that WebView2 does not isolate the sandboxed frame — no `iframe`
  target for `about:srcdoc`, so it runs in the page's own process. The owner had "A Mermaid render
  that never settles" widened rather than closed: the 30 s timeout rescues a frame that stops
  answering on every platform, but a render stuck in a loop still freezes the window on Windows
  (measured) and likely WebKitGTK and WKWebView (untested). Review's Minor 1 — a frame removed out
  from under a warm plus a failed reload caching "did not load" — was left, matching an earlier
  ruling on the same finding in Plan A's own review.
- Task 4: review round 1 raised two findings; the owner had both fixed (Minor 1: the link lookup
  pinned to element type — bar `rect`, label `text` — so a task named `t1-text` can't match t1's
  label; Minor 2: a neutral cursor for a link inside the full-window overlay copy, in CSS). The gantt-only rect/text wrapping ternary and
  the implementer's own why-comment were kept as written; a two-selector note in the CSS review
  needed no change.
- Task 5: review round 1 found nothing. The two boot-time `ipc.localhost` CSP console lines (seen
  since Task 2) count as known noise, as ruled for Tasks 2 through 4.
- Left as is from Plan A's review (owner's call): 16a (`@page`, the root-with-class refusal, the
  UTF-16 loop), 16b (ASCII-only selector ids), 16c (the plan's example).
- Final review: four findings kept as accepted limits (owner's call, 2026-09-27; see
  `docs/closed-items.md`): `renameIds` rewrites `url(#…)` text inside a link's own URL; the
  frame is trusted to send gantt links for any diagram, the same trust boundary as the SVG's
  own `<a>` links; `renameIds` leaves `aria-owns`/`aria-controls` alone, since mermaid doesn't
  emit them; the rightmost actor's popup can be clipped at the viewBox edge, mermaid's own
  layout. A sequence menu now opens at the click, a small gap below it, or above it where it
  would run past the drawing's bottom, so the clicked point stays clear; a click on the top box
  leaves it where mermaid put it. The render timeout was measured under a minimised window
  (WebView2 doesn't throttle it) and a timed-out diagram stays cached, with an open item for
  F5 to retry it. A gantt task named with its diagram's own root-prefixed id is an accepted
  limit (2026-09-28).

**Measured:** WebView2 gives the sandboxed diagram frame no `iframe` target of its own
(`Target.getTargets`), confirming it runs in the page's process rather than isolated. The two boot-time `ipc.localhost` CSP console
lines are unchanged noise across Tasks 2 through 5.

## Global Constraints

- mermaid stays **11.17.2**; no new dependencies.
- Commit straight to `main`; never push. Trailer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- `node --check src/app.js` and `node --check src/diagram-frame.js` (one file per command) before every commit that touches them.
- Driving the app: `.claude/skills/drive-app/SKILL.md`. **The owner keeps `"reopen": "off"` in `%APPDATA%\t4-markdown-viewer\config.json`; no agent writes `config.json`.** Back up and restore `session.json`; kill only the debug PIDs you launched (the owner's viewer shares the process name); never kill processes by command-line match; end with a plain `cargo build`. Any Settings choice a check changes is set back through the app.
- To see that a link would open externally without launching anything, use drive-app's recipe: wrap `window.chrome.webview.postMessage` and hold back (and record) messages whose command is `plugin:opener|open_url`. (`openUrl` in app.js is a `const` captured at load and can't be stubbed.)
- Fixtures in this session's scratchpad (`C:\Users\toper\AppData\Local\Temp\claude\f--src---pet-projects-t4-markdown-viewer\0c500661-cce1-4257-939d-10c9d8ffc8b1\scratchpad\`): `all-types.md` (35 diagram sections + `## Heading B`), `escape.md`. Scratch docs go there, never into the repo.
- Line numbers are the current file's (HEAD `4e93a57`); find code by the content quoted.
- Comments say why, in full sentences, like the surrounding code.
- **Anything a task can't do as written, or any choice it would need, goes back to the controller and on to the owner.** A check that must "bite first" is run on the build before the task's change; relaunch afterwards (backing up/restoring `session.json` around it).

## Review Focus

1. **Every diagram type still draws** after each task → each task's all-types check.
2. **A task link and a sequence menu link go where the app's other links go** (a `#heading` scrolls, a relative `.md` opens, `https` goes to the browser, `about:blank` does nothing) → Task 4 Step 4, Task 5 Step 4.
3. **A frame that stops answering no longer hangs every later diagram document**, and the rest of that document still draws → Task 3 Step 3.
4. **Clicking a diagram still opens the overlay**, except on an actor with links or inside an open menu → Task 5 Step 4.

---

### Task 1: Tidy names, comments and wrapping

**Files:** Modify `src/app.js`, `README.md`.

- [x] **Step 1: Comments and names** in `src/app.js`:
  - `stylesStayInside` (~496–499): replace the comment with
    ```js
    // Split on top-level commas only: `:is(a, b)` is one selector. A comma or
    // bracket that is escaped (`.a\(`) or inside a string (`[title="("]`) is
    // part of a name, so it neither splits nor nests — otherwise a selector
    // could hide a second, unscoped one behind it. A list whose brackets do not
    // balance is not trusted at all (null).
    ```
  - ~520: wrap `// The root itself, or inside it — not its siblings (\`~\`, \`+\`), and not \`#Mermaid-10\` for \`#Mermaid-1\`.` onto two lines at the file's width.
  - ~413: `frame.inert = true; // never focused or clicked into; its scripts still run` → the comment on its own line above: `// Never focused or clicked into; its scripts still run.`
  - ~815: `// Every id under the drawing's own, so a group named like part of the` → `// Every id goes under the drawing's own, so a group named like part of the` (rest unchanged).
  - `renameIds` (~696): `if (attr.localName === "href" && el.localName !== "a" && renamed.has(value.slice(1)) && value.startsWith("#"))` → `if (attr.localName === "href" && el.localName !== "a" && value.startsWith("#") && renamed.has(value.slice(1)))`.
  - Rename the module-level `let probe = null;` (~552) and its uses in `diagramLook` to `themeProbe`; its doc comment stays.
  - ~818: `renameIds(root, (id) => (id.startsWith(root.id) ? null : \`${root.id}-${id}\`));` → `renameIds(root, (old) => (old.startsWith(root.id) ? null : \`${root.id}-${old}\`));`.
- [x] **Step 2: Wrapping.** Re-wrap README.md ~524 and ~530 to the width of the lines around them. Words unchanged.
- [x] **Step 3: Verify:** `node --check src/app.js`; `grep -nw "probe" src/app.js` finds only `stylesStayInside`'s local; `git diff --word-diff README.md | grep -E '\[-|\{\+'` prints nothing. `cargo build`; open `all-types.md` under github-dark: 35 drawn / 0 refused.
- [x] **Step 4: Commit** — subject `chore: tidy the diagram pipeline's comments and names` (`src/app.js`, `README.md`).

---

### Task 2: Safer renaming, a probe that sees `#content`, numbered repeats

**Files:** Modify `src/app.js` — `renameIds`, `diagramLook`, `renderDiagrams`, `redrawDiagrams`.

**Interfaces:** Produces `numberRepeats(root: Element): void` — for each of the app's own figures under `root` (in document order) whose svg id was already seen, renames that figure's ids from `<svgId>…` to `<svgId>x<k>…` (k = 2, 3, …) through `renameIds`.

- [x] **Step 1: Skip text attributes in `renameIds`.** In the attribute loop, as the loop's first statement (before `let value = attr.value.replace(url, toUrl);`), add:
  ```js
      // Words, not references: a label or title that mentions `url(#…)` reads
      // the same. The two aria id lists are references, and are renamed below.
      const words = attr.name === "title" || attr.name.startsWith("data-") ||
        (attr.name.startsWith("aria-") && attr.name !== "aria-labelledby" && attr.name !== "aria-describedby");
      if (words) continue;
  ```
  (`continue` skips only that attribute; `href` and the id lists are never in the skipped set.) Update `renameIds`' doc comment: "`url(#…)` in any attribute but a human-readable one (`title`, `data-*`, `aria-*` other than the id lists)".
- [x] **Step 2: Probe inside `#content`.** In `diagramLook`, replace the creation block — `if (!probe) { … document.body.append(probe); }` (after Task 1: `themeProbe`) — so it no longer appends, and read inside `#content`:
  ```js
    if (!themeProbe) {
      themeProbe = document.createElement("div");
      themeProbe.className = "markdown-body";
      themeProbe.setAttribute("aria-hidden", "true");
      themeProbe.style.cssText = "position:absolute;left:-10000px;top:0;visibility:hidden";
      themeProbe.innerHTML = '<p>x</p><pre><code>x</code></pre><p><a href="#">x</a></p>';
    }
  ```
  and replace the three reads (`const fg = over(…)`, `const code = …`, `const accent = …`) with:
  ```js
    // Inside the document itself, for this read only, so a theme's `#content p`
    // colours it as well as its `.markdown-body p`; out again before anything
    // else can run and find it there.
    els.content.append(themeProbe);
    let fg, code, accent;
    try {
      fg = over(themeProbe.querySelector("p"), "color");
      code = over(themeProbe.querySelector("pre"), "backgroundColor");
      accent = over(themeProbe.querySelector("a"), "color");
    } finally {
      themeProbe.remove();
    }
  ```
  Update `themeProbe`'s doc comment: "Hidden stand-ins for prose, code and a link, put into the document for each read so the theme styles them like its own."
- [x] **Step 3: Number repeated diagrams.** Add after `renderDiagrams`:
  ```js
  /**
   * The same diagram twice in a document is the same drawing twice, ids and
   * all. Each copy after the first gets its own, numbered under the first's —
   * `Mermaid-3x2`, `Mermaid-3x3`; mermaid never puts an `x` straight after the
   * root, so no id of its own can match — and every reference inside a copy
   * then finds that copy, on engines that do not paint a reference into a
   * hidden twin.
   */
  function numberRepeats(root) {
    const seen = new Map();
    for (const fig of root.querySelectorAll(".mermaid-diagram")) {
      // Only our own figures: a diagram's nodes can wear the class too.
      if (!diagramBlocks.has(fig)) continue;
      const svg = fig.querySelector(":scope > svg");
      if (!svg) continue;
      // Counted by the first copy's id, whether this copy is fresh from the
      // cache or already numbered, so a copy re-set on its own gets its own
      // number back rather than a sibling's.
      const id = svg.id;
      const base = id.replace(/x\d+$/, "");
      const k = (seen.get(base) ?? 0) + 1;
      seen.set(base, k);
      const want = k > 1 ? `${base}x${k}` : base;
      if (id !== want) renameIds(svg, (old) => (old.startsWith(id) ? `${want}${old.slice(id.length)}` : null));
    }
  }
  ```
  Call `numberRepeats(root)` at the end of `renderDiagrams` (after the `forEach`), and `numberRepeats(els.content)` in `redrawDiagrams` right after the `for (const fig of figs)` loop (before `closeStaleDiagram()`). (A root id is always `Mermaid-<digits>`, so `x<digits>` at its end is only ever this numbering.)
- [x] **Step 4: Verify** (per `drive-app`):
  - (a) `all-types.md` under github-dark and github-light: 35 drawn / 0 refused; screenshot 3 sections and compare with `theme-<theme>-<section>.png` (same look).
  - (b) Text attributes: in the page, `const t = document.createElement('template'); t.innerHTML = '<svg id="R"><g id="x" aria-label="url(#x)" title="url(#x)" data-n="url(#x)" fill="url(#x)" aria-labelledby="x"></g></svg>'; renameIds(t.content.firstChild, (o) => o === 'x' ? 'R-x' : null);` → `aria-label`, `title`, `data-n` still `url(#x)`; `fill` is `url(#R-x)`; `aria-labelledby` is `R-x`.
  - (c) Probe: add a `<style>` to `document.head` with `#content p { color: #ff0000 }` (under "Diagrams follow the theme"), then `diagramLook().palette.textColor === "#ff0000"` and `!themeProbe.isConnected`; remove the `<style>` after. **Bite first:** the same on the build before this task gives a textColor other than `#ff0000`.
  - (d) Repeats: a scratch `dup.md` with the same flowchart (with arrows) three times, the second inside a `<details>`: svg ids `Mermaid-N`, `Mermaid-Nx2`, `Mermaid-Nx3`; no duplicate id inside `#content` (`new Set(ids).size === ids.length`); every `url(#…)` in each copy resolves inside that copy's own svg. Switch theme (github-dark → dracula): still no duplicates. Open the overlay on the third copy: its ids start with `Mermaid-Nx3-zoom`. A gantt with a task named `2`, twice: no duplicate ids. Re-set only the third copy from the cache (`const fig = diagramFigures()[2]; fig.innerHTML = diagrams.get(diagramKey(diagramLook(), diagramBlocks.get(fig).pre.textContent)).svg;`) and call `numberRepeats(els.content)`: ids are again `Mermaid-N`, `Mermaid-Nx2`, `Mermaid-Nx3`, no duplicates.
  - (e) No console errors.
- [x] **Step 5: Commit** — subject `chore: keep a repeated diagram's ids its own, and read theme colours from inside the document`.

---

### Task 3: A render that never answers gives up after 30 s

**Files:** Modify `src/app.js` — `loadRenderer`.

- [x] **Step 1: Shell and `stop`.** Restructure `loadRenderer` so a stop can tell whether its load is still the current one (necessary: `stop` may run after a newer load has replaced it):
  ```js
  let rendererLoad = null;
  function loadRenderer() {
    if (rendererLoad) return rendererLoad;
    const load = new Promise((resolve, reject) => {
      // … the existing body, with the additions below …
    });
    rendererLoad = load;
    return load;
  }
  ```
  Inside, next to `let n = 0;`: `const RENDER_TIMEOUT = 30000; // ~5× the slowest real diagram measured (a 600-node mindmap, ~6 s under load).` After `fail`, add:
  ```js
      // A frame that stopped answering — lost, or stuck in a render — would
      // hold the queue, and with it every diagram document after this one. Give
      // up on what it owes and drop it: whatever is drawn next gets a fresh
      // frame. Where the frame runs in the page's own process (WebKit), a stuck
      // render freezes the page too, and nothing here gets to run.
      const stop = () => {
        removeEventListener("message", onMessage);
        frame.remove();
        if (rendererLoad === load) rendererLoad = null;
        for (const settle of waiting.values()) settle({ error: "The diagram took too long to draw." });
        waiting.clear();
      };
  ```
  (`stop` only runs from a timer or a later `render`, after `load` is assigned.)
- [x] **Step 2: `render`.** Replace the `render` arrow with:
  ```js
      const render = (config, id, source) => {
        // Given up on, or taken away: what is still to draw goes to a fresh frame.
        if (!frame.isConnected) {
          stop();
          return loadRenderer().then((next) => next(config, id, source));
        }
        return new Promise((settle) => {
          const mine = ++n;
          const timer = setTimeout(stop, RENDER_TIMEOUT);
          waiting.set(mine, (result) => (clearTimeout(timer), settle(result)));
          // The width goes with the request: the frame may run in a process of
          // its own, where a resize could land after the render.
          frame.contentWindow.postMessage({ n: mine, config, id, source, width: document.body.offsetWidth }, "*");
        });
      };
  ```
  In `fail`, change `rendererLoad = null; // the next document tries again` to `if (rendererLoad === load) rendererLoad = null; // the next document tries again`, matching `stop`.
- [x] **Step 2b: A fresh frame that can't load ends the pass, as a first load does.** In `warmSources`, move `const render = await loadRenderer();` from before the `for` loop into it, as the loop's first statement after the `stale()` check and **before** the `try` — so a load failure (first or after a timeout) leaves the run through the queue's catch, caching nothing, and the next document tries again. `const config = { … }` stays where it is, before the loop (it doesn't depend on `render`). Comment, wrapped to the file's width:
  ```js
      // Each time, not once: after a frame is given up on, the next diagram
      // needs the fresh one — and a renderer that will not load stops the pass
      // here, caching nothing, as it always has.
  ```
- [x] **Step 3: Verify** (per `drive-app`):
  - (a) `all-types.md`: 35 drawn / 0 refused.
  - (b) A frame that stops answering. In a fresh window, **before any diagram document**, register a capturing listener first (registered first, it runs before the page's handler; registered later, it would run after and block nothing):
    ```js
    window.__block = 0;
    addEventListener("message", (e) => {
      if (__block > 0 && e.data && typeof e.data === "object" && "n" in e.data && e.source === document.querySelector("iframe[sandbox]")?.contentWindow)
        __block--, e.stopImmediatePropagation();
    }, true);
    ```
    Open a diagram document (frame ready). Set `__block = 1` and open a scratch document with **two** diagrams not yet cached: after ~30 s the first shows "The diagram took too long to draw.", the second draws (from a fresh frame: a new `iframe[sandbox]`, the old one gone). Report the timings. **Bite first** on the build before this task: the same document never finishes (stop after ~60 s, relaunch).
  - (b2) A frame removed from under a warm: `document.querySelector('iframe[sandbox]').remove()`, then open a new diagram document → it draws, from a fresh frame.
  - (b3) A fresh frame that can't load: with the check (b) listener in place, set `__block = 1`, apply Plan A Task 4 Step 5 (d)'s `srcdoc` override with `__from = 'diagram-frame.js'` (so the next frame never answers), open a scratch document with three new diagrams → the first shows "took too long", the other two stay as code with no error under them, exactly one new frame was attempted (count `iframe[sandbox]` insertions), and `diagrams` holds no "did not load" entry. Restore the override; open the document again → the two draw, and the first still shows its cached "took too long".
  - (c) Does WebView2 isolate the frame? Via CDP `Target.getTargets` (or `Target.setDiscoverTargets`): is there an `iframe` target for `about:srcdoc`? Report yes/no — it decides whether the timeout can protect a stuck render on Windows.
  - (d) Renderer-load failure paths (Plan A Task 4 Step 5 (d), both variants) still fail fast.
  - (e) No console errors beyond the expected.
- [x] **Step 4: Commit** — subject `chore: give up on a diagram the renderer never answers, and start it afresh`.

---

### Task 4: Gantt `click` links work

**Files:** Modify `src/diagram-frame.js`, `src/app.js` (`loadRenderer`'s `onMessage`, `warmSources`).

- [x] **Step 1: Bridge.** In `src/diagram-frame.js`, replace the success-path `parent.postMessage({ n, svg, diagramType }, "*");` with:
  ```js
      // A gantt's `click … href` links are not in its drawing: mermaid wires
      // them to the page it drew in, which is gone by the time the app has the
      // drawing. Its own map of them — URLs already made safe — goes back too.
      // A failure to read it costs the links, not the drawing.
      let links = [];
      if (diagramType === "gantt" && /\bclick\b/.test(source))
        try {
          links = [...(await mermaid.mermaidAPI.getDiagramFromText(source)).db.getLinks()];
        } catch {}
      parent.postMessage({ n, svg, diagramType, links }, "*");
  ```
- [x] **Step 2: Page receives it.** In `onMessage`, replace the last `settle(…)` line with:
  ```js
      // Pairs of strings only, for the same reason.
      const links = Array.isArray(e.data.links)
        ? e.data.links.filter((p) => Array.isArray(p) && typeof p[0] === "string" && typeof p[1] === "string")
        : [];
      settle(typeof svg === "string" ? { svg, diagramType: String(diagramType), links } : { error: String(error ?? "The diagram did not draw.") });
  ```
- [x] **Step 3: Wrap the tasks.** In `warmSources`, after `renameIds(root, …)` and before `done = { svg: root.outerHTML }`:
  ```js
        // A task with a link becomes one: its bar and its label each wrapped in
        // the kind of link a flowchart's `click` makes, which the page already
        // follows. Not `about:blank`: that is mermaid's mark for a URL it
        // would not trust, and there is nothing to follow.
        for (const [task, url] of drawn.links ?? []) {
          if (url === "about:blank") continue;
          for (const suffix of ["", "-text"]) {
            const el = root.querySelector(`#${CSS.escape(`${root.id}-${task}${suffix}`)}`);
            if (!el) continue;
            const a = document.createElementNS("http://www.w3.org/2000/svg", "a");
            a.setAttributeNS("http://www.w3.org/1999/xlink", "xlink:href", url);
            el.replaceWith(a);
            a.append(el);
          }
        }
        // A task mermaid marked as a link but that got none — an unsafe URL,
        // or a `call` — should not look like one.
        if (diagramType === "gantt")
          for (const el of root.querySelectorAll(".clickable")) if (!el.closest("a")) el.classList.remove("clickable");
  ```
- [x] **Step 4: Verify** (per `drive-app`), scratch `gantt-links.md`: a gantt with tasks t1–t6 and `click t1 href "#heading-b"`, `click t2 href "https://example.com"`, `click t3 href "notes.md"`, `click t4,t5 href "javascript:alert(1)"`, `click t6 call fn()`; then `## Heading B`, and a scratch `notes.md` beside it.
  - (a) t1 bar and label are each inside an `<a>` with `#heading-b`; clicking either scrolls to Heading B.
  - (b) t2 → an `open_url` for `https://example.com/` is recorded (drive-app's hold; nothing launched).
  - (c) t3 opens `notes.md` in place.
  - (d) t4, t5, t6: no `<a>`, no `clickable` class, default cursor, and each label is neither bold nor link-coloured (computed `font-weight` and `fill` equal an ordinary task label's); clicking them opens the overlay. t1–t3 labels stay bold and link-coloured.
  - (e) all-types gantt section unchanged; 35 drawn / 0 refused; no console errors.
- [x] **Step 5: Commit** — subject `Follow a gantt chart's click links`.

---

### Task 5: Sequence-diagram link menus open

**Files:** Modify `src/app.js` (`onLinkClick`, `openDiagram`, `onKeydown`, new helpers), `src/base.css`.

**Interfaces:** Produces `actorMenu(fig: Element, el: Element): Element | null` and `closeActorMenus(except?: Element): boolean` (true when one was open).

- [x] **Step 1: Helpers**, after `figureOf`:
  ```js
  /*
   * A sequence diagram's `link`/`links` give an actor a menu, which mermaid
   * draws hidden and opens with a handler the app's strict mode strips. So the
   * page opens it: a click on the actor's box or lifeline shows it; a click
   * anywhere else in the document, or Escape, puts it away.
   */

  /** The link menu of the actor whose box or lifeline `el` is in, within `fig`, or null. */
  function actorMenu(fig, el) {
    // The top box's group and every lifeline name their actor; a bottom box
    // only on its rect, which a database or queue, drawn in paths, has not.
    const name =
      el.closest('g[data-et="participant"], line[data-et="life-line"]')?.getAttribute("data-id") ??
      el.closest("g")?.querySelector("rect.actor[name]")?.getAttribute("name");
    if (!name) return null;
    // By name, not number: mermaid numbers actors from a count that runs on
    // across diagrams.
    const line = [...fig.querySelectorAll("line.actor-line[name]")].find((l) => l.getAttribute("name") === name);
    return line ? fig.querySelector(`#${CSS.escape(`${line.id}_popup`)}`) : null;
  }

  /** Put away every open actor menu but `except`; whether one was open. */
  function closeActorMenus(except) {
    let closed = false;
    for (const menu of els.content.querySelectorAll('.actorPopupMenu[display="block"]'))
      if (menu !== except) menu.setAttribute("display", "none"), (closed = true);
    return closed;
  }
  ```
- [x] **Step 2: Clicks.** In `onLinkClick`, replace the end of the `!a` branch — `const fig = figureOf(event.target); if (fig) openDiagram(fig); return;` — with:
  ```js
      // Inside an open menu but not on one of its links: nothing to do.
      if (event.target.closest(".actorPopupMenu")) return;
      const fig = figureOf(event.target);
      const menu = fig && actorMenu(fig, event.target);
      closeActorMenus(menu);
      if (menu) {
        menu.setAttribute("display", menu.getAttribute("display") === "block" ? "none" : "block");
        return;
      }
      if (fig) openDiagram(fig);
      return;
  ```
  (the `img` handling above it unchanged), and right after the `!a` branch's closing brace: `closeActorMenus(); // a link is being followed, a menu's own or not`.
  In `onKeydown`, right **after** the modal-dialog guard (`if (els.settings.open || els.updateDialog.open || els.diagramDialog.open) { … return; }`) and before the open-menu Escape: `if (event.key === "Escape" && closeActorMenus()) return;` — so Escape over an open dialog closes only the dialog.
- [x] **Step 3: Overlay and look.** First run check Step 4 (e) with the bite-first half. Then in `openDiagram`, after `t.innerHTML = fig.innerHTML;`: `t.content.querySelectorAll(".actorPopupMenu").forEach((m) => m.setAttribute("display", "none"));` with a comment (the copy is for reading, and its links are not followed there). In `base.css`, next to `.markdown-body div.mermaid-diagram`: `.markdown-body div.mermaid-diagram .actorPopupMenu a text { text-decoration: underline; }` with a comment (entries otherwise read as actor names).
- [x] **Step 4: Verify** (per `drive-app`), scratch `seq-links.md`: a sequence diagram with `participant Alice`, `participant Bob`, `participant Dave`, `participant Db@{ "type": "database" }`, `link Alice: Repo @ https://example.com`, `link Alice: Notes @ notes.md`, `links Bob: {"Site": "https://example.org"}`, `link Db: Docs @ https://example.net`, messages between them; then a second copy of the same diagram (numbered by Task 2); `notes.md` beside it.
  - (a) Click Alice's top box → her menu shows (`display="block"`); again → hides. Her bottom box → same menu. Her lifeline → same menu.
  - (b) Menu open, click Bob's box → Alice's closes, Bob's opens. Click empty diagram area → menus close **and** the overlay opens. Click in the document text → menus close. Escape → closes, nothing else happens.
  - (c) Click "Notes" → `notes.md` opens in place; "Repo" → an `open_url` is recorded (drive-app's hold; nothing launched).
  - (d) Click on the menu panel between entries → nothing (no overlay).
  - (e) Overlay copy: open Alice's menu, then call `openDiagram(fig)` over CDP → the copy's menus all have `display="none"`. **Bite first:** before adding Step 3's line, the copy's Alice menu is `block`.
  - (f) Dave (no links): clicking his box opens the overlay.
  - (g) Db: top box and lifeline open its menu; bottom box opens the overlay (accepted limit).
  - (h) The second copy's menus work independently (ids `…x2…`).
  - (i) Settings open over an open menu, Escape → Settings closes, the menu stays open; a second Escape closes the menu.
  - (j) An `actor Carol` (stick figure) with a `link` → the diagram stays as code with mermaid's error (accepted limit).
  - (k) all-types: 35 drawn / 0 refused; no console errors.
- [x] **Step 5: Commit** — subject `Open a sequence diagram's link menus`.

---

### Task 6: Docs

**Files:** `README.md`, `docs/open-items.md`, `docs/closed-items.md`; move this plan to `docs/archive/plans/mermaid-follow-ups.md` with an `## Outcome`.

- [x] **Step 1: README** Mermaid notes: gantt `click` links and sequence-diagram link menus work (menus open on the actor's box or lifeline; for `database`/`queue` the top box or lifeline; `#heading` targets can't be written in a sequence link, since `#` starts a comment there — use a relative file or a web address); a per-render timeout; repeated diagrams keep their own ids.
- [x] **Step 2: Items.**
  - Close "A Gantt `click … href` link does nothing" (open-items ~167) into closed-items `## Deferred` (Fixed <hash>, date, `archive/plans/mermaid-follow-ups.md`), wrapped to the file's width.
  - "A Mermaid render that never settles": Task 3 Step 3 (c) found WebView2 does **not** isolate the frame (no iframe target; it runs in the page's process). Owner (2026-09-27): record it and widen the open item — the 30 s timeout now rescues a frame that stops answering on every platform, but a render stuck in a loop still freezes the window on Windows (measured) and likely WebKitGTK/WKWebView. Reopen trigger: a real diagram freezing any of them.
  - New open item: *`click` tooltips never show* (flowchart/class/state `click A "tip"`; mermaid attaches them in `bindFunctions`, which only the frame could run). Reopen when someone uses one.
  - Accepted limits: `link`/`links` on `actor`/`boundary`/`control`/`entity` makes mermaid fail the whole diagram (a mermaid bug; revisit on a mermaid bump); `#` in a sequence link starts a comment, so a menu can't link to a heading; a `database`/`queue` participant's bottom box doesn't open its menu.
  - Outcome: record which of Plan A's review findings the owner left as is (16a `@page`, root-with-class refusal, UTF-16 loop; 16b ASCII-only selector ids; 16c the plan's example).
- [x] **Step 3:** Tick boxes, add `## Outcome`, `mv` to `docs/archive/plans/`. Commit — `docs: close the Mermaid follow-ups, and file their plan`.
