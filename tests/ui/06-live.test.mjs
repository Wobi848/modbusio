import { assert, chooseDevice, driveWithProfile, open } from "./helpers.mjs";

export default {
  name: "watch list and recording: points across devices, charts and samples",
  async setup(api) {
    await driveWithProfile(api);
  },
  async run({ page, base, api }) {
    await open(page, base);
    await chooseDevice(page, "Drive");
    await page.click("#next");
    await page.click(".row.mpoints:has-text('Output frequency')");
    await page.click("[data-act=toggle-watch]");
    await page.click("[data-act=toggle-record]");
    await page.waitForSelector(".row.mpoints:has-text('watched')");
    await page.keyboard.press("4");
    await page.waitForSelector(".row.mwatch:has-text('24.98 Hz')");
    await page.keyboard.press("5");
    await page.selectOption("#record-interval", "5");
    await page.click("#next");
    await page.waitForSelector(".status .chip:has-text('Recording')");
    await page.waitForFunction(() => document.querySelector(".chart path[d^='M']"), null, { timeout: 15000 });
    const recording = await api("recording");
    assert.ok(recording.series[0].samples.length >= 1);
    await page.click("[data-act=record-stop]");
    await page.waitForSelector("[data-act=record-start]");
  },
};
