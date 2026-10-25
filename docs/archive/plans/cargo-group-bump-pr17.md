# Cargo Group Bump (Dependabot PR #17) Plan

> **Status (2026-10-25):** executed in full: 02fa311 takes PR #17's `Cargo.lock` whole (scope as
> planned; gates passed, 215 tests). S1–S5 passed on Windows, macOS and Linux: boots, *Check
> now* reads "You are up to date.", the native picker opens a file, a link makes one
> `open_url` call, and a second launch opens its file in the running instance. Linux ran on a
> headless Xvfb display, since the VM's pointer input was down. No deviation; nothing left
> open. The update path with updater 2.14 first runs on the update after the release that
> ships it.

**Goal:** land Dependabot PR #17's five Tauri crate bumps on `main` before the next release, so
it ships with them.

**Spec:** PR #17 (`gh pr view 17`). The owner asked for it to go in before the release
(2026-10-25), following `archive/plans/cargo-group-bump.md`.

## Findings (measured 2026-10-25, before this plan)

PR #17's base is `ca62692`, and neither `src-tauri/Cargo.toml` nor `src-tauri/Cargo.lock` has
changed on `main` since: its lockfile can be taken as is. It changes `Cargo.lock` only (22 lines
in, 14 out); `Cargo.toml`'s ranges already allow the new versions.

| Bump | What changed | Consequence here |
|---|---|---|
| tauri-plugin-updater 2.13.3 → 2.14.0 | Only `src/updater.rs` differs (diffed in the registry). `Update::install` now runs the same `verify` as `Update::download`; AppImage updates no longer fail on a separate `/tmp` or cache mount, and a payload that isn't an AppImage is refused; `check` resolves to no update, not an error, when the release has no entry for this target; a warning for a non-https download URL; a clearer Windows installer-launch error | `update.rs` calls `download`, then `install(bytes)`: the bytes are now verified twice with the same key and the same `require_signed_version`, which `tauri.conf.json` leaves at its default, so a validly signed release passes both. The new install path first runs on the update *after* the release that ships this |
| tauri-plugin-dialog 2.8.2 → 2.8.3 | Its JS package's dependency only | Nothing on the Rust side |
| tauri-plugin-single-instance 2.5.3 → 2.5.4, tauri-plugin-opener 2.7.1 → 2.7.2, tauri-build 2.7.2 → 2.7.3 | Patch releases; no behaviour change listed | Smoke-checked below |

Not measured: the update path itself. An update needs a signed release on the GitHub endpoint,
so the 2.14 install path can't run before a release that carries it.

## Global constraints

- No dependency change beyond PR #17's. Never a bare `cargo update`.
- Gates, from the repo root, before the commit: `cargo fmt --manifest-path src-tauri/Cargo.toml
  --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --locked
  -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml --workspace --locked`,
  `node --check src/app.js`.
- Straight onto `main`, committed locally; no push without the owner's go. No AI attribution.
  Dependabot closes PR #17 by itself once `main` has the versions.
- Walks run on the peers (Windows VM, Linux VM, Mac) from a pushed `walk/*` branch, never on the
  host PC. Each backs up and restores `config.json` and `session.json` per the drive-app skill.

## Task 1: take the lockfile

1. Fetch the PR's branch and check its scope:
   `git fetch origin dependabot/cargo/src-tauri/cargo-c8b249efb4`, then
   `git diff --stat ca62692 FETCH_HEAD` (expect `src-tauri/Cargo.lock` only) and
   `git diff --stat ca62692 HEAD -- src-tauri/Cargo.toml src-tauri/Cargo.lock` (expect nothing).
   Anything else: stop and ask.
2. `git checkout FETCH_HEAD -- src-tauri/Cargo.lock`, and check the five versions landed.
3. Gates. Expect all to pass, with the test count unchanged (215).
4. Commit: `chore: bump the Tauri plugins from Dependabot PR #17`.

## Task 2: smoke walks

On one walk branch, with the debug build, on all three platforms:

- **S1, boots:** the empty screen, themed.
- **S2, updater:** Settings › *Check now* reads "You are up to date." (1.8.8 is the latest
  release and the build's version).
- **S3, dialog:** *Open file* shows the native picker; choosing a file opens it.
- **S4, opener:** an external link in a document reaches the opener (stubbed, as in the earlier
  walks: one call with the right URL, no browser opened).
- **S5, single instance:** launching the app again with a file opens it in the running
  instance, with no second process.

## Task 3: close out

Archive this plan to `docs/archive/plans/` with a status line giving the commit and S1–S5's
results, and an open-items entry if the triage leaves anything. In a `docs:` commit.

## Execution

The main session runs Task 1 (one file taken whole) and Task 3, and sends Task 2 to the peers.
Serial. Stop and ask on: PR #17 touching more than `Cargo.lock`, or `main`'s Cargo files having
changed since `ca62692`; any gate failing; any smoke failure.
