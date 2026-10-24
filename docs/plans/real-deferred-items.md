# Deferred Items That Aren't Edge Cases Plan

> **Status (2026-10-24):** final; the review loop is clean and every decision but Q-D1 is ruled or
> settled. Q-D1 comes after D1's measurements. Not approved; nothing executed.

**Spec:** the owner asked for a plan covering the deferred entries in `docs/open-items.md` that are
not edge cases: ones that happen in ordinary use, are measured rather than reasoned, or aren't bugs
at all. There are eight entries, in six parts. Each part stands alone: its own commit, walk and
records, in any order the owner picks.

| Part | Entries in `open-items.md` → Deferred | Kind |
|---|---|---|
| A | "A macOS tab tear-off lands 32 pt high", "A torn-off window can land off-screen", "A tab released level with the strip but outside the window reorders it" | bug, measured |
| B | "A diagram's links and actor menus lack full keyboard access" | accessibility gap |
| C | "In a narrow window a wide diagram shrinks until its labels can't be read" | bug, measured |
| D | "The JSON viewer's memory and layout cost per chunk" | performance, measured |
| E | "A size for the document font" | feature request |
| F | "Older commit messages carry Claude trailers and session links" | housekeeping, no code |

Part A's three entries share the trigger "the tear-off placement work", which this plan is. The
X11 un-maximize entry also names that work as a trigger, but its cause is session restore under
Mutter, not tear-off placement, so it stays deferred.

E and F aren't bugs. They were here as decisions, and the owner kept both deferred on 2026-10-24
(Q-E1, Q-F1). Their only work is a line in each open item recording that ruling.

**Architecture:** A touches `src-tauri/src/main.rs` and `src/app.js`. B and C touch `src/app.js`
and `src/base.css`. D starts as measurement only. No new dependency anywhere.

## Part A: tear-off placement

### What the user sees

1. **On the Mac,** every torn-off window opens 32 pt higher than on Windows and Linux. Measured
   2026-10-07 on macOS 26.7.1: a release at 200,140 pt opened the frame at 60,84 instead of the
   60,116 the code intends.
2. **On Windows and Linux X11,** a tab dropped near the bottom or right edge opens a window that
   runs off the screen. Measured on Windows: a drop at y=1100 on a 1440-high screen put the
   window's bottom at 1975. A drop within 24 px of the top would put the title bar above the
   screen; that is reasoned, not tested.
3. **On every platform,** releasing a tab at the strip's height but to the side of the window
   reorders it to the end of the strip instead of tearing it off. Seen on the Mac.

### Code today

- `window_origin` (`main.rs:1095`) returns `inner_position()` and the scale. On macOS, tao 0.37.1
  computes that from `contentRectForFrameRect`, which should exclude the title bar. The 2026-10-07
  walk says it returns the frame's top instead. Why is unknown.
- `screenPoint` (`app.js:4298`) adds `clientX/Y × scale` to that origin. On macOS the origin feeds
  only the tear-off, since `crossWindowDrag` is Windows-only and no `drag_over` probe runs there.
- `drop_tab` (`main.rs:1153`) passes `Placement::Cursor(x − 140·scale, y − 24·scale)`.
  `spawn_window` applies it with `set_position` after `build()`, with no clamp.
- `onDragMove` (`app.js:4450`) decides `inStrip` from `clientY` alone, within `STRIP_SLACK`
  (24 px) of the strip.

### A1. Find the macOS offset before fixing it

On the Mac, with drive-app, in a window at a known frame:

- Log `inner_position`, `outer_position`, `inner_size` and `outer_size` from a debug command or a
  temporary `eprintln!`.
- Log `window.screenY`, `window.screenTop`, and a real pointer's `clientY` against its screen
  point (`macpt.sh`).

That says which of these is true:

- **`inner_position` returns the frame top.** Fix in `window_origin` on macOS only: use
  `outer_position.y + (outer_size.height − inner_size.height)`.
- **`inner_position` is right, and `set_position` on macOS places something else 32 pt off.**
  Then the fix moves to `spawn_window`'s `Placement::Cursor` branch.

Either fix is a few lines under `#[cfg(target_os = "macos")]`. Which one is unknown until A1 runs.
If A1 shows a third cause, the plan comes back to the owner with it.

### A2. Clamp a torn-off window to the screen

- `drop_tab` looks up the work area of the monitor under the cursor with
  `app.monitor_from_point(x, y)` and `Monitor::work_area()`. Both exist in tauri 2.12.2.
- `Placement::Cursor` gains that work area as an `Option`. None, when no monitor is found, means
  no clamp. On Wayland the clamp may run, but `set_position` is ignored there anyway.
- `spawn_window` reads the built window's `outer_size()` and moves the target top-left inside the
  work area before `set_position`. The clamp is a pure function beside `Placement`, with unit
  tests: inside, past each edge, and larger than the work area.
- Each axis is clamped on its own. On an axis where the window is larger than the work area, it
  goes to the work area's start: the left edge, or the top (Q-A2). The other axis still follows
  the pointer.
- macOS clamps on its own. The clamp runs there too, and does nothing when the point is already
  inside.

Known side effect, reasoned: on Windows 10 and 11, `outer_size` includes the invisible resize
border of about 7 px a side. A clamped window then stops about 7 px short of the right and bottom
edges. Not measured.

Unknown: whether `outer_size` on Linux X11 includes Mutter's frame, and whether GTK reports the
real size for a window not yet shown. The walk measures both. If either is wrong, the clamp uses
1100×860 at the target monitor's scale instead.

Unknown, and not walkable here: two monitors at different scaling. The window is built on one
monitor and moved to another, and Windows may then resize it to the new monitor's scale after the
clamp read its size. The clamp could then be off by that difference. No two-monitor setup is
available for walks, as the accepted limit "Un-maximize across two monitors is unwalked" already
records.

### A3. Tear off a tab released beside the window

`inStrip` also requires `clientX` within `−STRIP_SLACK` to `innerWidth + STRIP_SLACK`. That is
one condition added to `app.js:4450`.

Whether the current behaviour was a deliberate forgiving reorder is unknown. Settled as a tear-off
(Q-A3).

### Part A walk

On the Windows VM, the Linux VM (X11 and Wayland), and the Mac, each with a real OS drag:

- Tear off at mid-screen. The pointer lands at the same spot on the new window's strip on all
  three. On the Mac that means within 1 pt of Windows' 140,24.
- Tear off near the bottom-right corner. The whole window is on screen, or on Windows within the
  invisible border.
- Tear off within 10 px of the top. The title bar is on screen.
- Release at the strip's height, 100 px left and right of the window. The tab tears off.
- Release at the strip's height inside the window. The tab still reorders.
- On Wayland, tear-off still works. Placement there is the compositor's.

### Part A commit

`Torn-off tabs open at the pointer and fully on screen` (A1 + A2 + A3), or one commit each if the
owner prefers.

## Part B: keyboard access to a diagram's links and actor menus

### What the user sees

- A keyboard user Tabbing through a flowchart's links loses focus whenever the light/dark look
  changes. `redrawDiagrams` (`app.js:1285`) sets the figure's `innerHTML`, which removes the
  focused link. Focus falls to `body`.
- A sequence diagram's actor menus open only by mouse (`onLinkClick`, `app.js:4858`). No key
  opens one or moves through its links.
- A redraw closes an open actor menu, because the new SVG draws it hidden.
- A keyboard-focused diagram that fails to draw in the new look turns back into its code block,
  and focus is lost.

### B0. Measure link focus on each engine first

Unknown, and it decides whether B3's menus step by Tab or by arrow keys: whether an SVG `<a>`
inside a diagram is a Tab stop on each engine. WKWebView is believed to skip links on Tab unless
the app turns that on, as Safari does by default. WebKitGTK is unknown. WebView2 is believed to
include them. None of this is measured.

No example document has diagram links, so the walks use one written for them and not committed:

- a flowchart with two `click` links;
- a sequence diagram whose actors have `link` menus;
- the same sequence diagram made taller than the window.

On each platform, Tab from the flowchart's figure and record where focus goes.

### B1. Keep a focused link across a redraw

In `redrawDiagrams`, before `fig.innerHTML = done.svg`: if the figure contains
`document.activeElement`, note its index among the figure's `a` elements. After the swap, focus
the same index with `preventScroll`. About eight lines.

### B2. Keep focus when a redraw fails

When `fig.replaceWith(drawn.pre)` runs on a figure that holds focus, give the `pre`
`tabindex="-1"` and focus it. A screen reader then reads the code and the error under it.
The `-1` keeps it out of the Tab order. Settled (Q-B2).

The way back needs the same care. A later redraw that draws it again runs `renderDiagrams`, which
replaces the `pre` with a new figure. If the `pre` held focus, `renderDiagrams` focuses the new
figure, or focus drops to `body` again.

### B3. Open actor menus from the keyboard

Options (Q-B3; the owner ruled option 1 on 2026-10-24):

1. **Focusable actors.** Each actor that has a menu gets `tabindex="0"`, `role="button"`,
   `aria-haspopup="menu"` and `aria-expanded` on its top box. Enter or Space toggles the menu,
   handled in `onKeydown` before the diagram's own Enter. It doesn't go through `el.click()`.
   `onLinkClick` places a menu from the click's coordinates, and a synthetic click reports
   0,0. That leaves the menu in place, except where mermaid's spot would run past the drawing's
   bottom, when the flip moves it up off the drawing (reasoned from the code). A key-opened menu
   stays where mermaid hangs it, under the top box. The menu's links take Tab, or arrow keys
   where B0 finds Tab skips them. Escape closes the menu and returns focus to the actor. A redraw reopens the menu that was
   open and refocuses its actor, found by name as `actorMenu` already does. `openDiagram` sets
   the copy's actors to `tabindex="-1"`, as it already does for the copy's links, so the
   full-window view gains no Tab stops. Cost: about 60–80 lines in `app.js`. Each actor with a
   menu becomes a Tab stop.
2. **A plain list of the actor links under the diagram.** Every menu's links, grouped by actor.
   Works the same on every engine and for screen readers, with no focus juggling. Cost: about 30
   lines. It changes how the page looks under each sequence diagram that has menus.
3. **Leave the menus mouse-only** and do B1 and B2 alone.

### Part B walk

On all three platforms, keyboard only, in B0's walk document: Tab to a flowchart link, switch the OS theme, and focus
stays on that link. Open an actor menu by key, step through its links, Escape, and focus is back
on the actor. Do the same on a diagram taller than the window: the menu opens under the top box.
Break a diagram for one look and switch to it: focus is on the code block. Switch back: focus is
on the redrawn diagram.

### Part B commit

`Diagram links and actor menus work from the keyboard`

## Part C: wide diagrams in a narrow window

### What the user sees

At about 700 px, a 10-node left-to-right flowchart of natural width about 2,240 px is shrunk to
the column, about 610 px. That is a scale of about 0.27, and its labels can't be read. Measured on
all three engines on 2026-10-11. The full-window view shows it at size.

### Code today

- `warmDiagrams` (`app.js:1111`) sets the SVG's `width` to its natural width and
  `max-width: 100%`.
- `base.css:1721` gives the figure `width: max-content` and a `max-width` of the pane less its
  padding.

### C1. A smallest scale, past which the diagram scrolls

- In `warmDiagrams`, beside the `width` it already sets: `root.style.minWidth =
  \`${natural * FLOOR}px\``. `min-width` beats `max-width`, so the drawing stops shrinking at the
  floor.
- In `base.css`, `.markdown-body div.mermaid-diagram { overflow-x: auto; }`. The figure keeps its
  pane-wide box, and the drawing scrolls inside it.
- A diagram already smaller than the column is unchanged: its floor is below its natural width.
- `openDiagram` (`app.js:1396`) clears the copy's `minWidth` beside the `maxWidth` it already
  clears. Otherwise the floor would stop the full-window view's Fit from shrinking a wide diagram
  to the window.

`FLOOR` is 0.6 (Q-C1). Mermaid draws labels at 16 px, so at 0.6 they read at about 10 px.

Unknowns the walk checks:

- Whether a click on the figure's scrollbar opens the overlay. Chromium and WebKit don't send a
  `click` for a scrollbar press, as far as is known; not measured here. If one does, the fix is in
  `onLinkClick`: ignore a click on a figure below its `clientHeight`, where the scrollbar sits.
- The focus ring. An `outline` on a scrolling box still draws outside it.

The cache key holds the look, not the width, so cached diagrams need no change: the floor rides
on the SVG string.

### Part C walk

At 700 px on all three, in `examples/widths.md`: the "Wide" 10-node flowchart shows labels at the floor and scrolls sideways in
its box. A small diagram looks as before. A click on the drawing opens the overlay; a press on its
scrollbar doesn't. In the overlay, Fit still fits the whole diagram to the window. Find skips
diagrams (`buildFindIndex` rejects `svg`), so it needs no check.

### Part C commit

`Wide diagrams in a narrow window scroll instead of shrinking past reading`

## Part D: the JSON viewer's memory per chunk

### What the user sees

Expanding a large JSON file's chunks costs 400–500 MB of webview memory each on Windows. 17 chunks
of a 9.75 MB file reached 9.7 GB on an 8 GB VM, which then thrashed. A fold takes 2.1–2.4 s at
409,000 text nodes on Linux, and 6–7 s with the file fully loaded on the Mac.

### Why this part starts with measurement

The cost per byte is the puzzle. A 512 KB chunk is about 1,000 bytes of memory per source byte.
DOM nodes cost hundreds of bytes each, so the token spans alone should come to tens of MB, not
hundreds. That is reasoned from typical node sizes, not measured. Something other than the node
count likely dominates. Candidates:

- **The fold `<button>`s.** Each `{` and `[` with a body gets one (`json.rs:92`). Buttons may cost
  far more than spans in layout.
- **The `::before` markers** on each fold (`base.css:2014`).
- **One huge `code` block's line layout,** unaffected by node count.
- **Something outside the chunk,** such as find's index or the highlighter, though find closed
  measured the same.

### D1. Measure

On the Windows VM, with drive-app's CDP access, on a 9.75 MB JSON file. The find walks' file isn't
named in any record. If it's still on the VM, it's reused. Otherwise a script makes one of the
same size, pretty-printed records, and this plan records how. The runs stop at 4 chunks, about
2 GB by the earlier numbers, well inside the VM's 8 GB.

1. Baseline: process memory, plus a CDP heap snapshot's DOM and JS sizes, after 1, 2 and 4
   chunks.
2. The same, with temporary changes, one at a time:
   - fold buttons emitted as spans;
   - `.fold::before` removed;
   - punctuation emitted as bare text, not spans.

   No `content-visibility` variant: a chunk is spliced inline where its button was, often
   mid-line after a comma (`onJsonClick`), so it has no block of its own to put that on.
3. Record each variant's memory and fold time in this plan.

D1 changes nothing in the product; the variants are reverted. It ends with the numbers and fix
options back to the owner (Q-D1). The fix is then planned here as D2, and the plan goes through
review again.

## Part E: a size for the document font (decision only)

**What it is.** Settings › *Document font* picks the family only. Libron's x-height is 0.53 of
its size, against Tufte's Palatino at 0.46, so Tufte reads about 15% larger in Libron. All three
walkers found it readable, and the owner kept it on 2026-10-09 (ruling F3 of the archived
document-font plan). No user has asked.

**Options (Q-E1):**

1. **Leave it deferred.** No cost.
2. **Automatic: `font-size-adjust`** on `#content`, set to the theme font's own x-height ratio,
   whenever a document font is chosen. Every theme then keeps its visual size in any font. No new
   setting. About 10 lines in `applyDocFont`. Unknown: support in WebKitGTK. WebView2 is believed
   to support it from Chromium 127, and Safari from 17, so the macOS 13 floor with an old Safari
   may not. Where it's unsupported, nothing changes.
3. **A size setting in Settings.** A new config field broadcast like the font, a control, and the
   reading width follows since it is in `ch`. About 60–100 lines across Rust and JS.

## Part F: old commit trailers (decision only, no code)

**What it is.** 279 old commits end in `Co-Authored-By: Claude`, and 13 in a `Claude-Session:`
link. The rewrite plan was shelved on 2026-10-13: GitHub keeps `refs/pull/N/head` for every pull
request, which keeps 244 of the 284 old commits fetchable with their original messages. A rewrite
can't remove them from GitHub. Only GitHub Support can, by deleting PRs #3–#11 and running garbage
collection. New commits are already clean.

**Options (Q-F1):**

1. **Leave it deferred.** No cost. The trailers stay on GitHub.
2. **The owner files a Support request.** Claude drafts the text. Deleting the PRs loses their
   discussion pages for good. If Support agrees, the shelved plan resumes at step 5, a force-push
   of a rewritten `main`. That changes every commit hash and breaks any clone or link to an old
   hash.

## Decisions

| Id | Question | Recommendation |
|---|---|---|
| Q-A2 | A torn-off window bigger than the work area: pin it to the work area's start on that axis, or also shrink it to fit? | **Ruled 2026-10-24 by the owner: pin only.** A shrink changes the window's size, which a tear-off never does today. |
| Q-A3 | A release level with the strip but beside the window: tear off (the fix), or keep the forgiving reorder? | **Settled 2026-10-24 as a clear call: tear off.** A release outside the window can't be aimed at a slot, and the reorder there always lands the tab at the end. |
| Q-B2 | Focus the code block when a focused diagram fails to redraw? | **Settled 2026-10-24 as a clear call: yes.** Focus otherwise drops to `body`, and the user loses their place. |
| Q-B3 | Actor menus from the keyboard: focusable actors, a link list, or mouse-only? | **Ruled 2026-10-24 by the owner: focusable actors (option 1).** B0 still runs first and sets Tab vs arrow keys inside the menu. It keeps one way to reach the menus for mouse and keyboard. |
| Q-C1 | The smallest scale for a diagram in the page. | **Settled 2026-10-24 as a clear call: 0.6,** so labels read at about 10 px. One constant; the walk confirms it. |
| Q-D1 | Asked after D1, with its numbers. | — |
| Q-E1 | Font size: leave, automatic `font-size-adjust`, or a setting? | **Ruled 2026-10-24 by the owner: leave it deferred.** No one has asked, and all three walkers found it readable. |
| Q-F1 | Trailers: leave, or file a Support request? | **Ruled 2026-10-24 by the owner: leave it deferred.** The cost is lost PR pages and new hashes for every old commit. |

## Execution

On the owner's go, per part. This plan and the E and F ruling lines in `open-items.md` are
committed first, as one `docs:` commit. A `coder` subagent (Opus) implements A, B and C, each committed on
`main` after `cargo fmt`, `cargo test` and `node --check src/app.js`. The main session runs A1, B0
and D1 itself, and the walks: Windows on the VM, Linux and the Mac through their Remote Control
sessions.

## Records

When a part closes: tick its entries and move them with their notes to `docs/closed-items.md` →
Deferred. An entry left open by a decision gets that decision's date and wording added. When
every part has closed or been ruled, this plan moves to `docs/archive/plans/` with a status line.
