# Themes

A theme is one plain CSS file. There is no build step and no restart.

## Where themes come from

| Location | Purpose |
| --- | --- |
| `<install dir>\themes\` | Bundled themes shipped with the app |
| `%APPDATA%\t4-markdown-viewer\themes\` | Your own themes |

Both are scanned at startup and re-scanned whenever a `.css` file in either one
changes. A user theme **shadows** a bundled theme with the same filename, so you
can override `azure-devops.css` without touching the install directory.

The file stem is the theme id and the picker label is derived from it:
`solarized-light.css` → **Solarized Light**. Files starting with `_` are skipped.

## Writing one

Copy `_template.css` into your themes directory, rename it, and start editing.
Save and the view re-styles immediately — the picker keeps your selection.

### The contract

`base.css` owns **structure**: content width, margins, `overflow`, border
*widths*, table collapsing. It deliberately declares no document colors.

A theme owns **everything visual**: colors, fonts, type sizes, border colors.
Every selector is scoped under `.markdown-body`, which wraps the rendered
document.

Four variables control the app's own top bar, which sits outside the document:

```css
:root {
  --ui-bg: #f5f5f5;
  --ui-fg: #1f1f1f;
  --ui-border: rgba(0, 0, 0, 0.12);
  --ui-accent: #0078d4;
  --content-width: 54rem;
}
```

Dark themes must also set `color-scheme: dark` on `:root` (not `body` —
Chromium reads it from the root element), which fixes the scrollbars and
checkboxes the browser draws itself.

### The theme dropdown needs literal colors

The theme picker's popup list is a separate OS-level widget rendered outside the
page. Two things follow, and both bite dark themes:

- **`var()` does not resolve there.** `background-color: var(--ui-bg)` silently
  falls back to base.css's light defaults instead of your theme's.
- **`color-scheme` is ignored there.** The popup stays light no matter what.

Meanwhile options inherit `color` from the select — your theme's foreground. On
a dark theme that is near-white text on a near-white popup, so the list looks
empty apart from whichever row the mouse is over.

So every theme restates the pair with literal values:

```css
#bar select option {
  background-color: #252526;             /* match --ui-bg */
  color: rgba(255, 255, 255, 0.9);       /* match --ui-fg */
}
```

### Elements to cover

`h1`–`h6`, `p`, `a`, `strong`/`em`, `ul`/`ol`/`li`, `dl`/`dt`/`dd`,
`blockquote`, `table`/`th`/`td`, `code`, `pre`, `hr`, `img`,
`li > input[type=checkbox]`, `.footnotes`.

Raw HTML in a document is stripped before rendering (the app opens untrusted
files), so tags that only come from hand-written HTML — `<kbd>`, `<sub>`,
`<mark>` — never reach the page. Styling them is dead CSS.

A few need care because `base.css` sets only half the property:

- `blockquote` — `border-left-width` is set; supply `border-left-color`.
- `th`, `td`, `hr` — border widths are set; supply the colors.
- `pre code` — inline `code` styling must be reset inside fenced blocks
  (`padding: 0; background: none`), or every code block gets a double chip.

### Syntax highlighting

Code is highlighted in the webview by [highlight.js](https://highlightjs.org),
which tags spans with `.hljs-*` classes. Any highlight.js theme from the wild
can be pasted into a theme file — scope its selectors under `.markdown-body` so
they beat the bundled defaults.

Blocks without a declared language are not auto-detected; they get the plain
`.hljs` background only. Style `.hljs` itself, not just the token classes.

## Bundled catalog

| Theme | Notes |
| --- | --- |
| `azure-devops` | **Default.** Palette, type scale and font stacks lifted from Microsoft's `azure-devops-ui` package. |
| `azure-devops-dark` | Same metrics, dark palette. |
| `github-light` / `github-dark` | GitHub Primer colors. |
| `solarized-light` / `solarized-dark` | Ethan Schoonover's Solarized. |
| `dracula` | Dracula palette. |
| `dracula-green` | Dracula with the purples rotated to green; warm accents kept. |
| `dracula-blue` | Dracula with the purples rotated to blue; navy base. |
| `sakura` | Minimal, roomy line height. Adapted from oxalorg/sakura. |
| `tufte` | Serif, italic headings, narrow measure. Adapted from Tufte CSS. |

See `../THIRD-PARTY-LICENSES.md` for attribution.
