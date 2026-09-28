# Viewer follow-ups (Plan B) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close four deferred items in `docs/open-items.md`:
- GitHub's link order in `docTarget` (#8);
- links on a page that keep working while a save redraws it (#9);
- one zoom-and-pan core for the picture tab and the diagram overlay (#7);
- diagrams and pictures that open from the keyboard (#2).

**Architecture:** Every change is in `src/app.js`, plus one CSS rule in `src/base.css`. The four are independent. They share only `onLinkClick` and `onKeydown`:
- in `onLinkClick`, #9 edits the link gate, and #2 reuses the click path through `el.click()` and adds a blur for mouse clicks;
- in `onKeydown`, #2 adds Enter/Space. No Rust changes.

**Tech Stack:** Tauri 2. The frontend is plain JS with no bundler and no JS tests. Verification is by driving the app (`.claude/skills/drive-app/SKILL.md`, CDP).

**Spec:** the owner's answers of 2026-09-28, below.

## Spec

- **#8 `docTarget` order (both fixes).**
  1. A name starting `fn-` or `fnref-` tries the bare footnote in the document first.
  2. A name already carrying `user-content-` is looked up as written first, which is GitHub's order: `#user-content-x` lands on `## X`, not on `## User content X`.
  - Every other name keeps today's order: prefixed, as written, then stripped.
- **#9 links during a refresh (same-page refresh).**
  - While the page on screen is the active entry's own, every link acts on it. That is the case during a save or an F5 in flight.
  - A navigating click (a document link, an in-page anchor) outdates the pending render, and the page is drawn for the new entry.
  - During a switch to another tab or document, links stay dead, as today.
- **#7 shared zoom and pan (core only).**
  - Share zoom-about-a-point and the drag-to-pan pointer handlers, parameterised on the view, the element and the state.
  - Wheel, double-click, toolbar and keys stay per viewer.
  - Both viewers behave exactly as before, which is checked by measuring before and after.
- **#2 keyboard (focusable plus Enter/Space).**
  - Diagrams (`.mermaid-diagram` figures) and pictures (`img[data-file]` not inside a link) get `tabindex="0"` and a visible focus ring.
  - Enter or Space opens them as a click would.
  - A diagram gets `role="group"` with `aria-label="Diagram — Enter opens it full window"`. A button's children are presentational, so role=button would hide the drawing's own links from a screen reader.
  - A picture gets `role="button"`; its `alt` is its name, or its file name when the alt is empty.
  - Enter/Space act only on a keyboard focus (`:focus-visible`).
  - Any mouse press on a diagram or picture lets go of the focus it gave, so a later key can't turn it into a keyboard focus. That covers a click (to open the overlay, or an actor's menu), a right or middle button, and a press dragged out. A mousedown lets go a task later, and a click lets go at once, for a fast click that beats the task.
  - Closing the overlay returns focus to the diagram it was opened from by keyboard. The dialog does this natively (measured). When a save replaced that diagram while the overlay was open, focus goes to its successor by index and source.
  - A diagram or picture with keyboard focus gets it back after a save re-renders the page, by its index among the page's openable elements, the same way a ticked checkbox does.
  - Out of scope, and they stay in the open item: keeping focus on a link inside a diagram across a light/dark redraw, and keyboard access to sequence actor menus.

## Global Constraints

- mermaid stays **11.17.2**. No new dependencies.
- Commit straight to `main`. Never push. Trailer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Run `node --check src/app.js` before every commit that touches it.
- Driving the app: `.claude/skills/drive-app/SKILL.md`.
  - **The owner keeps `"reopen": "off"` in `%APPDATA%\t4-markdown-viewer\config.json`. No agent writes `config.json`.**
  - Back up and restore `session.json`.
  - Kill only the debug PIDs you launched. The owner's viewer shares the process name, so never kill processes by command-line match.
  - End with a plain `cargo build`.
  - Set back through the app any Settings choice a check changes.
- **Holding an IPC call** uses drive-app's recipe for seeing a link open externally without launching anything: wrap `window.chrome.webview.postMessage`, then hold back and record the messages whose command matches.
  - For `plugin:opener|open_url`, record it and never release it.
  - For `load_file`, hold it and release it later. That is how a check keeps a refresh pending for as long as it needs.
  - `invoke` and `openUrl` in app.js are captured at load and can't be stubbed.
- Fixtures live in `docs/fixtures/`, including `all-types.md` (35 diagram
  sections). Scratch docs made while driving the app go in this session's
  scratchpad, never in the repo.
- Line numbers are the current file's (HEAD `6eae568`). Find code by the content quoted.
- Comments say why, in full sentences, like the surrounding code.
- **Anything a task can't do as written, or any choice it would need, goes back to the controller and on to the owner.** A check that must "bite first" is run on the build before the task's change. Relaunch afterwards, backing up and restoring `session.json` around it.

## Review Focus

1. **A save while the reader clicks an in-page link.**
   - Expected: the page ends at the anchor, and the refresh landing doesn't snap it back.
   - Test: Task 2, check (b).
2. **A link clicked mid-switch to another document.**
   - Expected: it still does nothing, so the wrong tab never navigates.
   - Test: Task 2, check (d).
3. **The picture tab after the refactor.**
   - Expected: zoom, pan, dblclick, a tab-switch restore and a resize give the same numbers as before.
   - Test: Task 3, checks (a) to (c).
4. **Space on a focused diagram.**
   - Expected: it opens the diagram and does not also scroll the page.
   - Test: Task 4, check (c).
5. **Links inside a focusable diagram.**
   - Expected: they are still reached by Tab and are still links to a screen reader.
   - Test: Task 4, checks (e) and (g).
6. **Space after a mouse click on a diagram.** This covers opening and closing the overlay, and an actor's menu followed by Escape.
   - Expected: the page scrolls, and nothing opens.
   - Test: Task 4, check (i).

---

### Task 1: `docTarget` in GitHub's order

**Files:** Modify `src/app.js` (`docTarget`, ~2714–2731).

- [ ] **Step 1: Fixture** `docs/fixtures/b1-targets.md`:
  ```markdown
  [to x](#user-content-x) · [to fn](#fn-1) · [to ref](#fnref-1) · [plain x](#x) · [gh fn heading](#user-content-fn-1)

  ## X
  ## User content X
  ## fn 1

  Text with a note.[^1]

  (padding: 60 lines of "filler")

  [^1]: The note.
  ```
  comrak puts footnotes at the end, so the footnote can't be given a scroll position of its own. Every check asserts by `location.hash` and `document.querySelector(":target")`, not by what is at the top of the window.
- [ ] **Step 2: Bite first** (HEAD build). Click each link and record `location.hash` and the `:target` element.
  - "to x" lands on `user-content-user-content-x`, which is wrong.
  - "to fn" lands on the heading `user-content-fn-1`, which is wrong.
- [ ] **Step 3: Change** `docTarget`. Replace the doc comment and body with:
  ```js
  /**
   * The element in the document that `id` names, or null. Prefixed first — no
   * app id can carry the prefix, which a test pins — then as written, then a
   * GitHub link without its prefix. Two kinds of name go as written first: one
   * that already carries the prefix, which is GitHub's order, so
   * `#user-content-x` finds `## X` rather than `## User content X`; and a
   * footnote's, since comrak writes its own footnote links bare (`#fn-1`,
   * `#fnref-1`) and they must find the footnote rather than a heading named
   * `fn 1`. The bare names are looked for inside the document only: the app's
   * own elements come first in the page, and a link must never land on one.
   */
  function docTarget(id) {
    if (!id) return null;
    const inDoc = (name) => els.content.querySelector(`#${CSS.escape(name)}`);
    const prefixed = () => document.getElementById(DOC_ID_PREFIX + id);
    const stripped = () => (id.startsWith(DOC_ID_PREFIX) ? inDoc(id.slice(DOC_ID_PREFIX.length)) : null);
    if (id.startsWith(DOC_ID_PREFIX) || /^fn(ref)?-/.test(id)) return inDoc(id) ?? prefixed() ?? stripped();
    return prefixed() ?? inDoc(id) ?? stripped();
  }
  ```
- [ ] **Step 4: Verify** (drive-app), on `b1-targets.md`. Each result below is the `:target` element:
  - (a) "to x" → `#user-content-x` (`## X`).
  - (b) "to fn" → the footnote `li#fn-1`.
  - (c) "to ref" → the back-reference `#fnref-1`.
  - (d) "plain x" → `## X`, unchanged.
  - (e) "gh fn heading" → the heading `fn 1`, unchanged.
  - (f) Back after each returns to the link line.
  - (f′) The case the item was filed for: click the real footnote reference in the text (`a[data-footnote-ref]`) and it lands on footnote 1, not on the heading `fn 1`. Its back-reference (`a.footnote-backref`) returns to the reference. Bite first: on HEAD the reference lands on the heading.
  - (g) all-types: 35 drawn, 0 refused, no console errors beyond the two known boot-time `ipc.localhost/get_settings` CSP lines.
- [ ] **Step 5: Commit.** Subject: `Land heading and footnote links where they point`.

### Task 2: Links keep working while a save redraws the page

**Files:** Modify `src/app.js` (`onLinkClick` gate ~3609–3618, `pushAnchorEntry` ~2763–2782).

**Interfaces:** Consumes `shownToken`, `renderToken`, `shownEntry`, `currentEntry(activeTab())`, `refresh()`.

- [ ] **Step 1: Fixture** `docs/fixtures/b2-links.md`:
  - an external link `[ext](https://example.com)`;
  - `[sec](#far)`, with `## Far` 80 lines down;
  - `[self](b2-links.md#far)`, for `loadPath`'s same-file branch;
  - `[doc](b2-other.md)`, with `b2-other.md` beside it. That file holds `[ext2](https://example.net)`, `[sec2](#end)`, and `## End` 80 lines down;
  - `[pic](b2-pic.png)`, where the picture is any small PNG.
- [ ] **Step 2: Bite first** (HEAD build).
  - Hold `load_file`, then press F5 (a refresh is now pending).
  - Click "ext": no `open_url` is recorded.
  - Release.
- [ ] **Step 3: The gate.** In `onLinkClick`, replace the comment and `if` that start `// Nothing to follow: an emptied link` with:
  ```js
    // Nothing to follow: an emptied link — comrak writes `href=""` for a
    // `file:`, `javascript:` or `data:` link, and mermaid `about:blank` for an
    // unsafe `click` URL — or a page that is not the active entry's, a switch
    // still under way. Either way the webview must not follow it on its own:
    // following `""` reloads the page, and every tab in the window goes with
    // it. A refresh of the page on screen — a save, F5 — is not a switch: its
    // links act on it, and one that navigates outdates the refresh.
    const pending = shownToken !== renderToken;
    if (!href || href === "about:blank" || (pending && shownEntry !== currentEntry(activeTab()))) {
      event.preventDefault();
      return;
    }
  ```
- [ ] **Step 4: The anchor case.** A refresh still drawing would land on the entry it started from, and put the page back where that entry stood. At the end of `pushAnchorEntry`, after `bankLanding(id);`, add:
  ```js
    // A refresh of this page still drawing would land on the entry it started
    // from, at that entry's spot. Draw it again for this one instead.
    if (shownToken !== renderToken) refresh().catch(console.error);
  ```
  Both anchor paths reach it: `onLinkClick`'s `#…` branch, and `loadPath`'s same-file branch. Document links already go through `showActive`, which outdates the pending render, and external, picture and reveal links don't touch the page.
- [ ] **Step 5: Verify** (drive-app). Each check holds `load_file`, presses F5, acts, then releases every held message.
  - (a) "ext": one `open_url` for `https://example.com/` is recorded.
  - (b) "sec": the page jumps to `## Far` at once. After release:
    - `scrollY` is still at `## Far` (±2 px);
    - the active entry's `hash` is `far`;
    - `shownEntry === currentEntry(activeTab())`;
    - Back returns to the link line.
  - (b′) The same with "self": `loadPath`'s same-file branch ends the same way.
  - (b″) The fast path: click "sec" and release every held message in the same eval. After the page settles it is still at `## Far`.
  - (c) "doc": its own `load_file` is held too, so `b2-other.md` opens only after release. Then `els.content.dataset.path` is `b2-other.md`, the old refresh has not painted over it, and Back returns to `b2-links.md`.
  - (d) Mid-switch, tab: open `b2-other.md` in a second tab. Hold `load_file` and switch to the first tab. The page on screen is still the second tab's; click "ext2" and "sec2" there.
    - Nothing happens.
    - No `open_url` is recorded.
    - `tabs[0].entries.length` and `tabs[1].entries.length` are unchanged.
    - Release: the first tab shows normally.
  - (d′) Mid-switch, same tab: hold `load_file`, click "doc", then click "ext" and "sec" on the `b2-links.md` page still on screen. Nothing happens, and no `open_url` is recorded. Release: `b2-other.md` shows.
  - (e) "pic" during a pending refresh opens the picture tab.
  - (f) With nothing pending, every link behaves as on HEAD.
  - (g) all-types: 35 drawn, 0 refused, no console errors beyond the two known lines.
- [ ] **Step 6: Commit.** Subject: `Keep a page's links working while a save redraws it`.

### Task 3: One zoom-and-pan core for the picture tab and the diagram overlay

**Files:** Modify `src/app.js`:
- the overlay (~1086–1114, 1181–1230);
- the image viewer (~1446–1467, 3801–3843);
- the listener wiring (~4247–4260, 4282).

**Interfaces:** Produces:
- `zoomAbout(view: Element, el: Element, resize: () => void, clientX?: number, clientY?: number): void`
- `dragPan(view: Element, active: () => unknown, moved?: () => void): { down, move, up, cancel }`

- [ ] **Step 1: Baseline** (HEAD build). Write a CDP script `scratchpad/b3-measure.js` that records numbers after each action. Run it on a large raster (≥ 2000 px wide), an SVG picture, and a flowchart diagram in the overlay.
  - Both runs, before and after, must be scripted identically, because `fitWidth` depends on the panel's size:
    - a fresh launch (reopen is off);
    - the window moved to a fixed size first (`MoveWindow`);
    - the same files, opened in the same order.
  - The numbers: the element's `style.width`, the view's `scrollLeft`/`scrollTop`, and the zoom label's text.
  - The actions:
    - Ctrl+wheel at a fixed client point, via `Input.dispatchMouseEvent` type `mouseWheel` with modifiers 2, three times in and one out;
    - a real drag of (−120, −80) via mouse pressed/moved/released;
    - a dblclick at a point, then another;
    - the toolbar's in, out and fit;
    - Ctrl+`+`, Ctrl+`-`, Ctrl+`0`.
  - On the picture only: pan, switch tabs, switch back, then record.
  - A window resize (drive-app's resize), then record.
  - Save the results as `b3-before.json`.
- [ ] **Step 2: Add the core** right above `zoomTo`, replacing `zoomTo`'s doc comment, which moves onto `zoomAbout`:
  ```js
  /**
   * Resize `el` inside the scroll box `view` about a point, so whatever was
   * under the cursor stays under it — or the box's centre, given no point.
   * Done by measuring before and after rather than by arithmetic on offsets:
   * the content is centred while it is smaller than the box and hard against
   * the edge once it is bigger, and measuring is right either way. `resize`
   * applies the new size.
   */
  function zoomAbout(view, el, resize, clientX, clientY) {
    const box = view.getBoundingClientRect();
    const before = el.getBoundingClientRect();
    const ax = clientX ?? box.left + box.width / 2;
    const ay = clientY ?? box.top + box.height / 2;
    const fx = before.width ? (ax - before.left) / before.width : 0.5;
    const fy = before.height ? (ay - before.top) / before.height : 0.5;
    resize();
    const after = el.getBoundingClientRect();
    view.scrollLeft += after.left + fx * after.width - ax;
    view.scrollTop += after.top + fy * after.height - ay;
  }

  function zoomTo(w, clientX, clientY) {
    if (!picture) return;
    zoomAbout(els.imageView, els.imageEl, () => applyWidth(w), clientX, clientY);
    rememberImage();
  }
  ```
  And `zoomDiagram` becomes:
  ```js
  /** Zoom the overlay's copy about a point (`zoomAbout`). */
  function zoomDiagram(scale, clientX, clientY) {
    if (!zoomed) return;
    zoomAbout(
      els.diagramView,
      zoomed.svg,
      () => {
        zoomed.scale = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, scale));
        const width = diagramFit() * zoomed.scale;
        zoomed.svg.style.width = `${width}px`;
        zoomed.svg.style.height = `${width / zoomed.ratio}px`;
        els.diagramLevel.textContent = `${Math.round(zoomed.scale * 100)}%`;
      },
      clientX,
      clientY,
    );
  }
  ```
- [ ] **Step 3: The pan.** Replace the `/* Panning the picture … */` block and `let pan = null;` through `onImagePointerUp` (~3801–3843) with:
  ```js
  /*
   * Drag-to-pan, for the picture and the diagram overlay alike. Pointer capture
   * rather than a document-level listener so a drag that leaves the window
   * still steers the scroll, and so releasing outside it still ends cleanly.
   * `active` says whether there is anything to pan; `moved` hears of a drag
   * that moved, once it ends.
   */
  function dragPan(view, active, moved) {
    let pan = null;
    function up(event) {
      if (!pan || event.pointerId !== pan.pointerId) return;
      const didMove = pan.moved;
      pan = null;
      try {
        view.releasePointerCapture(event.pointerId);
      } catch {
        /* capture already gone */
      }
      view.classList.remove("panning");
      if (didMove) moved?.();
    }
    return {
      down(event) {
        if (!active() || event.button !== 0) return;
        // Also what stops the browser starting its own image drag instead.
        event.preventDefault();
        pan = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, moved: false };
        view.setPointerCapture(event.pointerId);
      },
      move(event) {
        if (!pan || event.pointerId !== pan.pointerId) return;
        if (event.buttons === 0) return up(event);
        const dx = event.clientX - pan.x;
        const dy = event.clientY - pan.y;
        if (!pan.moved && Math.hypot(dx, dy) < PAN_THRESHOLD) return;
        pan.moved = true;
        pan.x = event.clientX;
        pan.y = event.clientY;
        view.classList.add("panning");
        // Dragging the picture left means looking further right.
        view.scrollLeft -= dx;
        view.scrollTop -= dy;
      },
      up,
      /** Let go of a drag no pointerup is coming for. */
      cancel() {
        if (pan) {
          try {
            view.releasePointerCapture(pan.pointerId);
          } catch {
            /* capture already gone */
          }
        }
        pan = null;
        view.classList.remove("panning");
      },
    };
  }

  const imagePan = dragPan(els.imageView, () => picture, rememberImage);
  ```
  In the overlay section:
  - replace `let diagramPan = null;` with `const diagramPan = dragPan(els.diagramView, () => zoomed);`;
  - delete `onDiagramPointerDown`, `onDiagramPointerMove` and `onDiagramPointerUp`;
  - in `onDiagramClose`, replace the `if (diagramPan) {…}`, `diagramPan = null;` and `classList.remove("panning")` lines with `diagramPan.cancel();`, keeping the comment above them.
- [ ] **Step 4: Wiring.** Change these registrations:
  - picture: `pointerdown/move/up/cancel` → `imagePan.down`, `imagePan.move`, `imagePan.up`, `imagePan.up`;
  - overlay: the same, with `diagramPan`;
  - `lostpointercapture` handlers → `(e) => e.target === els.imageView && imagePan.up(e)` and `(e) => e.target === els.diagramView && diagramPan.up(e)`.

  `grep -nE "onImagePointer|onDiagramPointer|\bpan\s*(=|\.)" src/app.js` must find nothing outside `dragPan`.
- [ ] **Step 5: Verify.**
  - (a) Re-run `b3-measure.js`, save the results as `b3-after.json`, and diff them: every number is identical.
  - (b) Escape mid-drag in the overlay: `panning` is cleared, and the next drag pans.
  - (c) Drag outside the window and release there: the drag ends, and the cursor is not stuck on `grabbing`.
  - (d) After a picture pan, switch tabs and back: the scroll is still restored. This doesn't prove `moved`: the image view's scroll listener also banks. That `moved` matches HEAD comes from the code review.
  - (e) all-types: 35 drawn, 0 refused, no console errors beyond the two known lines.
- [ ] **Step 6: Commit.** Subject: `chore: share the picture tab's zoom and pan with the diagram overlay`.

### Task 4: Open a diagram or a picture from the keyboard

**Files:** Modify `src/app.js`:
- `renderDiagrams` ~990;
- `resolveMedia` ~362;
- `renderDocument` ~1331–1363;
- `openDiagram` ~1149;
- `onDiagramClose` ~1196;
- `onLinkClick`, first line inside `if (!a) {`;
- the listener wiring, beside `els.content.addEventListener("click", onLinkClick);` (~4233);
- `onKeydown`, after `if (event.key === "Escape" && closeActorMenus()) return;`.

Modify `src/base.css`, next to `.markdown-body img[data-file]`.

**Interfaces:** Produces, beside `figureOf`:
- `const OPENABLE = ".mermaid-diagram[tabindex], img[data-file][tabindex]";`
- `openableKey(el): string`
- `markOpenable(el): { at: number, what: string }`
- `refocusOpenable(mark): void`

- [ ] **Step 1: Fixture** `docs/fixtures/b4-keys.md`, in this order:
  - a paragraph with a link;
  - a flowchart with `click A "https://example.com"`;
  - `![alt text](b2-pic.png)`;
  - `[![linked](b2-pic.png)](https://example.org)`;
  - a failing mermaid block (`graph TD; A-->`);
  - a second flowchart;
  - a sequence diagram: `participant Alice`, `link Alice: Profile @ https://example.com`, `Alice->>Bob: hi`;
  - about 80 filler lines, so the page is taller than the window.

  A second fixture, `docs/fixtures/b4-noalt.md`, holds only `![](b2-pic.png)`.
- [ ] **Step 2: Bite first** (HEAD build): Tab through the page. Neither diagram nor the unlinked picture takes focus.
- [ ] **Step 3: Make them focusable.**
  - In `renderDiagrams`, after `fig.className = "mermaid-diagram";`:
    ```js
        // Reached by Tab and opened by Enter or Space (`onKeydown`). A group
        // rather than a button: a button's content is presentational, which
        // would hide the drawing's own links from a screen reader.
        fig.tabIndex = 0;
        fig.setAttribute("role", "group");
        fig.setAttribute("aria-label", "Diagram — Enter opens it full window");
    ```
  - In `resolveMedia`, replace `if (el.tagName === "IMG" && isImage(file)) el.dataset.file = file;` with:
    ```js
      if (el.tagName === "IMG" && isImage(file)) {
        el.dataset.file = file;
        // Reached by Tab and opened by Enter or Space too — unless it is a
        // link's picture, where the link is what a key press means.
        if (!el.closest("a")) {
          el.tabIndex = 0;
          el.setAttribute("role", "button");
          if (!el.alt) el.setAttribute("aria-label", baseName(file));
        }
      }
    ```
  - Beside `figureOf`:
    ```js
    /** What a key press can open: a diagram, or a picture outside a link. */
    const OPENABLE = ".mermaid-diagram[tabindex], img[data-file][tabindex]";
    ```
- [ ] **Step 4: Enter and Space.** In `onKeydown`, right after `if (event.key === "Escape" && closeActorMenus()) return;`:
  ```js
    // A diagram or picture with focus opens as a click on it would. Space
    // too, which would otherwise scroll the page. Not on a held key's repeat,
    // which would open a picture's tab again before the first has taken the
    // page away.
    if ((event.key === "Enter" || event.key === " ") && !ctrl && !event.altKey && !event.shiftKey && !event.repeat) {
      const el = document.activeElement;
      // Only a keyboard focus. A mouse press lets go of any focus it gave (the
      // mousedown listener, and `onLinkClick`); this is the last guard.
      if (
        el &&
        els.content.contains(el) &&
        el.matches(":focus-visible") &&
        (diagramBlocks.has(el) || el.matches("img[data-file][tabindex]"))
      ) {
        event.preventDefault();
        el.click();
        return;
      }
    }
  ```
  And in `onLinkClick`, as the first line inside `if (!a) {`:
  ```js
      // A click focuses a diagram as well, now it can take focus, and a later
      // key — the Escape that closes its overlay or an actor's menu — would
      // turn that into a keyboard focus, so a Space meant to scroll would open
      // it. Let go of it. Not for a key press, which `el.click()` sends with no
      // detail.
      if (event.detail > 0 && diagramBlocks.has(document.activeElement)) document.activeElement.blur();
  ```
  And in the listener wiring, beside `els.content.addEventListener("click", onLinkClick);`:
  ```js
    // A press that never becomes a click on the page — a right or middle
    // button, or one dragged out of it — focuses a diagram or picture too, and
    // any key after makes that a keyboard focus. Let go of it once the press
    // has done its focusing. `onLinkClick` lets go at the click as well, which
    // a fast click can reach before this does.
    els.content.addEventListener("mousedown", () =>
      setTimeout(() => {
        const el = document.activeElement;
        if (els.content.contains(el) && el.matches(OPENABLE)) el.blur();
      }),
    );
  ```
- [ ] **Step 5: Back to the diagram after a save.** Measured (2026-09-28, WebView2): the dialog hands focus back to the element focused before `showModal()` by itself, whether closed by Escape or ×. So there's no explicit return for the plain case. What the dialog can't do is return focus to a diagram that a save replaced while the overlay was open, since that node is detached by then.
  - Beside `OPENABLE` (Step 3):
    ```js
    /** What makes an openable element the same one after a re-render: its source or its file. */
    function openableKey(el) {
      return diagramBlocks.get(el)?.pre.textContent ?? el.dataset.file;
    }

    /** Where an openable element stands among the page's, and what it is. */
    function markOpenable(el) {
      return { at: [...els.content.querySelectorAll(OPENABLE)].indexOf(el), what: openableKey(el) };
    }

    /**
     * Focus the element `mark` recorded, found again by its place among the
     * page's — a render makes new ones — and only if it is still the same one
     * there, or focus would land on a neighbour.
     */
    function refocusOpenable(mark) {
      const again = els.content.querySelectorAll(OPENABLE)[mark.at];
      if (again && openableKey(again) === mark.what) again.focus({ preventScroll: true });
    }
    ```
  - In `openDiagram`'s `zoomed = {…}`, add `back: fig.matches(":focus-visible") ? markOpenable(fig) : null,`, with the comment `// Opened from the keyboard: where to go back to, should a save replace it.` (`zoomed.path` already records the document.)
  - In `onDiagramClose`, change `zoomed = null;` to:
    ```js
      const back = zoomed?.back;
      const path = zoomed?.path;
      zoomed = null;
    ```
    and after `els.diagramView.replaceChildren();` add:
    ```js
      // The dialog hands focus back to the diagram it was opened from — unless
      // a save has since replaced it, and focus has fallen out of the page.
      // Only on the same document: another one put up behind the overlay
      // closed it, and its diagrams are not this one's.
      if (
        back &&
        !els.content.contains(document.activeElement) &&
        !els.content.hidden &&
        els.content.dataset.path === path
      )
        refocusOpenable(back);
    ```
- [ ] **Step 6: Back after a save.** In `renderDocument`:
  - After the `const refocus = …` line:
    ```js
      // A diagram or picture reached from the keyboard is put back the same way.
      const openable =
        f?.matches(OPENABLE) &&
        f.matches(":focus-visible") &&
        els.content.contains(f) &&
        doc.path === els.content.dataset.path
          ? markOpenable(f)
          : null;
    ```
  - After the `if (refocus) {…}` block:
    ```js
      if (openable) refocusOpenable(openable);
    ```
- [ ] **Step 7: Focus ring.** In `base.css`, after the `.markdown-body img[data-file]` rule:
  ```css
  /* Both open from the keyboard too; the ring is the only sign which has focus. */
  .markdown-body div.mermaid-diagram:focus-visible,
  .markdown-body img[data-file]:focus-visible {
    outline: 2px solid var(--ui-accent);
    outline-offset: 2px;
  }
  ```
- [ ] **Step 8: Verify** (drive-app, real key events via `Input.dispatchKeyEvent`), on `b4-keys.md`.
  - Measured: Space only scrolls when sent as `keyDown` with `text:" "`, `key:" "`, `code:"Space"`, `windowsVirtualKeyCode:32`; a `rawKeyDown` never does. Every Space below is sent that way. The measurement scripts are `scratchpad\bm\spacekey.mjs` and `enterkey.mjs`.
  - Mouse clicks go through `cdp.mjs click` (`Input.dispatchMouseEvent`, `clickCount: 1`), never `el.click()`, which sends `detail` 0 and would look like a key. Aim them at the second flowchart: the first one's node A is a real link.
    - Before each click, `el.scrollIntoView({ block: "center" })`: `cdp.mjs click` doesn't scroll to its target.
  - Hold `plugin:opener|open_url` for the whole of this step.
  - Right before each Space (not before the click that precedes it), scroll to 0 (`scrollTo(0, 0)`). Where a scroll is expected, assert `scrollY > 0`.
  - "Keyboard-focus X" means real Tab presses until X has focus. Before any save, Enter or Escape that depends on it, assert `document.activeElement === X && X.matches(":focus-visible")`. This applies to (b′), (f), (h-repeat) and (h′).
  - Bite first for (c): with the body focused, that Space makes `scrollY > 0`.
  - (a) Tab order: the paragraph link, the first diagram, its `click` link, the unlinked picture, the linked picture's link (the `img` itself is skipped), the second diagram, the sequence diagram. The failed block is never focused. Each focused diagram or picture has a computed `outline-style` of `solid`.
  - (b) Enter on the first diagram: the overlay opens. Escape closes it, and `document.activeElement` is that diagram.
  - (b′) Save while open:
    1. Keep `window.__old` = the second diagram.
    2. Press Enter on it.
    3. Touch the file (unchanged, so the overlay stays open).
    4. Poll until `!__old.isConnected`.
    5. Press Escape.
    - `activeElement !== __old`, and it is `els.content.querySelectorAll(OPENABLE)[2]`.
    - Bite: run it with Steps 3–4 in but before Step 5's `onDiagramClose` lines. It ends on body.
  - (c) Space on the second diagram: the overlay opens, and `scrollY` stays 0.
  - (d) Enter on the unlinked picture: the picture tab opens.
  - (e) Enter on the diagram's `click` link: an `open_url` is recorded (held), and the overlay does not open.
  - (f) Refocus:
    - Keyboard-focus the second diagram and touch the file: after the re-render, `activeElement` is the second diagram.
    - The same for the picture.
    - Two saves in a row: focus is still on the second diagram after the second.
    - Mouse: click the second diagram (the overlay opens), close it with × by mouse, then touch the file. Focus is not on any diagram.
  - (g) Using a scratch `ax.mjs` that holds one CDP socket for `DOM.getDocument` → `DOM.querySelector` → `Accessibility.getPartialAXTree`:
    - a diagram: role `group` with the label, and its link is still a `link` node;
    - the picture: role `button`, name `alt text`;
    - on `b4-noalt.md`: role `button`, name `b2-pic.png`.
  - (h) Ctrl+Enter and Shift+Space on a focused diagram don't open it.
  - (h-repeat) Hold `load_asset`. In one script, send a keyDown Enter then a keyDown Enter with `autoRepeat: true` on the focused picture. `tabs.length` rose by 1. Release.
    - Control run with two non-repeat Enters: `tabs.length` rose by 2, which proves the window exists.
    - After each release, close the tabs it opened. Assert that `els.content.dataset.path` ends with `b4-keys.md` again.
  - (h′) Refocus against a neighbour: keyboard-focus the second diagram (openable index 2), then edit the file to remove the first diagram and save.
    - After the re-render, index 2 is the sequence diagram.
    - `activeElement` is not the sequence diagram; it is body.
    - Restore the fixture afterwards.
  - (i) Mouse: a click on the second diagram still opens it.
    - Close it with × and press Space: `scrollY > 0` and the overlay stays shut.
    - The same, closing with Escape.
    - Click the sequence actor Alice (her menu opens), then Space: `scrollY > 0` and no overlay.
    - Actor menu, then Escape:
      1. Press Escape, and assert every `g.actorPopupMenu[id$="_popup"]` has `display="none"`.
      2. Click Alice, and assert her menu is `display="block"`.
      3. Press Escape, and assert it is `"none"`.
      4. Press Space: `scrollY > 0` and no overlay.
  - (i5) A press dragged out: `cdp.mjs drag` from the second diagram's centre to a point on the tab strip, then Space. `scrollY > 0` and no overlay. Do the same with a right-click on the diagram (then Escape to dismiss the context menu) and with a middle-click.
  - (j) all-types: 35 drawn, 0 refused, no console errors beyond the two known lines.
- [ ] **Step 9: Commit.** Subject: `Open a diagram or a picture from the keyboard`.

### Task 5: Docs

**Files:** `README.md`, `docs/open-items.md`, `docs/closed-items.md`. Move this plan to `docs/archive/plans/viewer-follow-ups.md`, with an `## Outcome`.

- [x] **Step 1: README.**
  - Where the README describes opening diagrams and pictures, add that Tab reaches them and Enter or Space opens them.
  - Where it describes in-page links, add two things: a `#user-content-…` link resolves in GitHub's order, and a footnote link finds its footnote even when a heading has the same name.
  - Wrap at 80.
- [x] **Step 2: Items.**
  - Close these into closed-items `## Deferred`, each with `Fixed <hash>, <date>, archive/plans/viewer-follow-ups.md` and a one-line how, wrapped to the file's width:
    - "The diagram overlay duplicates the image viewer's zoom and pan";
    - "`docTarget`'s lookup order can differ from GitHub's";
    - "Link clicks on a page are dropped while its refresh draws diagrams".
  - "Open a diagram or a picture from the keyboard": close the part now done (focusable, Enter/Space, focus ring, back after overlay and save). Rewrite the open item to hold only what's left: keeping focus on a link inside a diagram across a light/dark redraw, and keyboard access to sequence actor menus. It keeps the same reopen trigger (the next accessibility pass, or someone asks).
  - Outcome: every owner decision made during execution, from the ledger.
- [x] **Step 3:** Tick the boxes, add `## Outcome`, move the plan to `docs/archive/plans/` (`mv` + `git add`, since the plan is untracked). Commit with the subject `docs: close the viewer follow-ups, and file their plan`.

## Outcome (2026-09-28)

Done, one commit per task, all on `main`, none pushed: b5bb12b (Task 1, `docTarget` in GitHub's
order; fixup f7b79db), de4aebb (Task 2, links keep working while a save redraws the page),
1658765 (Task 3, `chore:` one zoom-and-pan core for the picture tab and the diagram overlay),
1b655d5 (Task 4, open a diagram or a picture from the keyboard).

**Owner's decisions during execution, in order** (full text in the ledger, `progress.md`):

- Task 1: the implementer wrote the fixture's padding as separate paragraphs (consecutive lines
  render as one), and judged check (f) ("Back") by scroll position rather than by hash, which a
  same-document Back leaves unchanged; the owner accepted both. The controller's read of the
  diff found the comment had dropped why a prefixed link is also tried stripped (GitHub
  prefixes footnotes too); the owner had it restored, fixup f7b79db.
- Task 2: the implementer noted that check (a)'s expected URL had a trailing slash the app never
  sends (a typo in the brief), and that two `load_file`s were held at release, the second being
  the fresh refresh the change starts; the owner accepted both.
- Task 3: the implementer chose the measurement script's shape (pan, tab switch and restore
  while zoomed; an extra resize while zoomed; a five-chain flowchart; `activateTab` for tab
  switches), found `scratchpad/movewin4.ps1` overwrote its height parameter and wrote
  `t3/size.ps1` instead, and noted Git Bash's `grep` printed nothing for the brief's pattern
  (ripgrep confirmed it). The owner accepted the script choices and left `movewin4.ps1` as is.
  Review: no findings.
- Task 4: implementation hit that WebView2's middle-click autoscroll eats the next key — the
  owner ruled Escape then Space for that check. The owner also accepted running the drag check
  both ways and adding extra focus assertions, and had the `back` field's comment on `zoomed`
  extended.
- Task 5 (this task): the controller's read of the docs found five misstatements — the README's
  `#user-content-…` wording, the attribution and wording of Tasks 1–3 above, the plan-review
  and measured lines, and the open item's title; the owner had all five fixed.
- Final review: two findings were measured first, with the viewer really the foreground window.
  M1: a save landing while the window is inactive lost a keyboard-focused diagram's focus,
  since `:focus-visible` reads false there — the owner had the `:focus-visible` test dropped
  from `renderDocument`'s diagram-and-picture refocus (the checkbox one stays), with a comment
  saying why: a mouse press never leaves focus on one, both blurs clear it. After it, focus
  lands on the new diagram. M2: a keyboard-opened overlay closed by a mouse click on ×, then
  Space, reopened it — the dialog hands focus back to the diagram, and the key press itself
  brings back its ring. The owner chose to blur that diagram on a mouse close (× by a click,
  the thumb's Back), and, since `onDiagramClose` then took focus out of the page for a save's
  doing and put it back, to also drop `zoomed.back` first; Space then scrolls, and Escape
  still returns focus with the ring. Also: closing a picture tab opened with Enter or Space
  puts focus back on the picture; the overlay copy's links are not Tab stops; `OPENABLE` is
  the one test for Enter/Space; comments reworded for both viewers and for Enter; the README
  lists Enter/Space; the open and closed items were corrected, and the macOS/Linux item now
  names the keyboard checks, measured on WebView2 only. The M1 check used a window of its own
  in place of Notepad, and drive-app gained a gotcha: a focus check needs the viewer really in
  front.

**Plan review:** three review rounds (`plan-review.md`) settled, among others:
- five open questions measured on WebView2 before deciding;
- Enter/Space act only on a `:focus-visible` diagram or picture, and a mouse press lets go of
  the focus it gave twice over — a `mousedown` listener a task later (right and middle buttons,
  a press dragged out) and `onLinkClick` at the click (a fast click that beats the task);
- the dialog's own focus return is relied on, with a `markOpenable`/`refocusOpenable` fallback
  for a diagram a save replaced while the overlay was open, on the same document only;
- a held key's repeat is guarded (`!event.repeat`);
- mid-switch picture and diagram clicks, ungated before this plan, stay as they were.

**Measured:** the dialog restores focus to the diagram it was opened from natively, with no
code of the app's own; Space only scrolls the page when sent as a `keyDown` with real
`text`, never as a `rawKeyDown`; WebView2's middle-click autoscroll eats the next key press, so a
check on it must close autoscroll with Escape before sending Space; and Task 3's zoom-and-pan
core matched the pre-refactor picture tab and diagram overlay on 290 of 290 measurements.
