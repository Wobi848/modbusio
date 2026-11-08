# Browser tests

The interface in a real browser against the simulators in `examples/`: the meter on 127.0.0.2, the
gateway on 127.0.0.3, the RTU gateway on 127.0.0.4, all on port 5020, and on Linux the RS-485 line
on a pseudo-terminal. Each test gets a fresh modbusio with an empty data folder.

```sh
cargo build --all-targets
npm i -D playwright && npx playwright install chromium firefox webkit   # once
node tests/ui/run.mjs                      # all tests in Chromium
node tests/ui/run.mjs write                # only tests with "write" in their name or file
MODBUSIO_TEST_BROWSER=firefox node tests/ui/run.mjs
```

`PLAYWRIGHT=<path>/node_modules/playwright` uses a Playwright installed elsewhere. On Linux, run
them in a network of their own, so simulators started for a demo do not answer too:
`unshare -Urn bash -c 'ip link set lo up; node tests/ui/run.mjs'`. A failing test leaves a
screenshot in the temp folder (`modbusio-ui-<file>.png`).

A test is a module with `name`, an optional `setup(api)` and `run({ page, api, base, context,
restart })`. Browser dialogs (`alert`, `confirm`) and errors in the page fail the test.
