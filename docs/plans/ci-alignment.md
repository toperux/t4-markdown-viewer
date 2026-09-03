# CI/CD alignment plan — t4-markdown-viewer (mdv)

## Context

This repo is one of three t4 projects whose GitHub Actions workflows are being aligned to a
single shared shape. The other two — `t4-claude-session-browser` (pure Rust, egui) and
`t4-git-ui` (Tauri 2 + npm, cargo workspace) — are out of scope here and have their own copies
of this plan. Everything below is self-contained; the shared shape is authoritative, and where
this repo already matches it, the item says "keep".

mdv is a Tauri 2 app with a vanilla-JS frontend (no `package.json`, no npm), a Tauri updater
with signed artifacts, and a `latest.json` manifest written at publish time. The updater
verifies each download against a `.sig` and the per-asset `.sha256` sidecars are what humans
verify with — those, the asset names, the signing steps and the manifest all stay exactly as
they are. This repo's release workflow is the *reference* the other Tauri repo is being aligned
to, so most of the release changes here are small.

Principles for every edit:

1. **Surgical.** Every changed line traces to a checklist item below. No drive-by rewrites of
   the release-notes body, comments, or unrelated steps.
2. **Don't touch what the updater depends on.** Asset names, `.sig`/`.sha256` layout,
   `latest.json`, signing env.
3. If this plan turns out to be wrong about something (a path, a runner name, an action
   version), fix the plan file in the same commit so it stays truthful.

## Decisions (resolved 2026-09-03, shared across all three repos)

| # | Decision | Resolution | Applies here |
| --- | --- | --- | --- |
| D1 | `paths-ignore` docs/md on CI | **Adopt in all three.** No `.md` test fixtures anywhere; no required-status-check rules to hang. | yes |
| D2 | Which leg runs `cargo fmt --check` | **Linux leg only.** Same code on every leg, so run it once on the cheapest runner. | yes — moves from Windows |
| D3 | git-ui asset naming | `T4-Git-UI_<ver>_<suffix>`, mirroring this repo. | no |
| D4 | git-ui updater | No. | no |
| D5 | Action pinning | **Majors + dependabot.** Verify current majors first (see Risks). | yes |
| D6 | Universal macOS for git-ui | Universal, as here. | no — already universal |
| D7 | Tests inside release builds | **None.** CI already tests the commit; the release skill's "push main before the tag" is the guard. | yes — remove the `Test` step |
| D8 | `--locked` on release builds | **Release builds only.** CI unchanged. | yes — add |
| D9 | Drop `"version"` from `tauri.conf.json` | **Yes**, own commit. Tauri 2 falls back to the crate's `Cargo.toml` version. 3 → 2 hand-synced files. | yes |
| — | Windows bundle | NSIS only, no `.msi`. | yes — already NSIS only |

## 1. Target `.github/workflows/ci.yml` (complete file)

Replace the file with this. Changes from today: `paths-ignore` (D1), `env:` moved above
`concurrency:` (shared order), `checkout@v7` (the plan first said `@v5`; v7 is the current
major — see Risks), Format step gated on Linux instead of Windows (D2), `--workspace` on
clippy and test (a no-op for a single crate; it keeps the line identical across repos).
Everything else — `working-directory`, `rust-cache workspaces`, Linux deps, matrix,
comments — is preserved.

```yaml
name: CI

on:
  push:
    branches: [main]
    # A docs-only push has nothing here to check. Mixed commits still run.
    paths-ignore:
      - "docs/**"
      - "**/*.md"
  workflow_dispatch:

env:
  CARGO_TERM_COLOR: always

# One run per push: a burst of pushes to main cancels the superseded runs
# instead of queueing a full 3-OS matrix for each.
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  check:
    name: ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        # 22.04 rather than latest: the .deb links against whatever glibc the
        # builder has, so building on the oldest supported runner is what makes
        # the package installable on more than just the newest distros.
        os: [windows-latest, macos-latest, ubuntu-22.04]
    defaults:
      run:
        working-directory: src-tauri

    steps:
      - uses: actions/checkout@v7

      - name: Linux build dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
            libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      # Formatting is platform-independent, so check it once, on the cheapest runner.
      - name: Format
        if: runner.os == 'Linux'
        run: cargo fmt --check

      # `-D warnings` goes after `--`, not in RUSTFLAGS: as an env var it also
      # applies to every dependency, so one warning in a crate we do not own
      # turns the build red.
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: Test
        run: cargo test --workspace
```

## 2. D9 — read the version from `Cargo.toml` only (own commit, **before** section 3)

Tauri 2 uses the crate's `Cargo.toml` version when `tauri.conf.json` has no `version` key.
Today the version is hand-synced across `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`
and `src-tauri/Cargo.lock`, and the `version` job fails the release if the first two disagree.
After this commit there is one source plus the lock.

- [ ] `src-tauri/tauri.conf.json`: delete the `"version": "…",` line. Nothing else.
- [ ] Locally: `cargo tauri build --bundles nsis` (or whichever bundle is cheap on your
      machine) and confirm the produced installer's filename carries the `Cargo.toml` version.
      If it does not, Tauri did not pick the version up — stop, restore the key, and note it
      in this plan.
- [ ] `.claude/skills/release/SKILL.md`, section "The version lives in three files": reduce
      the table to two rows (`src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`), retitle it
      "two files", and drop the sentence saying CI compares `Cargo.toml` against
      `tauri.conf.json`. The paragraph explaining *why* the `version` job exists (a v1.3.0
      release shipping a 1.2.0 installer) still holds — it now compares tag ↔ `Cargo.toml`.
      Step 3 of "Steps" ("Bump the two files") becomes "Bump `Cargo.toml`".
- [ ] Commit: `Read the version from Cargo.toml only`.

## 3. `.github/workflows/release.yml` changes

Exact edits, top to bottom:

- [ ] `actions/checkout@v4` → `@v7` in **both** the `version` and `build` jobs.
- [ ] `version` job, `read` step — replace the body so it reads one file, rejects anything
      that is not plain `x.y.z` (rpm tooling rejects `-rc.1`; no prerelease tag has ever been
      cut here, so nothing existing breaks), and compares against the tag:

  ```bash
  set -euo pipefail
  crate=$(grep -m1 '^version = ' src-tauri/Cargo.toml | cut -d'"' -f2)
  # Plain x.y.z only: the rpm tooling rejects a `-rc.1` suffix, and the
  # updater compares versions as released.
  if ! [[ "$crate" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "version $crate is not a plain x.y.z release" >&2
    exit 1
  fi
  # workflow_dispatch has no tag, so there is nothing to compare against.
  if [[ "${GITHUB_REF}" == refs/tags/* ]]; then
    tag="${GITHUB_REF_NAME#v}"
    if [[ "$tag" != "$crate" ]]; then
      echo "tag $tag does not match Cargo.toml version $crate" >&2
      exit 1
    fi
  fi
  echo "version=$crate" >> "$GITHUB_OUTPUT"
  ```

  Update the job's leading comment: it no longer mentions `tauri.conf.json`; the tag and
  `Cargo.toml` must agree.
- [ ] `build` job: **delete** the `Test` step (`working-directory: src-tauri` / `cargo test`)
      — D7.
- [ ] `build` job, "Build the packages" step: append `-- --locked` so the `run:` becomes

  ```yaml
  run: >
    cargo tauri build --bundles ${{ matrix.bundles }}
    ${{ matrix.target && format('--target {0}', matrix.target) || '' }}
    -- --locked
  ```

  Arguments after `--` are passed through to `cargo build`. `src-tauri/Cargo.lock` is tracked,
  so `--locked` has something to hold to.
- [ ] `actions/upload-artifact@v4` → `@v7`, `actions/download-artifact@v4` → `@v8` (current
      majors; see Risks). They are a matched pair: upload still zips by default, and download
      v8 unzips or not based on the response's content type.
- [ ] Nothing else. The matrix, signing env, `Clear stale bundle output`, staging, `.sha256`
      sidecars, `latest.json`, and the release-notes body stay as they are.

## 4. Non-workflow changes

- [ ] **`.github/dependabot.yml`** (new, optional, own commit):

  ```yaml
  version: 2
  updates:
    - package-ecosystem: github-actions
      directory: /
      schedule:
        interval: monthly
    - package-ecosystem: cargo
      directory: /src-tauri
      schedule:
        interval: monthly
  ```

- [ ] `.claude/skills/release/SKILL.md` — already covered in section 2. Additionally, its
      "Steps → 2. Verify" still tells the human to run `cargo test` locally; that becomes the
      only test run before a release now that the build job no longer tests (D7). Add one
      sentence there saying so.

## 5. Risks to verify

- **`ubuntu-22.04` runner lifetime — CONFIRMED RETIRING; unresolved, needs a cross-repo
  decision.** Both workflows build Linux on it so the `.deb` links against an old glibc.
  GitHub (actions/runner-images#14254) has it deprecated from **2026-09-17** and fully
  unsupported on **2027-04-17**, with brownouts — jobs on the label simply fail — on
  **March 23, March 30, April 6 and April 13, each 14:00–00:00 UTC**. Between now and then
  it keeps working, with longer queue times.

  Per this plan's own instruction the label was **left alone** in both workflows rather than
  silently switched to `ubuntu-latest`, which would raise the `.deb`'s glibc floor from 2.35
  to whatever the newest image carries and quietly drop older distros. The fix — bump to
  `ubuntu-24.04`, or keep 22.04 via `container: ubuntu:22.04`, or `cargo-zigbuild` — has to be
  chosen once and applied to all three repos. Nothing is urgent before March's brownouts.
- **D9 version pickup — resolved.** With the key gone, `cargo tauri build --bundles nsis`
  produced `T4 Markdown Viewer_1.4.7_x64-setup.exe`, i.e. the `Cargo.toml` version.
- **`-- --locked` pass-through — resolved.** Verified locally against tauri-cli 2.11.4: the
  build runs and bundles as before, so the argument reaches `cargo build`. CI installs
  `tauri-cli@^2`, so the same holds there.
- **Action majors — resolved 2026-09-03.** The guesses in this plan were stale; the current
  majors are `checkout@v7` (v7.0.1), `upload-artifact@v7` (v7.0.1) and
  `download-artifact@v8` (v8.0.1), all pinned at the major. upload v7 and download v8 are
  the matched pair: v7 added opt-in unzipped single-file uploads (`archive: false`, not used
  here, so uploads are still zipped) and v8 decides whether to unzip from the content type,
  so the default path is unchanged. v8 also fails on a download digest mismatch instead of
  warning, which is what we want for signed artifacts.

## 6. Commit plan

In this order, each its own commit:

1. `docs: add CI alignment plan` — this file. (May be squashed into commit 2.)
2. `ci: align workflow with the other t4 projects` — section 1.
3. `Read the version from Cargo.toml only` — section 2. **Must precede commit 4**, or the
   `version` job reads a key that is no longer there.
4. `release: align workflow with the other t4 projects` — section 3.
5. `ci: add dependabot` — section 4, if doing it.

Do not tag a release as part of this work.

## 7. Verification

Sections 1–4 are committed on `main` (locally, unpushed) as of 2026-09-03. Everything below
needs a push, so it is still outstanding — apart from the two local checks folded into Risks.

- [ ] Push commits to a branch. CI runs; all three legs green.
- [ ] In the CI run, `Format` executed on `ubuntu-22.04` only and was skipped on the other two.
- [ ] Push a commit touching only `docs/plans/ci-alignment.md`. CI does **not** run.
- [ ] Trigger `Release` via `workflow_dispatch` on the branch. Three `build` legs green, no
      `Test` step present, `publish` skipped. Download `packages-Windows` and confirm the
      installer is named `T4-Markdown-Viewer_<Cargo.toml version>_x64-setup.exe` and a
      matching `.sig` is beside it (signing still works).
- [ ] Merge to `main`. CI green on `main`.
- [ ] The next real release (via the release skill) is the end-to-end test of the `version`
      job; nothing to do now.

## 8. Post-review amendments (2026-09-03)

- **D8 now applies to CI too.** `--locked` on CI's clippy and test, not just the release
  build: otherwise a stale `Cargo.lock` is green here and red on every release leg, after
  the tag is public.
- **Dependabot branches get CI.** `on.push.branches` gains `dependabot/**`. Dropping the
  `pull_request` trigger (ea40fe6) left its PRs with no checks at all; its branches live in
  this repo, so a push trigger on them puts the matrix on the PR.
- **A release gates on a green CI run.** D7 took the tests out of the release build without
  putting anything in their place. The `version` job now finds CI's run for the tagged
  commit and waits on it (`actions: read`), so nothing is signed or published off a red,
  cancelled or missing run.
- **The x.y.z check is tag-only.** Section 3 put it before the tag branch, which stopped a
  `workflow_dispatch` build mid-bump — exactly the case dispatch exists for. It now lives
  inside the tag branch; a dispatch builds whatever version the tree has.
