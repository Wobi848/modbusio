// Shared steps of the browser tests.
import assert from "node:assert/strict";
export { assert };

/** The simulated devices: example, loopback address, more arguments. All share one port. */
export const SIMULATORS = [["simulated_meter", "127.0.0.2"], ["simulated_gateway", "127.0.0.3"], ["simulated_gateway", "127.0.0.4", "--rtu"]];
export const SIM_PORT = 5020;
export const SUBNET = "127.0.0.0/29";

/** Opens modbusio in `language` and `theme` at `area`. */
export async function open(page, base, { language = "en", area = "workspace", theme = "light" } = {}) {
  await page.addInitScript(([lang, look]) => {
    // only the first time: a test may change them and reload
    if (!localStorage.getItem("modbusio.language")) localStorage.setItem("modbusio.language", lang);
    if (!localStorage.getItem("modbusio.theme")) localStorage.setItem("modbusio.theme", look);
  }, [language, theme]);
  await page.goto(`${base}/#/${area}`);
  await page.waitForSelector(".side-card", { state: "attached" });
}

/** Waits until no search runs and the device list has `count` rows. */
export async function devicesListed(page, count) {
  await page.waitForFunction((n) => document.querySelectorAll(".row.mdevices:not(.head)").length === n && !document.querySelector("[data-act=stop-search]"), count, { timeout: 30000 });
}

/** Searches the simulators' subnet from the search settings. */
export async function searchSubnet(page) {
  await page.click("#search-settings");
  await page.click("#mode-tcp");
  await page.fill("#search-subnet", SUBNET);
  await page.fill("#search-port", String(SIM_PORT));
  await page.click(".modal [data-act=search]");
  await devicesListed(page, 2);
}

/** Searches the units behind the TCP gateway, 1 to 8. */
export async function searchGateway(page, address = "127.0.0.3", rtu = false, expected = 5) {
  await page.click("#search-settings");
  await page.click("#mode-gateway");
  await page.fill("#search-gateway", `${address}:${SIM_PORT}`);
  if (rtu) await page.check("#search-rtu");
  await page.fill("#search-to", "8");
  await page.fill("#search-timeout", "300");
  await page.click(".modal [data-act=search]");
  await devicesListed(page, expected);
}

/** Adds a device by its key through the API, with the drive's profile. */
export async function driveWithProfile(api) {
  const key = `tcp:127.0.0.3:${SIM_PORT}/2`;
  await api("devices/add", "POST", { key, name: "Drive" });
  const profile = await api("profiles", "POST", {
    id: "", name: "FU-7", points: [
      { name: "Set frequency", group: "Control", area: "holding_register", address: 1, type: "u16", factor: 0.01, unit: "Hz", writable: true, min: 0, max: 50 },
      { name: "Output frequency", group: "State", area: "holding_register", address: 100, type: "u16", factor: 0.01, unit: "Hz" },
      { name: "Run", group: "Control", area: "coil", address: 0, type: "bool", writable: true, states: [[0, "stopped"], [1, "running"]] },
    ],
  });
  await api("devices/update", "POST", { key, profile: profile.id });
  return key;
}

/** Chooses a device in the tree by the text it shows. */
export async function chooseDevice(page, text) {
  await page.click(`.tree .node:has-text("${text}")`);
}

/** The page has no sideways scrolling, and no part of a row is cut off at the right edge. */
export async function noSidewaysScroll(page) {
  const wide = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.ok(wide <= 1, `the page scrolls sideways by ${wide} px`);
  const cut = await page.evaluate(() => [...document.querySelectorAll(".row > *")]
    .filter((cell) => cell.getClientRects().length && cell.getBoundingClientRect().right > document.documentElement.clientWidth + 1)
    .map((cell) => cell.textContent.trim()));
  assert.deepEqual(cut, [], "parts of rows reach past the right edge");
}

/** Each column head stands over its values: the head row's grid has the same columns as the rows. */
export async function headsOverValues(page, rowClass) {
  const grids = await page.evaluate((name) => [...document.querySelectorAll(`.row.${name}:not(.group)`)]
    .map((row) => getComputedStyle(row).gridTemplateColumns), rowClass);
  assert.ok(grids.length >= 2, `a head and rows of .${rowClass}`);
  for (const grid of grids.slice(1)) assert.equal(grid, grids[0], `the columns of .${rowClass} differ from its head`);
}
