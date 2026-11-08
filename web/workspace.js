// The workspace: the tree of the site (lines and their devices), the table of what is chosen
// (the device list, or a device's points from its profile) and the details beside it.

import { h } from "./html.js";
import { codeText, localNumber, noteText, t } from "./i18n.js";
import { askHtml, writeHtml } from "./asking.js";
import { deviceName, readingText, registerView, selectedDevice } from "./store.js";

const AREA_SHORT = { coil: "coil", discrete_input: "DI", input_register: "IR", holding_register: "HR" };
export const areaShort = (area) => AREA_SHORT[area] ?? area;

/** The devices grouped by their line, in the order of the site. */
export function lines(devices) {
  const groups = new Map();
  for (const device of devices) {
    if (!groups.has(device.line)) groups.set(device.line, []);
    groups.get(device.line).push(device);
  }
  return [...groups.entries()].map(([line, members]) => ({ line, devices: members }));
}

/** Points grouped by their group, those without one last. */
export function groupPoints(points) {
  const groups = new Map();
  for (const point of points) {
    const key = point.group || "";
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(point);
  }
  return [...groups.entries()].sort(([a], [b]) => (a === "") - (b === "") || a.localeCompare(b));
}

/** Points whose name, group, address or unit holds every word of the filter. */
export function filterPoints(points, filter) {
  const words = filter.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return points;
  return points.filter((point) => {
    const text = `${point.name} ${point.group} ${areaShort(point.area)} ${point.address} ${point.unit} ${point.description}`.toLowerCase();
    return words.every((word) => text.includes(word));
  });
}

function treeHtml(state) {
  const all = `<div class="node level-1${state.selected ? "" : " active"}" role="treeitem" aria-selected="${!state.selected}" data-act="select-device" data-key=""><span class="label">${h(state.site.name)}</span><span class="end"><span class="count">${state.site.devices.length}</span></span></div>`;
  const groups = lines(state.site.devices).map((group) => {
    const devices = group.devices.map((device) => {
      const active = device.key === state.selected;
      const failed = (state.readings[device.key]?.failed ?? 0) > 0 ? `<span class="chip err">${state.readings[device.key].failed}</span>` : "";
      return `<div class="node level-3${active ? " active" : ""}" role="treeitem" aria-selected="${active}" data-act="select-device" data-key="${h(device.key)}"><span class="label">${h(deviceName(device))}</span><span class="end">${failed}<span class="count mono">${device.unit}</span></span></div>`;
    }).join("");
    return `<div class="node level-2 heading" role="presentation"><span class="label mono">${h(group.line)}</span></div>${devices}`;
  }).join("");
  return `<nav class="tree" role="tree" aria-label="${t("tree.label")}" data-keep="tree">${all}${groups}</nav>`;
}

function notesHtml(notes) {
  if (!notes?.length) return "";
  return `<div class="strips">${notes.map((note) => `<div class="strip info">${h(noteText(note))}</div>`).join("")}</div>`;
}

function deviceRow(state, device) {
  const active = device.key === state.selected ? " active" : "";
  const maker = [device.vendor, device.product].filter(Boolean).join(" ");
  return `<div class="row mdevices${active}" data-act="select-device" data-key="${h(device.key)}" role="option" aria-selected="${Boolean(active)}">
    <span class="name">${h(deviceName(device))}</span><span class="mono cut">${h(device.line)}</span><span class="mono num">${device.unit}</span>
    <span class="cut">${device.profile_name ? h(device.profile_name) : `<span class="chip">${t("dev.noProfile")}</span>`}</span><span class="cut">${h(maker)}</span></div>`;
}

function devicesHtml(state) {
  const devices = state.site.devices;
  const strips = notesHtml(state.scan.notes);
  const toolbar = `<div class="toolbar"><div class="end">
    <button type="button" class="quiet" data-act="add-device-menu" aria-expanded="${state.menu === "add-device"}">${t("dev.addByHand")}</button>
    <span class="menu-host"><button type="button" class="quiet" data-act="export-menu" aria-expanded="${state.menu === "export"}">${t("export")}</button>${exportMenu(state)}</span></div></div>
    ${state.menu === "add-device" ? addDeviceHtml(state) : ""}`;
  if (!devices.length) {
    return `<section class="listwrap">${toolbar}${strips}<div class="empty"><h2>${t("dev.emptyTitle")}</h2><p>${t("dev.emptyLead")}</p>
      <div class="actions"><button type="button" class="tonal" data-act="add-device-menu">${t("dev.addByHand")}</button></div></div></section>`;
  }
  const head = `<div class="row mdevices head" role="row"><span>${t("col.device")}</span><span>${t("col.line")}</span><span class="num">${t("col.unit")}</span><span>${t("col.profile")}</span><span>${t("col.vendor")}</span></div>`;
  return `<section class="listwrap">${toolbar}${strips}<div class="rows" data-keep="devices" role="listbox">${head}${devices.map((device) => deviceRow(state, device)).join("")}</div></section>`;
}

function exportMenu(state) {
  if (state.menu !== "export") return "";
  return `<div class="menu end exports" role="menu">
    <a class="quiet" href="/api/devices.csv" download>${t("export.devicesCsv")}</a>
    <a class="quiet" href="/api/site.xlsx" download>${t("export.siteXlsx")}</a>
    <a class="quiet" href="/api/site.json" download>${t("export.siteFile")}</a></div>`;
}

function addDeviceHtml(state) {
  return `<div class="ask add-device"><b>${t("dev.addByHand")}</b><span>${t("dev.addHint")}</span>
    <label class="form-field">${t("dev.key")}<input id="add-key" class="mono" placeholder="tcp:192.168.10.20:502/1" value="${h(state.addKey ?? "")}"></label>
    <label class="form-field">${t("dev.name")}<input id="add-name" value="${h(state.addName ?? "")}"></label>
    <div class="actions"><button type="button" data-act="add-device">${t("dev.add")}</button><button type="button" class="quiet" data-act="close-menu">${t("cancel")}</button></div></div>`;
}

function profileSelect(state, device) {
  const options = state.profiles.map((profile) =>
    `<option value="${h(profile.id)}"${profile.id === device.profile ? " selected" : ""}>${h(profile.name)} (${profile.points})</option>`).join("");
  return `<select id="device-profile" data-device-profile="${h(device.key)}"><option value="">${t("dev.noProfile")}</option>${options}</select>`;
}

function deviceDetailHtml(state) {
  const device = selectedDevice();
  if (!device) return `<aside class="detail"><p class="hint">${t("dev.choose")}</p></aside>`;
  const open = state.deviceDetailOpen ? " open" : "";
  const facts = [
    [t("dev.line"), `<span class="mono">${h(device.line)}</span>`],
    [t("col.unit"), `<span class="mono">${device.unit}</span>`],
    [t("dev.vendor"), h(device.vendor ?? "–")],
    [t("dev.product"), h(device.product ?? "–")],
    [t("dev.revision"), h(device.revision ?? "–")],
    [t("dev.limits"), h(limitsText(device.limits))],
  ].map(([label, value]) => `<dt>${label}</dt><dd>${value}</dd>`).join("");
  const removing = state.asking?.kind === "remove-device"
    ? askHtml(t("dev.removeAsk"), t("dev.removeWhy"), [["remove-device", t("dev.remove")]]) : "";
  return `<aside class="detail${open}" data-keep="device-detail">
    <button type="button" class="quiet back" data-act="close-point">← ${t("back")}</button>
    <div class="dhead"><b>${h(deviceName(device))}</b><span class="sub mono">${h(device.key)}</span></div>
    <label class="form-field">${t("dev.name")}<input id="device-name" data-device-name="${h(device.key)}" value="${h(device.name)}" placeholder="${h(device.product ?? "")}"></label>
    <label class="form-field">${t("col.profile")}${profileSelect(state, device)}</label>
    <div class="actions">
      ${device.profile ? `<button type="button" class="tonal" data-act="read-device">${t("dev.read")}</button>` : ""}
      <button type="button" class="quiet" data-act="explore-device">${t("dev.explore")}</button>
      <button type="button" class="quiet" data-act="import-for-device">${t("dev.importList")}</button>
      <button type="button" class="quiet" data-act="identify">${t("dev.identify")}</button>
    </div>
    <dl class="facts">${facts}</dl>
    ${device.profile ? `<div class="actions"><a class="button quiet" href="/api/points.csv?key=${encodeURIComponent(device.key)}" download>${t("export.pointsCsv")}</a><a class="button quiet" href="/api/points.xlsx?key=${encodeURIComponent(device.key)}" download>${t("export.pointsXlsx")}</a></div>` : ""}
    <div class="actions"><button type="button" class="quiet" data-act="ask-remove-device">${t("dev.remove")}</button></div>
    ${removing}
  </aside>`;
}

/** What modbusio learnt about a device, in words. */
function limitsText(limits) {
  if (!limits) return "–";
  const parts = [t("dev.mostRegisters", { n: limits.most_registers })];
  if (limits.no_holes) parts.push(t("dev.noHoles"));
  if (limits.single_register_write) parts.push(t("dev.singleWrites"));
  return parts.join(" · ");
}

function pointRow(state, point) {
  const active = point.name === state.selectedPoint ? " active" : "";
  const reading = point.reading;
  const failed = reading?.error ? " err" : "";
  const value = reading?.error ? `<span class="chip err" title="${h(codeText(reading.error))}">${h(codeText(reading.error))}</span>` : `<span class="num">${h(localNumber(reading?.text ?? ""))}</span>`;
  const watched = state.watch.some((item) => item.key === state.selected && item.point === point.name);
  return `<div class="row mpoints${active}${failed}" data-act="select-point" data-point="${h(point.name)}" role="option" aria-selected="${Boolean(active)}">
    <span class="name">${h(point.name)}</span><span class="mono cut">${areaShort(point.area)} ${point.address}</span>
    <span class="value">${value}</span><span class="chips">${point.writable ? `<span class="chip">${t("pts.writable")}</span>` : ""}${watched ? `<span class="chip accent">${t("pts.watched")}</span>` : ""}</span></div>`;
}

function pointsHtml(state, device) {
  const reading = state.readings[device.key];
  if (!device.profile) {
    return `<section class="listwrap"><div class="empty"><h2>${t("pts.noProfileTitle")}</h2><p>${t("pts.noProfileLead")}</p>
      <div class="actions"><button type="button" data-act="explore-device">${t("dev.explore")}</button><button type="button" class="tonal" data-act="import-for-device">${t("dev.importList")}</button></div></div></section>`;
  }
  if (!reading) {
    return `<section class="listwrap"><div class="empty"><h2>${t("pts.notReadTitle")}</h2><p>${t("pts.notReadLead", { n: device.points })}</p>
      <div class="actions"><button type="button" data-act="read-device">${t("dev.read")}</button></div></div></section>`;
  }
  const points = filterPoints(reading.points, state.pointFilter);
  const toolbar = `<div class="toolbar"><button type="button" class="quiet phone-back" data-act="select-device" data-key="">← ${t("tab.workspace")}</button><input id="point-filter" type="search" placeholder="${t("pts.filter")}" value="${h(state.pointFilter)}" autocomplete="off">
    <span class="hint">${t("pts.count", { n: points.length, all: reading.points.length })}</span>
    <div class="end"><button type="button" class="quiet narrow-only" data-act="open-device-detail">${t("dev.details")}</button><label class="check"><input type="checkbox" id="live" data-act="toggle-live"${state.live ? " checked" : ""}> ${t("pts.live")}</label>
    <button type="button" class="tonal" data-act="read-device">${t("pts.readAgain")}</button></div></div>`;
  const failed = reading.failed ? `<div class="strip warn">${t("pts.failed", { n: reading.failed })}</div>` : "";
  const strips = failed || reading.notes?.length ? `<div class="strips">${failed}${(reading.notes ?? []).map((note) => `<div class="strip info">${h(noteText(note))}</div>`).join("")}</div>` : "";
  const head = `<div class="row mpoints head" role="row"><span>${t("col.name")}</span><span>${t("col.address")}</span><span>${t("col.value")}</span><span></span></div>`;
  const rows = groupPoints(points).map(([group, members]) =>
    `${group ? `<div class="row group" role="presentation"><span class="name">${h(group)}</span><span class="hint">${members.length}</span></div>` : ""}${members.map((point) => pointRow(state, point)).join("")}`).join("");
  return `<section class="listwrap">${toolbar}${strips}<div class="rows" data-keep="points" role="listbox">${head}${rows || `<div class="empty"><p>${t("pts.none")}</p></div>`}</div></section>`;
}

function pointDetailHtml(state, device) {
  const point = state.readings[device.key]?.points.find((item) => item.name === state.selectedPoint);
  if (!point) return deviceDetailHtml(state);
  const reading = point.reading;
  const states = point.states?.length ? point.states.map(([number, word]) => `${number} = ${h(word)}`).join(", ") : "";
  const facts = [
    [t("col.address"), `<span class="mono">${areaShort(point.area)} ${point.address}${point.count > 1 ? `–${point.address + point.count - 1}` : ""}</span>${point.source_address ? ` <span class="hint">(${h(point.source_address)})</span>` : ""}`],
    [t("pt.type"), `<span class="mono">${h(point.type)}${point.count > 1 && point.type !== "bool" ? ` · ${point.effective_order}` : ""}</span>`],
    [t("pt.scale"), `<span class="mono">× ${localNumber(point.factor)}${point.offset ? ` + ${localNumber(point.offset)}` : ""}${point.scale_register !== undefined ? ` · 10^${areaShort(point.area)} ${point.scale_register}` : ""}</span>`],
    [t("pt.raw"), `<span class="mono">${h((reading?.raw ?? []).join(" ") || "–")}</span>`],
    ...(states ? [[t("pt.states"), states]] : []),
    ...(point.description ? [[t("pt.description"), h(point.description)]] : []),
  ].map(([label, value]) => `<dt>${label}</dt><dd>${value}</dd>`).join("");
  const watched = state.watch.some((item) => item.key === device.key && item.point === point.name);
  const recorded = state.recording.series.some((series) => series.device === device.key && series.point === point.name);
  return `<aside class="detail open" data-keep="point-detail">
    <button type="button" class="quiet back" data-act="close-point">← ${t("back")}</button>
    <div class="dhead"><b>${h(point.name)}</b><span class="sub">${h(point.group || deviceName(device))}</span></div>
    <div class="big-value${reading?.error ? " err" : ""}">${h(localNumber(readingText(reading)))}</div>
    <dl class="facts">${facts}</dl>
    <div class="actions"><button type="button" class="quiet" data-act="toggle-watch">${watched ? t("pt.unwatch") : t("pt.watch")}</button>
      <button type="button" class="quiet" data-act="toggle-record">${recorded ? t("pt.unrecord") : t("pt.record")}</button></div>
    ${point.writable ? writeHtml(state, point) : `<p class="hint">${t("pt.readOnly")}</p>`}
  </aside>`;
}

/** The one next step of the workspace. */
export function nextStep(state) {
  if (!state.scan.finished) return null;
  const device = selectedDevice();
  if (!device) return { act: "search-dialog", label: state.site.devices.length ? t("search.again") : t("search.open") };
  if (!device.profile) return { act: "explore-device", label: t("dev.explore") };
  if (!state.readings[device.key]) return { act: "read-device", label: t("dev.read") };
  return null;
}

export function viewWorkspace(state) {
  const device = selectedDevice();
  const table = device ? pointsHtml(state, device) : devicesHtml(state);
  const detail = device ? pointDetailHtml(state, device) : deviceDetailHtml(state);
  const devices = state.site.devices.length;
  const lead = device
    ? [device.name ? `${h(device.line)} · ${t("unit")} ${device.unit}` : "", device.profile_name ? t("ws.leadProfile", { profile: h(device.profile_name), n: device.points }) : t("dev.noProfile")].filter(Boolean).join(" · ")
    : devices === 0 ? t("ws.leadEmpty") : devices === 1 ? t("ws.leadOne") : t("ws.lead", { n: devices });
  return {
    title: device ? deviceName(device) : state.site.name || t("tab.workspace"),
    lead,
    next: nextStep(state),
    html: `<div class="workspace">${treeHtml(state)}${table}${detail}</div>`,
  };
}

registerView("workspace", viewWorkspace);
