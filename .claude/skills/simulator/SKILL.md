---
name: simulator
description: "Add or extend a simulated Modbus device in modbusio's examples/sim or in a Rust test so that a behaviour seen on a real plant (gaps in the map, a small block limit, swapped floats, scale registers, a gateway, one connection only, slow or silent units, refused functions) has a test without a real device. Use when fixing a field problem, when a test needs data no simulator has, or when a browser test needs another device."
---

# Simulators in examples/

Tests never touch real devices (AGENTS.md). Every behaviour from the field gets a simulator that
shows it, then a test against it. The simulators use tokio-modbus' servers: an implementation
independent of modbusio's, so a test does not pass because both sides share a bug.

## The parts

- `examples/sim/mod.rs`: the device model (`Device`: four areas, which addresses exist, read-only
  registers, most registers per request, identification, refusing 16 for one register, a delay),
  the devices (`energy_meter`, `heat_meter`, `drive`, `room_controller`), `Units` (devices by unit;
  answering every unit, or as a gateway with exception 0B), and serving them over TCP, RTU over TCP
  or a serial line.
- `simulated_meter`, `simulated_gateway` (`--rtu`), `simulated_rtu` (a pseudo-terminal): the
  examples, described in README.md (`tests/docs.rs` checks it).
- `tests/common/mod.rs` includes `examples/sim` and starts devices on free loopback ports.

## Steps

1. Prefer a device built inside the Rust test (`sim::Device { .. }` with its maps) when only that
   test needs it, as `tests/read.rs` does.
2. Extend `examples/sim` when the browser tests or the maintainer's demos need it; a new example
   only for a device that differs in kind.
3. Make the behaviour explicit and commented: a gap, `most_registers`, `single_writes_only`, a
   delay, a unit that stays silent.
4. Names and numbers are made up; never copy a real site's names or addresses.
5. Update tests that count devices or addresses of an example (`tests/ui/*.test.mjs`).
6. Run the full check set (skill browser-tests).
