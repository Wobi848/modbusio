---
name: try-it
description: "Set up a modbusio for the maintainer to try a branch in the browser on this machine (http://127.0.0.1:18082 with the simulators), keep it running while other work goes on, and switch or stop it. Use when the maintainer wants to test, try, see or click through a change themselves (\"kann ich das testen?\", \"zeig mal\"), or a running demo has to move to another branch."
---

# A modbusio for the maintainer to try

## The trap

A debug build serves `web/` from the disk. Switching the branch in the folder that serves the
maintainer's page changes that page at the next reload, half built. So the demo runs from a
worktree of its own:

```sh
git worktree add ../modbusio-demo <branch>
cd ../modbusio-demo && cargo build --all-targets
```

## Start

Outside `unshare`, so the maintainer's browser reaches it; on loopback, so nothing reaches a plant.
Ports above 1024, so no root is needed:

```sh
target/debug/examples/simulated_meter 127.0.0.2 5020 &
target/debug/examples/simulated_gateway 127.0.0.3 5020 &
target/debug/examples/simulated_gateway 127.0.0.4 5020 --rtu &
target/debug/examples/simulated_rtu &          # prints its terminal, e.g. /dev/pts/7
MODBUSIO_TEST_SERIAL=/dev/pts/7 target/debug/modbusio --no-browser --test-loopback --web-port 18082 --data-dir <scratchpad>/live/data &
```

Run each as a background task and keep its id, to stop exactly that one (never `pkill -f`).

## Tell the maintainer (German, short)

- the address and to reload the page
- the search: *Sucheinstellungen*, *Netz (TCP)*, subnet `127.0.0.0/29`, port `5020`; the gateway
  `127.0.0.3:5020`; the RTU gateway `127.0.0.4:5020` with *RTU über TCP*; the serial line as the
  port *simulated line*
- what to look at, in a few points

## Keep in mind

- The data folder keeps the write log, profiles and sites over restarts: a value written days ago
  comes back as standing. Say where it comes from (`writes.jsonl`).
- After a rebuild, stop and start modbusio again, **and the simulators on pseudo-terminals with
  it** (`simulated_rtu`, `simulated_rtu --settings`, `simulated_trunk`): modbusio opens a serial
  port for itself alone, and a pseudo-terminal the simulator keeps open stays locked after
  modbusio ends, so the new modbusio gets "used by another program". They print new terminals;
  pass them all in `MODBUSIO_TEST_SERIAL`, comma-separated.
- The simulators for the bus analyser and for devices at other settings: `simulated_trunk
  --cycle-ms 1000` (a trunk with its own master) and `simulated_rtu --settings` (devices at
  settings of their own; the trunk runs 19200 8E1).
