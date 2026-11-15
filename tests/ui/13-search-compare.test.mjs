import { assert, chooseDevice, driveWithProfile, noSidewaysScroll, open, SIM_PORT } from "./helpers.mjs";

// The search over the whole site from the sidebar, and two devices of one profile side by side.
export default {
  name: "site search finds devices and read points; two devices of a profile compared",
  async setup(api) {
    const key = await driveWithProfile(api);
    const site = await api("site");
    const profile = site.devices.find((device) => device.key === key).profile;
    await api("devices/add", "POST", { key: `tcp:127.0.0.3:${SIM_PORT}/3`, name: "Drive two" });
    await api("devices/update", "POST", { key: `tcp:127.0.0.3:${SIM_PORT}/3`, profile });
    await api("devices/add", "POST", { key: `tcp:127.0.0.2:${SIM_PORT}/1`, name: "Meter" });
  },
  async run({ page, base }) {
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.setViewportSize({ width: 1280, height: 900 });
    await open(page, base);
    await chooseDevice(page, "Drive two");
    // only devices of the same profile are offered
    assert.deepEqual(await page.$$eval("#compare-with option", (options) => options.map((option) => option.textContent.split(" · ")[0])), ["Choose a device with the same profile …", "Drive"]);
    await page.selectOption("#compare-with", `tcp:127.0.0.3:${SIM_PORT}/2`);
    await page.waitForSelector(".row.mcompare:not(.head)");
    await page.waitForSelector("[data-act=compare-read]:not([disabled])");
    assert.equal(await page.textContent(".pagehead h1"), "Drive two and Drive");
    assert.equal(await page.locator(".row.mcompare:not(.head)").count(), 3);
    const different = await page.locator(".row.mcompare.differs").count();
    assert.match(await page.textContent("#compare-different"), new RegExp(`Different ${different}`));
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/compare.png" });
    await page.click("#compare-different");
    assert.equal(await page.locator(".row.mcompare:not(.head)").count(), different);
    assert.equal(await page.getAttribute("#compare-different", "aria-pressed"), "true");
    await page.click("[data-act=compare-read]");
    await page.waitForSelector("[data-act=compare-read]:not([disabled])");
    await page.setViewportSize({ width: 390, height: 844 });
    await noSidewaysScroll(page);
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.click("[data-act=compare-close]");
    await page.waitForSelector("#compare-with");

    // the search: / goes to it, devices by name, points of the devices read
    await page.click(".pagehead");
    await page.keyboard.press("/");
    assert.equal(await page.evaluate(() => document.activeElement.id), "site-search");
    await page.keyboard.type("drive");
    await page.waitForSelector(".row.found");
    assert.equal(await page.textContent(".pagehead h1"), "Found on the site");
    assert.equal(await page.locator(".row.found").count(), 2);
    assert.equal(await page.inputValue("#site-search"), "drive", "typing goes on through the redraws");
    await page.fill("#site-search", "frequency");
    await page.waitForFunction(() => document.querySelectorAll(".row.found").length === 4);
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/found.png" });
    await page.fill("#site-search", "zzz");
    await page.waitForSelector(".empty:has-text('Nothing on this site')");
    // a point found opens its device with the point chosen, and the search is over
    await page.fill("#site-search", "output drive");
    await page.fill("#site-search", "output");
    await page.click(".row.found:has-text('Drive ·') >> nth=0");
    await page.waitForSelector(".row.mpoints.active:has-text('Output frequency')");
    assert.equal(await page.inputValue("#site-search"), "");
    // emptied, the search gives the workspace back
    await page.fill("#site-search", "meter");
    await page.waitForSelector(".row.found");
    await page.fill("#site-search", "");
    await page.waitForSelector(".row.mpoints");
    assert.deepEqual(errors, []);
  },
};
