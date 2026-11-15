import { assert, chooseDevice, driveWithProfile, headsOverValues, noSidewaysScroll, open } from "./helpers.mjs";

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
    await headsOverValues(page, "mwatch");
    await page.keyboard.press("5");
    await page.selectOption("#record-interval", "5");
    await page.click("#next");
    await page.waitForSelector(".status .chip:has-text('Recording')");
    await page.waitForFunction(() => document.querySelector(".chart path[d^='M']"), null, { timeout: 15000 });
    await headsOverValues(page, "mrecord");
    const recording = await api("recording");
    assert.ok(recording.series[0].samples.length >= 1);
    await page.click("[data-act=record-stop]");
    await page.waitForSelector("[data-act=record-start]");
    // the charts with more to look at: two points of one unit over ten minutes, one reading failed
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.evaluate(async () => {
      const { state, render } = await import("/store.js");
      const start = Date.UTC(2026, 0, 5, 10, 0, 0);
      const line = (label, base) => ({ ...state.recording.series[0], label, point: label, unit: "Hz",
        samples: Array.from({ length: 121 }, (_, index) => [start + index * 5000, index === 60 ? null : base + Math.sin(index / 10) * 5]) });
      state.recording.series = [line("Output frequency", 25), line("Set frequency", 40)];
      render();
    });
    assert.match(await page.textContent(".legend"), /low 20 · high 30 · mean 2\d/);
    const draw = await page.locator(".chart .draw").boundingBox();
    const y = draw.y + draw.height / 2;
    // the crosshair names every line's value at the time under the pointer, and stays through a redraw
    await page.mouse.move(draw.x + draw.width / 4, y);
    await page.waitForSelector(".chart .tip:visible");
    assert.match(await page.textContent(".chart .tip"), /Output frequency: [\d.]+ Hz.*Set frequency: [\d.]+ Hz/s);
    await page.evaluate(async () => (await import("/store.js")).render());
    assert.ok(await page.isVisible(".chart .tip"), "the values stay under a pointer at rest");
    await page.mouse.move(draw.x + draw.width / 2, y);
    assert.match(await page.textContent(".chart .tip"), /Output frequency: no answer/);
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/chart-cross.png" });
    // a span drawn with the pointer is looked at closer; one button shows everything again
    const ends = () => page.evaluate(() => [...document.querySelectorAll(".chart .axis.x span")].map((end) => end.textContent));
    const whole = await ends();
    await page.mouse.move(draw.x + draw.width * 0.25, y);
    await page.mouse.down();
    await page.mouse.move(draw.x + draw.width * 0.4, y, { steps: 4 });
    assert.ok(await page.isVisible(".chart .span"), "the span shows while it is drawn");
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/chart-span.png" });
    await page.mouse.up();
    await page.waitForSelector("[data-act=chart-whole]");
    assert.notDeepEqual(await ends(), whole);
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/chart-zoomed.png" });
    // a click alone is no span
    await page.mouse.click(draw.x + draw.width / 2, y);
    await page.click("[data-act=chart-whole]");
    assert.deepEqual(await ends(), whole);
    assert.equal(await page.locator("[data-act=chart-whole]").count(), 0);
    await page.setViewportSize({ width: 390, height: 844 });
    await noSidewaysScroll(page);
    if (process.env.MODBUSIO_SHOTS) await page.screenshot({ path: process.env.MODBUSIO_SHOTS + "/chart-phone.png" });
  },
};
