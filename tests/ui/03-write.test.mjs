import { assert, chooseDevice, driveWithProfile, open } from "./helpers.mjs";

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
    await page.fill("#write-value", "70");
    await page.click("[data-act=write-check]");
    await page.click("[data-act=write]");
    await page.waitForSelector(".toast:has-text('70 is outside')");
    await page.click("[data-act=ask-write]");
    await page.fill("#write-value", "30,5");
    await page.keyboard.press("Enter");
    assert.match(await page.textContent(".ask b"), /Write 30,5 to Set frequency\?/);
    await page.click("[data-act=write]");
    await page.waitForSelector(".toast:has-text('Written: 30.5 Hz (was 25 Hz)')");
    await page.waitForSelector(".row.mpoints:has-text('30.5 Hz')");
    await page.waitForSelector(".status .chip:has-text('1 changed by you')");
    // after a restart the write still stands and can be put back
    await restart();
    await page.goto(`${base}/#/changed`);
    await page.waitForSelector(".row.mlog:has-text('still written')");
    await page.click("#next");
    assert.match(await page.textContent(".ask"), /Unlock writing/);
    await page.click("[data-act=unlock]");
    await page.click("[data-act=restore]");
    await page.waitForSelector(".toast:has-text('1 put back')");
    const journal = await api("journal");
    assert.equal(journal.standing, 0);
    assert.equal(journal.entries.at(-1).restores, 1);
  },
};
