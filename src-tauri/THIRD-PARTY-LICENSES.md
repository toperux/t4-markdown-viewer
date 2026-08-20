# Third-party notices

This app is MIT-licensed (see [`../LICENSE`](../LICENSE)). Everything it
depends on is permissively licensed and compatible with that. Nothing here is
GPL, LGPL, AGPL or SSPL.

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

306 crates reach the Windows release build. Audited against
`cargo metadata --filter-platform x86_64-pc-windows-msvc`, dev-dependencies
excluded. Every crate declares an SPDX license; none relies on a bare
`license-file`.

| License | Crates | Obligation |
| --- | --- | --- |
| `MIT OR Apache-2.0` (incl. legacy `MIT/Apache-2.0` spellings) | 254 | attribution |
| `MIT` | 56 | attribution |
| `Unicode-3.0` | 21 | attribution |
| `Unlicense OR MIT` | 10 | none |
| `MPL-2.0` | 5 | see below |
| `BSD-3-Clause` (incl. `… AND MIT`) | 6 | attribution, no-endorsement |
| `BSD-2-Clause` | 1 — `comrak` | attribution |
| `CC0-1.0` | 2 — `notify`, `dunce` | none (public-domain dedication) |
| `Zlib` | 1 — `foldhash` | attribution |
| `MITNFA` | 1 — `fmt2io` | MIT plus no-false-attribution |
| `Apache-2.0` only | 1 — `tao` | preserve `NOTICE`, patent grant |

Direct dependencies: `tauri`, `tauri-build`, `tauri-plugin-dialog`,
`tauri-plugin-opener`, `tauri-plugin-single-instance` (Apache-2.0 OR MIT),
`serde`, `serde_json`, `dirs`, `windows-sys` (MIT OR Apache-2.0),
`comrak` (BSD-2-Clause), `notify` (CC0-1.0).

### The MPL-2.0 crates

`cssparser`, `cssparser-macros`, `dtoa-short` and `selectors` arrive through
`tauri → dom_query`; `option-ext` through `dirs → dirs-sys`. All five are
transitive — none is used directly and none is modified here.

MPL-2.0 is **file-level** copyleft. Its §3.3 explicitly permits combining the
covered files into a "Larger Work" under any other license, including MIT, and
distributing the result in binary form. The only obligation is that modified
*MPL files themselves* stay MPL and stay available. Since these crates are
consumed verbatim from crates.io, that obligation is already satisfied by
upstream.

### Regenerating

For a per-crate manifest with full license texts:

```
cargo install cargo-about && cargo about generate about.hbs
```

## Not redistributed

The **WebView2 runtime** is a Windows system component supplied by Microsoft
under its own terms. This app links the loader and calls into whatever runtime
the machine already has; no part of WebView2 ships in this repo or its
installer.
