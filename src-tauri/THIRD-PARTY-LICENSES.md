# Third-party notices

This app is MIT-licensed (see [`../LICENSE`](../LICENSE)). No Rust crate here
is GPL, LGPL, AGPL or SSPL. The Linux AppImage also bundles system libraries;
see below.

## Bundled at runtime

### highlight.js

`src/vendor/highlight.min.js` — v11.12.0, redistributed verbatim.
BSD-3-Clause. Copyright (c) 2006, Ivan Sagalaev. <https://highlightjs.org>

## Theme colors

The bundled themes in `themes/` are original CSS written for this app. Their
color values are drawn from the following palettes and design systems; no CSS
was copied verbatim.

| Theme file | Palette source | License |
| --- | --- | --- |
| `azure-devops.css`, `azure-devops-dark.css`, `azure-devops-blue.css`, `azure-devops-dark-blue.css` | Microsoft `azure-devops-ui` design tokens (`Core/core.css`, `Core/override.css`, `buildScripts/cssDefaults.json`); Visual Studio Light+/Dark+ syntax colors | MIT |
| `github-light.css`, `github-dark.css`, `github-light-blue.css`, `github-dark-blue.css` | GitHub Primer color palette (the `-blue` variants tint headings with a Dracula-derived blue) | MIT |
| `solarized-light.css`, `solarized-dark.css` | Solarized, Ethan Schoonover | MIT |
| `dracula.css` | Dracula theme palette | MIT |
| `dracula-green.css`, `dracula-blue.css` | Dracula theme palette, hue-shifted; not official Dracula variants | MIT |
| `sakura.css` | oxalorg/sakura | MIT |
| `tufte.css` | Tufte CSS, Dave Liepmann | MIT |

Trademarks (Azure DevOps, GitHub) belong to their respective owners. The themes
are visual approximations for personal use and are not affiliated with or
endorsed by those projects.

## Rust dependencies

349 crates reach the Windows release build. Audited against
`cargo metadata --filter-platform x86_64-pc-windows-msvc`, dev-dependencies
excluded. Every crate declares an SPDX license; none relies on a bare
`license-file`.

| License | Crates | Obligation |
| --- | --- | --- |
| `MIT OR Apache-2.0` (incl. legacy `MIT/Apache-2.0` spellings, and a third option: `Zlib`, `ISC`, `0BSD`) | 227 | attribution |
| `MIT` | 70 | attribution |
| `Unicode-3.0` (incl. `(MIT OR Apache-2.0) AND Unicode-3.0` and `… AND Unicode-DFS-2016`) | 21 | attribution |
| `Unlicense OR MIT` | 10 | none |
| `MPL-2.0` | 5 | see below |
| `BSD-3-Clause` (incl. `… AND MIT`, `BSD-3-Clause/MIT`) | 5 | attribution, no-endorsement |
| `BSD-2-Clause` | 1 — `comrak` | attribution |
| `ISC` | 2 — `rustls-webpki`, `untrusted` | attribution |
| `CC0-1.0` (incl. `… OR MIT-0 OR Apache-2.0`) | 2 — `notify`, `dunce` | none (public-domain dedication) |
| `Zlib` | 1 — `foldhash` | attribution |
| `MITNFA` | 1 — `fmt2io` | MIT plus no-false-attribution |
| `Apache-2.0` only | 2 — `tao`, `sync_wrapper` | preserve `NOTICE`, patent grant |
| `Apache-2.0 AND ISC` | 1 — `ring` | preserve `NOTICE`, patent grant, attribution |
| `Apache-2.0 AND MIT` | 1 — `dpi` | preserve `NOTICE`, patent grant, attribution |

Direct dependencies: `tauri`, `tauri-build`, `tauri-plugin-dialog`,
`tauri-plugin-opener`, `tauri-plugin-single-instance`, `tauri-plugin-updater`
(Apache-2.0 OR MIT), `serde`, `serde_json`, `dirs`, `windows-sys`
(MIT OR Apache-2.0), `comrak` (BSD-2-Clause), `notify` (CC0-1.0).

### The MPL-2.0 crates

`cssparser`, `cssparser-macros`, `dtoa-short` and `selectors` arrive through
`tauri → tauri-utils → dom_query`; `option-ext` through `dirs → dirs-sys`. All five are
transitive — none is used directly and none is modified here.

MPL-2.0 is **file-level** copyleft. Its §3.3 explicitly permits combining the
covered files into a "Larger Work" under any other license, including MIT, and
distributing the result in binary form. The only obligation is that modified
*MPL files themselves* stay MPL and stay available. Since these crates are
consumed verbatim from crates.io, that obligation is already satisfied by
upstream.

### Regenerating

The table counts every crate reachable from this package through normal and
build dependencies, from the resolved graph for the Windows target:

```
cargo metadata --format-version 1 --locked --filter-platform x86_64-pc-windows-msvc --manifest-path src-tauri/Cargo.toml
```

Walk `resolve.nodes` from `resolve.root`, following each dependency whose
`dep_kinds` includes a kind other than `dev`, and group the crates reached by
their `license` field.

## Linux AppImage

The AppImage also carries shared libraries that linuxdeploy copies from the
Ubuntu 22.04 build machine — GTK 3, WebKitGTK, GLib, GStreamer, libsoup and
what they link to (the full list is `usr/lib` after `--appimage-extract`). They
are unmodified Ubuntu binaries under their own licences: mostly
LGPL-2.1-or-later, some MIT/BSD/ICU, and `libjbig` (through libtiff) under
GPL-2.0-or-later. The app loads them dynamically; any of them can be replaced
by extracting the AppImage and running `AppRun` from the extracted tree. Their
source is the matching Ubuntu 22.04 (jammy) source package
(`apt-get source <package>`), and the author will provide it on request — open
an issue at <https://github.com/toperux/t4-markdown-viewer/issues> — for three
years after each release. The .deb and .rpm bundle none of these.

## Not redistributed

The **WebView2 runtime** is a Windows system component supplied by Microsoft
under its own terms. This app links the loader and calls into whatever runtime
the machine already has; no part of WebView2 ships in this repo or its
installer.
