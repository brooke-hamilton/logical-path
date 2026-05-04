# Releasing `logical-path`

This document describes how to publish a new version of the `logical-path` crate to [crates.io](https://crates.io/crates/logical-path) and produce a corresponding GitHub Release.

Releases are automated by [release-plz](https://release-plz.dev/) and run from the [`Release`](../.github/workflows/release.yml) workflow. You never run `cargo publish` or `git tag` by hand.

## TL;DR

1. Open a PR that bumps `version` in [`Cargo.toml`](../Cargo.toml).
2. In the same PR, add a new section to [`CHANGELOG.md`](../CHANGELOG.md) describing the changes.
3. Get the PR reviewed and merge it to `main`.
4. Wait for `CI` to pass. The `Release` workflow then publishes to crates.io, tags `vX.Y.Z`, and creates the GitHub Release.

That's the entire flow. The rest of this document explains each step in detail and covers edge cases.

## Versioning policy

`logical-path` follows [Semantic Versioning](https://semver.org/):

| Change | Bump |
| ------ | ---- |
| Bug fixes, internal refactors, doc changes | **patch** (`0.1.0` → `0.1.1`) |
| Behavior change that callers might notice but compiles unchanged | **patch** while pre-1.0; **minor** after 1.0 |
| Public API addition (new method, new type) | **minor** (`0.1.x` → `0.2.0`) while pre-1.0; **minor** after 1.0 |
| Public API removal, signature change, or behavior change that callers must adapt to | **minor** while pre-1.0 (because `0.x` minor bumps are breaking by Cargo convention); **major** after 1.0 |

While the crate is `0.x.y`, **any minor bump is treated as breaking** by Cargo's resolver. Plan releases accordingly.

## The release procedure, step by step

### 1. Decide the new version

Inspect changes since the last release:

```sh
git log --oneline "v$(cargo metadata --no-deps --format-version 1 \
    | jq -r '.packages[] | select(.name=="logical-path") | .version')"..HEAD
```

Pick the next version per the policy above.

### 2. Create a release-prep branch

```sh
git switch -c release/vX.Y.Z
```

### 3. Bump the version in `Cargo.toml`

Edit the `[package]` section of [`Cargo.toml`](../Cargo.toml):

```toml
[package]
name = "logical-path"
version = "X.Y.Z"   # ← update this line
```

Then refresh `Cargo.lock` so it records the new version. Any of these works:

```sh
cargo update --workspace
# or simply
cargo build
```

Commit both `Cargo.toml` and `Cargo.lock`. The `--locked` checks in CI and in `cargo publish` require the two files to agree.

### 4. Update `CHANGELOG.md`

Open [`CHANGELOG.md`](../CHANGELOG.md). It uses the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

Convert the existing `## [Unreleased]` placeholder into the new release section and add a fresh `## [Unreleased]` above it. The result should look like this:

```markdown
## [Unreleased]

## [X.Y.Z] - YYYY-MM-DD

### Added
- New `LogicalPathContext::with_prefix()` constructor.

### Fixed
- Windows subst-drive paths now round-trip correctly.

## [previous version] - older-date
...
```

The categories from Keep a Changelog are: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`. Use whichever apply; omit the rest.

The contents of the `[X.Y.Z]` section will become the body of the GitHub Release, so write it for users of the crate, not for yourself.

### 5. Verify locally

Run the same checks that CI runs:

```sh
make ci
```

Then verify that the package is publishable:

```sh
cargo publish --dry-run --locked
```

This validates the manifest, license, included files, and that the crate builds in isolation.

### 6. Open a pull request

```sh
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -s -m "Release vX.Y.Z"
git push -u origin release/vX.Y.Z
```

Then open a PR titled `Release vX.Y.Z` (or similar). Include the new changelog section in the PR body so reviewers can read the user-facing notes in context.

### 7. Merge to `main`

After review, merge the PR using your normal merge strategy (squash is fine).

### 8. Watch the workflows

Go to the **Actions** tab.

1. The [`CI`](../.github/workflows/ci.yml) workflow runs on the merge commit. It must pass before release-plz will run.
2. When `CI` finishes successfully, the [`Release`](../.github/workflows/release.yml) workflow triggers via `workflow_run`. It will:
   - Run `cargo publish` to upload `vX.Y.Z` to crates.io.
   - Create the annotated git tag `vX.Y.Z`.
   - Create a GitHub Release titled `vX.Y.Z` whose body is the matching `CHANGELOG.md` section.

If `CI` fails, the release does not happen — fix forward with another PR.

### 9. Verify the release

After the `Release` workflow completes:

- Check <https://crates.io/crates/logical-path> for the new version.
- Check <https://docs.rs/logical-path> — docs build is automatic but can lag by a few minutes.
- Check the [Releases page](https://github.com/brooke-hamilton/logical-path/releases) for the `vX.Y.Z` entry with your changelog notes.

## How it works

The release pipeline is intentionally minimal:

```text
PR merged to main
        │
        ▼
   CI workflow ───── fails ────▶ no release; fix forward
        │
   succeeds
        │
        ▼
 Release workflow (workflow_run gate)
        │
        ▼
 release-plz release
   • reads Cargo.toml version
   • compares to crates.io latest
   • if new:
       - cargo publish
       - git tag vX.Y.Z
       - create GitHub Release using
         the [X.Y.Z] section of CHANGELOG.md
   • if same: no-op
```

Key configuration files:

- [`.github/workflows/release.yml`](../.github/workflows/release.yml) — the workflow definition. It is gated by `workflow_run` on `CI` so a release cannot ship a commit that failed tests.
- [`release-plz.toml`](../release-plz.toml) — release-plz configuration. Notable settings:
  - `changelog_update = false` — release-plz reads `CHANGELOG.md` to populate the GitHub Release body but never modifies it. We hand-write the file in the release PR.
  - `semver_check = true` — runs [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks) before publishing to catch accidental API breakage.
  - `publish_timeout = "10m"` — fails fast if `cargo publish` hangs.

## Edge cases and recovery

### I forgot to update `CHANGELOG.md`

The release will succeed, but the GitHub Release body will be empty. To fix: edit the GitHub Release on the web UI and paste the notes manually. Then add the section to `CHANGELOG.md` in your next PR (under the version that was already released — purely for historical accuracy).

### I bumped the version but `CI` failed

No release happened. Fix the failure on a follow-up PR. When that PR merges and `CI` passes, release-plz will publish the version that was originally bumped (assuming `Cargo.toml` still has the new version).

### I want to skip a release after merging a version bump

Don't merge the version bump until you're ready to release. If you've already merged it, the only "skip" option is to immediately bump again to a patch above what's about to publish. There is no way to cancel a release once `CI` passes other than failing the `Release` workflow's job manually in time.

### Re-running the release workflow

Re-running the `Release` workflow on a green `CI` is safe. release-plz checks crates.io and does nothing if the version is already published.

### A bad release shipped to crates.io

You **cannot** delete a published version from crates.io. The standard recovery is:

1. **Yank** the bad version: `cargo yank --version X.Y.Z`. This prevents new projects from depending on it but keeps existing `Cargo.lock`s working.
2. Publish a fixed version (`X.Y.(Z+1)`) using the normal procedure.
3. Note the yank and the reason in `CHANGELOG.md` under the new version.

### Updating the pinned action SHAs

The actions in `release.yml` are pinned to specific commit SHAs for supply chain safety. To update:

1. Find the latest tag at the action's GitHub repository (e.g. <https://github.com/release-plz/action/tags>).
2. Look up that tag's commit SHA.
3. Update both the SHA and the trailing `# version` comment in the workflow.

## See also

- [Architecture](architecture.md)
- [Platform behavior](platform-behavior.md)
- [release-plz documentation](https://release-plz.dev/docs)
- [Cargo Reference: Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html)
