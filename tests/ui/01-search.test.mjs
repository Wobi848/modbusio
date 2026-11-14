import { assert, devicesListed, open, searchGateway, searchSubnet, SIM_PORT } from "./helpers.mjs";

export default {
  name: "search: a subnet, the units behind a gateway, and behind an RTU gateway",
  async run({ page, base, api }) {
    await open(page, base);
    assert.equal(await page.textContent("#next"), "Search devices …");
    await searchSubnet(page);
    const rows = await page.locator(".row.mdevices:not(.head) .name").allTextContents();
    assert.deepEqual(rows, ["EM-340", "Unit 1"], "the line stands in its own column, not in the name");
    const notes = await page.locator(".strip").allTextContents();
    assert.ok(notes.some((note) => note.includes("answers every unit ID")), notes.join(" | "));
    assert.ok(notes.some((note) => note.includes("is a gateway")), notes.join(" | "));
    await searchGateway(page, "127.0.0.3", false, 4);
    await searchGateway(page, "127.0.0.4", true, 7);
    const site = await api("site");
    assert.ok(site.devices.some((device) => device.key === `rtu-tcp:127.0.0.4:${SIM_PORT}/5`));
    await page.reload();
    await devicesListed(page, 7);
    // a device added by hand is chosen, and modbusio says whether it answers
    const addByHand = async (key, name) => {
      await page.click(".tree .node.level-1");
      await page.click("[data-act=add-device-menu]");
      await page.fill("#add-key", key);
      await page.fill("#add-name", name);
      await page.click("[data-act=add-device]");
    };
    await addByHand(`tcp:127.0.0.2:${SIM_PORT}/3`, "Meter 3");
    await page.waitForSelector("#toast:has-text('Meter 3 added.')");
    assert.equal(await page.textContent(".pagehead h1"), "Meter 3");
    await addByHand(`tcp:127.0.0.3:${SIM_PORT}/9`, "Unit nine");
    await page.waitForSelector("#toast:has-text('Unit nine added, but it does not answer: device behind gateway silent.')");
    assert.equal(await page.textContent(".pagehead h1"), "Unit nine");

    // a whole line leaves the site at once, after a question; the other lines stay
    const line = `127.0.0.2:${SIM_PORT}`;
    const lines = () => page.$$eval(".tree .node.heading", (nodes) => nodes.map((node) => node.querySelector(".label").textContent));
    assert.ok((await lines()).includes(line));
    await page.click(`.tree .node.heading[data-line="${line}"]`);
    assert.equal(await page.textContent(".pagehead h1"), line);
    const keys = await page.$$eval(".row.mdevices:not(.head)", (rows) => rows.map((row) => row.dataset.key));
    assert.ok(keys.length >= 2 && keys.every((key) => key.startsWith(`tcp:${line}/`)), `${keys}`);
    await page.click("[data-act=ask-remove-line]");
    await page.waitForSelector(".ask:has-text('Remove the line')");
    await page.click(".ask [data-act=remove-line]");
    await page.waitForSelector(`#toast:has-text('${line} removed from the site')`);
    assert.ok(!(await lines()).includes(line));
    assert.ok((await lines()).length >= 1, "the other lines stay");
  },
};
