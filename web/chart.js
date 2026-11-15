// The recording's charts: one chart per unit, one line per point, a shared time axis. The lines are
// plain SVG stretched to the width, coloured from the series tokens in tokens.css; the axis texts are
// HTML beside it, so they keep their size on a phone instead of shrinking with the drawing. After
// bacnetio's charts: a crosshair with the values at a time, a span drawn with the pointer to look
// closer, key figures of each line, and fewer samples drawn where there are more than can show.

import { figures, inWindow, thinned } from "./analysis.js";
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

/** What each chart on screen shows, by its id, for the pointer. */
const shown = new Map();
/** The span of time looked at, by the name of a group of charts; none shows everything. */
const zooms = new Map();
/** Where the pointer is: the chart and the time under it. Drawn again after the page is. */
let pointer = null;
/** The time a span began at while one is drawn with the pointer held down. */
let dragFrom = null;

const timeText = (ms) => new Date(ms).toLocaleTimeString(currentLocale(), { hour: "2-digit", minute: "2-digit", second: "2-digit" });
const valueText = (number) => localNumber(String(Number(number.toPrecision(4))));

/** Lowest, highest and mean of a line, for its legend. */
function figuresHtml(item) {
  const found = figures(item.samples);
  if (found.low === null) return "";
  return `<span class="figures">${t("chart.figures", { low: valueText(found.low), high: valueText(found.high), mean: valueText(found.mean) })}</span>`;
}

function oneChart(id, unit, series, offset, range) {
  const box = bounds(series);
  if (!box) return `<p class="hint">${t("rec.noValues")}</p>`;
  if (range) Object.assign(box, { first: range.from, last: range.to });
  shown.set(id, { series, box });
  const colour = (index) => `var(--series-${((offset + index) % COLOURS) + 1})`;
  const lines = series.map((item, index) => `<path d="${pathData(thinned(item.samples), box)}" fill="none" stroke="${colour(index)}" stroke-width="2" vector-effect="non-scaling-stroke"/>`).join("");
  const legend = series.map((item, index) => `<span class="legend-item"><span class="swatch" style="background: ${colour(index)}"></span>${h(item.label)}${figuresHtml(item)}</span>`).join("");
  return `<figure class="chart"><figcaption>${h(unit || t("rec.noUnit"))}</figcaption>
    <div class="plot"><div class="axis y"><span>${valueText(box.high)}</span><span>${valueText(box.low)}</span></div>
      <div class="draw" data-chart="${h(id)}"><svg viewBox="0 0 ${WIDTH} ${HEIGHT}" role="img" aria-label="${h(t("rec.chartLabel", { unit: unit || t("rec.noUnit") }))}" preserveAspectRatio="none">${lines}</svg>
        <div class="span" hidden></div><div class="cross" hidden></div><div class="tip" hidden></div></div>
      <div class="axis x"><span>${timeText(box.first)}</span><span>${timeText(box.last)}</span></div></div>
    <div class="legend">${legend}</div></figure>`;
}

/**
 * The charts of `series`, one per unit. `name` tells this group of charts from the others on the
 * page: its span of time is kept under it.
 */
export function chartHtml(series, name) {
  const range = zooms.get(name) ?? null;
  const tools = range
    ? `<div class="chart-tools"><span class="hint">${t("chart.span", { from: timeText(range.from), to: timeText(range.to) })}</span><button type="button" class="quiet" data-act="chart-whole" data-chart="${h(name)}">${t("chart.whole")}</button></div>`
    : `<div class="chart-tools"><span class="hint">${t("chart.dragHint")}</span></div>`;
  let offset = 0;
  return `<div class="charts" data-charts="${h(name)}">${tools}${byUnit(series).map(([unit, members], index) => {
    const inView = members.map((item) => ({ ...item, samples: inWindow(item.samples, range) }));
    const html = oneChart(`${name}-${index}`, unit, inView, offset, range);
    offset += members.length;
    return html;
  }).join("")}</div>`;
}

/** Shows everything of a group of charts again. */
export function showWhole(name) {
  zooms.delete(name);
}

/** The chart under the pointer and the time there, or null off a chart. */
function placeOf(event) {
  const draw = event.target.closest?.(".chart .draw");
  const chart = draw && shown.get(draw.dataset.chart);
  if (!chart) return null;
  const rect = draw.getBoundingClientRect();
  const share = Math.min(Math.max((event.clientX - rect.left) / rect.width, 0), 1);
  return { id: draw.dataset.chart, name: draw.closest("[data-charts]").dataset.charts, time: chart.box.first + share * (chart.box.last - chart.box.first) };
}

export function pointerMoved(event) {
  pointer = placeOf(event);
  drawPointer();
}

export function pointerLeft() {
  pointer = null;
  dragFrom = null;
  drawPointer();
}

export function spanBegun(event) {
  dragFrom = placeOf(event)?.time ?? null;
}

/** Ends a span; true when it is wide enough to look at, so the page is drawn anew. */
export function spanEnded(event) {
  const place = placeOf(event);
  const from = dragFrom;
  dragFrom = null;
  if (from === null || !place) return false;
  const box = shown.get(place.id).box;
  // a click is not a span
  if (Math.abs(place.time - from) < (box.last - box.first) / 50) return false;
  zooms.set(place.name, { from: Math.min(from, place.time), to: Math.max(from, place.time) });
  pointer = null;
  return true;
}

/** The sample of a line nearest to a time. */
const nearest = (samples, time) => samples.reduce((best, sample) => (Math.abs(sample[0] - time) < Math.abs(best[0] - time) ? sample : best), samples[0]);

/**
 * Draws the crosshair, the values at its time and the span being drawn, on the charts as they are
 * on screen now; called after every drawing of the page, so they stay under a pointer at rest.
 */
export function drawPointer() {
  for (const draw of document.querySelectorAll(".chart .draw")) {
    const chart = shown.get(draw.dataset.chart);
    const [cross, tip, span] = [".cross", ".tip", ".span"].map((part) => draw.querySelector(part));
    const here = pointer && chart && draw.closest("[data-charts]").dataset.charts === pointer.name;
    cross.hidden = !here;
    tip.hidden = !here || draw.dataset.chart !== pointer.id;
    span.hidden = !here || dragFrom === null;
    if (!here) continue;
    const share = (time) => Math.min(Math.max((time - chart.box.first) / (chart.box.last - chart.box.first), 0), 1) * 100;
    cross.style.left = `${share(pointer.time)}%`;
    if (!span.hidden) {
      const [left, right] = [share(dragFrom), share(pointer.time)].sort((a, b) => a - b);
      Object.assign(span.style, { left: `${left}%`, width: `${right - left}%` });
    }
    if (tip.hidden) continue;
    const lines = chart.series.filter((item) => item.samples.length).map((item) => {
      const [, value] = nearest(item.samples, pointer.time);
      return `<div>${h(item.label)}: ${value === null ? t("chart.noAnswer") : `${valueText(value)} ${h(item.unit ?? "")}`}</div>`;
    }).join("");
    tip.innerHTML = `<div>${timeText(pointer.time)}</div>${lines}`;
    tip.style.left = `${Math.min(Math.max(share(pointer.time), 20), 80)}%`;
  }
}
