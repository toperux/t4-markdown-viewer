# Two Cheap Deferred Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep Settings' Done button in view on a short window, and redraw Mermaid diagrams when the OS switches between light and dark under a theme that follows it.

**Architecture:** Frontend only; Rust is unchanged. Task 1 is CSS: the Settings footer becomes `position: sticky` at the bottom of the dialog's own scroll box, plain, with no divider (owner's call). Task 2 is one `matchMedia` listener in `main()`: it calls the existing `redrawDiagrams()`, which already skips figures drawn in the current look, and it refreshes `--page-bg`, the page colour the active tab, the top fade and the diagram overlay wear.

**Tech Stack:** plain JS in `src/app.js`, CSS in `src/base.css`, no bundler, no JS test harness. The `drive-app` skill (CDP against the debug build) is the only way to check the frontend.

**Spec:** the two open-items entries, `docs/open-items.md` → `## Deferred`:
- *Settings' Done button sits below the fold on a short window.* At an 860 px window height the dialog scrolls and Done is out of sight; Escape still closes it. Fix: keep the footer in view.
- *An OS light/dark flip doesn't redraw diagrams under a see-through theme.* `diagramLook` reads the page background composited on the dialog's `Canvas`, which follows the OS under `color-scheme: light dark` with a transparent body. Only a theme change triggers a redraw. Fix: a `matchMedia("(prefers-color-scheme: dark)")` change listener that calls `redrawDiagrams()`.

  Found while planning, same cause: a theme may also restyle its body inside `@media (prefers-color-scheme: dark)` (`src-tauri/themes/README.md`: "a light theme that also adapts to the OS"). Then an OS flip changes an opaque page background too, and `--page-bg`, set only in `applyTheme`, goes stale. The same listener covers both.

**Measured before planning was final** (debug build, 2026-09-27):
- Settings, by window size:

  | Window | Dialog scrolls | Done's bottom | Dialog's bottom |
  |---|---|---|---|
  | 1080 | no | — | — |
  | 860 | yes | 1002 | 804 |
  | 640 | yes | 1002 | 584 |
  | 320 | yes | 1002 | 264 |

  The UA gives the modal `max-height: calc(100% - 34px)` and `overflow: auto`.
- OS flip: `Emulation.setEmulatedMedia` over CDP flips `Canvas` under `color-scheme: light dark` (the dialog read `rgb(18, 18, 18)`). A `change` event fires only on a real transition. The host OS here is dark.

## Outcome (2026-09-27)

Done: ce940ff (Task 1, Settings footer) and e0dd026 (Task 2, OS light/dark). Closed in
`closed-items.md` under Deferred. A review loop afterwards added two changes, squashed into
e0dd026: the overlay closes at the switch, not once the redraw lands; and a diagram clicked in
a look the page no longer has is redrawn instead of opening in the old look, and the next
click opens it, so a user theme whose colours move without a theme change cannot leave it
unclickable.

- Task 1's own before-fix run measured Done's bottom at 960 at 860 px, which matches the
  1080 baseline's content height; the table's 1002 came from another state of the dialog
  (likely the update row's text) and was not pinned down.

- The app's real minimum, measured by `drag-corner.ps1`, is 420×320: Done's bottom at 286 is
  in view of the dialog's 303.
- Found while running it: `drag-corner.ps1`'s first run once didn't take and printed its
  800×600 start instead; a rerun gave 420×320.
- Found during plan review, same cause as the found-while-planning note above: the host OS
  being dark meant a plain flip to dark was no transition, so Task 2's checks step through
  `light,dark` rather than trusting the OS's own current side.

## Global Constraints

- No new dependencies, no Rust changes.
- Commit straight to `main`; never push (the owner pushes).
- Commit subjects: these are fixes to shipped behaviour, so plain subjects (they become release-note bullets); docs commits are prefixed `docs:`.
- Commit trailer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Drive the app only through `.claude/skills/drive-app/SKILL.md`: the audit identifier build, CDP on port 9223, back up and restore `%APPDATA%\t4-markdown-viewer\config.json` and `session.json`, kill only the debug build you launched, finish with a plain `cargo build`. Switch themes with `eval "applyTheme('<name>')"`, which does not save; the picker and F8 write `config.json`.
- Relaunching within a task:
  - Back up `config.json` and `session.json` once per task, before its first launch. A relaunch is `drive-app` §2 only; backing up again would save the test config over the user's.
  - After every relaunch, re-read the debug PID (`Get-Process t4-markdown-viewer | select Id,Path`, the one under `target\debug`) and its handle (`(Get-Process -Id <pid>).MainWindowHandle`). Never reuse an old one: `drag-corner.ps1` given a dead handle drags at the screen's corner.
- Keep the app window visible (not minimised or covered) while flipping media: media-query `change` events fire on a rendering update, and WebView2 stops rendering hidden windows.
- Match the file's comment style: comments say why, in full sentences, as in the surrounding code.

## Review Focus

1. **Settings at a normal height looks exactly as before.** No new line, gap or background band above Done when nothing scrolls → Task 1, Step 4 check (b).
2. **Keyboard focus never lands under the pinned footer.** Tabbing through a scrolling dialog shows each control → Task 1, Step 4 check (d).
3. **Content scrolling under the pinned footer is hidden, in a dark theme too.** The footer's background is the dialog's own `--ui-bg` → Task 1, Step 4 check (c).
4. **An OS flip with nothing to redraw costs nothing and throws nothing.** A picture tab, the empty screen → Task 2, Step 4 check (c).
5. **An OS flip while the overlay is open** closes it once its drawing is out of date, as a theme change does → Task 2, Step 4 check (b).

Known and accepted: a user theme whose `--ui-bg` is see-through would show a darker band where the footer overlaps the dialog. All 15 bundled themes use an opaque hex.

---

### Task 1: Keep Settings' Done button in view

**Files:**
- Modify: `src/base.css:562-571` (`#settings-dialog`) and `src/base.css:676-680` (`#settings-actions`)

**Interfaces:**
- Consumes: the UA's modal-dialog box (`els.settings.showModal()`, `app.js:3565`): `max-height` and `overflow: auto`, so `#settings-dialog` is its own scroll container. The footer is the `<form>`'s last child (`index.html:218`), and the form (`padding: 1.25rem 1.5rem 1rem`, `base.css:582-584`) is all of the scrolled content, so the sticky footer can travel its full height.
- Produces: nothing other tasks use.

- [x] **Step 1: Reproduce.** Build and launch per `drive-app`, open `examples/kitchen-sink.md`, and resize the window to 1200×860 with Win32 `MoveWindow` (SKILL.md, *Window size*). The handle for this and for `drag-corner.ps1` is `(Get-Process -Id <debug pid>).MainWindowHandle`: by PID, since your installed viewer shares the process name. Open Settings with `node .claude/skills/drive-app/scripts/cdp.mjs 9223 click "#settings-btn"`, then:

```bash
node .claude/skills/drive-app/scripts/cdp.mjs 9223 eval "(() => { const d = document.getElementById('settings-dialog'); const b = document.querySelector('#settings-actions button'); const r = d.getBoundingClientRect(), s = b.getBoundingClientRect(); return { inner: innerHeight, scrolls: d.scrollHeight > d.clientHeight, doneBottom: Math.round(s.bottom), dialogBottom: Math.round(r.bottom), dialogH: Math.round(r.height) }; })()"
```

Expected (measured): `scrolls: true`, `doneBottom` 1002 > `dialogBottom` 804. Also resize to 1200×1080 and record `dialogH` (measured: 960) as the baseline for Step 4 (b).

- [x] **Step 2: Pin the footer.** Replace the `#settings-actions` rule:

```css
/*
 * Pinned to the foot of the dialog's own scroll box: on a short window the
 * dialog scrolls, and the one way out should not scroll away with it. The
 * negative margins take it out to the dialog's edges, so what scrolls under it
 * is covered, and its padding puts the spacing back where it was.
 */
#settings-actions {
  position: sticky;
  bottom: 0;
  display: flex;
  justify-content: flex-end;
  margin: 0.5rem -1.5rem -1rem;
  padding: 0.75rem 1.5rem 1rem;
  background: var(--ui-bg);
}
```

The spacing stays the same: the old `margin-top: 1.25rem` becomes `0.5rem` of margin plus `0.75rem` of padding. The form's bottom padding (`1rem`) moves into the footer through `margin-bottom: -1rem; padding-bottom: 1rem`. Sideways, `-1.5rem` of margin and `1.5rem` of padding cancel out.

Then add to the `#settings-dialog` rule, after `box-shadow`:

```css
  /* Focus scrolls a control into view only when it is out of sight; the
     pinned footer hides the bottom of the box, so take its height off what
     counts as in sight. */
  scroll-padding-bottom: 4rem;
```

- [x] **Step 3: Rebuild.** Kill the debug build you launched (`taskkill //PID <pid>`; Windows can't overwrite a running exe). Rebuild with the `TAURI_CONFIG` audit-identifier command (assets are embedded at build time). Relaunch per `drive-app` with `examples/kitchen-sink.md`, and open Settings (`cdp.mjs click "#settings-btn"`). Reopen it the same way after any check below that closes it.

- [x] **Step 4: Verify.** In a light and a dark theme (`eval "applyTheme('github-light')"`, then `'github-dark'`):
  - (a) At 1200×860 (`MoveWindow`) and at the app's real minimum (drag the corner with `drag-corner.ps1 -Hwnd <handle>`, which prints the client size left; `MoveWindow` ignores the minimum), run the Step 1 eval. Expected: `doneBottom <= dialogBottom` at both. Then `cdp.mjs click "#settings-actions button"` closes the dialog: `eval "document.getElementById('settings-dialog').open"` → `false`.
  - (b) At 1200×1080, `dialogH` equals the Step 1 baseline (±1 px), and a `shot` of the dialog shows no band or line above Done.
  - (c) At the minimum size (drag the corner again after (b)), with the dialog scrolled half-way (`eval "(d => (d.scrollTop = (d.scrollHeight - d.clientHeight) / 2, d.scrollTop))(document.getElementById('settings-dialog'))"`), a `shot` shows no fieldset text behind or through the footer.
  - (d) At the minimum size, close Settings (`cdp.mjs click "#settings-actions button"`) and reopen it (`click "#settings-btn"`), so the walk starts from a fresh dialog. Then press Tab (`cdp.mjs key Tab 9`; 9 is the key code) until Done is focused, about six stops. After each Tab, check the focused control lies above the footer:

    ```bash
    node .claude/skills/drive-app/scripts/cdp.mjs 9223 eval "(() => { const f = document.activeElement.getBoundingClientRect(), a = document.getElementById('settings-actions').getBoundingClientRect(); return { el: document.activeElement.id || document.activeElement.value || document.activeElement.tagName, clear: f.bottom <= a.top || document.activeElement.closest('#settings-actions') !== null }; })()"
    ```

    Expected: `clear: true` for every control up to Done.

- [x] **Step 5: Clean up** per `drive-app`: kill the debug build you launched, restore `config.json` and `session.json` from the backups, and run a plain `cargo build` to put the real identifier back. Each task runs its own launch-to-cleanup cycle, so Task 2 starts clean.

- [x] **Step 6: Commit.**

```bash
git -C "<repo>" add src/base.css
git -C "<repo>" commit -m "Keep Settings' Done button in view on a short window" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Redraw diagrams when the OS switches light and dark

**Files:**
- Modify: `src/app.js`, in `main()`, right after the `#image-view` `scroll` listener (about `app.js:3826-3833`; before the first `await` in `main()`, so it is always registered).
- Modify: `src-tauri/themes/README.md:101-102`, one sentence.

**Interfaces:**
- Consumes (all existing):
  - `redrawDiagrams(): Promise<void>` (`app.js:624`). It returns early mid-switch, skips figures already drawn in the current look, and ends with `closeStaleDiagram()`.
  - `diagramFigures(): Element[]` (`app.js:584`).
  - `els.content`, and `--page-bg` as `applyTheme` writes it (`app.js:2933`): `getComputedStyle(document.body).backgroundColor` on the root's inline style. It stays unset until a theme has loaded, and `.tab.active` falls back when it is unset (`base.css:893-894`).
- Produces: nothing other tasks use.

- [x] **Step 1: Write the check script** to `<scratchpad>/flip.mjs`, not the repo, and call it by that absolute path. Emulated media lasts only while its CDP connection stays open, so one script sets it, waits, reads, and resets. A `change` event fires only on a real transition, and the host OS may already be dark (it was when this plan was measured). So the script steps through a list of schemes, `light,dark`, and reads after each one, which gives a real flip whatever the OS is set to:

```js
// flip.mjs <port> <scheme,scheme,…> "<expr to read after each>"
const [port, schemes, expr] = process.argv.slice(2);
const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = pages.find((p) => p.type === "page" && p.url.includes("tauri"));
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r, { once: true }));
let id = 0;
const send = (method, params) =>
  new Promise((resolve) => {
    const my = ++id;
    ws.addEventListener("message", function on(e) {
      const msg = JSON.parse(e.data);
      if (msg.id !== my) return;
      ws.removeEventListener("message", on);
      resolve(msg.result ?? msg.error);
    });
    ws.send(JSON.stringify({ id: my, method, params }));
  });
for (const scheme of schemes.split(",")) {
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: scheme }] });
  await new Promise((r) => setTimeout(r, 3000)); // the redraw warms through mermaid
  const r = await send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
  console.log(scheme, JSON.stringify(r.exceptionDetails ?? r.result?.value ?? r));
}
await send("Emulation.setEmulatedMedia", { features: [] });
ws.close();
```

The expression each check reads (call it `LOOK`): `({ look: diagramFigures().map((f) => diagramBlocks.get(f).look.split('\u0000')[0]), pageBg: document.documentElement.style.getPropertyValue('--page-bg') })`.

- [x] **Step 2: Reproduce, with two test themes.** Start clean: kill any debug build still running (`taskkill //PID <pid>`), back up `config.json` and `session.json`, build with the `TAURI_CONFIG` audit-identifier command, and launch per `drive-app` with `examples/kitchen-sink.md` open and `github-light` applied. Keep the window in front: `eval "document.visibilityState"` must return `"visible"` before any `flip.mjs` run. If it doesn't, bring the window forward as `drag-corner.ps1` does (`SendKeys('%')`, then `SetForegroundWindow` on the handle). Record the host's side with `eval "matchMedia('(prefers-color-scheme: dark)').matches"`, and scroll the diagram into view (`eval "diagramFigures()[0].scrollIntoView({block:'center'})"`). For each test theme, first `eval "applyTheme('github-light')"` to start clean. Then append the theme to the live stylesheet, so the user's theme files are never touched, and draw in the host's look:

```bash
# (i) the spec's case: see-through page, colours from the OS
node .claude/skills/drive-app/scripts/cdp.mjs 9223 eval "(async () => { els.themeStyle.textContent += ':root{color-scheme:light dark} body{background:transparent}'; await redrawDiagrams(); return 1; })()"
# (ii) found while planning: an opaque page restyled under @media
node .claude/skills/drive-app/scripts/cdp.mjs 9223 eval "(async () => { els.themeStyle.textContent += ':root{color-scheme:light dark} body{background:#fff} @media (prefers-color-scheme: dark){body{background:#111}}'; await redrawDiagrams(); return 1; })()"
```

After each, run `node <scratchpad>/flip.mjs 9223 light,dark "<LOOK>"`.

Expected before the fix: under both themes the look never changes. It reads the host's side after `light` and after `dark` alike, and one of those two reads is stale.

- [x] **Step 3: Add the listener** in `main()`, right after the `#image-view` scroll listener:

```js
  // A theme can follow the OS — `color-scheme: light dark` over a see-through
  // page, or rules of its own under `prefers-color-scheme` — and then a flip
  // changes the page's side with no theme change to redraw its diagrams, or to
  // recolour what wears the page colour. `redrawDiagrams` skips any already
  // drawn in the look the page now has. `--page-bg` is refreshed only once a
  // theme has set it; until then the active tab and the overlay use fallbacks
  // of their own.
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    const root = document.documentElement.style;
    if (root.getPropertyValue("--page-bg")) root.setProperty("--page-bg", getComputedStyle(document.body).backgroundColor);
    if (!els.content.hidden && diagramFigures().length > 0) redrawDiagrams().catch(console.error);
  });
```

In `src-tauri/themes/README.md`, after the sentence ending "a light theme that also adapts to the OS stays light.", add:

```markdown
Its diagrams, the active tab and the page's top fade still follow the page when
the OS switches: they are redrawn and recoloured from what the page shows.
```

- [x] **Step 4: Rebuild, relaunch, verify.** Kill the debug build you launched (`taskkill //PID <pid>`), rebuild with the `TAURI_CONFIG` audit-identifier command, relaunch per `drive-app` with `examples/kitchen-sink.md`, then scroll the diagram into view and check `document.visibilityState` is `"visible"` (Step 2's checks; not its backup). Then install an error trap, which also catches the listener's `.catch(console.error)`:

```bash
node .claude/skills/drive-app/scripts/cdp.mjs 9223 eval "(() => { window.__errs = []; const log = console.error; console.error = (...a) => { __errs.push(a.map(String).join(' ')); log(...a); }; addEventListener('error', (e) => __errs.push(String(e.message))); addEventListener('unhandledrejection', (e) => __errs.push(String(e.reason))); return 1; })()"
```

  - (a) Repeat Step 2 from `applyTheme('github-light')` on, without relaunching (a relaunch loses the trap). Expected:
    - Theme (i): `light` reads `look: ["false"]` and `dark` reads `["true"]`. `pageBg` reads `rgba(0, 0, 0, 0)` after each flip (before the first, it still holds github-light's white: the appended rules never go through `applyTheme`).
    - Theme (ii): the same looks. `pageBg` reads `rgb(255, 255, 255)`, then `rgb(17, 17, 17)`.
    - On a light host only: after the script exits and clears the emulation, a plain `cdp.mjs eval "<LOOK>"` about 3 s later reads `["false"]` again. On a dark host the clear is no transition, so skip this.
  - (b) Theme (ii) in place and the diagram in view. Open the overlay with `cdp.mjs eval "(openDiagram(diagramFigures()[0]), els.diagramDialog.open)"`, which should return `true`. Then run `flip.mjs 9223 light,dark "els.diagramDialog.open"`. Expected: `false` by the first read whose side differs from the drawing's.
  - (c) Nothing to draw. With a picture tab active (`eval "openTab('<repo, forward slashes>/src-tauri/icons/32x32.png')"`), and then on the empty screen (`eval "(async () => { for (const t of [...tabs]) await closeTab(t.id); return tabs.length; })()"`, which returns `0`), run `flip.mjs 9223 light,dark "__errs"`. Expected: `[]` on every read, in both runs.
  - (d) An opaque theme that ignores the OS. After (c) the window is empty, so apply the theme first and then open the document; that also keeps the theme change's own redraw from replacing the tagged drawing: `eval "(async () => { await applyTheme('github-light'); await openTab('<repo, forward slashes>/examples/kitchen-sink.md'); diagramFigures()[0].firstElementChild.dataset.probe = 1; return 1; })()"`. Then run `flip.mjs 9223 light,dark "[<LOOK>, diagramFigures()[0].firstElementChild.dataset.probe]"`. Expected: the look stays `["false"]`, and the probe is still `"1"`, so the drawing was not replaced.
  - (e) `__errs` is `[]` at the end.

- [x] **Step 5: Clean up** per `drive-app`: kill the debug build you launched, restore `config.json` and `session.json` from the backups, and run a plain `cargo build`.

- [x] **Step 6: Commit.**

```bash
git -C "<repo>" add src/app.js src-tauri/themes/README.md
git -C "<repo>" commit -m "Redraw diagrams when the OS switches light and dark" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Close the two items

**Files:**
- Modify: `docs/open-items.md`: remove the two entries from `## Deferred`, and extend "Mermaid diagrams on macOS and Linux".
- Modify: `docs/closed-items.md`: add both entries, ticked, under `## Deferred`, each with a `*Fixed <hash>, <date> …*` note.
- Move: `docs/plans/cheap-deferred-fixes.md` → `docs/archive/plans/cheap-deferred-fixes.md`, with an `## Outcome` section. The plan is untracked, so use a plain `mv`, not `git mv`.

- [x] **Step 1:** Move each entry's text verbatim from open-items to closed-items `## Deferred`, change `- [ ]` to `- [x]`, and append the note. For the OS-flip entry, the note also says the listener covers themes with their own `prefers-color-scheme` rules and refreshes `--page-bg`.
- [x] **Step 2:** In open-items' "Mermaid diagrams on macOS and Linux", extend the checklist sentence with: "…and the light/dark toggle redraws it, as does an OS light/dark switch under a theme that follows the OS."
- [x] **Step 3:** Tick this plan's boxes, add the Outcome (commits, the minimum size `drag-corner.ps1` reported, anything found), and `mv` the plan to `docs/archive/plans/`.
- [x] **Step 4: Commit.**

```bash
git -C "<repo>" add docs
git -C "<repo>" commit -m "docs: close the Settings footer and OS light/dark items, and file their plan" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
