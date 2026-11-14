// The bus analyser (docs/concept-analyser.md): what a trunk's own master asks and what its units
// answer, heard without sending. Starting, the units and the exchanges; the buttons are in
// analyser-actions.js.

import { h } from "./html.js";
import { codeText, localNumber, noteText, t } from "./i18n.js";
import { answerChart, framesHtml, heardTime, registerDetail, registersHtml } from "./analyser-registers.js";
import { deviceName, registerView } from "./store.js";
import { areaShort } from "./workspace.js";

export const BAUDS = ["1200", "2400", "4800", "9600", "19200", "38400", "57600", "115200"];
export const FRAMES = ["8E1", "8N1", "8N2", "8O1"];
/** Exchanges and frames the page keeps; the server keeps more and gives them in pages. */
export const KEPT = 2000;
/** Rows drawn at most: the newest. */
const SHOWN = 300;
const VALUES_SHOWN = 6;
/** Notes that inform rather than warn. */
const INFO_NOTES = ["slow_answers", "broadcasts", "bus_busy"];

/** The page's lists with what the server answered after them; a new analysis replaces them. */
export function takeHeard(held, answer) {
  if (!answer.running && answer.started_ms === undefined) return null;
  const same = held && held.started_ms === answer.started_ms;
  const keep = (old, added) => [...(same ? old : []), ...added].slice(-KEPT);
  return { ...answer, exchanges: keep(held?.exchanges ?? [], answer.exchanges), frames: keep(held?.frames ?? [], answer.frames) };
}

/** The number of the last item of a list, for asking only what came after it. */
export const lastNumber = (list) => list?.at(-1)?.number;

/** The site's device on this port with this unit ID, if there is one. */
export function siteDevice(state, port, unit) {
  return state.site.devices.find((device) => device.unit === unit && device.key.startsWith(`serial:${port}:`)) ?? null;
}

/** What a request asked for, in words: "HR 100, 2 values", "write HR 1". */
export function askedText(exchange) {
  const asked = exchange.asked;
  if (!asked) return t("an.function", { code: exchange.function.toString(16).toUpperCase().padStart(2, "0") });
  const place = `${areaShort(asked.area)} ${asked.address}`;
  if (asked.write) return asked.count > 1 ? t("an.writeMany", { place, n: asked.count }) : t("an.write", { place });
  return asked.count > 1 ? t("an.read", { place, n: asked.count }) : place;
}

/** What came back, in words. */
export function resultText(exchange) {
  const outcome = exchange.outcome;
  switch (outcome.kind) {
    case "registers": {
      const shown = outcome.values.slice(0, VALUES_SHOWN).join(" ");
      return outcome.values.length > VALUES_SHOWN ? `${shown} …` : shown;
    }
    case "bits": return outcome.values.map((on) => (on ? "1" : "0")).join("");
    case "written": {
      const after = exchange.asked?.values?.join(" ");
      if (!after) return t("an.written");
      return exchange.before ? t("an.writtenFrom", { before: exchange.before.join(" "), after }) : t("an.writtenValues", { after });
    }
    case "exception": {
      const code = outcome.code.toString(16).padStart(2, "0");
      return t("an.exception", { code: code.toUpperCase(), text: codeText(`exception_${code}`) });
    }
    case "answered": return t("an.answered");
    case "malformed": return codeText("malformed");
    case "damaged": return codeText("damaged");
    case "no_answer": return t("an.noAnswer");
    case "broadcast": return t("an.broadcast");
    default: return t("an.answerOnly");
  }
}

export const failed = (exchange) => ["exception", "malformed", "damaged", "no_answer"].includes(exchange.outcome.kind);

/** The exchanges the filters let through, newest first. */
export function filteredExchanges(analyser) {
  const unit = analyser.unitFilter === "" ? null : Number(analyser.unitFilter);
  return (analyser.heard?.exchanges ?? []).filter((exchange) =>
    (unit === null || exchange.unit === unit)
    && (!analyser.problemsOnly || failed(exchange))
    && (!analyser.writesOnly || exchange.asked?.write)).reverse();
}

const millis = (value) => localNumber(String(Math.round(value)));


function startCard(state) {
  const analyser = state.analyser;
  const ports = state.serialPorts.map((port) =>
    `<option value="${h(port.name)}"${port.name === analyser.port ? " selected" : ""}>${h(port.name)}${port.description ? ` · ${h(port.description)}` : ""}</option>`).join("");
  const speeds = [["listen", t("an.findSpeed")], ...BAUDS.map((baud) => [baud, baud])].map(([value, label]) =>
    `<option value="${value}"${value === analyser.speed ? " selected" : ""}>${label}</option>`).join("");
  const frames = FRAMES.map((frame) => `<option${frame === analyser.frame ? " selected" : ""}>${frame}</option>`).join("");
  return `<section class="card analyser-start"><h2>${t("an.startTitle")}</h2><p>${t("an.startLead")}</p>
    <pre class="wiring" aria-label="${t("an.wiringLabel")}">${h(t("an.wiring"))}</pre>
    <p class="hint">${t("an.swapped")}</p>
    <div class="start-fields">
      <div class="port-field"><label class="form-field">${t("search.serialPort")}<select id="analyser-port" data-analyser="port">${ports || `<option value="">${t("an.noPort")}</option>`}</select></label>
        <button type="button" class="quiet" data-act="analyser-ports">${t("an.lookAgain")}</button></div>
      <label class="form-field">${t("search.baud")}<select id="analyser-speed" data-analyser="speed">${speeds}</select></label>
      <label class="form-field">${t("search.frame")}<select id="analyser-frame" data-analyser="frame"${analyser.speed === "listen" ? " disabled" : ""}>${frames}</select></label>
    </div>
    <p class="hint">${t("an.heldHint")}</p></section>`;
}

function unitRow(state, unit) {
  const analyser = state.analyser;
  const device = siteDevice(state, analyser.heard.port, unit.unit);
  const problems = [
    unit.no_answer ? `<span class="chip err">${t("an.chipNoAnswer", { n: unit.no_answer })}</span>` : "",
    unit.damaged ? `<span class="chip err">${t("an.chipDamaged", { n: unit.damaged })}</span>` : "",
    ...Object.entries(unit.exceptions).map(([code, count]) =>
      `<span class="chip err">${count} × ${Number(code).toString(16).toUpperCase().padStart(2, "0")}</span>`),
  ].join("");
  const answer = unit.answer_ms ? `${millis(unit.answer_ms.min)}–${millis(unit.answer_ms.max)}` : "–";
  const cycle = unit.cycle_ms ? localNumber((unit.cycle_ms.median / 1000).toFixed(1)) : "–";
  const active = analyser.chosen === `u${unit.unit}` ? " active" : "";
  return `<div class="row aunits${active}" role="option" data-act="analyser-choose" data-chosen="u${unit.unit}">
    <span class="cut"><b>${unit.unit}</b> · ${device ? h(deviceName(device)) : `<span class="hint">${t("an.notOnSite")}</span>`}</span>
    <span class="num mono">${unit.requests}</span><span class="num mono">${unit.answers}</span>
    <span class="chips">${problems || "–"}</span><span class="num mono">${answer}</span><span class="num mono">${cycle}</span></div>`;
}

function unitsHtml(state) {
  const units = state.analyser.heard.units;
  if (!units.length) return `<div class="empty"><h2>${t("an.nothingYet")}</h2><p>${t("an.nothingYetLead")}</p></div>`;
  return `<div class="rows" data-keep="analyser-units" role="listbox"><div class="row aunits head" role="row"><span>${t("col.unit")}</span><span class="num">${t("an.requests")}</span><span class="num">${t("an.answers")}</span><span>${t("an.problems")}</span><span class="num">${t("an.answerMs")}</span><span class="num">${t("an.cycleS")}</span></div>
    ${units.map((unit) => unitRow(state, unit)).join("")}</div>`;
}

function exchangeRow(state, exchange) {
  const active = state.analyser.chosen === `e${exchange.number}` ? " active" : "";
  const result = failed(exchange) ? `<span class="chip err">${h(resultText(exchange))}</span>` : h(resultText(exchange));
  const pen = exchange.asked?.write ? `<span class="pen" title="${t("an.writtenByMaster")}">✎</span> ` : "";
  const ms = exchange.answer_micros === null ? "–" : millis(exchange.answer_micros / 1000);
  return `<div class="row aexch${active}" role="option" data-act="analyser-choose" data-chosen="e${exchange.number}">
    <span class="mono">${heardTime(state.analyser.heard, exchange.at)}</span><span class="mono">${exchange.unit}</span>
    <span class="cut">${pen}${h(askedText(exchange))}</span><span class="cut">${result}</span><span class="num mono">${ms}</span></div>`;
}

function exchangesHtml(state) {
  const analyser = state.analyser;
  const units = analyser.heard.units.map((unit) => `<option value="${unit.unit}"${String(unit.unit) === analyser.unitFilter ? " selected" : ""}>${unit.unit}</option>`).join("");
  const shown = filteredExchanges(analyser);
  const filters = `<div class="toolbar"><label class="form-field inline">${t("col.unit")}<select id="analyser-unit" data-analyser="unitFilter"><option value="">${t("an.allUnits")}</option>${units}</select></label>
    <label class="check"><input type="checkbox" data-act="analyser-check" data-field="problemsOnly"${analyser.problemsOnly ? " checked" : ""}> ${t("an.problemsOnly")}</label>
    <label class="check"><input type="checkbox" data-act="analyser-check" data-field="writesOnly"${analyser.writesOnly ? " checked" : ""}> ${t("an.writesOnly")}</label></div>`;
  const rows = shown.length
    ? shown.slice(0, SHOWN).map((exchange) => exchangeRow(state, exchange)).join("")
    : `<p class="hint pad">${t("an.noExchanges")}</p>`;
  return `${filters}<div class="rows" data-keep="analyser-exchanges" role="listbox"><div class="row aexch head" role="row"><span>${t("an.time")}</span><span>${t("col.unit")}</span><span>${t("an.request")}</span><span>${t("an.result")}</span><span class="num">ms</span></div>${rows}</div>`;
}

/** The bytes of a frame with their parts named: unit, function, data, CRC. */
export function bytesHtml(hex) {
  const bytes = hex.split(" ");
  if (bytes.length < 4) return `<div class="bytes mono">${h(hex)}</div>`;
  const part = (text, label) => `<span>${h(text)}<small>${label}</small></span>`;
  const data = bytes.slice(2, -2).join(" ");
  return `<div class="bytes mono">${part(bytes[0], t("col.unit"))}${part(bytes[1], t("an.functionPart"))}${data ? part(data, t("an.dataPart")) : ""}${part(bytes.slice(-2).join(" "), "CRC")}</div>`;
}

function unitDetail(state, number) {
  const heard = state.analyser.heard;
  const unit = heard.units.find((item) => item.unit === number);
  if (!unit) return "";
  const device = siteDevice(state, heard.port, unit.unit);
  const times = (summary, scale, digits) => (summary
    ? t("an.times", { min: localNumber((summary.min / scale).toFixed(digits)), median: localNumber((summary.median / scale).toFixed(digits)), p95: localNumber((summary.p95 / scale).toFixed(digits)), max: localNumber((summary.max / scale).toFixed(digits)) })
    : "–");
  const exceptions = Object.entries(unit.exceptions).map(([code, count]) => {
    const hex = Number(code).toString(16).padStart(2, "0");
    return `${count} × ${hex.toUpperCase()} (${h(codeText(`exception_${hex}`))})`;
  }).join("<br>") || "–";
  const add = device ? "" : `<button type="button" class="tonal" data-act="analyser-add-unit" data-unit="${unit.unit}">${t("an.addToSite")}</button>`;
  return `<div class="dhead"><b>${t("an.unitTitle", { unit: unit.unit })}</b><span class="sub">${device ? h(deviceName(device)) : t("an.notOnSite")}</span></div>
    <dl class="facts"><dt>${t("an.requests")}</dt><dd>${unit.requests}</dd><dt>${t("an.answers")}</dt><dd>${unit.answers}</dd>
      <dt>${t("an.noAnswers")}</dt><dd>${unit.no_answer}</dd><dt>${t("an.damagedAnswers")}</dt><dd>${unit.damaged}</dd>
      <dt>${t("an.exceptions")}</dt><dd>${exceptions}</dd><dt>${t("an.writes")}</dt><dd>${unit.writes}</dd>
      <dt>${t("an.answerTime")}</dt><dd>${times(unit.answer_ms, 1, 0)}</dd><dt>${t("an.cycle")}</dt><dd>${times(unit.cycle_ms, 1000, 1)}</dd>
      ${unit.answer_only ? `<dt>${t("an.answerOnlyCount")}</dt><dd>${unit.answer_only}</dd>` : ""}</dl>
    <div class="actions"><button type="button" class="quiet" data-act="analyser-unit-exchanges" data-unit="${unit.unit}">${t("an.showExchanges")}</button>
      <button type="button" class="quiet" data-act="analyser-unit-registers" data-unit="${unit.unit}">${t("an.showRegisters")}</button>${add}</div>
    <h3>${t("an.answerTime")}</h3>${answerChart(state, unit.unit) || `<p class="hint">–</p>`}`;
}

function exchangeDetail(state, number) {
  const heard = state.analyser.heard;
  const exchange = heard.exchanges.find((item) => item.number === number);
  if (!exchange) return "";
  const device = siteDevice(state, heard.port, exchange.unit);
  const who = device ? deviceName(device) : t("an.unitTitle", { unit: exchange.unit });
  const result = failed(exchange) ? `<span class="chip err">${h(resultText(exchange))}</span>` : h(resultText(exchange));
  const values = exchange.outcome.kind === "registers" && exchange.outcome.values.length > VALUES_SHOWN
    ? `<p class="mono values">${exchange.outcome.values.join(" ")}</p>` : "";
  return `<div class="dhead"><b>${h(askedText(exchange))}</b><span class="sub">${h(who)} · ${heardTime(heard, exchange.at)}</span></div>
    ${exchange.request ? `<h3>${t("an.request")}</h3>${bytesHtml(exchange.request)}` : ""}
    <h3>${t("an.answer")}</h3>${exchange.answer ? bytesHtml(exchange.answer) : ""}<p>${result}</p>${values}
    ${exchange.answer_micros === null ? "" : `<p class="hint">${t("an.answerAfter", { ms: millis(exchange.answer_micros / 1000) })}</p>`}
    ${exchange.asked?.write ? `<p class="hint">${t("an.writtenByMaster")}</p>` : ""}`;
}

function detailHtml(state) {
  const analyser = state.analyser;
  const chosen = analyser.chosen;
  let inner = "";
  if (analyser.view === "registers") inner = registerDetail(state);
  else if (chosen) inner = chosen[0] === "u" ? unitDetail(state, Number(chosen.slice(1))) : exchangeDetail(state, Number(chosen.slice(1)));
  if (!inner) return `<aside class="detail"><p class="hint">${t("an.choose")}</p></aside>`;
  return `<aside class="detail open" data-keep="analyser-detail"><button type="button" class="quiet back" data-act="analyser-back">← ${t("back")}</button>${inner}</aside>`;
}

/** CSV of the table in view (the exchanges for the frames), and all tables as one Excel file. */
function exportLinks(view) {
  const offset = -new Date().getTimezoneOffset();
  const table = view === "frames" ? "exchanges" : view;
  return `<a class="button quiet" href="/api/analyser.csv?table=${table}&offset=${offset}" download>CSV</a><a class="button quiet" href="/api/analyser.xlsx?offset=${offset}" download>Excel</a>`;
}

function heardHtml(state) {
  const analyser = state.analyser;
  const views = [["units", t("an.units", { n: analyser.heard.units.length })], ["exchanges", t("an.exchanges")], ["registers", t("an.registers")], ["frames", t("an.frames")]].map(([view, label]) =>
    `<button type="button" data-act="analyser-view" data-view="${view}" aria-pressed="${analyser.view === view}">${label}</button>`).join("");
  const ended = !analyser.heard.running && analyser.heard.ended !== "stopped"
    ? `<div class="strip err">${t("an.endedBy", { reason: h(analyser.heard.ended) })}</div>` : "";
  const notes = (analyser.heard.notes ?? []).map((note) =>
    `<div class="strip ${INFO_NOTES.includes(note.code) ? "info" : "warn"}">${h(noteText(note))}</div>`).join("");
  const strips = ended || notes ? `<div class="strips analyser-notes">${ended}${notes}</div>` : "";
  const toolbar = `<div class="toolbar"><span class="seg" role="group" aria-label="${t("an.views")}">${views}</span>
    <div class="end">${analyser.heard.running ? `<button type="button" class="quiet" data-act="analyser-pause">${analyser.paused ? t("an.resume") : t("an.pause")}</button>` : ""}
    <button type="button" class="quiet" data-act="analyser-clear">${t("an.clear")}</button>
    ${exportLinks(analyser.view)}
    <a class="button quiet" href="/api/analyser.pcapng" download>${t("an.savePcap")}</a></div></div>`;
  const bodies = { units: unitsHtml, exchanges: exchangesHtml, registers: registersHtml, frames: framesHtml };
  const body = (bodies[analyser.view] ?? unitsHtml)(state);
  return `<div class="split"><section class="listwrap">${strips}${toolbar}${body}</section>${detailHtml(state)}</div>`;
}

export function viewAnalyser(state) {
  const analyser = state.analyser;
  const heard = analyser.heard;
  const running = Boolean(heard?.running);
  const lead = running
    ? t("an.leadRunning", { port: h(heard.port), settings: h(heard.settings), since: heardTime(heard, 0).slice(0, 5) })
    : t("an.lead");
  const start = running ? "" : `<div class="page analyser-page"><div class="cards">${startCard(state)}</div></div>`;
  return {
    title: t("tab.analyser"),
    lead,
    next: running
      ? { act: "analyser-stop", label: t("an.stop") }
      : { act: "analyser-start", label: analyser.busy ? t("an.starting") : t("an.start") },
    html: `${start}${heard ? heardHtml(state) : ""}`,
  };
}

registerView("analyser", viewAnalyser);
