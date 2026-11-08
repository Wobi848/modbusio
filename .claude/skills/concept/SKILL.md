---
name: concept
description: "Write or update a concept in modbusio's docs/ before a bigger change, with the function list and open questions, then stop for the maintainer's approval. Use whenever a change is more than a fix: a new page, area or panel, moved or removed controls, anything new that writes to devices, a new transport, profile format, language or file format, or when the maintainer asks to plan, propose, redesign or \"überleg dir was\"."
---

# Concepts in modbusio

AGENTS.md: bigger changes get a concept in `docs/` first, then code. The main concept is
`docs/concept.md`; models for topic concepts are bacnetio's `docs/concept-*.md` (`../bacnetio`),
for example `concept-mstp.md` or `concept-redesign.md` (with a clickable mockup).

## When

- A new page, area or panel, or a control moves or goes away
- Anything new that writes to a device, or a new kind of write
- A new transport (ASCII, another gateway mode), a change to the profile format, a new export
  format or file the user keeps, a new language
- Something taken from bacnetio that has to differ for Modbus
- The maintainer asks for a plan, a redesign or "überleg dir was"

A bug fix or a small addition inside an existing concept needs none; update that concept's state.

## File and structure

`docs/concept-<topic>.md`, in English, short sentences:

1. **Title and status line**: proposal / approved on <date> / built on <date>, with the branch.
2. **Why**: what the maintainer said or saw (quote briefly in German if it helps), what goes wrong
   today, measured or seen, not assumed.
3. **Proposal**: what the user sees and does, step by step. An ASCII sketch for layouts; a
   clickable mockup in `docs/mockup/<topic>.html` (static data, nothing talks to a device) when
   operation changes.
4. **Writing to plants** (if any): unlock, confirmation in the page, old value read and logged,
   read back, restore, what the leaving check shows, what is refused at the server.
5. **Functions and where they live**: every function the change touches, its old place, its new
   place, its test. Moves update the list in `docs/concept.md` section 3.2; never a second list.
6. **Compatibility**: stored data, profiles, files and API fields keep loading; what goes into
   `CHANGELOG.md`.
7. **Order of building**: small steps, each leaving everything working.
8. **Open for the decision**: the questions only the maintainer can answer, each with a
   recommendation.

## Then

- Stop and ask the maintainer (in German) to approve, naming the open questions. Build only what
  does not depend on the answer.
- After building: set the status line, tick what is built, keep the function list true.
