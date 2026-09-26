---
name: release
description: Cut a release of T4 Markdown Viewer — bump the version, tag it, and let CI build and publish for all three platforms. Use when asked to release, ship, cut a version, or bump the version (major/minor/patch).
---

# Releasing

Pushing a `v*` tag is the whole trigger. `.github/workflows/release.yml` then
builds Windows, macOS and Linux packages, signs them, writes the updater
manifest and publishes a GitHub release. **A published release is public and
installed copies will offer it as an update**, so get the version right before
the tag goes up.

## The version lives in two files

| File | Form |
| --- | --- |
| `src-tauri/Cargo.toml` | `version = "1.2.0"` (line 3) |
| `src-tauri/Cargo.lock` | under `name = "t4-markdown-viewer"` |

Edit the first; refresh the lock with `cargo check --manifest-path
src-tauri/Cargo.toml` rather than hand-editing it. `tauri.conf.json` has no
`version` key — Tauri falls back to the crate's, so the installer is named from
`Cargo.toml`.

The `version` job in the workflow compares the tag against `Cargo.toml`. It
exists because the failure it catches is silent otherwise: a release called
v1.3.0 shipping an installer named 1.2.0, which the updater then refuses.

## Steps

1. **Land the work first.** Feature commits are separate from the release
   commit. Check `git status` is otherwise clean. Two checks nothing else
   makes:
   - **highlight.js.** `src/vendor/highlight.min.js` is vendored, so
     Dependabot never offers a new release. Check for one; a bump swaps the
     file and its version line in `src-tauri/THIRD-PARTY-LICENSES.md`.
   - **Actions on the publish path.** If an action in the `publish` job
     (`softprops/action-gh-release`, `download-artifact`) was bumped since the
     last tag (`git diff <last tag> -- .github/workflows/release.yml`), run a
     dry run (below) first. Dependabot's pull requests run only `checks.yml`,
     so the dry run is the only place those run before a tag.
2. **Verify** — `cargo test --manifest-path src-tauri/Cargo.toml`. CI runs the
   same suite on all three platforms, so a failure here is a failure there. The
   release build does not run the tests itself; the Release workflow runs the
   same checks separately, at the tagged commit.
3. **Bump** `Cargo.toml`, then `cargo check` to refresh `Cargo.lock`. Plain
   `x.y.z` only — the rpm tooling rejects a `-rc.1` suffix, and the workflow
   refuses such a tag, but only once the tag is already public. Catch it here.
4. **Commit the bump on its own**, touching only those two files:

   ```
   Release 1.3.0

   <a short paragraph in prose about what this release gives the reader.
   This is for `git log` only — the release page is built from the feature
   commits' subject lines, not from here.>
   ```

5. **Tag and push:**

   ```sh
   git tag v1.3.0
   git push origin main
   git push origin v1.3.0
   ```

   Push the branch before the tag, or CI builds a commit GitHub does not have.
   The release does not depend on that run: it runs the same checks itself, at
   the tagged commit, and publishes nothing unless they pass.
6. **Approve the build.** The run waits at the three build legs until the owner
   approves the `signing` environment (see *Signing*).
7. **Tick first runs.** If `docs/open-items.md` has a *First runs* section, tick
   every sub-item this release (or the push before it) proved, with the run id,
   and move the item to `closed-items.md` once all are ticked. Commit as `docs:`.

## Checking the packaging without burning a version

`workflow_dispatch` on the Release workflow builds every artifact, then
rehearses the publish job: it writes `latest.json` and the notes, uploads
everything to a **draft** release tagged `dry-run-<run id>` (named "Dry run —
…"), checks the draft's assets match `dist/`, and deletes it. A draft never
creates its tag, so nothing public changes. The `checks` job is skipped. Use
it when the doubt is about packaging or the publish path rather than code.

A `dry-run-*` draft left on the Releases page means a run never got to clean
up. It is safe to delete by hand (`gh release delete dry-run-<id> --yes`).

## Release notes

The publish job builds them from `git log` since the previous tag. Every commit
subject in the range becomes a bullet, and the newest one becomes the release
title after the tag — `v1.3.0 — View SVGs and images in a tab`. Dropped from
both: the `Release x.y.z` commit itself, and anything prefixed `ci:`, `docs:`,
`release:` or `chore:`.

So the subject line of each feature commit *is* the release note. Write it for
the person reading the release page. The commit body is for whoever runs `git
log` — it does not reach the page.

Install instructions are not on the release page. They are in the README, which
is one place to correct rather than one per release.

The updater's `latest.json` is written by the publish job, which is the only one
holding all three platforms' signatures at once. It cannot carry the real notes
(it runs before the release exists), so it links to the tag.

## Signing

Every signing secret lives in the `signing` **environment**, not in repo
secrets. Only main and `v*` tags may use it, and every leg does, so a dispatch
from any other branch fails every leg. The environment also requires the
owner's approval, so a push to main alone cannot sign anything: every Release
run, tag or dispatch, waits at the build legs until the owner approves it
(Actions → the run → *Review deployments* → `signing` → *Approve and deploy*;
one approval releases all three legs). By CLI:
`gh api -X POST repos/toperux/t4-markdown-viewer/actions/runs/<run id>/pending_deployments -F 'environment_ids[]=22673426996' -f state=approved -f comment=ok`.
An unapproved run waits up to 30 days, then fails. The build is split so the secrets never
meet the compile: *Build the app* (`cargo tauri build --no-bundle`) runs every
build script and proc-macro with no secrets in env; *Bundle and sign* (`cargo
tauri bundle`) compiles nothing and holds the keys.

`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` sign the
updater `.sig` files. Without them *Bundle and sign* errors rather than
shipping a release no installed copy can accept. If it fails there, suspect
the secrets first.

`APPLE_CERTIFICATE` and `APPLE_CERTIFICATE_PASSWORD` hold the self-signed macOS
certificate shared with t4-git-ui and kept outside both repos. The workflow
imports it itself, after the compile, because Tauri's own import only accepts
Apple-named certificates. A missing or wrong one fails *Import the macOS
signing certificate*. Set them before pushing a tag. Before the first tag after
any change to the signing steps, run a `workflow_dispatch` packaging build and
check the macOS leg passes: once a tag is public the only fix is re-running
the job, never retagging, and re-running cannot fix a workflow bug. Rotating
the certificate means updating the fingerprint in *Check the macOS signature*,
which checks the bundle, the `.app.tar.gz` and the `.dmg`.

`CERTUM_EMAIL` and `CERTUM_OTP` sign the Windows exe and installer with the
Certum Open Source certificate in SimplySign's cloud, through `ssign`. Only the
Windows leg's *Bundle and sign* is handed them. `CERTUM_OTP` is the TOTP seed
from the SimplySign activation QR, and can sign as the project until it is
regenerated. A missing or wrong one fails *Bundle and sign* on Windows; a
build signed with anything else fails *Check the Windows signature*. The
certificate expires **2027-09-22**. Renewing it means updating the thumbprint
in that check, and a new QR means updating `CERTUM_OTP`.

**After any tauri-cli bump, run a dry run before the next tag.** *Pin the
AppImage tools* seeds the bundler's tool cache with hashed copies under the
file names tauri-bundler 2.9.4 uses. A bundler that renames or adds a tool
downloads it again, and *Bundle and sign* fails on the Linux leg when its log
shows a download. Update the names, URLs and hashes there, and the version in
*Install the Tauri CLI*.

## Gotchas

- **Icon changes need `touch src-tauri/build.rs`.** `tauri-build` does not
  declare the icon files as build inputs, so a rebuild silently keeps the old
  icon embedded in the exe. Verify with `[System.Drawing.Icon]::ExtractAssociatedIcon`.
- **Only Windows has a trusted signature.** SmartScreen can still warn until
  the Certum certificate builds reputation. macOS is self-signed and not
  notarized, so it has a quarantine flag. Both are documented in the README.
- **`.deb`/`.rpm` installs do not self-update** — `update.rs` reports them as
  not installable and sends the user to the download page instead.
- The macOS updater takes the `.app.tar.gz`, not the `.dmg`. Humans take the
  `.dmg`. Both are published.
