# Third-party notices

## Bundled at runtime

### highlight.js

`src/vendor/highlight.min.js` — v11.11.1, redistributed verbatim.
BSD-3-Clause. Copyright (c) 2006, Ivan Sagalaev. <https://highlightjs.org>

## Theme colors

The bundled themes in `themes/` are original CSS written for this app. Their
color values are drawn from the following palettes and design systems; no CSS
was copied verbatim.

| Theme file | Palette source | License |
| --- | --- | --- |
| `azure-devops.css`, `azure-devops-dark.css` | Microsoft `azure-devops-ui` design tokens (`Core/core.css`, `Core/override.css`, `buildScripts/cssDefaults.json`); Visual Studio Light+/Dark+ syntax colors | MIT |
| `github-light.css`, `github-dark.css` | GitHub Primer color palette | MIT |
| `solarized-light.css`, `solarized-dark.css` | Solarized, Ethan Schoonover | MIT |
| `dracula.css` | Dracula theme palette | MIT |
| `dracula-green.css`, `dracula-blue.css` | Dracula theme palette, hue-shifted; not official Dracula variants | MIT |
| `sakura.css` | oxalorg/sakura | MIT |
| `tufte.css` | Tufte CSS, Dave Liepmann | MIT |

Trademarks (Azure DevOps, GitHub) belong to their respective owners. The themes
are visual approximations for personal use and are not affiliated with or
endorsed by those projects.

## Rust dependencies

Build a full dependency manifest with:

```
cargo install cargo-about && cargo about generate about.hbs
```
