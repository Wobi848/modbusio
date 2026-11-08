---
name: browser-tests
description: "Run modbusio's full check set (cargo fmt/test/clippy, the UI logic tests, the browser tests in Chromium, Firefox and WebKit) against the simulators, and take screenshots of the interface at phone and desktop width, light and dark. Use this before saying a modbusio change is done, after any change in web/, when a browser test fails, or when the maintainer asks to test, check a screen, take a screenshot or \"schau dir das an\" — even if they only say \"läuft alles?\"."
---

# Browser tests and screenshots for modbusio

Everything runs against the simulators in `examples/` on loopback addresses and a
pseudo-terminal. Never against a real network, line or device.

## Full check set (what CI runs, plus the browsers)

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
node --test tests/web/*.test.js
cargo build --all-targets
export PLAYWRIGHT=$(ls -d ~/.npm/_npx/*/node_modules/playwright 2>/dev/null | head -1)
for browser in chromium firefox webkit; do
  MODBUSIO_TEST_BROWSER=$browser unshare -Urn bash -c 'ip link set lo up; node tests/ui/run.mjs'
done
```

- **Run them in a network of their own** (`unshare -Urn`, Linux): the tests start the simulators on
  `127.0.0.2`, `127.0.0.3` and `127.0.0.4`, port 5020. Simulators already running there for the
  maintainer (skill try-it) would answer too. `cargo test` the same way when simulators run.
- Without a Playwright in the npm cache: `npm i -D playwright && npx playwright install chromium firefox webkit`.
- One test only: `node tests/ui/run.mjs <part of its name or file>`.
- Two runs at once (another session in the folder): `MODBUSIO_TEST_PORT=18600` for the second.
- A failure leaves a screenshot in the temp folder: `modbusio-ui-<file>.png`. Look at it with the
  Read tool before guessing.
- Report each part as passed or failed with the failing names; never "done" with a red part.

## Screenshots of the interface

1. Start the simulators and a modbusio with its own data folder and port in a network of their own:
   ```sh
   S=<scratchpad>; cargo build --all-targets
   unshare -Urn bash -c "ip link set lo up
     target/debug/examples/simulated_meter 127.0.0.2 502 &
     target/debug/examples/simulated_gateway 127.0.0.3 502 &
     target/debug/modbusio --no-browser --web-port 18700 --data-dir $S/shots-data --test-loopback &
     sleep 1; node $S/shots.mjs; kill %1 %2 %3"
   ```
   The Playwright script (`shots.mjs`, in the scratchpad) runs inside, as the browser must reach
   the server in that network.
2. Search the subnet `127.0.0.0/29` from *Search settings*, then the gateway's units.
3. Take each screen at 390 × 844 (phone), 1000 and 1440 wide, in light and dark
   (`localStorage modbusio.theme`), and in German (`modbusio.language`).
4. Save the pictures in the scratchpad, never in the repo, and look at each with the Read tool:
   nothing cut off, no sideways scrolling, the next step visible, colours from `web/css/tokens.css`.
5. Stop the processes you started (by PID, not `pkill -f`, which can hit other sessions).

## Rules

- Do not change the tests to make them pass; fix the cause, or say why the test is wrong.
- Say what could not be checked: real devices, real lines, Windows, Android.
