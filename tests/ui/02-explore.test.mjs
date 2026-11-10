import { assert, chooseDevice, headsOverValues, open, SIM_PORT } from "./helpers.mjs";

export default {
  name: "explore: the drive's addresses, a point into a new profile, read in the workspace",
  async setup(api) {
    await api("devices/add", "POST", { key: `tcp:127.0.0.2:${SIM_PORT}/1`, name: "Meter" });
    await api("devices/add", "POST", { key: `tcp:127.0.0.3:${SIM_PORT}/2`, name: "Drive" });
  },
  async run({ page, base }) {
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await open(page, base);
    await chooseDevice(page, "Drive");
    assert.equal(await page.textContent("#next"), "Explore registers");
    // the area's key takes the device chosen in the workspace along, as the button does
    await page.keyboard.press("2");
    assert.equal(await page.inputValue("#explore-key"), `tcp:127.0.0.3:${SIM_PORT}/2`);
    await page.fill("#explore-count", "120");
    await page.click(".explore-form [data-act=explore]");
    await page.waitForSelector(".row.registers[data-address='103']");
    await headsOverValues(page, "registers:not(.missing)");
    assert.match(await page.textContent(".strips"), /8 addresses with data, 112 without/);
    assert.match(await page.textContent(".strips"), /refused larger blocks: reading at most \d+ registers per request/);
    await page.click(".row.registers[data-address='1']");
    await page.click("[data-act=add-to-profile]");
    await page.fill("#add-point-name", "Set frequency");
    await page.fill("#add-point-factor", "0,01");
    await page.fill("#add-point-unit", "Hz");
    await page.click("[data-act=save-point]");
    await page.waitForSelector(".toast:has-text('Set frequency added')");
    await page.keyboard.press("1");
    assert.match(await page.textContent(".pagehead .lead"), /profile Drive, 1 point\b/);
    // a second point into the same profile: the workspace counts it at once
    await page.keyboard.press("2");
    await page.click(".row.registers[data-address='100']");
    await page.click("[data-act=add-to-profile]");
    await page.fill("#add-point-name", "Output frequency");
    await page.click("[data-act=save-point]");
    await page.waitForSelector(".toast:has-text('Output frequency added')");
    await page.keyboard.press("1");
    assert.match(await page.textContent(".pagehead .lead"), /profile Drive, 2 points/);
    await page.click("#next");
    await page.waitForSelector(".row.mpoints:not(.head)");
    assert.equal((await page.textContent(".row.mpoints:has-text('Set frequency') .value")).trim(), "25 Hz");
    // leaving the point filter by a click redraws once, and the click still opens the point
    await page.fill("#point-filter", "freq");
    await page.click(".row.mpoints:has-text('Output frequency')");
    await page.waitForSelector(".detail .dhead:has-text('Output frequency')");
    // the profile can be saved from the first character typed, not only once the field is left
    await page.keyboard.press("3");
    await page.click(".row.mprofiles:not(.head)");
    await page.click("[data-act=edit-point]");
    await page.click("#point-description");
    await page.keyboard.type("x");
    assert.ok(await page.isEnabled("[data-act=save-profile]"), "Save profile while typing");
    await page.click("[data-act=save-profile]");
    await page.waitForSelector(".toast:has-text('saved')");
    // closing with something typed and not saved asks first; Cancel keeps the profile open
    await page.click("#point-description");
    await page.keyboard.press("End");
    await page.keyboard.type("y");
    await page.click("[data-act=close-profile]");
    await page.waitForSelector(".ask [data-act=discard-profile]");
    await page.click(".ask [data-act=cancel-ask]");
    assert.equal(await page.inputValue("#point-description"), "xy", "Cancel keeps what was typed");
    await page.click("[data-act=close-profile]");
    await page.click(".ask [data-act=discard-profile]");
    await page.waitForSelector(".row.mprofiles:not(.head)");
    await page.click(".row.mprofiles:not(.head)");
    await page.click("[data-act=close-profile]");
    await page.waitForSelector(".row.mprofiles:not(.head)");
    assert.deepEqual(errors, [], "no errors in the page");
  },
};
