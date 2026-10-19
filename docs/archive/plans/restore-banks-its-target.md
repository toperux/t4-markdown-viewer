# Restore Banks Its Target Plan

> **Status (2026-10-19):** executed on the owner's go (e679e69, plus comment fixups from the
> change review); walked on all three; change review clean on its third pass; triaged below.

**Spec:** the open item "A page left or re-rendered while its position restores can bank a passing
spot" (`docs/open-items.md` → Deferred, triage 2-4 of `archive/plans/restore-banks-its-spot.md`),
which the owner reopened on 2026-10-19. Its recorded likely fix: while a hold runs for the entry
on screen, `rememberScroll` banks the hold's target rather than `scrollY`.

**Architecture:** frontend only (`src/app.js`), plus the records. No Rust change, no new
dependency.

## What the user sees

While a document restores its reading position (the hold: ~0.1-0.7 s on Windows, 2 s or more on
Linux Wayland), anything that saves the position saves where the page happens to be passing, not
where the reader was:

- a live reload (the file saved by another program) lands the page a picture or two low;
- a tab switched away and quit at once could reopen the document at that passing spot;
- on Linux, a crash or kill in a ~1.5 s gap reopens it there.

## Measured (2026-10-19 walks, at v1.8.7's code, a248520)

- Live reload mid-restore, Windows, `slow.md`, 3 runs: landed 20000, 21005, 22010.
- Tab switched away mid-restore, Windows, 1 of 3: `slow.md`'s entry read 938 for under 2 s after
  the switch, then 20000 again.
- Linux Wayland, 11 of 20 launches (v1.8.6 and v1.8.7): a report ~2 s in wrote 3252 or 6084 to
  `session.json`; the final report (~3.4-3.9 s) wrote 20000.

**The 938, reasoned (review pass 1):** after the swap nothing can write `slow.md`'s entry (every
bank goes through `shownEntry`, and the abort's bank is guarded, `1697`), so both the 938 and the
20000 were banked before the swap, with `slow.md` still on screen while the new document loaded
(`load_file` and `warmDiagrams`, `2081-2086`). Getting back to 20000 with no input takes a hold's
`scrollTo`, so the hold was live (unless the walk had switched back to `slow.md`, which re-renders
it; the confirming walk logs every bank). The 938 is a clamp: `scrollTo(20000)` on a page whose
pictures haven't sized yet lands at its end. So the new rule covers it; the walk confirms.

## Current behaviour (main at a248520, v1.8.7)

- `rememberScroll` (`app.js:361-372`) writes the live `window.scrollY` into `shownEntry.scrollY`.
  Its callers: the window `scroll` listener (`5882-5889`, the call at `5885`, with
  `scheduleReport`); `showActive` at the swap (`2072`, `2096`) and before an error (`2117`); `go`
  (`3184`), `pushAnchorEntry` (`3282`), `removeTab` (`2357`), the drag's end (`4483`),
  `update-installing` (`6012`); and the hold's own abort (`1691-1701`).
- `scrollHold` (`3353`) holds the running hold's `AbortController`; it is set in `holdScroll`
  (`1690`), cleared in its abort listener (`1693`), and aborted by a find jump (`3554`). Nothing
  else uses it.
- A live reload is `refresh` (`3311-3324`) → `showActive()` with no `scrollY`, so the page lands
  on `entry.scrollY` as banked at the swap (`2096`, passed at `2099`).

## The change

Keep the hold's target and entry beside its controller, and bank the target while that hold runs
for the entry on screen:

```js
let scrollHold = null; // the running holdScroll: { hold, target, entry }
```

- `holdScroll`: `scrollHold = { hold, target, entry }`; its abort listener clears it with
  `if (scrollHold?.hold === hold) scrollHold = null;`. Amend the comment at `1689` ("So a jump to
  a find match can let go of it"): `rememberScroll` reads it too.
- The find jump: `scrollHold?.hold.abort()`.
- `rememberScroll`:

  ```js
  // A restore still holding the page: where it's going, not where anchoring
  // has it on the way (a reload, a switch or a report mid-restore).
  else shownEntry.scrollY = scrollHold?.entry === shownEntry ? scrollHold.target : window.scrollY;
  ```

Every way a hold ends, checked against the new rule (review pass 1 traced each):

| Ends by | Order | Banked |
|---|---|---|
| Images settled (`1720`, `1750`) | abort clears `scrollHold`, then its bank reads the live spot | the target, or the page's end if clamped |
| Reader's wheel, key, touch, scrollbar press | the hold aborts on the input, before the page moves; later scroll events bank live | the reader's spot |
| The scroll test (`1760`) | the window listener may bank the target first; the abort then clears and banks live, synchronously, so either order ends on the live spot | the reader's spot |
| A find jump (`3554`) | aborted before `scrollBy`; the jump's event banks live | the hit |
| The 60 s cap (`1822`) | clear, then a live bank; the hold has kept the page on the target | ≈ the target, or the page's end if clamped |
| The observer's or poll's token/entry check (`1707`, `1729`) | clear; the abort's guard fails, so no bank | unchanged |
| Another render or entry shown | the old hold lives until its next tick (`shownToken` changes only at `2120`); until then a bank on its entry gives its target | target |

The last row is the fix: a live reload or a switch mid-restore banks at the swap (`2072`/`2096`)
while the old hold is still alive, so the target, not the passing spot. A mid-hold report sends
entries whose `scrollY` is the target. `removeTab`, the drag's end and `update-installing`
mid-restore also carry the target now (an improvement).

Other cases, traced in review pass 1:

- A reader scroll the hold doesn't stop on its input (middle-click autoscroll, keys its keydown
  filter skips, a focus scroll): it trips the height test (abort, live bank), or the hold pulls
  the page back to the target on its next tick, so banking the target matches what the page
  will show.
- A live reload's new render restores to `entry.scrollY`, which the swap just banked as the old
  target, so the old and new holds agree.
- A target past the page's end (a reload that shortened the file): banks read the target while
  the page sits at its end, until the hold's own bank writes the end. A quit in between reopens
  to the same clamp. Harmless.

**Q1 (for the owner): a gap on Linux Wayland, reasoned only.** WebKitGTK holds back
`requestAnimationFrame` and the observer ~2 s while large pictures decode, but timers run. After
any render's swap (a live reload, or a switch onto a tab whose spot is restoring), the new hold
waits for the held frame, and no hold matches the incoming entry: after a reload the old hold's
100 ms poll sees the new token and aborts (no bank); after a switch the running hold, if any, is
the old tab's. In that gap a scroll event (layout clamping the page to the new, still short
content, say) banks the live spot on the incoming entry, and its report writes it 500 ms later.
The swap's own `reportSoon` (`2149`) runs before that and carries the target. Windows is
unaffected (reasoned): scroll events dispatch before frame callbacks, and the frame's `scrollTo`
then banks the target. Options: walk it first and plan a follow-up only if it shows; or close it
now by having the new render set `scrollHold`'s target before its frame (a few more lines, the
hold's lifecycle split in two). Recommendation: walk it first (step 2 below).
*Q1, answered by the owner 2026-10-19: walk it first; a follow-up only if it shows.*

## Verification (walks, report only, at the change's commit)

1. **Windows VM**, `slow.md` at 20000:
   - A live reload mid-restore (append a line 2-3 ms after the hold is seen), 10 runs: lands
     20000 each (v1.8.7: 1-2 pictures low in 2 of 3).
   - Ctrl+Tab away mid-restore and back, 10 runs, logging each bank of `slow.md`'s entry with
     `scrollHold` and the live `scrollY` (a temporary log in `rememberScroll` on the walk build,
     not committed): the entry never leaves 20000.
   - Regression: 10 reloads and 10 real launches exact; a wheel mid-restore saves where the wheel
     left the page; a find jump mid-restore saves the hit; a reader scroll the height test
     catches (a scripted `window.scrollBy` over CDP mid-restore with the height stable: it fires
     no input event, so it reaches `1760`) saves the reader's spot.
   - Optional: a reload that shortens `slow.md` below 20000 mid-restore: once the hold ends the
     entry reads the page's end.
2. **Linux VM, Wayland**:
   - 10 launches: every report during the hold carries 20000 (`session.json` read every 100 ms
     until the final report).
   - A live reload mid-restore, 10 runs, `session.json` read every 100 ms: does any report carry
     a spot other than 20000 (Q1's gap)?
   - Ctrl+Tab onto `slow.md` from a tab scrolled further down than `slow.md` can reach before its
     pictures load, 10 runs, `session.json` read every 100 ms: the same question.
3. **The Mac**: 10 launches and a live reload mid-restore: exact (regression only; it never
   reproduced the original bug).

## Records

- Move the open item to `docs/closed-items.md` with a *Fixed* note; Q1's outcome in that note, or
  as a new item if the gap shows and isn't fixed here.

## Release

A fix to shipped behaviour: one commit, no prefix, release-note summary such as "A document
saved by another program during its restore reloads in its place".

## Walks and triage (2026-10-19)

- Windows: a live reload mid-restore 10 of 10 at 20000; a switch away mid-restore 10 of 10, 78
  logged banks all 20000 while the page passed 938-29045 (the 938 was a hold's clamp, as
  reasoned); reloads, launches, a wheel, a find jump (80-picture doc), a height-test scroll
  (pictures paused by CDP) and a shortened file all right.
- Linux Wayland: 0 of 30 reports off (launches, live reloads, Ctrl+Tab onto a restoring tab);
  Q1's gap didn't show.
- The Mac: regressions only, all right; its hold lasts 10-50 ms, so nothing landed mid-hold.
- Triage, by the owner:
  1. The Mac banks 0 for 6-21 ms after a swap, never reported: **accepted** with a trigger
     (`open-items.md`).
  2. One Linux launch lost its WebDriver session (1 of 31): **noted** in the fixed record; an
     item if it recurs.
