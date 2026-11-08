---
name: pull-request
description: "Finish a modbusio topic and hand it over as a pull request: own branch, commits staged by name, CHANGELOG, docs and function list in the same PR, full checks, PR text without AI attribution; then watch CI, read failing logs and merge on the maintainer's OK. Use whenever a modbusio change is done and should reach main, when asked to commit, push, open, update, describe or merge a PR, or when CI is red."
---

# From a topic to a pull request

Changes reach `main` only through a pull request, with green CI, merged on the maintainer's
explicit OK. Never commit to `main`, never merge without that OK, never tag or release (skill
release).

## Branch and commits

- One short branch per topic, named after it.
- Stage your own files by name, never `git add -A`: another session may have changes in the
  folder. Check `git status` first.
- Separate commits per concern; the subject says what changes, in English. No AI attribution: no
  `Co-Authored-By`, no "Generated with".

## In the same pull request as the code

- `CHANGELOG.md` under "Unreleased": what a user notices.
- `README.md` (Usage) for what a user sees; `docs/api.md` for routes (`tests/docs.rs` checks every
  route and simulator); the concept's status.
- `docs/concept.md` section 3.2 when a function is added or moves, with its test.
- Texts in every language (skill new-text).

## Before opening it

Run the full check set (skill browser-tests) and say in the PR which parts ran and what could not
be checked (real devices, real lines, Windows, Android).

## The PR text

In English, short: what changes for the user, why, how it was tested, what is open. Mention
behaviour changes and anything that writes to a plant. Then tell the maintainer in German that it
waits for the merge and what to look at.

## CI

- Wait for it: `gh pr checks N` until nothing is pending (a loop with `timeout 30 tail -f
  /dev/null` as the pause).
- A red job: read its log first: `gh api repos/Wobi848/modbusio/actions/jobs/<job id>/logs`. Fix
  the cause. Only a known flake of a browser itself is run again: `gh run rerun <run id> --failed`.

## Merging, on the maintainer's OK

```sh
gh pr merge N --squash --delete-branch -A <the maintainer's no-reply address> --subject "<subject> (#N)"
```

The address is in `CLAUDE.local.md`. No real site names, addresses or personal details in commits,
PRs or issues: the repo is public.
