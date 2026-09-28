# macOS feature-build gotchas — plan

**Goal.** Record in `.claude/skills/drive-app/macos.md` what the 2026-09-28/29 Mac
run found the `webdriver` feature build can't show, so the next walk checks
those things on a plain build instead.

**Source.** The Mac session's reports of that run (recorded in `docs/closed-items.md`,
the Mermaid item). Measured facts go in as fact; the one guessed cause is marked
as a guess.

**Scope.** One file, `.claude/skills/drive-app/macos.md`. No app code.

## Outcome

Done 2026-09-29: the three edits landed as planned, and the change review found
nothing. No run was needed; the command and the results are the Mac run's own.

## Edits

### 1. Gotchas — a new bullet, first in the list

> - **No keyboard scrolling in a feature build** (measured). Space, Down and
>   PageDown sent through System Events reach the page — `keydown` fires, not
>   `defaultPrevented` — but nothing scrolls. A plain build (§1's build
>   without `--features webdriver`, started with §2's launch command) scrolls
>   with the same keys, and with the owner's own. The cause is unknown; the
>   plugin replacing the webview's UI delegate is a guess. So check anything
>   that scrolls from the keyboard on a plain build. It has no WebDriver: the
>   owner watches, and the page's scroll is read from its scroll bar through
>   Accessibility:
>
>   ```bash
>   osascript -e 'tell application "System Events" to tell (first process whose unix id is <pid>) to get value of scroll bar 1 of scroll area 1 of group 1 of group 1 of window 1'
>   ```
>
>   The value is the fraction of the scroll range, 0 at the top and 1 at the
>   bottom. Measured with one window, one tab and no sidebar, the app just made
>   frontmost. `window 1` is the front window, a page too short to scroll
>   probably has no scroll bar, and a sidebar or the tab strip may change the
>   nesting (all reasoned). If the path fails, walk it down by role:
>   `get {role, name} of every UI element of window 1`, then of each `group`,
>   to the `AXScrollArea` holding the `AXWebArea` and the `AXScrollBar`.

### 2. Gotchas — extend *A feature build loses*

After the existing bullet's last sentence (`The app uses none of them.`), add:

> Per wry #1593, an iframe's navigation on macOS reaches wry's new-window
> handler, and that handler may be part of what the plugin replaces (not
> checked). So a check of how the diagram frame loads, or that nothing opens a
> window, runs on a plain build by the owner's eye. The 2026-09-28 frame check
> ran on a plain build for that reason, and passed; whether a feature build
> would differ was not measured.

### 3. §3 Drive — pointer from the real-keys line

In *All of these were measured to work*, at the end of the **Real keys**
bullet, add: `They don't scroll the page in a feature build; see *Gotchas*.`

## Verify

- `macos.md` reads straight through; *Gotchas* and the pointer resolve.
- Every added claim traces to the Mac reports, or is marked reasoned.
- No run needed: the AX command and the keyboard result are the run's own,
  measured on this build and macOS 26.7.

## Commit

`docs: note what drive-app's macOS feature build can't check, and how to check it on a plain build`
— local, on main.
