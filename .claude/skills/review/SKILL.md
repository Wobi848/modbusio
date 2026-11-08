---
name: review
description: "Review a modbusio branch, commit or pull request read-only: real bugs, the safety of everything that writes to a plant (unlock checked by the server, confirmation, old value read and logged, read back, restore, leaving check), lost functions, and what a good Modbus tool offers. Use when the maintainer asks to review, audit or \"gegenlesen\" modbusio code or a branch, to \"Code prüfen\", or to look over another session's work; it changes nothing in the repo."
---

# Reviewing modbusio

Read only. No file, commit, branch or worktree in the repo; read files of a commit with
`git show <commit>:<path>` and changes with `git diff main...<branch>`. Start nothing on a real
network or line and write to no device. Another session may be working in the folder.

## Read first

- `AGENTS.md`, `docs/concept.md` (section 3.2 lists every function, its place and its test)
- the concepts the change touches, `docs/api.md` for routes

## Check in this order

1. **Writing to a plant.** Every route that changes a device (`/api/write`, `/api/restore`):
   - refused by the server while the session is locked (`require_unlocked`), not only in the page
   - a confirmation in the page that names value and place, never a browser dialog
   - the old value read first; without it nothing is written (`write::write`)
   - read back afterwards; a device keeping another value is shown
   - logged in `writes.jsonl`, refused or not; *Changed by you* and the leaving check derived from
     the log, not kept apart
   - input checked at the server: finite numbers, the point's min and max, the type's range, a
     raw write only with `confirm_raw`; broadcast (unit 0) never written (`write::BROADCAST`)
   - the `Host`/`Origin` guard covers the route
2. **Protocol.** Requests within the spec's limits (`Request::check`), answers matched to their
   request (transaction ID on TCP, unit and CRC on RTU), the 3.5-character gap before a request on
   a line, no late answer taken for a new request, exceptions named.
3. **Real bugs.** Wrong results (word order, scaling, signed values, the address convention),
   crashes (`unwrap`, `expect` on input from outside), lost state after a reload or restart,
   texts without a translation.
4. **Never lose a function.** A removed or moved control has a row in concept section 3.2 and a
   test.
5. **Gaps against a good tool.** Compare with Modbus Poll, QModMaster or a vendor's tool. Look it up
   in the concept, `docs/api.md` and `src/` before calling something missing. Name it, do not
   build it.

## Write the result

German, short, the most important first, each point with file and line and the code that proves
it. Say plainly what was not checked.
