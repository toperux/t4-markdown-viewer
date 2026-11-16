# Smooth Pinch on macOS Plan

> **Status (2026-11-16):** the trial has run (*Trial results*), and the owner has ruled on the
> approach, the ending and the toast (*Rulings*, 2 to 4). The build is written (*The build*)
> and reviewed (passes 4 to 9: thirteen findings, then four, one, three, two and none, all
> taken). Nothing is built: the build waits for the owner's go.

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

With nothing to hold at the pointer (`zoomAnchor` answers null: the empty space below a short
document) the pinch holds what a key step holds, the point at the top of the view
(`app.js:4417-4419`), and scales about that. Else the text would scale about the pointer and
jump to the top of the view's hold when the pinch ends (*reasoned*). In an empty document
there is nothing to hold there either: the pinch has no anchor, its ending passes none, and
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
  on all five documents and the README's four set-ups, judged by the same bar.
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
  spread, a squeeze, and a spread with a rest in the middle. The owner says whether it is
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

Found by the trial, for the triage:

- When the fingers lift, the page is laid out once. Where text re-wraps, the point under the
  pointer moves sideways (100 to 600 px measured), and on a large document the page stands
  still for 0.2 to 0.4 s.
- While the fingers move, what the pane limits keeps its old wrapping and is cut at the pane's
  sides, and a squeeze shows empty space around the page.
- On a JSON file of 10 MB with every chunk loaded, a pinch stalls for over a second at its
  start and about 7 s at its end, as one zoom step does today. A real pinch may break into
  several there, each with its own save, toast and ending, where today's makes three to six
  steps of 7.9 s (*reasoned*).
- The page renders at 60 Hz on a 120 Hz display, so a pinch moves at 60 frames a second.
- Memory on the GPU's side for the layer was not measurable.
- A pinch near a document's end cannot hold its point when a squeeze leaves the page shorter
  than the view, as with a step today (*reasoned*).
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
