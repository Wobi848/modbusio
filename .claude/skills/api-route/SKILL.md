---
name: api-route
description: "Add or change a route of modbusio's local web API in src/server/, with the Host/Origin guard, input checks at the server, the write log and the session unlock when it changes a device, docs in docs/api.md and tests. Use whenever the interface needs new data or a new action from the server, or a handler in src/server/ changes."
---

# A route of the web API

## Where

- Routes in `src/server/mod.rs` (`router`); handlers in the module of their area (`scanning.rs`,
  `sites.rs`, `devices.rs`, `profiles.rs`, `writing.rs`, `recording.rs`, `capturing.rs`,
  `exports.rs`). The logic lives outside `server/`: the server knows HTTP, the logic knows Modbus.
  Split a file over ~400 lines.
- Every route passes the `Host`/`Origin` guard (`src/server/guard.rs`) through the router layer.

## Rules

- **Check input at the server**, not only in the page: ranges, finite numbers, empty fields, unit
  IDs 1–247; answer `400` in words.
- **Errors** as `ApiError` with the status from `docs/api.md`; English text, and a pattern in
  `web/i18n.js` plus texts in every language when the interface shows it (skill new-text).
- **Anything that changes a device** goes through `write::journaled`, behind `require_unlocked`,
  with a confirmation in the page. The old value is read first.
- **Talking to a device** goes through `state.links` (one connection per line); what was learnt
  about the device is kept with `state.update_device`.
- **Stored data and API fields** stay compatible: add fields, do not rename or remove them.

## Docs and tests

- Describe the route in `docs/api.md`: method, path, request, answer, errors. `tests/docs.rs`
  fails otherwise.
- A test for the behaviour: `tests/api.rs` against the simulators, a browser test for what the
  user does.
- `CHANGELOG.md` when a user notices it.
