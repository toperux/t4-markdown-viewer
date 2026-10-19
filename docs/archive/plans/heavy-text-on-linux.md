# Heavy Text On Linux Plan

> **Status (2026-10-19):** executed on the owner's go (c5e195b); widened from the switcher's list
> to the tab strip and the sidebar after the scope probe (Q1). Walked: Wayland, Xvfb, XWayland
> (with `WEBKIT_DISABLE_DMABUF_RENDERER=1`, as it froze without; `open-items.md`) and a local
> AppImage (host WebKitGTK 2.52.6) all at the short weight with 0% fringes; scope exact;
> Windows and the Mac unchanged. Scale 2.0 skipped by the owner. Change review clean on its
> first pass; nothing to triage.

**Spec:** the open item "On Wayland, the tab switcher's list rendered heavier in a light theme"
(`docs/open-items.md` → Deferred), which the 2026-10-19 probes on the Linux VM reproduced and
traced. The owner chose the fix "grayscale smoothing, Linux only" (2026-10-19), and, at Q1, to
widen it to wherever a scope probe measured the same heaviness.

**Architecture:** one CSS rule in `src/base.css`, plus the records. No JS or Rust change.

## What the user sees

On Linux Wayland, in a light theme, three lists turn visibly heavier once they're long enough to
scroll; a short one looks normal:

- the tab switcher's list, where the bold matches then barely stand out;
- the tab strip's labels, once the tabs don't fit;
- the sidebar's file tree, once a folder is longer than the window: the common case.

In a dark theme the switcher's list turns slightly thinner instead.

## Measured (Linux VM, v1.8.6, Wayland, Virtual-1 1920×1200, scale 1.0, DPR 1)

Ink coverage (higher = heavier) and, in brackets, the share of colour-fringed (subpixel) ink
pixels; overflowing vs not.

| Text | sakura | github-light | Heavy? |
|---|---|---|---|
| Switcher row "21.md" (26-29 rows vs 8) | 126 (42%) vs 88 (85%) | 125 (0%) vs 88 (89%) | yes, +43% |
| Tab label "file2" (31 tabs vs 4) | 109 (46%) vs 76 (81%) | 107 (0%) vs 76 (83%) | yes, +42% |
| Tree "file21.md" (60 files vs 5) | 207 (44%) vs 144 (81%) | 205 (0%) vs 144 (88%) | yes, +43% |
| Switcher input "le" (long vs short list) | 49.5 vs 49.5 | 49.5 vs 49.5 | no |
| Code `pre` "const value" (scrolls sideways vs not) | 212 vs 212 | 196 vs 196 | no |

- The switcher's bold-to-plain contrast falls from 1.69 to 1.37 (sakura) and 1.42 (github-light).
  In azure-devops-dark the list goes thinner: 82 vs 88, fringes kept (79%).
- **With the rule** (`-webkit-font-smoothing: antialiased`): every heavy case lands on its short
  weight (switcher 88.3 and contrast 1.69 in all three themes; tab 76.6 and tree 144.7 in sakura
  and github-light, the dark theme unmeasured for those two), overflowing or not, with 0%
  fringes. Input and code unchanged by it.
- **Cause:** every overflowing scroller becomes a composited layer
  (`WEBKIT_SHOW_COMPOSITING_DEBUG_VISUALS=1`: borders around the switcher, strip, tree and code
  block, and their scrollbars, when overflowing; none when not). Text drawn into that layer comes
  out heavier dark-on-light and thinner light-on-dark. Why is reasoned only: blended differently
  from text in the page's tiles. Why the code block and the input escape is unknown (reasoned
  candidates: the monospace font, the highlight spans, an input's own rendering).
- **Tried, no effect (pixels identical to today):** an opaque list background; the switcher
  without its `transform`; the strip's `scrollbar-width: none` (already on Linux, `1226-1228`).
- **The rule's cost:** the text in these three loses its colour fringes, short lists and the dark
  theme included. By the ink measure the grayscale weight equals the subpixel one (88.3 vs 88.1).
  Reasoned, not checked: on screen a touch softer, like Xvfb's rendering.
- **Not measured:** scales other than 1.0; a real X11 session with GNOME's `rgba` antialiasing
  (Xvfb has none, and was grayscale already); the compositor's own framebuffer (the screenshots
  were WebKit snapshots); the AppImage, which forces X11 (`main.rs:1794-1803`) and bundles
  Ubuntu 22.04's older WebKitGTK (`open-items.md:161-162`), so whether it honours the property
  is unknown; the other scrollers (Settings in a short window, wide tables, diagrams).

## Current behaviour (main at 9b861b4)

- The three scrollers: `#switcher-list` (`base.css:308-316`, `overflow-y: auto`), `#tabs`
  (`1210-1221`, `overflow-x: auto`), `#sidebar` (`586-599`, `overflow: auto`; it holds the head, the
  filter and the tree, `index.html:105-120`, and scrolls them all).
- No font-smoothing anywhere in the app's CSS or its themes.
- `html.linux` is set from the user agent (`app.js:285-286`) and already scopes one Linux-only
  rule, `html.linux #tabs` (`base.css:1226-1228`).

## The change

In `base.css`, after `html.linux #tabs` (`1226-1228`):

```css
/* On Linux a scroller that overflows is composited, and WebKitGTK's subpixel
   text in it comes out over 40% heavier in a light theme (Wayland at scale
   1.0, measured; the switcher's list went thinner in a dark one). Grayscale
   gives each its weight when it fits. Code blocks and the switcher's input
   don't change (measured). */
html.linux #switcher-list,
html.linux #tabs,
html.linux #sidebar {
  -webkit-font-smoothing: antialiased;
}
```

- **Linux only:** on the Mac the property thins text (reasoned, well known for WebKit); Windows'
  WebView2 ignores it (reasoned). Scoping keeps both untouched.
- **Whole scrollers:** the sidebar's head and filter scroll with the tree, so they're in the
  same layer and take the rule too. The switcher's input and "no matches" line sit outside its
  list (`index.html:91-98`) and keep `auto`; the input measured unaffected.
- **Applies on X11 too:** the page can't tell Wayland from X11, so an X11 user loses the colour
  fringes in these three as well.
- **Out of scope:** code blocks and the switcher's input (measured unaffected); Settings, wide
  tables and diagrams (unmeasured; reopen on a report); the tab-drag chip (`.tab-ghost`, on
  `body`, fixed, `opacity: 0.95`, `base.css:1336-1356`, moved by an inline `transform`,
  `app.js:4302-4309`) and the toast (`#toast`, fixed with a `transform`, `base.css:1362-1368`):
  likely their own layers, unmeasured; the chip now keeps subpixel text unlike the tab it came
  from.
- *Q1, answered by the owner 2026-10-19: probe first, then widen only where measured heavy. The
  probe (Linux VM, 2026-10-19) measured the strip and the tree heavy, the input and code not.*

## Verification (Linux VM, report only, at the change's commit)

1. Wayland, scale 1.0, sakura, github-light and azure-devops-dark: the switcher (long and short),
   the strip (31 and 4 tabs), the tree (60 and 5 files). Expect each at its short weight with 0%
   fringes, overflowing or not: in the light themes ~88 / contrast ~1.69, ~76, ~144; in the dark
   theme the switcher ~88, and the strip and the tree the same overflowing as short (not measured
   before).
2. X11 (`GDK_BACKEND=x11`, and a real X11 or XWayland session if the VM offers one): the same.
   Expect the same numbers, which also shows the rule applies there.
3. The AppImage (a CI artifact of the change's commit) on the Wayland desktop, so under XWayland
   with its bundled WebKitGTK: sakura, the three, overflowing and not. Record the measures and
   whether the computed `-webkit-font-smoothing` is `antialiased`.
4. Wayland at scale 2.0, if the VM's screen allows (`archive/plans/session-restore.md` found it
   small at 200%): one theme, the three overflowing.
5. Scope, on Linux: the computed `-webkit-font-smoothing` is `antialiased` on `#switcher-list`, a
   `mark`, a `.switcher-folder`, a `.tab`, `#sidebar-name`, `#tree-filter` and a tree row, and
   `auto` on `#switcher-input`, `#switcher-none`, `#content`, a `pre`, `#tab-search`, a
   `#tree-menu` button, `.tab-ghost` (mid-drag), `#find-input` and `#toast`.
6. Windows and the Mac: the three look as before (screenshots, one light theme, overflowing), and
   the computed `-webkit-font-smoothing` on each is `auto`.

## Records

- Move the open item from `docs/open-items.md` (`229-235`) to `docs/closed-items.md` with a
  *Fixed* note: the measured cause, and that the strip and the tree had it too.

## Release

A fix to shipped behaviour: one commit, no prefix, release-note summary such as "On Linux, long
sidebar, tab strip and switcher lists no longer look heavy".
