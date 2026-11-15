import { assert, driveWithProfile, open, SIM_PORT } from "./helpers.mjs";

export default {
  name: "keyboard: arrows through lists and menus, Enter opens, menus inside the window, adapters read again",
  async setup(api) {
    await api("devices/add", "POST", { key: `tcp:127.0.0.2:${SIM_PORT}/1`, name: "Meter" });
    await driveWithProfile(api);
  },
  async run({ page, base }) {
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.setViewportSize({ width: 1280, height: 800 });
    await open(page, base);
    await page.waitForSelector(".row.mdevices:has-text('Drive')");
    // the device list leads away when a row is chosen: the arrows mark, Enter opens
    const marked = () => page.evaluate(() => document.querySelector(".row.cursor .name")?.textContent.trim() ?? null);
    await page.keyboard.press("ArrowDown");
    const first = await marked();
    assert.ok(first, "the first arrow marks a row");
    await page.keyboard.press("ArrowDown");
    const second = await marked();
    assert.notEqual(second, first);
    // the mark survives the page being drawn anew
    await page.evaluate(async () => (await import("/store.js")).render());
    assert.equal(await marked(), second);
    await page.keyboard.press("Home");
    assert.equal(await marked(), first);
    await page.keyboard.press("End");
    assert.equal(await marked(), second);
    const opened = (await page.getAttribute(".row.cursor", "data-key"));
    await page.keyboard.press("Enter");
    await page.waitForFunction((key) => document.querySelector(".tree .node.level-3.active")?.dataset.key === key, opened);
    // the points stay on screen when chosen: the arrows choose, the details follow
    await page.evaluate(async (key) => {
      const { actions } = await import("/actions.js");
      actions["select-device"]({ dataset: { key } });
    }, `tcp:127.0.0.3:${SIM_PORT}/2`);
    await page.click("#next");
    await page.waitForSelector(".row.mpoints:has-text('Set frequency')");
    const chosen = () => page.evaluate(() => document.querySelector(".row.mpoints.active .name")?.textContent.trim() ?? null);
    await page.keyboard.press("ArrowDown");
    const one = await chosen();
    assert.ok(one, "the first arrow chooses a point");
    await page.waitForSelector(".detail .dhead");
    await page.keyboard.press("ArrowDown");
    assert.notEqual(await chosen(), one);
    await page.keyboard.press("ArrowUp");
    assert.equal(await chosen(), one);
    // typing in the filter keeps its arrows
    await page.keyboard.press("f");
    await page.keyboard.press("ArrowDown");
    assert.equal(await chosen(), one);
    await page.evaluate(() => document.activeElement.blur());
    await page.keyboard.press("Escape");

    // a menu: down from its button, and placed inside the window however narrow the list is
    await page.evaluate(async () => (await import("/actions.js")).actions["select-device"]({ dataset: { key: "" } }));
    for (const width of [1280, 390]) {
      await page.setViewportSize({ width, height: 800 });
      await page.click("[data-act=export-menu]");
      const box = await page.evaluate(() => {
        const rect = document.querySelector(".menu.exports").getBoundingClientRect();
        return { left: rect.left, right: rect.right, bottom: rect.bottom, wide: innerWidth, high: innerHeight };
      });
      assert.ok(box.left >= 0 && box.right <= box.wide && box.bottom <= box.high, `the export menu inside the window at ${width}: ${JSON.stringify(box)}`);
      await page.keyboard.press("ArrowDown");
      assert.equal(await page.evaluate(() => document.activeElement.id), "export-devices-csv");
      await page.keyboard.press("ArrowDown");
      assert.equal(await page.evaluate(() => document.activeElement.id), "export-site-xlsx");
      // the place in the menu survives a redraw
      await page.evaluate(async () => (await import("/store.js")).render());
      assert.equal(await page.evaluate(() => document.activeElement.id), "export-site-xlsx");
      await page.keyboard.press("Escape");
      assert.equal(await page.isVisible(".menu.exports"), false);
    }
    // an open menu follows the window when it gets narrower
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.click("[data-act=export-menu]");
    await page.setViewportSize({ width: 390, height: 800 });
    await page.waitForFunction(() => document.querySelector(".menu.exports")?.getBoundingClientRect().right <= innerWidth);
    await page.keyboard.press("Escape");
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.click("#site-switch");
    await page.keyboard.press("ArrowDown");
    assert.match(await page.evaluate(() => document.activeElement.id), /^site-choice-/);
    await page.keyboard.press("Escape");

    // the search settings: Tab stays inside, and an adapter that comes up shows without reopening
    await page.click("#search-settings");
    await page.click("#mode-tcp");
    await page.waitForSelector("#search-interface option", { state: "attached" });
    const before = await page.locator("#search-interface option").count();
    await page.route("**/api/adapters", async (route) => {
      const response = await route.fetch();
      const adapters = await response.json();
      adapters.push({ ...adapters[0], name: "vpn0", ip: "10.99.0.2", default: false });
      await route.fulfill({ response, json: adapters });
    });
    await page.evaluate(() => window.dispatchEvent(new Event("focus")));
    await page.waitForFunction((n) => document.querySelectorAll("#search-interface option").length === n + 1, before);
    assert.match(await page.textContent("#search-interface"), /vpn0/);
    // chosen, then gone again: the choice goes back to the default adapter
    await page.selectOption("#search-interface", "10.99.0.2");
    await page.unroute("**/api/adapters");
    await page.evaluate(() => document.activeElement.blur());
    await page.evaluate(() => window.dispatchEvent(new Event("focus")));
    await page.waitForFunction((n) => document.querySelectorAll("#search-interface option").length === n, before);
    assert.equal(await page.evaluate(async () => (await import("/store.js")).state.search.interface), "");
    for (let presses = 0; presses < 40; presses += 1) {
      await page.keyboard.press("Tab");
      assert.ok(await page.evaluate(() => Boolean(document.activeElement.closest(".modal-box"))), "Tab left the dialog");
    }
    await page.keyboard.press("Escape");
    assert.deepEqual(errors, []);
  },
};
