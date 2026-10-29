# Cargo Group Bump (Dependabot PR #18) Plan

> **Status (2026-10-29):** executed in full: b2d5952 takes PR #18's two Cargo files whole (scope
> as planned; gates passed, 215 tests). The render diff was byte-identical on all 8 documents,
> the snippet included. S1–S4 passed on Windows, macOS and Linux (Linux on a headless Xvfb
> display, since the VM's pointer input is still down; its icon checked through
> `_NET_WM_ICON`). An image passed as a launch argument opens no tab, by design (launch
> arguments take only Markdown or JSON), so S3 opened it through *Open file*. On Linux, the
> walk's own picker helper needed a second Return once (helper timing, not the app; the owner
> ruled no entry). Windows' launch console showed the long-standing `ipc.localhost` CSP errors
> (no `connect-src`, so IPC falls back to postMessage). Not from this bump; the owner ruled it
> fixed separately, in `ipc-connect-src.md`.

**Goal:** land Dependabot PR #18's bumps on `main` (tauri 2.12.2 → 2.12.5, comrak 0.56 → 0.57,
tauri-build 2.7.3 → 2.7.4) so the next release ships with them.

**Spec:** PR #18 (`gh pr view 18`). The owner deferred it to the release after 1.8.9
(2026-10-25), to go in with its own plan, a render diff and walks, as
`archive/plans/cargo-group-bump-pr17.md` did.

## Findings (measured 2026-10-26, before this plan)

PR #18's base is `8509415` (Release 1.8.9), which is `main` today, so its Cargo files can be
taken as they are. It changes `src-tauri/Cargo.toml` (`comrak = "0.56"` → `"0.57"`) and
`src-tauri/Cargo.lock` only. Its CI run (40210921119) passed on all three platforms.

| Bump | What changed upstream | Consequence here |
|---|---|---|
| comrak 0.56 → 0.57 | New `ofm_comments` extension (`%%…%%`) and wikilink embeds (`![[…]]`). New node types, an `embed` field on `NodeWikiLink`. Read in the diff: every new parser path is behind `extension.ofm_comments` or `extension.wikilinks()`, both off by default | `render.rs`'s `options()` turns on neither, so output should be unchanged. One change isn't behind an option: `render_paragraph` now writes a newline before a paragraph in a *tight* list item, or a description item, when the paragraph before it is also a paragraph (comments skipped). Reasoned unreachable without comments, since two paragraphs in one item normally make it loose; not measured. The render diff below measures it |
| tauri 2.12.2 → 2.12.5 | No panic in the IPC and asset protocol handlers on a URI with an empty path. `Channel` keeps delivering after an `onmessage` handler throws. NSIS downgrade-blocking fixes. iOS and mobile fixes | The app uses the asset protocol (`convertFileSrc`, `app.js:391`) for images, so the walk opens one. It uses no `Channel`. The NSIS fixes live in the bundler, which comes with tauri-cli (pinned at 2.12.1 in `release.yml`), not with this crate, so the installer is unchanged |
| tauri-runtime-wry 2.12.1 → 2.12.3, tauri-runtime 2.12.1 → 2.12.2 | Undecorated window resizing on Linux inside another GTK container. `once_cell` replaced with `std` | Not this app's case (decorated, no GTK container) |
| tauri-codegen and tauri-macros 2.7.1 → 2.7.4 | `base64` 0.22 → 0.23, `ico` 0.5 → 0.6, generated files dated 2000-01-01 so a rebuild with no changes skips the app crate | Codegen embeds the frontend, the CSP hashes and the window icon. A break shows at boot: a blank page or a missing icon. The walk checks both |
| tauri-build 2.7.3 → 2.7.4 | Doesn't watch a missing `capabilities` directory | `src-tauri/capabilities/` exists. Nothing changes |

Not measured: the paragraph change's reach (Task 1's render diff measures it), and anything at
run time (Task 2's walks).

Lockfile side effects: `png` 0.17.16 drops out (only `ico` 0.5 used it), and tauri-runtime-wry
drops `once_cell`. Nothing is added.

## Global constraints

- No dependency change beyond PR #18's. Never a bare `cargo update`.
- Gates, from the repo root, before the commit: `cargo fmt --manifest-path src-tauri/Cargo.toml
  --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked
  -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`,
  `node --check src/app.js`.
- Straight onto `main`, committed locally; no push without the owner's go. No AI attribution.
  Dependabot closes PR #18 by itself once `main` has the versions.
- Walks run on the peers (Windows VM, Linux VM, Mac) from a pushed `walk/pr18` branch, never on
  the host PC. Pushing it needs the owner's go at the time. Each peer backs up and restores
  `config.json` and `session.json` per the drive-app skill.

## Task 1: take the bump, with a render diff

1. **Before renders.** Add a throwaway `#[ignore]` test to `render.rs`'s `mod tests` that writes
   `render()` of each corpus document to the directory in `T4_DUMP` (shape:
   `archive/plans/cargo-group-bump.md`, Task 1 Step 2). Corpus: every `examples/*.md`,
   `README.md`, `docs/open-items.md`, `docs/closed-items.md`, and one snippet over the paths
   comrak 0.57 touched (rendered with the app's options, so both new extensions are off):
   - `50%% off %%a note%%` inline, and `%%` alone on a line, then text, then `%%`.
   - `[[page]]`, `[[page|label]]`, `![[page]]`, `![alt](img.png)`, `!` then text.
   - A tight list, and a tight and a loose description list (`T\n: d`), each with two lines
     per item.
   - The autolink edge cases from that plan.

   Run it into `<scratchpad>/before`.
2. **Take PR #18's files.** `git fetch origin dependabot/cargo/src-tauri/cargo-30d4dad21f`,
   then check `git diff --stat 8509415 FETCH_HEAD` lists exactly the two Cargo files and
   `git diff --stat 8509415 HEAD -- src-tauri/Cargo.toml src-tauri/Cargo.lock` prints nothing.
   Then `git checkout FETCH_HEAD -- src-tauri/Cargo.toml src-tauri/Cargo.lock`, and check the
   versions landed.
3. **After renders**, into `<scratchpad>/after`. Expect every file byte-identical
   (`diff -r`). Any difference: stop and ask.
4. Delete the throwaway test (`git diff src-tauri/src` empty). Gates. Expect all to pass, with
   the same test count as a run before the bump.
5. Commit: `chore: bump tauri and comrak from Dependabot PR #18`.

## Task 2: smoke walks

On `walk/pr18`, with the debug build, on all three platforms:

- **S1, boots:** the empty screen, themed, with the app's icon on the window (taskbar, dock or
  title bar). Proves codegen's embedded frontend, CSP and icon. On a headless Linux display
  (Xvfb, while the VM's pointer input is down) there's no panel: check the window's
  `_NET_WM_ICON` is set instead (`xprop`).
- **S2, renders:** `examples/kitchen-sink.md` opens with its table, code, footnotes, task list
  and diagram drawn. Ticking a task writes the change to the file (restore the file after).
- **S3, images:** kitchen-sink's relative image (`img/icon.png`) shows, and
  `examples/img/raster-wide.png` opened on its own shows in its tab. Both go through the asset
  protocol.
- **S4, IPC and plugins:** Settings › *Check now* reads "You are up to date." (1.8.9 is the latest
  release and the build's version), and *Open file* shows the native picker and opens the
  chosen file.

## Task 3: close out

Archive this plan to `docs/archive/plans/` with a status line giving the commit, the render
diff's result and S1–S4's, and an open-items entry if the triage leaves anything. In a `docs:`
commit, `docs: plan and record the tauri and comrak bump from Dependabot PR #18`.

## Execution

The main session runs Task 1 (two files taken whole, one throwaway test) and Task 3, and sends
Task 2 to the peers. Serial. Stop and ask on:

- PR #18 touching more than the two Cargo files, or `main`'s Cargo files having changed since
  `8509415`.
- Any difference in the render diff.
- Any gate failing, or clippy reporting anything new.
- Any smoke failure.
