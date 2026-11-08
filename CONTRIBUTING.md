# Contributing to modbusio

Thank you for helping. modbusio is a work tool for people on building automation sites, so two
things come before everything else: it must never do anything to a plant the user did not ask
for, and it must say in plain words what is going on.

## Before you start

- For a bug, open an issue; for anything bigger than a fix, open an issue first and describe what
  you want to change. Larger changes start as a concept in `docs/`, agreed before the code.
- Everything in the repository is in English: code, comments, docs, commits, branches, issues and
  pull requests.

## Set up

```sh
cargo build --all-targets
cargo run                     # web server on http://127.0.0.1:8082/, opens the browser
```

The simulated devices in `examples/` stand in for a plant (README, *Usage*). **Never test against a
real Modbus device you are not responsible for, and never write to one to try something out.**

## Checks

Run what CI runs, and the browser tests, before you open a pull request:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
node --test tests/web/*.test.js
node tests/ui/run.mjs         # see tests/ui/README.md; Chromium, Firefox and WebKit
```

The integration and browser tests talk real TCP on loopback addresses and open pseudo-terminals.
On Linux they can run in a network namespace of their own:
`unshare -Urn bash -c 'ip link set lo up; cargo test'`.

## Pull requests

One topic per pull request, on its own branch. Say what changes for the user and how it was
tested. User-facing changes go in `CHANGELOG.md` under *Unreleased*; a new or changed route of the
web API in `docs/api.md` (a test checks it), a new simulator in `README.md`.
