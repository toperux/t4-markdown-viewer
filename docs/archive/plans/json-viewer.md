# View .json / .jsonc files — highlighted, foldable, any size

> **Status (2026-09-12):** shipped as 1.6.0, in one squashed commit on `main` (e3c6af3).
> Two review rounds ran on top of the first implementation; what they found and changed
> is in *Outcome* at the end. One thing is left to prove on an installed build — the file
> registration on macOS and deb/rpm (Windows proven 2026-09-13) — see
> `../../open-items.md`.

## Context

The viewer opened Markdown and images. JSON sits beside Markdown in most notes and
config folders, and was hidden from the sidebar, refused from the command line, and a
`[config](settings.json)` link only revealed the file in the file manager. Goal: `.json`
/ `.jsonc` opens like a document — double-click, `Ctrl+O`, sidebar, link, CLI — shown as
highlighted source with folding, and a 30 MB dump opens without stalling the UI.

## Decisions

- Shown as written (comments, key order, formatting kept). Exception: a file on one
  line — after any blank or `//` header lines — is reflowed first, since one 5 MB line
  is unreadable. The reflow is token-based, so numbers and comments survive it.
- Every `{…}` / `[…]` folds. Fold state is not persisted.
- Large files render in chunks: the first ~512 KB of source, then a "… N MB more"
  control per open container that fetches the next chunk from Rust.
- Highlighting is done in Rust, emitting the same `hljs-*` classes highlight.js would,
  so every theme already styles it and hljs never runs on JSON.
- Installers register `.json` and `.jsonc` for Open With (never the default), under
  their own ProgID so Explorer does not call a `.json` a Markdown document.
- Extensions: `json`, `jsonc`. Out of scope: json5, tree view, `.txt`.

## Design

**Pipeline reuse.** `load_file` returns a `Document` whose `html` is
`<pre><code>…</code></pre>` built by `json.rs` instead of comrak. Nothing downstream
changes: `renderDocument` inserts it, `highlight()` sees no `language-` class and only
adds `.hljs` (theme background), live reload, scroll memory, tabs, session and drag all
key off the path.

**Tokenizer.** A streaming iterator over `(kind, byte range)`; never panics, accepts
anything: strings (unterminated runs to end), numbers, `true/false/null`, `//` and
`/* */` comments, punctuation, whitespace; anything else is escaped text. A string whose
next non-trivia token is `:` is a key.

**Folds.** A non-empty container emits a `<button class="fold">` before its opening
bracket and wraps the body in `<span class="fold-body">`. One delegated click handler
toggles `aria-expanded` and `hidden`. A bracket that opens its line gets a `gutter`
class and hangs in the `pre` padding, so indentation stays aligned; a mid-line bracket
takes its own width.

**Chunking.** The document — and each slice — is an implicit outermost container. Past
`CHUNK_BYTES` the emitter looks ahead 64 KB for the shallowest comma (a record boundary
if one is near) and cuts there, emitting a `<button class="more" data-range="START:END">`
for the remainder of every open container, then closing the brackets. Offsets are
absolute into the derived source. `json_region(path, start, end)` re-reads the file,
re-derives the source, renders `source[start..end]` and the button is replaced by the
result. The frontend remembers how far the document was loaded and hands that back to
`load_file` as the budget, so a tab switch, live reload or F5 restores the loaded
chunks (capped at 8 MB in one go).

## Changes

- `src-tauri/src/json.rs` (new): tokenizer, `source()` reflow, `render_to()`,
  `render_slice()`, emitter with explicit container stack.
- `src-tauri/src/main.rs`: `JSON_EXTS`, `is_json`, `is_document` (replaces the three
  `is_markdown` gates), `load_file` branch and `extent` parameter, `json_region`
  command, `check_size` shared.
- `src/app.js`: `MD_EXTS`/`JSON_EXTS` arrays feeding `DOC_LINK` and the dialog filters
  ("Documents" first, since Windows and GTK show only the first filter), `onJsonClick`,
  `extent` on history entries.
- `src/base.css`: `.fold`, `.fold.gutter`, `.more`, focus rings; structure only.
- Installers: second `fileAssociations` entry (`T4MarkdownViewer.Json`,
  `application/json`); NSIS macros take a ProgID; Linux MIME xml adds `*.jsonc`.
- `README.md`: features, Windows registration, design note, layout.

## Verification

Gates as in `checks.yml`: `cargo fmt --check`, `cargo clippy --workspace --all-targets
--locked -- -D warnings`, `cargo test --workspace --locked`, `node --check src/app.js`.

`drive-app` on the debug build: JSONC with comments (folds, empty containers get none),
32 MB array of records (opens in ~1 s, cuts land between records, chunks splice), one-line
and header-plus-one-line files reflowed, invalid JSON shown as text, link from `.md` loads
in place with Back, sidebar lists the files, live reload, stale range errors cleanly,
native Open dialog, CLI single-instance route, all 15 themes, tab switch keeps expanded
chunks and scroll, focus ring on fold/more, gutter vs inline markers on a matrix file.

## Outcome

Shipped 2026-09-12. Rust tests went from 89 to 114.

**Review round 1 (high)** found ten issues; seven fixed, three accepted as ceilings.
Fixed: the Open dialog's first filter hid `.json` on Windows/GTK; the reflow fused
space-separated tokens and missed a header-plus-one-line file; a comma-only remainder
after a cut was dropped; no focus ring on the new buttons; the fold marker painted over
the preceding character; a re-render dropped every fetched chunk and clamped the scroll.
Verification of those fixes turned up one more: the cut landed at the first comma past
the budget at whatever depth, splitting a record into three buttons ("… 0 KB more"
included). Hence the 64 KB lookahead.

**Review round 2 (medium)** on the fixes found ten more; nine fixed. Real ones: a stray
closing bracket in the lookahead could pin the cut to a depth the emitter never
reached (whole file rendered); the extent grew to "everything" and a file rewritten
larger came back whole; the average-line-length reflow gate re-indented a formatted
file with one large value; a leading newline became a leading space. Declined: adding
`.txt` to the "Documents" filter — it is deliberately CLI/sidebar only.

**Accepted ceilings**, marked `ponytail:` in the source: a single token bigger than the
budget renders whole; a stale-but-in-bounds range inside the watcher's debounce window
splices garbage that the re-render replaces; every `more` click re-reads the file; fold
state resets on re-render; selection-copy includes the "… more" label.
