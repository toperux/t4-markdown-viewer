# Session Restore Implementation Plan

> **Status (2026-10-08):** executed in full, in the order 1, 2, 2b, 4, 3.
> - **Commits:** Task 1 91a3a9b, Task 2 f68844c, Task 2b b0b1d7f, Task 4 a5f4e35, Task 3 77f73cd.
> - **Gates:** Windows 210 tests; Linux (VM) 211 at Task 3, before the review's added tests.
> - **Change review:** six passes, the last clean. Its findings and the walks' were fixed as
>   fixups onto their tasks:
>   - Task 1's hold now also stops at reader input (wheel, scroll key, touch, scrollbar press),
>     since the height check alone lost 6 of 8 wheel turns on WebView2 (owner, revising R3/R7);
>   - a wheel over the sidebar stops it when the tree can't scroll that way;
>   - a 100 ms height poll, because WebKitGTK on Wayland holds rAF and ResizeObserver for about
>     2 s while large images decode (measured); the frame-later retry alone didn't help;
>   - the input stops narrow R7's first edge (a reader scroll in the same frame as an image
>     landing) to an overlay-scrollbar drag.
> - **Walks:** every W1-W5 step passed on the Windows VM, the Linux VM (Wayland and X11) and the
>   Mac, except X11's un-maximize place. Second-monitor and gone-monitor steps were not possible
>   (one display everywhere).
> - **Walk results (2026-10-08):**
>   - Windows VM: W1 README and Back exact; the scroll probe's drift (1500 → 2297) gone; reader
>     input during a slow load kept in every trial (scripted and real wheel, PageDown, a
>     scrollbar drag, a wheel over the sidebar, arrows in the tree). W2 every quit order,
>     including real × clicks, the taskbar thumbnail's × and *Close all windows*, plus *Reopen*
>     and a file opened while restoring. W3 exact over two relaunches and through *Reopen*; a
>     1.7.3 file un-maximizes to the default size; a window maximized within ~1 s of opening
>     un-maximizes to the default size (deferred). W5 exact.
>   - Linux VM (GNOME 50): on Wayland, with the 100 ms poll, the position is back within 52 ms of
>     the page reaching full height; before the poll it landed ≈2 s late, only as the last image
>     loaded. The size stays 1100×860 through three cycles (was +52×89 each). W2 passes; W3 passes
>     on Wayland. On X11 un-maximizing keeps the size but lands at 67,32, not 300,200 (five
>     relaunches). X11 freezes the page on long or image-heavy documents, 5 of 5, on 1.7.3 too.
>   - Mac (real input): every W1-W3 and W5 step. Full screen comes back at its pre-full-screen size.
>     A window maximized (zoomed) within ~1 s of opening un-maximizes to the default size
>     (deferred). After *Reopen*, `session.json` listed the windows in the wrong order for ~0.3 s
>     and was right by 2.5 s.
> - **Triage (owner):**
>   - deferred: a window maximized right after it first opens; the X11 freeze on long documents
>     (it predates this plan, and is the next plan); the X11 un-maximize place;
>   - accepted, open: the order briefly wrong after *Reopen*; R4; R7's two edges; 200 % on
>     Wayland unwalked; multi-monitor un-maximize unwalked;
>   - accepted, closed: a move then maximize within 0.5 s; R10.
> - **Deviation:** the probes P1 and P3 moved from the owner's PC to the Windows VM midway, at the
>   owner's request.
>
> Paths and line numbers below are as they stood when it was written.

> **For agentic workers:** Execute per the **Execution** section below. Steps use checkbox
> (`- [ ]`) syntax for tracking.

**Goal:** A restored session puts every window back as it was left: the reading position in a
document with images, the last-used window in front, the size on GNOME, and the size and place
a maximized window un-maximizes to.

**Architecture:** Four changes on the restore path.
- The frontend re-applies a restored scroll while the page is still growing under it.
- Rust reserves the label of the last-used window before it builds any. Every other restored
  window hands the front to it, reusing the `behind` hand-off a held file already uses.
- `Frame` keeps a maximized or full-screen window's last normal rectangle and sets it before
  the maximize.
- On Linux the saved size is read from GTK (`gtk_window_get_size`, on the main thread), the
  counterpart of the `gtk_window_resize` that sets it back.

**Tech Stack:** Rust (`session.rs`, `main.rs`, `routing_tests.rs`), JavaScript (`app.js`), gtk
0.18 (already in the tree through tauri), `drive-app`.

**Spec:**
- open-items › Deferred:
  - *A document with images comes back lower than it was left*;
  - *On Windows, the last-used window doesn't come back in front*;
  - *On GNOME, a restored window grows each launch*.
- open-items › Accepted limits: *Un-maximizing a restored window loses where it was before the
  maximize*. The owner folded it in on 2026-10-07; its trigger was "if the session-restore plan
  touches frames".

**Owner rulings (2026-10-07):**
- R1. A window maximized on one monitor and moved, still maximized, to another comes back
  where it was left. The normal rectangle is kept only while it is on the monitor the window is
  maximized on. Otherwise un-maximizing gives the default size, as today.
- R2. macOS full screen is treated like minimized: the last normal rectangle stands. The window
  comes back at that size, not in full screen.
- R3. Task 1 tells the reader's scroll from the page growing by position: a scroll while the
  content height is unchanged is the reader's. It stops then, or once every image has settled,
  with a 60 s fallback cap.
- R4. Only the last-used window's place on top is restored. The other windows stack in the
  order they finish loading (a known limit, below).
- R5. On native Wayland R1's monitor check is skipped and the normal size is always kept. The
  app can't learn window positions there (they read about 0,0, `main.rs:1059`), so the check
  would drop the size of every window maximized on a second monitor.
- R6. "On the monitor" means the monitor the normal rectangle overlaps most, which is how
  Windows picks where to maximize. It measures the overlap with the comparisons
  `Frame::overlaps` already makes.
- R7. Two rare edge cases in Task 1 are accepted as known limits (below).
- R8. *Superseded by R11.* It left a boot-gap save as a rare race; it happens on every restore.
- R9. After P1 (the front window follows close order), keep Task 2 and add Task 2b: closed
  windows are written by a focus-order snapshot, not by close order.
- R10. Task 2b stays, knowing it helps only closes that don't activate the window (scripted,
  probably the taskbar's thumbnail × and *Close all windows*, the macOS red button on a
  background window). A real click on a back window's × activates it first, so that window
  comes back in front, as today; that is a known limit (below).
- R11. A window that isn't visible yet keeps its previous frame (Task 3), so the save every
  restore makes before its maximize no longer writes `maximized: false`.

**Prior art:** t4-git-ui keeps a maximized window's normal rectangle (`sampled`,
`../t4-git-ui/src-tauri/src/commands/window.rs:570-593`). Minimized or full screen keeps the
old rectangle; maximized keeps the old one, marked maximized; otherwise it takes the current
frame. It never writes a rectangle from a maximized sample. Task 3 follows that, plus R1.

## Global Constraints

- No new crate in `Cargo.lock`. gtk 0.18.2 is already a tauri dependency (`Cargo.lock:1737`). It
  becomes a direct Linux-only dependency; the lock gains only the edge and is committed (CI runs
  `--locked`).
- `session.json` written by 1.7.3 must still load. A maximized frame from it holds the maximized
  size, not a normal one, and must never become the window's normal size.
- No GTK call off the main thread, and no lock held across a window getter
  (`session.rs:294-297`).
- Commit straight to main, `cargo fmt` before each commit, LF line endings.
- Behaviour on platforms a task doesn't target must not change; walked, not assumed.

## Review Focus

1. **The reader scrolls while images are still loading.** Their scroll must stop the re-apply
   at once.
2. **Back to a document whose images are cached.** Nothing grows; the first `scrollTo` stands,
   with no second jump.
3. **The last-used window fails to build.** The others hand the front to a label that never
   appears, so each must raise itself as today. Reasoned safe: `getByLabel` returns null, and
   `setFocus` on a hidden window is a no-op on all three platforms (tao `windows/window.rs:
   139-150`, `linux/window.rs:582-591`, `macos/window.rs:675-683`).
4. **A 1.7.3 `session.json` with a maximized window.** It must come back maximized, and
   un-maximize to the default size, on the first relaunch and every later one.
5. **An update restart.** `install_update` snapshots from a tokio thread (`update.rs:180` →
   `session.rs:248` → `save` → `Frame::of`). The GTK read must not run there.

6. **A saved normal rectangle on a monitor that is gone.** `apply` skips it and the window gets
   default placement (`session.rs:126-128`), but the frame is still recorded in `state.frames`.
   Reasoned safe: R1 drops it once the window is maximized, and a sample of the window
   un-maximized replaces it.

## Known limits

- **R4: the windows behind the front one.** Each window that hands the front over is still
  raised by its own `show()`. So `focus_order` ends as boot-finish order, then the front window.
  The next save's order for the windows that aren't in front is their boot order, not their
  real history.
- **R7: Task 1's edge cases.** Both are reasoned from the browsers' event order, not measured.
  - A reader scroll in the same frame as an image landing isn't counted, so one more re-apply
    pulls the reader back once. Their next scroll stops it.
  - A window that grows taller while the saved spot is near the end of the document clamps the
    scroll with the content height unchanged. The hold then stops early, and that document may
    still land a little off.
- **R10: a real click on a back window's ×.** It activates that window before the close
  (Windows, GNOME with click-to-focus; reasoned, unmeasured), so the snapshot ranks it most
  recent and it comes back in front, as it does today.

---

## Current behaviour (read 2026-10-07; line numbers as of b5a8220)

- **Scroll restore** (`app.js:1411-1426`): one `requestAnimationFrame` after the render, then
  `window.scrollTo(0, scrollY)`. An image above that point that loads later grows `els.content`
  (`<article>`, `index.html:91`); the body scrolls.
  - On WebView2, scroll anchoring moves the view down with the text: README saved at 800 came
    back at ≈1422 (measured).
  - On WebKitGTK the page was still short at the `scrollTo`, which clamped: 15819 → 14079
    (measured once, under Xvfb). P3 couldn't reproduce it on the live desktop.
  - `rememberScroll` (`app.js:4415-4422`) banks the resulting position into the history entry,
    so the wrong one is what the next save writes.
- **Window order:** windows are saved least-recently-focused first (`session.rs:219-221`,
  `save` at 404-416).
  - `restore_session` (`main.rs:1383-1421`) gives the first window to `main` and spawns the
    rest. `restore_offered_session` (`main.rs:1235-1267`) gives the first to the window whose
    button was pressed.
  - Each window raises itself when its boot finishes (`app.js:4583-4594`). Whichever finishes
    **last** ends in front, not necessarily the last-used one.
  - The exception is a file the reader opened. Then every restored window carries
    `behind: "main"` and raises `main` instead.
  - Measured: on Windows a maximized `main` came out in front (twice). The Mac and Linux put the
    last-used window in front (once each). P1 reproduced it with scripted closes of the back
    window first: the saved order itself was wrong, from the close order (Task 2b). Whether the
    Tauri 2.12 walk closed the same way, or real use does, is unknown (R10).
  - Closed windows are written by close order reversed (`restorable`, `session.rs:394-401`),
    on the assumption that the window in front is closed first.
- **Frames** (`session.rs:94-143`): `Frame::of` saves `outer_position()` and `inner_size()`, or
  nothing when minimized.
  - `apply` sets the position, and the size unless the frame is maximized. A maximized frame's
    numbers are the maximized ones, used only to pick the monitor.
  - The frontend maximizes before `show()` at boot (`app.js:4584-4585`; on Windows the maximize
    shows the window). It maximizes after the restore on the offer path (`app.js:2940`).
  - `state.frames` starts empty each run. Only `save` and `note_frame` fill it
    (`session.rs:326-333`, `424-428`), so a window still being built is saved with `frame: None`.
- **Sizes on Linux** (tao 0.37.1 `platform_impl/linux/window.rs:332-356`, `495-514`):
  - `inner_size` is the size from the last GTK configure event. That is the GdkWindow's size,
    which on GNOME Wayland includes the client-side title bar and shadow.
  - `set_size` calls `gtk_window_resize`, whose size excludes them.
  - The difference is added on every launch: +52×89 px measured on Ubuntu 26.04 GNOME Wayland
    with the debug build.
  - **Not measured:** the AppImage, which now runs under X11 (`GDK_BACKEND=x11`, 852a186).

## Probes (run during the plan review, before the go)

Each fix rests on a cause that is reasoned, not measured. Results go into this plan before it is
offered for execution.

- [x] **P1 — Windows front window.** Debug build with the renamed identifier (`drive-app`).
  - Two windows: `main` maximized, the other normal and focused last. Quit and relaunch. Record
    which window is in front.
  - Log the order the boots finish in: a `console.log` around `maximize()`/`show()`/`setFocus()`
    in each window.
  - Repeat with `main` not maximized.
  - Expected: the window that finishes last is in front. Task 2 then holds whatever the cause.
  - **Result (2026-10-07, Windows VM, debug build of b5a8220, 6 runs: 3 with `main`
    maximized, 3 without):**
    - Same outcome in all six. The window that finished booting last came back in front, as
      expected. But it was the window used **first**: the last-used one (`w1`, open-items) was
      written **first** in `session.json`.
    - Cause: the quit closed `main` (behind) first, then `w1` (front), as the brief said. A
      quit on Windows is closing the windows one by one, and `restorable`
      (`session.rs:394-401`) orders the closed windows by close order. It assumes the window in
      front is closed first. `focus_order` drops a window when it closes (`main.rs:1779`), so
      it can't correct that.
    - So the item's symptom is reproduced by closing the back window first. That is likely how
      the Tauri 2.12 walk quit too (unverified). Closing the front window first gives the right
      order (`the_chain_is_written_behind_first`).
    - Boot timings: `main` was shown and focused 45-65 ms after its boot finished; `w1` 155-340
      ms. The spawned window finished last every time, but nothing guarantees that.
- [x] **P2 — GNOME sizes, both backends.** On the Linux VM, with the debug build (native
  Wayland) and the 1.7.3 AppImage (X11):
  - Start a 1100×860 window. Read `inner_size`, `outer_size`, and GTK's `gtk_window_get_size`
    (a throwaway `eprintln!` in a debug build).
  - Quit and relaunch three times.
  - Also at 200 % scale, if the VM allows it.
  - Expected: native Wayland grows by the title bar and shadow, while GTK's size equals what was
    set. X11 may not grow at all.
  - **Result (2026-10-07, Linux VM, GNOME 50, debug build of b5a8220):**
    - Native Wayland, 100 %: grows +52×89 every cycle (1100×860 → 1152×949 → 1204×1038 →
      1256×1127). `inner_size` equals `outer_size`, both GTK's size + 52×89. GTK's size is
      always the size set (1100×860, then 1152×949, …). `outer_position` is `Ok(0,0)`, not an
      error (R5's premise). Display type `GdkWaylandDisplay`.
    - `GDK_BACKEND=x11` (what the AppImage runs): no growth over three cycles. `inner_size`
      1100×860 equals GTK's; `outer_size` 1100×897; real position 443,168. `GdkX11Display`.
    - 200 % (set through Mutter's DisplayConfig, temporarily): the 1100×860 window doesn't fit
      the 960×600 logical screen, so GNOME maximized it every cycle. Growth is hidden there.
      Wayland: `inner_size` 1786×1136 with GTK 893×531 (still title-bar-inclusive). X11:
      `inner_size` 1786×1062 = GTK 893×531 × 2, so Task 4's size × scale is right. A
      non-maximized size at 200 % is unmeasured.
    - So Task 4 holds as planned; it changes nothing on X11, where the two sizes already agree.
- [x] **P3 — Scroll drift timing.** On Windows and Linux, with README and its screenshot:
  - Log `els.content.offsetHeight` at the `scrollTo` and at each image `load`.
  - Expected: the content grows after the `scrollTo` by the heights of the images above the
    saved point.
  - **Result, Linux (2026-10-07, GNOME Wayland, debug build):** no drift in any case.
    - A relaunch with a warm cache, a relaunch with WebKit's cache and data emptied, and Back
      from 1500 and from the bottom (11565) all logged the content at its final height at the
      `scrollTo`. No image loaded after it, and the scroll stayed exact.
    - The 15819 → 14079 drift seen in the Tauri 2.12 walk (Xvfb, README first opened through the
      Open dialog) did not recur; its cause is unknown.
    - Task 1 still applies there: it does nothing when every image is already complete
      (Review Focus 2).
  - **Result, Windows (2026-10-07, VM, 6 restores and 3 Backs; the owner's PC gave the same
    10163 → 12506 before the probes moved):**
    - At the `scrollTo`: `scrollY` 1500, content 10163 px. The images have no height yet; they
      reload with a `?v=N` cache-buster.
    - As they load, the content grows to 12506 px (11446 when maximized). Scroll anchoring moves
      `scrollY` 1500 → 2297 at the image that adds about 800 px.
    - The final position is 2297 against 1500 saved, every run, Back included: +797 px.
    - With `main` maximized, the scroll restore ran 8-10 ms **before** the maximize. The first
      `scrollTo` lands at the un-maximized width, and the maximize changes the height again.
      Task 1's observer re-applies at the final layout, which is the layout the position was
      saved at.

## Task 1: Re-apply a restored scroll while the page grows (`app.js`)

**Files:** `src/app.js:1411-1426` (the restore), and a helper beside it.

- [ ] A helper `holdScroll(target, token, entry)` runs after `window.scrollTo(0, scrollY)`, when
  `scrollY` is a number above 0. One `AbortController` owns everything below.
  - A `ResizeObserver` on `els.content`. On each callback, if the same render and entry are
    still shown (`token === shownToken && shownEntry === entry`):
    - note the content height;
    - `window.scrollTo(0, target)`;
    - note the resulting `window.scrollY` as `expected`;
    - if every `img` in `els.content` is `complete`, abort.
    - Otherwise (another render or entry), abort.
  - `load` and `error` listeners on `els.content`, with capture. Once every `img` is
    `complete`, abort. An image that settles without changing the height fires no observer
    callback, and a hold left running would undo scroll anchoring at a later sidebar toggle or
    window resize.
  - A passive `scroll` listener on `window`. The reader moved the page when
    `Math.abs(window.scrollY - expected) > 1` and `els.content.offsetHeight` equals the height
    last noted. Then abort. The slack allows for a fractional scroll at zoom (unmeasured).
  - Scroll anchoring's own move happens while the height has changed, so it doesn't count. Our
    re-apply lands on `expected`.
  - A `setTimeout` of 60 s aborts: `// ponytail: fixed fallback cap; images that settle later
    drift as before`.
- [ ] The observer's first callback, at `observe()`, re-applies the same value, so cached images
  (Review Focus 2) give no jump; it aborts there when all images are complete.
- [ ] The anchor branch (`jumpToAnchor`) is left alone (see *Out of scope*).
- [ ] Gates: `node --check src/app.js`; `cargo test`.
- [ ] Commit: `A document with images comes back where it was left`.

## Task 2: The last-used window comes back in front (`main.rs`, `app.js`)

**Files:**
- `src-tauri/src/main.rs`: `spawn_window` ≈376; its callers at 1012, 1146, 1259, 1371, 1403;
  `restore_offered_session` ≈1235; `restore_session` ≈1383; `session_payload` ≈1428.
- `src/app.js`: `reopenSession` ≈2931.
- `src-tauri/src/routing_tests.rs`: 388-397 asserts `main`'s exact pending JSON; 449 calls
  `session_payload`.

- [ ] Split the label out of `spawn_window`. `fn next_label(state: &AppState) -> String` takes
  the existing `fetch_add` and `format!`, and `spawn_window` takes the label as a parameter.
  Every caller passes `next_label(&state)`.
- [ ] `session_payload(w, behind: Option<&str>)` replaces the `behind_main: bool`.
- [ ] A pure helper decides who hands the front to whom: `fn behind_of(count: usize, index:
  usize, held_or_lost: bool, front: Option<&str>) -> Option<String>`.
  - `held_or_lost` (today's `behind_main`): every spawned window gets `"main"`.
  - Otherwise every index but the last gets `front`, and the last gets none.
  - With one window there is no `front`.
- [ ] `restore_session`:
  - With more than one window, reserve the last one's label first (`front`).
  - `main`'s claim carries `behind: front` only when **not** `held`. When held, it carries none,
    as today: `main` is the reader's file and stays in front.
  - The spawns use `behind_of` with `behind_main`. The last is spawned under the reserved label.
- [ ] `restore_offered_session`: the same rule. The answer to the asking window carries
  `behind: front`, and `reopenSession` ends, after its maximize, with
  `(front ?? appWindow).setFocus()` as the boot does. The window is already shown there.
- [ ] Tests:
  - `behind_of` in a unit test: one window; three windows; `held_or_lost`; index 0 held and not
    held.
  - Update `routing_tests.rs:388-397`: `main`'s pending JSON now has `"behind": "w1"`.
  - Update `routing_tests.rs:449` to the new signature. That test restores two windows, so the
    asking window's expected answer is `session_payload(&first, Some("w1"))`.
- [ ] Gates: `cargo fmt`, `cargo test`, `cargo clippy`.
- [ ] Commit: `The window used last comes back in front after a restart`.

## Task 2b: Closing the windows in any order keeps their stacking (`session.rs`, `main.rs`)

P1 showed the front window is decided by close order (R9). Closing the window in front first
restores right; closing the one behind first (a CDP `close()`, and possibly the taskbar's
*Close all windows* or a taskbar thumbnail's ×, neither measured) brings the back one forward.

Focus can't simply be read at each close: closing the front window focuses the next one, which
would then look most recent. So the order is fixed when the chain starts, at the first close
*request*. Taken at `Destroyed` instead, the OS's focus hand-off from the destroyed window can
reach `touch_focus` (`main.rs:1745`) first and rank the next window as most recent, a
regression where today works (unmeasured on every platform). At `CloseRequested`
(`main.rs:1748`, where `note_frame` already runs) the window still exists, and with *Close all
windows* every request arrives before any destroy.

**Files:**
- `src-tauri/src/main.rs`: `AppState`'s `closed` field (`main.rs:94`); the `CloseRequested` arm
  (1748); `set_reopen`'s clear (1218).
- `src-tauri/src/session.rs`: `note_frame` 326-333 or a new `note_close` beside it; every user
  of the chain's type: `remember` 313-317 (the expiry and `closed.is_empty()`, which means "no
  entries"), `window_closed` 361-365, `restorable` 394-401, `save`'s call at 441 (passes
  `&chain.entries`), and `the_chain_is_written_behind_first` 670-692.
- `src-tauri/src/routing_tests.rs`: `a_quit_within_the_grace_keeps_every_window` (501-518).

- [ ] The chain becomes `closed: Mutex<Chain>`, with
  `struct Chain { order: Vec<String>, entries: Vec<(Instant, Option<usize>, WindowSession)>,
  pending_rank: HashMap<String, Option<usize>> }`. `order` is the focus order as it stood when
  the chain began; each entry carries the closing window's rank in it. One mutex, so no new
  `AppState` field and no lock pair.
- [ ] `Chain::clear()` empties `order`, `entries` and `pending_rank` together. Every place that
  empties the chain uses it: the expiry in `remember` and `window_closed`, the expiry at the
  request, and `set_reopen` off.
- [ ] At `CloseRequested`: read `focus_order` (its own lock, released), then lock `closed`.
  - The chain has begun when `entries` **or** `pending_rank` is non-empty. A close can wait up
    to `CLOSE_WAIT` (2 s, `main.rs:1757-1765`) for its page's report, and a second request
    meanwhile must rank against the same snapshot.
  - If it hasn't begun (after the expiry check), set `order` to that copy.
  - Labels in the current `focus_order` but missing from `order` (a window opened and focused
    after the snapshot) are appended to `order`, so they rank most recent, not never-focused.
  - The window's rank is taken here and kept until its `window_closed` pushes the entry (a
    `pending_rank` map inside `Chain`, keyed by label, removed at the push).
- [ ] `window_closed` takes the label's pending rank out at its top, before the early returns
  for reopen `off` and an empty window (`session.rs:349-355`), then pushes
  `(now, rank, session)`. A window destroyed with no request gets its rank computed there the
  same way; possibly the macOS exit after Cmd+Q, unverified.
- [ ] `restorable` writes the open windows first, as now, then the chain sorted by rank:
  `None` first, then least-recently-focused. The sort is stable, so equal ranks keep close
  order. It no longer reverses close order.
- [ ] Tests:
  - replace `the_chain_is_written_behind_first`'s premise, keeping its cases: front closed
    first, then back → front written last; back closed first, then front → front written last
    (the P1 case); a window never focused → written first; a window opened after the snapshot
    → ranked most recent; the mid-session case (`restorable([c], [b])` → c, b) unchanged;
  - a new routing test through the request hook (`note_close`) then `window_closed`, for two
    windows, the back one closed first, and a second request arriving before the first
    window's destroy (one snapshot);
  - `a_quit_within_the_grace_keeps_every_window` (`routing_tests.rs:501-518`) never focuses
    `w1`, so it would now be written first: `touch_focus` both windows in the order the test
    means, and keep its expectation `[["m.md"], ["w.md"]]` true to that order.
- [ ] Gates: `cargo fmt`, `cargo test`, `cargo clippy`.
- [ ] Commit: `Closing the windows in any order brings back the one used last in front`.

## Task 3: A maximized window un-maximizes to where it was (`session.rs`, `main.rs`)

**Files:**
- `src-tauri/src/session.rs`: `Frame` 85-143, `note_frame` 326-333, `save` 404-430.
- `src-tauri/src/main.rs`: `spawn_window`'s claim at 387-389 (the record) and its build-failure
  path at 411-422 (the removal); the restore apply sites at 409, 1255-1257 and 1412-1415.
- `src-tauri/src/routing_tests.rs:127-133`: a `Frame` literal.
- `src/app.js`: `reopenSession` 2935-2941 (maximize first).

- [ ] `Frame` gains `#[serde(default)] normal: bool`.
  - For a maximized frame, it is true when `x/y/width/height` are the normal rectangle.
  - It is false when they are the maximized ones. That is the default, so every 1.7.3 file
    gets false.
- [ ] A pure rule: `fn sampled(old: Option<&Frame>, now: Sample) -> Option<Frame>`.
  - `Sample` holds `minimized`, `fullscreen`, `maximized`, the current outer position and size,
    and `monitors: Option<(Rect, Vec<Rect>)>`: the current monitor and all of them, as plain
    physical `(x, y, width, height)` rectangles. `Frame::of` converts each `tauri::Monitor`
    through `position()`/`size()`; `tauri::Monitor` has no public constructor
    (`tauri-2.12.1/src/window/mod.rs:58-64`), so tests build rectangles.
  - `monitors` is `None` when either monitor read fails, on native Wayland (R5), and when
    the GTK read fails (Wayland unknown).
  - Wayland comes from Task 4's `gtk_read`, which runs before this task. This task widens it to
    return `Option<(PhysicalSize<u32>, bool)>`, adding whether the display is Wayland:
    `gtk_window.display().type_().name() == "GdkWaylandDisplay"`, with `use gtk::prelude::*`
    (`ObjectExt::type_` through glib's prelude, `gtk-0.18.2/src/prelude.rs:14`;
    `WidgetExt::display`, `auto/widget.rs:618`); no new dependency. In tests it is never
    Wayland.
  - `type Rect = (i32, i32, u32, u32)`, physical. Overlap widths are computed in `i64` before
    multiplying, so extreme coordinates can't overflow.
  - Minimized, full screen (R2), or not visible (R11; `Sample` gains `visible` from
    `is_visible()`) → `old` unchanged. A restored window is still hidden when its first report
    saves (`reportSoon` from `restoreTabs`, `app.js:1712`, before the maximize at 4584), so it
    keeps the frame recorded at restore.
  - Maximized, with a *real* normal `old` (`!old.maximized || old.normal`) → `old`'s rectangle
    with `maximized: true, normal: true`. This holds when `monitors` is `None`, or when the
    monitor `old` overlaps most is the current one (R1, R6). A new
    `fn overlap_area(&self, monitor: Rect) -> u64`, built from the comparisons in
    `Frame::overlaps` (`session.rs:135-141`), measures it. On a tie the first monitor wins.
    Zero overlap with every monitor (a gone monitor) isn't the current one.
  - Maximized otherwise → the current (maximized) frame with `normal: false`. So a 1.7.3 frame
    never becomes normal.
  - Not maximized → the current frame, `maximized: false, normal: false`.
- [ ] `Frame::of(window, old)` reads the `Sample` and calls `sampled`.
  - Callers (`save`, `note_frame`) clone the stored frame and drop the `frames` lock first.
  - They call `Frame::of` with no lock held, then lock again to insert, as `save` does today
    (`session.rs:294-297`).
- [ ] Every restore records the frame it applies in `state.frames[label]`, synchronously,
  before any build thread starts. `frames` is only ever locked alone (`session.rs:327-332`,
  `347`, `425`), so the order is safe.
  - `restore_session`'s `main` apply (`main.rs:1412-1415`);
  - a spawned window's `Placement::Frame`, in `spawn_window` beside the label claim
    (`main.rs:387-389`), not on the build thread after `build()`;
  - `restore_offered_session` (`main.rs:1255-1257`).
  - The build-failure path (`main.rs:411-422`) removes the label's entry.
  - This way a normal rectangle survives a run in which the window was never sampled
    un-maximized. A save during the build writes the restored frame, not `None`.
- [ ] `reopenSession` (`app.js:2935-2941`) maximizes **before** `restoreTabs`, not after. That
  window is already visible, so R11 doesn't cover it: its first report (`restoreTabs` →
  `reportSoon`) would otherwise save `maximized: false` for the ~0.5 s until the maximize's
  resize report. As a bonus the tabs render, and Task 1 restores the scroll, at the final width.
- [ ] `apply`: set the position as now, and the size when `!maximized || normal`. The monitor
  check is unchanged: it runs on the rectangle the frame holds.
- [ ] Tests:
  - Unit tests on `sampled`:
    - normal → maximized keeps the normal rectangle;
    - maximized with no old → `normal: false`;
    - maximized with a 1.7.3-style maximized old (`normal: false`) → stays `normal: false`;
    - maximized with the old rectangle mostly on another monitor → `normal: false`;
    - maximized, old rectangle straddling two monitors, mostly on the current one → kept;
    - maximized with `monitors: None` → kept;
    - maximized with an old rectangle that overlaps no monitor → `normal: false`;
    - minimized keeps the old;
    - full screen keeps the old;
    - not visible keeps the old (a restored maximized frame stays `maximized: true`);
    - not visible with no old → `None`, as a minimized window today;
    - un-maximized takes the new.
  - A serde test: a 1.7.3 maximized frame (no `normal` key) loads with `normal: false`.
  - Update `routing_tests.rs:127-133`.
- [ ] Gates: `cargo fmt`, `cargo test`, `cargo clippy` on Windows; `cargo test` and
  `cargo clippy` on the VM (the Wayland read is Linux-only).
- [ ] Commit: `A window restored maximized un-maximizes to its own size and place`.

## Task 4: A restored window keeps its size on GNOME (`session.rs`, `Cargo.toml`)

Shaped by P2. If X11 (the AppImage) doesn't grow, this still applies to the .deb and .rpm,
which run native Wayland.

**Files:** `src-tauri/Cargo.toml` (a Linux target dependency), `src-tauri/Cargo.lock` (the
edge), `src-tauri/src/session.rs` (`Frame::of`'s size read).

- [ ] Add `[target.'cfg(target_os = "linux")'.dependencies] gtk = "0.18"`, the version already
  locked.
- [ ] On Linux, outside tests (`#[cfg(all(target_os = "linux", not(test)))]`: `MockRuntime::
  gtk_window` is `unimplemented!()`, `tauri-2.12.1/src/test/mock_runtime.rs:823-825`), the size
  comes from GTK:
  - a helper `gtk_read(window) -> Option<PhysicalSize<u32>>`: inside `app.run_on_main_thread`,
    `window.gtk_window()` (tauri 2.12.1 `webview_window.rs:1980`) `.size()`, sent back over a
    channel, times `scale_factor()`, rounded;
  - `run_on_main_thread` runs inline when already on the main thread
    (`tauri-runtime-wry-2.12.1/src/lib.rs:263-273`). That covers `note_frame` and `remember`;
    the update snapshot's tokio thread waits on the channel.
  - It is called with no lock held, as today (`save` calls `Frame::of` before taking `frames`,
    `session.rs:424-425`; `note_frame` holds none, `327`), so the main thread can't block on a
    lock the waiting thread holds. Task 3 keeps it so.
  - If the read fails, the size falls back to `inner_size()`.
  - Elsewhere, and in tests, `inner_size()` as now.
  - A comment says why: tao's `inner_size` is the configure size, which includes client-side
    decorations on Wayland, while `set_size` is `gtk_window_resize`, which does not.
- [ ] Gates: `cargo fmt`, `cargo test` and `cargo clippy` on Windows; `cargo test` and
  `cargo clippy` on the VM (the code is Linux-only).
- [ ] Commit: `A restored window keeps its size on GNOME`.

## Task 5: Walks

Debug builds with the renamed identifier (`drive-app`), with `config.json` and `session.json`
backed up first. Every platform runs through a Remote Control session: Windows on the Windows
VM (`md-viewer-win-vm`; never the owner's PC, which they are using), Linux on the Linux VM, and
the Mac. Each fetches the work first, pushed or as a bundle (Execution); unpushed work isn't visible
there.

- [ ] **W1 (all three) — scroll.**
  - README scrolled past its screenshot (≈800); quit; relaunch. It comes back at the saved
    position (±1 line).
  - Back to it from another document: the same.
  - With the README in a back window behind a maximized front window: the same, once raised.
  - Scroll during the load (a large or slow image if needed): the reader's scroll wins, with no
    pull back.
- [ ] **W2 (all three) — front window.**
  - Two windows, `main` maximized, the other used last: the other comes back in front.
    - quit by closing the front window first with a real click on its ×, then the other: the
      front one comes back in front (the case that works today must not regress);
    - quit by closing the back window first, without raising it (CDP `appWindow.close()`, or a
      taskbar thumbnail's × on Windows);
    - Windows: the taskbar's *Close all windows*.
  - Swap which was used last: `main` in front.
  - Through the empty screen's *Reopen* button (`"reopen": "ask"`): the same.
  - A file opened from the file manager while restoring (`"reopen": "restore"`): that file's
    window in front, as today.
- [ ] **W3 (all three) — un-maximize.**
  - A window at a known size and place, maximized; quit; relaunch, **twice**. It comes back
    maximized; un-maximized, it is back at that size and place.
  - A 1.7.3 `session.json` with a maximized window, through two relaunches: it un-maximizes to
    the default size, not the screen's.
  - Windows, if a second monitor is attached: maximized on one, moved maximized to the other,
    quit, relaunch. It comes back maximized on the second (R1).
  - Through the *Reopen* button (`"reopen": "ask"`), all three: a window maximized at quit;
    *Reopen*; un-maximize. It returns to its saved size and place (Linux Wayland: size only).
    On GTK the resize `apply` asks for may not land before the maximize (unmeasured).
    - Read `session.json` straight after *Reopen*. On Linux the maximize state updates only
      at the compositor's event, so the first report may still save `maximized: false` for
      about 0.5 s (reasoned). If it does, it goes to the owner at triage.
  - Mac: full screen, quit, relaunch. It comes back at its pre-full-screen size (R2).
  - Linux Wayland, if the VM can add a second monitor: maximized there, quit, relaunch,
    un-maximize. The normal size is kept (R5).
  - A gone monitor (Review Focus 6), if a second monitor can be detached: a normal window on
    it, quit, detach, relaunch. It comes back on the remaining monitor at default placement;
    maximize, quit, relaunch, un-maximize gives the default size.
- [ ] **W4 (Linux) — GNOME size.** A 1100×860 window through three quit-relaunch cycles keeps
  the same size each time:
  - on the debug build (Wayland);
  - with `GDK_BACKEND=x11` (what the AppImage runs), which P2 showed doesn't grow today: still
    no growth, and the position still restored.
- [ ] **W5 (all three) — no regression.** A normal window's size and place round-trip as before:
  exact on Windows and the Mac, as measured in the Tauri 2.12 walks.

## Out of scope

- The anchor landing drift: arriving through a link to a heading below an unloaded image drifts
  the same way, but the item covers restore and Back only.
- Tear-off placement, which is its own work with three deferred items.
- The update-restart path can't be walked before a release; Review Focus 5 is checked by
  reading the code. The next release's First runs (updater 2.13.1's install path) restarts
  through it.

## Execution

- The main session orchestrates.
- Tasks 1, 2, 2b, 4 and 3 go to a `coder` subagent (Opus), one task at a time and in that
  order, each committed on main after its gates. Task 3 adds the Wayland flag to Task 4's GTK
  read in the same commit that first uses it, so no commit runs R1 on Wayland unguarded and
  none leaves an unused value for clippy.
- Getting the code to the VMs and the Mac: main isn't pushed (a push needs the owner's go,
  asked separately). Each machine first checks out the base, `origin/main` as pushed when the
  work starts (b5a8220 or the docs commits after it), then fetches the series from a bundle.
  - Gates: for Task 4's and Task 3's Linux gates, the series from the base through that task's
    commit (`<tip>` = that commit). A task's commit alone won't build: Task 3
    edits `spawn_window` after Task 2's change and widens Task 4's `gtk_read`.
  - Walks: the full series (`<tip>` = main) to all three machines before
    Task 5.
  - The series travels as a `git bundle` (`git bundle create series.bundle <base>..<tip>`,
    where `<tip>` is a branch name, since a bundle carries refs: `main`, or a throwaway
    `git branch walk-tip <task commit>` deleted afterwards),
    base64-encoded into the brief with its `sha256sum`, the only channel a Remote Control
    session can read. The machine runs `git fetch origin` and checks out the base, decodes the
    bundle (`base64 -d`), checks the `sha256sum`, runs `git bundle verify series.bundle` (which
    names a missing base plainly), then `git fetch series.bundle <tip ref>:walk` and checks out
    `walk`. Binary-safe: no tab, trailing space or CRLF can corrupt it. If the brief would be
    too large, ask the owner for a go to push a walk branch instead.
  - The machine goes back to its base afterwards and deletes `walk`.
- Walks follow all five tasks, then the change review loop (CLAUDE.md step 5).
