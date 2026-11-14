import { assert, driveWithProfile, noSidewaysScroll, open } from "./helpers.mjs";

export default {
  name: "settings and layout: sites, German, dark, phone width without sideways scrolling",
  async setup(api) {
    await driveWithProfile(api);
  },
  async run({ page, base, api }) {
    await open(page, base, { area: "settings" });
    // typing before the page has its site would race the site's name arriving into the field
    await page.waitForFunction(() => document.querySelector("#site-name")?.value === "Site");
    // what is typed survives a redraw, as when the page finishes starting or a live value comes in
    const redraw = () => page.evaluate(async () => (await import("/store.js")).render());
    await page.fill("#site-name", "Site A");
    await page.fill("#new-site", "School");
    await redraw();
    assert.equal(await page.inputValue("#site-name"), "Site A");
    assert.equal(await page.inputValue("#new-site"), "School");
    await page.fill("#site-name", "Site");
    await page.click("[data-act=new-site]");
    await page.waitForSelector(".site-now:has-text('School')");
    assert.equal((await api("site")).devices.length, 0);
    await page.click(".site-line:has-text('Site') [data-act=site-open]");
    await page.waitForSelector(".site-now:has-text('Site')");
    await page.selectOption("#language", "de");
    await page.waitForSelector("h1:has-text('Einstellungen')");
    await page.selectOption("#theme", "dark");
    assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), "dark");
    for (const width of [390, 1000, 1440]) {
      await page.setViewportSize({ width, height: 844 });
      for (const area of ["workspace", "explorer", "profiles", "watch", "recording", "changed", "settings"]) {
        await page.goto(`${base}/#/${area}`);
        await page.waitForSelector("h1", { state: "attached" });
        await noSidewaysScroll(page);
      }
    }
    // the drafts are longer than English: nothing may push the page sideways either
    for (const language of ["fr", "it", "es"]) {
      for (const width of [390, 1000]) {
        await page.setViewportSize({ width, height: 844 });
        for (const area of ["workspace", "explorer", "changed", "settings"]) {
          await page.goto(`${base}/?lang=${language}#/${area}`);
          await page.waitForSelector("h1", { state: "attached" });
          assert.equal(await page.evaluate(() => document.documentElement.lang), language);
          await noSidewaysScroll(page);
        }
      }
    }
    await page.goto(`${base}/?lang=de#/workspace`);
    // on a phone the details slide over the list, with the way back
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(`${base}/#/workspace`);
    // the tree is not shown on a phone: the device list stands in for it
    await page.click(".row.mdevices:has-text('Drive')");
    await page.click("#next");
    await page.click(".row.mpoints:has-text('Set frequency')");
    assert.ok(await page.isVisible(".detail.open .big-value"));
    await page.click(".detail.open .back");
    await page.click(".phone-back");
    await page.waitForSelector(".row.mdevices:has-text('Drive')");
  },
};
