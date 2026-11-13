# Document Zoom Plan

> **Filed (2026-11-13):** executed on the owner's go, reviewed, and walked on all three VMs
> (walk branch `walk/doc-zoom`: 68649b9, then 42bbb51 with the walk's two fixes, which every
> re-walk check passed). The walk's fixes are written into *Design*: the actor menu placed
> from the drawing's rectangle, and a step going back to where the last one aimed its line.
> The owner's rulings from the walk and the triage, all 2026-11-13:
>
> - the stepped pinch on a document ships as built on macOS, and a smooth one follows the
>   release (`open-items.md`, *Deferred*);
> - the slip of a line of text over a burst of steps was fixed; the creep on empty space on
>   macOS and Linux, and find's match up to 3 px off centre at 300% on macOS, are accepted;
> - the picture and diagram pinch rate on macOS stays as built;
> - *A size for the document font* stays deferred, with a note;
> - Task 7's list and the walk's other findings are filed in `open-items.md` and
>   `closed-items.md`, 21 rulings in one go.
>
> One check in Task 6 was wrong about the app and is corrected in place: a link can't be
> opened in a tab that is not shown. The status below is as it stood before the go.
>
> **Status (2026-11-12):** the rulings are made (*Rulings*, *Decisions after Task 1b*). The
> three trials by script are done on all three VMs: Task 1, the measuring walk; Task 1b, the
> layout corrections and the pinch; Task 1c, the diagram labels. Each has a results section.
> Candidate A2 passes every check on every engine, a pinch can be told from `Control`+wheel,
> and rewriting a drawing's `em` lengths keeps its labels in place. The build, Tasks 2 to 7,
> is not approved.

**Goal:** a reader can make every document bigger or smaller, as a picture or a diagram can be
zoomed today.

**Spec:** the owner's request of 2026-11-11, "a plan for document zoom functionality, give me
options", and the rulings below.

## Rulings (owner, 2026-11-12)

1. **What zooms: the document's content.** CSS `zoom` on `#content`. Text re-wraps to the pane;
   pictures, tables, code and diagrams scale with it; the tab bar, sidebar, find bar and Settings
   don't change. Turned down: a font-size factor (only scales `em` sizes, and `base.css`'s
   heading scale is in `rem`), and the webview's page zoom (scales the app's controls too).
2. **One level for the app.** Saved in `config.json` and broadcast to every window, as the
   document font is. Turned down: a level per tab, and a level per file.
3. **Keys and wheel, plus one row in Settings.** Turned down: keys only, and buttons on the bar.
4. **On macOS a pinch zooms and `Control`+wheel doesn't,** as in Safari and Chrome there. This
   holds for a document, a picture and a diagram alike. It changes what ships: today
   `Control`+wheel zooms a picture and a diagram on macOS, and a pinch does because the page
   sees it as the same event (`README.md:238`). Turned down: `Control`+wheel as today,
   `Cmd`+wheel, and no pinch either. It rests on the page telling the two apart, and it can
   (C8, measured).

Settled with the rulings, without a separate question:

- **Keys:** `Ctrl++` (and `Ctrl+=`), `Ctrl+-` and `Ctrl+0`, as a picture has (`Cmd` on macOS,
  `app.js:291`), and `Ctrl`+wheel on Windows and Linux. On a picture or an open diagram they
  keep their meaning; on a document they zoom the document, and `Ctrl+0` returns it to 100%.
- **Steps:** 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300%.

## Findings (read in the code 2026-11-11 and 2026-11-12; what was run is in *Task 1 results*)

- **Documents have no zoom.** The zoom keys do nothing unless a picture is open
  (`app.js:5626-5642`), and the wheel handlers sit on the picture and diagram views only
  (`app.js:6083`, `6090`). The webview's own page zoom is off: `tauri.conf.json` doesn't set
  `zoomHotkeysEnabled`.
- **A document is one element.** Markdown and JSON both render into `<article
  class="markdown-body" id="content">` (`index.html:125`). The page itself scrolls
  (`window.scrollY`), not that element.
- **The setting has a path to copy.** The document font: a `Config` field cleaned on load
  (`config.rs:99`, `180`), a command that saves and broadcasts (`set_doc_font`,
  `main.rs:1351`), the boot value (`app.js:6213`), a listener in every window (`app.js:6256`),
  and a fieldset in Settings › Appearance (`index.html:264-274`).
- **A reading position is a pixel offset.** Each history entry banks `window.scrollY`
  (`rememberScroll`, `app.js:361`), a restore holds it while pictures load (`holdScroll`,
  `app.js:1872`), and the session file carries the same numbers.
- **Few places read positions back from the document.** Find (`hitRect`, `scrollToHit`,
  `app.js:3698-3751`), the diagram overlay's starting shape (`app.js:1549`), and a sequence
  diagram's actor menu, placed from the click through the drawing's `getScreenCTM()`
  (`app.js:5092`). WebKit's matrix leaves out an ancestor's `zoom` (the walk, WebKitGTK
  2.52.6: the menu opened in the wrong place at 150% and 300%), so the click is now placed
  from the drawing's `getBoundingClientRect()`, which is in window px on every engine; V7
  clicks an actor. The anchor jump
  sets `location.hash` and leaves the scroll to the browser, with a `scroll-margin-top` of the
  bar's height plus the page padding (`base.css:2178`).
- **Four lengths tie the document to things outside it:**
  - the top padding, which matches the fade band under the bar (`base.css:562-569`, `1693`);
  - that `scroll-margin-top`;
  - the limit on tables, diagrams and pictures that step out of the column, `100cqw` of the pane
    less the page padding (`base.css:1726`);
  - the side padding that centres the reading column, a `calc()` of `100%` of the pane and the
    measure in `ch` (`base.css:1706-1710`). Added after Task 1; the first three were foreseen.
- **Diagrams are drawn outside the document** (a frame on `body`, `app.js:643`), so a zoom
  doesn't change how mermaid measures its labels. Confirmed by M9. How a drawn label sits
  inside a zoomed document is another matter (*Task 1c results*).
- **`toast` is a polite live region** (`index.html:171`), so a level said there is also read
  out by a screen reader.
- **An open item overlaps.** *A size for the document font* (`open-items.md`, Deferred).
- **macOS 13 can't be measured here.** The Mac VM runs macOS 26; no macOS 13 machine exists
  (`open-items.md`, *Diagrams are unproven on the macOS 13 floor*).

## Task 1 results (measured 2026-11-12 at 98919db; zoom put on by script)

Windows 11 26H2 with WebView2 157.0.4341.46; Ubuntu 26.04.1 with WebKitGTK 2.52.6 under Xvfb;
macOS 26.7.1 with its WKWebView. The app's window was 1400 and 700 px wide. "Measured" unless
a line says otherwise.

**The same on all three:**

- `zoom` is supported, text re-wraps, and the page never scrolls sideways (M1).
- `getBoundingClientRect`, `elementFromPoint` and `caretRangeFromPoint` give true window
  positions; `offset*`, `client*` and `scroll*` inside `#content` are in unzoomed px (M2).
- Find lands on its word and shows it (M6). A click on a task box changes the right line (M8).
  JSON folding and the "more" row work (M10). A tab switch returns to the same line (M11). The
  heading's *Copy section* button works (M12; a code block has no copy button, the plan was
  wrong).
- A diagram draws while zoomed and `diagramLook().key` doesn't change with the level, so a
  change of level redraws nothing. The full-window view opens the same at every level (M9).
- Keeping the place by the text at the top of the view leaves under 1 px. Scaling the offset
  alone drifts 16 to 254 px (M7). The browser's own scroll anchoring never moved the page.
- The caret range's rectangle is all zeros when the point is on an element with no text (a
  heading's button, Windows) or on white space in a table (Linux). Then there is nothing to
  hold on to (M7).
- `Ctrl`+wheel scrolls the page today. A cancelling wheel listener costs no smoothness: the
  longest frame gap was the same with and without it (M13).
- A level change takes 5 to 50 ms on the README. On a 1.8 MB JSON file it takes 0.2 to 0.35 s
  on Windows and macOS, and about 1 s on Linux (M14).

**Where they differ:**

| | Windows (WebView2) | Linux (WebKitGTK) | macOS (WKWebView) |
|---|---|---|---|
| `rem` lengths inside the zoom | scale | don't scale | scale |
| `cqw` lengths inside the zoom | scale | don't scale | scale |
| Page padding at 50 / 150 / 300% | 20 / 60 / 120 px | 40 / 40 / 40 px | 20 / 60 / 120 px |
| First line against the fade band (M5) | in the band at 50%, faded | clear | in the band at 50%, faded |
| Wide tables, diagrams, pictures (M3) | cut at both window edges at 150% and 300% | inside the pane | cut at both window edges |
| Anchor jump, heading below the bar at 50 / 100 / 150 / 300% (M4) | 1 / 40 / 79 / 194 px | 5 / 40 / 75 / 180 px | −4 / 40 / 95 / 260 px |
| Characters a line at 150%, 1400 px pane (M1) | 75 | 75 | 57 |
| Characters a line at 50%, 1400 px pane | 100 | 100 | 178 |
| One wheel notch (M13) | 100 px, made by script | 96 px | 40 px, made by script |

- **The cut tables** are the `100cqw` limit scaled by the level, so the box is wider than the
  pane and `main` clips it. Its own scrollbar can't reach the hidden columns.
- **The macOS column** is the centring `calc()` gone wrong. The four measured paddings fit
  exactly if the pane's width is taken unzoomed, `ch` from the zoomed font, and the result
  zoomed again. Reasoned from the numbers, not traced in WebKit. Whether `%`, `ch` or their
  mix is at fault is not known; Task 1b's unit table finds out.
- **At 50% WebKit draws body text at 9 px, not 7.** "Lorem" is 29 px wide on Linux and 28 on
  macOS where half size would be 23 and 22. The ratio is 9/7 on both, which fits WebKit's
  minimum font size; reasoned, not traced. The layout follows the drawn size, so nothing
  breaks; small text just stops shrinking.
- **Linux at 150%:** some step-out boxes come out 1 px wider than their limit and scroll
  sideways by 1 px, with no scrollbar showing.
- **The wheel notch on Windows is Chromium's usual 100 px at 100% display scaling.** At 150%
  scaling it is about 67 px (from memory, unmeasured). A real mouse on macOS is unmeasured.

**What the walk got wrong about the app:**

- A debug build on Windows shares the real `%APPDATA%\t4-markdown-viewer` folder. The peer
  backed up `config.json` and `session.json` and restored them byte for byte.
- The drive-app skill has no `windows.md`; `SKILL.md` covers Windows.

No stop condition fired: `zoom` works on every engine and a zoomed document is readable. The
differences have a design answer (below), but it is new, so it was trialled before it is built.

## Task 1b results (measured 2026-11-12 at 98919db; rules and properties put in by script)

The same three machines and builds. "Measured" unless a line says otherwise. Candidate B was
not run anywhere: A2 passed C3 on every engine.

**Units inside a `zoom: 1.5` box (C0):**

| | Windows (WebView2) | Linux (WebKitGTK) | macOS (WKWebView) |
|---|---|---|---|
| `px`, `em`, `ch` | scale | scale (`10ch` 100 → 140, by whole glyphs) | scale |
| `rem`, `cqw`, `vw` | scale | don't scale | scale |
| `%` of the parent | not scaled | not scaled | not scaled |
| The `cqw` probe | 200 | 100 | 200 |

- `%` is taken from the unzoomed parent, in window px, and not scaled again, on all three.
- So the macOS column of Task 1 was neither `%` nor `ch` alone but their mix in one `calc()`:
  `calc(50% - 10ch)` gives 500 − 144.5 there, as it does on Windows. Why the same arithmetic
  centres on Windows and not on macOS in the real rule was not traced; A2 takes `%` out of it.

**The checks:**

| | Windows | Linux | macOS |
|---|---|---|---|
| C1, 100% unchanged | A1, A2 pass | A1, A2 pass | A1, A2 pass |
| C2, first line below the bar | 40.0 at every level | 40.0 | 40.0 |
| C3, the column, A1 | pass | pass | **fails**: 57 and 178 characters a line, as in Task 1 |
| C3, the column, A2 | pass | pass | pass |
| C4, step-outs inside the pane | pass | pass | pass |
| C5, anchor jump at 50 / 100 / 150 / 300% | 40.3 / 40.4 / 39.8 / 40.2 | 39.8 / 39.6 / 40.4 / 39.8 | 40.2 / 40.4 / 39.6 / 40.2 |
| C6, sideways page scroll | none | none | none |

- **C3 under A2, 1400 px:** 100 characters a line at 50%, 75 at 150%, and the pane less 80 px
  at 300%, on all three. Linux gives 100 at 100% as well.
- **C3 under A2, 700 px:** the pane less 80 px at 150% and 300% on all three. At 50%: 100
  characters on Windows, 75 on Linux and macOS. Both are inside the rule. WebKit's 9 px floor
  makes a `ch` there wider than half size (5.4 px on macOS against 4.2), so two columns no
  longer fit the pane and the clamp picks the narrow measure.
- **A1 and A2 give the same numbers on Windows and Linux.** Only macOS tells them apart.
- **Linux at 150% still scrolls some step-out boxes 1 px sideways** under A1 and A2 (tables,
  pictures and diagrams in `widths.md`, one diagram in `kitchen-sink.md`). None at 50, 100 or
  300%.
- **WebKit's 9 px floor is measured on Linux:** the `ch` of the body text is 5 / 8 / 13 / 25 px
  at 50 / 100 / 150 / 300%.
- **Headings on Linux were not measured.** Their sizes are `rem` font sizes
  (`base.css:1760-1785`), and a `rem` width doesn't scale there. The body text's size is a
  `rem` font size too and does scale (the line above), and no screenshot at 150% or 300%
  drew a remark from the peer or the owner. So a `rem` font size is taken to scale where a
  `rem` width doesn't; reasoned. V7 measures a heading.
- **No console errors** on Windows and macOS. Linux's console can't be read over WebDriver.

**Seen outside the checks:**

- **macOS: a flowchart's node labels sit above the middle of their boxes when zoomed.** Label
  centre less box centre on the first node: −0.6 px at 100%, −9.0 at 150% (box 76 px high),
  −33.9 at 300% (box 152 px). The same with the bare `zoom` and no candidate rule, and after a
  redraw, so it comes from `zoom` itself. The numbers fit the label's `dy="1.1em"` not being
  scaled while everything else is (a 14.6 px font at both levels); reasoned, not traced.
  Windows and Linux were not measured for it.
- macOS, A1 only, 1400 px at 50%: the widest table's last cell ends at 289 px after scrolling
  to its end. Not looked into; A1 is not the candidate.

**The pinch (C8, macOS; pinches made by the owner's hand, 23 of them):**

- A pinch arrives as `gesturestart`, then a `gesturechange` and a wheel event by turns, then
  `gestureend`. Every wheel event has `ctrlKey` true, `deltaMode` 0 and a small `deltaY`
  (0.1 to 11 px).
- A real `Control`+wheel notch is one wheel event, `ctrlKey` true, `deltaY` 40, and **no
  gesture event at all**. So the gesture events tell the two apart.
- `gesturestart` comes before the pinch's first wheel event. `gestureend` comes before its
  **last** one, in the same millisecond.
- One pinch's `deltaY` adds up to about 100 px for each factor of *e* the fingers spread:
  −112 px for a spread to 3.0 times, −143 for 4.0, −51 for 1.65, −23 for 1.26, +125 for a
  squeeze to 0.27.
- An unhandled pinch does nothing: over text the page neither scrolled nor zoomed, and
  `visualViewport.scale` stayed 1. An unhandled `Control`+wheel notch scrolls 40 px.
- Over a picture, today's handler zoomed it; the owner: "zoomed in and out but sensitivity is
  slow". Today's rate turns a spread to 4 times into a picture 1.3 times as big.

## Decisions after Task 1b (owner, 2026-11-12)

1. **The flowchart labels on macOS:** one more trial by script before the build, on all three
   engines (Task 1c). Turned down: taking it to triage as a limit, and building the reasoned
   correction untested.
2. **How fast a pinch zooms:** it follows the fingers (*Design*, a pinch's rate). Turned down:
   today's rate, and halfway.

## Task 1c results (measured 2026-11-12 at 98919db; A2 and the rewrite put in by script)

The same three machines and builds, 1400 px wide. Flowchart, sequence, class, state, ER, gantt
and pie diagrams; all seven draw on every engine.

**What a drawing holds (the same on all three):** a label line in a flowchart, class, state
or ER diagram is a `<tspan>` with `y="-0.1em"` and `dy="1.1em"`; a flowchart can also have a
`y="1em"`; a sequence diagram and a gantt put `dy="1em"` (or `0em`) on the `<text>`; a pie
has none. No other attribute holds a relative length that places anything, and the drawings'
own styles have none but an icon rule.

**How far a label is from its place at 100%, worst in each drawing, in px:**

| | Windows | Linux | macOS |
|---|---|---|---|
| 150%, as drawn | under 1 | 7.5 high (11.9 on a two-line label) | 8 high (12.4) |
| 300%, as drawn | 1 to 2 | 19 to 31 high (48.6) | 19 to 32 high (49.6) |
| 50%, as drawn | under 1 | 8.5 low | 8 low |
| After the rewrite, up and down | as before it | 1.1 at most | 0 |
| After the rewrite, sideways | 2 at most | 7.5 at most, at 300% | 0.2 |
| At 100%, after the rewrite | nothing moves | nothing moves | nothing moves |

- **WebKit doesn't scale an `em` length in a text position with the zoom.** On macOS the
  shift is exactly the level less one, times the `em` length, times the font size: 8 px at
  150% and 31.3 at 300% for `dy="1em"` at 16 px. A pie, with no `em`, never moves. Measured;
  the mechanism inside WebKit is reasoned.
- **The rewrite** turns each such `x`, `y`, `dx` and `dy` into that number times the
  element's computed font size, taken at 100%. It cures the shift on Linux and macOS, and
  changes nothing on Windows (the screenshots before and after are the same byte for byte).
- **A two-line label's lines overlap** on Linux and macOS at 150% as drawn, and sit on two
  clean lines after the rewrite (seen in the screenshots).
- **What the rewrite doesn't touch:** on Linux, a label anchored at its left edge sits up to
  7.5 px left at 300% (class, state and ER diagrams; 2 px in a pie's legend), and on Windows up
  to 2 px. Traced on both to the text's own width: at three times the size it is a few
  percent narrower than three times as wide. The same holds with no zoom at all (ten zeros
  are 100 px wide at 16 px and 290 at 48 on Linux).
- **A changed level redraws nothing; a tab switch puts each drawing in again from the cache**
  and the `em` lengths are back. So the rewrite belongs where a drawing is made, before it is
  cached.
- **A drawing wider than the pane doesn't grow at first.** It is already shrunk to the pane,
  down to a floor of 60% of its own width (`app.js:1121-1126`); zoomed, it stays the pane's
  width until the floor times the level passes that, then grows and scrolls in its box. The
  gantt: 1320 px at 100% and 150%, 1980 px at 300% on macOS; a wide flowchart is 4042 px at
  300% on Windows.
- No console errors on Windows and macOS.

## Design

### Rust (`config.rs`, `main.rs`)

- `Config.doc_zoom: u32`, a percentage, default 100.
- `config::ZOOM_LADDER`, the thirteen steps. It lives here only; the page never needs the list.
- `config::doc_zoom(raw) -> u32`: `raw` if it is on the ladder, else 100. Applied in `load()`,
  since the file is the reader's to edit.
- `config::doc_zoom_step(current: u32, dir: i32) -> u32`: `dir` 0 gives 100; above 0 gives the
  next step up and below 0 the next down, staying put at either end.
- A `doc_zoom` of the wrong type (`"150"`, `1.5`) fails the whole file's parse and every
  setting falls back to its default, as for every other field today (`config.rs:173-176`).
  Not changed here.
- `step_doc_zoom(app, dir) -> u32` in `main.rs`: load, step, save, emit `doc-zoom-changed` with
  the level, and return it. Registered beside `set_doc_font` (`main.rs:1969`).

### Page (`app.js`, `diagram-frame.js`, `index.html`, `base.css`)

- **Applying a level.** `showDocZoom(percent)`, called at boot (`settings.doc_zoom`) and from
  the `doc-zoom-changed` listener. It keeps the level in `state.docZoom`, writes it into the
  Settings row, and sets three properties on `#content`, all removed at 100% so a reader who
  never zooms gets today's layout untouched:
  - `--doc-zoom`, the level as a number; `base.css` gives `#content` `zoom: var(--doc-zoom, 1)`;
  - `--content-pad`, the page padding in `px` divided by the level. The padding is the root's
    `--content-pad`, a `rem` number (`base.css:56`), times the root's computed font size. `px`
    scales on all three engines, so every use of the padding inside the document comes out at
    today's 40 window px: top, sides, the JSON view's, the step-out limit and the anchor
    margin. The fade band and the empty and error panels sit outside `#content` and keep the
    root's value;
  - `--cq-zoom`, the level where `cqw` scales inside a zoomed element and 1 where it doesn't.
    Found once, the first time a level other than 100% is applied, by a probe: a 100 px wide
    box on `body` with `position: absolute`, `visibility: hidden` (laid out, not shown) and
    `container-type: inline-size`, holding a `zoom: 2` box holding a `width: 100cqw` box.
    `cqw` scales when that measures over 150 px (200 and 100 were measured). The probe has a
    width of its own, so a window not yet shown shouldn't change it (reasoned; V5 restarts at
    150% on each engine).
- **The four tied lengths** (candidate A2, which passed Task 1b on all three engines):
  - top padding: no rule changes; it follows `--content-pad`;
  - `scroll-margin-top: calc(var(--bar-h, 3rem) / var(--doc-zoom, 1) + var(--content-pad))`.
    `--bar-h` is set in `px` by `measureBar()`;
  - step-out limit: `max-width: calc(100cqw / var(--cq-zoom, 1) - 2 * var(--content-pad))`;
  - the centring `calc()`: both `100%` become `100cqw / var(--cq-zoom, 1)`, in the
    `.markdown-body` rule itself (`base.css:1706-1710`). The pane is the container and has no
    padding, so at 100% the two are the same length (C1).
- **Asking for a step.** `stepDocZoom(dir)` invokes `step_doc_zoom`. The level is applied when
  the event comes back, as the document font is, so every window ends on the last save. A step
  from a key or the wheel says the result in the toast (`Zoom 125%`); a step from Settings
  doesn't, since the row shows it.
- **Keeping the reader's place, in the document on screen** (`#content` not hidden). In
  `showDocZoom`, when the event arrives, since that is when the layout changes. Before the
  properties are set, an anchor is taken at one point:
  - for a wheel step, the pointer's position. `stepDocZoom(dir, point)` keeps `point` for
    every `showDocZoom` in that window until a key or a Settings step there, which has none
    and clears it, or a resize (`onResize`), after which it may lie outside the window.
    Kept, not used once (change review, pass 1): a turn's steps overlap, each applied when
    Rust answers, and using the point once left every step after the first to the top of
    the view. A level set from another window so anchors on the last wheel
    step's point when there was one: still a place on screen held still;
  - otherwise the top of the view: across, the middle of `main`'s rectangle; down, the page
    padding below the bar's bottom, or below the find bar's when that is lower
    (`els.find.getBoundingClientRect().bottom`, as `firstShownHit` reads it, `app.js:3713`).

  The anchor is:
  1. the caret range from `document.caretRangeFromPoint`, if it starts inside `#content` and
     its rectangle isn't all zeros;
  2. else the first child of `#content` whose rectangle's bottom is below the point; then,
     while a child of that element has the point between its rectangle's top and bottom, the
     first such child instead; with how far down the last one's rectangle the point falls, as
     a fraction of its height;
  3. else nothing, and `window.scrollY` is multiplied by the ratio of the two levels.

  Step 2 reads the layout and tests no hit, because a hit test can't reach the document when
  something lies over it: Settings is a modal dialog (`app.js:5695`), which makes the page
  behind it inert, and the tab switcher hangs under the bar. Reasoned, not measured; it is
  also what holds a heading's button and a table's white space, where M7 found no caret. The
  descent takes any child, with no filter for blocks: a list gives up its item, a table its
  row and then a cell, a paragraph a link or a `code` on that line, a drawing its shapes. Each
  of those moves with the layout, and a hidden one has no height and is never taken. Text
  re-wraps, so the result should be a line or two off where the caret range is under 1 px
  (M7). The caret range was measured; this was not, and V3 and V6 record it in px. The anchor
  is read and used inside one synchronous `showDocZoom`, so nothing can replace it between.

  After the properties are set the page is scrolled by how far the anchor moved: the caret
  range's rectangle, or the same fraction down the child's new rectangle.
  A caret range whose rectangle's top is within 2 px of where the last step aimed its line
  is taken as that line again and goes back to that aim, not to where it ended; the aim is
  dropped when the page's scroll offset or `#content`'s height is no longer as the last step
  left it (another document, a scroll, a new layout), and by a step that holds no caret
  range (added after the walk: on WebKit a step ends up to 1.2 px off, the same way each
  time, and a turn of eight steps added up to 5.5 px on macOS; the cause is reasoned to be
  its whole-px scroll, not measured).
  A restore still holding the page is let go first, as find does (`app.js:3748`). Then the
  place is banked and the session reported directly (`rememberScroll`, `scheduleReport`): the
  scroll may fire no scroll event (`app.js:1880-1882`). No report is made from a window with
  no tab: it has nothing to say (added in the build).
- **Keeping the place everywhere else.** Other documents aren't laid out, so every history
  entry of every tab and every closed tab (`closedTabs`, `app.js:278`) whose `scrollY` is a
  number has it multiplied by the ratio of the two levels, and the session is reported again.
  The entry on screen is left to the anchor. That is `shownEntry` (`app.js:324`), not the
  active tab's current entry: while a tab switch loads, the page still shows the entry being
  left. When no document is on screen (a picture tab, the empty or the error panel) there is
  no anchor and every entry is scaled. An entry with
  `scrollY: null` is waiting to jump to its heading (`app.js:1833`, `3350`, `3470`) and stays
  `null`. Measured drift of a scaled offset: 16 to 254 px, a limit for triage. Nothing is
  scaled at boot: a restored session's offsets were banked at the saved level.
- **Find's sideways scroll.** `scrollToHit` adds a distance in window px to a `scrollLeft` in
  unzoomed px (`app.js:3742`), for a code block and a table alike (`app.js:3739`). The
  distance is divided by `state.docZoom / 100`. That `scrollLeft` is in unzoomed px is M2's;
  V7 checks the result.
- **Keys.** In `onKeydown`, after the modal guard, where the picture's zoom keys are
  (`app.js:5626-5642`): with a picture open they do what they do today; else, when `#content`
  isn't hidden (`!els.content.hidden`), they call `stepDocZoom`. So they do nothing on the
  empty and error panels, nor in a picture tab whose picture is still loading.
- **Which wheel turns zoom.** One test, shared by the document, the picture
  (`onImageWheel`, `app.js:5370`) and the diagram (`onDiagramWheel`, `app.js:1648`), in place
  of their `event.ctrlKey`: on Windows and Linux, `event.ctrlKey`; on macOS (`isMac`,
  `app.js:282`), `event.ctrlKey` within 100 ms of the last gesture event. One listener on
  `window` for `gesturestart`, `gesturechange` and `gestureend` keeps `performance.now()`,
  and the test reads `performance.now()` again: one clock, whatever each kind of event
  stamps itself with. Every wheel event of a pinch follows a gesture event in the same millisecond,
  the last one after `gestureend` (C8, shown for the first and last ten events of a pinch),
  and a `Control`+wheel notch has none. A time, not a flag, so nothing sticks if a
  `gestureend` never comes.
- **Wheel.** One listener on `window`, not passive. It acts only on a turn that passes that
  test while `#content` is shown, and whose target is inside `main` or is `#body-row`, `body`
  or `html`: the empty space below a document shorter than the window (`base.css:541-544`).
  Or is the toast, which a step leaves over the pane for 4 s (change review, pass 1).
  The event's target, not the pointer's coordinates: under the diagram overlay or Settings
  the target is the dialog. So the picture view, the diagram overlay, the
  sidebar and Settings are untouched. It adds up `deltaY`, normalised as `onImageWheel` does
  (`app.js:5373`). When the sum reaches 40 px either way it takes one step and starts again
  from zero; a turn the other way, or a pause of over 300 ms, also starts from zero. So one
  notch is one step at 40, 67, 96 and 100 px alike, and a touchpad's stream of small turns
  steps once per 40 px.
  - `Ctrl`+wheel over a document scrolls it today; on Windows and Linux it will zoom instead.
  - On macOS `Control`+wheel will scroll a document, a picture and a diagram, as a plain turn
    does.
- **A pinch's rate (macOS; decision 2 after Task 1b).** Two named constants, since the feel is
  the owner's hand to judge in Task 6, and `Ctrl`+wheel on Windows and Linux keeps today's:
  - a picture and a diagram: `Math.exp(-dy * 0.01)` for a pinch in place of today's `0.002`
    (`app.js:1651`, `5374`), so the picture grows by what the fingers spread (100 px of
    `deltaY` for each factor of *e*, C8);
  - a document: one step for every 20 px of a pinch in place of 40, so a spread to 3 times
    goes from 100% to about 200%, and the smallest pinch measured (23 px) is one step.
- **Diagram labels.** In the diagram frame, after `mermaid.render` (`diagram-frame.js:29`)
  and before the drawing is sent back: the SVG string is set as the `innerHTML` of a `div`
  added to the frame's `body` (an HTML parse, as the page's own is, and attached, so computed
  styles exist), every `x`, `y`, `dx` and `dy` on a `text` or `tspan` that is one `em` length
  becomes that number times the element's computed font size, the `div`'s `innerHTML` is
  read back as the string to send, and the `div` is removed. The frame
  is where mermaid measured the labels, it is never zoomed, and the page caches what it
  sends (`app.js:1129-1133`), so each drawing is rewritten once. Two things here are
  reasoned, and V8 checks both: that one more HTML parse and write changes nothing else (the
  page already does one of each, `app.js:1083`, `1129`), and that the frame's font size is
  the page's, so nothing moves at 100% (the size is set by the drawing's own styles;
  `base.css` sets none on a diagram's text). The full-window view shows a copy of the same drawing and gets the same numbers.
- **Settings.** A *Document zoom* fieldset in Appearance, after *Document font*
  (`index.html:264-274`): the hint "Makes every document bigger or smaller, in every
  window.", then − (`aria-label` "Zoom out"), the level (`role="status"`), + ("Zoom in") and
  *Reset*. The three are `type="button"`: they sit in the dialog's form, where a plain button
  would close it. The document behind the dialog previews the
  change, as it does for a theme. The buttons stay enabled at the ends of the ladder, as the
  picture toolbar's do.
- **No correction is chosen by platform.** The one engine fact the page needs, whether `cqw`
  scales, comes from the probe: an older WebKit than the ones measured may answer differently.

### Not built

- A level per tab or per file (ruling 2), buttons on the bar (ruling 3).
- Disabling − and + at the ladder's ends, and a typed-in percentage.
- Any other change to how pictures and diagrams zoom: only the test for which wheel turns
  count changes (ruling 4), and a pinch's rate on macOS (decision 2 after Task 1b). The
  full-window diagram view doesn't follow the document's level.
- A throttle on steps. Steps are asked for without waiting for the last one's answer; the
  command is a plain `fn` like `set_doc_font`, and Tauri runs those on the main thread one at
  a time (`config.rs:201-202`), so a burst counts right. V1 checks a fast burst.
- Candidate B, and any use of a gesture event's own `scale`: the wheel events carry the pinch.

## Global constraints

- Gates before each commit, from the repo root: `cargo fmt --manifest-path src-tauri/Cargo.toml
  --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked
  -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`, and
  `node --check src/app.js`.
- Tests: 215 today on Windows and macOS, 217 on Linux. Task 3 adds two.
- Straight onto `main`, committed locally. No push, branch, tag or delete without the owner's go
  at that moment. No AI attribution.
- The app is driven on the VMs only (`md-viewer-win`, `md-viewer-lin`, `md-viewer-mac`), never on
  the host PC. A peer backs up `config.json` and `session.json` before a run and restores them
  after it, wherever the run can reach the real ones (Windows does).
- No new dependency and no new Tauri permission: `step_doc_zoom` is the app's own command.

## Task 1: measuring walk (done 2026-11-12)

The debug build of `main` at 98919db on each VM, with the zoom put on by script
(`document.getElementById("content").style.zoom = z`) at 0.5, 1.5 and 3, on
`examples/kitchen-sink.md` and `examples/widths.md`, 1400 and 700 px wide. Fifteen measures,
M1 to M15; the numbers are in *Task 1 results*.

## Task 1b: trial of the corrections (no code change; done 2026-11-12)

The numbers are in *Task 1b results*. No stop condition fired. What was asked:

The same build, on each VM. The candidate's rules go in by script as a `<style>` at the end of
`<head>`, and the three properties are set on `#content` by script, as `showDocZoom` will.
Levels 0.5, 1.5 and 3, 1400 and 700 px wide, on `examples/kitchen-sink.md`,
`examples/widths.md` and a scratch document with a far heading.

- **A1:** the `zoom` rule, the `scroll-margin-top` rule and the step-out limit from *Design*.
  The centring `calc()` is left as it is.
- **A2:** A1 plus the centring `calc()` on `cqw`.
- **B, only where A2 fails C3:** `zoom: var(--doc-zoom, 1)` on `#content > *` instead of
  `#content`, so the padding and the centring stay outside the zoom. In the centring, each
  measure is multiplied by `var(--doc-zoom, 1)`. `--content-pad` is not overridden; the divided
  padding goes in as `--pad-z`, which the step-out limit and the `scroll-margin-top` use in
  its place.

| # | What is measured | Passes when |
|---|---|---|
| C0 | A unit table: inside a `zoom: 1.5` box, the drawn width of `100px`, `10rem`, `10em`, `10ch`, `10cqw`, `10vw`, `width: 50%`, `padding-left: 10%`, `padding-left: calc(10% + 0px)`, `padding-left: calc(10% - 1ch)` and `width: calc(10ch + 0px)` | Recorded; explains any failure below |
| C1 | With the rules in and no property set (100%): the first long paragraph and every step-out in `widths.md`, against the same without the rules | Identical to the px, both widths |
| C2 | First line's top below the bar's bottom, at the top of a document | 40 px, within 1, at every level |
| C3 | The first long paragraph: its left edge, and characters a line (its width over a tenth of the width of ten zeros set in it) | Left edge at 40 px or more; 75 or 100 characters, within 1, or the paragraph is the pane less 80 px |
| C4 | Every step-out in `widths.md` | None passes 40 px from either edge of the pane, within 1; a wide table scrolled to its end shows its last column |
| C5 | A click on the link to the far heading | Heading 40 px below the bar, within 2, at every level |
| C6 | Sideways page scroll | None |
| C7 | Screenshots: `kitchen-sink.md` table and diagram sections at 150% (both widths) and 300%, and the top at 50% | The owner's eye |
| C8 | macOS only, on a document and on a picture: a pinch out and back in, then a `Control`+wheel notch. For each, every `gesturestart`, `gesturechange` and `gestureend` seen on `window`, and every wheel event's `deltaY`, `deltaMode` and `ctrlKey`, in order, with the count and the sum of `deltaY` for one full pinch. If the VM can't make a pinch, the peer says so and the owner is asked to make one by hand | A pinch can be told from `Control`+wheel by something the page can read |

Stop and ask the owner when A2 and B both fail C2 to C5 on any engine, C1 fails for A2, or C8
finds nothing that tells a pinch from `Control`+wheel.

After the trial: A2 is in *Design* and Task 4.

## Task 1c: trial of the diagram labels (no code change; done 2026-11-12)

The numbers are in *Task 1c results*. T3 passed wherever T2 failed. What was asked:

The same build on each VM, 1400 px wide, with A2's rules and properties put in by script. On
`examples/kitchen-sink.md`, `examples/widths.md` (flowcharts and a sequence diagram) and a
scratch document with one small diagram each of: flowchart with a two-line label, sequence,
class, state, ER, gantt and pie.

| # | What is measured | Passes when |
|---|---|---|
| T1 | At 100%: where every label's centre sits in its drawing, as a fraction of the drawing's box. And every attribute in the drawings with a length in `em`, `ex`, `ch`, `rem` or `%` | Recorded |
| T2 | At 50, 150 and 300%: how far each label is from where T1's fraction puts it | No label over 1 px off |
| T3 | Only where T2 fails. At 100%, every `x`, `y`, `dx` and `dy` that is a single `em` length is rewritten as that number times the element's computed font size. Then T2's measure at 100, 50, 150 and 300% | Nothing moves at 100%; no label over 1 px off at the others |
| T4 | Only where T3 fails: up to half an hour on what moves the label | Reported |
| T5 | Screenshots of the diagrams at 150% and 300%, before and after the rewrite | The owner's eye |

After it: the rewrite is in *Design* (diagram labels) and Task 4.

## Task 2: the pinch on macOS, for a picture and a diagram

Shipped behaviour changing (ruling 4, decision 2), so it is its own commit, and the first: the
document's wheel in Task 4 uses the same test. Nothing changes on Windows and Linux.

1. `app.js`: the gesture listener and the shared wheel test (*Design*, which wheel turns
   zoom), used in `onImageWheel` and `onDiagramWheel` in place of their `event.ctrlKey`; and
   the pinch's rate there, `0.01` on macOS beside today's `0.002`, as a named constant.
2. `README.md:236-238`: on macOS `Ctrl`+wheel is a pinch, and `Control`+wheel scrolls.
3. Gates.
4. Commit locally: `On macOS, Control+wheel now scrolls and a pinch follows the fingers`,
   with the body `Control+wheel no longer zooms a picture or a diagram there. A pinch still
   does, five times as fast.` wrapped at 72.

## Task 3: the setting, in Rust

1. `config.rs`: `ZOOM_LADDER`, `doc_zoom`, `doc_zoom_step`, the `doc_zoom` field with its
   default, and the clean in `load()`.
2. Two tests: an old config and an off-ladder value both load as 100 while each ladder value is
   kept; stepping moves one rung, stops at 50 and 300, and `0` gives 100.
3. `main.rs`: `step_doc_zoom`, registered in the handler list.
4. Gates. Expect 217 tests on Windows.

## Task 4: the page

1. `base.css`: `zoom: var(--doc-zoom, 1)` on `#content`, and the three rules of A2 (*Design*,
   the four tied lengths): `base.css:1706-1710`, `1726` and `2178`.
2. `app.js`: `state.docZoom`, the `cqw` probe, `showDocZoom`, `stepDocZoom`, the place-keeping,
   the scaling of banked offsets, the boot call after `showDocFont`, and the listener beside
   `doc-font-changed`.
3. `app.js`: the three keys, the document's wheel listener with its 20 px step on macOS, and
   the division in `scrollToHit`.
4. `index.html` and `app.js`: the Settings fieldset and its four controls.
5. `diagram-frame.js`: the rewrite of a drawing's `em` lengths.
6. Gates, and `node --check src/diagram-frame.js`.

## Task 5: words

- `README.md`: the keyboard table's zoom row (`README.md:258`: in, out and back to 100% on a
  document; in, out and fit on a picture or a diagram), `Ctrl`+wheel on a document beside the
  picture paragraph (`265-268`), a line where the features are listed, and a short *Document
  zoom* section after *Document font* (`437-448`): what zooms, the keys, the wheel and the
  pinch, the Settings row, one level for every window, kept across restarts.
- Gates, then Tasks 3 to 5 are committed locally as one, a release-note line:
  `Zoom a document with the keyboard, the wheel or a pinch`. The keys differ by platform and
  a release page has no note to say so, so they go in the body. Its body is three lines with
  a blank line between them: `Ctrl++, Ctrl+- and Ctrl+0 (Cmd on macOS); Ctrl+wheel, or a
  macOS pinch.` (71 characters), `One level for every document and window, kept across restarts.` and `A
  Document zoom row in Settings shows and changes it.`

## Task 6: change review, then the verifying walk

1. The change review loop on the two commits: review, fix, review again, until a pass finds
   nothing. Each fix is its own local commit until Package.
2. The walk below.
3. A fix the walk leads to goes through the review loop too. Then the V checks it touches
   are walked again, on the engines it touches, from the branch pushed again on the owner's
   go.

From a `walk/doc-zoom` branch, pushed on the owner's go, on all three VMs. The Linux and macOS
gates run there too (expect 219 and 217 tests). The feature writes `doc_zoom` to `config.json`,
so each peer backs the real file up first and restores it after. Documents, unless a step
names others: `examples/kitchen-sink.md`, `examples/widths.md` and Task 1c's seven-diagram
document, 1400 and 700 px wide.

- **V1** each key and the wheel step the level on a document; the toast says it; `Ctrl+0` gives
  100%; the ends of the ladder hold; five steps asked for at once land five rungs away. The
  keys do nothing on the empty panel and on the error panel. The wheel also steps with the
  pointer in the empty space below a document shorter than the window, and not with it over
  the sidebar or the bar. On macOS the wheel's part is a pinch, made by the owner's hand: a
  script can't make one.
- **V2** the same keys in a picture tab and in the diagram overlay do only what they did before.
  So does `Ctrl`+wheel there on Windows and Linux.
- **V2m** macOS: `Control`+wheel scrolls and zooms nothing on a document, a picture and a
  diagram; a pinch zooms each, about the cursor on a picture and a diagram as before. The
  owner pinches all three by hand and judges the two rates (*Design*, a pinch's rate).
- **V3** Settings shows the level, −, + and Reset work without closing the dialog, and the
  document behind follows. How far the line of text at the top of the view moves at each
  step is recorded in px; it passes within two lines of text at the new level. Settings open
  in a second window shows the new level too.
- **V4** a second window follows a change made in the first.
- **V5** after a restart at 150%: the level is kept, restored tabs come back to their lines,
  and C3 and C4 hold (the probe, run at boot).
- **V6** the place holds through a change:
  - on screen, from a key (M7's lines): with the top of the view on text, on a heading's
    button and on white space in a table; with the find bar open; with the sidebar open;
    with the tab switcher open. Where no caret is found (the button, the white space, the
    switcher) the move of the top line is recorded in px and passes within two lines of text
    at the new level. With the switcher open, that includes a long list and a tall table at
    the top of the view, 700 px wide;
  - on screen, from the wheel, with the pointer on text: the line under the pointer stays
    under it, within 2 px;
  - just after a tab switch to a document with pictures, while the restore still holds;
  - a hidden tab, a tab reopened with `Ctrl+Shift+T`, and a document tab left for a picture
    tab in which the level was changed: each within the drift Task 1 measured;
  - a link to another file's heading, followed in the same tab, the level changed after it
    lands: the heading stays 40 px below the bar, within 2. (A link can't be opened in a
    tab that is not shown, so an entry with no position yet is always the one on screen.)
- **V7** C2 to C6 and M6, M8 to M12 again, on the real feature. And:
  - find's next match inside a wide code block and inside a wide table, at 150% and 300%:
    the match's centre is within 2 px of the box's centre, where the box can scroll that far;
  - a heading that fits on one line is 1.5 times as tall at 150% as at 100%, within 1 px,
    at 1400 px on each engine;
  - a click on a sequence diagram's actor at 150% and 300% opens its menu just below the
    click, as at 100%.
- **V7d** Task 1c's T1 and T2 on the real feature: no `em` length is left in a drawing, no
  label is over 1 px off up and down at 50, 150 and 300%, and the full-window view of a
  diagram looks as on 1.8.11.
- **V8** at 100%, against a build of 98919db on the same VM: the three documents at both
  widths are drawn the same (screenshots compared), and every diagram label sits where it
  did (Task 1c's `labels()`, within 0.5 px).
- **V9** a `doc_zoom` of 137 typed into `config.json` loads as 100%.

## Task 7: close out

- Triage with the owner, one item at a time. Known to come up:
  - a level change in the one frame between a document being drawn and its place being
    restored, which restores the old level's offset (reasoned from `app.js:1823-1839`);
  - the place being a line or two off wherever no caret is found: a step made from Settings
    or under the tab switcher, or with the point on a heading's button or a table's white
    space. On text, a key or the wheel holds it to the px;
  - a hidden tab's place being approximate after a change of level (16 to 254 px measured);
  - a session offered for reopening on the empty panel, then a change of level before it is
    taken: its offsets were banked at the old level (reasoned);
  - macOS 13 being unmeasured;
  - a level change on a very large JSON file taking 0.2 to 1 s, and one large pinch there
    queueing about 2 s of steps on macOS (reasoned from M14 and the 20 px step);
  - WebKit not shrinking body text below 9 px at 50% (measured on Linux);
  - Linux step-out boxes scrolling 1 px sideways at 150% (still there under A2);
  - a diagram label anchored at its left edge sitting up to 7.5 px left at 300% on Linux, and
    2 px on Windows (Task 1c);
  - a diagram wider than the pane staying the pane's width at first, then growing and
    scrolling in its box (Task 1c);
  - the full-window diagram view not following the document's level;
  - a real mouse wheel on a scaled Windows display being unmeasured;
  - a touchpad pinch on Windows and Linux being unmeasured (it zooms wherever the webview
    reports it as `Ctrl`+wheel);
  - a reader's own theme that turns the measure off with `100vw`
    (`src-tauri/themes/README.md:61-63`): zoomed out, it may get a limit back on Windows and
    macOS, where `vw` scales (C0). The effect itself is reasoned, not run. No bundled theme
    sets a measure;
  - whether *A size for the document font* (`open-items.md`) closes, stays, or is reworded.
- After triage, one local `docs:` commit: the deferred and accepted items in
  `docs/open-items.md` and `docs/closed-items.md`, and this plan archived with a status note.
- Package: the fixes from the review loop and the walk are squashed into the commit they
  belong to, Task 2's or Task 5's. The squash is rehearsed in a throwaway worktree, and
  `main` is moved only once the rehearsed tree is identical to it.
- Asked separately, each at its moment: pushing `main`, deleting `walk/doc-zoom`, and the
  release. By the project's pattern this is a new capability, so a minor release; the owner
  names the number.

## Execution

- **Tasks 1, 1b and 1c:** done, by the three peers on the main session's briefs.
- **Tasks 2 to 5:** a `coder` agent, serially, with the main session reviewing each diff and its
  gate output.
- **Task 6:** the main session reviews; the peers walk. **Task 7:** the main session.

Stop and ask on: any gate failing; a walk step behaving differently from *Design*; a need the
design doesn't cover.
