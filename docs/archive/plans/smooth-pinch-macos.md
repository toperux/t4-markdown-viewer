# Smooth Pinch on macOS Plan

> **Status (2026-11-17): done.** Built, walked and triaged; filed here. The record runs in the
> order things happened, so the early sections are overtaken by the late ones: read *Rulings*
> first.
>
> - **What shipped:** a pinch follows the fingers and ends on any whole percent from 50 to
>   300, saved once, with one toast. A document of up to 2 600 elements, about three READMEs,
>   gets the real zoom on every frame (*The mix*); a larger one is scaled as a picture and
>   laid out once at the end (*The build*, with rulings 5 and 6 for a page's ends).
> - **How it got there:** a trial of two candidates and a third found on the way (*Trial
>   results*); the scaled page built, reviewed and walked on all three platforms (*Walk
>   results*, *Re-walk of ruling 6*); the owner's hand finding that the scaled page is not
>   what the level lays out (*What did not match*), and ruling 7 taking back ruling 1; a
>   measurement to place the mix's two numbers (*The cost of a live frame*); the mix built,
>   reviewed and walked on the Mac, the owner's hand setting the cap (*Walk of the mix*,
>   *Re-check at the owner's cap*).
> - **What is left:** *Triage*, and `open-items.md`.

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
2. **The approach is W** (2026-11-16, after the trial): T with `will-change: transform` while
   the pinch is on, the only candidate that passed every line of the bar. Turned down: Z, which
   the owner's hand liked a little better on small documents and called "Not smooth" on the
   large README; going back on ruling 1 for a mix of the two; and building nothing.
3. **A pinch ends where the fingers left it** (2026-11-16): any whole percent from 50 to 300 is
   a level. Turned down: snapping to the nearest of the ladder's 13, which changes the page's
   size once more at the end (272% to 250% in the trial).
4. **One toast when the pinch ends**, "Zoom 137%" (2026-11-16), as a key or a wheel step shows.
   Turned down: no toast, and a number that runs with the fingers (work on every frame that
   the trial did not measure).
5. **A squeeze near a page's top keeps the top where it is** (2026-11-16, change review pass
   1, finding 1). As built first, the page shrank towards the fingers, left a blank band above
   it, and would jump up when the fingers lift, since the real zoom cannot scroll above the
   top: about 130 px for a point 400 px down squeezed to 67% (*reasoned, not seen*). Turned
   down: walking it first, and accepting it as a limit.
6. **The same at a page's end, and for a document shorter than the view** (2026-11-17, after
   the walk). A spread of a short document, the pointer below its text, rose 458 px out of
   view and jumped back when the fingers lifted (*measured*). The page is moved by what the
   real zoom will not be able to scroll, at either end. Turned down: fixing only the pointer
   below the text, and accepting it as a limit.
7. **The real zoom, live, where the document keeps up; the scaled page where it does not**
   (2026-11-17, after the re-walk). This takes back ruling 1. The owner's words on a squeeze of
   the README to 50%: "the jump was more like the zoom did not match after relayout". Measured:
   outside a band of levels the scaled page is not what the real zoom lays out, and the text's
   column comes out 1.3 to 1.9 times wider or 0.8 times narrower when the fingers lift (*What
   did not match*). Turned down: shipping W as built with that as a limit; the same with the
   mix kept as an open item; and comparing the two by hand once more first.

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

## Trial results (2026-11-16, the Mac session, on the owner's MacBookPro18,3)

Run on a `webdriver` debug build of `f5e1a98`, the candidates put in by script; no repo change.
The trial's script, its driver and every per-run table stay in that session's scratch folder
(`scratchpad/pinch/`), kept for the build's walk.

- **The frame.** F = 16.66 ms: the page renders at 60 Hz on the 120 Hz display. So 1.5 F is
  25 ms, where this plan expected 12.5 ms.
- **M0, four pinches by the owner's hand.** They reach the build. The order is `gesturestart`,
  a wheel event, then `gesturechange` and a wheel event by turns, `gestureend`, one last wheel
  event: a wheel event comes before the first `gesturechange`. One to three wheel events per
  frame, mostly two. `deltaY` is 0.1 to 16.3 px; it summed to −87.5, −88.8, −115.5 and −65 px
  against a `scale` of 2.27, 2.31, 2.98 and 1.76, so the rate of 0.01 holds within about 9%.
- **A third candidate, W**, added by the Mac session and run beside T: T with
  `will-change: transform` on `#content` while the pinch is on. Without it every frame
  repaints the whole document on the README.

"Small documents" are `README.md` (plain, with the sidebar open, in a 700 px window, and with
the pointer beside a heading), `kitchen-sink.md` and `widths.md`: 80 runs for each candidate.

| | Z | T as written | W |
|---|---|---|---|
| Small documents: runs under 95% of frames within 1.5 F | 11 (worst 2%) | 24, all on the README (worst 2%) | none (98 to 100%) |
| Small documents: a run's median frame | 16 to 28 ms | 16.5 to 45.5 ms on the README, 17 ms elsewhere | 16 to 17 ms |
| README × 30: median, worst frame | 165 to 190, 396 ms | 30 to 53, 70 ms | 17, 42 ms |
| JSON's first chunk: median, worst frame | 284 to 301, 524 ms | 22.5 to 39, 83 ms | 17, 57 ms |
| Held point on the pinch's last frame | within 1.0 px | 0 px | 0 px |
| Held point across the "fingers" ending, down | 0 px | within 1.0 px | within 1.0 px |
| By the owner's hand (M3) | smooth; "Not smooth" on README × 30 | "stutters but slightly" (README only) | smooth, text sharp, "no blanks" |

Z fails three lines of the bar (the small documents' frames, the large documents' ceiling, the
owner's hand on the large README) and T the first of them; W passes every line.

- **W's ending**, one full re-layout, measured to the next frame: 15 to 36 ms on the small
  documents, 174 to 254 ms on README × 30, 291 to 444 ms on the JSON's first chunk.
- **Across an ending** the held point stays within 1 px down the page. Where text re-wraps and
  the column re-centres it moves 100 to 600 px sideways, the same order for T, for W and for
  Z's ladder ending.
- **M2, a pinch held open** (README, sidebar open, at 165%): the page gets no sideways scroll
  (`scrollWidth` = `clientWidth` = 1100); the clip on `main` moves nothing (0 of 40 rectangles
  changed); the sidebar is not covered; after the end the transform, its origin, `will-change`
  and the clip are all gone. A still picture of that state looks slightly soft to the Mac
  session; the owner, pinching, called W's text sharp.
- **M4, the edges:** all as designed, the same for Z, T and W. At 50% and 300% nothing is
  applied and nothing is left; a pinch that starts over the sidebar, the bar or Settings does
  nothing; one that starts on the document and drifts over the sidebar goes on; with
  `gestureend` withheld the pinch ends once, 202 to 203 ms after its last event; a zoom wheel
  event after the end is dropped; every pinch ended exactly once.
- **The 10 MB JSON with every chunk loaded** (12.3 million px tall): making the layer takes
  699 ms, the first scaled frame 545 ms, taking the layer off 584 ms, and the ending 6.9 to
  7.7 s. One step of today's zoom takes 7.9 s there. In the scripted run the stall at the
  start outlasted the 200 ms timer, which ended the pinch after one change of level.
- **Memory with W:** no rise seen. The GPU process stayed at 55 to 59 MB and the page's process
  moved between 838 and 949 MB with no trend; memory held on the GPU's side cannot be read.
- **The owner's own words**, in order: "I prefer 3/4 if you ask me" (W); "I take it back 3/4
  feels wierd after you lift, 1/2 might be better" (Z); asked what was weird, "not noticeable
  by normal people, it's just because I know about the relayout"; "yeah 3/4 better with large
  docs". Both endings: "Both fine", "All fine", "Fine".
- **The baseline**, today's steps: 94 to 100% of frames in time on the small documents, 22
  frames over 100 ms on each large one, and a pinch that ends on 200, 75 or 250% where the
  fingers asked for 272, 50 or 296%.

## The build

W, with the level kept where the fingers left it and one toast at the end (rulings 2 to 4).
The pinch changes on macOS only; the Rust change reaches every platform.

### Task 2: Rust, a level the page names

- **`config::doc_zoom`** (`config.rs:78`) keeps any whole percent from the ladder's first rung
  to its last (50 to 300); anything else is still 100. Its doc comment and the field's
  (`config.rs:124`) say so. `ZOOM_LADDER`'s own comment stays true: a step still lands only on
  a rung.
- **A new command, `set_doc_zoom(app, percent: u32)`,** beside `step_doc_zoom`
  (`main.rs:1368`) and listed with it in the handler (`main.rs:2133`), which is all a command
  needs here (*read*: no app manifest, no capability for the app's own commands). The level
  goes through `config::doc_zoom`, is saved and is broadcast as `doc-zoom-changed`. It returns
  nothing: the page has applied the level already. `step_doc_zoom` is unchanged.
- **Tests** (`config.rs:405-427`): the first, renamed and its doc comment with it, holds that
  137 stays 137, that 49, 301 and 0 become 100, and that every rung stays. The step test gains
  the off-ladder case: from 137, up is 150, down is 125 and 0 is 100.

### Task 3: the page, a pinch that follows the fingers

In `app.js`. On Windows and Linux `onDocWheel` goes on stepping as today, untouched.

**What is kept while a pinch is on:** the level at its start; the level the fingers are at,
as a fraction; the held point and the node it is in; whether a frame is asked for; the 200 ms
timer. Beside it, from the gesture events (`app.js:6324`):

- **whether the last gesture event was `gestureend`**, kept with `lastGesture`. Any
  `gesturestart` or `gesturechange` unsets it, so a lost `gesturestart` leaves nothing stuck;
- **whether the gesture began somewhere that isn't the document's**, unset by `gesturestart`
  and by `gestureend`.

**A zoom wheel event on the Mac** (after `onDocWheel`'s two tests of today, `app.js:4476`):

- With no pinch on:
  - if the last gesture event was `gestureend`, it is a pinch's trailing event, come late:
    prevented and dropped;
  - in a gesture marked as not the document's: left alone, as today;
  - else today's test of the target (`app.js:4478`). Failing it marks the gesture as not the
    document's, so a pinch begun over the sidebar does nothing when the pointer reaches the
    document. Passing it starts a pinch: a running scroll hold is aborted, the start level is
    `state.docZoom`, and the held point is taken.
- Then, for every event of a pinch: the default prevented; the level multiplied by
  `Math.exp(-deltaY * WHEEL_ZOOM_RATE)` and kept inside 50 to 300 at each event, so a turn
  back from either stop answers at once; the timer started again; a frame asked for, if none
  is.

`gestureend`, with a pinch on, asks for a frame too, if none is asked for. It leaves the timer
alone: with the window hidden no frame comes, and the timer still ends the pinch.

**The held point** is `zoomAnchor`'s (`app.js:4356`), asked to hold the point and not the
line's top: on text, the pointer's fraction down the caret's rectangle, with none of
`zoomHeld`'s bookkeeping (it is cleared); elsewhere the element fallback as it is, which
already holds a fraction. It answers how far the point is from its starting height, and the
pinch keeps the node it measures (the caret's container, or the fallback's element).

With nothing to hold at the pointer (`zoomAnchor` answers null) the pinch holds what a key
step holds, the point at the top of the view (`app.js:4417-4419`), and scales about that. This
was written for the empty space below a short document, and the walk showed it never runs
there: WebKit's caret from a point below the text is the last line's, so there is always
text to hold (*measured*). Ruling 6 covers that case. In an empty document
there is nothing to hold at the top of the view either: the pinch has no anchor, its ending
passes none, and
`showDocZoom` scales the offset as it does today. The height the page scales about is always
that of the point that was asked about, the pointer's or the top of the view's, never read
from the anchor.

**A frame** ends the pinch if the last gesture event was `gestureend`; else it paints. The
trailing wheel event comes in `gestureend`'s own millisecond, before that frame, so it is
counted. To paint: the level, rounded to a whole percent; if it differs from the one on
screen, `transform: scale(level / start)` on `#content`. The first time a pinch paints a level
other than its start, it also sets, inline: `will-change: transform` and
`transform-origin: 50% Ypx` on `#content`, Y being that point's height below `#content`'s top
over the start level (the element's own px, which held the point at 0 px in the trial); and
`overflow-x: clip` on `main`. So a pinch against a stop sets nothing and makes no layer.

**Where the page cannot scroll** (rulings 5 and 6) the transform is
`translateY(off) scale(…)`. Taken at the first paint, in window px: D, the held point's height
below the top of the text (below `#content`'s top, less the padding there, which the real
zoom keeps at its size on screen); H, the text's height (`#content`'s, less that padding);
and B, where `#content` ends in the page. With s the scale:

- the real zoom will want the page scrolled to `scrollY` + D × (s − 1);
- it will be able to scroll from 0 to B + H × (s − 1) less the window's height, or not at all
  if that is below 0: the page ends where `#content` does (*read*: nothing follows `main` in
  the flow), and everything under the top padding is taken to scale with the level;
- `off` is the first less what the second allows of it, in the element's own px as the origin
  is. It is 0 wherever the page can scroll as far as it wants.

So the text stands where the ending will leave it: at the page's top on a squeeze, where the
blank padding above the text comes down by up to its own height times (1 − s) while the
fingers move; in a document shorter than the view on a spread, which grows down from its top;
and at a document's end on a squeeze. Where blocks do not scale with the level (a picture
that isn't loaded, a block the pane limits), the estimate is off, and the text moves when the
pinch ends by the unevenness of what lies between the held point and the end that stops the
scroll. For a spread that stretch is about a view tall at most. For a squeeze it is up to the
point's distance from that edge of the view over s: two views for a squeeze to half, six for
one from 300% to 50%. Further from an end the page can scroll (*reasoned*). At the top, with
the pointer mid-view, the walk measured 7 to 35 px.

Against the pinch as first walked (`4008e3c`): the top is as ruling 5 left it. At the end the
point used to move by all that the real zoom could not scroll. It is now worse only where the
estimate stops a scroll at the end that the real zoom would have made, and then by no more
than that same unevenness (*reasoned*).

**The pinch ends**, from that frame or when the timer runs out, once:

1. The frame and the timer are cancelled and the pinch's state is cleared.
2. The four inline styles are taken off.
3. `showDocZoom(level, held)` applies the real level, the level being the fingers' own,
   rounded, not the last one painted. `showDocZoom` (`app.js:4403`) gains that second
   argument: an anchor to use in place of the pointer's or the top of the view's. The ending
   leaves it out when the held node is no longer in the document (`isConnected`): a node that
   is gone measures as zeros. Everything a step does there happens once: the offsets of the
   other tabs, `rememberScroll`, `scheduleReport`, the level in Settings. It does nothing when
   the level is the start's.
4. If the level changed, `invoke("set_doc_zoom", { percent })`: one save and one broadcast. The
   broadcast comes back to this window as a level it already has.
5. `toast("Zoom N%")` if the level changed, or stands at 50 or 300: a pinch against a stop says
   so, as a key at a stop does today. A brush of the trackpad that changes nothing elsewhere
   shows nothing, as today.

The level is applied here before Rust answers, unlike a step: waiting would show the start
level for the frames in between.

**A timer's end is not the gesture's.** If the fingers rest for 200 ms and then move again,
with no new `gesturestart`, the next event starts a new pinch from the level and the pointer
as they are now. Only an event after `gestureend` is dropped. The trial dropped every event
after an end, which would leave the page dead until the fingers lift (*reasoned; the walk
tries it by hand*).

**A level from elsewhere while a pinch is on** (another window, a key, Settings: all arrive as
the broadcast): `showDocZoom` drops the pinch, which is the ending's steps 1 and 2 and no
more, then applies the level as usual. If the fingers go on, the next event starts a new pinch
from that level. The drop sits after `showDocZoom`'s early return (`app.js:4405`), so only a
level that differs from this window's drops a pinch. Before it, the window's own broadcast
would: a pinch that ends on the timer saves, the fingers move on into a new pinch, and the
first one's broadcast comes back in the middle of it.

**A live reload or a switch of tabs while a pinch is on:** nothing is hooked. The transform is
on `#content`, which stays, so the new page is shown scaled until the pinch ends in its own
time. The held node is gone by then, so the ending holds the top of the view, as a key step
does. While `#content` is hidden (a picture's tab) the pinch's events are not counted and the
timer ends it.

**What goes:** `DOC_WHEEL_STEP`'s Mac value (`app.js:4464`) and the words about a pinch in the
comments there. `zoomPoint` is then set on Windows and Linux only.

### Task 4: the README

- Line 269-270: `Ctrl`+wheel zooms a step at a time; on macOS a pinch follows the fingers.
- Lines 459-463: the keys and the wheel move in steps; a pinch on macOS is smooth and can stop
  on any level between 50% and 300%, and the keys step on from there.
- Lines 56-58 already hold: they name no steps.

### Task 5: the walk

**Gates:** what `checks.yml` runs: `cargo fmt --check`, `cargo clippy --workspace --all-targets
--locked -- -D warnings`, `cargo test`, and `node --check src/app.js`, the one check of the
page's script.

**On the Mac**, on a pushed `walk/smooth-pinch` branch (the owner's go), a `webdriver` debug
build, with the trial's driver sending its events to the app's own listeners. The build has no
switch that holds a pinch open, and a screenshot, a second window or a save takes longer than
the 200 ms timer. So the walk holds one open from the page: an interval that sends, every
50 ms, a `gesturechange` and a zoom wheel event of 0 px at the starting point, until cleared.
While it runs, an event starts a new pinch as soon as one is dropped, so a check for "nothing
stuck" comes after the interval is cleared and the 200 ms timer has passed. A second window
stands beside the first, not over it: a covered window's frames and timers may slow
(*reasoned*).

- **W1, the bar again, on the real code:** the trial's M1 and M2 runs for the pinch as built,
  on all five documents and the README's four set-ups, judged by the same bar. And, for
  ruling 5, on `README.md` at its top (scroll 0), the pointer mid-view, a squeeze from 100%
  and one from 150%: the top of the first block of text stays where it was, within 2 px,
  while the pinch is held open, and it and the text under the pointer move no more than 2 px
  across the ending. The squeeze from 150% is the one that shows a wrong unit: about 40 px
  off. The same squeeze 2000 px down the page holds the point as before: `over` is 0 there.
- **W2, one save per pinch:** `doc-zoom-changed` arrives once per pinch that changed the level
  and not at all otherwise; `config.json` holds the level; a second window keeps its level
  while the pinch is held open and takes the new one at its end.
- **W3, the edges:** M4's list on the real code, three of its checks made to bite there:
  - *the trailing event dropped:* a pinch left to end on the timer, then `gestureend` and a
    zoom wheel event of −10 px in one task. The wheel event, sent `cancelable` as every wheel
    event of the walk is, is prevented (`dispatchEvent` answers false), and the level, the
    toasts and the broadcasts stay as they were. A wheel
    event sent later than 100 ms after the last gesture event is no zoom wheel, and would
    pass with no rule at all;
  - *the trailing event counted:* with a pinch still on, `gestureend` and a wheel event of
    −10 px in one task. The level the pinch ends on has that event in it;
  - *ended exactly once:* one `doc-zoom-changed` and one write to the toast (counted by a
    `MutationObserver` on it; it stays up 4 s) for each pinch that changed the level.

  And:
  - a lost `gesturestart`: once a pinch has ended (its broadcast has come), a second one sent
    with no `gesturestart` (a `gesturechange` and a wheel event by turns, then `gestureend`)
    ends with a broadcast of its own, at another level. Sent before the first one's ending,
    its events would count in the first and show nothing;
  - a spread far past 300% then a squeeze answers at once;
  - events that go on after a timer's end, with no `gesturestart`, zoom on as a new pinch;
  - a pinch on the document straight after one begun over the sidebar zooms;
  - a pinch on the empty space below a short document ends where it stood, with no jump;
  - with a pinch held open: a key step, a step from a second window, a save of the file and a
    switch of tabs each leave the level they should and nothing stuck (no transform, origin,
    `will-change` or clip); and `showDocZoom(state.docZoom)`, called from the driver with the
    pinch held mid-spread, leaves `#content`'s transform at the same scale (a dropped pinch
    loses it, and the 0 px events of the new one never paint it back);
  - a pinch in a picture's tab and in the full-window diagram view still zooms those, and
    leaves the document's level alone.
- **W4, a level off the ladder:** after a pinch to about 137%, Settings shows it; `Cmd++` goes
  to 150%, `Cmd+-` from 137% to 125%, `Cmd+0` to 100%; the level is back after a restart; a
  find's highlights stay on their text, by a screenshot read by eye; one toast per pinch that
  changed the level, and none for one that changed nothing away from a stop. And a second
  tab's place: after a pinch from 100% to about 150%, how far off it comes back, beside the
  same after the keys' three steps from 100% to 150% (both in px).
- **W5, by the owner's hand:** `README.md`, `widths.md` and the README thirty times over: a
  spread, a squeeze, a squeeze at the very top of the page, and a spread with a rest in the
  middle. And a pinch begun over the sidebar and carried onto the document: nothing zooms and
  nothing scrolls (its wheel events are no longer prevented there, and a script's events
  have no default to show it). The owner says whether it is
  smooth and how the end looks.

**On Windows and Linux**, for the Rust change: with `"doc_zoom": 137` written into the scratch
`config.json`, the app opens at 137% and Settings shows it; the keys go to 150% and, from 137%,
to 125%; `Ctrl`+wheel still moves a step at a time with a toast for each.

### Who builds it, and the commits

Tasks 2 to 4 go to the `coder` agent in one pass, committed locally on `main` as one commit
whose summary is the release-note line: *On macOS, a pinch zooms a document smoothly, following
your fingers*. Then the change review loop, the walk, the triage and the packaging, each push
on the owner's go. At the triage, `open-items.md`'s deferred item for the pinch is closed, and
the accepted limit *a burst of zoom steps on empty space creeps a few px* is looked at again:
on the Mac no wheel steps are left to creep.

## Limits known now

- macOS only. A touchpad pinch on Windows and Linux reaches the page as `Ctrl`+wheel with
  nothing to tell it from a wheel (`open-items.md`, *Document zoom is unwalked with some real
  input devices*), so it keeps the steps.
- The page follows the fingers up and down only: the column stays centred and the page never
  scrolls sideways, as with the keys.
- The feel is the owner's hand to judge; a script measures frames, not smoothness.
- The trial runs on the owner's Mac, on its display and trackpad. Other Macs are not measured.
- With T, a squeeze may show areas of the page that were never painted, for a moment
  (*reasoned*); M3 looks for it. The owner saw none ("no blanks", on the large README).

Found by the trial, for the triage. Since ruling 7 the first two, and the one on where the
page cannot scroll, hold only for a pinch that is scaled: a large document, or one that went
over (*The mix*, *What follows from it*):

- When the fingers lift, the page is laid out once. Where text re-wraps, the point under the
  pointer moves sideways (100 to 600 px measured), and on a large document the page stands
  still for 0.2 to 0.4 s.
- While the fingers move, what the pane limits keeps its old wrapping and is cut at the pane's
  sides, and a squeeze shows empty space around the page.
- On a JSON file of 10 MB with every chunk loaded, a pinch stalls for over a second at its
  start and about 7 s at its end, as one zoom step does today. A real pinch may break into
  several there, each with its own save, toast and ending, where today's makes three to six
  steps of 7.9 s (*reasoned*).
- The page renders at 60 Hz on a 120 Hz display, so a pinch moves at 60 frames a second at
  most; a live pinch at fewer (*The mix*).
- Memory on the GPU's side for the layer was not measurable.
- Where the page cannot scroll to hold the point (its top on a squeeze, its end, a document
  shorter than the view), the point under the fingers is not held, as with a step today. The
  page is kept where the pinch's end will leave it (rulings 5 and 6), by an estimate that
  takes the text to scale evenly. Where it doesn't, the text under the fingers moves when
  they lift: 7 to 35 px at the top of the three small documents, the pointer mid-view
  (*measured*, the walk); more for a hard squeeze from a high level a little way down a page
  with blocks that do not scale (*reasoned*).
- The walk runs on a debug build with the driver in it. The release build's pinch is first
  seen in the release itself, or in a dry run's artifact.

## Chosen without asking

- **T scales about the middle of the column, not about the pointer** (review pass 1, finding 4,
  which the reviewer marked for the owner). About the pointer, the text would jump sideways
  when every pinch ends, by the scale's change times the pointer's distance from the middle,
  since the real zoom always centres the column. The owner sees the result in M3 and can
  reverse it there.
- **The bar** (finding 13's other half): in frames of the display's own rate, with a ceiling
  for the large documents, written down before any number is in.

In the build:

- **Fingers that rest and move again go on zooming**, as a new pinch; the trial's rule would
  have dropped them until the fingers lift.
- **The level is kept inside 50 to 300 at every event**, not only when it is painted, so a
  turn back from a stop answers at once.
- **A level from elsewhere drops the pinch**, and the level that arrived stands. The other way
  round, the pinch going on and overruling it at its end, needs the late level kept for the
  case where the pinch changes nothing.
- **A live reload or a switch of tabs is not hooked**: the pinch ends in its own time and
  holds the top of the view.
- **A toast for a pinch that changed the level, or that stands at a stop**; none for one that
  changed nothing elsewhere.
- **With nothing under the pointer to hold, the pinch scales about the top of the view**,
  which is what its ending holds.

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

Pass 4, of the build (a fresh reviewer, read-only), thirteen findings, none a decision for the
owner; all taken:

1. The drop of a pinch sits after `showDocZoom`'s early return, or the window's own broadcast
   drops the next pinch.
2. A drop cancels the frame and the timer too: it is the ending's steps 1 and 2.
3. The walk holds a pinch open with an interval of events; the build has no switch for it.
4. On the fully loaded 10 MB JSON a pinch breaks into several; it does not end early.
5. Z fails three lines of the bar, not two.
6. "The last gesture event was `gestureend`" stands in for a flag only `gesturestart` unsets,
   so a lost `gesturestart` cannot leave a pinch dead.
7. No toast for a pinch that changed nothing away from a stop.
8. The gates name `node --check src/app.js` and clippy's flags.
9. The test's doc comment changes with the test.
10. The pinch keeps the held node, and its ending tests that it is still in the document.
11. With nothing under the pointer to hold, the pinch scales about the top of the view.
12. The walk says what is read by eye and by what measure, and gains four checks.
13. `set_doc_zoom` goes through `config::doc_zoom` and returns nothing; the ending is the
    frame's own job once `gestureend` has come, with no second callback.

Pass 5 (the same reviewer), four findings, all taken: the height the page scales about comes
from the point asked about, so an empty document has one too; the layer and the clip go on
with the first level that differs from the start, not on the first frame; the walk clears its
hold-open interval and waits out the timer before it looks for anything stuck, and keeps a
second window uncovered; and the second tab's place is compared after a change the keys can
make.

Pass 6 (the same reviewer), one finding, taken: the check that a same-level `showDocZoom`
leaves a pinch running reads the transform's scale, since the hold-open interval would start a
new pinch after a drop and hide it.

Pass 7 (the same reviewer), three findings, all in the walk, all taken: the trailing wheel
event is sent in `gestureend`'s own task, dropped after a timer's end and counted with a pinch
on; a pinch with no `gesturestart` is walked; and "ended exactly once" is counted by the
broadcast and the toast.

Pass 8 (the same reviewer), two findings, both in the walk, both taken: the walk's wheel
events are cancelable, or "prevented" cannot be read; and the pinch with no `gesturestart` is
sent after the one before it has ended, and judged by its own broadcast.

Pass 9 (the same reviewer): no findings.

## Change review

Of `5ab44b7` (the build, by the `coder` agent; the gates passed: `cargo fmt --check`, clippy
with `-D warnings`, 218 tests, `node --check src/app.js`) and the two fixes on top of it, by a
fresh reviewer, read-only. Nothing was run in the app.

Pass 1, two findings:

1. A squeeze near a page's top would jump up when the fingers lift. To the owner at once:
   ruling 5, fixed in `745c4e0`.
2. `showDocZoom`'s doc comment did not name a pinch's end. Fixed with it.

Sixteen cases traced through the code, all as planned; Windows and Linux unchanged.

Pass 2, two findings, both taken in `af24ab1`: the fix counted the padding above the text,
which the real zoom does not scale, so the text would still have dropped 13 to 33 px at the
end; and the plan's words and its W1 check followed the code as it was.

Pass 3: no findings.

For the triage, from the reviewer's notes (all *reasoned*):

- If WebKit sends `gestureend` and the pinch's last wheel event in separate tasks, a frame
  already asked for could end the pinch between them, and that last event (0.1 to 16 px)
  would be dropped. The trial's rule was the same, and the owner called its endings fine.
- A pinch begun over the sidebar and carried onto the document has its wheel events left
  alone there, where they zoomed before. W5 looks for a scroll by hand.
- A live reload during a pinch is not seen when the caret under the pointer sat between two
  blocks: the held node is then `#content` itself, which stays. The ending would measure a
  caret that has moved.
- A reader's own theme that changes the padding above the text misleads ruling 5's fix, as it
  already misleads the padding a zoom level keeps.

## Walk results (2026-11-16 and 17, `walk/smooth-pinch` at `4008e3c`)

**Windows** (the VM, a debug build, real keys and wheel over CDP; beside a build of 1.9.2):
the gates pass (218 tests). `"doc_zoom": 137` opens at 137% and Settings shows it, where
1.9.2 opens at 100%; the keys go to 150%, 125% and 100%; 49, 301 and 0 open at 100%;
`Ctrl`+wheel moves a step with a toast, the line under the pointer within 0.5 px, and does
nothing over the sidebar; a key step moves the top line 0.4 px; two windows stay in step.
Nothing differs from 1.9.2. The owner's own `config.json` was used, backed up and restored
byte for byte: Windows has no way to point the app at another.

**Linux** (the VM, a debug build, real keys and wheel, a scratch config): the same seven
steps pass (223 tests); the line under the pointer within 0.63 px, the top line within 1 px.
Not run beside 1.9.2.

**macOS** (the owner's Mac, a `webdriver` debug build, the trial's driver at the app's own
listeners). F = 16.66 ms again.

- **W1, the bar: met.** Small documents: 72 runs, each with every frame within 1.5 F. README
  × 30: worst frame 38 ms; the JSON's first chunk: 51 ms. The held point: 0 px on the last
  frame, within 1.0 px across the ending. The ending to the next frame: 15 to 46 ms, 167 to
  242 ms and 293 to 450 ms. Held open: no sideways scroll, nothing outside moved, the sidebar
  not covered, the four styles gone after.
- **W1, ruling 5** (README at its top, a squeeze from 100% and from 150%): the top of the
  first block 0 px off, held open and across the ending; no wrong unit
  (`translateY(-102.68px) scale(0.4933)` from 150%). **But the text under the pointer moved
  34.9 and 27.4 px across the ending** (`kitchen-sink.md` 7.3 and −1.4, `widths.md` 26.4 and
  −3.0), against the 2 px this plan asked. The README's row of badges keeps its 58 px at
  every level while remote pictures are blocked, and blocks the pane limits do not scale
  with the level, so the height above the point is not the scale's; at the top no scroll
  makes it up. The 2 px was this plan's mistake: it took the text to scale evenly. A limit
  for the triage. 2000 px down: no shift, 0.8 and 0.9 px.
- **W2: passes.** One broadcast and one toast for each pinch that changed the level; none for
  a brush, an out-and-back, or a pinch against a stop (which shows its toast); `config.json`
  holds the level; a second window kept 100% while the first was held at 165%, then took it.
- **W3: passes, but for one fault.** The trailing event dropped and counted; a lost
  `gesturestart`; a turn back from 300%; events after a timer's end; the sidebar, the bar and
  Settings; a drift over the sidebar; `gestureend` withheld; the stops; a key, a second
  window's step, a save and a switch of tabs with a pinch held open; `showDocZoom` at the same
  level; a picture's tab and the diagram view. **The fault:** on a short document, the pointer
  350 px below the text, a spread to 182% took the text 457.6 px up while the fingers moved
  and back when they lifted. The caret from a point below the text is the last line's, so the
  hold of the top of the view never ran. Ruling 6.
- **W4: passes.** 137% in Settings; the keys to 150%, 125% and 100%; 137% again after a
  restart; a find's highlights on their words at 137% and after a pinch to 160%; a second
  tab's place after 100% to 150% is 966 px off by a pinch and by the keys alike (the README's
  blocks that do not scale; an accepted limit of the document zoom's, larger here than its
  words suggest).
- **W5, the owner's hand** (answers picked from the Mac session's wording): `README.md`,
  `widths.md` and README × 30, "Smooth, end fine"; a pinch begun over the sidebar and carried
  onto the document, "Nothing moved" (every wheel event kept the sidebar as its target).

**To walk again, for ruling 6** (macOS, `3d0541d`):

- a document of a few lines, its text under half the view tall at 180%, so that it stays
  shorter than the view: a spread from 100% to about 180%, the pointer on its text and then
  350 px below it. The top of the first block within 2 px while held open and across the
  ending. And a spread far enough for the document to outgrow the view, where the top
  rightly moves: the move across the ending reported;
- `README.md` at its very end, a squeeze from 100% and from 150%: how far the text under the
  pointer and the last block's bottom move across the ending, beside the same on `4008e3c`;
- `README.md` at 300%, scrolled a view or two down, a squeeze to 50% with the pointer
  mid-view: how far the text under the pointer moves across the ending (the remainder at its
  largest, reported);
- ruling 5's checks at the top again, and the squeeze 2000 px down: unchanged;
- W1's bar on `README.md`, one set of runs: unchanged;
- by the owner's hand: a spread and a squeeze on a short document, and a squeeze at a
  document's end.

## Change review, after the walk

Of ruling 6's fix (`3d0541d`), by the same reviewer, read-only.

Pass 4, three findings, none in the code's logic, all taken (`f103347` and this plan): the
remainder where blocks do not scale evenly was put as a view's worth at most, which holds for
a spread and not for a squeeze; a comment said "measured" of a case only reasoned; and the
walk's short document had to stay shorter than the view for its 2 px to mean anything. The
geometry was worked through eight cases with numbers, and the page's end read off the markup
and the stylesheet.

Pass 5: no findings.

## Re-walk of ruling 6 (2026-11-17, macOS, `walk/smooth-pinch` at `eb3ce5e`)

The same driver and build as the walk; `4008e3c` built beside it for the comparisons. The
gates pass (218 tests). View 1100 × 828 px.

- **A short document: passes.** A spread from 100% to 182%: the top of the first block 0 px
  off while held open and 0 px across the ending, with the pointer on the text and with it
  350 px below (457.6 px each way on `4008e3c`); the last block's bottom 1.6 px across. A
  document that outgrows the view at 300%: its top moves up 126 px while held, rightly, then
  1.0 px across the ending with the pointer on the text, 7.6 px with it below.
- **The README's very end, a squeeze: passes.** Across the ending the last block's bottom
  moves 0.5 px (from 100%) and 0.1 px (from 150%), where `4008e3c` moved 207.6 and 210.1; the
  text under the pointer −8.6 and 36.0 px, where it moved 198.6 and 246.0.
- **The remainder at its largest** (the README at 300%, a squeeze to 50%, the pointer
  mid-view; reported, not judged): 12.7 px on text two views down; 71.0 and 46.7 px one and
  one and a half views down, where the pointer was on the large screenshot, a block the pane
  limits.
- **The top again: unchanged.** The first block's top 0 px held open and across, on both
  builds, with the same transform at the same pointer. The text under the pointer moved 24.4
  and −0.9 px here, 31.1 and 8.2 px on `4008e3c` the same day, 34.9 and 27.4 px in the walk:
  the blocked badge picture stood 58, 37 and 20 px tall on three launches, so the pointer met
  another line each time. 2000 px down: 0.9 and 0.6 px, and `translateY(0px)`.
- **The bar on the README, twelve runs: unchanged.** Every frame within 1.5 F; the held point
  0 px on the last frame, within 0.9 px across the ending; every transform
  `translateY(0px) scale(…)`.
- **By the owner's hand** (answers picked from the Mac session's wording): the short
  document, a spread and a squeeze, the pointer on the text and below it, "Smooth, end fine".
  The README at its end, a squeeze: "Something jumped", with no more said. That pinch went
  from 100% to 50% with `translateY(188.1875px) scale(0.5)` as the fingers lifted and left
  nothing stuck; what moved, and how far, was not recorded.

With the pointer below the text the driver's "held point" is the last line's rectangle
drawn out 350 px, so its own figures there (−29.1 and −38.7 px) measure nothing; the top and
the bottom are the measure.

## What did not match (2026-11-17, macOS, `eb3ce5e`, by script)

The owner's "Something jumped", in their words: "the jump was more like the zoom did not match
after relayout". Measured on `README.md` with a pinch held open and then let go: one paragraph
near the pointer, under the transform against after the ending.

| Window | Pinch | Width on screen | Ratio | Lines | What differs |
|---|---|---|---|---|---|
| 1100 px | 100% to 50%, at the end | 317.6 to 544.5 px | 1.714 | 9 to 6 | column and font |
| 1100 px | 100% to 50%, mid-document | 317.6 to 544.5 px | 1.714 | 3 to 3 | column and font |
| 1100 px | 100% to 80% | 508.2 to 677.6 px | 1.333 | 3 to 3 | column |
| 1100 px | 100% to 94% | 597.2 to 597.2 px | 1.000 | 3 to 3 | nothing |
| 1100 px | 100% to 152% | 965.6 to 965.6 px | 1.000 | 3 to 3 | nothing |
| 1100 px | 100% to 201% | 1276.9 to 1020 px | 0.799 | 3 to 4 | the pane's limit |
| 1100 px | 300% to 149% | 506.6 to 946.5 px | 1.868 | 15 to 9 | the pane's limit |
| 700 px | 100% to 50% | 310 to 408.4 px | 1.317 | 3 to 3 | font and the pane's limit |
| 1512 px | 100% to 50% | 423.5 to 544.5 px | 1.286 | 3 to 3 | font |

Three causes, each a way the real layout answers a level that a scaled picture cannot:

- **The column's two measures.** `--measure` (75ch) gives way to `--measure-wide` (100ch) once
  the pane, in the document's own units, passes twice the measure (`base.css`, `.markdown-body`).
  In the 1100 px window that is at 86% and below; in the 1512 px one at 119% and below, so
  there a spread from 100% past 120% narrows the column by a quarter (*reasoned from the
  layout at each level; that spread was not pinched*). The 700 px window never has it.
- **A smallest font.** WebKit draws no text under 9 px on screen. Body text (14 px) stops
  shrinking at 64%, code (12.25 px) at 73%: at 50% the letters are 1.29 times the size a
  scaled picture shows. This is the document zoom's own on macOS, with the keys too; where
  WebKit sets it was not looked into.
- **The pane's limit.** From about 160% in the 1100 px window the column is as wide as the
  pane, and so are tables and pictures; a scaled picture runs off both sides.

So the scaled page is true only inside a band of levels, and the band depends on the window.
The trial did not measure this: its ending was judged by the held point's height and by the
owner's eye, who twice said the end felt wrong ("feels wierd after you lift", and now).

## The mix (ruling 7)

A pinch applies the real zoom on every frame while the document keeps up, so the page is
always what the level lays out and nothing changes when the fingers lift; where a frame of
that is too slow, it scales the page as built. All in `src/app.js`, on top of the pinch as
built. The Rust and the README stand.

### Task 6: the page

- **Whether a pinch starts live.** At its start: `#content` holds no more than
  `PINCH_LIVE_ELEMENTS` elements (`getElementsByTagName("*").length`). The count is the
  document's own: its height, planned first, changes with the level and the window, since
  the pane's limit and the smallest font re-wrap the text, so one document would have started
  live from 100% and scaled from 250%.
- **A live frame** calls `showDocZoom(level, held, true)`: the real level and all a step does
  with it (the level in Settings, the other tabs' offsets, `rememberScroll`, and
  `scheduleReport`, which only restarts a timer). The third argument, new, says the pinch is
  asking, so `showDocZoom` does not drop it. So `state.docZoom` follows the fingers. Nothing
  is saved or broadcast before the end.
- **The held point, gone.** `held` is passed while the page is the one the pinch began on and
  its node is still in the document, on a live frame and at the end. The first is new: the
  pinch keeps `shownToken` from its start, which every render changes. The test of the node
  alone, as built, missed a reload when the caret sat between two blocks: the held node is
  then `#content` itself, which stays, and a live pinch would scroll by a wrong amount on
  every frame. The test of the node stays for what changes the page with no render: a
  diagram redrawn on a change of look, a JSON chunk's button gone once it is loaded. With
  `held` gone `showDocZoom` holds the top of the view, as for a key.
- **Keeping up.** A live frame that changes the level times its call of `showDocZoom`, in
  `onPinchFrame`, around the call. What it times is the style and the layout, which the
  anchor's read forces; painting comes after and is not in it. Over `PINCH_LIVE_MS` on two
  timed frames in a row, or over 100 ms once, the pinch goes over to the scaled page for the
  rest of its time, from the level the page is then laid out at. A pinch's first timed frame
  is left out of the two in a row: a first call runs long on every document under the cap
  (10 to 43 ms *measured*), as the trial's bar left a pinch's first frame out. The once is
  for a frame so long that the 200 ms timer would end the pinch before a second could be
  counted (*measured in the trial, on the fully loaded JSON*), and counts on the first frame
  too. Nothing is remembered: the next pinch on that document starts live again and goes
  over again.
- **The scaled page** is as built, but for one thing: it is measured from `state.docZoom`, the
  level the page is laid out at, where it used the pinch's start (the scale, and the own px
  of the origin and of the shift). The two were the same until now.
- **The end** is as built: the styles off, then `showDocZoom(percent, held)`, which has nothing
  to do when the last live frame was at that level; one save if the level is not the one the
  pinch began at; one toast by the same rule as before.
- **A level from elsewhere** drops the pinch and stands, unless it is the level the pinch began
  at: `showDocZoom`'s early return, for a call that is not the pinch's own while a pinch is
  on, compares with the pinch's start and no longer with `state.docZoom`, which now moves.
  As built the two were one, and this keeps what the build did: the window's own save, come
  back in the middle of the next pinch, drops nothing. A key in the middle of a pinch steps
  from the saved level, the one the pinch began at, as built: `Cmd++` during a pinch from
  100% that stands at 160% lands on 110%.

The two numbers are placed from a measurement made before the build (*The cost of a live
frame*, below): the call's time and the frame's length, by document size, on the Mac.

The document's size decides, on this Mac; the time is the net under it, for a slower Mac and
for a page that costs more than its size says. A rule led by the time would sit where calls
scatter around the limit, and the same document would end one pinch true and the next
re-arranged.

- `PINCH_LIVE_ELEMENTS = 3400`, about four READMEs (845 elements each): **a first value, the
  owner's to set by hand in L6.** It is where being true gives way to being smooth. Up to
  twice the README's length a live frame is 18 to 30 ms; at four times, 29 to 41 ms, some 25
  to 35 frames a second; at eight times, 51 to 63 ms. Over the cap, the pinch starts scaled,
  always. The call costs about 2 ms and 6 ms more for each 1 000 elements on the README's
  kind of page, and 4.4 ms for each 1 000 on the JSON (*measured*). A page of long
  paragraphs and few elements costs more than its count says, and the net is for that. A
  page heavy with diagrams counts more than it costs, every node of a drawing being an
  element: it starts scaled sooner than it need (*reasoned*).
- `PINCH_LIVE_MS = 33`: a call that long is a frame of about 43 ms, since a frame is the call
  plus about 10 ms (*measured*). By the figures no document under the cap reaches it but the
  README four times over on the ramp from 150% (a median call of 33 ms), which would go over
  within a few frames (*reasoned from the medians and the 90th percentiles, not replayed
  over the frames*). A document that does go over pays its first frame and two more slow
  ones at the start of every pinch, nothing being remembered: some 140 ms there.
- **The net bounds the cap.** By the fit, calls pass 33 ms at some 5 000 elements, about six
  READMEs; measured, four READMEs are under it and eight over. A cap above that keeps
  nothing more live: it only adds that stall to each pinch. The owner's choice in L6 lies
  between about two and six READMEs; keeping more live means moving the net too, which is
  then the owner's as well.

For L6 the cap is declared with `let`, so the driver can set it between runs and the owner
can feel one document both ways. The `fixup!` that follows L6 writes the owner's value and
makes it a `const`. A change of the cap needs only L3's last check run again at the new
value (a document just under and one just over), and L2 if the new value passes a large
document's count (25 350 for the README thirty times over).

The two ways that measurement could have come out that this design would not survive did not
happen: the README is far under both numbers in every set-up measured (a call of 6 to 16 ms),
and the call's time follows the frame's length. Both numbers are for this Mac.

**Comments the mix makes untrue**, to be put right with it: `onPinchWheel`'s ("the page is
scaled, not laid out, until the pinch ends"), the `pinch` object's (`start`, `shown`, `held`),
`showDocZoom`'s (it now also runs on every live frame) and `endPinch`'s (a held node gone).

**Who builds it, and the commit:** the `coder` agent, as one local `fixup!` of `5ab44b7`, to be
folded into it at the packaging with the others.

What follows from it:

- On a live pinch the text re-wraps as the fingers move, the column switches and the pane
  limits as they come, and the page's ends behave as for a step: where the page cannot
  scroll, the point under the fingers slides, and nothing jumps.
- **Documents under the cap may lose frames.** The pinch as built met the bar on the small
  documents, 72 runs of 72 with every frame in time. Live, by *The cost of a live frame*,
  they hold one frame too (17 ms, 98 to 100% within 25 ms), but for the README scrolled
  2000 px down in the 1100 px window (frames of 23 to 24 ms, 76 to 99% within 25 ms); and
  documents two to four READMEs long run at 18 to 41 ms a frame. The trial's Z read worse
  (11 runs of 80 under the bar, the worst with the sidebar open) through its own handler;
  the walk times the build. That bar ruled Z out; ruling 7 puts the page being true above it,
  up to the cap, which L6 puts under the owner's hand.
- Rulings 5 and 6 serve the scaled page only.
- On a document that goes over, or starts scaled, the page still re-arranges when the fingers
  lift, outside the band. A limit, for the triage.
- If the app quits in the middle of a pinch, the saved level is the one it began at.

Chosen without asking: a live frame is `showDocZoom` whole, not a cut-down copy of it, so the
window's level is always one thing; two slow frames in a row, not one, send a pinch over, and
one over 100 ms; nothing is remembered from pinch to pinch (a memory by tab entry was planned
and dropped in review: one unlucky pinch would have left a tab scaled for good); the
document's size deciding, by its count of elements, and the time as a net under it. Not
chosen: the cap. Its 3 400 elements is a first value for the walk to be built on, and the
owner sets it in L6, within what the net's 33 ms leaves, or moves both.

### Task 7: the walk (macOS)

The gates, then, with the driver as before:

- **L1, live, on the small documents** (the README plain, with the sidebar, in the 700 px
  window and beside a heading; `kitchen-sink.md`; `widths.md`), M1's runs:
  - the pinch is live: held open, `#content`'s `--doc-zoom` and `state.docZoom` are the
    fingers' level, rounded, with no transform, while `config.json` and a second window are
    still at the level the pinch began at;
  - frames: the share within 1.5 F, the median and the worst, beside *The cost of a live
    frame* (reported; the owner's hand judges);
  - the held point within 2 px on the last frame and across the ending, away from the page's
    ends;
  - nothing changes when the pinch ends: a paragraph's width, lines and left edge are the same
    on the last frame and after the ending, for the cases of *What did not match* (100% to
    50%, to 80% and to 201%; 300% to 149%), within 1 px.
- **L2, the large documents** (the README thirty times over, the JSON's first chunk): the
  pinch starts scaled (`state.docZoom` does not move before the end) and meets W1's bar as it
  did.
- **L3, going over.** The app's `showDocZoom` is a global the driver can wrap, so a slow frame
  is made by a busy wait added before it, only on the pinch's own calls (the third argument
  true), which sit inside the span the frame times:
  - one timed frame over `PINCH_LIVE_MS`, the rest fast: the pinch stays live;
  - the pinch's first timed frame and the next both over it: it stays live; one more: it goes
    over;
  - two in a row later in a pinch: it goes over on the second; one over 100 ms, the first
    frame too: at once;
  - on the frame it goes over and the frames after: the held point within 2 px, and the
    transform's scale the fingers' level over the level the page is laid out at
    (`state.docZoom`, no longer the pinch's start);
  - the next pinch on that document starts live;
  - a document just under `PINCH_LIVE_ELEMENTS` and one just over (the README so many times
    over): the first starts live, the second scaled, from 100% and from 250% alike.
- **L4, one save:** one `doc-zoom-changed` and one toast for each pinch that changed the
  level, live or scaled, and after a pinch that went over.
- **L5, the edges, live:** the stops; out and back to the start level (no save, no toast, the
  level the start's); a rest and more movement; the trailing event dropped and counted; a
  lost `gesturestart`; a picture's tab and the diagram view. With a live pinch held open
  away from its start level:
  - `showDocZoom` called with the level the pinch began at leaves the pinch on and the page
    at the fingers' level (the window's own save coming back);
  - a key step lands one rung from the level the pinch began at, and a second window's step
    on its level; each stands;
  - a save of the file: the line at the top of the view is the same line, within two lines'
    height, across the reload and on the frames after it, also with the pointer between two
    blocks. The driver reads `pinch.held.node` there: if it is never `#content` itself, the
    case the token is for was not reached, and the run says so;
  - a switch of tabs: the pinch ends at its level, and the tab left comes back as near its
    place as after the same change by the keys;
  - the page that arrives, on a save and on a switch in the middle of a pinch that is moving:
    how far it lands from its place (reported; *Change review of the mix*, finding 1). The
    hold-open interval's 0 px events change no level and cannot make the miss, so here a
    steady spread is sent, an event on every frame, across the save or the switch, from
    about 2000 px and about 8000 px down, a few runs each. The measure, in px, once the
    restore has settled: on a save, the line at the top of the view just before the swap
    against the same line after; on a switch, the line that stood at the top of the arriving
    tab when it was last shown (read before leaving it), and how far it now is from the top
    of the view. Each run reads `state.docZoom` at the swap and at the restore's frame:
    if no live frame changed it between them, the case was not reached, and the run says so.
- **L6, by the owner's hand:** the README, sidebar open and closed, and in a full-width
  window: a squeeze to 50% and a spread to 250%. Is it smooth, scrolled 2000 px down in the
  1100 px window above all, where it lost the most frames? Does anything change when the
  fingers lift? Then **the cap**: the README two, three, four and six times over, each
  pinched live and then scaled (the driver sets the cap between runs, and logs for every
  pinch which it was, and whether a live one went over). Up to which is live smooth enough
  to keep? Then eight and thirty times over, scaled. The owner is told the range the net
  leaves, two to about six READMEs, and their answer sets `PINCH_LIVE_ELEMENTS`.

Windows and Linux are not walked again: the pinch's code does not run there, and every new
line in `showDocZoom` is behind the new argument or a pinch being on, which they never have
(*read*; the change review checks it).

## The cost of a live frame (2026-11-17, macOS, `eb3ce5e`, by script)

`showDocZoom(level)` called once per frame over three ramps of whole percents (100 to 272,
100 to 50, 150 to 296), nothing saved. Per frame: the call's own time, and the frame's length
(one `requestAnimationFrame` timestamp to the next, painting included). F = 16.66 ms. Medians
over a ramp, the range over the three ramps; the height is `#content`'s at 100%, and the
elements are counted as the build will count them, `#content`'s
`getElementsByTagName("*").length`.

| Document | Height | Elements | The call | The frame | Frames within 25 ms |
|---|---|---|---|---|---|
| `kitchen-sink.md` | 4 100 px | 430 | 4 to 6 ms | 17 ms | 98 to 100% |
| `widths.md` | 6 700 px | 869 | 7 to 9 ms | 17 ms | 98 to 99% |
| `README.md`, at its top, 1100 px window | 17 000 px | 845 | 6 to 7 ms | 17 ms | 98 to 100% |
| … 2000 px down, the sidebar open, 1100 px window | 16 600 px | 845 | 6 ms | 17 ms | 98 to 99% |
| … 2000 px down, a 700 px window | 16 500 px | 845 | 6 ms | 17 ms | 98 to 99% |
| … 2000 px down, a 1512 px window | 15 000 px | 845 | 6 ms | 17 ms | 98 to 99% |
| … 2000 px down, 1100 px window | 17 000 px | 845 | 7 to 16 ms | 17 to 24 ms | 76 to 99% |
| README × 2 | 34 000 px | 1 690 | 11 to 19 ms | 18 to 30 ms | 13 to 84% |
| README × 4 | 68 000 px | 3 380 | 20 to 33 ms | 29 to 41 ms | 0 to 2% |
| README × 8 | 136 000 px | 6 760 | 40 to 48 ms | 51 to 63 ms | 0% |
| README × 15 | 255 000 px | 12 675 | 75 to 78 ms | 90 to 94 ms | 0% |
| README × 30 | 509 000 px | 25 350 | 153 to 158 ms | 176 to 186 ms | 0% |
| The JSON's first chunk | 659 000 px | 60 067 | 261 to 266 ms | 295 to 303 ms | 0% |

- The frame is the call plus about 10 ms, more on the largest (painting), and never under
  one refresh of the display. So the call's time follows the frame's length.
- The call's time follows the height: about 0.3 ms for each 1 000 px on the README's kind of
  page, 0.4 ms on the JSON. And the count of elements: about 2 ms and 6 ms more for each
  1 000 on the README's kind of page (12, 22, 43, 78 and 154 ms by that rule for the five
  sizes, against the 11 to 19, 20 to 33, 40 to 48, 75 to 78 and 153 to 158 ms measured), and
  4.4 ms for each 1 000 on the JSON.
- The README is one frame long everywhere but one set-up, scrolled 2000 px down in the
  1100 px window, on two of the three ramps, both times it was run: a call of 15 to 16 ms and
  a frame of 23 to 24 ms. With the sidebar open, in the other two widths and at the top, the
  same scroll costs 6 ms. The cause was not looked into.
- A ramp's first call runs long: 10 to 36 ms on the small documents, 24 to 43 ms on the
  README two and four times over.
- The trial's Z read worse on the README than this (11 runs of 80 under the bar, the worst
  with the sidebar open): there a pinch's events came one to three to a frame through the
  trial's own handler. The numbers here time the call the build will make, and the walk times
  the build itself.

## Review of the mix

By a fresh reviewer, read-only, over eight passes; all findings taken.

Pass 1, nine: the window's own save, come back, would have dropped the next pinch once the
window's level follows the fingers; the time taken is the call's, not a frame's; live, the
small documents lose the frames the scaled page kept; a frame that outlasts the 200 ms timer
could never count to two; the memory by tab entry dropped; a reload whose held node is
`#content` itself; the walk could not move the numbers; checks that could not fail; one
label.

Pass 2, five: the page can change with no render, so the node is tested as well as the
token; the time is taken around the call, where the walk's slow frames fall; what the
measurement's two bad outcomes would mean; lines the mix made stale, here and in the code's
comments; a walk case that might not be reached.

Passes 3 and 4, one each: a limit that still said 60 frames a second, and a line too long.

Pass 5, eight, with the measurement in: the rule fires on pairs of slow frames in the tail,
not on medians, so a limit placed where calls scatter makes a document end one pinch true
and the next re-arranged; a pinch's first frame is left out; the line between true and
smooth is the owner's to draw, not chosen for them; and wording against the raw figures.

Pass 6, five: a height changes with the level and the window, so the cap became a count of
elements; the cap is set by the owner's hand with each document pinched both ways; what a
later change of it needs walked again.

Pass 7, three: the net leaves the cap up to about six READMEs, not four and a half; a page
heavy with diagrams counts high; a line too long.

Pass 8: no findings.

## Change review of the mix

Of `1a6a01b` (Task 6, by the `coder` agent; `node --check src/app.js` passes), by a fresh
reviewer, read-only. Nothing was run in the app. Some thirty cases traced through the code,
all as Task 6 has them; with no pinch on, `showDocZoom` is as it was.

Pass 1, four findings:

1. **A save of the file or a switch of tabs in the middle of a live pinch can land the page
   that arrives off its place** (*reasoned*): `renderDocument` puts the offset back a frame
   after the swap, and a live frame in between has changed the level, so the old level's
   offset is restored. About 40 px at 2000 px down and 100 to 300 px at 5000 to 10 000 px
   down, if a frame changes the level by 2 to 3%, a guess at a spread's speed. It is the
   document zoom's accepted limit *A change of zoom just as a document is shown can land it
   off*, which needed a step from another window, or a key within a frame of the switch, and
   which a live pinch now reaches with every frame. To the owner at once (2026-11-17):
   **recorded and measured, not fixed.** Turned down: scaling the restored offset by the
   change of level, two lines in code every platform runs. L5 measures it.
2. After a live pinch has slid at a page's end, then gone over, then turned back, the scaled
   page scales about the pointer's first height and the end holds the point there: a jump of
   up to the slide times the scale (*reasoned*). Likewise where a reload or a switch lost the
   held point before the pinch went over: the scaled page scales about the pointer's height
   and the end holds the top of the view. A limit for the triage.
3. and 4. Two comments, put right in `01d1c72`.

Left as it is, on the reviewer's reading: a level equal to the pinch's start, arriving in the
middle of a live pinch, sets the label in Settings to it. A pinch cannot start under
Settings, which is modal; if a key opens it in the middle of one, the label is right again
at the next live frame or at the end, within 200 ms.

Pass 2, four findings, none in the code's logic, all taken: "Settings cannot be open under a
pinch" was untrue (a key can open it in the middle of one); L5's new check could not make
the miss it measures, its events changing no level; the comment "Counted, not measured" could
be misread, and was reworded in `42b8268`; and findings 1 and 2 above had lost a depth, an
assumption and a second case.

Pass 3, two, both in this plan: for a switch of tabs L5 measured a line that is not on the
page that arrives; and a line too long.

Pass 4: nothing left in the code or in what the walk measures. Its one finding was that this
section recorded only pass 1, which these paragraphs answer.

## Walk of the mix (2026-11-17, macOS, `walk/smooth-pinch` at `04fbbf5`)

The same driver, at the app's own listeners. The gates pass (218 tests). F = 16.66 ms.

- **L1, live on the small documents: passes.** Held open, `--doc-zoom` and `state.docZoom`
  are the fingers' level and there is no transform, while `config.json` and a second window
  stay at the start level until the end. Nothing changes when the pinch ends: a paragraph's
  width, lines, left edge and top are the same to 0.0 px on the last frame and after, for
  100% to 50%, to 80% and to 201%, and 300% to 149%. The held point within 1 px on the last
  frame and across the ending. Frames, reported: a median of 17 ms on spreads (95 to 100%
  within 1.5 F) and 23 to 25 ms on squeezes to 50% (20 to 60% within it); no frame over
  100 ms; the worst 57 ms. The README's first two runs after launch were slow (2 to 6%
  within 1.5 F) and later repeats of them were not.
- **L2, the large documents: passes.** The README thirty times over and the JSON's first
  chunk start scaled, 24 runs of 24, and meet W1's bar as before.
- **L3, going over: passes**, each of the eight forced cases as designed (one slow frame,
  live; the first two slow, live; three, over on the third; two in a row later, over on the
  second; 110 ms once, over at once, the first frame too; slow and fast by turns, live). On
  going over the held point is within 1.1 px and the scale is the fingers' level over
  `state.docZoom`. The next pinch starts live. At the first cap of 3400 the README four
  times over (3380 elements) starts live and five times over (4225) scaled, from 100% and
  from 250% alike. Reported: with nothing forced the README four times over goes over by
  itself, from 100% at about 180% and from 250% on its third frame, with one frame of 196 ms
  at the switch.
- **L4, one save: passes.** 122 scripted pinches (88 live, 27 scaled, 7 that went over): one
  broadcast and one toast for each that changed the level, none otherwise.
- **L5, the edges, live: passes.** The first walk's list again, nothing stuck. Held open at
  165%: `showDocZoom` with the pinch's start level leaves the pinch on; a step (the function
  a key calls, not a keystroke) lands on 110% and stands, from this window and from a second.
  A save of the file: the top line 10 px off across the reload with the pointer on text, 0 px
  with it between two blocks. **Not reached:** the held node was never `#content` itself, on
  any test document; the token's test stands on reading alone. A switch of tabs: the tab
  left comes back 449 px off, and 450 px after the same change with nothing moving.
- **L5, a save and a switch under a moving pinch** (finding 1 of the mix's change review;
  every run reached the case). A save: the top line 0.5 to 46 px off at 2000 px down and 18
  to 58 px at 8000 px down at the restore, then more as the spread goes on (up to 346 px at
  300%). A switch: the arriving tab 219 to 448 px from its place, which is 21 to 82 px nearer
  than the same switch with nothing moving. So the switch's miss is the document zoom's own
  (*A tab not on screen comes back near its place*), and the save's is the live pinch's.
- **L6, by the owner's hand** (answers picked from the Mac session's wording, live called
  "Way A" and scaled "Way B"). The README 2000 px down, with the sidebar open, and in a
  full-width window, a squeeze to 50% and a spread to 250%: "Smooth, nothing changes" each
  time, 29 pinches, all live. The cap:

  | README | Elements | Live | Scaled | The driver's log of the live pinches |
  |---|---|---|---|---|
  | × 2 | 1 690 | "Slightly rough" | "A better" | 9, all live; calls of 11 to 28 ms |
  | × 3 | 2 535 | "Slightly rough" | "About the same" | 37: 31 live, 6 went over |
  | × 4 | 3 380 | "Too rough" | "B better" | 21: 18 went over |
  | × 6 | 5 070 | "Too rough" | "B better" | 7: 1 went over, 6 with slow calls apart |

  "Up to ×3". Eight and thirty times over, scaled: "Smooth, end fine".

**The owner's ruling on the cap** (2026-11-17, confirmed to the main session):
`PINCH_LIVE_ELEMENTS = 2600`, just over the README three times over; nothing between that
and four times over was pinched. Turned down: about 1 800 (up to twice over) and about 3 000.
Written as a `const` in the `fixup!` that follows. To walk again at the new value: the README
three times over starts live and four times over scaled, from 100% and from 250% alike.

## Triage (2026-11-17, with the owner)

Decided one at a time:

1. **The jump at the README's end by the owner's hand**: not accepted as measured. In their
   words it was the zoom not matching after the re-layout, which led to *What did not match*
   and ruling 7.
2. **A save or a switch of tabs in the middle of a live pinch** (the mix's change review,
   finding 1): recorded and measured, not fixed. It is a note on the accepted limit *A change
   of zoom just as a document is shown can land it off*.
3. **A tab not on screen loses its place after a change of zoom**: deferred. It was an
   accepted limit of the document zoom's, written as 16 to 254 px; the walks measured 449 to
   966 px on the README, by the keys and a pinch alike. Turned down: keeping it accepted with
   the real numbers, and fixing it in this change.
4. **On macOS, text stops shrinking below about 64% zoom**: deferred. The document zoom's
   own, found by the walk. Turned down: accepting it, and looking into it before this
   release.

Eighteen plain limits, put to the owner as one list and taken as proposed. In `open-items.md`
under *Accepted limits*, each with its trigger: a large document re-arranges when the fingers
lift; it is a picture while pinched; the text shifts at its top or end; a live pinch is not
always 60 frames a second; a document near the limit can go over in the middle of a pinch; a
pinch that slid at a page's end and went over can jump; the fully loaded 10 MB JSON stalls;
the last sliver of a pinch may be dropped; a reader's theme that changes the top padding; a
page heavy with diagrams; the memory on the GPU's side; one Mac; the release build unwalked;
Windows and Linux not walked after the mix; a reload case not reached. In `closed-items.md`,
kept with no trigger: a level off the ladder reads as 100% in 1.9.2 and older; 60 frames a
second at most on a 120 Hz display.

With them: the deferred item *On macOS, a pinch zooms a document in steps, not smoothly* is
closed, and the accepted limit on a burst of zoom steps creeping on empty space, whose
trigger was this plan, now names Linux alone.

## Re-check at the owner's cap (2026-11-17, macOS, `walk/smooth-pinch` at `d8aa83e`)

By script, one launch. `node --check` and `cargo fmt --check` pass and the build compiles;
clippy and the tests were not run again there, the Rust being as it was at `04fbbf5`.

- `PINCH_LIVE_ELEMENTS` reads 2600 in the running build and cannot be assigned.
- The README three times over (2 535 elements) is live on all four pinches (100% to 272% and
  to 50%, 250% to 300% and to 124%): a median frame of 24 to 26 ms, 18 to 57% of frames within
  1.5 F, the worst 36 ms. Four times over (3 380) is scaled on all four: 17 ms, every frame
  within 1.5 F, the held point 0 px on the last frame. None went over.
- Each of twelve pinches (four live on three times over, four scaled on four times over,
  four live on the plain README) ended once, with one broadcast and one toast, and left
  nothing behind.
- The plain README, live: the held point within 0.7 px; from 100% to 50% a paragraph's width,
  lines, left edge and top the same on the last frame and after the ending, to 0.0 px.
  Reported: these first pinches after launch ran at a median of 22 to 25 ms, against 17 ms in
  the walk of the mix, which saw slow first runs too.
