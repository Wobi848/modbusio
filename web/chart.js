// The recording's charts: one chart per unit, one line per point, a shared time axis. Plain SVG,
// coloured from the series tokens in tokens.css.

import { h } from "./html.js";
import { currentLocale, localNumber, t } from "./i18n.js";

const WIDTH = 800;
const HEIGHT = 220;
const PAD = { left: 56, right: 12, top: 12, bottom: 26 };
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
  const lines = series.map((item, index) => `<path d="${pathData(item.samples, box)}" fill="none" stroke="var(--series-${((offset + index) % COLOURS) + 1})" stroke-width="2"/>`).join("");
  const time = (ms) => new Date(ms).toLocaleTimeString(currentLocale(), { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  const value = (number) => localNumber(String(Number(number.toPrecision(4))));
  const legend = series.map((item, index) => `<span class="legend-item"><span class="swatch" style="background: var(--series-${((offset + index) % COLOURS) + 1})"></span>${h(item.label)}</span>`).join("");
  return `<figure class="chart"><figcaption>${h(unit || t("rec.noUnit"))}</figcaption>
    <svg viewBox="0 0 ${WIDTH} ${HEIGHT}" role="img" aria-label="${h(t("rec.chartLabel", { unit: unit || t("rec.noUnit") }))}" preserveAspectRatio="none">
      <line x1="${PAD.left}" y1="${PAD.top}" x2="${PAD.left}" y2="${HEIGHT - PAD.bottom}" stroke="var(--line-strong)"/>
      <line x1="${PAD.left}" y1="${HEIGHT - PAD.bottom}" x2="${WIDTH - PAD.right}" y2="${HEIGHT - PAD.bottom}" stroke="var(--line-strong)"/>
      <text x="${PAD.left - 6}" y="${PAD.top + 10}" text-anchor="end" class="axis">${value(box.high)}</text>
      <text x="${PAD.left - 6}" y="${HEIGHT - PAD.bottom}" text-anchor="end" class="axis">${value(box.low)}</text>
      <text x="${PAD.left}" y="${HEIGHT - 6}" class="axis">${time(box.first)}</text>
      <text x="${WIDTH - PAD.right}" y="${HEIGHT - 6}" text-anchor="end" class="axis">${time(box.last)}</text>
      ${lines}
    </svg><div class="legend">${legend}</div></figure>`;
}

export function chartHtml(series) {
  let offset = 0;
  return `<div class="charts">${byUnit(series).map(([unit, members]) => {
    const html = oneChart(unit, members, offset);
    offset += members.length;
    return html;
  }).join("")}</div>`;
}
