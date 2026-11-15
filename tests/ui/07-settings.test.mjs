import { assert, driveWithProfile, headsOverValues, noSidewaysScroll, open, tablesFit } from "./helpers.mjs";

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
    await page.click("[data-act=theme][data-theme=dark]");
    assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), "dark");
    // accent and background: each choice shows at once, is marked as chosen and survives a reload
    const look = () => page.evaluate(() => {
      const style = getComputedStyle(document.documentElement);
      return { accent: style.getPropertyValue("--accent").trim(), surface: style.getPropertyValue("--surface").trim() };
    });
    const blueOnNavy = await look();
    await page.click("[data-act=accent][data-accent=teal]");
    await page.click("[data-act=background][data-background=black]");
    const tealOnBlack = await look();
    assert.notEqual(tealOnBlack.accent, blueOnNavy.accent);
    assert.notEqual(tealOnBlack.surface, blueOnNavy.surface);
    assert.equal(await page.getAttribute("button[data-accent=teal]", "aria-pressed"), "true");
    assert.equal(await page.getAttribute("button[data-background=black]", "aria-pressed"), "true");
    // the light theme's background is its own choice and leaves the dark one alone
    await page.click("[data-act=background][data-background=warm]");
    assert.deepEqual(await look(), tealOnBlack);
    await page.reload();
    await page.waitForSelector("button[data-accent=teal][aria-pressed=true]");
    assert.deepEqual(await look(), tealOnBlack);
    if (process.env.MODBUSIO_SHOTS) {
      await page.setViewportSize({ width: 1280, height: 900 });
      await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/look-dark.png" });
      await page.click("[data-act=theme][data-theme=light]");
      await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/look-light.png" });
      await page.setViewportSize({ width: 390, height: 844 });
      await page.locator("button[data-accent=teal]").scrollIntoViewIfNeeded();
      await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/look-phone.png" });
      await page.click("[data-act=theme][data-theme=dark]");
    }
    await page.click("[data-act=accent][data-accent=blue]");
    await page.click("[data-act=background][data-background=navy]");
    for (const width of [390, 1000, 1440]) {
      await page.setViewportSize({ width, height: 844 });
      for (const area of ["workspace", "explorer", "profiles", "watch", "recording", "changed", "problems", "help", "settings"]) {
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
    // no borders to drag on a phone
    assert.equal(await page.locator(".resize:visible").count(), 0);

    // on a wide window the sidebar, the tree and the details are resized by their borders
    await page.setViewportSize({ width: 1500, height: 900 });
    await page.goto(`${base}/#/workspace`);
    await page.click(".row.mdevices:has-text('Drive')");
    const width = (selector) => page.$eval(selector, (element) => Math.round(element.getBoundingClientRect().width));
    const drag = async (handle, by) => {
      const box = await page.locator(handle).boundingBox();
      // in the middle of the border, which for a column is only as tall as its head
      const y = box.y + Math.min(box.height / 2, 200);
      await page.mouse.move(box.x + box.width / 2, y);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + by, y, { steps: 5 });
      await page.mouse.up();
    };
    const side = await width(".side");
    await drag("#resize-side", 60);
    assert.ok(Math.abs(await width(".side") - (side + 60)) <= 2, "the sidebar grew by the drag");
    const tree = await width(".tree");
    await drag("#resize-tree", 40);
    assert.ok(Math.abs(await width(".tree") - (tree + 40)) <= 2, "the tree grew by the drag");
    // kept in this browser, also after a reload
    await page.reload();
    await page.waitForSelector(".tree");
    assert.ok(Math.abs(await width(".side") - (side + 60)) <= 2, "the sidebar kept its width");
    // the arrow keys move a focused border; a double-click hands it back to the window
    await page.focus("#resize-side");
    await page.keyboard.press("ArrowLeft");
    assert.ok(Math.abs(await width(".side") - (side + 44)) <= 2, "the arrow key moved it");
    await page.dblclick("#resize-side");
    assert.equal(await width(".side"), side);

    // at every width between phone and wide the tables fit their list, in German's longer words
    await page.goto(`${base}/?lang=de#/workspace`);
    for (const wide of [1000, 1150, 1280, 1500]) {
      await page.setViewportSize({ width: wide, height: 900 });
      for (const area of ["workspace", "profiles", "watch", "recording", "changed"]) {
        await page.goto(`${base}/?lang=de#/${area}`);
        await page.waitForSelector("h1", { state: "attached" });
        await tablesFit(page, `${area} ${wide} px`);
      }
    }
    await page.setViewportSize({ width: 1500, height: 900 });
    await page.goto(`${base}/?lang=en#/workspace`);
    await page.waitForSelector(".tree");

    // a click on a column head sorts the list: up, down, back to the site's own order
    await page.click(".tree [data-act=select-device][data-key='']");
    await page.waitForSelector(".row.mdevices:not(.head)");
    const names = () => page.$$eval(".row.mdevices:not(.head) > .name", (cells) => cells.map((cell) => cell.textContent.trim()));
    const own = await names();
    await page.click(".row.mdevices.head [data-key=name]");
    assert.deepEqual(await names(), [...own].sort((a, b) => a.localeCompare(b)));
    await page.click(".row.mdevices.head [data-key=name]");
    assert.deepEqual(await names(), [...own].sort((a, b) => b.localeCompare(a)));
    await page.click(".row.mdevices.head [data-key=name]");
    assert.deepEqual(await names(), own);

    // a column as wide as its border is dragged, kept after a reload, back on a double-click
    const lineWidth = () => page.$eval(".row.mdevices.head > :nth-child(2)", (cell) => Math.round(cell.getBoundingClientRect().width));
    const before = await lineWidth();
    // a column grows by what the first one can give: little, with the tree widened above
    await drag(".row.mdevices.head [data-key=line] + .col-border", 15);
    assert.ok(Math.abs(await lineWidth() - (before + 15)) <= 3, "the column grew by the drag");
    await page.reload();
    await page.waitForSelector(".row.mdevices.head");
    assert.ok(Math.abs(await lineWidth() - (before + 15)) <= 3, "the column kept its width");
    await page.dblclick(".row.mdevices.head [data-key=line] + .col-border");
    assert.ok(Math.abs(await lineWidth() - before) <= 3, "the column is back");
    await headsOverValues(page, "mdevices");
  },
};
