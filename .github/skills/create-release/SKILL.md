---
name: create-release
description: 'Prepare a new release of the logical-path crate by opening a PR that bumps the version in Cargo.toml and updates CHANGELOG.md. Use when asked to "cut a release", "prepare a release", "release a new version", "bump the version", or "open a release PR". Follows the documented procedure in docs/releasing.md and stops at an open PR — it never publishes, tags, or merges.'
argument-hint: 'Optional: the target version or bump type (patch/minor/major)'
---

# Create a Release

## What This Produces

An **open pull request** on a `release/vX.Y.Z` branch that:

1. Bumps `version` in [`Cargo.toml`](../../../Cargo.toml).
2. Refreshes `Cargo.lock` so it agrees with the new version.
3. Converts the `## [Unreleased]` section of [`CHANGELOG.md`](../../../CHANGELOG.md) into a dated `## [X.Y.Z]` section and adds a fresh `## [Unreleased]` above it.

The actual publish to crates.io, the git tag, and the GitHub Release all happen **automatically** after the PR is merged and CI passes. This skill deliberately stops at the open PR — do **not** merge, tag, or run `cargo publish`.

## Authoritative Reference

The full, canonical procedure lives in [docs/releasing.md](../../../docs/releasing.md). Read it before acting. This skill automates steps 1–6 of that document ("The release procedure, step by step") and intentionally does not perform steps 7–9 (merge and post-merge verification).

## Procedure

### 1. Determine the new version

Read the versioning policy in [docs/releasing.md](../../../docs/releasing.md#versioning-policy). Inspect changes since the last release:

```sh
git log --oneline "v$(cargo metadata --no-deps --format-version 1 \
    | jq -r '.packages[] | select(.name=="logical-path") | .version')"..HEAD
```

If the user gave a target version or bump type, use it. Otherwise propose a version based on the policy and the pending `## [Unreleased]` changelog notes, then confirm it with the user before continuing.

### 2. Create the release branch

```sh
git switch -c release/vX.Y.Z
```

Do not set an upstream to `main`. Create the branch from the current local `HEAD`.

### 3. Bump `Cargo.toml` and refresh `Cargo.lock`

Edit the `version` field in the `[package]` section of [`Cargo.toml`](../../../Cargo.toml), then regenerate the lockfile:

```sh
cargo build
```

### 4. Update `CHANGELOG.md`

Edit [`CHANGELOG.md`](../../../CHANGELOG.md) per [docs/releasing.md step 4](../../../docs/releasing.md#4-update-changelogmd):

- Rename the current `## [Unreleased]` section to `## [X.Y.Z] - YYYY-MM-DD` (today's date).
- Add a new empty `## [Unreleased]` section above it.
- Write user-facing notes under the Keep a Changelog categories (`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`). This text becomes the GitHub Release body.

If `## [Unreleased]` has no entries, ask the user for the release notes rather than shipping an empty section.

### 5. Verify locally

```sh
make ci
cargo publish --dry-run --locked
```

Both must pass. Fix any failures before opening the PR.

### 6. Commit and open the PR

Commit with sign-off (never stage or commit unrelated files):

```sh
git commit -s -m "Release vX.Y.Z" -- Cargo.toml Cargo.lock CHANGELOG.md
git push -u origin release/vX.Y.Z
```

Then open the PR titled `Release vX.Y.Z`, using the new `## [X.Y.Z]` changelog section as the PR body so reviewers see the user-facing notes in context. Prefer the GitHub MCP or the VS Code Pull Request tool to create the PR; fall back to the `gh` CLI (`gh pr create`) only if neither is available.

## Completion Checklist

- [ ] `Cargo.toml` version bumped and `Cargo.lock` refreshed to match.
- [ ] `CHANGELOG.md` has a dated `## [X.Y.Z]` section plus a fresh `## [Unreleased]`.
- [ ] `make ci` and `cargo publish --dry-run --locked` both pass.
- [ ] A PR titled `Release vX.Y.Z` is open with the changelog notes as its body.
- [ ] Nothing was merged, tagged, or published — that is automated post-merge.

## Guardrails

- Never merge the PR, create a git tag, or run `cargo publish` (other than `--dry-run`). Merging triggers the automated release.
- Never push to `main`.
- Always commit with `git commit -s`.
- Only stage the three release files; do not sweep in unrelated changes.
