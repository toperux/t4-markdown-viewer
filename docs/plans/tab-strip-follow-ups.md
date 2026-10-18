# Tab Strip Follow-ups Plan

> **Status (2026-10-18):** executed on the owner's go; walked on all three; change review and
> triage below. Ships unreleased with the tab switcher, in one release.
>
> **At archiving:** this plan moves to `archive/plans/` with `tab-switcher.md`; repoint the
> records' `plans/tab-strip-follow-ups.md` references (`closed-items.md`) with it.

**Spec:** the items the owner triaged on 2026-10-17 as "right after the switcher ships"
(`docs/open-items.md` → Deferred), to ship in one release with the tab switcher (62de326,
pushed, unreleased):

1. On Linux, a tab in an overflowing strip takes clicks only in its top 7 px.
2. The strip doesn't scroll to the active tab, and a plain wheel doesn't scroll it.
3. On macOS, the app's Ctrl shortcuts also fire on the Control key.

Folded in after review pass 1 (Q4-Q8): four older open items whose reopen triggers this work
meets, and the scroll hold.

**Architecture:** frontend only (`src/app.js`, `src/base.css`, `src/index.html`, `README.md`),
plus the records. No Rust change, no new dependency.

## Current behaviour (main at 62de326)

### The strip

- `#tabs` (`base.css:1210-1221`): `display: flex; padding: 6px 0.5rem 0; overflow-x: auto;
  overflow-y: hidden; scrollbar-width: thin`. It sits in `#tab-row` beside `#tab-search`.
- **`renderTabs`** (`app.js:2125-2167`) rebuilds every `.tab` on each call. It returns early
  when the strip is hidden (one tab). Its callers:
  - `updateChrome`, on every tab change and every live reload;
  - `showImage` (`1905`);
  - `reorderTo` (`2289`);
  - the drop caret's moves;
  - the drag's end and cancel (`4414`, `4444`, `4487`).
- **Nothing in the app sets the strip's `scrollLeft`.** Only the browser's own focus scroll
  moves it, when keyboard focus lands on a tab (`4513`, `focusActiveTab` `4525`). There is no
  wheel listener on the strip.
- **The scroll hold:** after a switch restores a reading position, the hold keeps the page
  there while pictures load (`holdScroll`). A window capture `wheel` listener stops it for
  any wheel the sidebar doesn't take (`sidebarTakes`, `app.js:1738-1750`). It runs before the
  strip's own listeners, so any wheel over the strip, Shift+wheel included, ends the hold
  today.
- `activateTab` (`2283`) → `showActive` → `updateChrome` → `renderTabs`.
- **Ids are never reused:**
  - `makeTab` gives every tab a new one (`315`), and `reopenClosed` gives a fresh one (`2325`).
  - A tab moved in from another window, or restored, goes through `rebuildTab` → `makeTab`.
- **Closing and reopening a tab:**
  - `closeTab` pushes the tab `removeTab` removed onto `closedTabs` (`2315-2320`);
    `removeTab` finds its index as `i` (`2293`).
  - `reopenClosed` appends it with `tabs.push`, at the strip's end (`2322-2330`).

### Measured on the Linux VM (probe, 2026-10-18, WebKitGTK 2.52.6, Xvfb and Wayland)

- **The dead band:** the overlay scrollbar's hit area.
  - It is a fixed 21 px band at the strip's bottom edge. It takes no layout space, so it lies
    over the tabs.
  - With the strip overflowing (31 tabs, strip y 36-70), `elementFromPoint` finds a `.tab` only
    at y 42-48; from 49 to 69 it finds `#tabs`. `elementsFromPoint` at y 56 has no `.tab` at all.
  - The band doesn't depend on the scroll position. With 5 tabs (no overflow) the whole tab
    is clickable.
- **What restores the full band (y 42-69):**

  | Variant | Clicks | Plain wheel | Shift+wheel | Height |
  |---|---|---|---|---|
  | today (`thin`, overlay) | top 7 px only | page, or the strip by accident in the band | strip | 34 px |
  | `scrollbar-width: none` | whole tab | page | strip | 34 px |
  | `overflow-x: hidden` | whole tab | page | nothing | 34 px |
  | `padding-bottom: 21px` | whole tab | page | strip | 55 px |

  - A 12 px padding still leaves 61-69 dead.
  - `scrollbar-width: auto` and `::-webkit-scrollbar { height }` change nothing; WebKitGTK
    ignores the latter here.
  - With overlay scrollbars off (`GTK_OVERLAY_SCROLLING=0`), a classic 21 px bar takes layout
    space below the tabs and every tab is clickable. Users get overlay scrollbars by default on
    GNOME (reasoned), and the app doesn't control that.
- **The wheel today:** over the tabs, a plain wheel scrolls the page (Linux, real X11 wheel;
  Windows, measured in the switcher walks). It scrolls the strip only by landing on the overlay
  band. Shift+wheel scrolls the strip.

### Keys

- **`onKeydown`** (`app.js:5088`): `const ctrl = event.ctrlKey || event.metaKey` (`5138`), so
  Control and Cmd alike, on every platform.
  - Before the modal guard (`5182`):
    - the reload, Ctrl+R (`5145`);
    - the find and print suppression (`5165`);
    - `switcherKey` (`5161`), already Cmd-only on the Mac;
    - Ctrl+Q, off on the Mac (`5173`).
  - `!ctrl` at `5211` lets Enter or Space open a focused diagram or picture.
  - Alt+Left and Alt+Right go Back and Forward from anywhere (`5228-5239`), text fields
    included.
  - The gate `if (!ctrl || event.altKey) return;` (`5248`). After it:
    - `[` and `]` (comment `5250-5251`: "accepted everywhere rather than branching on the
      platform");
    - Tab (`5263`);
    - the picture zoom keys `+ = - 0`;
    - the key switch (`5287+`): O, Shift+O, Shift+F, F, G, P, Shift+T, T, N, W (with any
      Shift, `5321`), comma.
- **`onDiagramKeydown`** (`app.js:1478`): `event.ctrlKey || event.metaKey` for `+ = - 0` in the
  diagram dialog.
- **Wheel zoom** of pictures (`5063-5068`) and diagrams (`1447-1451`) checks `ctrlKey` only. A
  trackpad pinch arrives as Control+wheel.
- **Already Cmd-only on the Mac:** the tree's Ctrl+click (`2925-2927`; Control-click is the
  right-click there), and its menu labels, which read "⌘" (`5540-5542`).
- **The Mac menu** (`main.rs:1619-1680`):
  - Quit (Cmd+Q);
  - Hide, Hide Others, Show All;
  - Edit's predefined items;
  - Minimize, Maximize, Fullscreen;
  - Close Window (Shift+Cmd+W).

  There is no Cmd+W, so the page handles it. The page also sees Shift+Cmd+W before the menu
  and closes a tab (`5321`, open item, measured 2026-09-28).
- **On-screen key names** say Ctrl on every platform:
  - the tooltips at `index.html:15`, `27`, `56`, `70`, `135-138` and `307-310`;
  - the Open button's runtime title (`app.js:3905`);
  - the Settings hints at `index.html:224-225` and `230`, which also say "in Explorer".
- **README:**
  - `README.md:236`: "On macOS, read `Cmd` for every `Ctrl` below."
  - `263`: "`Ctrl`+wheel zooms" (on the Mac that is Control, not Cmd).
  - `268-274`: Cmd+W and the Shift+Cmd+W workaround.
- **Records:**
  - `docs/closed-items.md:1301-1304`: "On macOS, Control+F opens find too", kept 2026-10-09
    (ruling R6). Measured 2026-10-17: Control+N in the sidebar filter opens the Open panel.
  - `docs/open-items.md` holds the four older items folded in here: `104-112` Shift+Cmd+W,
    `160-164` reopen at the end, `165-169` app text speaks Windows, `217-220` Option+Arrow.

## Owner rulings (2026-10-18)

- **Q1. Linux: hide the strip's scrollbar** (`scrollbar-width: none`) on Linux only. Windows
  and the Mac keep theirs, which work.
- **Q2. A plain wheel over the strip scrolls it sideways** while it overflows. When every tab
  fits, the wheel scrolls the page as today.
- **Q3. Mac shortcuts are Cmd-only,** except Control+Tab and Control+Shift+Tab, which keep
  switching tabs (Cmd+Tab belongs to macOS). Wheel and pinch zoom stay on Control.

After review pass 1:

- **Q4. The scroll hold:** a wheel over an overflowing strip no longer ends it.
- **Q5. Mac on-screen text:**
  - key names say ⌘ ("Ctrl+O" becomes "⌘O", "Ctrl+Shift+O" becomes "⇧⌘O"; spelling per Q9);
  - "Explorer" becomes "Finder" on the Mac and "your file manager" on Linux.
  - Closes the open item "some app text speaks Windows".
- **Q6. Shift+Cmd+W closes the window on the Mac:** the page leaves it to the menu. Closes
  the open item.
- **Q7. Ctrl/Cmd+Shift+T puts a tab back where it was,** and **on the Mac, Option+Left/Right
  in a text field moves by a word.** Closes both open items.
- **Q8. The reveal also follows the active tab's label** (a link or Back/Forward within the
  tab). A window resize doesn't reveal: a closed accepted limit.

After review pass 2:

- **Q9. One Mac spelling for key names:**
  - "⌘O" and "⇧⌘O" in tooltips and in the Settings hints, with no "+" between the keys;
  - "⌘-Click" in the sidebar's menu (from its "Ctrl+Click"), the macOS way of writing a
    modified click.

## Decisions A-H (ruled by the owner 2026-10-18: as proposed)

- **A. Reveal the active tab** whenever the active tab or its label changes. That covers a
  switch, a new tab, a close, a restore, and a link or Back/Forward within the tab (Q8). The
  strip scrolls by the least amount that shows the tab whole, plus its 0.5rem side padding.
  Other redraws don't reveal: a live reload, a drop-caret move, a reorder. So a strip the
  reader scrolled by hand stays where they put it.
- **B. Instant, not smooth.** A smooth scroll would race the next redraw and is harder to test.
- **C. The wheel at the strip's ends:** the strip keeps the wheel even once it can't scroll
  further that way; it doesn't pass it on to the page. One rule: over an overflowing strip, the
  wheel moves the strip.
- **D. Wheel direction:** down and right move right, as Chrome on Linux and VS Code do. A
  trackpad's sideways swipe (`|deltaX| >= |deltaY|`) and any modifier (Shift, Ctrl, Alt, Cmd)
  are left to the browser.
- **E. A Mac shortcut keeps its Control spelling** only for Tab. On the Mac, `[` and `]` need
  Cmd (Control+[ no longer goes Back), and so do the picture and diagram zoom keys. Control+Enter
  on a focused diagram or picture now opens it, as Enter does (`5211`).
- **F. Reopen in place:** the tab goes back to the index it was closed from, clamped to the
  strip's length. Tabs closed or moved meanwhile shift it, as in browsers.
- **G. Option+Left/Right on the Mac:** inside any text `input`, they are left to the field.
  That covers the tree filter, the find bar and the switcher. Elsewhere they go Back and
  Forward as now. Cmd+[ and Cmd+] still go Back and Forward from a field. Off the Mac,
  Alt+Left/Right stay Back/Forward everywhere, as Windows and Linux readers expect.
- **H. Records:**
  - The Control+F limit in `closed-items.md` gets a *Lifted 2026-10-xx* line.
  - The seven open items close into `closed-items.md` with what was done: the three strip and
    Mac items and the four folded in.
  - Q8's resize case becomes a closed accepted limit.
  - At release, this plan and `tab-switcher.md` move to `archive/plans/`, and the records'
    `plans/` references are repointed (already ruled).

## The change

### 1. Linux: no scrollbar on the strip (Q1)

- `app.js`, at the top level beside `isMac` (`281`):

  ```js
  /** WebKitGTK's user agent names Linux, X11 and Wayland alike; WebView2's doesn't. */
  const isLinux = navigator.userAgent.includes("Linux");
  document.documentElement.classList.toggle("linux", isLinux);
  ```

  `isLinux` is used again by step 8.

  The script runs before the hidden window's first paint, so the strip never shows a
  scrollbar first.
- `base.css`, after `#tabs`:

  ```css
  /* WebKitGTK's overlay scrollbar takes clicks in a 21px band over the tabs'
     lower part, so on Linux the strip goes without one; the wheel, Shift+wheel
     and the active tab's reveal move it. */
  html.linux #tabs {
    scrollbar-width: none;
  }
  ```

### 2. Reveal the active tab (A, B, Q8)

- A module-level `let revealed = "";` beside `dropCaret`: the active tab's id and label when
  the strip last scrolled to it.
- `renderTabs`, after `replaceChildren` (the visible path only):
  - with `key = `${activeId}\n${activeTab()?.label}``, if `key !== revealed`, bring the active
    `.tab` into view and set `revealed = key`;
  - from the rects: `s = els.tabs.getBoundingClientRect()`, `r = tab.getBoundingClientRect()`,
    `pad` = the strip's computed `padding-left`;
  - if `r.left < s.left + pad`: `scrollLeft -= s.left + pad - r.left`;
  - else if `r.right > s.right - pad`: `scrollLeft += r.right - (s.right - pad)`.
- Not `scrollIntoView`, which would also scroll the page (the switcher's reason, too).
- The hidden-strip path leaves `revealed` alone. So when a second tab appears and the strip
  shows, the active tab is revealed then.

### 3. The wheel (Q2, Q4, C, D)

- One test, shared with the scroll hold, beside `holdScroll`:

  ```js
  /** A wheel the tab strip takes for itself: over it while it overflows. */
  function stripTakes(event) {
    return els.tabs.contains(event.target) && els.tabs.scrollWidth > els.tabs.clientWidth;
  }
  ```

- The hold (`1748`): `if (!sidebarTakes(event) && !stripTakes(event)) stop();`. This also
  covers today's Shift+wheel, which scrolls the strip natively.
- On `els.tabs`, beside its other listeners in `main()`, not passive:

  ```js
  // A plain wheel over an overflowing strip moves it sideways (owner's ruling).
  // Shift+wheel and trackpad swipes already do, and modifiers mean other things.
  els.tabs.addEventListener("wheel", (event) => {
    if (event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) return;
    if (Math.abs(event.deltaY) <= Math.abs(event.deltaX) || !stripTakes(event)) return;
    event.preventDefault();
    els.tabs.scrollLeft += event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY;
  }, { passive: false });
  ```

  `16` for a line, as the picture and diagram wheels have it (`1450`, `5067`).

### 4. Mac: Cmd-only shortcuts (Q3, E)

- **One helper**, beside `isMac`:

  ```js
  /** The shortcut modifier: Cmd on the Mac, where Control+letter belongs to text boxes. */
  function cmdKey(event) {
    return isMac ? event.metaKey : event.ctrlKey || event.metaKey;
  }
  ```

  Off the Mac it is today's `ctrl`, unchanged.
- **`onKeydown`:**
  - `const ctrl = cmdKey(event);`;
  - `switcherKey` drops its own Mac test, `(k === "p" || event.keyCode === 80)`, and its
    comment (`5159-5160`) says the Mac's Control is left out by `ctrl` itself;
  - **Tab moves directly above the gate (`5248`).** That keeps it after the modal guard, so a
    dialog still owns Ctrl+Tab:
    `if (event.key === "Tab" && !event.altKey && (isMac ? event.ctrlKey : ctrl))`, then
    `cycleTab` as now. Off the Mac the condition is today's.
  - The comment at `5250-5251` becomes: "Alt+Arrow above is the Windows idiom, Cmd+[ and
    Cmd+] the Mac one; both are accepted off the Mac."
- **`onDiagramKeydown`:** `cmdKey(event)` in place of `(event.ctrlKey || event.metaKey)`.
- **Unchanged:**
  - the wheel and pinch zoom (`ctrlKey`);
  - the tree's Ctrl+click;
  - Ctrl+Q (off on the Mac);
  - the menu.

### 5. Shift+Cmd+W to the menu (Q6)

- In the key switch's `w` branch (`5321`), first:
  `if (isMac && event.shiftKey) return; // Close Window, the menu's (Shift+Cmd+W)`. No
  `preventDefault`, so the menu's accelerator gets it.
  - Reasoned that it then fires; the open item measured the page taking it first. The walk
    checks it with a real key.
- Off the Mac, Ctrl+Shift+W still closes the tab, as now.
- **README `268-274`:**
  - "For now `Shift+Cmd+W` closes a tab instead…" goes;
  - "`Shift+Cmd+W` closes the window, from **Window › Close Window**" stays, in its place.

### 6. Reopen in place (Q7, F)

- `removeTab`: `gone.closedAt = i;` after the splice (`2293` has `i`). A tab torn off to
  another window carries it too, harmlessly: `packTab` doesn't send it, so it lives only in
  memory, as `closedTabs` does.
- `reopenClosed`: `tabs.splice(Math.min(tab.closedAt ?? tabs.length, tabs.length), 0, tab)` in
  place of `tabs.push(tab)`.

### 7. Option+Left/Right in a Mac text field (Q7, G)

- One test, beside `cmdKey`:

  ```js
  /** On the Mac, Option+Left/Right in a text box move by a word, not Back/Forward. */
  function macWordKeys(event) {
    return (
      isMac &&
      event.altKey &&
      (event.key === "ArrowLeft" || event.key === "ArrowRight") &&
      document.activeElement?.matches("input[type=text], input[type=search]")
    );
  }
  ```

- The Alt+Arrow branch (`5228`) starts with `if (macWordKeys(event)) return;`. There's no
  `preventDefault`, so the field moves the caret by a word. That covers the tree filter, the
  find input and the switcher.
- **Settings' document-font box:** the modal guard (`5183`) prevents Alt+Left/Right under a
  dialog, against WebView2's own Back/Forward. It skips the prevent when `macWordKeys(event)`,
  so Option+Arrow moves by a word there too. Off the Mac the guard is unchanged.

### 8. Mac and Linux on-screen text (Q5)

- At boot, in place of the tree menu's swap (`5540-5542`), on the Mac (Q9):
  - every `[title]` in the page: "Ctrl+Shift+" becomes "⇧⌘", then "Ctrl+" becomes "⌘";
  - the tree menu's `.menu-key`: "Ctrl+" becomes "⌘-" ("⌘-Click");
  - the Settings hints' key pairs `<kbd>Ctrl</kbd>+<kbd>O</kbd>`: the "Ctrl" `kbd` becomes "⌘",
    and the "+" text node after it is removed. Its pair then reads "⌘O".
- The pass runs before any tab or tree row exists, so no path in a title is rewritten.
- `showOpenMode` (`3905`) builds its title from one `KEY_OPEN` string, `isMac ? "⌘O" :
  "Ctrl+O"`, so the boot swap and later mode changes agree.
- **"Explorer"** in the Settings hint (`index.html:224`) goes in a `<span id="file-manager">`.
  At boot it becomes "Finder" on the Mac and "your file manager" on Linux (`isLinux`);
  Windows keeps "Explorer".
- `cargo test` reads `index.html`'s ids (`render.rs:586-617`): the new id is checked against
  document ids there.

### 9. README

- **Line 236** becomes: "On macOS, read `Cmd` for every `Ctrl` below, except `Ctrl+Tab`,
  which is `Control+Tab` there as in Safari (`Cmd+Tab` belongs to macOS), and `Ctrl`+wheel,
  which is `Control`+wheel or a pinch."
- The Shift+Cmd+W sentence as in step 5.
- `Ctrl+Shift+T` | "Reopen the last closed tab, where it was".
- **Line 240,** Back / forward: add "(on macOS, `Option+←/→` in a text box moves by a word
  instead)".

## Risks

- **R1. Hiding the scrollbar hides the overflow on Linux.** Nothing shows that more tabs lie
  off the edge. The reveal, the wheel and the switcher cover reaching them. An edge fade is
  possible later; it isn't in this plan.
- **R2. The wheel on a mouse with a tilt wheel or a trackpad:** sideways input is left alone (D).
  A trackpad's mostly vertical swipe over the strip now moves the strip, not the page.
  Reasoned, not measured; the walk tries a trackpad on the Mac.
- **R3. The reveal during a drag:** a drag doesn't change `activeId` until a tab is torn off
  or dropped elsewhere. After a tear-off the next tab becomes active and is revealed, which
  is fine.
- **R4. Shift+Cmd+W reaching the menu** is reasoned, not measured (step 5). If WKWebView
  swallows it anyway, the fix is a menu-side change, outside this plan; the walk tells.
- **R5. A Linux user agent without "Linux":** not expected for WebKitGTK; the walk logs it.
- **R6. The Mac's Fullscreen menu key,** if it is Control+Cmd+F, now passes `ctrl` (Cmd) and
  opens find as well. That was already so before this plan; the walk notes what it does.

## Tasks

### Task 1: the change

- [ ] Steps 1-9 above.
- [ ] `node --check src/app.js`; `cargo test` (a test reads `index.html`'s ids).
- [ ] Commit: "Scroll the tab strip to the active tab and with the mouse wheel". Its body has
  one line per further topic: the Linux clicks, the Mac keys and text, Shift+Cmd+W, reopen in
  place. Or split per topic at packaging (the owner's call then).

### Task 2: walks (Windows VM, Linux VM, Mac; drive-app skill)

- **All:**
  - 30+ tabs: Ctrl+Tab through them, and the active tab is always fully in view.
  - The switcher picking a far tab: revealed.
  - A new tab opens revealed.
  - Close the active tab: the next one is revealed.
  - A link to a long-named file within the active tab at the strip's edge: revealed.
  - Scroll the strip by hand, then a live reload of the active file: the strip stays put.
  - A tab drag reorders and tears off as before.
  - The wheel over an overflowing strip scrolls it sideways, and keeps the wheel at its ends.
    Over a strip that fits, the page scrolls. Shift+wheel is unchanged.
  - After a switch that restores a reading position, wheeling the strip leaves the position.
  - Ctrl/Cmd+Shift+T puts a tab back at its old index.
- **Linux** (real X11 pointer, and Wayland):
  - no scrollbar on the strip, and the user agent logged;
  - a click anywhere on a tab (y 42-69) switches;
  - a drag at a label's middle drags the tab;
  - the Settings hint says "your file manager", and the tooltips say Ctrl.
- **Mac:**
  - Control+N, F, G, T, O, W, R, P and comma inside the sidebar filter do nothing of the
    app's, and the keys reach the text box;
  - their Cmd spellings work;
  - Control+Tab and Control+Shift+Tab switch tabs;
  - Cmd+[ and Cmd+] go Back and Forward, and Control+[ doesn't;
  - picture zoom: Cmd+= works, a trackpad pinch zooms, Control+= doesn't;
  - the diagram dialog's zoom keys the same;
  - Shift+Cmd+W closes the window (a real key), and Cmd+W closes a tab;
  - Option+Left/Right in the tree filter, the find bar, the switcher and Settings' font box
    move by a word; in the document they go Back and Forward;
  - tooltips and the Settings hints read "⌘O" or "⇧⌘O" with no "+", the tree menu reads
    "⌘-Click", and the hint says Finder;
  - a trackpad swipe over the strip (R2);
  - Control+Cmd+F (R6).
- **Windows:**
  - every Ctrl shortcut as before (a quick pass), Alt+Left/Right included;
  - the tooltips and hints unchanged;
  - the Windows key alone or Win+letter does nothing new.

### Task 3: records

- `docs/open-items.md`: the seven items close.
- `docs/closed-items.md`:
  - they move there, each with what was done;
  - the Control+F entry gets its *Lifted* line;
  - the resize case (Q8) is added as a kept limit.
- Skipped findings and new limits go through triage as usual.

## Execution

Main session orchestrates. Task 1 goes to a `coder` subagent. The walks go to the VMs and the
Mac, then the change review loop. At release, this plan and `tab-switcher.md` move to
`archive/plans/` together, and the version is the owner's to name.

## Change review (2026-10-18)

Review passes 1-5 and the walks led to these fixes, each walked again:

- The hold lets the strip have a wheel only when it's cancelable and holds no Alt, Control or
  Cmd, so a pinch or a zoom wheel over the strip still stops the hold.
- The strip ignores a wheel it can't cancel, which would scroll the page anyway.
- The reveal keys on the active tab's id and path, so a link within the tab reveals it.
- Ctrl+Shift+Tab and the Open menu's Tab also match `keyCode` 9, for WebKitGTK's
  `Unidentified` Shift+Tab.
- Mac text: ⌥← / ⌥→, ⇧-Click and ⇧Enter (owner, extending Q9). README's duplicate sentence
  removed.
- On the Mac, keys holding Control and Cmd together go to the system (owner): Control+Cmd+F
  is full screen again, not find.
- The strip's wheel test leaves out its end padding (owner): with every tab in view, a wheel
  scrolls the page.

Triage (owner, 2026-10-18):

- The 19-37 px creep when wheeling the strip as a document restores: accepted with a trigger.
- The 125 % display case: emulated walk (whole-pixel layout only) plus a fractional-width probe;
  accepted with a trigger.
- A last tab cut by exactly 1 px reads as fitting, from the rounding slack: kept, no trigger.
- A restore a screen off after a page reload (909faa9 too): deferred.
- Short tab drags moving a slot fewer (Linux, 909faa9 too): deferred.
- The 2026-09-28 limit on Mac menu keys vs the page: its trigger fired; a Mac check of Cmd+H,
  M, A, C, V, X and Z with real keys passed, so it closed.
