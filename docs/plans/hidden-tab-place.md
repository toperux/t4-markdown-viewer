# A Hidden Tab Keeps Its Place Through a Zoom Change Plan

> **Status (2026-11-17):** written and reviewed (five passes: eight findings, then five,
> five, two and none, all taken; ruling 1). Nothing is built. It waits for the owner's go.

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
  JSON document's *more* splices its chunk in beside the text already there
  (`insertAdjacentHTML`, `5672-5673`), where a render at that `extent` gives one text node;
  and a diagram that failed on one showing and draws on the next has a paragraph more or
  fewer beside it (`sayDiagramFailed`, `1361`). A fold changes no node (`5638-5647`).

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
  length and 24 characters from the offset; an element's name, its count of children and
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
wrong place would be worse than that, and a JSON page that has had *more* pressed is one
below the splice.

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
src/app.js`. One local commit, a release-note line: *A tab keeps its place when the zoom
changes while it is in the background*.

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
  with the place below the splice; a diagram above the place that failed on one showing and
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
  its own; it is put to the owner with the figures either way.

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
  long that reads the same there; or, where there are no source lines to go by, an element
  of the same name and count of children. That is a JSON page, and what the page makes
  itself with no source lines above it: a diagram's figure and its drawing
  (`1384-1398`), a table's scroll box (`447-453`). For a wrong place there it takes a
  changed file and two diagrams, or two such boxes, as many blocks apart as were added
  above: next to each other, for one paragraph added.
- **A JSON document after *more***, below the splice, and **a diagram that changes between
  failing and drawing** above the place: the same fallback (W6). Not mended here: a render
  that gave the tree the page then grows into would be a change to the JSON viewer.
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
  changed file; or accept wrong places.
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
