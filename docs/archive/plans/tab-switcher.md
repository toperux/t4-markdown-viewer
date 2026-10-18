# Tab Switcher Plan

> **Status (2026-10-17):** executed, walked on all three platforms, change review and triage
> done (see **Change review** at the end). Not yet shipped.

**Spec:** the owner, 2026-10-17: "sometimes when there are many open tabs, I find it hard to
locate a certain document to select and switch to it", narrowed to "something I can search with
for switching".

**Goal:** Ctrl/Cmd+P, or a button at the end of the tab strip, opens a box under the bar. You type
part of a tab's file name, heading or folder; the list of open tabs narrows as you type; Enter or a
click switches to the one picked.

**Architecture:** frontend only (`src/index.html`, `src/base.css`, `src/app.js`, `README.md`). No
Rust change, no new dependency, no new permission.

## Current behaviour (read 2026-10-17; line numbers as of 8f69bc9)

- **Tabs are this window's alone:** `tabs` and `activeId` (`app.js:260-261`). Rust keeps a copy
  of each window's paths for the session file only.
- **A tab** (`makeTab`, `app.js:301-323`):
  - `label` is the file name; `path` is the current document's path.
  - `heading` is the document's first heading (`title_of`, `render.rs:499`). It is set **only
    when the tab is shown** (`showActive`, `app.js:2034`; `""` for a picture, `app.js:2004`).
  - `packTab` (`app.js:2318`) leaves `heading` out, so a tab restored from the session, or
    dragged in from another window, has no heading until it is first shown.
- **Switching:** `activateTab(id)` (`app.js:2263`) → `showActive` → `updateChrome`
  (`app.js:2071`), which every tab change reaches ("the funnel", its comment), and which calls
  `renderTabs` (`app.js:2107`) then `reportSoon`.
  - `activeId` changes first (`app.js:2258, 2265, 2276, 2305, 2348, 2373`); `updateChrome`
    runs only once the load ends (`app.js:2064`), and never for a render a newer switch superseded
    (`app.js:2024, 2028`). A slow share can take ~21 s (`app.js:2186`).
  - `renderTabs` also runs on every live reload of a file being rewritten (several times a
    second, `app.js:2230`) and on every drop-caret move (`app.js:2155-2158`).
- **The strip** `#tabs` (`index.html:66`, `base.css:1087-1096`):
  - It is a `role="tablist"` of `.tab` divs, rebuilt in full by every `renderTabs`.
  - Hidden with one tab (an early return in `renderTabs`, `app.js:2110-2114`), and scrolls
    sideways once the tabs reach their 5rem minimum.
  - The tab drag reads `els.tabs`' rect vertically only (`app.js:4061-4065`), and its slots from
    `.tab` children only (`insertionIndex`, `app.js:3972`).
- **Keys** (`onKeydown`, `app.js:4789`):
  - The reload, the find keys' suppression and Ctrl+Q come before the modal guard
    (`app.js:4839-4873`); the guard (`app.js:4878`) gives Settings, the update dialog and the
    diagram dialog the keyboard.
  - Escape order after the guard: open-menu, actor menus, then the find bar
    (`app.js:4884-4895`).
  - One capture `pointerdown` on `document` closes the menus on a press outside them
    (`app.js:5202-5215`).
  - `if (!ctrl || event.altKey) return;` (`app.js:4938`) keeps AltGr out; the key switch starts
    at `app.js:4977`.
  - **Ctrl+P is unbound.** What the engines do with it today is unmeasured: WebView2 most likely
    opens its print preview, as it opened its find bar on Ctrl+F (measured then). The macOS menu
    has no Print item (`main.rs:1619-1680`).
- **The find bar** (`#find`, `index.html:69-79`, `base.css:223-274`) hangs under the bar at the
  right, `position: absolute` in the sticky `#bar`. Its input's look (`base.css:245-266`) and
  `openFind`/`closeFind` focus handling (`app.js:3542`, `3563`) are the models here.
- **Menu rows** (`base.css:370-416`): transparent buttons; hover and focus draw an inset accent
  ring.

## Owner rulings (2026-10-17)

- **Q1. Scope: this window's tabs only.** No other windows, no sidebar files.
- **Q2. Key: Ctrl+P** (Cmd+P on macOS).
- **Q3. A button at the end of the tab strip** opens the same switcher.
- **Q4. With nothing typed, most recently used first.**

A-I were ruled as proposed (the recommended option in each); J was changed: matches are
highlighted.

- **A. Matching.** The query splits on spaces. Every word must appear, case-insensitively and in
  any order, in the tab's file name, heading or folder path. `\` and `/` are the same character,
  so `docs/readme` finds a Windows path. Plain substring: no fuzzy or out-of-order letters.
- **B. Ranking.** Tabs whose every word hits the file name or heading come before tabs that need
  the folder path. Within each group, most recently used first (Q4). So `api` puts `api.md` above
  `src/api/client.md`.
- **C. The current tab** is listed first on an empty query, marked "current". The second row is
  selected, so Ctrl+P, Enter flips back to the previous tab (VS Code's Ctrl+Tab does this). Once
  something is typed, the first match is selected, the current tab included.
- **D. One tab** open: Ctrl+P still opens, listing it. **No tab** (the empty screen): Ctrl+P does
  nothing.
- **E. Closing:** Enter or a click on a row (switching), Esc, a press anywhere outside it, or
  focus moving out (Tab). Not on the window losing focus: Alt+Tab away and back finds it open.
  Focus goes back to where it was before opening, as `closeFind` does, unless that element is
  gone (a rebuilt tab) or hidden.
- **F. Live while open:** any tab change (Ctrl+Tab, Ctrl+W, a file opened from elsewhere, a
  heading arriving) redraws the list. The selected tab stays selected while it is still listed;
  otherwise the pick falls back as on opening (C). A redraw doesn't scroll the list, so a list
  wheeled through stays put while a file reloads behind it.
- **G. Headings of tabs not shown yet are unknown** (restored or dragged in; see `packTab`). Such
  a tab is found by name and folder only, until it has been shown once. A closed accepted
  limit, no trigger (`docs/closed-items.md`). Not taken: saving `heading` in the session, which
  changes what each window reports and what Rust stores.
- **H. Under a dialog** (Settings, update, diagram), Ctrl+P is suppressed and opens nothing, like
  Ctrl+F.
- **I. The find bar and the switcher** may both be open: the switcher hangs centred (36rem), the
  find bar at the right (about 370 px). On a window narrower than about 1 350 px they overlap,
  which is most windows; the switcher covers the find bar (later in the DOM) until it closes.
  Esc closes the switcher first. Computed from the widths, not measured. The alternative is
  closing the find bar on Ctrl+P, which loses its highlights.
- **J. No cap on rows; matches highlighted.** The list scrolls past `60vh`. Every occurrence of
  every typed word is bold in the row's name, heading and folder lines. A word that runs from
  the folder into the name (`docs/readme`) is bold in both parts (ruled after pass 6).

## The change

### Markup (`index.html`)

- `#tabs` goes into a new row with the button, so the button stays put while the strip scrolls,
  and the `tablist` holds only tabs:

  ```html
  <div id="tab-row">
    <div id="tabs" role="tablist" hidden></div>
    <button id="tab-search" class="nav" type="button" title="Search tabs (Ctrl+P)"
            aria-label="Search tabs" aria-controls="switcher" aria-expanded="false">
      <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="4.25" /><path d="M10.2 10.2 L13.5 13.5" /></svg>
    </button>
  </div>
  ```

- After `#find`, inside `#bar`:

  ```html
  <!-- Tab switcher: Ctrl+P. Hangs under the bar, centred; see #switcher in base.css. -->
  <div id="switcher" hidden>
    <input id="switcher-input" type="text" role="combobox" aria-label="Search open tabs"
           placeholder="Search open tabs" aria-controls="switcher-list" aria-expanded="true"
           aria-autocomplete="list" spellcheck="false" autocomplete="off" />
    <ul id="switcher-list" role="listbox" aria-label="Open tabs" tabindex="-1"></ul>
    <p id="switcher-none" role="status"></p>
  </div>
  ```

  - `type="text"`, not `search`, for find's reason: Chromium clears a search field on Escape.
  - `tabindex="-1"` on the list: since Chromium 130 a scroll box with no focusable children is a
    Tab stop, and the list scrolls once it passes 60vh. Focus there would break E and the keys.
  - The "no match" line sits outside the `listbox`, which may hold only options. It is always
    in the page and only its text changes ("No open tab matches", or empty), so screen readers
    announce it: a live region shown by unhiding is announced unreliably.

### CSS (`base.css`)

- `#tab-row { display: flex; }`; `#tabs` gains `flex: 1; min-width: 0`.
  `#tabs[hidden] + #tab-search { display: none; }`, so with one tab the row is empty and
  takes no height. `#tab-search`: `flex: none`, `align-self: flex-start` (its own height, not
  the row's), `margin-top: 6px` (the strip's top padding,
  `base.css:1092`, so it lines up with the tabs, not the row; an overflowing strip's scrollbar
  adds height below) and a small right margin. The screenshot walk confirms the alignment.
  The bar's `:has(#tabs:not([hidden]))` glow rule (`base.css:1106`) is unchanged.
- `#switcher`: `position: absolute; top: 100%; left: 50%; transform: translateX(-50%); width:
  min(36rem, calc(100vw - 2rem)); z-index: 1;` the find bar's background, border and radius,
  plus the menus' shadow. No `display`, so the browser's own `[hidden]` rule hides it.
- `#switcher-input`: the find input's look (`base.css:245-266`), `width: 100%`.
- `#switcher-none`: `margin: 0`, muted, the rows' padding; `#switcher-none:empty { display:
  none; }` so the padding takes no room when there are matches.
- `#switcher-list`: no bullets, `position: relative`, `max-height: 60vh; overflow-y: auto;
  overscroll-behavior: contain` (a wheel at its end doesn't scroll the document). Each `li`:
  - the file name, then the heading muted, on one line; the folder path below it, muted and
    smaller; each line ellipsised, with `title` holding the full path;
  - `.current` adds a muted "current" after the name;
  - `#switcher-list mark { background: none; color: inherit; font-weight: 600; }` (J). Bold
    only, no colour: an accent-coloured letter on `--ui-bg` isn't contrast-checked in every
    theme;
  - `[aria-selected="true"]` gets the menus' inset accent ring and a faint accent tint; hover
    gets the ring alone.

### Script (`app.js`)

State, beside `tabs`:

```js
// Tab switcher (docs/plans/tab-switcher.md).
let useClock = 0; // stamps tab.used, most recent highest
let switcherShown = []; // the tabs the list shows, in its order
let switcherPick = -1; // index into switcherShown
let switcherReturn = null; // what had focus before it opened
```

- `makeTab` gains `used: 0`. `updateChrome` stamps the active tab first thing:
  `if (tab) tab.used = ++useClock;`. Every finished activation reaches it (the error path
  included), so the stamps order tabs by when each was last shown. A restored session starts
  with every stamp 0 but the active tab's, so the rest fall back to strip order (the sort is
  stable).
- `switcherMatches(list, query, activeId)`, pure, so the walk can test it. It sorts a copy
  (`[...list]`), never `tabs` itself, which is the strip's order:
  1. no words: the `activeId` tab first, explicitly, then the rest by `used` descending. Not by
     stamp alone: during a slow load the active tab isn't stamped yet (Current behaviour);
  2. else keep tabs where every word matches in `label`, `heading` or `path` (A); sort stable by
     group (B), then `used` descending.
- **The words as patterns** (`switcherWords(query)`): split on whitespace, drop empties; each
  word becomes `new RegExp(word.split(/[\\/]/).map(literal).join("[\\\\/]"), "giu")` (A): split
  on the separators first, escape each piece, join with the class `[\\/]`. Not escape-then-
  rewrite: the escape already turns `/` into `\/`, which would then become two classes.
  `literal` is lifted out of `findPattern` (`app.js:3324`) so both share it. Every escape it
  makes is valid under `u`, and a word is never empty, so no query throws or matches empty.
  Matching runs on the original strings, not lowercased copies: `toLowerCase` can change a
  string's length (`İ` becomes two characters), which would shift J's highlight offsets. The
  `iu` flags fold case without that, as find relies on.
- **No `lastIndex` state:** a word is tested with `str.search(re) !== -1`, which ignores the `g`
  flag, and highlighted with `str.matchAll(re)`, which works on a copy of the pattern.
- **Highlighting** (J), two helpers:
  - `matchRanges(text, words)`: every match of every word in `text` gives a range; ranges are
    sorted, and overlapping or touching ones merged (`start <= previous end`).
  - `markMatches(el, text, ranges)`: `el` gets text nodes and `<mark>` elements for the
    ranges, built with `createTextNode` and `textContent`. Never `innerHTML`: a file name can
    hold `<`.
- **Which text gets which marks** (J, and the owner's ruling on words that cross the folder
  boundary: bold the parts):
  - The heading line: matches in `heading`.
  - The name and folder lines: matches in the **whole `path`**, cut at the line boundaries, so
    `docs/readme` bolds `docs` in the folder line and `README` in the name line. The separator
    between them isn't shown, so its part of a mark is dropped.
  - Both lines are true slices of `path`, so a path range maps by clipping and shifting:
    - `cut = path.length - label.length`: where the name starts. `label` is always
      `baseName(tab.path)` (set together, `app.js:2001-2003`, `2030-2033`, `makeTab`), and a
      tab's path never ends in a separator, so it is `path`'s suffix.
    - The folder line is `path.slice(0, cut)` minus its trailing separator, kept when what is
      left is a root: empty (`/`), a drive (`C:`) or a long-path drive (`\\?\C:`), tested with
      `/^(|[a-z]:|\\\\\?\\[a-z]:)$/i`. Not just "ends in `:`": a Unix folder may be named
      `a:`.
    - Name ranges: each path range clipped to `[cut, path.length)`, minus `cut`. Folder
      ranges: clipped to `[0, folder.length)`.
  - Not `dirName` (`app.js:224`): for a drive root it writes `C:\` even after a `C:/`, which
    is then no slice of `path`. Not `tab.dir`, which is `""` until the tab is first shown.
  - So the rows call `matchRanges` once on `path` and once on `heading`, then `markMatches` per
    line.
- `renderSwitcher(keep)`: with no tabs left (Ctrl+W on the last), `closeSwitcher()` and return
  (D). `switcherShown = switcherMatches(…)`. The pick: with `keep` (a redraw for F), the same
  tab id if it is still shown; otherwise (opening, typing, or the kept tab gone: C) 1 on an
  empty query with 2+ rows, else 0, else −1 when nothing is shown. Builds the `li`s
  (`id="switcher-opt-<tabId>"`, `role="option"`, `aria-selected`), sets the input's
  `aria-activedescendant`, and sets `#switcher-none`'s text: "No open tab matches" when
  nothing is shown, else empty. Without `keep`, it brings the picked row into view.
- **Bringing a row into view** (opening, typing, ↑/↓; never a `keep` redraw): set the list's
  `scrollTop` by hand from the row's `offsetTop` and height. The list has `position: relative`,
  so `offsetTop` is measured from it, not from `#switcher` (which would add the input's
  height). Not `scrollIntoView`, which can also scroll the document when the box runs past the
  bottom of a short window.
- `openSwitcher()`: with no tabs, return (D). In `openFind`'s order (`app.js:3546`, then
  `3558`): if hidden, `switcherReturn = document.activeElement` (the open-menu's button if focus
  was in that menu), clear the input, show, set `#tab-search`'s `aria-expanded`; then
  `showOpenMenu(false)`, `renderSwitcher(false)`, focus and select the input.
- `closeSwitcher(restore = true)`: return at once if already hidden (the focus restore below
  fires `focusout`, which would close again). As `closeFind` does (`app.js:3565`, `3580`): read
  whether focus is inside **before** hiding; hide, `aria-expanded` false; then, if it was
  inside and `restore`, blur the input and put focus back on `switcherReturn` when connected
  and visible (`getClientRects().length`), with `preventScroll`. The `focusout` rule passes
  `restore = false`: focus is already going where Tab sent it.
- `pickSwitcher(i)`: close, then `activateTab(switcherShown[i].id).catch(toast)`.
- `renderTabs` starts with `if (!els.switcher.hidden) renderSwitcher(true);` (F). First, not
  last: the one-tab path returns early.
- `els` gains `tabSearch`, `switcher`, `switcherInput`, `switcherList`.

Keys and listeners:

- **Suppression** joins find's block before the modal guard (`app.js:4857-4863`): add
  `k === "p" || event.keyCode === 80` to its `ctrl && !event.altKey` clause. So no print dialog
  under a dialog either (H).
- **Esc** after the guard, right after the open-menu's block (`app.js:4888`), before the actor
  menus' (`app.js:4889`) and the find bar's: if the switcher is open and the key isn't
  composing, `closeSwitcher()` and return (I). The open-menu can't be up with it (`openSwitcher`
  closes it); an actor popup opened before Ctrl+P would otherwise take the first Esc.
- **Ctrl+P** in the key switch (`app.js:4977` on): `(key === "p" || event.keyCode === 80) &&
  !event.shiftKey` → `openSwitcher()`. No Shift, so Ctrl+Shift+P stays free, and a Cyrillic
  Ctrl+Shift+З (keyCode 80) doesn't open it.
- **The input's `keydown`** (stopped once handled, so `onKeydown` doesn't see it; ignored while
  composing, `keyCode` 229 too):
  - ↓ / ↑: with nothing shown, do nothing (the wrap would be `% 0`); else move the pick, round
    the ends, and bring it into view; `preventDefault` (no caret move);
  - Enter: `preventDefault`, then `pickSwitcher(switcherPick)` when there is one. Without
    `preventDefault`, the focus restore inside the keydown hands the Enter's activation to the
    element focus went back to (reasoned: a button would be clicked);
  - Ctrl+W, Ctrl+Tab and the rest reach `onKeydown` unchanged, and F redraws the list.
- **`input`**: `renderSwitcher(false)` at once. No debounce: a few dozen tabs, no layout reads.
- **The list**: `mousedown` with `preventDefault`, so the input keeps focus (`pointerdown`'s
  `preventDefault` doesn't stop the focus change in Chromium); then `click` on an option →
  `pickSwitcher` of its index. The walk checks the list's scrollbar still drags.
- **`#tab-search`**: `mousedown` with `preventDefault`, so a click never focuses it, on any
  engine; `click` toggles (`openSwitcher` or `closeSwitcher`). Still a Tab stop for the
  keyboard.
- **Closing from outside** (E), each only while the switcher is open:
  - a press outside `#switcher` and `#tab-search`: a branch in the existing capture
    `pointerdown` (`app.js:5202-5215`), shaped like the sort menu's
    `closest("#sort-menu, #sidebar-sort")` check. The button is left out so the press doesn't
    close it only for the click to reopen it.
  - `focusout` on `#switcher` with a `relatedTarget` outside it: `closeSwitcher(false)`.
    Shift+Tab onto the button closes too. A null `relatedTarget` (the window losing focus, a
    click on nothing focusable) leaves it open; the press rule covers the click.
- **Suppressing with `ctrl && !altKey`** also catches Ctrl+Shift+P, which then does nothing
  (WebView2's default for it, if any, is lost; none is known).

### README

- Shortcut table, after `Ctrl+Tab`: `Ctrl+P` | Search the open tabs and switch to one.
- The tabs bullet (`README.md:37-40`): "With many tabs open, `Ctrl+P` or the magnifier at the
  end of the strip searches them by name, heading or folder."

## Risks

- **R1. Ctrl+P's default per engine** is unmeasured. Expected: WebView2 opens print preview
  unless suppressed; WebKitGTK and WKWebView do nothing. The walk presses it with a dialog open
  and closed on each.
- **R2. A centred absolute box** with `transform` inside a sticky bar: expected fine (find hangs
  the same way without transform). The walk screenshots it at 420 px and at full width.
- **R3. Clicks on WebKit:** a click there may not focus what it lands on (macOS), so
  `relatedTarget` is null and only the press rule closes. Covered by design (the button never
  takes focus on a click anywhere), walked on the Mac.
- **R4. `#tab-row`** changes the bar's structure. `measureBar` reads the bar's height, so it
  follows. The tab drag reads `#tabs`' rect, which keeps its height; the button takes about 2rem
  of strip width.

## Tasks

### Task 1: the switcher

- [ ] Markup, CSS, script, README rows as above.
- [ ] `node --check src/app.js`; `cargo test` (nothing in Rust changes, but a test reads
  `index.html`'s ids, `render.rs:586-617`).
- [ ] Commit: "Search the open tabs and switch to one with Ctrl+P".
- [ ] `docs/closed-items.md` › Accepted limits: G, "*Kept 2026-10-17, owner's call, no
  trigger.*" A `docs:` commit.

### Task 2: walks

On the Windows VM, the Linux VM and the Mac (drive-app skill; never the owner's PC), from a
`walk/tab-switcher` branch:

- `switcherMatches` by `eval` on made-up tabs:

  | Tabs (label, heading, path) | Query | Expected order |
  |---|---|---|
  | a.md used 3, b.md used 5, c.md used 1, active c | "" | c, b, a |
  | the same, `tabs` read again afterwards | | strip order unchanged |
  | `api.md`; `client.md` in `/src/api/` | `api` | api.md, client.md |
  | `README.md` in `C:\x\docs` | `docs/readme` | it |
  | `notes.md`, heading "Release plan" | `plan rel` | it |
  | anything | `zzz` | none |
  | `a.b.md`, `axb.md` | `a.b` | a.b.md only (no regex meaning) |

- The row's lines for `C:\x\docs\README.md` with `docs/readme`: folder `C:\x\docs` with
  `docs` bold, name `README.md` with `README` bold. `C:\a.md` with `a`: folder `C:\`, nothing
  bold in it; name `a.md` with `a` bold. `/a.md` likewise with folder `/`. `/x/a:/f.md`:
  folder `/x/a:`.
- `markMatches` by `eval` on a detached `span`: `"İstanbul.md"` with `stan` bolds exactly
  `stan`; `"Read me README"` with `read me` gives three marks: `Read`, `me`, and
  `README` (its touching `READ` and `ME` ranges merged into one); a name `"<b>x</b>.md"`
  shows as text.

- Real keys (`sendkeys.ps1` on Windows): Ctrl+P opens, no print preview (R1); with Settings open
  nothing opens and no print preview; Ctrl+P, Enter flips to the previous tab; type, ↓, Enter;
  Esc returns focus; click outside closes; Alt+Tab away and back keeps it open; Ctrl+W while open
  redraws; Ctrl+Shift+P does nothing; the button opens and closes it; with the find bar open, Esc
  closes the switcher first.
- By keyboard: Tab to the button, Enter opens, Enter picks: the switch happens and the
  switcher stays closed. A mouse click on the button leaves focus where it was. Shift+Tab from
  the input closes it, and focus stays on the button.
- 30 tabs: the list scrolls and the picked row stays in view; Tab from the input closes it (no
  stop on the list); the list's scrollbar drags; a live reload behind it doesn't move the list;
  the button stays at the strip's end while it scrolls; a tab drag still reorders and tears off.
- Screenshots at 420 px and full width, a light and a dark theme (R2).
- The Mac: Cmd+P; a click on a row and on the button (R3).

## Execution

Main session orchestrates. Task 1 goes to a `coder` subagent; the walks go to the VMs and the
Mac. Then the change review loop (CLAUDE.md step 5). Packaging: one feature commit plus this plan
as a `docs:` commit.

## Change review

Walks ran from the temp branch `walk/tab-switcher` (WebView2 155 on the Windows VM, WebKitGTK
2.52.6 on the Linux VM under Wayland and Xvfb, macOS 26.7 WKWebView). Task 1 passed all nine
checks on each. Ctrl/Cmd+P shows no print preview or dialog on any of them.

Fixed after the walks and two code-review passes:

- **Activate guard:** a click on a tab closed a moment before (the strip and list redraw only
  once the next tab has loaded) blanked the window. `activateTab` now ignores a tab that is gone,
  which also covers the strip, where it was older.
- **O1 (owner):** matches are underlined as well as bold; bold alone barely showed on
  near-white names in dark themes.
- **N1:** the list starts from the top on opening and when the query changes.
- **N2 (owner):** folder lines are cut at the start. A CSS right-to-left trick worked on
  Chromium, but WebKit painted its "…" over the text, so it is cut in JS:
  - `cutStart`, sized on a canvas at the text's own weight plus each mark's bold;
  - the rows are filled before the width is read, so a scrollbar that appears is counted;
  - the cut is searched among grapheme boundaries only (`Intl.Segmenter`), so accents and
    emoji stay whole, and a joined emoji that fits isn't dropped;
  - `text-overflow: clip`, so no second "…".
- **N3 (owner):** hover is a neutral tint; only the pick has the accent ring.
- **Redraws while open** keep the list's scroll and pick. One that would change nothing (a file
  rewritten behind it) is skipped, so a click can't straddle a swap.
- **Triage 2 (owner):** on macOS only Cmd+P opens it; Control+P stays the text box's.
- **Triage 7 (owner):** the magnifier is centred on the tabs.

Triage (owner, 2026-10-17):

- **Accepted, no record:** none.
- **Closed accepted limits** (`docs/closed-items.md`):
  - G;
  - stale background headings;
  - the folder line's 1 px margin;
  - matches in a cut part;
  - one-letter words;
  - the resize cost;
  - a held Enter.
- **Deferred** (`docs/open-items.md`):
  - Wayland's heavier text;
  - three items to plan right after this ships: the Linux strip clicks, the strip not
    revealing the active tab (with the wheel), and Control-key shortcuts on the Mac.
- **At archiving** (owner): the new entries in both records cite `plans/tab-switcher.md`; point
  them at `archive/plans/tab-switcher.md` in the commit that moves this plan.

The triage fixes went through a final review pass (clean, after its one nit was fixed) and a
short walk of the last tip on all three platforms.
