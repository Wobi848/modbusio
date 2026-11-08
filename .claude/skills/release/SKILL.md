---
name: release
description: "Publish a modbusio release (version, changelog date, tag, release notes, checking the files) only when the maintainer explicitly asks for that release in the current conversation (\"mach 0.1.0\", \"neues Release\"). A release is public and cannot be taken back."
disable-model-invocation: true
---

# A modbusio release

Only on the maintainer's explicit word in this conversation. A `v*` tag builds and publishes for
everyone; ask first when anything is unclear.

## Before

- Every pull request meant for it is merged (skill pull-request), `main` is green.
- Read `CHANGELOG.md` "Unreleased" against the merged pull requests.
- Bump by semver: patch for fixes only, minor for new functions, major when something breaks.

## Steps

1. Branch `release-vX.Y.Z` from `origin/main`: `Cargo.toml` `version = "X.Y.Z"` (`cargo build`
   updates `Cargo.lock`); `CHANGELOG.md` `## Unreleased` becomes `## vX.Y.Z (DD.MM.YYYY)`. Commit
   "Prepare release vX.Y.Z", pull request, green CI, merge on the maintainer's OK.
2. Tag on the merged `main`:
   ```sh
   git fetch && git switch main && git merge --ff-only origin/main
   grep -m1 '^version' Cargo.toml          # must say X.Y.Z
   git tag vX.Y.Z && git push origin vX.Y.Z
   ```
3. Wait for the run of the tag (`gh run list --branch vX.Y.Z`).
4. Release notes from the changelog section:
   ```sh
   awk '/^## vX.Y.Z/{on=1;next} /^## v/{on=0} on' CHANGELOG.md > <scratchpad>/notes.md
   gh release edit vX.Y.Z --notes-file <scratchpad>/notes.md
   ```
5. Check the files: `gh release view vX.Y.Z --json assets -q '.assets[].name'`: the Windows exe and
   the Linux program.

Then tell the maintainer in German: the link, what is new in three lines, and what was not tested
on real devices.
