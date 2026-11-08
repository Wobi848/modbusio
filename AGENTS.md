# Working on modbusio

Modbus browser (TCP, RTU, RTU over TCP), bacnetio's sibling. Concept: [docs/concept.md](docs/concept.md).
Everything there is a proposal and open to change.

Talk to the person you work for in their language. Everything in the repo and on GitHub is in
English: code, comments, docs, commit messages, branches, PRs, issues, CLI commands and flags. The
UI takes the system's language (English, German; French, Italian and Spanish as drafts).

Local notes that only concern one person's setup go in `CLAUDE.local.md` or `AGENTS.local.md`;
both are ignored by Git.

## Skills
Recurring tasks have project skills in `.claude/skills/`: `concept`, `browser-tests`, `review`,
`field-data`, `simulator`, `pull-request`, `api-route`, `new-text`, `try-it`, `standards`,
`register-map`, and `builds` and `release` (only on the maintainer's explicit request). Keep them
true when the commands, paths or rules they name change.

## Git and GitHub
- Commits, branches, PRs, issues, tags, release notes, file names and identifiers in English.
- No AI attribution: no `Co-Authored-By`, no "Generated with".
- One short branch per topic, named after the topic. Never commit directly to `main` once it exists
  on GitHub; changes reach it through a pull request with green CI, merged by the maintainer or on
  their explicit OK.
- Stage your own files by name, never `git add -A`. Someone else may have changes in the same
  folder.
- Separate commits per concern; the subject says what changes.

## Releases and publishing
- Tags, releases and Windows or Android builds only on the maintainer's explicit request.
- No secrets, real site addresses or personal details in the repo, its history, issues or pull
  requests. Android signing keys never enter the repo.

## Taken from bacnetio
- bacnetio (`../bacnetio`) is the model for server, interface, write journal, recording, exports,
  CI and Android. Copy and adapt; no shared crate before M2 (concept section 14, answer 6). Do not
  change anything in bacnetio from here.
- Note in the code or concept when a part is copied unchanged, so the comparison after M2 finds it.

## Before saying "done"
- Run everything CI runs, plus the browser tests: `cargo fmt --check`, `cargo test`,
  `cargo clippy --all-targets -- -D warnings`, `node --test tests/web/*.test.js` and
  `node tests/ui/run.mjs` in Chromium, Firefox and WebKit (skill browser-tests).
- Say what could not be checked (real devices, real trunks, other browsers, Windows, Android).
- Never say "done" while known points are open. Name them and work through them.
- Bigger changes: concept in `docs/` first, then code (skill concept).
- User-facing changes go in `CHANGELOG.md` under "Unreleased".
- Docs change in the same pull request as the code: `README.md` (Usage), the web API in
  `docs/api.md`, a concept's state in its `docs/concept-*.md`. `tests/docs.rs` fails when a route
  of the API or a simulator in `examples/` is not described.

## Never lose a function
- Before replacing or removing something, list every function of the old version and show each one
  in the new version, preferably with a test. Old tests still passing is not enough. The list is
  `docs/concept.md` section 3.2.

## Operation
- A work tool: self-explanatory, clean, every function reachable; bacnetio's interface principles.
- Buttons look like buttons and name their task. Never weaken a control unasked.
- Check screens with screenshots, at phone width too.

## Languages
- New texts need every existing language; a test fails otherwise.
- A new language enters the menu only after review by someone from the trade.

## Real plants
- Never write to a real Modbus device unasked, not even to test. Tests run against the simulators
  in `examples/`.
- Never search a real RTU trunk while another master is active on it.
- Every write from the UI stays behind the session unlock, with a clear confirmation, the old value
  read and logged first, and a log entry.
