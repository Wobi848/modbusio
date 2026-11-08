import { assert, devicesListed, open, searchGateway, searchSubnet, SIM_PORT } from "./helpers.mjs";

export default {
  name: "search: a subnet, the units behind a gateway, and behind an RTU gateway",
  async run({ page, base, api }) {
    await open(page, base);
    assert.equal(await page.textContent("#next"), "Search devices …");
    await searchSubnet(page);
    const rows = await page.locator(".row.mdevices:not(.head) .name").allTextContents();
    assert.deepEqual(rows, ["EM-340", `127.0.0.3:${SIM_PORT} · unit 1`]);
    const notes = await page.locator(".strip").allTextContents();
    assert.ok(notes.some((note) => note.includes("answers every unit ID")), notes.join(" | "));
    assert.ok(notes.some((note) => note.includes("is a gateway")), notes.join(" | "));
    await searchGateway(page, "127.0.0.3", false, 4);
    await searchGateway(page, "127.0.0.4", true, 7);
    const site = await api("site");
    assert.ok(site.devices.some((device) => device.key === `rtu-tcp:127.0.0.4:${SIM_PORT}/5`));
    await page.reload();
    await devicesListed(page, 7);
  },
};
