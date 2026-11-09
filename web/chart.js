// The recording's charts: one chart per unit, one line per point, a shared time axis. The lines are
// plain SVG stretched to the width, coloured from the series tokens in tokens.css; the axis texts are
// HTML beside it, so they keep their size on a phone instead of shrinking with the drawing.

import { h } from "./html.js";
import { currentLocale, localNumber, t } from "./i18n.js";

const WIDTH = 800;
const HEIGHT = 220;
const PAD = { left: 0, right: 0, top: 8, bottom: 8 };
const COLOURS = 8;

/** Series grouped by unit, in the order they come. */
export function byUnit(series) {
  const groups = new Map();
  for (const item of series) {
    if (!groups.has(item.unit)) groups.set(item.unit, []);
    groups.get(item.unit).push(item);
  }
  return [...groups.entries()];
}

/** The smallest and largest value and time over the series; a flat line gets some room. */
export function bounds(series) {
  let low = Infinity;
  let high = -Infinity;
  let first = Infinity;
  let last = -Infinity;
  for (const item of series) {
    for (const [time, value] of item.samples) {
      first = Math.min(first, time);
      last = Math.max(last, time);
      if (value === null) continue;
      low = Math.min(low, value);
      high = Math.max(high, value);
    }
  }
  if (low === Infinity) return null;
  if (low === high) {
    const room = Math.abs(low) * 0.05 || 1;
    low -= room;
    high += room;
  }
  if (first === last) last = first + 1000;
  return { low, high, first, last };
}

/** The line of one series as SVG path data; gaps where reading failed. */
export function pathData(samples, box) {
  const x = (time) => PAD.left + ((time - box.first) / (box.last - box.first)) * (WIDTH - PAD.left - PAD.right);
  const y = (value) => PAD.top + (1 - (value - box.low) / (box.high - box.low)) * (HEIGHT - PAD.top - PAD.bottom);
  let path = "";
  let drawing = false;
  for (const [time, value] of samples) {
    if (value === null) {
      drawing = false;
      continue;
    }
    path += `${drawing ? "L" : "M"}${x(time).toFixed(1)},${y(value).toFixed(1)}`;
    drawing = true;
  }
  return path;
}

function oneChart(unit, series, offset) {
  const box = bounds(series);
  if (!box) return `<p class="hint">${t("rec.noValues")}</p>`;
  const lines = series.map((item, index) => `<path d="${pathData(item.samples, box)}" fill="none" stroke="var(--series-${((offset + index) % COLOURS) + 1})" stroke-width="2" vector-effect="non-scaling-stroke"/>`).join("");
  const time = (ms) => new Date(ms).toLocaleTimeString(currentLocale(), { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  const value = (number) => localNumber(String(Number(number.toPrecision(4))));
  const legend = series.map((item, index) => `<span class="legend-item"><span class="swatch" style="background: var(--series-${((offset + index) % COLOURS) + 1})"></span>${h(item.label)}</span>`).join("");
  return `<figure class="chart"><figcaption>${h(unit || t("rec.noUnit"))}</figcaption>
    <div class="plot"><div class="axis y"><span>${value(box.high)}</span><span>${value(box.low)}</span></div>
      <svg viewBox="0 0 ${WIDTH} ${HEIGHT}" role="img" aria-label="${h(t("rec.chartLabel", { unit: unit || t("rec.noUnit") }))}" preserveAspectRatio="none">${lines}</svg>
      <div class="axis x"><span>${time(box.first)}</span><span>${time(box.last)}</span></div></div>
    <div class="legend">${legend}</div></figure>`;
}

export function chartHtml(series) {
  let offset = 0;
  return `<div class="charts">${byUnit(series).map(([unit, members]) => {
    const html = oneChart(unit, members, offset);
    offset += members.length;
    return html;
  }).join("")}</div>`;
}
