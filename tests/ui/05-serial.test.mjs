import { assert, devicesListed, open } from "./helpers.mjs";

export default {
  name: "serial line: listen first, find the settings, search the units",
  async run({ page, base }) {
    if (!process.env.MODBUSIO_TEST_SERIAL) return;
    await open(page, base);
    await page.click("#search-settings");
    await page.click("#mode-serial");
    await page.selectOption("#search-port-serial", process.env.MODBUSIO_TEST_SERIAL);
    // on a phone the button is at the end of a long form: what was heard shows where one tapped
    const size = page.viewportSize();
    await page.setViewportSize({ width: 390, height: 700 });
    await page.click("[data-act=listen]");
    await page.waitForSelector(".modal .strip:has-text('is quiet')", { timeout: 60000 });
    const heard = await page.locator(".modal .strip").boundingBox();
    assert.ok(heard.y >= 0 && heard.y + heard.height <= 700, `the answer is on screen: ${JSON.stringify(heard)}`);
    await page.setViewportSize(size);
    await page.click("[data-act=find-settings]");
    await page.waitForSelector(".status:has-text('Ready')", { timeout: 30000 });
    await page.click("#search-settings");
    assert.equal(await page.inputValue("#search-baud"), "9600");
    await page.fill("#search-to", "8");
    await page.fill("#search-timeout", "150");
    await page.click(".modal [data-act=search]");
    await devicesListed(page, 3);
  },
};
