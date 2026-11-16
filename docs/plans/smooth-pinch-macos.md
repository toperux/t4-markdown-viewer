# Smooth Pinch on macOS Plan

> **Status (2026-11-16):** written and reviewed (fourteen findings, then eight, then two, all
> taken; one ruling by the owner; *Review* at the end). Nothing is built, and no trial has run:
> the trial waits for the owner's go.

**Goal:** on macOS, a pinch on a document zooms it smoothly, following the fingers, where today
it walks the zoom levels in steps.

**Spec:** the deferred item in `open-items.md` (*On macOS, a pinch zooms a document in steps,
not smoothly*, 2026-11-13), which the owner ruled to follow the document-zoom release, and the
owner's roadmap ("then the smooth pinch on macOS"). The owner's words on today's pinch: "chunky
or not smooth".

**Not in scope:** the keys, the wheel on Windows and Linux, the Settings row, and the pinch on a
picture or in the full-window diagram view, which already follows the fingers.

## Rulings (the owner)

1. **One approach for every document** (2026-11-16). Turned down: live zoom for documents that
   are fast enough and scaling for the rest (two code paths and a rule for when to switch), and
   leaving that open until after the trial.

## Findings (read 2026-11-16, at `57a7e79`; the measured numbers are from earlier walks)

### How a pinch arrives

From `archive/plans/document-zoom.md`, check C8, 23 pinches by the owner's hand on the Mac:

- `gesturestart`, then a `gesturechange` and a wheel event by turns, then `gestureend`, which
  comes just before the pinch's **last** wheel event, in the same millisecond. Every wheel
  event has `ctrlKey` true and a small `deltaY` (0.1 to 11 px).
- `deltaY` adds up to about 100 px for each factor of *e* the fingers spread: −112 px for a
  spread to 3 times, −23 px for 1.26 times, +125 px for a squeeze to 0.27.
- A real `Control`+wheel notch has no gesture event beside it. `isZoomWheel` (`app.js:304`)
  tells the two apart by the time since the last gesture event (100 ms), so a lost `gestureend`
  leaves nothing stuck.

### What a pinch does today

- `onDocWheel` (`app.js:4475`) adds up `deltaY`. Every 20 px either way (`DOC_WHEEL_STEP`) it
  asks Rust for one step (`step_doc_zoom`, `main.rs:1368`). Rust moves one rung from the saved
  level on the ladder of 13 (`ZOOM_LADDER`, `config.rs:74`: 50 to 300%), saves it and tells
  every window. It is the only command that changes the level.
- Each window then applies it (`showDocZoom`, `app.js:4403`): `--doc-zoom` on `#content`, which
  `base.css:1696` turns into the CSS `zoom` property. That is a full re-layout of the document.
  The line of text under the pointer is held where it was (`zoomAnchor`), and a toast names the
  level (`app.js:4489`).
- So one pinch is three to six steps: each a saved file, a broadcast, a re-layout in every
  window and a toast, and the page moves a rung at a time (5 to 50 points). The owner's 16
  pinches moved 3 to 6 levels each, about one step fewer than the fingers asked for.
- The saved level must be on the ladder: `config::doc_zoom` turns anything else into 100, and a
  test holds it to that (`config.rs:411`, with 137).
- A step on empty space leaves the page about 0.4 px lower on macOS each time (`open-items.md`,
  *a burst of zoom steps on empty space creeps a few px*), an accepted limit whose reopen
  trigger is this plan.

### What a re-layout costs (measured earlier)

- A level change on the README: 5 to 50 ms, one range for all three engines; the Mac's own
  figure was not kept (`archive/plans/document-zoom.md`, M14).
- A full re-layout of the README thirty times over: 109 to 114 ms on the Mac (the
  centred-column walks).
- A level change on a 1.8 MB JSON file: 0.2 to 0.35 s on Windows and macOS.
- A full re-layout of the first chunk of a JSON file of about 10 MB: 274 to 277 ms on the Mac
  (`open-items.md`, *A full re-layout may be slightly slower on Windows*).

So a re-layout per frame cannot keep up on a large document (*reasoned from those numbers*):
Z is expected to fail the trial's bar on the two large documents.

### What is not known

- Whether a page scaled by a CSS transform stays sharp while the fingers move.
- How visible the change is when the pinch ends and the real level is applied.
- Whether a real pinch's events come faster than one pair per frame.

So the design is settled by a trial first (Task 1), as the document zoom's was.

## The two candidates

Both follow the fingers at the rate a picture already does: the level is the level at the
pinch's start times `Math.exp(-sum of deltaY * 0.01)` (`WHEEL_ZOOM_RATE`), kept inside 50 to
300% and rounded to a whole percent. `deltaY` is summed, as the picture's handler does, and
`gesturechange`'s own `scale` is not read: the two agree within about 5% on the measured
pinches (e^1.12 = 3.06 against a spread to 3.0), and one source keeps the document and the
picture alike.

Common to both:

- **The handler only adds up.** The wheel listener sums `deltaY`; one `requestAnimationFrame`
  callback per frame applies the level. Two events in one frame cost one update.
- **A pinch belongs to where it starts.** Whether it is a document's pinch is decided at its
  first zoom wheel event, by today's test of the target (`app.js:4478`); the rest of its events
  count wherever the pointer drifts.
- **What is held.** The point under the pointer at the pinch's start: the line of text there
  and how far down its height the pointer was, as `zoomAnchor`'s fallback already holds a point
  of an element (`app.js:4391-4393`). It is taken once and kept at the pointer's starting
  height for the whole pinch, so it cannot creep. A step today holds the line's top instead,
  which is right for one step; under a pinch the fingers' point would slide down a growing
  line.
- **At the pinch's start** a running scroll hold is aborted (`scrollHold`, as `showDocZoom`
  does at `app.js:4438`): after a tab switch it would pull the page back at every change of
  height.
- **At the pinch's end, once:** what `showDocZoom` does per step. `state.docZoom`; the other
  tabs' saved offsets scaled by the end level over the start level; `rememberScroll`;
  `scheduleReport`; `zoomHeld` cleared; the level shown in Settings.
- **What ends a pinch.** One frame after `gestureend`, so that the last wheel event, which comes
  after it, is taken in; or 200 ms with no zoom wheel event. A zoom wheel event after the end
  with no new `gesturestart` is dropped.
- **Other windows and the file** are left alone until the pinch ends: one save and one
  broadcast per pinch.

**Candidate Z, the real zoom, live.** Each frame sets `--doc-zoom` (and the two properties that
go with it) to the level the fingers are at, then scrolls the held point back to its height.
Nothing else per frame.

- For: the text is sharp and wrapped right at every moment; nothing changes when the pinch
  ends.
- Against: one re-layout per frame, which the numbers above say a large document cannot keep
  up with. With ruling 1 it is built only if it passes on every document of the trial.

**Candidate T, a transform, then one zoom.** During the pinch `#content` gets
`transform: scale(s)`, `s` being the level the fingers are at over the level at the start. No
re-layout. When the pinch ends the transform is taken off and the real level applied once, the
held point put back where it stood on screen just before.

- The origin of the scale, fixed for the whole pinch: across, the middle of `#content`
  (written as `50%`); down, the held point. `#content` spans the pane and pads both sides
  alike, so its middle is the column's, the sidebar open or not (*read*). The real zoom keeps
  the column centred and the page never scrolls sideways, so while the column fits the pane,
  scaling about the middle ends where the real zoom will put the text; the point under the
  fingers stays at its height and slides sideways a little while they move.
- While the transform is on, `main` gets `overflow-x: clip`. Without it a spread would give the
  page a sideways scroll range and paint the document over the sidebar: `#content`'s own
  `overflow-x: clip` (`base.css:1724`) scales with it, and `main`, `#body-row` and `body` clip
  nothing (*reasoned, not measured*).
- For: the cost per frame should not grow with the document (*reasoned*).
- Against: until the pinch ends, text may look soft; what the pane limits (a table, a diagram,
  a picture, or the column in a narrow window or at a high level) keeps its old wrapping, and
  during a spread is cut at the pane's sides by the clip; a squeeze shows empty space around
  the page, and a spread lengthens the page's scroll range for its duration; the end is a
  visible change where anything re-wraps. Where the column is limited
  by the reading width alone, the wrapping is the same after the real zoom (*reasoned*: `ch`
  scales with it).

Nothing inside `#content` is `sticky` or `fixed` in `base.css` or the bundled themes, so a
transform on it moves no pinned element (*read*; a reader's own theme could add one). Find's
highlights are painted with the text under both candidates, and diagrams do not redraw on a
change of level.

## The level a pinch ends on: for the owner, after the trial

Both endings need a new command that saves a level the page names: `step_doc_zoom` moves one
rung from the saved level, and a pinch ends several rungs away.

- **On the ladder.** The level snaps to the nearest of the 13 when the pinch ends.
  `config::doc_zoom` stays as it is. The page changes size once more at the end, by up to half
  a rung: from 137% to 125%.
- **Where the fingers left it.** The level is saved as a whole percent between 50 and 300. No
  change of size at the end. On top of the command it costs: `config::doc_zoom` takes any whole
  percent in that range, its test and its doc comment (`config.rs:124`) change, and
  `doc_zoom_step`'s test gains an off-ladder case (it already takes 137% to 150% up and 125%
  down); Settings and the toast show "137%" (they print any whole number); the archived walk's
  "137 in `config.json` loads as 100%" no longer holds.

Under either, `README.md` changes where it says a pinch zooms "in steps" (lines 56-58, 269-270,
459-463).

## Task 1: the trial (macOS, by script, no repo change)

On a debug build of `main` with the `webdriver` feature, as drive-app needs on macOS; the
release has no driver. Both candidates are put in by script, the app's own wheel listener
taken off first (`removeEventListener("wheel", onDocWheel, { passive: false })`), with a switch
between Z and T and between the two endings, and one that holds a pinch open (no ending until
it is let go), for a picture mid-pinch. The candidates save nothing: the trial keeps its level
in the page. The baseline, today's stepped pinch, does save and broadcast, to the scratch
`config.json`; the level is put back to 100% in Rust and the page before every run.

Documents:

- `README.md`, `examples/kitchen-sink.md` and `examples/widths.md` (eleven picture and diagram
  blocks and an inline icon);
- the README thirty times over, written as a scratch file at the repo's root so its pictures
  resolve, and removed afterwards;
- a JSON file of about 10 MB made by script in a scratch folder: its first chunk for every
  run, and every chunk loaded for T's ending only.

Before any timing: the app in front (`front()`), and the interval between idle
`requestAnimationFrame` callbacks recorded as the frame, F (8.3 ms on a 120 Hz display, more in
Low Power Mode). Long runs go in the background and are polled, so the driver's script timeout
cannot cut them.

- **M0, a real pinch reaches this build.** One pinch by the owner's hand, the events logged:
  `gesturestart`, `gesturechange`, `gestureend` and the wheel events arrive as in C8, and how
  many pairs come per frame.
- **M1, the cost per frame, by script.** A pinch made of dispatched events (`gesturechange`
  then a wheel event with `ctrlKey`), for Z, for T, and for today's stepped pinch as the
  baseline. On each document:
  - a spread of 50 pairs of −2 px (to about 272%) and a squeeze of 35 pairs of +2 px, from 100%;
  - a spread of 34 pairs of −2 px from 150% (to about 296%);
  - at one pair per frame, and at three pairs per frame (or M0's rate, if higher).

  And on `README.md`: with the sidebar open; in a 700 px window, from 100% and from 150%; and
  with the pointer on empty space beside a heading. Report, per run: each frame's length
  (median, worst, the share over 1.5 F); the lag from the last event to the page standing
  still; and, on the pinch's last frame before the ending, how far the held point is from its
  starting height (px). `requestAnimationFrame` times the page's own thread, not the
  compositor, so T's figures may flatter it: M3 is the check on that.
- **M2, the end of the pinch, by script.** After each M1 run: how long the ending takes, and
  how far the held point moved across it, down and across (px), for both endings. For T, with
  the pinch held open mid-spread: `scrollWidth` against `clientWidth` of the page (equal: no
  sideways scroll), whether putting the clip on `main` changed any layout (it should not), and
  a screenshot with the sidebar open.
- **M3, by the owner's hand.** On `README.md` (also in a 700 px window at 150%), `widths.md`
  and the large README, for Z and T and both endings: a spread, a squeeze, and a spread
  followed at once by a squeeze. The owner says which is smooth, whether T's text looks soft
  while it moves, and how each ending looks.
- **M4, the edges, by script.** A squeeze at 50% and a spread at 300% do nothing and leave
  nothing stuck. A pinch that starts over the sidebar, the bar or Settings does nothing. A pinch
  that starts on the document and drifts over the sidebar goes on. A pinch with `gestureend`
  withheld still ends, after 200 ms. Each pinch ends exactly once (one call of the ending, which
  is where the save will be).

**The bar, set before measuring** (ruling 1: a candidate is built only if it passes on every
document):

- on `README.md`, `kitchen-sink.md` and `widths.md`, in every M1 run on them (the sidebar, the
  700 px window, both starting levels, both rates): at least 95% of frames within 1.5 F, the
  pinch's first frame left out;
- on the large README and the JSON's first chunk: no frame over 100 ms while the fingers move.
  The ending is not one of those frames: for T it is one full re-layout, about 110 and 275 ms
  there by the earlier measurements;
- on the pinch's last frame before the ending, from 100% and from 150%: the held point within
  2 px of its starting height;
- across the ending with the level kept where the fingers left it, the held point within 2 px,
  down the page (the document zoom's own bound for a step). How long an ending takes is
  reported, and judged by the owner's eye;
- and the owner calls it smooth in M3.

The one that passes is built. If both pass, the owner picks. If neither passes, by the numbers
or by the owner's eye, stop and report: ruling 1 is then the owner's to revisit.

## After the trial

The owner rules, one at a time: the candidate, if both pass; the ending; and whether a toast
names the level when the pinch ends. The build is then written into this plan (the code, the
new command, its tests, the README, its walk) and the plan reviewed again before the go to
build.

To settle in the build's design, not in the trial: a level that arrives from another window, a
key or Settings while a pinch is under way; and a live reload part-way through a pinch.

## Limits known now

- macOS only. A touchpad pinch on Windows and Linux reaches the page as `Ctrl`+wheel with
  nothing to tell it from a wheel (`open-items.md`, *Document zoom is unwalked with some real
  input devices*), so it keeps the steps.
- The page follows the fingers up and down only: the column stays centred and the page never
  scrolls sideways, as with the keys.
- The feel is the owner's hand to judge; a script measures frames, not smoothness.
- The trial runs on the owner's Mac, on its display and trackpad. Other Macs are not measured.
- With T, a squeeze may show areas of the page that were never painted, for a moment
  (*reasoned*); M3 looks for it.

## Chosen without asking

- **T scales about the middle of the column, not about the pointer** (review pass 1, finding 4,
  which the reviewer marked for the owner). About the pointer, the text would jump sideways
  when every pinch ends, by the scale's change times the pointer's distance from the middle,
  since the real zoom always centres the column. The owner sees the result in M3 and can
  reverse it there.
- **The bar** (finding 13's other half): in frames of the display's own rate, with a ceiling
  for the large documents, written down before any number is in.

## Review

Pass 1 (a fresh reviewer, read-only), fourteen findings, all taken:

1. Both endings need a command that saves a named level, not only the off-ladder one.
2. The trial needs the `webdriver` debug build; M0 first shows that a real pinch reaches it.
3. T needs a clip on `main`, or a spread scrolls the page sideways and covers the sidebar.
4. Where T's scale is centred (*Chosen without asking*); M2 measures both axes; a 700 px
   window is in the trial.
5. Z's creep on empty space: the held line is taken once per pinch; a run on empty space.
6. The wheel event that follows `gestureend` is taken in before the pinch ends.
7. One update per frame; runs at three pairs per frame; whole percents.
8. The earlier measurements are quoted as they were measured.
9. A rung is 5 to 50 points, not "10 to 25%".
10. M1's spread stays under 300%, and gains a squeeze and a start at 150%.
11. The trial's runs made runnable: the listener taken off, the frame measured, long runs
    polled, the documents' sources, a baseline, `widths.md`.
12. What an off-ladder level costs, in full; the README changes under either ending.
13. The owner's ruling 1, and the bar.
14. Limits and edges added: a pinch keeps its starting target; levels from elsewhere and a
    live reload during a pinch go to the build's design.

Pass 2 (the same reviewer), eight findings, none a decision for the owner; all taken:

1. Both candidates hold the point under the pointer, not the line's top; T's origin is that
   point, fixed, and T's ending puts it back where it stood.
2. T matches the real zoom only while the column fits the pane; otherwise a spread is cut at
   the pane's sides. The origin across is `50%`. A 700 px window at 150% is in M1 and M3.
3. What `showDocZoom` does per step is done once, at the pinch's end; a scroll hold is aborted
   at its start.
4. The bar covers every M1 run and the ending, and says what follows from each outcome.
5. The spread from 150% is 34 pairs, under the 300% clamp.
6. The 10 MB JSON's measured re-layout is quoted.
7. A switch holds a pinch open, for M2's picture.
8. The baseline saves to the scratch config; the level is reset before every run.

Pass 3 (the same reviewer), two findings, both taken: the 100 ms ceiling covers the frames
while the fingers move, not the ending; and the held point's drift during the pinch is
measured and held to 2 px, which the check across the ending alone would have hidden.
