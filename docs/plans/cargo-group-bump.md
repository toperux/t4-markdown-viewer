# Cargo Group Bump (Dependabot PR #3) Implementation Plan

> **For agentic workers:** Execute per the **Execution** section below — it says who runs each task and in what order, and it overrides the one-subagent-per-task default of superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land Dependabot PR #3's seven crate bumps on `main`, with the one code change comrak 0.55 needs, so the open item "Dependabot's cargo PR #3 fails all three CI legs" closes.

**Architecture:** Take the PR's `Cargo.toml` and `Cargo.lock` as Dependabot built them, drop the deprecated `tagfilter` option from `render.rs`, and pin what it used to promise with a test. No merge of the PR branch: the owner commits straight to `main`, and Dependabot closes the PR on its own once `main` has the versions.

**Tech Stack:** Rust + Tauri v2, comrak (Markdown), `dirs`, `windows-sys`, Tauri plugins.

**Spec:** `docs/open-items.md` → *Bugs* → "Dependabot's cargo PR #3 fails all three CI legs", and PR #3 itself (`gh pr view 3`).

## Findings (measured 2026-09-24, before this plan)

| Bump | What changed | Consequence here |
|---|---|---|
| comrak 0.54 → 0.55 | `Extension::tagfilter` deprecated (removed in 0.56); two DoS fixes in autolinks (GHSA-xg9p-p4jc-c46g) | `render.rs:18` is the only CI error, on all three legs (run 35534611594). `autolink` is on in this app, so the DoS fix matters |
| dirs 6 → 7 | One behaviour change: `preference_dir` on Windows. Same `dirs-sys` 0.5. Repo moved to Codeberg | The app calls only `home_dir` and `config_dir` (`config.rs:87,94,247`): settings and `session.json` stay where they are |
| windows-sys 0.59 → 0.61 | Major bump | Already compiles: the Windows leg's only error was `render.rs:18`. Used for `WindowFromPoint` / `GetAncestor` (`main.rs:384`) |
| tauri-plugin-updater 2.10.1 → 2.11.0 | Windows: installer spawn failure now returns an error; new opt-in `restart_after_install`; `system-proxy` feature, on by default | Default install path unchanged; the error lands in `update.rs`'s existing `map_err` |
| dialog 2.7.3, opener 2.5.5, single-instance 2.4.4 | Android Gradle fix and docs only | Nothing on desktop |

`tagfilter` does nothing in this app: it filters tags inside raw HTML that is *rendered*, and `render.unsafe_` is off, so raw HTML is already replaced with `<!-- raw HTML omitted -->`. A throwaway probe on comrak 0.54 rendered 12 documents (every `examples/*.md`, `README.md`, eight hostile snippets) with it on and off: identical output.

PR #3's base is `528ca95` (Release 1.6.6), and neither Cargo file has changed on `main` since, so its lockfile can be taken as-is.

Toolchain: local rustc 1.98.0 and CI's `stable` both clear the new floors (comrak 0.55 needs 1.85, tauri-plugin-updater 2.11 needs 1.77.2, windows-sys 0.61 needs 1.71). Task 1 Step 1's test was run on the current tree while planning, and passes.

## Global Constraints

- Surgical changes only. Match the surrounding comment style (full-sentence "why" comments) and naming.
- No dependency changes beyond PR #3's. Never run a bare `cargo update`.
- Gates, all run from the repo root, all must pass before each commit:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --check` (CI runs this on the Linux leg only — run it every time)
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`
  - `node --check src/app.js`
- All commits go straight onto `main`; no feature branches. Commit locally only. **Never push.** Subjects are given per task; `chore:` / `docs:` keep them out of the release notes.
- End every commit message with the executing model's `Co-Authored-By:` line.
- Bash on Windows: never `cd` in a compound command that writes; run from the repo root and use `--manifest-path` / `git -C`. A hook condenses shell output through `rtk`; prefix `rtk proxy` for raw output.
- The debug build shares `%APPDATA%\t4-markdown-viewer\` (`config.json`, `session.json`) with the installed app. Task 2 follows the drive-app skill's backup / `"reopen": "off"` / diff-before-restore rules.

## Review Focus

Inputs the bump could change that no CI failure would show, most likely first:

1. **Documents with raw HTML** (`<script>`, `<iframe>`, `<style>`, `<textarea>`…): must still render without the tag. Pinned by Task 1 Step 1.
2. **Anchor targets** (`<a id="x"></a>` then `[go](#x)`): recovery keys on comrak's exact `<!-- raw HTML omitted -->` marker (`OMITTED`, `render.rs:7`). A changed marker silently breaks every recovered anchor. Covered by the existing anchor tests, by Task 1's before/after render diff (the `examples/defect-96194-…` document in its corpus has 18 raw anchors), and by Task 2 S3.
3. **Autolinks** (bare `https://…`, `www.…`, emails): the code comrak 0.55 changed. Covered by the before/after render diff, whose corpus includes an autolink sample with edge cases. The DoS fix could legitimately change output on an edge case: a difference there is a stop-and-ask, not a failure to work around.
4. **Task-list write-back:** needs `data-sourcepos` on each `<li>`. Covered by the existing `toggle_task` tests and the render diff.
5. **Settings location** (`dirs` 7): a moved `config_dir` would lose the owner's theme and session with no error. Argued above from the crate's changes; Task 2 S2 confirms on Windows that the saved theme is read back.

## Execution

| Phase | Work | Who |
|---|---|---|
| 0 | Commit this plan (`docs: plan the cargo group bump from Dependabot PR #3`). | main session |
| 1 | Task 1, one commit | main session (Cargo files taken whole, one line removed, one test reworked) |
| 2 | Task 2: smoke pass on the debug build | `tester`. It has no Skill tool: tell it to `Read` `.claude/skills/drive-app/SKILL.md` first and follow it. Raw results only. |
| — | Triage | main session |
| 3 | Task 3: close-out docs | main session |

Serial. Stop and ask the owner only on:
- PR #3's head no longer touching exactly the two Cargo files, or `main`'s Cargo files having changed since `528ca95` (Dependabot rebased onto something new: re-derive the bump rather than guess);
- Task 1's before/after render diff showing any difference;
- clippy after the checkout showing anything besides the `tagfilter` deprecation;
- a smoke failure that cannot be triaged.

## File Map

| File | Task | What changes |
|---|---|---|
| `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` | 1 | Exactly PR #3's versions |
| `src-tauri/src/render.rs` | 1 | `o.extension.tagfilter = true;` removed; `iframes_do_not_survive` becomes `tagfiltered_tags_do_not_survive`, covering all nine tags |
| `docs/open-items.md`, `docs/closed-items.md` | 3 | The PR #3 item moved to closed, with Task 1's and Task 2's results |
| `docs/plans/cargo-group-bump.md` → `docs/archive/plans/` | 3 | This plan, archived with a status line |

---

### Task 1: Take the bump, drop `tagfilter`

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` (whole files, from PR #3)
- Modify: `src-tauri/src/render.rs:18` and the test at `render.rs:543-549`

**Interfaces:** none. No signature changes.

- [ ] **Step 1: Widen the raw-HTML guard, on the current tree.** In `render.rs`, replace the whole of `iframes_do_not_survive` (doc comment included) with:

```rust
    /// None of the tags GFM's tagfilter singles out reach the page, inline or
    /// as a block. That extension is gone — deprecated in comrak 0.55 and a
    /// no-op here anyway, since raw HTML is dropped whole — so this pins what
    /// it used to promise.
    #[test]
    fn tagfiltered_tags_do_not_survive() {
        for tag in [
            "title", "textarea", "style", "xmp", "iframe", "noembed", "noframes", "script",
            "plaintext",
        ] {
            let md = format!("a <{tag}>x</{tag}> b\n\n<{tag}>\nblock\n</{tag}>\n");
            let html = render(&md);
            assert!(!html.contains(&format!("<{tag}")), "{tag}: {html}");
        }
    }
```

Run: `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked tagfiltered_tags`
Expected: PASS. It is a guard, not a red test: it must hold before and after.

- [ ] **Step 2: Record the "before" renders.** Append this throwaway test inside `render.rs`'s `mod tests` (Step 6 deletes it):

```rust
    // THROWAWAY — deleted before commit.
    #[test]
    #[ignore]
    fn dump_renders() {
        let out = std::path::PathBuf::from(std::env::var("T4_DUMP").unwrap());
        std::fs::create_dir_all(&out).unwrap();
        let mut docs: Vec<std::path::PathBuf> = std::fs::read_dir("../examples")
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "md"))
            .collect();
        docs.push("../README.md".into());
        docs.push("../docs/open-items.md".into());
        docs.push("../docs/closed-items.md".into());
        for p in docs {
            let html = render(&std::fs::read_to_string(&p).unwrap());
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            std::fs::write(out.join(format!("{name}.html")), html).unwrap();
        }
        let autolinks = "Visit https://example.com/a_(b) or www.example.org/x?y=1.\n\n\
                         Mail someone@example.com, see <https://example.net>.\n\n\
                         Trailing punctuation: https://example.com/end). And http://a.b/c*.\n";
        std::fs::write(out.join("autolinks.html"), render(autolinks)).unwrap();
    }
```

Run, with `<S>` the session's scratchpad directory (absolute, forward slashes):
`T4_DUMP=<S>/before rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked dump_renders -- --ignored`
Expected: PASS, and `<S>/before` holds one `.html` per document plus `autolinks.html`.

- [ ] **Step 3: Take PR #3's Cargo files.** Fetch its branch and check its scope first:

```bash
git -C . fetch origin dependabot/cargo/src-tauri/cargo-22ef10ad04
git -C . diff --stat 528ca95 FETCH_HEAD
git -C . diff --stat 528ca95 HEAD -- src-tauri/Cargo.toml src-tauri/Cargo.lock
```

Expected: the first diff lists exactly `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`; the second prints nothing. Anything else: stop and ask (see Execution).

```bash
git -C . checkout FETCH_HEAD -- src-tauri/Cargo.toml src-tauri/Cargo.lock
```

Check the versions landed:
`grep -A1 -E '^name = "(comrak|dirs|windows-sys|tauri-plugin-updater)"$' src-tauri/Cargo.lock`
Expected: comrak 0.55.0, dirs 7.0.0, tauri-plugin-updater 2.11.0, and a windows-sys 0.61.2 among the windows-sys entries (others stay; tauri pulls its own).

- [ ] **Step 4: See the red.**
Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked -- -D warnings`
Expected: exactly one error, `use of deprecated field ... tagfilter` at `src/render.rs:18`. Anything else: stop and ask.

- [ ] **Step 5: Drop the option.** In `options()` in `render.rs`, delete the line:

```rust
    o.extension.tagfilter = true;
```

Nothing replaces it: `render.unsafe_` stays off (see the doc comment on `render_titled`), and Step 1's test pins the result.

- [ ] **Step 6: Record the "after" renders and compare.**
Run: `T4_DUMP=<S>/after rtk proxy cargo test --manifest-path src-tauri/Cargo.toml --locked dump_renders -- --ignored`
Then: `rtk proxy diff -r <S>/before <S>/after && echo IDENTICAL` (`rtk proxy`: the hook otherwise rewrites `diff`'s output)
Expected: `IDENTICAL`. Any difference: stop and ask, with the diff.
Then delete the `dump_renders` test (the whole `// THROWAWAY` block).

- [ ] **Step 7: Gates.** Run all four from Global Constraints. Expected: all pass; the test count is the same as before this task (Step 1 replaced one test, Step 6 removed the throwaway).

- [ ] **Step 8: Commit.**

```bash
git -C . add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/render.rs
git -C . commit -m "chore: bump the cargo group from Dependabot PR #3, dropping comrak's deprecated tagfilter" -m "Co-Authored-By: <executing model line>"
```

### Task 2: Smoke pass on the debug build

**Files:** none changed. Scratch files only.

Follow `.claude/skills/drive-app/SKILL.md` for build (`TAURI_CONFIG` identifier override), launch, backups, and `"reopen": "off"`. Report raw values for each check.

- [ ] **S1 — boots.** Launch with no file. First screenshot shows the empty screen, themed, not blank.
- [ ] **S2 — settings read from the same place (`dirs` 7).** `eval "state.theme"` equals the `"theme"` in the backed-up `config.json`.
- [ ] **S3 — render and recovered anchors.** `openTab` on `examples/kitchen-sink.md` (forward slashes). Screenshot; read it. Then `openTab` on `examples/defect-96194-app-store-does-not-show-up-in-search-results.md`, which has 18 raw `<a id>` anchors: report `document.querySelectorAll('#content [id]').length`, then `click` the first `#content a[href^="#"]` whose target is one of those recovered ids, and report `scrollY` before and after (it must move).
- [ ] **S4 — task-list write-back.** Copy `examples/kitchen-sink.md` to the scratchpad, open the copy, click its first `input[type=checkbox]`, and report the first `- [` line of the file before and after.
- [ ] **S5 — updater (2.11).** Open Settings, click "Check now". Report the status text. Expected: "You are up to date." (1.6.6 is the latest release).
- [ ] **S6 — `WindowFromPoint` (`windows-sys` 0.61, `hwnd_at` in `main.rs`).** A tear-off does not prove it: a failed lookup also ends in a tear-off. The proof is a drop *into another window*:
  1. With three tabs open in `main`, `drag` one 250 px down: it becomes window `w1` (the skill's recipe; checked this way on 2026-09-21).
  2. Move `w1` with Win32 `MoveWindow` from PowerShell so the two windows do not overlap (the skill: Rust uses `WindowFromPoint`, so an overlapped target is never hit).
  3. From `main`'s CDP session, `drag` one of its tabs to a point on `w1`'s tab strip, mapped into `main`'s client CSS px with `invoke("window_origin")` on both windows (the skill's cross-window recipe).
  4. Report both windows' `tabs.length` after each step. Expected at the end: `w1` has 2 tabs and `main` has 1.
- [ ] **Restore.** Close the app, diff-before-restore per the skill, and `cmp` both files against their backups. No gate re-run: Task 1 ran them on this tree.

### Task 3: Close out

**Files:** `docs/open-items.md`, `docs/closed-items.md`

- [ ] **Step 1:** Move "Dependabot's cargo PR #3 fails all three CI legs" from `open-items.md` (*Bugs*) to the end of `closed-items.md`'s *Bugs*, ticked, with an italic closing note: the commit hash, the measured `tagfilter` no-op, the render diff result, and S1–S6's values. Say that PR #3 closes by itself once `main` is pushed, and that the updater's Windows spawn-error change first runs on the update *after* the one that installs this build.
- [ ] **Step 2:** `git mv docs/plans/cargo-group-bump.md docs/archive/plans/cargo-group-bump.md`, and add a status line under its title in the style of the other archived plans: `> **Status (<date>):** executed in full — <Task 1's hash>; paths and unticked boxes below are as they stood when it was written.` Note any deviation from the plan in the same line.
- [ ] **Step 3:** Commit: `docs: close the Dependabot PR #3 item`.
