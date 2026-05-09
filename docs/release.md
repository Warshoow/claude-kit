# Releasing claude-kit

Step-by-step guide for cutting a new release. Follow this every time
you tag a new version — it's short on purpose.

## TL;DR — the happy path

```bash
# 1. Bump versions in the three manifests (see § "Bump the version")
#    → package.json, src-tauri/Cargo.toml, src-tauri/tauri.conf.json
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json

# 2. Commit
git commit -m "chore: bump version to X.Y.Z"

# 3. Tag (note the leading `v`, required by the workflow filter)
git tag vX.Y.Z

# 4. Push commit + tag separately — pushing the tag is what triggers
#    the Release workflow on GitHub.
git push origin master
git push origin vX.Y.Z

# 5. Watch the build → review the draft release on GitHub → Publish.
```

That's it. Total active time on your side: ~5 min. Wall-clock for the
build matrix: ~15 min.

## Pick a version

Semver applied loosely to a pre-1.0 app:

- **Patch (0.x.Y)** — bug fixes only, no new feature
- **Minor (0.X.0)** — new features, backward-compat
- **Major (X.0.0)** — only when you reach 1.0

For practical purposes, every "milestone batch of features" gets a
minor bump. We're going from 0.1 → 0.3 because we shipped: AI flows
(generator, harmonizer, recommender), force-overwrite plugin updates,
multiple marketplaces, and a landing site — that's at least two minor
bumps' worth of work.

## Bump the version

The version lives in **three places**. They must all match — the one
in `tauri.conf.json` is the source of truth for installer file names,
the others are for tooling consistency.

| File | Field | Format |
|---|---|---|
| `package.json` | `"version"` | `"0.3.0"` |
| `src-tauri/Cargo.toml` | `version` | `"0.3.0"` |
| `src-tauri/tauri.conf.json` | `"version"` | `"0.3.0"` |

Edit them by hand or use `npm version --no-git-tag-version 0.3.0`
(handles `package.json` only — you still touch the two Tauri files
manually).

Don't pre-tag anything yet. Commit the bump first, *then* tag.

## Tag

```bash
git commit -m "chore: bump version to 0.3.0"
git tag v0.3.0
```

The leading `v` is **required** — the workflow trigger is
`on: push: tags: ['v*']`. A tag like `0.3.0` (without the v) would
be ignored.

## Push

```bash
git push origin master       # the bump commit
git push origin v0.3.0       # the tag — THIS triggers the workflow
```

Pushing the branch and the tag separately is intentional: it lets
you double-check before the workflow actually fires.

## What the workflow does

Trigger: any push to a tag matching `v*` (see
[`.github/workflows/release.yml`](../.github/workflows/release.yml)).

It builds in parallel across 4 runners:

| Runner | Output bundles |
|---|---|
| `ubuntu-22.04` | `.deb`, `.rpm`, `.AppImage` |
| `windows-latest` | `.msi`, `.exe` (NSIS) |
| `macos-latest` (aarch64) | `.dmg`, `.app` (Apple Silicon) |
| `macos-latest` (x86_64) | `.dmg`, `.app` (Intel) |

Each build runs `npm ci` → `npm run tauri build` → uploads the bundles
as assets on a **draft GitHub Release** named `claude-kit vX.Y.Z`.

First-time builds take 5–15 min per runner (Rust release-mode LTO
takes a while). Subsequent builds reuse caches and are faster.

Watch progress in the **Actions** tab on GitHub. Each row of the matrix
is a separate job — partial failures don't abort the whole release
(matrix has `fail-fast: false`).

## Review and publish

When all four builds finish:

1. Open the **Releases** tab on GitHub. There's a draft `claude-kit
   vX.Y.Z` with all bundles attached as assets.
2. Click **Edit draft** and write a changelog. There's no auto-changelog
   today; use `git log vPREV..vTHIS --oneline` to gather material.
   Suggested structure:

   ```markdown
   ## What's new

   - **Feature 1** — one-sentence summary
   - **Feature 2** — one-sentence summary

   ## Bug fixes

   - …

   ## Notes

   - Windows/macOS binaries are unsigned (see § "Unsigned binaries"
     below)
   ```

3. Optional: pick a "Latest" asset (usually the AppImage or .msi —
   not strictly required, GitHub picks one by default).

4. Click **Publish release**. The draft becomes public. The
   `https://github.com/Warshoow/claude-kit/releases/latest` URL
   updates and the website's "Download" button starts pointing here.

## Test before going public — RC builds

Worth doing for the very first release, optional later. The exact
same workflow runs on `vX.Y.Z-rc.N` tags (the `v*` filter matches),
but GitHub marks the resulting release as "Pre-release" automatically.

```bash
git tag v0.3.0-rc.1
git push origin v0.3.0-rc.1
# Wait for builds. Download bundles. Smoke-test on each OS.
# When happy:
git tag v0.3.0
git push origin v0.3.0
```

If you do this, you can either keep the RC drafts around as evidence
of testing, or delete them once the real release is out (no consumer
has linked to them).

## Unsigned binaries — what users see

We don't sign yet. So:

- **Windows**: SmartScreen says *"Windows protected your PC"* on first
  launch. The user clicks *More info > Run anyway*. After the first
  successful run, SmartScreen remembers the binary and stops nagging.
- **macOS**: Gatekeeper refuses a double-click of the `.dmg`. The user
  has to right-click the `.app` inside the disk image and choose
  *Open* — that bypasses the warning. Or: System Settings > Privacy &
  Security > "claude-kit was blocked, allow anyway".
- **Linux**: nothing special. `.AppImage` needs `chmod +x` then double-click;
  `.deb`/`.rpm` install through the regular package managers.

To remove these warnings entirely you'd need:

- **Windows**: a code-signing certificate ($200/year-ish from Sectigo,
  DigiCert…)
- **macOS**: an Apple Developer ID ($99/year) plus notarization
  workflow (Tauri docs cover this when you're ready)

Worth doing once you have actual users complaining. Not before.

## If something goes wrong

### A build failed on one OS

The other three keep running. You can:

- Re-run only the failed job from the Actions UI ("Re-run failed jobs")
- Or publish the partial release and skip that target for now

### You realise the tag was wrong

If the draft hasn't been published:

```bash
# Local
git tag -d v0.3.0
# Remote
git push origin :refs/tags/v0.3.0
# Delete the draft release on GitHub via the UI

# Fix what you need to fix, then re-tag
git tag v0.3.0
git push origin v0.3.0
```

If the release is **already published**, never delete it. Cut a new
patch (`v0.3.1`) and move on.

### Local cargo check works but CI fails

Your CI uses `--no-default-features` (see
[CLAUDE.md](../CLAUDE.md)), but the Release workflow does a full
`tauri build`. If the release fails with `frontendDist` not found, it
means `npm run build` failed silently — check the workflow logs for
the npm step.

### Release is live, version mismatches

Don't try to "fix in place". Cut a `v0.3.1` with the corrected version
in all three manifests. Releases are immutable in users' minds.

## After publishing

- The website's *Download* button (in the hero section) links to
  `/releases` — that auto-points at "latest", so it picks up the new
  version with no site change required.
- Bump the working version on `master` *immediately* to a `+dev` or
  patch increment if you want to mark in-progress work, e.g.:

  ```
  chore: start 0.4.0 development cycle
  ```

  Optional, but it makes commits between releases easier to read in
  retrospect. We can also skip it and let the next release commit
  do the bump.

## Cheat sheet

```bash
# Bump → commit → tag → push → wait → review → publish
vim package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git add -p && git commit -m "chore: bump version to X.Y.Z"
git tag vX.Y.Z
git push origin master vX.Y.Z
# → check Actions tab on GitHub
# → wait 10-15 min
# → open Releases tab, edit draft, write changelog, click Publish
```
