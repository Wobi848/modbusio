import { assert, chooseDevice, driveWithProfile, headsOverValues, noSidewaysScroll, open } from "./helpers.mjs";

export default {
  name: "write: unlock, value, confirmation, logged with the old value, put back",
  async setup(api) {
    await driveWithProfile(api);
  },
  async run({ page, base, api, restart }) {
    await open(page, base);
    await chooseDevice(page, "Drive");
    await page.click("#next");
    await page.click(".row.mpoints:has-text('Set frequency')");
    await page.click("[data-act=ask-write]");
    // writing asks for the unlock first, and the server refuses until then
    assert.match(await page.textContent(".ask"), /Unlock writing for this session/);
    await assert.rejects(api("write", "POST", { key: "tcp:127.0.0.3:5020/2", point: "Set frequency", value: "30" }), /locked/);
    await page.click("[data-act=unlock]");
    // a value the point does not take is refused when checked, before the confirmation is asked
    await page.fill("#write-value", "70");
    await page.click("[data-act=write-check]");
    await page.waitForSelector(".toast:has-text('70 is outside')");
    assert.equal(await page.locator("[data-act=write]").count(), 0, "no confirmation for a value refused");
    await page.fill("#write-value", "30,5");
    await page.keyboard.press("Enter");
    // the value is checked by modbusio first, then the confirmation is asked
    await page.waitForSelector(".ask b:has-text('Write 30,5 to Set frequency?')");
    await page.click("[data-act=write]");
    await page.waitForSelector(".toast:has-text('Written: 30.5 Hz (was 25 Hz)')");
    await page.waitForSelector(".row.mpoints:has-text('30.5 Hz')");
    await page.waitForSelector(".status .chip:has-text('1 changed by you')");
    await headsOverValues(page, "mpoints");
    // after a restart the write still stands and can be put back
    await restart();
    await page.goto(`${base}/#/changed`);
    await page.waitForSelector(".row.mlog:has-text('still written')");
    await headsOverValues(page, "mlog");
    // at phone width the row keeps its state and button in sight, also with German's longer words;
    // the page stays (leaving it asks first while a value stands), only its size and language change
    await page.setViewportSize({ width: 390, height: 844 });
    await noSidewaysScroll(page);
    await page.keyboard.press("7");
    await page.selectOption("#language", "de");
    await page.keyboard.press("6");
    await page.waitForSelector(".row.mlog:has-text('Zurückschreiben')");
    await noSidewaysScroll(page);
    await page.keyboard.press("7");
    await page.selectOption("#language", "en");
    await page.keyboard.press("6");
    await page.setViewportSize({ width: 1500, height: 900 });
    await page.click("#next");
    assert.match(await page.textContent(".ask"), /Unlock writing/);
    await page.click("[data-act=unlock]");
    // the confirmation names the value that is written and its place
    await page.waitForSelector(".ask:has-text('Writes 25 Hz to Drive · Set frequency · HR 1.')");
    await page.click("[data-act=restore]");
    await page.waitForSelector(".toast:has-text('1 put back')");
    const journal = await api("journal");
    assert.equal(journal.standing, 0);
    assert.equal(journal.entries.at(-1).restores, 1);
  },
};
