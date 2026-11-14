// The bus analyser's views *Registers* and *Frames* (docs/concept-analyser.md): what the master
// reads and writes, register by register, and the raw frames.

import { h } from "./html.js";
import { currentLocale, localNumber, t } from "./i18n.js";
import { chartHtml } from "./chart.js";
import { pairHtml } from "./explorer.js";
import { areaShort } from "./workspace.js";

const FRAMES_SHOWN = 300;

/** Wall-clock time of something heard `at` microseconds after the start, to the millisecond. */
export function heardTime(heard, at) {
  const time = new Date(heard.started_ms + at / 1000);
  return `${time.toLocaleTimeString(currentLocale(), { hour12: false })}.${String(time.getMilliseconds()).padStart(3, "0")}`;
}

/** A register's changes as a step line up to `untilMs`: a value holds until the next change. */
export function stepSamples(history, startedMs, untilMs) {
  const samples = [];
  for (const [at, value] of history) {
    const time = startedMs + at / 1000;
    if (samples.length) samples.push([time, samples.at(-1)[1]]);
    samples.push([time, value]);
  }
  if (samples.length && untilMs > samples.at(-1)[0]) samples.push([untilMs, samples.at(-1)[1]]);
  return samples;
}

/** The unit whose registers are shown: the one chosen, else the first heard. */
export function registersUnit(analyser) {
  const units = analyser.heard?.units ?? [];
  const chosen = units.find((unit) => String(unit.unit) === analyser.registersUnit);
  return chosen?.unit ?? units[0]?.unit ?? null;
}

/** The profile's point that starts at a register, if the unit has one there. */
const pointAt = (data, row) => data.points.find((point) => point.area === row.area && point.address === row.reading.address);

const placeKey = (row) => `${row.area}:${row.reading.address}`;

function registerRow(analyser, data, row) {
  const point = pointAt(data, row);
  const active = analyser.register === placeKey(row) ? " active" : "";
  const range = row.min === null ? "–" : row.min === row.max ? String(row.min) : `${row.min}–${row.max}`;
  const last = point ? h(localNumber(point.text)) : row.reading.value ?? "–";
  return `<div class="row aregs${active}" role="option" data-act="analyser-register" data-place="${placeKey(row)}">
    <span class="mono">${areaShort(row.area)} ${row.reading.address}</span><span class="cut">${point ? h(point.name) : ""}</span>
    <span class="num mono">${row.reads}</span><span class="num mono">${row.writes ? `<span class="pen">✎</span> ${row.writes}` : "0"}</span>
    <span class="num mono">${row.changes}</span><span class="num mono cut">${last}</span><span class="num mono">${range}</span></div>`;
}

export function registersHtml(state) {
  const analyser = state.analyser;
  const unit = registersUnit(analyser);
  const units = (analyser.heard?.units ?? []).map((item) =>
    `<option value="${item.unit}"${item.unit === unit ? " selected" : ""}>${item.unit}</option>`).join("");
  const toolbar = `<div class="toolbar"><label class="form-field inline">${t("col.unit")}<select id="analyser-registers-unit" data-analyser="registersUnit">${units}</select></label></div>`;
  const data = analyser.unitData?.unit === unit ? analyser.unitData : null;
  if (!data) return `${toolbar}<p class="hint pad">${t("an.loading")}</p>`;
  if (!data.registers.length) return `${toolbar}<div class="empty"><h2>${t("an.noRegisters")}</h2><p>${t("an.noRegistersLead")}</p></div>`;
  return `${toolbar}<div class="rows" data-keep="analyser-registers" role="listbox"><div class="row aregs head" role="row"><span>${t("col.address")}</span><span>${t("an.point")}</span><span class="num">${t("an.reads")}</span><span class="num">${t("an.writesShort")}</span><span class="num">${t("an.changes")}</span><span class="num">${t("an.last")}</span><span class="num">${t("an.range")}</span></div>
    ${data.registers.map((row) => registerRow(analyser, data, row)).join("")}</div>`;
}

export function registerDetail(state) {
  const analyser = state.analyser;
  const data = analyser.unitData;
  const row = data?.registers.find((item) => placeKey(item) === analyser.register);
  if (!row) return "";
  const point = pointAt(data, row);
  const reading = row.reading;
  const bits = row.area === "coil" || row.area === "discrete_input";
  const order = data.order ?? "ABCD";
  const pairs = (reading.pairs ?? []).map((pair) => pairHtml(pair, order)).join("");
  const history = analyser.history?.place === analyser.register ? analyser.history.values : [];
  const until = analyser.heard.started_ms + analyser.heard.elapsed_ms;
  const chart = history.length
    ? chartHtml([{ label: `${areaShort(row.area)} ${reading.address}`, unit: point ? "" : t("an.raw"), samples: stepSamples(history, analyser.heard.started_ms, until) }])
    : "";
  const facts = bits
    ? `<dt>${t("an.last")}</dt><dd class="mono">${reading.value ? t("on") : t("off")}</dd>`
    : `<dt>u16</dt><dd class="mono">${reading.value}</dd><dt>s16</dt><dd class="mono">${reading.signed}</dd><dt>hex</dt><dd class="mono">${h(reading.hex ?? "")}</dd>${reading.text ? `<dt>${t("ex.text")}</dt><dd class="mono">${h(reading.text)}</dd>` : ""}`;
  return `<div class="dhead"><b class="mono">${areaShort(row.area)} ${reading.address}</b><span class="sub">${point ? `${h(point.name)} · ${h(localNumber(point.text))}` : t("an.noPoint")}</span></div>
    <dl class="facts">${facts}<dt>${t("an.reads")}</dt><dd>${row.reads}</dd><dt>${t("an.writesShort")}</dt><dd>${row.writes}</dd>
      <dt>${t("an.lastSeen")}</dt><dd class="mono">${heardTime(analyser.heard, row.last_at)}</dd></dl>
    ${pairs ? `<h3>${t("ex.withNext")}</h3><dl class="facts pairs">${pairs}</dl>` : ""}
    <h3>${t("an.overTime")}</h3>${chart}<p class="hint">${t("an.fromAnswers")}</p>`;
}

/** The unit's answer times over time, for its details. */
export function answerChart(state, unit) {
  const data = state.analyser.unitData;
  if (data?.unit !== unit || !data.answer_times.length) return "";
  const started = state.analyser.heard.started_ms;
  return chartHtml([{ label: t("an.answerTime"), unit: "ms", samples: data.answer_times.map(([at, micros]) => [started + at / 1000, micros / 1000]) }]);
}

export function framesHtml(state) {
  const heard = state.analyser.heard;
  const frames = heard.frames;
  if (!frames.length) return `<p class="hint pad">${t("an.noFrames")}</p>`;
  const rows = [];
  for (let index = frames.length - 1; index >= 0 && rows.length < FRAMES_SHOWN; index--) {
    const frame = frames[index];
    const before = frames[index - 1];
    const gap = before && before.number === frame.number - 1 ? localNumber(((frame.at - before.at) / 1000).toFixed(1)) : "";
    const role = frame.role === "damaged" ? `<span class="chip err">${t("an.roleDamaged")}</span>` : t(`an.role.${frame.role}`);
    rows.push(`<div class="row aframes"><span class="mono">${heardTime(heard, frame.at)}</span><span class="num mono">${gap}</span><span>${role}</span><span class="mono bytes-cell">${h(frame.bytes)}</span></div>`);
  }
  return `<div class="rows" data-keep="analyser-frames"><div class="row aframes head" role="row"><span>${t("an.time")}</span><span class="num">${t("an.gapMs")}</span><span>${t("an.role")}</span><span>${t("an.bytes")}</span></div>${rows.join("")}</div>
    <p class="hint pad">${t("an.timesNote")}</p>`;
}
