import { assert, noSidewaysScroll, open } from "./helpers.mjs";

// The help opens with ? and from the sidebar, shows a guide with the interface's button names and
// leads to the place of its first step, without doing anything there.
export default {
  name: "help: guides, the key ?, the way to the first step",
  async run({ page, base, api }) {
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.setViewportSize({ width: 1280, height: 900 });
    await open(page, base, { language: "de" });
    await page.click(".pagehead");
    await page.keyboard.press("?");
    await page.waitForSelector(".help-guide");
    assert.equal(await page.evaluate(() => location.hash), "#/help");
    assert.equal(await page.textContent(".pagehead h1"), "Hilfe");
    assert.equal(await page.getAttribute("a.help-link", "aria-current"), "page");
    // the page survives a reload
    await page.reload();
    await page.waitForSelector(".help-guide");

    // a guide names the buttons as they are written in the interface
    await page.click("button[data-guide=trunk]");
    assert.equal(await page.textContent("#help-guide-title"), "Eine RS-485-Leitung durchsuchen");
    assert.equal(await page.getAttribute("button[data-guide=trunk]", "aria-pressed"), "true");
    const named = await page.$$eval(".help-steps b", (bold) => bold.map((one) => one.textContent));
    assert.ok(named.includes("Sucheinstellungen") && named.includes("Zuerst mithören"), named.join(", "));
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/help.png" });

    // its button opens the search settings, and only opens them
    await page.click("button[data-act=help-go]");
    await page.waitForSelector(".modal-box.search-settings");
    assert.equal(await page.evaluate(() => location.hash), "#/workspace");
    assert.equal((await api("scan")).finished, true, "nothing was searched");
    await page.keyboard.press("Escape");

    // the keys, from the sidebar's link
    await page.click("a.help-link");
    await page.click("button[data-guide=keys]");
    await page.waitForSelector("#help-guide kbd");
    assert.equal(await page.textContent("#help-guide-title"), "Tasten");

    // every guide opens, and each button leads to its place
    const guides = await page.$$eval("button[data-act=help-guide]", (buttons) => buttons.map((button) => button.dataset.guide));
    assert.equal(guides.length, 11);
    for (const guide of guides) {
      await page.click(`button[data-guide=${guide}]`);
      assert.ok((await page.textContent("#help-guide")).length > 40, guide);
      assert.doesNotMatch(await page.textContent("#help-guide"), /[{}]|undefined/, guide);
    }
    for (const [guide, hash] of [["profile", "#/profiles"], ["explore", "#/explorer"], ["write", "#/changed"], ["record", "#/recording"], ["analyser", "#/analyser"], ["exports", "#/settings"]]) {
      await page.goto(`${base}/#/help`);
      await page.click(`button[data-guide=${guide}]`);
      await page.click("button[data-act=help-go]");
      await page.waitForFunction((wanted) => location.hash === wanted, hash);
    }
    // on a phone the list stands above the guide, nothing reaches past the edge
    await page.setViewportSize({ width: 390, height: 844 });
    for (const language of ["de", "fr", "it", "es"]) {
      await page.goto(`${base}/?lang=${language}#/help`);
      await page.waitForSelector(".help-guide");
      await page.click("button[data-guide=othersettings]");
      await noSidewaysScroll(page);
    }
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/help-phone.png", fullPage: true });
    assert.deepEqual(errors, []);
  },
};
