import { assert, chooseDevice, driveWithProfile, noSidewaysScroll, open, SIM_PORT } from "./helpers.mjs";

// Watch and record switched in the point list, also by dragging over rows; the problems across
// the devices read.
export default {
  name: "switches in the point list with dragging; problems across the site",
  async setup(api) {
    const key = await driveWithProfile(api);
    const profile = (await api("site")).devices.find((device) => device.key === key).profile;
    // a unit nothing answers at: every point fails for the same reason
    await api("devices/add", "POST", { key: `tcp:127.0.0.3:${SIM_PORT}/3`, name: "Drive two" });
    await api("devices/update", "POST", { key: `tcp:127.0.0.3:${SIM_PORT}/3`, profile });
  },
  async run({ page, base, api }) {
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.setViewportSize({ width: 1280, height: 900 });
    await open(page, base);
    await chooseDevice(page, "Drive");
    await page.click("#next");
    await page.waitForSelector(".row.mpoints:has-text('Set frequency')");

    // a click on a row's switch switches that point and leaves the choice of the row alone
    const pressed = (act) => page.$$eval(`.row.mpoints button[data-act=${act}]`, (buttons) => buttons.map((button) => button.getAttribute("aria-pressed")));
    await page.click(".row.mpoints:has-text('Run') button[data-act=quick-watch]");
    await page.waitForSelector(".row.mpoints:has-text('Run') button[data-act=quick-watch][aria-pressed=true]");
    assert.equal(await page.locator(".row.mpoints.active").count(), 0);
    assert.equal((await pressed("quick-watch")).filter((on) => on === "true").length, 1);
    await page.click(".row.mpoints:has-text('Run') button[data-act=quick-watch]");
    assert.equal((await pressed("quick-watch")).filter((on) => on === "true").length, 0);

    // pressed on one and dragged over the others: all take the first one's new state
    const boxes = await page.$$eval(".row.mpoints button[data-act=quick-record]", (buttons) => buttons.map((button) => { const box = button.getBoundingClientRect(); return [box.x + box.width / 2, box.y + box.height / 2]; }));
    assert.equal(boxes.length, 3);
    await page.mouse.move(...boxes[0]);
    await page.mouse.down();
    await page.mouse.move(...boxes[1], { steps: 3 });
    await page.mouse.move(...boxes[2], { steps: 3 });
    await page.mouse.up();
    await page.waitForFunction(() => document.querySelectorAll(".row.mpoints button[data-act=quick-record][aria-pressed=true]").length === 3);
    assert.equal((await api("recording")).series.length, 3);
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/switches.png" });
    await page.mouse.move(...boxes[2]);
    await page.mouse.down();
    await page.mouse.move(...boxes[0], { steps: 5 });
    await page.mouse.up();
    await page.waitForFunction(() => document.querySelectorAll(".row.mpoints button[data-act=quick-record][aria-pressed=true]").length === 0);
    assert.equal((await api("recording")).series.length, 0);

    // the problems: one device is not read yet; reading every device finds the silent one
    await page.click(".pagehead");
    await page.keyboard.press("7");
    assert.equal(await page.textContent(".pagehead h1"), "Problems");
    assert.match(await page.textContent(".toolbar .hint"), /1 device is not read yet/);
    await page.click("[data-act=read-all]");
    await page.waitForSelector(".row.mproblems:has-text('does not answer')");
    await page.waitForSelector("[data-act=read-all]:not([disabled])");
    assert.match(await page.textContent(".toolbar .hint"), /Every device with a profile is read/);
    assert.equal(await page.locator(".row.mproblems:not(.head)").count(), 1, "a silent device is one row, not one per point");
    assert.equal(await page.textContent(".tabs a[data-area=problems] .n"), "1");
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/problems.png" });
    await page.setViewportSize({ width: 390, height: 844 });
    await noSidewaysScroll(page);
    await page.setViewportSize({ width: 1280, height: 900 });
    // a row opens its device
    await page.click(".row.mproblems:has-text('Drive two')");
    await page.waitForFunction(() => document.querySelector(".tree .node.level-3.active")?.textContent.includes("Drive two"));
    assert.deepEqual(errors, []);
  },
};
