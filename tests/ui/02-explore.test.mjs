import { assert, chooseDevice, open, SIM_PORT } from "./helpers.mjs";

export default {
  name: "explore: the drive's addresses, a point into a new profile, read in the workspace",
  async setup(api) {
    await api("devices/add", "POST", { key: `tcp:127.0.0.3:${SIM_PORT}/2`, name: "Drive" });
  },
  async run({ page, base }) {
    await open(page, base);
    await chooseDevice(page, "Drive");
    assert.equal(await page.textContent("#next"), "Explore registers");
    await page.click("#next");
    await page.fill("#explore-count", "120");
    await page.click(".explore-form [data-act=explore]");
    await page.waitForSelector(".row.registers[data-address='103']");
    assert.match(await page.textContent(".strips"), /8 addresses with data, 112 without/);
    assert.match(await page.textContent(".strips"), /at most \d+ registers per request/);
    await page.click(".row.registers[data-address='1']");
    await page.click("[data-act=add-to-profile]");
    await page.fill("#add-point-name", "Set frequency");
    await page.fill("#add-point-factor", "0,01");
    await page.fill("#add-point-unit", "Hz");
    await page.click("[data-act=save-point]");
    await page.waitForSelector(".toast:has-text('Set frequency added')");
    await page.keyboard.press("1");
    await page.click("#next");
    await page.waitForSelector(".row.mpoints:not(.head)");
    assert.equal((await page.textContent(".row.mpoints:not(.head) .value")).trim(), "25 Hz");
  },
};
