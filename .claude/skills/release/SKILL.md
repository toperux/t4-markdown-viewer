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
   commit. Check `git status` is otherwise clean.
2. **Verify** — `cargo test --manifest-path src-tauri/Cargo.toml`. CI runs the
   same suite on all three platforms, so a failure here is a failure there. The
   release build no longer runs the tests itself, so this and the CI run on the
   pushed commit are the only ones between the work and the tag.
3. **Bump** `Cargo.toml`, then `cargo check` to refresh `Cargo.lock`. Plain
   `x.y.z` only — the rpm tooling rejects a `-rc.1` suffix, and the workflow
   refuses such a tag, but only once the tag is already public. Catch it here.
4. **Commit the bump on its own**, touching only those two files:

   ```
   Release 1.3.0

   <a short paragraph in prose about what this release gives the reader —
   not a changelog; the release page generates one from the commits.>
   ```

5. **Tag and push:**

   ```sh
   git tag v1.3.0
   git push origin main
   git push origin v1.3.0
   ```

   Push the branch before the tag, or CI builds a commit GitHub does not have.
   The release workflow waits for that commit's CI run and refuses to publish
   unless it is green, so a red or cancelled CI run means a failed release
   rather than a broken one.

## Checking the packaging without burning a version

`workflow_dispatch` on the Release workflow builds every artifact and skips the
publish job (`if: startsWith(github.ref, 'refs/tags/')`). Use it when the doubt
is about packaging rather than code.

## Release notes

`generate_release_notes: true` writes the changelog from commits since the last
tag, and the workflow appends a fixed body — install instructions per platform,
the SmartScreen and Gatekeeper caveats, checksum verification. That body lives
in `release.yml` and is edited there, not per release.

The updater's `latest.json` is written by the publish job, which is the only one
holding all three platforms' signatures at once. It cannot carry the real notes
(it runs before the release exists), so it links to the tag.

## Signing

`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are repo
secrets. Without them the build still succeeds but emits no `.sig` files, and
staging fails loudly rather than shipping a release no installed copy can
accept. If a build fails at staging, suspect the secrets first.

## Gotchas

- **Icon changes need `touch src-tauri/build.rs`.** `tauri-build` does not
  declare the icon files as build inputs, so a rebuild silently keeps the old
  icon embedded in the exe. Verify with `[System.Drawing.Icon]::ExtractAssociatedIcon`.
- **Nothing is code-signed.** Expect SmartScreen on Windows and a quarantine
  flag on macOS; both are documented in the release body already.
- **`.deb`/`.rpm` installs do not self-update** — `update.rs` reports them as
  not installable and sends the user to the download page instead.
- The macOS updater takes the `.app.tar.gz`, not the `.dmg`. Humans take the
  `.dmg`. Both are published.
