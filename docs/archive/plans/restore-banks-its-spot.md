# Restore Banks Its Spot Plan

> **Status (2026-10-19):** executed on the owner's go (f6a05a3); walked on all three; change
> review clean on its first pass; triaged below.

**Spec:** the open item "A restored reading position can land a screen past its spot after a page
reload" (`docs/open-items.md` → Deferred), which the 2026-10-19 probe on the Windows VM reproduced
and traced. The owner chose the fix "bank the position when the hold ends" (2026-10-19).

**Architecture:** frontend only (`src/app.js`), plus the records. No Rust change, no new
dependency.

## What the user sees

Open an image-heavy document, read it without scrolling, quit. At the next launch it reopens up to
nine pictures (~9000 px) below where it was left.

## Measured (Windows VM, v1.8.6 = 9b861b4, the original `slow.md`: 30 PNGs, 1005 px each)

- Off by more than 5 px from the target 20000: 7 of 20 real launches, 9 of 30 driver reloads, 0 of
  10 Ctrl+Tab arrivals.
- Every off run restored *exactly* to what was saved; the saved value was wrong. 1.1 s before the
  quit, `scrollY` read 20000, but `session.json` held 21005, 22010 … 29045.
- 6 of 10 runs had the tab's saved entry (`shownEntry.scrollY`) differ from the live `scrollY` at
  6.5 s, long after the hold had ended (118-731 ms, exit at `app.js:1713`, every image complete).
- Ctrl+Tab can't show it: leaving a tab calls `rememberScroll()`, which reads the live value.
- The 909faa9 baseline (before the tab-strip work), same document: 1 of 10 off, at 16015, by a
  different path (the hold's scroll test at `app.js:1753` let go at 236 ms on a 2 px move with the
  height unchanged, and anchoring carried the page). Not seen in 30 reloads at v1.8.6; out of
  scope here.
- Linux and the Mac: not measured. The code path is the same (reasoned).

## Current behaviour (main at 9b861b4)

- **The scroll listener** (`app.js:5855-5861`) calls `rememberScroll()` and `scheduleReport()` on
  every scroll event. `rememberScroll` (`361-372`) writes the live `window.scrollY` into
  `shownEntry.scrollY`.
- **The hold** (`holdScroll`, `1687-1817`) puts the page back at `target` with `scrollTo` while
  the content grows, then lets go (`hold.abort()`): when every image has settled (`1713`, `1743`),
  at the reader's own scroll or input, at a find jump, when another render or entry is shown, or
  at the 60 s cap.
- **The gap, measured:** during the restore, scroll anchoring moves the page past the spot and a
  scroll event banks that passing value. The hold's last `scrollTo` puts the page back at the
  target, but no scroll event follows it, so the entry keeps the passing value. Why no event
  follows is not known (reasoned: the event for the move back is coalesced or never queued).
- **On quit**, the `quit-requested` listener (`6063`) sends `reportSession()` as it stands. The
  `update-installing` listener (`5984-5987`) calls `rememberScroll()` first; quit doesn't. The
  comment at `5846-5852` explains why the report itself doesn't bank: a render puts the page at
  the top before restoring, and a bank in that gap would write 0 over the saved spot.

## The change

In `holdScroll`, when the hold ends and its entry is still the one on screen, bank the live
position and schedule a report, as the scroll listener does (`5858-5859`), so the right value
reaches `session.json` even when no orderly quit follows (a kill, an OS shutdown, a Quit from
outside the app, `2301-2303`). Cost: one extra session write per restore.

```js
hold.signal.addEventListener("abort", () => {
  // A newer hold may have taken its place.
  if (scrollHold === hold) scrollHold = null;
  // Its last scrollTo may fire no scroll event, which would leave the entry on a
  // spot anchoring passed through on the way (measured, Windows: up to 9000 px).
  if (token === shownToken && shownEntry === entry) {
    rememberScroll();
    scheduleReport();
  }
});
```

And amend the scroll listener's comment (`5845-5852`), which the measurement disproved: "the
restore fires one of its own, so the last write is the correct one" isn't so, and the position
is now banked at a hold's end too, not "only on the way out of a tab".

Every way the hold ends, checked:

| Ends by | Live `scrollY` at that moment | Banked value |
|---|---|---|
| Images settled (`1713`, `1743`) | the target, just set by `scrollTo` | right |
| Reader's wheel, key, touch, scrollbar press | the spot before the reader's scroll; their own scroll events bank after | right |
| The scroll test (`1753`) | where the reader scrolled to | right |
| A find jump (`scrollHold.abort()`) | the spot before the jump; the jump's scroll event banks after | right |
| The 60 s cap | wherever the page is | right |
| Another render or entry shown | not banked (the guard): the page may be mid-render at 0, which `5846` warns about, or `go`/`pushAnchorEntry` moved `shownEntry` to a sibling whose spot the live `scrollY` isn't (`3189`, `3280`) | unchanged |

Checked in review pass 1: `token` and `entry` are `holdScroll`'s parameters, `shownToken` (`312`)
and `shownEntry` (`324`) module-level; `scrollTo` is instant (no `scroll-behavior` in the CSS), so
the live value at `1713`/`1743` is the target; a live reload or a switch mid-hold banks the old
page while it's still on screen, and the swap (`1598` → `2113`) is synchronous.

Not changed:
- `quit-requested` (`6063`) and the window's close (`onCloseRequested`, `6058`), the two last-report
  paths. Banking there could catch a hold still running, mid-restore. With the bank above, the
  entry is right once the hold ends, and a quit within the first second or so of a restore was
  never measured. (The comment at `6059-6060`, "the macOS Quit, the only thing that sends this",
  is stale: Ctrl+Q sends it too. Out of scope; noted.)
- **Leaving a tab mid-hold:** a tab switch, a refresh (a live reload or Ctrl+R), Back/Forward,
  an in-page link or a tab close during the hold bank the live value (`2065`, `2089`, `3177`,
  `3275`, `2350`), which can be a passing one. The hold lasts ~0.1-0.7 s on Windows (measured),
  and can run 2 s or more on Linux, where WebKitGTK holds back rendering while large images
  decode (`1717-1720`). Reasoned, not measured; this change doesn't make it worse.
  *Q1, answered by the owner 2026-10-19: out of scope; verification step 4 tries it, and if it
  shows, it comes back as its own item.*

## Verification (walks, report only, at the change's commit)

1. **Windows VM**, the rebuilt `slow.md`, target 20000:
   - 10 driver reloads and 10 real launches (scroll to 20000, wait 1.1 s, Ctrl+Q, relaunch):
     expect 0 of 20 off.
   - In each run, `shownEntry.scrollY === window.scrollY` at 6.5 s.
   - 10 Ctrl+Tab arrivals: still exact.
   - A reader's wheel during a restore still stops the hold (`1775-1787`), and that run's saved
     position is where the reader stopped.
2. **Linux VM and the Mac:** the same reloads and launches, at v1.8.6 first (does it reproduce
   there?) and then at the change.
3. A document with no pictures (no hold runs past the first frame): reopen exact.
4. Paths the guard and the other exits take (Windows VM, `slow.md`):
   - Ctrl+Tab away within ~200 ms of a reload, before the hold ends, and back: each tab keeps its
     own spot. Record where `slow.md` lands (Q1's case).
   - A find jump during a restore: the saved spot is the hit, not the target.
   - `slow.md` edited on disk during a restore (a live reload): where it lands. Report only.
5. **The scheduled report** (Windows VM; Linux only if step 2 reproduces the bug there at
   v1.8.6, else as a sanity check), 10 launches of `slow.md`: wait until the hold has been seen
   running (`scrollHold !== null`) and then ended (`scrollHold === null`); `scrollHold` is null
   before the hold starts too. Poll every animation frame (the hold can end 118 ms after it
   starts), and give up after ~10 s as "hold not seen". Then wait over 500 ms (`REPORT_DELAY`,
   `2272`) and read `session.json` directly: expect the target every time. Then kill the app
   (Task Manager or `kill -9`) and relaunch: lands on the target. Gate on the hold's end, not a
   fixed time: on Linux the hold can run 2 s or more.
   - One control run set, with only `rememberScroll()` in the abort listener: other reports
     (the `focus` and `resize` listeners, `6049-6050`) may already carry the corrected entry. If
     they do, the plan's note says `scheduleReport` is belt-and-braces, not needed.

## Records

- Close the open item in `docs/closed-items.md` with a *Fixed* note: the measured cause, and the
  correction that the "overshoot" was a stale saved value, not the hold overshooting.
- The 909faa9 early let-go (16015): put to the owner at change review as a separate item.

## Release

A fix to shipped behaviour (`squash-follow-ups-into-their-feature`): one commit, no prefix,
release-note summary such as "A document reopens where it was left, not up to a screen lower".

## Walks and triage (2026-10-19)

- Windows (`slow.md`): 0 of 10 reloads and 0 of 10 launches off (v1.8.6: 9 of 30, 7 of 20);
  the entry matched the page at 6.5 s in all; switches, a wheel mid-restore, a no-picture doc,
  a switch away mid-restore, a find jump and 10 force-kills all right.
- The Mac (0 of 40) and Linux, X11 and Wayland (0 of 80), never reproduced it, at v1.8.6 or the
  fix. On Wayland the hold ended 5-205 ms after the last scroll event.
- Triage, by the owner:
  1. `scheduleReport()` at the hold's end: **kept**. The control without it passed 10 of 10 on
     Windows, and Wayland's gap never reached the 500 ms report delay; it covers a slower hold.
  2. A live reload mid-restore landed 0-2 pictures low (Windows, 3 runs): **deferred**.
  3. A tab switched away mid-restore briefly held 938 (Windows, 1 of 3): **deferred with 2**.
  4. On Wayland a mid-restore report left a passing spot in `session.json` for ~1.5 s (11 of 20,
     v1.8.6 too): **deferred with 2** (`open-items.md`, "A page left or re-rendered while its
     position restores can bank a passing spot").
  5. The 909faa9 early let-go (16015): **noted** in the fixed item's record only.
