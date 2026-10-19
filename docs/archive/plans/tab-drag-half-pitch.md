# Tab Drag Half-Pitch Plan

> **Status (2026-10-19):** executed on the owner's go (1b2fd3c); walked on all three, every check
> passing (the Mac within 1 px of the rule); change review clean on its first pass; nothing to
> triage. The README's "Dragging tabs" gives no distances, so it stays.

**Spec:** the open item "A short tab drag moves the tab one slot fewer than its distance"
(`docs/open-items.md` → Deferred), which the 2026-10-19 probes measured on all three platforms.
The owner chose the fix "swap at half a tab" (2026-10-19): a dragged tab swaps with its neighbour
once its leading edge passes the neighbour's middle. The tab still doesn't slide with the pointer.

**Architecture:** frontend only (`src/app.js`), plus the records. No Rust change, no new
dependency.

## What the user sees

Drag a tab sideways by about one tab's width: it doesn't move. Three widths: it moves three. And
it depends on the direction: rightward needs more travel than leftward.

## Measured (v1.8.6, real OS mouse, two runs each, identical)

| Platform, tabs | Pitch | +85 / +170 / +255 | −85 / −170 / −255 |
|---|---|---|---|
| Windows, 31 | 82 px | 0 / 1 / 3 | 1 / 2 / 3 |
| Mac, 31 | 82 px | 0 / 1 / 3 | 1 / 2 / 3 |
| Linux, 31 (2026-10-18) | 82 px | 0 / 1 / 3 | not measured |
| Windows, 8 | 87-148 px | 0 / 1 / 1 | 0 / 1 / 1 |
| Mac, 8 | ~101 px | 0 / 1 / 2 | 0 / 1 / 2 |

- Every slot change matched the rule below to the pixel on the Mac, and within one pointer step
  (≤10 px) on Windows. No pointer moves were lost.
- The press, on the label's middle, sits 9 px left of the tab's centre: the 18 px close button
  takes the right side.

## Current behaviour (main at 9b861b4)

- `onDragMove` (`app.js:4406-4450`): past the 5 px `DRAG_THRESHOLD`, within the strip, every move
  calls `reorderTo(insertionIndex(event.clientX))`.
- `insertionIndex` (`4330-4337`) returns the first tab, in DOM order and *counting the dragged
  tab*, whose midpoint is right of the pointer. `reorderTo` (`4339-4349`) subtracts 1 for a move
  to the right, splices `tabs`, and calls `renderTabs()`.
- So the tab swaps only when the *pointer* passes the next tab's middle: one full pitch from the
  dragged tab's own centre, ± the grab offset. Rightward 82 + 9 = 91 px, leftward 82 − 9 = 73 px.
- `insertionIndex` also places the caret and the drop for a tab hovering from **another window**
  (`5991`, `6000`). There the pointer is the only position there is, and the rule is right; it
  stays as is.

## The change

Track where the tab was grabbed, and swap on the dragged tab's *edges*, not the pointer:

- **The grab offset:** `beginDrag` looks up the dragged tab with
  ``els.tabs.querySelector(`.tab[data-id="${id}"]`)`` and stores `drag.grab = event.clientX −
  rect.left`. On the document-name path (`onDocNamePointerDown`) the strip is empty (`2170`), so
  there is no tab and `grab` is 0; harmless, as the in-strip path runs there only if a tab arrives
  mid-drag. No new argument.
- **`dragSlot(clientX)`**, used only at `onDragMove`'s in-strip call (`4430`) in place of
  `insertionIndex(event.clientX)`: probe `insertionIndex` with the dragged tab's leading edge
  instead of the pointer.

  ```js
  const el = els.tabs.querySelector(`.tab[data-id="${drag.id}"]`);
  // Closed mid-drag (Ctrl+W on the active tab): reorderTo finds no tab and returns.
  if (!el) return insertionIndex(clientX);
  const r = el.getBoundingClientRect();
  const left = clientX - drag.grab;
  return insertionIndex(left < r.left ? left : left + r.width);
  ```

  Moving right, the dragged tab's own midpoint is left of its new right edge, so it is counted and
  `reorderTo`'s −1 still applies; moving left, the result is at most its own slot; at rest it
  returns its own slot + 1, which `reorderTo` makes a no-op (review pass 1, checked).
- `reorderTo`, `insertionIndex` and the cross-window path: unchanged.

**The rule:** a tab swaps with its neighbour once the drag has travelled the gap plus half that
neighbour's width (`g + w_next/2`), and each further swap adds the centre-to-centre distance to
the next neighbour (`w_N/2 + g + w_M/2`). Tabs are 80 px minimum (`base.css:1265`) with a 2 px
gap (`1215`), and no tab state changes a tab's width (`1277-1332`). At 31 equal 80 px tabs:
thresholds 42, 124, 206, so +85 / +170 / +255 → 1 / 2 / 3; the same leftward; and the same
wherever on the tab the press lands, as the grab offset cancels out.

- **No flip-back:** right after a swap, the tab just passed sits where the dragged one was. The
  reverse swap needs travel under `w_next/2`; the forward one needed over `g + w_next/2`. So the
  margin against swapping back is the 2 px gap, for any widths: a pointer jittering ±2 px on a
  boundary can still swap back and forth, as a real move.
- **Ends:** the first tab dragged left or the last dragged right has no neighbour, and doesn't
  move; a pointer past the strip's ends counts every tab.
- **The strip scrolled:** all rects are client coordinates, re-read on every move, as today. The
  strip scrolls mid-drag only by the wheel (`5827-5839`); the next move re-slots against the new
  layout, as today. A reorder doesn't scroll it: the scroll to the active tab (`2211-2220`) fires
  only when the active tab changes, and a drag activates nothing until release.

## Verification

1. **Windows VM and the Mac, then Linux**, real OS mouse, 31 and 8 tabs, press on the label's
   middle and, once each, near the tab's left and right edges:
   - ±85, ±170, ±255 px: the slots match the rule, the same in both directions and for each grab
     point. With 31 tabs, the thresholds 42, 124, 206. With 8 (content-sized, 87-148 px), work
     out each expected threshold from the measured rects before the run.
   - A drag back and forth over one boundary: one swap each way.
   - The first tab dragged left and the last dragged right: no move. A drag past the strip's
     ends: the end slot.
   - 31 tabs with the strip scrolled to its middle: the same thresholds.
2. **Unchanged paths:**
   - A click, and a move under 5 px, still activates the tab.
   - Ctrl+W on the dragged active tab mid-drag: no error in the console; the drag ends at
     release.
   - Tear-off (drag off the strip) and back into the strip mid-drag.
   - Windows only (`crossWindowDrag`): drop onto another window's strip still lands where its
     caret showed.
   - A lost release (`buttons === 0`), `pointercancel` and lost capture still cancel back to
     `startIndex` (`onDragCancel`, `4525`).
   - The order still lands in `session.json`.

## Records

- Close the open item in `docs/closed-items.md` with a *Fixed* note and the measured rule.
- The README: no change unless it describes drag distances (check at execution).

## Release

A fix to shipped behaviour: one commit, no prefix, release-note summary such as "A dragged tab
swaps once it's halfway over its neighbour".
