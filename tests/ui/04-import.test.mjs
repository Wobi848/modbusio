import { assert, chooseDevice, open, SIM_PORT } from "./helpers.mjs";

const LIST = "HM-50 register list\nRegister;Name;Type;Unit\n40001;Energy;float;kWh\n40003;Flow temperature;float;°C\n40005;Return temperature;float;°C\n40009;Volume;uint16;m³\n";

export default {
  name: "import: a vendor's list previewed with values from the device, saved as its profile",
  async setup(api) {
    await api("devices/add", "POST", { key: `tcp:127.0.0.3:${SIM_PORT}/1`, name: "Heat meter" });
  },
  async run({ page, base, api }) {
    await open(page, base);
    await chooseDevice(page, "Heat meter");
    const chooser = page.waitForEvent("filechooser");
    await page.click(".detail [data-act=import-for-device]");
    await (await chooser).setFiles({ name: "hm50.csv", mimeType: "text/csv", buffer: Buffer.from(LIST) });
    await page.waitForSelector(".row.mimport:not(.head)");
    assert.match(await page.textContent(".import-box"), /like 40001/);
    // read as ABCD the swapped floats make no sense; CDAB gives the meter's values
    await page.selectOption("#import-order", "CDAB");
    await page.waitForSelector(".row.mimport:has-text('1234.5 kWh')");
    await page.fill("#import-name", "HM-50");
    await page.click("[data-act=import-save]");
    await page.waitForSelector(".toast:has-text('Profile HM-50 saved with 4 points')");
    await page.click("#next");
    await page.waitForSelector(".row.mpoints:has-text('1234.5 kWh')");
    const site = await api("site");
    assert.equal(site.devices[0].profile_name, "HM-50");
  },
};
