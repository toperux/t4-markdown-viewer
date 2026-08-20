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

Those four, plus `color-scheme`, are the whole chrome contract. The toolbar, the
tab strip and the Settings dialog are all drawn from them, so a theme never has
to know those parts exist.

> **Removed in favour of that:** themes used to need a `#bar select option` rule
> with *literal* colors, because the theme picker was a `<select>` whose popup
> is an OS-level widget — `var()` does not resolve inside it and `color-scheme`
> is ignored, so a dark theme's near-white text landed on a near-white popup and
> the list looked empty. The picker is now radio buttons inside the Settings
> dialog, which is ordinary in-page DOM. If your theme still carries that rule
> it is harmless, just dead.

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
