---
name: verify-release
description: 'Verify that a published release of the logical-path crate landed correctly on crates.io, docs.rs, and GitHub Releases. Use when asked to "verify a release", "confirm the release shipped", "check crates.io/docs.rs", or "did version X.Y.Z publish". Runs the post-merge checks from docs/releasing.md step 9 and reports pass/fail per surface. Read-only except for one optional action: backfilling an empty GitHub Release body from CHANGELOG.md with the user''s approval.'
argument-hint: 'Optional: the version to verify (defaults to the version in Cargo.toml)'
---

# Verify a Release

## What This Confirms

After a `Release vX.Y.Z` PR merges, the automated `Release` workflow publishes to crates.io, tags `vX.Y.Z`, and creates a GitHub Release. This skill confirms all three landed:

1. **crates.io** — version `X.Y.Z` of `logical-path` is published.
2. **docs.rs** — documentation for `X.Y.Z` has built (may lag a few minutes).
3. **GitHub Releases** — a `vX.Y.Z` release exists with the changelog notes as its body.

This skill is **read-only** apart from one optional, opt-in action: if the `vX.Y.Z` release body is empty, it can backfill it from the matching `CHANGELOG.md` section (step 4a) with your explicit approval. It never publishes, tags, deletes releases, or merges.

## Authoritative Reference

The checks mirror [docs/releasing.md step 9 ("Verify the release")](../../../docs/releasing.md#9-verify-the-release). Read it if any surface is ambiguous.

## Procedure

### 1. Determine the version to verify

If the user gave a version, use it. Otherwise read the current version from [`Cargo.toml`](../../../Cargo.toml):

```sh
cargo metadata --no-deps --format-version 1 \
    | jq -r '.packages[] | select(.name=="logical-path") | .version'
```

### 2. Check crates.io

Query the crates.io API and confirm `X.Y.Z` appears in the published versions. The crates.io API **requires a `User-Agent` header** — without one it returns `403`, and a silent `curl -sf` failure would be indistinguishable from a genuinely missing version. So send a `User-Agent` and separate "request failed" from "version not found":

```sh
resp=$(curl -s -w '\n%{http_code}' \
    -H "User-Agent: logical-path-release-check (brooke-hamilton/logical-path)" \
    https://crates.io/api/v1/crates/logical-path)
status=$(printf '%s' "$resp" | tail -n1)
body=$(printf '%s' "$resp" | sed '$d')

if [ "$status" != "200" ]; then
    echo "crates.io: request failed (HTTP $status) — cannot confirm X.Y.Z"
elif printf '%s' "$body" | jq -e --arg v "X.Y.Z" \
        'any(.versions[].num; . == $v)' >/dev/null; then
    echo "crates.io: X.Y.Z published"
else
    echo "crates.io: X.Y.Z NOT found"
fi
```

Do not treat a non-`200` status as "not published" — report it as a request failure so a blocked or throttled API call is never mistaken for a missing release.

### 3. Check docs.rs

Confirm the docs build for the version resolves (a `200`/`302` means it built or is building):

```sh
curl -s -o /dev/null -w '%{http_code}\n' \
    "https://docs.rs/logical-path/X.Y.Z/logical_path/"
```

A `404` usually means the docs build has not finished yet — note that it can lag a few minutes and suggest re-running.

### 4. Check GitHub Releases

Confirm the `vX.Y.Z` release exists and has a non-empty body. Prefer the GitHub MCP or the VS Code Pull Request tooling; fall back to the `gh` CLI:

```sh
gh release view "vX.Y.Z" --repo brooke-hamilton/logical-path \
    --json tagName,name,body,url
```

An empty body means the `## [X.Y.Z]` changelog section was missing at release time — see the ["I forgot to update CHANGELOG.md"](../../../docs/releasing.md#i-forgot-to-update-changelogmd) recovery note.

### 4a. (Optional) Backfill an empty release body from `CHANGELOG.md`

This is the **only** write this skill may perform, and only with the user's explicit go-ahead. If step 4 found the `vX.Y.Z` release body empty, offer to populate it from the matching `CHANGELOG.md` section. Do not merge, tag, publish, or change anything else.

Extract the `## [X.Y.Z]` section body (everything under the heading, up to the next `## [` heading, trimmed of leading/trailing blank lines):

```sh
notes=$(awk -v ver="X.Y.Z" '
    $0 ~ "^## \\[" ver "\\]" {f=1; next}
    /^## \[/ {f=0}
    f' CHANGELOG.md \
    | sed -e '/./,$!d' | tac | sed -e '/./,$!d' | tac)
```

Show the extracted notes to the user and confirm before writing. If they approve, set the release body (never overwrite a body that is already non-empty):

```sh
printf '%s\n' "$notes" \
    | gh release edit "vX.Y.Z" --repo brooke-hamilton/logical-path --notes-file -
```

If the `## [X.Y.Z]` section is itself empty or missing, do not guess — ask the user for the notes or point them to the recovery note above.

### 5. Report

Summarize each surface as pass/fail with the relevant URL:

- crates.io: <https://crates.io/crates/logical-path>
- docs.rs: <https://docs.rs/logical-path>
- Releases: <https://github.com/brooke-hamilton/logical-path/releases>

If any check fails, point to the matching recovery section in [docs/releasing.md](../../../docs/releasing.md#edge-cases-and-recovery) rather than attempting a fix automatically.

## Completion Checklist

- [ ] Target version resolved (from argument or `Cargo.toml`).
- [ ] crates.io reports `X.Y.Z` as published.
- [ ] docs.rs returns a success status for `X.Y.Z` (or is noted as still building).
- [ ] A `vX.Y.Z` GitHub Release exists with a non-empty body (or an empty body was backfilled from `CHANGELOG.md` with the user's approval).
- [ ] A per-surface pass/fail summary with URLs was reported.

## Guardrails

- Read-only by default: never run `cargo publish`, create tags, or delete/otherwise edit releases.
- The **only** permitted write is step 4a: backfilling an *empty* release body from the matching `CHANGELOG.md` section, and only after showing the notes and getting the user's explicit approval. Never overwrite a body that already has content.
- Do not attempt any other automated recovery for a bad release — surface the relevant section of [docs/releasing.md](../../../docs/releasing.md#edge-cases-and-recovery) and let the user decide.
