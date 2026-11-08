// The register explorer (concept section 5.3): which addresses exist, what each register could
// mean, and taking what was found into the device's profile.

import { h } from "./html.js";
import { askHtml, unlockHtml } from "./asking.js";
import { codeText, localNumber, noteText, t } from "./i18n.js";
import { deviceName, registerView } from "./store.js";
import { areaShort } from "./workspace.js";

export const EXPLORE_AREAS = ["holding_register", "input_register", "coil", "discrete_input"];
export const ORDERS = ["ABCD", "CDAB", "BADC", "DCBA"];
export const TYPES = ["u16", "s16", "u32", "s32", "u64", "s64", "f32", "f64", "bool"];

const isBits = (area) => area === "coil" || area === "discrete_input";

function formHtml(state) {
  const explorer = state.explorer;
  const devices = state.site.devices.map((device) =>
    `<option value="${h(device.key)}"${device.key === explorer.key ? " selected" : ""}>${h(deviceName(device))} · ${h(device.line)} · ${device.unit}</option>`).join("");
  const areas = EXPLORE_AREAS.map((area) => `<option value="${area}"${area === explorer.area ? " selected" : ""}>${t(`area.${area}`)}</option>`).join("");
  const orders = ORDERS.map((order) => `<option${order === explorer.order ? " selected" : ""}>${order}</option>`).join("");
  return `<div class="toolbar explore-form">
    <label class="form-field">${t("col.device")}<select id="explore-key" data-explorer="key">${devices || `<option value="">${t("ex.noDevice")}</option>`}</select></label>
    <label class="form-field">${t("ex.area")}<select id="explore-area" data-explorer="area">${areas}</select></label>
    <label class="form-field">${t("ex.start")}<input id="explore-start" data-explorer="start" class="mono" inputmode="numeric" value="${h(explorer.start)}"></label>
    <label class="form-field">${t("ex.count")}<input id="explore-count" data-explorer="count" class="mono" inputmode="numeric" value="${h(explorer.count)}"></label>
    ${isBits(explorer.area) ? "" : `<label class="form-field">${t("ex.order")}<select id="explore-order" data-explorer="order">${orders}</select></label>`}
    <div class="end"><button type="button" data-act="explore"${explorer.busy || !explorer.key ? " disabled" : ""}>${explorer.busy ? t("ex.busy") : t("ex.go")}</button></div></div>`;
}

const pair = (row, order) => row.pairs?.find((reading) => reading.order === order);

function rowHtml(state, row) {
  const explorer = state.explorer;
  const active = explorer.row === row.address ? " active" : "";
  if (row.value === null || row.value === undefined) {
    return `<div class="row registers missing" role="row"><span class="mono">${areaShort(explorer.area)} ${row.address}</span><span class="hint">${h(codeText(row.error))}</span></div>`;
  }
  if (isBits(explorer.area)) {
    return `<div class="row registers${active}" data-act="explore-row" data-address="${row.address}" role="option"><span class="mono">${areaShort(explorer.area)} ${row.address}</span><span class="mono">${row.value ? t("on") : t("off")}</span></div>`;
  }
  const reading = pair(row, explorer.order);
  return `<div class="row registers${active}" data-act="explore-row" data-address="${row.address}" role="option">
    <span class="mono">${areaShort(explorer.area)} ${row.address}</span><span class="mono num">${row.value}</span><span class="mono num">${row.signed}</span>
    <span class="mono">${row.hex}</span><span class="mono">${h(row.text ?? "")}</span>
    <span class="mono num${reading && !reading.plausible ? " dim" : ""}">${reading ? h(localNumber(reading.float)) : ""}</span><span class="mono num">${reading ? reading.u32 : ""}</span></div>`;
}

function tableHtml(state) {
  const result = state.explorer.result;
  if (!result) return `<div class="empty"><h2>${t("ex.emptyTitle")}</h2><p>${t("ex.emptyLead")}</p></div>`;
  const bits = isBits(state.explorer.area);
  const head = bits
    ? `<div class="row registers head" role="row"><span>${t("col.address")}</span><span>${t("col.value")}</span></div>`
    : `<div class="row registers head" role="row"><span>${t("col.address")}</span><span class="num">u16</span><span class="num">s16</span><span>hex</span><span>${t("ex.text")}</span><span class="num">f32 ${state.explorer.order}</span><span class="num">u32 ${state.explorer.order}</span></div>`;
  const shown = state.explorer.showMissing ? result.rows : result.rows.filter((row) => row.value !== null && row.value !== undefined);
  const missing = result.rows.length - result.found;
  const strips = [
    `<div class="strip info">${t("ex.found", { found: result.found, missing })} <button type="button" class="quiet" data-act="toggle-missing">${state.explorer.showMissing ? t("ex.hideMissing") : t("ex.showMissing")}</button></div>`,
    ...(result.notes ?? []).map((note) => `<div class="strip info">${h(noteText(note))}${note.code === "word_order" && note.order !== state.explorer.order ? ` <button type="button" class="quiet" data-act="explore-order" data-order="${note.order}">${t("ex.useOrder", { order: note.order })}</button>` : ""}</div>`),
    ...(result.stopped ? [`<div class="strip err">${h(codeText(result.stopped.code))}: ${h(result.stopped.text)}</div>`] : []),
  ].join("");
  return `<div class="strips">${strips}</div><div class="rows" data-keep="registers" role="listbox">${head}${shown.map((row) => rowHtml(state, row)).join("")}</div>`;
}

function detailHtml(state) {
  const explorer = state.explorer;
  const row = explorer.result?.rows.find((item) => item.address === explorer.row);
  if (!row) return `<aside class="detail"><p class="hint">${t("ex.choose")}</p></aside>`;
  const back = `<button type="button" class="quiet back" data-act="explore-close">← ${t("back")}</button>`;
  const pairs = (row.pairs ?? []).map((reading) => `<dt class="mono">${reading.order}</dt><dd class="mono${reading.plausible ? "" : " dim"}">${h(localNumber(reading.float))} · u32 ${reading.u32} · s32 ${reading.s32}</dd>`).join("");
  const adding = explorer.adding;
  const types = TYPES.filter((type) => (isBits(explorer.area) ? type === "bool" : type !== "bool"))
    .map((type) => `<option${type === adding?.type ? " selected" : ""}>${type}</option>`).join("");
  const form = !adding
    ? `<div class="actions"><button type="button" class="tonal" data-act="add-to-profile">${t("ex.addPoint")}</button>${explorer.area === "holding_register" || explorer.area === "coil" ? `<button type="button" class="quiet" data-act="ask-raw-write">${t("ex.write")}</button>` : ""}</div>`
    : `<div class="ask add-point"><b>${t("ex.addPoint")}</b>
      <label class="form-field">${t("col.name")}<input id="add-point-name" data-adding="name" value="${h(adding.name)}"></label>
      <label class="form-field">${t("pt.group")}<input id="add-point-group" data-adding="group" value="${h(adding.group)}"></label>
      <label class="form-field">${t("pt.type")}<select id="add-point-type" data-adding="type">${types}</select></label>
      ${isBits(explorer.area) ? "" : `<label class="form-field">${t("pt.factor")}<input id="add-point-factor" data-adding="factor" class="mono" value="${h(adding.factor)}"></label>
      <label class="form-field">${t("pt.unit")}<input id="add-point-unit" data-adding="unit" value="${h(adding.unit)}"></label>`}
      ${explorer.area === "holding_register" || explorer.area === "coil" ? `<label class="check"><input type="checkbox" data-adding-check="writable"${adding.writable ? " checked" : ""}> ${t("pt.writable")}</label>` : ""}
      <div class="actions"><button type="button" data-act="save-point">${t("ex.savePoint")}</button><button type="button" class="quiet" data-act="cancel-add">${t("cancel")}</button></div></div>`;
  return `<aside class="detail open" data-keep="register-detail">${back}
    <div class="dhead"><b class="mono">${areaShort(explorer.area)} ${row.address}</b><span class="sub mono">${row.hex ?? (row.value ? t("on") : t("off"))}</span></div>
    ${row.pairs?.length ? `<h3>${t("ex.withNext")}</h3><dl class="facts">${pairs}</dl>` : ""}
    ${form}
    ${rawWriteHtml(state, row)}
  </aside>`;
}

/** Writing a register without a profile point: unlock, value, a confirmation naming the address. */
function rawWriteHtml(state, row) {
  const asking = state.asking;
  if (asking?.kind !== "raw-write" || asking.address !== row.address) return "";
  if (!state.unlocked) return unlockHtml();
  const place = `${areaShort(state.explorer.area)} ${row.address}`;
  if (asking.step === "confirm") {
    return askHtml(t("ex.rawConfirm", { value: asking.value, place }), t("ex.rawWhy"), [["raw-write", t("write.do")]]);
  }
  return `<div class="ask" role="group"><b>${t("ex.rawTitle", { place })}</b><span class="hint">${isBits(state.explorer.area) ? t("write.onOff") : t("ex.rawHint")}</span>
    <label class="form-field">${t("write.value")}<input id="write-value" class="mono" value="${h(asking.value ?? "")}" autocomplete="off"></label>
    <div class="actions"><button type="button" data-act="raw-write-check">${t("write.next")}</button><button type="button" class="quiet" data-act="cancel-ask">${t("cancel")}</button></div></div>`;
}

export function viewExplorer(state) {
  return {
    title: t("tab.explorer"),
    lead: t("ex.lead"),
    next: null,
    html: `<div class="split"><section class="listwrap">${formHtml(state)}${tableHtml(state)}</section>${detailHtml(state)}</div>`,
  };
}

registerView("explorer", viewExplorer);
