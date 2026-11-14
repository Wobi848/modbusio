import { assert, headsOverValues, noSidewaysScroll, open } from "./helpers.mjs";

export default {
  name: "bus analyser: listen to the simulated trunk, units, exchanges, add a unit",
  async run({ page, base, api }) {
    const trunk = process.env.MODBUSIO_TEST_TRUNK;
    if (!trunk) return;
    await open(page, base, { area: "analyser" });
    await page.waitForSelector(`#analyser-port option[value="${trunk}"]`, { state: "attached" });
    // nothing heard yet: the page stays as it is, so an open choice list does not close
    const port = await page.$("#analyser-port");
    await page.waitForTimeout(2500);
    assert.ok(await port.evaluate((element) => element.isConnected), "the port's list was drawn anew");
    // looking again says what it found, also when nothing changed
    await page.click("[data-act=analyser-ports]");
    await page.waitForSelector("#toast:has-text('serial ports found')");
    await page.selectOption("#analyser-port", trunk);
    // finding the speed by listening finds the frame too: the frame says so instead of a choice
    assert.equal(await page.textContent("#analyser-frame"), "found by listening too");
    await page.selectOption("#analyser-speed", "19200");
    assert.equal(await page.inputValue("#analyser-frame"), "8E1");
    assert.equal(await page.textContent("#next"), "Start listening");
    await page.click("#next");
    await page.waitForSelector(".status .chip:has-text('Listening on')");

    // the trunk's faults, as the master meets them: a silent unit, a refused block, missed answers
    await page.waitForSelector(".row.aunits[data-chosen=u9] .chip:has-text('no answer')", { timeout: 20000 });
    await page.waitForSelector(".row.aunits[data-chosen=u7] .chip:has-text('× 02')");
    await page.waitForSelector(".row.aunits[data-chosen=u12] .chip:has-text('no answer')", { timeout: 20000 });
    await headsOverValues(page, "aunits");
    // and in words
    await page.waitForSelector(".analyser-notes .strip:has-text('unit 9, which never answered')");
    await page.waitForSelector(".analyser-notes .strip:has-text('exception 02 every time')");
    const heard = await api("analyser");
    assert.deepEqual(heard.units.map((unit) => unit.unit), [1, 5, 7, 9, 12]);

    await page.click(".row.aunits[data-chosen=u12]");
    await page.waitForSelector(".detail.open:has-text('Unit 12')");
    await page.click("[data-act=analyser-add-unit]");
    await page.waitForFunction(() => !document.querySelector(".row.aunits[data-chosen=u12]")?.textContent.includes("not on the site"));
    const site = await api("site");
    assert.ok(site.devices.some((device) => device.unit === 12 && device.key.startsWith(`serial:${trunk}:19200:8E1/`)));

    await page.click("[data-act=analyser-unit-exchanges]");
    await page.waitForSelector(".row.aexch:not(.head)");
    await headsOverValues(page, "aexch");
    const units = await page.locator(".row.aexch:not(.head) > :nth-child(2)").allTextContents();
    assert.ok(units.every((unit) => unit === "12"), `only unit 12: ${units}`);
    await page.selectOption("#analyser-unit", "");
    await page.check("text=only problems");
    const results = await page.locator(".row.aexch:not(.head)").evaluateAll((rows) => rows.map((row) => Boolean(row.querySelector(".chip.err"))));
    assert.ok(results.length && results.every(Boolean), "only exchanges with a problem");
    await page.click(".row.aexch:not(.head) >> nth=0");
    await page.waitForSelector(".detail.open .bytes");

    // the master's write of the drive's set frequency, with the value it read before
    await page.uncheck("text=only problems");
    await page.check("text=only writes");
    await page.waitForSelector(".row.aexch:has-text('write HR 1')", { timeout: 20000 });

    // what the master reads at the meter, register by register, and a register's changes
    await page.click("[data-act=analyser-view][data-view=registers]");
    await page.selectOption("#analyser-registers-unit", "1");
    await page.waitForSelector(".row.aregs[data-place='holding_register:1']");
    await headsOverValues(page, "aregs");
    await page.click(".row.aregs[data-place='holding_register:1']");
    await page.waitForSelector(".detail.open .facts.pairs");
    await page.waitForSelector(".detail.open .chart path[d^='M']", { timeout: 10000 });

    await page.click("[data-act=analyser-view][data-view=frames]");
    await page.waitForSelector(".row.aframes:has-text('01 03 00 00 00 08 44 0C')");
    await headsOverValues(page, "aframes");

    // the areas count up: the analyser is 7, the settings 8
    await page.keyboard.press("8");
    await page.waitForSelector("h1:has-text('Settings')");
    await page.keyboard.press("7");
    await page.waitForSelector("h1:has-text('Bus analyser')");

    await page.click("[data-act=analyser-view][data-view=units]");
    await page.setViewportSize({ width: 390, height: 800 });
    await noSidewaysScroll(page);
    await page.click(".row.aunits[data-chosen=u7]");
    await page.click(".detail.open .back");
    await page.click("[data-act=analyser-view][data-view=exchanges]");
    await noSidewaysScroll(page);
    await page.setViewportSize({ width: 1500, height: 900 });

    // a draft profile from what the master reads at the meter
    await page.click("[data-act=analyser-view][data-view=registers]");
    await page.selectOption("#analyser-registers-unit", "1");
    await page.waitForSelector(".row.aregs[data-place='holding_register:7']");
    await page.click("[data-act=analyser-draft-profile]");
    await page.waitForSelector("#profile-name");
    assert.equal(await page.inputValue("#profile-name"), "Unit 1 from the trunk");
    await page.waitForSelector(".row.mprofile-points:has-text('HR 7')");
    // not saved: closing asks first
    await page.click("[data-act=close-profile]");
    await page.click("[data-act=discard-profile]");
    await page.keyboard.press("7");

    await page.click("#next");
    await page.waitForFunction(() => document.querySelector("#next")?.textContent === "Start listening");
    assert.equal((await api("analyser")).running, false);

    // what was heard, saved and opened again: the same units, at the desk
    const saved = Buffer.from(await (await fetch(`${base}/api/analyser.pcapng`)).arrayBuffer());
    const choosing = page.waitForEvent("filechooser");
    await page.click("[data-act=analyser-open-file]");
    await (await choosing).setFiles({ name: "trunk.pcapng", mimeType: "application/octet-stream", buffer: saved });
    await page.waitForSelector(".pagehead .lead:has-text('trunk.pcapng')");
    await page.waitForSelector(".row.aunits[data-chosen=u9]");
    assert.equal((await api("analyser")).file, true);
  },
};
