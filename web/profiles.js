// Profiles (concept section 6): the list, the editor of one profile and its points, and the
// import of a vendor's register list with a preview that reads the first points from a device.

import { h } from "./html.js";
import { askHtml } from "./asking.js";
import { codeText, localNumber, t } from "./i18n.js";
import { ORDERS, TYPES } from "./explorer.js";
import { deviceName, registerView } from "./store.js";
import { areaShort } from "./workspace.js";

const AREAS = ["holding_register", "input_register", "coil", "discrete_input"];
const ROLES = ["name", "group", "area", "address", "type", "count", "unit", "factor", "offset", "scale_register", "description", "access", "states", "order", "min", "max"];
const CONVENTIONS = ["zero_based", "one_based", "reference"];

/** A point's states as one line: `0=off; 1=on`. */
export const statesText = (states) => (states ?? []).map(([number, word]) => `${number}=${word}`).join("; ");

/** The states a line names; parts that are no `number=word` are left out. */
export function parseStates(text) {
  return text.split(/[;\n]/).map((part) => /^\s*(-?\d+)\s*[=:]\s*(.+?)\s*$/.exec(part)).filter(Boolean).map((match) => [Number(match[1]), match[2]]);
}

function listHtml(state) {
  const rows = state.profiles.map((profile) => {
    const active = profile.id === state.profileId ? " active" : "";
    return `<div class="row mprofiles${active}" data-act="open-profile" data-id="${h(profile.id)}" role="option"><span class="name">${h(profile.name)}</span><span class="cut">${h([profile.vendor, profile.model].filter(Boolean).join(" "))}</span><span class="mono num">${profile.points}</span><span class="num">${profile.used_by ? t("prof.usedBy", { n: profile.used_by }) : ""}</span></div>`;
  }).join("");
  const toolbar = `<div class="toolbar"><div class="end"><button type="button" class="tonal" data-act="new-profile">${t("prof.new")}</button><button type="button" class="quiet" data-act="import-list">${t("prof.import")}</button></div></div>`;
  if (!state.profiles.length) {
    return `<section class="listwrap">${toolbar}<div class="empty"><h2>${t("prof.emptyTitle")}</h2><p>${t("prof.emptyLead")}</p></div></section>`;
  }
  const head = `<div class="row mprofiles head"><span>${t("col.name")}</span><span>${t("col.vendor")}</span><span class="num">${t("prof.points")}</span><span></span></div>`;
  return `<section class="listwrap">${toolbar}<div class="rows" data-keep="profiles" role="listbox">${head}${rows}</div></section>`;
}

const select = (id, data, options, chosen) => `<select id="${id}" ${data}>${options.map(([value, label]) => `<option value="${h(value)}"${String(value) === String(chosen ?? "") ? " selected" : ""}>${h(label)}</option>`).join("")}</select>`;

function editorHtml(state) {
  const draft = state.profileDraft;
  const head = `<div class="toolbar profile-head">
    <label class="form-field">${t("col.name")}<input id="profile-name" data-profile="name" value="${h(draft.name)}"></label>
    <label class="form-field">${t("col.vendor")}<input id="profile-vendor" data-profile="vendor" value="${h(draft.vendor)}"></label>
    <label class="form-field">${t("prof.model")}<input id="profile-model" data-profile="model" value="${h(draft.model)}"></label>
    <label class="form-field">${t("prof.order")}${select("profile-order", 'data-profile="order"', ORDERS.map((order) => [order, order]), draft.order)}</label>
    <div class="end"><button type="button" data-act="save-profile"${state.profileDirty ? "" : " disabled"}>${t("prof.save")}</button><button type="button" class="quiet" data-act="close-profile">${t("prof.close")}</button></div></div>`;
  const rows = draft.points.map((point, index) => {
    const active = index === state.pointIndex ? " active" : "";
    return `<div class="row mprofile-points${active}" data-act="edit-point" data-index="${index}" role="option"><span class="name">${h(point.name)}</span><span class="cut">${h(point.group)}</span><span class="mono">${areaShort(point.area)} ${point.address}</span><span class="mono">${h(point.type)}</span><span class="mono num">${point.factor !== 1 ? `× ${localNumber(point.factor)}` : ""}</span><span>${h(point.unit)}</span><span>${point.writable ? `<span class="chip">${t("pts.writable")}</span>` : ""}</span></div>`;
  }).join("");
  const tableHead = `<div class="row mprofile-points head"><span>${t("col.name")}</span><span>${t("pt.group")}</span><span>${t("col.address")}</span><span>${t("pt.type")}</span><span class="num">${t("pt.factor")}</span><span>${t("pt.unit")}</span><span></span></div>`;
  const tools = `<div class="toolbar"><span class="hint">${t("prof.pointCount", { n: draft.points.length })}${draft.source ? ` · ${t("prof.source", { source: h(draft.source) })}` : ""}</span><div class="end">
    <button type="button" class="tonal" data-act="add-profile-point">${t("prof.addPoint")}</button>
    ${draft.id ? `<a class="button quiet" href="/api/profile.csv?id=${encodeURIComponent(draft.id)}" download>CSV</a><a class="button quiet" href="/api/profile.xlsx?id=${encodeURIComponent(draft.id)}" download>Excel</a>` : ""}
    ${draft.id ? `<button type="button" class="quiet" data-act="ask-delete-profile">${t("prof.delete")}</button>` : ""}</div></div>`;
  const deleting = state.asking?.kind === "delete-profile" ? askHtml(t("prof.deleteAsk", { name: draft.name }), t("prof.deleteWhy"), [["delete-profile", t("prof.delete")]]) : "";
  return `<section class="listwrap">${head}${tools}${deleting}<div class="rows" data-keep="profile-points" role="listbox">${tableHead}${rows || `<div class="empty"><p>${t("prof.noPoints")}</p></div>`}</div></section>`;
}

function pointFormHtml(state) {
  const point = state.profileDraft?.points[state.pointIndex];
  if (!point) return `<aside class="detail"><p class="hint">${state.profileDraft ? t("prof.choosePoint") : t("prof.choose")}</p></aside>`;
  const text = (key, label, mono = false) => `<label class="form-field">${label}<input id="point-${key}" data-point-field="${key}" class="${mono ? "mono" : ""}" value="${h(point[key] ?? "")}"></label>`;
  const types = TYPES.map((type) => [type, type]).concat([["text20", "text20"]]);
  if (!types.some(([type]) => type === point.type)) types.push([point.type, point.type]);
  return `<aside class="detail open" data-keep="point-form"><button type="button" class="quiet back" data-act="close-point-form">← ${t("back")}</button>
    <div class="dhead"><b>${h(point.name || t("prof.newPoint"))}</b></div>
    ${text("name", t("col.name"))}${text("group", t("pt.group"))}
    <label class="form-field">${t("ex.area")}${select("point-area", 'data-point-field="area"', AREAS.map((area) => [area, t(`area.${area}`)]), point.area)}</label>
    ${text("address", t("prof.address"), true)}
    <label class="form-field">${t("pt.type")}${select("point-type", 'data-point-field="type"', types, point.type)}</label>
    <label class="form-field">${t("ex.order")}${select("point-order", 'data-point-field="order"', [["", t("prof.orderDefault")], ...ORDERS.map((order) => [order, order])], point.order)}</label>
    ${text("factor", t("pt.factor"), true)}${text("offset", t("pt.offset"), true)}${text("scale_register", t("pt.scaleRegister"), true)}
    ${text("unit", t("pt.unit"))}${text("description", t("pt.description"))}
    <label class="form-field">${t("pt.states")}<input id="point-states" data-point-field="states" value="${h(statesText(point.states))}" placeholder="0=off; 1=on"></label>
    <label class="check"><input type="checkbox" data-point-check="writable"${point.writable ? " checked" : ""}> ${t("pt.writable")}</label>
    ${text("min", t("pt.min"), true)}${text("max", t("pt.max"), true)}
    <div class="actions"><button type="button" class="quiet" data-act="duplicate-point">${t("prof.duplicate")}</button><button type="button" class="quiet" data-act="remove-point">${t("prof.removePoint")}</button></div>
  </aside>`;
}

/** The preview of an imported list: columns, convention, points with values from the device. */
export function importHtml(state) {
  const importing = state.importing;
  if (!importing) return "";
  const result = importing.result;
  const imported = result?.imported;
  const devices = [["", t("imp.noDevice")], ...state.site.devices.map((device) => [device.key, `${deviceName(device)} · ${device.line} · ${device.unit}`])];
  const headers = imported ? [["", "–"], ...imported.headers.map((header, index) => [index, header || `#${index + 1}`])] : [];
  const columns = imported ? ROLES.map((role) => `<label class="form-field">${t(`role.${role}`)}${select(`role-${role}`, `data-role="${role}"`, headers, imported.columns[role])}</label>`).join("") : "";
  const rows = (imported?.points ?? []).slice(0, 40).map((point, index) => {
    const reading = result.preview?.[index];
    const value = reading ? (reading.error ? `<span class="chip err">${h(codeText(reading.error))}</span>` : h(localNumber(reading.text))) : "";
    return `<div class="row mimport"><span class="name">${h(point.name)}</span><span class="mono">${h(point.source_address)}</span><span class="mono">${areaShort(point.area)} ${point.address}</span><span class="mono">${h(point.type)}</span><span>${h(point.unit)}</span><span class="value">${value}</span></div>`;
  }).join("");
  const skipped = (imported?.skipped ?? []).slice(0, 10).map(([row, reason]) => `<li>${t("imp.row", { n: row })}: ${h(reason)}</li>`).join("");
  return `<div class="modal"><div class="modal-box import-box" role="dialog" aria-modal="true" aria-labelledby="import-title">
    <h2 id="import-title">${t("imp.title", { file: h(importing.fileName) })}</h2>
    <p class="sub">${t("imp.lead")}</p>
    <div class="import-settings">
      <label class="form-field">${t("imp.device")}${select("import-key", 'data-import="key"', devices, importing.key)}</label>
      <label class="form-field">${t("imp.convention")}${select("import-convention", 'data-import="convention"', CONVENTIONS.map((convention) => [convention, t(`conv.${convention}`)]), imported?.convention)}</label>
      <label class="form-field">${t("imp.area")}${select("import-area", 'data-import="area"', [["", "–"], ...AREAS.map((area) => [area, t(`area.${area}`)])], importing.options.area)}</label>
      <label class="form-field">${t("prof.order")}${select("import-order", 'data-import="order"', ORDERS.map((order) => [order, order]), importing.order)}</label>
    </div>
    ${imported ? `<p class="hint">${t("imp.found", { n: imported.points.length, skipped: imported.skipped.length, reason: h(imported.convention_reason) })}</p>` : ""}
    <details><summary>${t("imp.columns")}</summary><div class="import-columns">${columns}</div></details>
    ${rows ? `<div class="rows import-rows"><div class="row mimport head"><span>${t("col.name")}</span><span>${t("imp.inList")}</span><span>${t("imp.onWire")}</span><span>${t("pt.type")}</span><span>${t("pt.unit")}</span><span>${t("imp.value")}</span></div>${rows}</div>` : ""}
    ${skipped ? `<details><summary>${t("imp.skipped", { n: imported.skipped.length })}</summary><ul class="plain">${skipped}</ul></details>` : ""}
    <label class="form-field">${t("imp.name")}<input id="import-name" data-import="name" value="${h(importing.name)}"></label>
    <div class="acts"><button type="button" class="quiet" data-act="close-import">${t("cancel")}</button><button type="button" class="tonal" data-act="import-again">${t("imp.again")}</button><button type="button" data-act="import-save"${imported?.points.length ? "" : " disabled"}>${t("imp.save")}</button></div>
  </div></div>`;
}

export function viewProfiles(state) {
  const main = state.profileDraft ? editorHtml(state) : listHtml(state);
  return {
    title: state.profileDraft ? state.profileDraft.name || t("prof.newTitle") : t("tab.profiles"),
    lead: state.profileDraft ? t("prof.leadEdit") : t("prof.lead"),
    next: state.profileDraft ? (state.profileDirty ? { act: "save-profile", label: t("prof.save") } : null) : { act: "import-list", label: t("prof.import") },
    html: `<div class="split">${main}${pointFormHtml(state)}</div>`,
  };
}

registerView("profiles", viewProfiles);
