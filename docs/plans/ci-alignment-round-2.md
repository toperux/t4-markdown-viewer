# CI/CD alignment, round 2 — t4-markdown-viewer (mdv)

## Context

This repo is one of three t4 projects (with `t4-claude-session-browser` and `t4-git-ui`) whose
GitHub Actions workflows are kept to a single shared shape. Round 1 landed here in commits
`0290e08`, `9bad897`, `520a385` and `da45f30`; `docs/plans/ci-alignment.md` is its record and
stays as-is.

Round 1 worked, but each of the three repos was executed by someone who could not see the
other two, and all three independently patched the same gap — Dependabot PRs arriving with no
checks — in three incompatible ways. Round 2 removes that drift.

**Two things this repo's executor added beyond its round-1 plan were right, and are now the
shared shape**: `--locked` in CI (D11 below) is adopted by all three. The `gh run watch` gate
is superseded by a cleaner mechanism (D12) that does the same job without polling the Actions
API — the *problem* it identified was real and the fix is being generalised, not discarded.

**mdv-specific:** Tauri 2, vanilla-JS frontend (no `package.json`, no npm), a Tauri updater
with signed artifacts and a `latest.json` manifest. The signing env, `.sig` staging, per-asset
`.sha256` sidecars, asset names and the manifest are all load-bearing for the updater and are
**not** touched.

Principles:

1. **Surgical.** Every changed line traces to a checklist item below.
2. **Don't touch what the updater depends on.** Asset names, `.sig`/`.sha256` layout,
   `latest.json`, signing env.
3. **Do not invent improvements.** If you find something worth changing that is not in this
   plan, do not apply it — write it under *Deviations and findings* at the bottom. Round 1
   drifted precisely because good local judgment was applied in three places at once. A
   finding recorded there gets picked up by the next master pass and applied to all three.

## Round 2 decisions (shared across all three repos)

| # | Decision | Resolution | Applies here |
| --- | --- | --- | --- |
| D10 | Dependabot | **Drop it everywhere.** Its only output is PRs; no repo runs anything on `pull_request`, so bumps arrive unchecked. What matters is **security alerts** — a repo setting, no config file, no PRs, no Actions minutes. Action majors don't rot silently: GitHub annotates runs on retiring runtimes. | yes — delete `dependabot.yml` **and** the `dependabot/**` push branch added for it |
| D11 | `--locked` in CI | **All three.** Supersedes round 1's D8 "release only". This repo already has it; the other two are adopting it. | yes — already present, keep, and it moves into `checks.yml` |
| D12 | Gating a release on the checks | **A reusable `checks.yml` (`on: workflow_call`), called by both `ci.yml` and `release.yml`, gating `publish`.** Replaces this repo's `gh run watch` gate. | yes — new file, new job, old gate removed |
| D13 | Semver regex in the `version` job | **Tag refs only.** This repo already does it; csb and git-ui are adopting it. | yes — already correct, no change |
| D14 | Action majors | `checkout@v7`, `setup-node@v7`, `upload-artifact@v7`, `download-artifact@v8`. `action-gh-release` stays `@v2` in all three deliberately. | yes — already v7/v7/v8; nothing to bump |

Round 1 decisions D1–D9 still hold and are already implemented here. D8 is superseded by D11.

### Why the `gh run watch` gate goes

It was the right instinct — D7 removed the release build's tests on the promise that CI had
tested the commit, and nothing was checking that promise. But asking the Actions API after the
fact has three failure modes the replacement does not:

- a tag pushed before `main` — the loop burns 2 minutes and then errors;
- a CI run **cancelled** by a later push to `main` (this repo runs `cancel-in-progress: true`,
  so that is a live case) — the gate fails on a commit that is probably fine;
- a tagged commit that `paths-ignore` skipped entirely — no run exists to find.

Calling the checks directly sidesteps all three: they simply run, at the tagged commit. It
also drops the `actions: read` permission and the polling loop.

## 1. New file — `.github/workflows/checks.yml`

The check matrix moves here verbatim from `ci.yml`, `--locked` and all. This becomes the only
place the matrix is defined.

```yaml
name: Checks

# Called by ci.yml on a push and by release.yml on a tag. Referenced by a local
# `./` path, so it always runs at the caller's commit - which is what lets a
# release verify the exact tree it is about to publish.
on:
  workflow_call:

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: ${{ matrix.os }}
    # A called workflow runs with the caller's token, and release.yml grants
    # `contents: write` for `publish`. Without this the check matrix would
    # inherit write access it has no use for; it only builds and tests.
    permissions:
      contents: read
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
      #
      # `--locked` because the release build passes it too: without it here a
      # stale `Cargo.lock` goes green, and then kills every release leg after
      # the tag is already public.
      - name: Clippy
        run: cargo clippy --workspace --all-targets --locked -- -D warnings

      - name: Test
        run: cargo test --workspace --locked
```

## 2. Replace `.github/workflows/ci.yml`

Drops to triggers plus a call. The `dependabot/**` push branch is **gone** (D10) — it existed
only so Dependabot's branches would get checks, and Dependabot is going. The comment about
`ea40fe6` goes with it; `main` is the only branch again.

```yaml
name: CI

on:
  push:
    branches: [main]
    # A docs-only push has nothing here to check. Mixed commits still run.
    # This cannot let a tagged commit through unchecked: release.yml calls
    # checks.yml on every tag, whatever paths the commit touched.
    paths-ignore:
      - "docs/**"
      - "**/*.md"
  workflow_dispatch:

# One run per push: a burst of pushes to main cancels the superseded runs
# instead of queueing a full 3-OS matrix for each.
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  check:
    uses: ./.github/workflows/checks.yml
```

## 3. `.github/workflows/release.yml` changes

- [ ] **Delete the `Require a green CI run for this commit` step** from the `version` job
      (the whole step, including its leading comment).
- [ ] **Delete `actions: read`** from `permissions:`, and the comment above it referencing
      `gh run list` / `gh run watch`. `permissions:` goes back to `contents: write` alone.
- [ ] **Add a `checks` job** calling the reusable workflow, and make `publish` wait for it.
      `build` keeps `needs: version` only, so builds and checks run concurrently:

  ```yaml
  jobs:
    version:
      ...

    # The release build does not run the tests (D7), so the checks run here
    # instead, at this exact commit. They gate `publish`, not `build`: the two
    # run side by side, and a failure means the artifacts exist but nothing is
    # published - including no latest.json, so no installed copy is offered a
    # release that did not pass.
    #
    # Tags only. All this job protects is `publish`, and `publish` is itself
    # tag-only, so on a workflow_dispatch packaging run the matrix would be
    # three legs guarding nothing. Skipping it here also skips `publish`, which
    # is what the dispatch wanted anyway.
    checks:
      if: startsWith(github.ref, 'refs/tags/')
      uses: ./.github/workflows/checks.yml

    build:
      needs: version
      ...

    publish:
      needs: [version, checks, build]
      if: startsWith(github.ref, 'refs/tags/')
      ...
  ```

- [ ] The `version` job's `read` step is **already correct** for D13 (regex inside the tag
      branch). Leave it.
- [ ] Nothing else. Action versions are current; `action-gh-release@v2` stays. The matrix,
      signing env, `Clear stale bundle output`, staging, `.sha256` sidecars, `latest.json` and
      the release-notes body are untouched.

## 4. Delete `.github/dependabot.yml` (D10)

- [ ] `git rm .github/dependabot.yml`.
- [ ] Then, **in the GitHub UI or API, turn on Dependabot security alerts** for
      `toperux/t4-markdown-viewer`. This is the part of Dependabot worth having and it is
      currently off in at least one of the three repos (checked on git-ui, 2026-09-03); this
      repo was never checked. Settings → Advanced Security → Dependabot alerts, or:

  ```sh
  gh api -X PUT repos/toperux/t4-markdown-viewer/vulnerability-alerts
  ```

  Record below whether it was already on.

- [ ] **Do not enable Dependabot security updates** (`automated-security-fixes`). Alerts
      notify; security *updates* open pull requests, and after this round nothing runs on
      `pull_request` in any of the three repos — so such a PR would arrive with no checks at
      all, which is the exact hole D10 closes. A security fix is the last thing to merge
      unchecked. When an alert fires, bump it by hand (`cargo update -p <crate>` from
      `src-tauri`), push to `main`, and the full matrix runs.

## 5. Update `.claude/skills/release/SKILL.md`

The skill describes the release workflow, and two things it says are about to stop being true.

- [ ] Anywhere it describes the release waiting on / checking a CI run, replace with: the
      Release workflow runs the checks itself, at the tagged commit, and will not publish
      unless they pass.
- [ ] "Push the branch before the tag, or CI builds a commit GitHub does not have" — still
      true and still the right advice, but the consequence has changed: the release no longer
      *fails* for want of a CI run, it just runs the checks itself. Adjust if the wording
      implies otherwise.
- [ ] Round 1 already reduced the version table to `Cargo.toml` + `Cargo.lock`; leave that.
- [ ] Read the whole file before editing — do not rewrite sections this plan does not name.

## 6. Risks to verify

- **Reusable-workflow resolution.** `uses: ./.github/workflows/checks.yml` resolves at the
  caller's commit. On the first push this means `checks.yml` must exist in the same commit as
  the `ci.yml` that calls it — land section 1 and 2 in one commit, not two.
- **Job naming.** The matrix legs now appear as `check / ${{ matrix.os }}`. Cosmetic; do not
  add `name:` overrides chasing the old labels.
- **`defaults.run.working-directory` inside a reusable workflow.** It is job-level here, which
  is unchanged from the current `ci.yml` — it should carry over as-is. If any step suddenly
  runs from the repo root, that is the cause.
- **Workflow-level `env` does not cross into a called workflow.** That is why `checks.yml`
  carries its own `env: CARGO_TERM_COLOR: always` rather than relying on `release.yml`'s. Do
  not delete it as a duplicate.
- **A tagged release now costs a check matrix on top of the builds.** Wall-clock is unchanged
  (they run alongside `build`), but a release spends three more legs of Actions minutes than
  it did under round 1 — the old `gh run watch` gate only watched an existing run. That is the
  price of not depending on the Actions API; it is not a mistake to be optimised away.
  `workflow_dispatch` runs are unaffected — `checks` is tag-gated.
- **Recovering from a failed check on a tag.** `build` will have succeeded and uploaded
  artifacts; `publish` will not have run, so no release, no `latest.json`, and no installed
  copy is offered anything. Fix the commit, delete the tag locally and on the remote
  (`git tag -d vX.Y.Z && git push origin :refs/tags/vX.Y.Z`), then re-tag. Do not re-run only
  the failed job to force a publish.
- **`ubuntu-22.04` retirement — still open, cross-repo.** Deprecated from 2026-09-17,
  brownouts 2027-03-23 / -03-30 / -04-06 / -04-13 (14:00–00:00 UTC), unsupported 2027-04-17
  (`actions/runner-images#14254`). Deliberate here for the glibc floor. **Not in scope for
  round 2** — do not switch it. The fix has to be picked once for all three.

## 7. Commit plan

1. `ci: run the checks from one reusable workflow` — sections 1 and 2 **in a single commit**
   (see Risks).
2. `release: gate publishing on the checks` — section 3, plus section 5's skill edits.
3. `ci: drop dependabot` — section 4's file deletion. The alerts setting is a repo setting,
   not a commit.

Do not tag a release as part of this work.

## 8. Verification

- [ ] Push to a branch, then `workflow_dispatch` **CI** on it. Three legs green, shown as
      `check / <os>`.
- [ ] Confirm `Format` ran on `ubuntu-22.04` only, and that clippy and test show `--locked`.
- [ ] `workflow_dispatch` **Release** on the branch. `version` green; three `build` legs
      green; `checks` **skipped** and `publish` skipped (both are tag-gated). No `Require a
      green CI run` step in the log, and no `actions: read` in `permissions:`.
- [ ] The `build`-and-`checks`-run-concurrently behaviour cannot be observed from a dispatch
      run, since `checks` is skipped there. It is first exercised by a real tag; confirm the
      timings then.
- [ ] Download `packages-Windows`: installer still named
      `T4-Markdown-Viewer_<version>_x64-setup.exe` with a `.sig` beside it — signing is
      untouched by this round, so a change here means something went wrong.
- [ ] Confirm `.github/dependabot.yml` is gone and `ci.yml` triggers on `main` only.
- [ ] Merge to `main`; CI green on `main`.
- [x] Dependabot security alerts on for this repo. Were they already on? **No — both were
      off.** `GET /vulnerability-alerts` returned 404 and `automated-security-fixes` returned
      `{"enabled": false, "paused": false}` (checked 2026-09-04). Alerts are now on;
      `automated-security-fixes` was deliberately left off, see Deviations.
- [ ] The next real release exercises `publish`; nothing to do now.

## 9. Deviations and findings

> Anything you changed that this plan did not ask for, and anything you noticed that the other
> two repos should probably also do. **Record here; do not act on it beyond this repo.** The
> next master pass reads this section. Round 1's `--locked`-in-CI improvement was found this
> way and is now shared — but it was found by reading the diff, not because anyone wrote it
> down. If this section is empty, say so explicitly.

Sections 1–5 were implemented exactly as written; no code or workflow line departs from the
plan. Three things for the next master pass:

1. **Section 4 contradicts D10, and the contradiction is in the shared decision, not just
   here.** D10 argues Dependabot goes because "no repo runs anything on `pull_request`, so
   bumps arrive unchecked", and sells alerts as "no PRs, no Actions minutes". But section 4's
   second command, `PUT /automated-security-fixes`, is precisely the switch that makes
   Dependabot open PRs again — security-fix PRs, which land with no checks for exactly the
   reason D10 gives. Only `PUT /vulnerability-alerts` was run here; `automated-security-fixes`
   was left off (it was already off). **All three repos need the same answer** — either
   section 4 drops the second command, or D10 stops claiming "no PRs". Alerts alone still
   notify on a vulnerable dependency, which is the part D10 says is worth having.

2. **`ci.yml` no longer needs a workflow-level `env:`** now that it has no steps of its own —
   the plan's target file correctly omits it, and `checks.yml` carries its own. Worth stating
   outright in the other two repos' plans, since deleting a workflow's `env:` looks like an
   oversight in review rather than a consequence of the split. `release.yml` keeps its `env:`
   because its `build` job still runs cargo directly.

3. **`publish: needs: [version, checks, build]` is safe when `checks` is skipped**, but only
   incidentally: a `needs` on a skipped job skips the dependent, and `publish` is tag-gated
   anyway, so the two conditions agree. If any repo ever makes `publish` run outside a tag,
   that agreement breaks silently and publishing would skip rather than fail. Not worth
   changing now; worth knowing before anyone loosens `publish`'s `if:`.

Not done, and outside this plan's scope: `docs/plans/ci-alignment-round-2.md` — this file — is
untracked, and section 7's commit plan does not include committing it. Round 1's plan file is
tracked in the repo. Left untracked rather than inventing a fourth commit; the repo owner
should decide whether these plans belong in git.
