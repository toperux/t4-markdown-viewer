# Fixtures

Markdown documents (and the images they reference) used by the manual and
`drive-app` checks in the archived Mermaid and viewer plans. They used to
live only in a session's scratchpad, where they'd vanish with it; they're
kept here so a check can be re-run later.

- `all-types.md` — one section per mermaid 11.17.2 diagram type, plus a
  swimlane case and `## Heading B`. `archive/plans/mermaid-pipeline.md`
  (baseline and every task's regression check) and
  `archive/plans/mermaid-follow-ups.md` (each task's "35 drawn, 0 refused").
- `escape.md` — three diagrams whose `themeVariables`/`themeCSS` try to
  escape their own styles via a quote. `mermaid-pipeline.md` Task 1 Step 5
  (the style check refuses them) and Task 5 Step 6 (locking under "Follow
  the theme"/"Mermaid's own").
- `ids.md` — a flowchart whose label names its own id, a swimlane section,
  and a `click` link to `## Heading B`. `mermaid-pipeline.md` Task 3 Step 4
  (id namespacing doesn't collide with the app's own ids).
- `three.md` — three small flowcharts. `mermaid-pipeline.md` Task 2 Step 3
  (c) (the LRU diagram cache keeps the newest document's diagrams when it's
  full).
- `blank.md` — a diagram-free page. `mermaid-pipeline.md` Task 1 Step 1 (the
  baseline screenshot opens from a diagram-free page, so gantt's
  width-from-page-on-screen sizing is consistent).
- `dup.md` — the same flowchart three times, one copy inside a `<details>`.
  `mermaid-follow-ups.md` Task 2 Step 4 (d) (repeated diagrams get their own
  numbered ids, `Mermaid-Nx2`/`Mermaid-Nx3`, with no collisions).
- `gantt-links.md` (with `notes.md` beside it) — a gantt with `click … href`
  links to a heading, a web address, a relative file, an unsafe URL and a
  `call`. `mermaid-follow-ups.md` Task 4 Step 4 (gantt task links go where
  the app's other links go).
- `seq/seq-links.md` (with its own `seq/notes.md` beside it) — a sequence
  diagram with actor `link`/`links` menus, repeated once, plus a stick-figure
  actor whose `link` is expected to fail the diagram. `mermaid-follow-ups.md`
  Task 5 Step 4 (sequence-diagram link menus open and their links resolve).
  Kept in its own folder because its `notes.md` companion is a different
  file from `gantt-links.md`'s.
- `b1-targets.md` — heading, footnote and GitHub-style `#user-content-…`
  targets. `archive/plans/viewer-follow-ups.md` Task 1 (`docTarget` resolves
  names in GitHub's order).
- `b2-links.md`, `b2-other.md` (a second document it links to) and
  `b2-pic.png` (a linked picture) — external, in-page, same-file, other-file
  and picture links. `viewer-follow-ups.md` Task 2 (links on a page keep
  working while a save redraws it).
- `b4-keys.md` and `b4-noalt.md` (an unlinked picture with no `alt`) — a
  document with diagrams, linked and unlinked pictures, and a failing
  diagram, reused for the keyboard checks. `viewer-follow-ups.md` Task 4
  (a diagram or picture opens from the keyboard).

Not copied: scratch scripts, screenshots, saved JSON results, backups and
HTML probes the same plans used alongside these fixtures — none of that is
an input document, so none of it belongs here.
