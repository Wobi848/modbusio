import { assert, open } from "./helpers.mjs";

export default {
  name: "devices at other settings: asked first, listened first, named and joined at theirs",
  async run({ page, base, api }) {
    const port = process.env.MODBUSIO_TEST_SETTINGS;
    if (!port) return;
    await open(page, base);
    await page.click("#search-settings");
    await page.click("#mode-serial");
    await page.selectOption("#search-port-serial", port);
    await page.selectOption("#search-baud", "19200");
    await page.selectOption("#search-frame", "8E1");
    await page.fill("#search-to", "6");
    // first a question with the time it takes and what it sends
    await page.click("[data-act=other-settings-ask]");
    await page.waitForSelector(".modal .ask:has-text('Ask every unit at every setting?')");
    assert.match(await page.textContent(".modal .ask"), /units 1–6 at 16 settings/);
    assert.match(await page.textContent(".modal .ask"), /master must be stopped/);
    await page.check("#search-all-speeds");
    await page.waitForSelector(".modal .ask:has-text('at 32 settings')");
    await page.uncheck("#search-all-speeds");
    await page.waitForSelector(".modal .ask:has-text('at 16 settings')");
    await page.click("[data-act=other-settings]");
    // the dialog stays open with how far it is, then what it found
    await page.waitForSelector(".modal .finding");
    await page.waitForSelector(".modal .strip:has-text('Unit 2 answers only at 9600 8E1')", { timeout: 120000 });
    assert.match(await page.textContent(".modal"), /Unit 5 answers only at 19200 8N2/);
    assert.match(await page.textContent(".modal details"), /Unit 1: 19200 8E1/);
    const keys = (await api("site")).devices.map((device) => device.key);
    for (const key of [`serial:${port}:19200:8E1/1`, `serial:${port}:9600:8E1/2`, `serial:${port}:19200:8N2/5`]) {
      assert.ok(keys.includes(key), `${key} in ${keys}`);
    }
  },
};
