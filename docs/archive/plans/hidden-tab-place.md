# A Hidden Tab Keeps Its Place Through a Zoom Change Plan

> **Status (2026-11-17): done.** Planned and reviewed (five passes), built on the owner's go
> and reviewed (three passes), walked on Windows, macOS and Linux, mended at a fault the walk
> found (*The caret at a wrap*, reviewed in three passes), walked again on all three, and
> triaged. Five rulings. The record runs in the order things happened: read *Rulings* and
> *Status at the end* first; *The change* is the first design, and its code is not quite
> what was built.

**Spec:** the open item *A tab not on screen loses its place after a change of zoom*
(`docs/open-items.md` → Deferred; deferred by the owner on 2026-11-17 in the triage of
`archive/plans/smooth-pinch-macos.md`). Its recorded way to start from: remember the line at
the top of the view, as a step holds one on screen, not a number of px.

**Goal:** a document that was not on screen while the zoom level changed comes back with the
line that was at the top of the view where one change of level would have left it had the
document been on screen: the top of that line at the same distance from the top of the
view, within 2 px. (The top of the view is below the bar, or below the find bar when that is
open; if the find bar opened or closed meanwhile, the line follows the view's top, not its
old height in the window.)

## Rulings

1. **The line's top is held, as a step holds it** (the owner, 2026-11-17, on review pass 2's
   finding 1). Turned down: holding the exact point, as a pinch does, which the plan had
   chosen without asking; it lands a tab that was away up to a line from where the same tab
   would have landed on screen (about 10 px from 100% to 150%, reasoned).
2. **A place's mark holds a number worked out from the text, not the text** (the owner,
   2026-11-17, on the change review's pass 1, finding 1). As first built it held 24
   characters of the document, which went into `session.json` with every report and into a
   tab sent to another window, where until then no run of a document's text had been: paths,
   offsets, and the ids of headings a reader had jumped to, which are words of their
   headings. Turned down: keeping the characters, the file being local.

**Architecture:** frontend only (`src/app.js`), plus the records. No Rust change: a tab's
entries travel and are saved as the frontend's own JSON (`session.rs:50`, `packTab`
`app.js:2610`). No new dependency. Every platform runs it.

## What the user sees

Two tabs are open. The reader changes the zoom while looking at one, by the keys, the wheel,
Settings or a pinch. Switching back to the other, the page is not where it was left: on a
long document it is hundreds of px away, most of a view or more. The same holds for a tab
closed and reopened (`Ctrl+Shift+T`), for a document left for a picture in the same tab, and
for Back to another document.

It needs a change of level while the document is away. With the level unchanged a tab comes
back exactly, and that path is not touched (*What stays as it is*).

## Measured

- `archive/plans/document-zoom.md`, the trial (M7): keeping the place by the text at the top
  of the view leaves under 1 px; scaling the offset alone drifts 16 to 254 px. The walk: 32
  to 35 px (a hidden tab, a tab reopened, a document left for a picture tab).
- `archive/plans/smooth-pinch-macos.md`, the walks (macOS, the README): 449 to 450 px after a
  pinch or a step from 100% to 165%; 966 px after 100% to 150%, by three key steps and by a
  pinch alike; a tab switched to in the middle of a change 219 to 448 px.
- Not measured: Windows and Linux on the README (the earlier figures are theirs too, on other
  documents); a JSON document; a document whose file changed while it was away.

**Why it drifts (read in the code, and measured for the README in *What did not match* of
the pinch's plan).** A page's height is not the level times a constant: the reading column
changes width in px of the page as the level changes (the pane limits it, and it switches
between its two measures), so text re-wraps, and some blocks keep their size. The error adds
up down the page.

## Current behaviour (main at `6dc41ab`, v1.9.3)

- An entry is `{ path, scrollY, hash?, extent? }` (`makeTab`, `app.js:374`). `scrollY` is the
  page offset in px at the level it was banked at.
- `rememberScroll` (`380-393`) banks `window.scrollY` on `shownEntry`, or the target of a
  restore still holding the page. Its callers: the window's `scroll` listener (`6569`);
  `showActive` at the swap (`2279`, `2303`) and before an error (`2324`); `removeTab`
  (`2564`); `go` (`3391`); `pushAnchorEntry` (`3489`); the drag's end (`5075`);
  `update-installing` (`6700`); `showDocZoom` (`4467`); a hold's abort (`1903`).
- `showDocZoom` (`4429-4475`) keeps the place of the document on screen by what was at one
  point (`zoomAnchor`, `4359-4405`: the caret's text, else the innermost element across the
  point). Every other entry, in `tabs` and `closedTabs`, has its `scrollY` multiplied by the
  ratio of the levels (`4458-4460`). That multiplication is the drift.
- A document is put back in `renderDocument` (`1767`), a frame after the render: `scrollTo`
  the offset, then `holdScroll` (`1891-2029`) puts the same offset back each time the content
  changes size, until its pictures have settled or the reader scrolls. `go` puts an entry of
  the document already on screen back with one `scrollTo` (`3402-3406`).
- After the render, `highlight`, `wrapTables`, `renderDiagrams` and `addCopyButtons` run in
  the same task (`1814-1825`), and find paints with `CSS.highlights`, not with elements
  (`3573`). So two renders of the same file at the same `extent` give the same tree of nodes
  (read, not measured).
- The page a reader leaves is not always the tree a render gives (read, review pass 1). A
  diagram that failed on one showing and draws on the next has a paragraph more or fewer
  beside it (`sayDiagramFailed`, `1361`). A fold changes no node (`5638-5647`).
  **Corrected by the walk:** this bullet also said that a JSON document's *more* splices
  its chunk in as a text beside the text already there, where a render gives one text node.
  It does not: a chunk starts at the comma where the cut fell and is spliced in as the same
  spans and texts a render makes (`json.rs:546-558`, `587-612`), so the two trees are the
  same and a place below the splice resolves (measured on all three platforms). Two
  exceptions, read and not measured, both failing their mark: bare text before that comma
  (white space, or a token the viewer does not know, `json.rs:490`), and a render at the
  larger extent that stops folding (`emit_within`, `json.rs:431-436`). What the plan says of
  the splice from here to *Walk results* was written before that.

## The change

An entry that leaves the screen also banks **what was at the top of the view**, in a form
that outlives the page. When it comes back and the level is no longer the one it was banked
at, the page is put back by that, not by the scaled offset. The scaled offset stays as the
fallback.

### The place

```js
entry.at = { path, offset, down, frac, mark, zoom }
```

- `path`: the node, as its index among its parent's `childNodes` at each step down from
  `#content`.
- `offset` and `down`, for text: the caret's offset in that node, and how far below the top
  of the caret's box the top of the view was, in window px, which no level changes.
- `frac`, for an element, where there is no caret: the share of its height at which the top
  of the view was. A step does the same there (`4401-4404`).
- `mark`: enough of the node to tell it from another that has taken its path: a text's
  length and a number worked out from 24 characters at the offset (ruling 2; the code below
  is as first planned, with the characters themselves); an element's name, its count of children and
  the source lines of the block it is in (`data-sourcepos`, which a Markdown page's blocks
  carry and a JSON page's do not).
- `zoom`: `state.docZoom` when it was taken.

It is found with `zoomAnchor` at `viewTop()`, the point a key step already holds, in its
`point` mode, which is the one that carries the node. Two more properties go on the function
it returns: `offset` and `down` (`y - r.top`) on the caret's branch (`4372`), `frac` on the
element's (`4404`).

**The line's top is held** (ruling 1): the top of the caret's box goes back to the height in
the window it had, which is what a step's own function measures (`4384`). A step's aim
(`zoomHeld`, `4378-4383`), which takes a line within 2 px of where the last step aimed it
back to that aim, is not carried over: a tab that leaves right after a step may bank a top
up to 1.2 px off that aim (the measured error of a step on WebKit), inside the bar.

```js
/**
 * Enough of a node to tell it from another that has taken its path. A picture
 * is like its neighbours in name and children, so an element goes by the
 * source lines of its block too.
 */
function placeMark(node, offset) {
  if (node.nodeType === 3) return `${node.length}:${node.data.substr(offset, 24)}`;
  const lines = node.closest?.("[data-sourcepos]")?.dataset.sourcepos;
  return `${node.nodeName}:${node.childNodes.length}:${lines}`;
}

/** What is at the top of the view, in a form that outlives the page; undefined when nothing is. */
function placeAt() {
  const aim = zoomHeld; // leaving is no step: a step's aim stands
  const held = zoomAnchor(...viewTop(), true);
  zoomHeld = aim;
  if (!held) return undefined;
  const path = [];
  for (let n = held.node; n !== els.content; n = n.parentNode)
    path.unshift(Array.prototype.indexOf.call(n.parentNode.childNodes, n));
  const { node, offset, down, frac } = held;
  return { path, offset, down, frac, mark: placeMark(node, offset), zoom: state.docZoom };
}

/** Where a banked place is now, as a page offset; null when its node is gone or shows nothing. */
function placeY(at) {
  try {
    let node = els.content;
    for (const i of at.path) node = node.childNodes[i];
    // Another node at that path: a chunk spliced into a JSON page, a diagram
    // that drew this time, a file that changed while away.
    if (placeMark(node, at.offset) !== at.mark) return null;
    let r;
    if (at.offset == null) r = node.getBoundingClientRect();
    else {
      const range = document.createRange();
      range.setStart(node, at.offset);
      r = range.getBoundingClientRect();
    }
    const down = at.offset == null ? at.frac * r.height : at.down;
    const y = window.scrollY + r.top + down - viewTop()[1];
    return (r.x || r.y || r.width || r.height) && Number.isFinite(y) ? Math.max(0, y) : null;
  } catch {
    // No such path, or no place at all: a session file from elsewhere.
    return null;
  }
}
```

A place that fails its mark falls back to the scaled offset, which is today's behaviour: a
wrong place would be worse than that. (As built, the comment in `placeY` names the diagram
and the changed file alone; `placeMark` and the walk up the tree differ from the code above;
and a caret's box is read through `caretBox`: rulings 2 and 3, and *Walk results, Linux*.)

The `try` is the check at a trust boundary: `at` comes back from `session.json` and from
another window as it was written (`rebuildTab`, `2616-2620`, takes the entries whole).

### Banking it

`rememberScroll(leaving = false)`. Its last branch becomes two:

```js
// A restore still holding the page: where it's going, not where anchoring
// has it on the way (a reload, a switch or a report mid-restore). And the
// place it was handed: the swap's own scroll may have cleared the entry's.
else if (scrollHold?.entry === shownEntry) {
  shownEntry.scrollY = scrollHold.target;
  shownEntry.at = scrollHold.at;
} else {
  shownEntry.scrollY = window.scrollY;
  // What is at the top of the view, for a level that changes while this page
  // is away. Only on the way out: it costs a hit test, and a place kept through
  // a scroll would be stale. The top of a page needs none.
  shownEntry.at = leaving && window.scrollY > 0 ? placeAt() : undefined;
}
```

`true` is passed where the page on screen is about to leave it: `showActive` (`2279`, `2303`,
`2324`), `removeTab` (`2564`), `go` (`3391`) and `pushAnchorEntry` (`3489`). The rest pass
nothing, so a scroll, a zoom step and a hold's end clear the place: it is only ever as old as
the offset beside it. `JSON.stringify` drops an `undefined`, so a session file carries a
place only for an entry that has one.

### Putting the page back

`showActive` hands the place to `renderDocument` when it hands it the entry's own offset
(`2306`): `renderDocument(doc, scrollY ?? entry.scrollY, entry.hash, scrollY == null ?
entry.at : undefined)`. It is taken there, before the render, because the `scroll` event of
the swap can clear the entry's own before the frame in which the page is put back. In that
frame (`1856-1857`):

```js
// The level changed while this page was away, so its offset was only scaled
// (`showDocZoom`): go by what was at the top of the view.
const place = at?.zoom !== state.docZoom ? at : null;
const y = (place && placeY(place)) ?? scrollY ?? 0;
window.scrollTo(0, y);
if (y > 0) holdScroll(y, token, entry, at);
```

`holdScroll(target, token, entry, at)` keeps the place it was handed beside its target, for
`rememberScroll` above, and puts the page back through one function, in place of its five
`window.scrollTo(0, target)` (`1914`, `1923`, `1937`, `1952`, `1954`):

```js
const me = (scrollHold = { hold, target, entry, at });
// After a change of level the page goes by its place's node, which the
// pictures above it move as they load.
const place = at?.zoom !== state.docZoom ? at : null;
const put = () => {
  if (place) me.target = target = placeY(place) ?? target;
  window.scrollTo(0, target);
};
```

With the level unchanged `put` is the `scrollTo` it replaces. The place a hold was handed
matches its target: banked together with it at an unchanged level, and what the target is
worked out from after a change. One that fails its mark is never used for the target.

`go`, for an entry of the document on screen (`3404`), uses the same two lines as
`renderDocument` with `to.at` and `to.scrollY`, and one `scrollTo`.

### What stays as it is

- **The level unchanged while away: the common case.** `at.zoom === state.docZoom`, so
  `place` is null and every line runs as today, to the px. The cost: a hit test and a walk
  up the tree each time `rememberScroll(true)` runs with the page scrolled, which is once as
  a page leaves, twice when a tab closes or Back goes to another file (`2564` or `3391`,
  then `2303`), and once at every live reload, where the page does not leave (`2303`); and
  some 100 bytes for each place in `session.json` and in a tab sent between windows.
- **`showDocZoom` still scales every hidden offset.** It is what a place falls back to: no
  place (a picture, the top of a page, an entry banked by an older copy), or one that no
  longer resolves.
- **`zoomAnchor` for a step and for a pinch.** It only gains two properties on what it
  returns.
- **The entry on screen, the hold's rules for letting go, find, pictures.**

## Tasks

### Task 1: the build

The change above, in `src/app.js`. Comments as the code around them. Gates: `cargo fmt
--check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `node --check
src/app.js`. One local commit, a release-note line: *A background tab keeps its place when
the zoom changes*.

### Task 2: the walk, on Windows, Linux and macOS

By script (`drive-app`), on the VMs, on the owner's go; code that is not pushed goes on a
`walk/*` branch. Each figure is taken twice: at the base (`6dc41ab`) and at the build. The
measure: the caret at `viewTop()` is marked before the page leaves (its node and offset, and
the top of its box in the window, read by the driver with `caretRangeFromPoint`, not from
`entry.at`); after the round trip, how far the top of that same caret's box is from where
it stood, in px. The caret under the point, not a line's first character: the line re-wraps
at the new level.

- **W1, the fault.** The README from twenty places between 500 px and its end, in a second
  tab; the level changed in the first tab; back. Levels: 100% to 150%, 100% to 50%, 100% to
  300%, 250% to 80%; by the keys, and on macOS by a pinch too; in a 1100 px window, with the
  sidebar open, and at full width. **Bar: within 2 px** where the top of the view was on
  text, the bound a step on screen is held to; and for each place the same change of level
  made in one go with the tab on screen lands within 2 px of the tab that was away. The
  driver sets `zoomPoint` to null, as a key step leaves it, and calls
  `invoke("set_doc_zoom", { percent })`, the call a pinch's end makes: it reaches
  `showDocZoom` with no anchor of its own, which then holds the top of the view as a key
  step does. Not by the keys
  there: they go a rung at a time, each with a hit test of its own, and on the way down a
  line that has shrunk can hand over to the line below. The base's figures are recorded
  beside them. A place a whole line off is looked
  into: a caret that falls exactly where a line wraps at the new width may be given the end
  of the line before (reasoned, differs by engine).
- **W2, no text at the point.** The top of the view on a picture, on a table's white space,
  beside a heading: within two lines of text, the bound of a step with no text to hold.
- **W3, the other ways back.** A tab closed and reopened; a document left for a picture in
  the same tab; Back to another document; Back between two headings of one document (`go`'s
  own `scrollTo`). Bar as W1.
- **W4, the common case is untouched.** The level unchanged: every round trip of W1 and W3
  lands as at the base, to the px, and `placeY` is not called (counted by the driver). On
  Windows with slow pictures above the place (`slow.md` of the earlier walks), where the hold
  does its work.
- **W5, pictures above the place, the level changed.** The README's badges and `slow.md`:
  the line's top is within 2 px once the pictures have settled, and a wheel or a key during the
  hold still lets go. And a tab left again while that hold still runs, the level changed
  once more, and back: within 2 px, where the entry's own place had been cleared by the
  swap's scroll. That clearing needs the swap to clamp the scroll, so the tab is come to
  from a page scrolled further down than the README is long; the driver wraps the global
  `rememberScroll` and logs the entry's `at` after each call, to show the clearing and the
  hold's putting it back (the hold writes it back and its end clears it again, so one
  reading after the swap proves nothing).
- **W6, what a place cannot follow.** The level is changed in every case, or no place is
  tried. Each falls back to the scaled offset, lands where the base lands, and leaves no
  error in the console: the file changed while away (a paragraph added above the place, and
  removed), with the place on text and with it on a picture; a JSON document after *more*,
  with the place below the splice (which the walk found to resolve, rightly: *Walk
  results*); a diagram above the place that failed on one showing and
  drew on the next (the driver puts a failure for that diagram into `diagrams`, the cache
  of drawings, before the first showing, as `{ error }` under `diagramKey(diagramLook(),
  source)`, and takes it out while the tab is away, the theme unchanged; a diagram that
  really times out would time out again). And two a place does follow, to the bar: a
  JSON document with a node folded above the place, and with the place above the splice.
- **W7, the place travels.** A background tab's place is in `session.json` after a report;
  the app quit and started again at the same level puts it back by its offset, as today; a
  tab dragged to another window lands as today. A `session.json` whose `at` is hand-edited
  to nonsense and whose `at.zoom` is not the saved level, so that it is tried (a string, a
  path of 10 000, an offset past the text, a `mark` of another text), falls back to the
  offset with no error.
- **W8, the cost.** The time `rememberScroll(true)` takes, and one `placeY`, which a hold
  calls at every change of the page's size: on the README, the README thirty times over,
  the 10 MB JSON's first chunk and that JSON fully loaded (2.3 million text nodes, whose
  parents' lists of children are the longest there are). Reported; over 5 ms is put to the
  owner.
- **W9, a change of level under a restore** (macOS, the pinch's L5 again): a save of the
  file, and a switch of tabs, in the middle of a moving live pinch. Reported beside the
  pinch walk's figures (0.5 to 58 px for the save): the place is taken at the swap, at the
  level of that moment, so the frame that puts the page back at another level goes by it
  (read in the code by the plan and by the review; not measured). If the walk bears it out,
  this mends a case the owner ruled on 2026-11-17 to record and not to fix, at no cost of
  its own; it is put to the owner with the figures either way. And a switch of tabs in the
  middle of a scaled pinch on a large document (the README thirty times over), ended at
  another level, and back: there the place is measured under the pinch's transform, with
  the level still the pinch's start (change review, pass 1). Reported.

### Task 3: the records

`docs/open-items.md`: the deferred item closes with what was done and measured. Accepted
limits whose wording the walk changes are put to the owner in the triage, not edited here: *A
change of zoom just as a document is shown can land it off*, and *A session offered on the
empty panel keeps the old zoom's places*.

## Limits known now

- **Where the top of the view has no text**, the place is the innermost element across the
  point and a share of its height: a line or two off, as a step on screen is there. Under
  Settings, which a hit test cannot see through, the same.
- **A line that shrinks can end above the view**, as after a step on screen: its top stays
  where it was, and the band between the bar and the top of the view is 2.5rem, about 40 px,
  so it takes a line taller than that, body text from about 170% up or a large heading
  (read, review pass 2).
- **A file that changed while its tab was away** fails its mark and falls back to the scaled
  offset, as today, unless the node at that path happens to pass for the old one: a text as
  long that reads the same there, or whose 24 characters give the same number by chance
  (one in four thousand million); or, where there are no source lines to go by, an element
  of the same name and count of children. That is a JSON page, and what the page makes
  itself with no source lines above it: a diagram's figure and its drawing
  (`1384-1398`), a table's scroll box (`447-453`). For a wrong place there it takes a
  changed file and two diagrams, or two such boxes, as many blocks apart as were added
  above: next to each other, for one paragraph added.
- **A diagram that changes between failing and drawing** above the place: the same fallback
  (W6). So do two rare JSON pages after *more* (read, not measured): one whose cut fell
  after bare text, and one that a render at the larger extent no longer folds. Any other
  JSON page after *more* is followed, below the splice too (measured; the plan first said
  it would fall back).
- **The tab on screen when the app quits** has no place, so the one case of the accepted
  limit *A session offered on the empty panel* stays for it; its background tabs gain one.
- **A fold re-opened by a render** moves the layout and no node, so after a change of level
  a place follows it where an offset does not (read; W6). With the level unchanged the
  offset is used and the page moves with the fold, as today.
- **A copy of 1.9.3 or older** ignores `at` and writes it back unchanged with the entries,
  so a place can outlive its offset across a step back to an old copy and forward again. It
  is used only if the level also changed in between.

## Chosen without asking

- **By the node, not by a share of the page's height.** Scaling the offset by the height the
  page had and has is three lines, but it is still an estimate (what keeps its size is not
  spread evenly), it was not measured, and it needs the same change to the hold.
- **A step's aim is not carried over** (*The place*): up to 1.2 px, inside the bar.
- **A mark on the place** (review pass 1, finding 2, put there as the owner's to decide;
  taken as the one clearly right of its three). A few lines turn the places that would
  resolve to the wrong node into today's behaviour, bar the look-alikes named in *Limits*.
  The others: bank no place on a JSON page after *more*, which leaves the diagram and the
  changed file; or accept wrong places. (The JSON page was the first reason given for the
  mark and turned out not to need it; the diagram and the changed file do, as walked.)
- **Banked on leaving, not on every scroll.** One place fewer to pay for it, and no stale
  place to guard against.
- **The scaled offset stays** as the fallback, where it could have been dropped.
- **`go` between two headings of one document is in.** The same fault by another road, and
  two lines.
- **Nothing in Rust.** The place rides in the entry, which Rust already keeps whole.

## Review

**Pass 1, eight findings, all taken.** The place held the point where the goal and the bars
spoke of the line's top, so the bar could not be met as written (answered at first by
keeping the point and rewording the goal; see pass 2). The tree a reader leaves is not
always the tree a render gives, so a path could resolve to the wrong node, which is worse
than today (a mark on the place; the limit's worry about folds was misplaced). A hold could
run by a place the
entry no longer carried, the swap's own scroll having cleared it (the hold keeps it). W7's
nonsense places were never tried at an unchanged level. W8 left out the fully loaded JSON
and `placeY`, and the walk up the tree copied each list of children. The cost named for the
common case undercounted. W1's marked line was not defined. A caret exactly at a soft wrap
may be given the line before (noted in W1). Every line number and the list of
`rememberScroll`'s callers checked out.

**Pass 2, five findings, all taken.** Point or line's top was the owner's to decide, and the
case made for the point was overstated: a line stays in view under the bar unless it is over
40 px tall (asked at once; ruling 1, the line's top). An element's mark went by its name and
count of children alone, which a picture shares with its neighbours (the source lines of its
block are added). W6 could not tell a fallback from a place followed unless the level
changes, and its diagram case had no way to be made. W5 might never reach the cleared place
it is there to measure. One sentence said a hold's place and target were always banked
together. Carrying the place in the hold opened no stale or wrong place, by a trace of each
way a hold is left.

**Pass 3, five findings, all taken.** W1's comparison with the keys on screen could not meet
its bar on the way down, the keys going a rung at a time (one change in one go now). The
limit on look-alike marks left out the elements the page makes itself, which have no source
lines above them. W5's one reading of the entry's place proved nothing either way. W6's
diagram would have failed again on the second showing. The goal spoke of a height in the
window where the code holds a distance from the top of the view. The line's-top rule is the
quantity a step measures, with the same sign, and the source lines in the mark behave as the
plan says, both by reading.

**Pass 4, two findings, both wording, both taken.** W1's one change on screen was described
as what a pinch's end does, which applies the level itself; and it needs `zoomPoint` null,
which the plan did not say. The look-alike condition counted lines where the path counts
blocks.

**Pass 5: no findings.**

## The build (2026-11-17)

Task 1, on the owner's go: `caf73b0`, *A background tab keeps its place when the zoom
changes*, `src/app.js` alone (115 lines added, 28 removed). As planned, bar two calls
wrapped over several lines. The gates pass (218 tests).

## Change review

**Pass 1, three findings, no bug.** The diff does what *The change* says and nothing more;
every caller of `rememberScroll` passes what the plan gives it; nothing new runs on a
scroll; no exception can leave `placeAt`; with the level unchanged a page is put back as
before.

1. **A place's mark put 24 characters of the document into `session.json`.** To the owner
   at once: ruling 2, a number worked out from them. A `fixup!`.
2. The walk had no leave in the middle of a scaled pinch, where the place is measured under
   a transform. Added to W9.
3. A comment said the place is banked "only on the way out", and a live reload banks it
   too. Reworded in the same `fixup!`.

**Pass 2, two findings, both wording, both taken.** The new comment and ruling 2 said the
session held no word of a document, and it holds the ids of headings jumped to (now "no run
of a document's text"; a second `fixup!`). Ruling 2 cited the finding by another number
than this list's, and *The build* called removed lines changed. The number's code is right,
and nothing else in a place carries a document's text.

**Pass 3: no findings.**

## Walk results (2026-11-17, `walk/hidden-tab-place` at `cc32f71` against 1.9.3, `d9fc067`)

By script on Windows (WebView2, CDP) and macOS (the in-app driver); Linux's is further down.
Gates pass on both (218 tests). The measure is the plan's: the driver's own caret at
`viewTop()`, marked before the page leaves. All measured unless marked.

| Step | Windows, build | Windows, 1.9.3 | macOS, build | macOS, 1.9.3 |
|---|---|---|---|---|
| W1, on text | 0.48 px at most | 4 to 45 443 px | 1.0 px at most (186 places) | 797 to 44 401 px |
| W1, against the change on screen | 1.00 px at most | | 0.9 px at most | |
| W2, no text at the point | 1.1 px | 12 to 1 877 px | 1.0 px | up to 25 757 px |
| W3, the other ways back | 0.7 px | 62 to 3 241 px | 1 px | 762 to 16 528 px |
| W4, level unchanged | as 1.9.3 to the px; `placeY` never called | | the same, 88 round trips | |
| W5, pictures above, a real hold | 0.41 px | 386 to 9 948 px | 0.6 px | 310 to 2 193 px |
| W5, left again in the middle of the hold | 0.27 px | 3 109 to 8 129 px | 0.8 px | 760 px |

- **W1, on macOS a pinch from the other tab** lands as the keys do, to 1 px.
- **W1, the page's end.** The last one or two places, zooming out, move 57 to 724 px on
  Windows and 273 to 555 px on macOS, with the tab away and on screen alike, on both builds.
  Verified on Windows: the page sits exactly at its scroll limit (6881 of 6881, where the
  line needs 6938). A shorter page cannot scroll that far.
- **W1, between two blocks.** The driver's caret there is the article itself, whose top moves
  with the level (12 to 1 807 px read, and the same on screen): the marker's figure. By what
  the app holds there, the next block below the point: 0.48 px at most, the same on screen.
- **W5.** A wheel and a key during a hold each let go, within 1 ms on Windows; on macOS a
  scripted scroll stood in for both. The swap's scroll clearing the entry's place was seen
  on macOS and not on Windows, where the hold was already running at the first bank; both
  land on the place.
- **W6, what a place cannot follow: as planned, bar one.** A paragraph added or removed
  above the place, with the place on text and on a picture, and a diagram that failed and
  then drew: `placeY` gives null and the page lands where 1.9.3 lands, to the px, with no
  error. A fold above the place, re-opened by the render: followed (0 and −0.5 px, where
  1.9.3 is 6 584 and 8.5 px off). **A JSON page after *more*, below the splice, did not fall
  back: it resolved, rightly** (0 px on both platforms). The plan's reading was wrong: the
  chunk is spliced in as the same nodes a render makes (a record's own spans and texts inside
  the same `fold-body`), not as one text beside the old one, so on this file the page left
  and the page rendered are the same tree.
- **W7.** `session.json` carries a place for each background tab (`mark "303:-1976457684"`);
  its only strings besides paths are the version, the sort and the marks. A restart at the
  same level puts every tab back by its offset, `placeY` uncalled. Four kinds of nonsense
  place fall back with no error. A tab torn off into a new window lands as on 1.9.3 (−33 px
  on both builds and both platforms: that window's bar is shorter).
- **W8, the cost of `rememberScroll(true)`** (1.9.3: 0 to 1 ms everywhere; one `placeY`:
  under 1 ms everywhere):

  | Document | Windows | macOS |
  |---|---|---|
  | README | 0.5 ms at most | 1 ms at most |
  | README thirty times over | 2.9 ms at most | 0 to 2 ms, 6 ms at most |
  | 10 MB JSON, first chunk | median 0.1 ms; 47 ms once in 21 | 0 to 3 ms, 10 ms at most |
  | 10 MB JSON, fully loaded | not loaded | medians 4 to 37 ms; 322 ms once |

  On the fully loaded JSON (1.73 million text nodes counted, where a comment in the code
  says 2.3) the hit test is 26 ms and the walk up the tree 11 ms, through one parent of
  170 001 children.
- **W9, macOS: the cases under a pinch are mended.** A save in the middle of a moving live
  pinch: 0.9 to 1.0 px (1.9.3: 32 to 267 px). A switch of tabs in the middle of one: 0.1 to
  1.4 px (93 to 640 px). A switch in the middle of a scaled pinch on the README thirty times
  over, ended at another level, and back: 0.4 to 0.9 px (23 350 to 68 928 px).
- **Memory, Windows.** The first run was stopped by Claude Code, the VM short of memory with
  the 10 MB JSON re-rendered at a larger extent (the renderer at 3.4 GB). Resumed on the
  owner's go without ever loading it fully. The renderer's working set on the same steps,
  build against 1.9.3: README 176 and 172 MB, thirty times over 291 and 291, the JSON's
  first chunk 708 to 738 and 653 to 684, after one *more* 1 279 and 1 161, peak 3 415 and
  3 392. No clear difference from this change; the JSON's re-renders are what weigh.
- **Seen beside it, on both builds:** on macOS a tab switched to in the middle of a scaled
  pinch has its first frame drawn under the old page's scale, and is right once the pinch
  ends; and the first place read ±26 px once on a close and reopen after a change of level,
  0 on repeats.
- Nothing differed from 1.9.3 in ordinary use; no error in the console in any run; the
  owner's files restored byte for byte on both.

**Found by the walk: a caret where a line wraps** (macOS, 3 of about 235 text places). At the
new level the line wraps exactly at the banked caret, and WebKit gives that caret a box of
all zeros. `placeY` takes a zero box for no place and falls back: 1 350, 1 518 and 5 890 px
off, as on 1.9.3. The same change made on screen is worse, and is 1.9.3's own: `zoomAnchor`
reads the zero box as a move, and the page jumps 4 945, 6 312 and 15 302 px, at the same
places on both builds. Windows shows none.

**Rulings from the walk** (the owner, 2026-11-17, each asked at once):

3. **The caret at a wrap is mended now, for the step on screen and the tab that was away
   alike**: one helper that takes the box of a character beside the caret where WebKit gives
   none. Turned down: mending the tab that was away alone; recording both.
4. **The cost on a fully loaded 10 MB JSON is accepted**, on macOS's figures: medians up to
   about 40 ms more to leave such a tab, once 322 ms, where one zoom step already takes
   about 8 s. Not covered by it, and put to the owner again with the figures after the
   walk up the tree was changed: Linux's far larger cost there (*Walk results, Linux*), and
   the readings over the plan's 5 ms on other documents (the README thirty times over 6 ms
   at most on macOS; the JSON's first chunk 10 ms at most on macOS, 19 ms once on Linux,
   47 ms once in 21 on Windows). Turned down: banking no place
   above a size, which would take the fix from the documents that drift furthest.

## The caret at a wrap (ruling 3)

### Measured first (macOS, 2026-11-17, `cc32f71`, by page evaluation, nothing built)

- **Every box of all zeros is at a wrap.** Of 36 862 offsets in the README's 962 rendered
  texts, in a 1100 px window: 258 at 150%, 575 at 300%, 177 at 80% (339, 817 and 255 with
  the sidebar open). Each is before the space, or the newline that stands for one, at which
  a line wraps. None at a text's start or end, none after a space. A collapsed range there
  has no rectangle at all (`getClientRects()` empty), 48 of 48 sampled.
- **The character before the caret always has a box** (48 of 48), and it is on the line that
  ends at the wrap, at that line's top and height. So has the character after it, and a
  collapsed range one offset back. A collapsed range one offset on is on the line that
  starts after the wrap.
- **That box is the caret's own where there is no wrap**: at 60 places (20 each at 80%, 150%
  and 300%; beside spaces, a hyphen, a slash, in code, a list item and a table's cell) the
  character before and the character after match the caret's box to 0.0 px in top and in
  height.
- **At the walk's three places**, with the character before: the line that ends at the wrap
  lands at the marked caret's old top, 0 px, for the tab that was away and for the change on
  screen alike (worked out by the driver from the app's two formulas, which come to the same
  scroll).
- **A hit test returns such a caret too.** With the point past a wrapped line's end at 150%,
  1 443 of 3 068 tests (1 867 of 3 656 with the sidebar open) came back with a box of all
  zeros, most of them text before the space at the wrap. `zoomAnchor` then takes its
  element's branch, a line or two off. At `viewTop()`, whose x is the column's middle, that
  needs a short line (reasoned, not counted).

### The change

One function, `caretBox(range)`: the range's own box, or where that is all zeros on a text,
the box of the character before the caret (of the first character, for a caret at offset
0). It replaces the three reads of a caret's box in `zoomAnchor` (the hit test's, and the
two functions it returns, a step's and a pinch's) and the one in `placeY`. Nothing else
changes: where the hit test gives no caret in a text (a heading's button; white space with
no line of text beside it), the box is still all zeros and the element's branch is still
taken. White space to the right of a wrapped line, in a table's cell too, is "past a line's
end": there the hit test hands back the caret at the wrap, which now holds that line.

So, on WebKit: a step on screen and a pinch no longer read a wrap as a move of thousands of
px; a tab that was away is put back by its place there too; and a hit test past a short
line's end now holds that line's text where it held the element. On WebView2, where the
walk met no box of zeros on text, nothing changes (to be walked).

**The line that ends is the caret's.** The caret stands after the last word before the
wrap, so the line that ends is the one whose words the reader had at the top; a collapsed
range one offset on would hold the next line, 35 px lower at 150%. Chosen without asking:
the measurement leaves no other candidate that is the caret's own line.

With it, in the same `fixup!`: a text's mark takes its 24 characters from either side of the
caret (12 before), where it took them after it. The walk met marks of the length alone,
`49:0` and `12:0`, for carets at their text's end.

### To walk again

- **macOS:** the scan above gives the places. At twenty of them per level (150%, 300%, 80%):
  the change on screen in one go, and the round trip of a tab that was away, land the line
  that ends at the wrap within 2 px, where 1.9.3 jumps. W1 once more, whole, in the 1100 px
  window with the sidebar open: no place excluded for a wrap, and the rest as before. W2
  once more (a picture, a table's white space, beside a heading), now that white space
  beside a wrapped line holds that line's text: within two lines as before, and which
  branch each place took. A step by a key and a pinch across a level at which the held
  caret meets a wrap: no jump. A hit test past a short line's end at `viewTop()`: the text
  is held (the function returned carries a text node). And W8 on the fully loaded JSON
  again, with a real switch away from it: ruling 4 rests on the Mac's figures, which were
  taken before the walk up the tree was changed.
- **Linux:** a scan of its own for carets with a box of all zeros, its fonts and wraps not
  being the Mac's; where it finds any, the same checks at those places. W1 and W2 once more
  in one window. And W8 on the fully loaded JSON again, with a real switch away from it,
  now that the walk up the tree counts back through the siblings.
- **Windows:** W1 and W4 once more, in one window, as a check that nothing moved; and W8 on
  the JSON's first chunk, where one reading in 21 was 47 ms.

## Walk results, Linux (2026-11-17, `cc32f71` against `d9fc067`, WebKitGTK under Xvfb)

By script, as on the other two; gates pass (223 tests there). Before the wrap's fix and
before the mark's wider window.

| Step | Build | 1.9.3 |
|---|---|---|
| W1, on text (240 round trips, three layouts) | 0.98 px at most | 14 to 43 332 px |
| W1, against the change on screen | 1 px at most | |
| W2, no text at the point | 0.99 px | 15 to 1 264 px |
| W3, the other ways back | 0.94 px | up to 2 375 px |
| W4, level unchanged | as 1.9.3 to the px; `placeY` never called | |
| W5, pictures above (three of 16.6 MB) | 0.11 px | |
| W5, left again in the middle of the hold | 0.44 px | 10 to 17 px |

- **The page's end** as on the other two: the last place of each pair that zooms out is 212
  to 551 px off, away and on screen alike.
- **No caret at a wrap was met**, and no place a whole line off. A table's white space took
  the text's branch in all nine tries: the box of all zeros recorded for it in the document
  zoom's trial did not come back.
- **W5.** A wheel and a key during the hold let go. A natural leave came after the hold had
  ended, so the leave was forced at the hold's start: the swap's clamp cleared the entry's
  place, the leave inside the hold banked the hold's back, and the page landed on it.
- **W6** as on the other two, the JSON below the splice resolving rightly here too (0 px,
  where 1.9.3 is 17 686 px off). **W7** as on the other two; a drag between windows is
  Windows' own and was not run. The owner's files were never touched (scratch XDG folders;
  checked by their dates, no hash was taken first).
- **W8, the cost: flagged.** The README, thirty times over, and the JSON's first chunk:
  2 ms at most (19 ms once on the chunk's first call). A fully loaded 10 MB JSON (3.08
  million text nodes, one array of 350 001 children): `rememberScroll(true)` a median of 108
  to 169 ms, and a real switch away from it about 0.6 s longer than on 1.9.3 (1.65 to 1.76 s
  against 0.96 to 1.16 s). Of that, the hit test is 9 to 78 ms, which a zoom step on 1.9.3
  pays too; the walk up the tree is 5 to 36 ms, and **its first call 0.5 to 6 s**: the
  position of a node among 350 001 siblings, asked of the list of children. **Not the Mac's
  file:** this one was made by a script for the walk (9.7 MB, one array of 350 001
  records), where the Mac loaded its own 10 MB JSON (19 chunks, 1.73 million text nodes, its
  largest parent 170 001 children). The two platforms' figures on "the fully loaded JSON"
  are of different documents and are not to be set side by side; the re-walk gives each its
  own before and after.
- **The mark of a caret at its text's end is the length alone** (`"33:0"`): 25 of 133 places
  sampled on the README. No wrong place came of it. Already answered by the wider window
  (*The caret at a wrap*).
- Nothing else differed from 1.9.3 in ordinary use; no error in the console.

**After the walk: the walk up the tree counts back through the siblings** (a `fixup!`),
where it asked the list of children for the node's index. The same number, without the
list. Not measured yet; ruling 4, which rested on the Mac's 37 ms, is put to the owner again
with Linux's figure once it is.

## Review of the wrap's fix

**Pass 1 (`e6d3f21`), five findings, no bug.** `caretBox` gives the old box wherever there
is one and cannot throw where the old read could not; a place banked before the mark's new
window fails its mark and falls back.

1. W8 met costs over its 5 ms that ruling 4 does not cover. For the owner: put with Linux's
   figure once the walk up the tree is measured again (ruling 4 now says so).
2. Ruling 4 said "up to about 40 ms" where the table has medians to 37 ms and 322 ms once.
3. The sentences on the JSON splice that the walk disproved, marked where they stand
   (*Current behaviour*, *The place*, W6, *Limits*, *Chosen without asking*), and one in a
   comment of `placeY`. The code bears the walk out; two rare exceptions are now named.
4. What the fix does to a hit test in white space was said two ways: only a caret that is
   in no text keeps its zeros. Said so here and in `zoomAnchor`'s comment; W2 is walked again
   on macOS.
5. *To walk again* did not fit Linux, whose wraps need a scan of their own.

With them, a `fixup!` for the three comments, `caretBox`'s among them, which now says a
caret at its text's start takes the first character's box.

**Pass 2 (`e06c7f8`, `1ef1810`), three findings in the plan, the code clean.** macOS was not
asked to measure the fully loaded JSON again, though ruling 4 rests on its figures. The note
on what was built differently left `caretBox` out. And the two platforms' "fully loaded
JSON" were two documents, which the plan did not say. Counting back through the siblings
gives the index the list would, and `placeY`'s one indexed read is no risk of the same cost
(read).

**Pass 3: no findings.**

## Re-walk results (2026-11-17, `walk/hidden-tab-place` at `5b83312` against 1.9.3)

### Linux

By script; gates pass (223 tests). The first run was stopped by Claude Code, the VM short of
memory as the 10 MB JSON loaded; resumed on the owner's word **without that file**, which is
not loaded on that VM again. The owner's two files have the same sha256 before and after.

- **WebKitGTK has the caret at a wrap too.** Of the README's 38 454 offsets in a 1100 px
  window, those with a character before and a space after and no box: 252 at 150%, 544 at
  300%, 169 at 80% (329, 771 and 235 with the sidebar open), close to the Mac's counts, each
  with no rectangle at all. The first walk had not met one.
- **At the wrap places, the fix holds, and 1.9.3 jumps on screen here as well.** Twenty per
  level and one more, each with a box at 100% and none at the level gone to:

  | | Build | 1.9.3 |
  |---|---|---|
  | A tab that was away | 0.98 px at most, all 61 | up to 41 538 px |
  | The change on screen in one go | within 2 px | up to 74 485 px |
  | A key's steps across a wrap | no jump | one of 16 409 px |

  One place read −37 px on both builds: its line is at the document's top, where the page
  cannot scroll back further. Three places were left out, the driver's own point having
  missed a caret with no box at 100%. A key's steps to 300% end about a line off (68 to
  142 px) on both builds: a rung at a time, a line that has shrunk hands over to the next.
- **W1, W2, W4 once more:** 0.94 px at most on text (1.9.3: up to 21 575 px); 0.78 px
  without text, all nine table places on the text's branch as before; the level unchanged as
  1.9.3 to the px, `placeY` uncalled.
- **The mark:** none of 138 text places is the length alone, where 25 of 133 were.
- **The cost.** The README 1 ms, thirty times over 2 ms, the JSON's first chunk 22 ms on its
  first call and 1 ms after; the walk up the tree 1 ms at most on all three. **The fully
  loaded JSON was not measured again.** In its place, a made-up list of 350 001 children:
  the old `indexOf` 10 to 13 ms on its first call, the count back through the siblings 1 to
  3 ms, one indexed read (as `placeY` makes) 2 to 15 ms. **That list did not bring back the
  0.5 to 6 s of the real file**, so the count is shown to be no slower and is not shown to
  cure it; what took the seconds there is not known. On Linux a switch away from a fully
  loaded 10 MB JSON stands at about 0.6 s more than 1.9.3, measured before the change, and
  unmeasured after it.
- No error in the console; nothing else differed from 1.9.3.

### Windows

By script; gates pass (218 tests). The JSON was never loaded past its first chunk and the VM
kept 3.9 GB free. The owner's two files restored byte for byte.

- **Nothing moved.** W1 in the 1100 px window with the sidebar open: each of the 80 figures
  equals the first walk's to 0.01 px (0.5 px at most on text, 1 px at most from the change on
  screen; the three at the page's end as before). W4: as 1.9.3 to the px in all 40, `placeY`
  uncalled.
- **`caretBox`'s new branch never ran:** none of its 1 478 calls over these runs met a box
  of all zeros on a text. A scan of the README at 150% does find such carets on WebView2
  too, where the plan expected none: 72 in shown text, at the very end of a text whose line
  ends there and at both offsets of the newline in a code block's line, besides 1 526 in
  white space that is not drawn. The same on 1.9.3. The app's hit test at `viewTop()` did
  not land on one; if it does, `caretBox` takes the character before it, on that line
  (reasoned, not walked).
- **The 47 ms reading on the JSON's first chunk is the first hit test after the document is
  first shown**, not the walk up the tree: 43.9 ms on the first run at the first place, then
  0.2 to 1.1 ms; at later fresh places 2.4 ms at most, the hit test 0.3 ms and the walk
  0.7 ms at most. (That one run could not be split; reasoned from the later ones.) The
  README thirty times over: 2.8 ms at most. The renderer's memory as on 1.9.3 (273 and
  274 MB; 661 and 635 MB).
- No error in the console; nothing else differed from 1.9.3.

### macOS

By script; gates pass (218 tests); three builds side by side (`5b83312`, `cc32f71`, 1.9.3). The
owner's two files byte for byte the same.

- **At the wrap places the fix holds**, and 1.9.3's fault is wider than the first walk saw.
  Fifty places (20 at 150%, 20 at 300%, 10 at 80%), each checked to be the caret the hit
  test returns at `viewTop()` and to have no box at the level gone to:

  | | Build | 1.9.3 |
  |---|---|---|
  | The change on screen in one go | 1 px at most | 306 to 72 570 px |
  | A tab that was away | 1 px at most | 34 to 42 364 px |
  | A key's step, a rung (18 places) | 1 px at most | 207 to 24 339 px |
  | A live pinch across a wrap (15) | 1.0 px between frames; ends within 2.0 px | 5 407 px between frames |
  | A scaled pinch's end, README thirty times over (9) | 1.5 px | 28 589 to 391 586 px |

  The first walk's three places: 0.9, 0 and 0.6 px, on screen and away.
- **W1 once more, whole:** 1 px at most by the keys, by a pinch and on screen, no place left
  out for a wrap. **W2:** 1 px at most; a cell's white space took the text's branch in 15 of
  16, a picture the element's, beside a heading either. **W4:** as 1.9.3 to the px.
- **Past a short last line's end the fix changes nothing:** the hit test there gives the
  text's last caret with a real box, on 1.9.3 too (20 of 20), and both hold the line within
  0.8 px. What changes is past a *wrapped* line's end. *The caret at a wrap* said "a short
  line" where it meant that.
- **The cost, on the Mac's own fully loaded 10 MB JSON** (19 chunks, 1.12 million elements):

  | | `rememberScroll(true)`, median at 10/50/95% | Its first call | A real switch away |
  |---|---|---|---|
  | `5b83312` | 5 / 22 / 41 ms | 372 / 97 / 107 ms | 1.23 to 1.92 s |
  | `cc32f71` | 4 / 20 / 36 ms | 298 / 79 / 53 ms | 1.17 to 1.71 s |
  | 1.9.3 | 0 | 0 | 0.75 to 1.38 s |

  The hit test is 3 to 27 ms of it and the walk up the tree 1 to 12 ms, by the siblings and
  by the list alike: counting back through the siblings neither helps nor hurts here. A
  first hit test after loading is slow on 1.9.3 too (338 ms). A switch away is 0.3 to 0.55 s
  longer than 1.9.3's, with a spread of about 0.3 s from run to run.
- No error in the console on any build; no hold stuck.

**Ruling 5** (the owner, 2026-11-17, ruling 4 asked again with these figures): **the cost is
accepted.** On a 10 MB JSON loaded to its end, leaving the tab takes a third to half a
second longer on macOS, and took about 0.6 s longer on Linux before the walk up the tree was
changed and is unmeasured there since. Under 3 ms on the README and thirty times over; on
the JSON as first opened, 22 to 44 ms once, the first hit test after it is shown, and under
1 ms after. Turned down: banking no place above a size, which costs nothing there and
brings such a tab back by the scaled offset, 15 000 px or more off.

## Triage (2026-11-17, with the owner)

Decided as the work went, each asked at once: rulings 1 to 5.

Thirteen plain limits, put to the owner as one list and taken as proposed. In
`open-items.md` under *Accepted limits*, each with its trigger: a file that changed while its
tab was away; a banked place that can pass for another in two rare shapes of page; two rare
JSON pages after *more*; a key's steps to a high level ending about a line off; a tab
switched to in the middle of a scaled pinch drawn once at the old scale; a tab torn off
landing 33 px off; a reopened tab once 26 px off; carets with no box on Windows, unwalked;
the cost of leaving a fully loaded 10 MB JSON; the first leave after a large JSON is first
shown; and, as a note on the limit *A session offered on the empty panel*, that the tab on
screen when the app quits has no place. In `closed-items.md`, kept with no trigger: the
page's ends when zooming out, and a copy of 1.9.3 or older keeping a place it does not know.

With them: the deferred item *A tab not on screen loses its place after a change of zoom* is
closed; the fault the walk found in 1.9.1 to 1.9.3, a change of zoom throwing the page where
a line wraps at the caret held, is recorded as fixed under *Bugs*; and the limit *A change
of zoom just as a document is shown* trades its note on a save under a pinch for the walk's
figures, that case now mended.

## Status at the end

Built, walked on three platforms and walked again after two fixes, triaged. Two commits for
the release notes, by the owner's ruling at packaging: the fault at a wrap on its own, *On
macOS and Linux, zooming no longer throws the page where a line wraps*, then *A background
tab keeps its place when the zoom changes*. The first holds `caretBox` and its three uses
in `zoomAnchor` and was never run as a build of its own; the two together are the tree that
was walked. The sections above stand in the order things happened: *The change* is the
first design, and *Rulings*, *The caret at a wrap* and the two walks say what was built in
the end.
