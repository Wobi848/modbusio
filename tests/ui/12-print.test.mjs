import { assert, chooseDevice, driveWithProfile, noSidewaysScroll, open } from "./helpers.mjs";

// The open device's points as a page to print, and Excel files worded for their reader.
export default {
  name: "print view of a device's points; Excel links carry the reader's heads",
  async setup(api) {
    await driveWithProfile(api);
  },
  async run({ page, base }) {
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.setViewportSize({ width: 1280, height: 900 });
    await open(page, base, { language: "de" });
    await chooseDevice(page, "Drive");
    await page.click("#next");
    await page.waitForSelector(".row.mpoints:has-text('Set frequency')");

    await page.click("[data-act=print-open]");
    await page.waitForSelector(".print-table");
    assert.equal(await page.isVisible(".side"), false, "modbusio's own frame is not printed");
    assert.equal(await page.textContent(".print-page h1"), "Drive");
    assert.match(await page.textContent(".print-facts"), /Alle 3 Datenpunkte/);
    assert.equal(await page.locator(".print-table tbody tr:not(.print-group)").count(), 3);
    assert.deepEqual(await page.$$eval(".print-group th", (heads) => heads.map((head) => head.textContent.trim().replace(/\s+/g, " "))), ["Control 2", "State 1"]);
    assert.match(await page.textContent(".print-table"), /Output frequency.*24[.,]98 Hz/s);
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/print.png" });
    await page.setViewportSize({ width: 390, height: 844 });
    await noSidewaysScroll(page);
    await page.setViewportSize({ width: 1280, height: 900 });
    // back with its button, and with Escape
    await page.click("[data-act=print-close]");
    await page.waitForSelector(".row.mpoints");
    assert.ok(await page.isVisible(".side"));
    // the list's search narrows the page too
    await page.fill("#point-filter", "frequency");
    await page.click("[data-act=print-open]");
    assert.match(await page.textContent(".print-facts"), /"frequency": 2 von 3 Datenpunkten/);
    assert.equal(await page.locator(".print-table tbody tr:not(.print-group)").count(), 2);
    await page.keyboard.press("Escape");
    await page.waitForSelector(".row.mpoints");

    // Excel: the heads in the reader's words and a title line go along; the file comes
    await page.fill("#point-filter", "");
    await page.click(".row.mpoints:has-text('Set frequency')");
    await page.keyboard.press("Escape");
    const href = await page.getAttribute("a[href*='points.xlsx']", "href");
    const query = new URL(href, base).searchParams;
    assert.equal(JSON.parse(query.get("heads")).Value, "Wert");
    assert.match(query.get("title"), /^Site · Drive · .* · modbusio \d/);
    const response = await page.request.get(new URL(href, base).href);
    assert.equal(response.status(), 200);
    assert.equal((await response.body()).subarray(0, 2).toString(), "PK");
    assert.deepEqual(errors, []);
  },
};
