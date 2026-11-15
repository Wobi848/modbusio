// The workspace: the tree of the site (lines and their devices), the table of what is chosen
// (the device list, or a device's points from its profile) and the details beside it.

import { h } from "./html.js";
import { listBorders } from "./panels.js";
import { sortHead, sortRows } from "./sorting.js";
import { codeText, counted, currentLocale, localNumber, noteText, t } from "./i18n.js";
import { askHtml, writeHtml } from "./asking.js";
import { deviceName, deviceNameOnLine, readingText, registerView, selectedDevice } from "./store.js";

const AREA_SHORT = { coil: "coil", discrete_input: "DI", input_register: "IR", holding_register: "HR" };
export const areaShort = (area) => AREA_SHORT[area] ?? area;

/** The devices grouped by their line, in the order of the site. */
/** The wire a device hangs on: the serial port or the TCP address, whatever the settings. */
export function portOf(device) {
  const serial = /^serial:(.+):\d+:\w+\/\d+$/.exec(device.key);
  if (serial) return serial[1];
  const tcp = /^(rtu-tcp|tcp):(.+)\/\d+$/.exec(device.key);
  if (tcp) return tcp[1] === "rtu-tcp" ? `${tcp[2]} (RTU)` : tcp[2];
  return device.line;
}

/** A device's serial settings (`9600 8E1`), or "" off a serial port. */
export const settingsOf = (device) => (device.key.startsWith("serial:") ? device.line.slice(portOf(device).length).trim() : "");

/** The devices grouped by their wire, in the order of the site; `settings` is the setting most of
 * them use, so a device set otherwise stands out. */
export function lines(devices) {
  const groups = new Map();
  for (const device of devices) {
    const port = portOf(device);
    if (!groups.has(port)) groups.set(port, []);
    groups.get(port).push(device);
  }
  return [...groups.entries()].map(([line, members]) => {
    const counts = new Map();
    for (const device of members) counts.set(settingsOf(device), (counts.get(settingsOf(device)) ?? 0) + 1);
    const settings = [...counts.entries()].sort((a, b) => b[1] - a[1])[0][0];
    // one wire carries one setting: devices set differently cannot all be reached by one master
    return { line, settings, devices: members, mixed: counts.size > 1, others: members.filter((device) => settingsOf(device) !== settings).length };
  });
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
  const site = !state.selected && !state.selectedLine;
  const all = `<div class="node level-1${site ? " active" : ""}" role="treeitem" aria-selected="${site}" data-act="select-device" data-key=""><span class="label">${h(state.site.name)}</span><span class="end"><span class="count">${state.site.devices.length}</span></span></div>`;
  const groups = lines(state.site.devices).map((group) => {
    const open = !state.collapsed[group.line];
    const devices = !open ? "" : group.devices.map((device, index) => {
      const active = device.key === state.selected;
      const failed = (state.readings[device.key]?.failed ?? 0) > 0 ? `<span class="chip err">${state.readings[device.key].failed}</span>` : "";
      // set otherwise than most devices on its wire: that is worth seeing at once
      // on a wire set unevenly every device says its setting; the ones off the usual stand out
      const other = !group.mixed ? ""
        : settingsOf(device) !== group.settings ? `<span class="chip warn" title="${t("tree.otherSettings")}">${h(settingsOf(device))}</span>`
          : `<span class="hint mono">${h(settingsOf(device))}</span>`;
      const last = index === group.devices.length - 1 ? " last" : "";
      return `<div class="node level-3 branch${last}${active ? " active" : ""}" role="treeitem" aria-selected="${active}" data-act="select-device" data-key="${h(device.key)}"><span class="label">${h(deviceNameOnLine(device))}</span><span class="end">${other}${failed}<span class="count mono">${device.unit}</span></span></div>`;
    }).join("");
    const chosen = !state.selected && state.selectedLine === group.line;
    const mixed = group.mixed ? `<span class="chip err" title="${t("tree.mixed")}">⚠ ${group.others}</span>` : "";
    const toggle = `<button type="button" class="quiet twisty" data-act="toggle-line" data-line="${h(group.line)}" aria-expanded="${open}" aria-label="${open ? t("tree.close") : t("tree.open")}">${open ? "▾" : "▸"}</button>`;
    return `<div class="node level-2 port${chosen ? " active" : ""}" role="treeitem" aria-expanded="${open}" aria-selected="${chosen}" data-act="select-line" data-line="${h(group.line)}">${toggle}<span class="label mono">${h(group.line)}</span><span class="end">${mixed || (group.settings ? `<span class="hint mono">${h(group.settings)}</span>` : "")}<span class="count">${group.devices.length}</span></span></div><div class="branches" role="group">${devices}</div>`;
  }).join("");
  return `<nav class="tree" role="tree" aria-label="${t("tree.label")}" data-keep="tree">${all}${groups}</nav>`;
}

function notesHtml(notes) {
  if (!notes?.length) return "";
  return `<div class="strips">${notes.map((note) => `<div class="strip info">${h(noteText(note))}</div>`).join("")}</div>`;
}

const DEVICE_COLUMNS = {
  name: (device) => deviceNameOnLine(device),
  line: (device) => device.line,
  unit: (device) => device.unit,
  profile: (device) => device.profile_name,
  vendor: (device) => [device.vendor, device.product].filter(Boolean).join(" "),
};

const POINT_COLUMNS = {
  name: (point) => point.name,
  // by area, then by address as a number
  address: (point) => `${areaShort(point.area)} ${String(point.address).padStart(5, "0")}`,
  value: (point) => point.reading?.value ?? point.reading?.text,
};

/** A strip for each wire in `devices` whose devices are not all set alike, naming each one's setting. */
export function mixedStrips(devices) {
  return lines(devices).filter((group) => group.mixed).map((group) => {
    const list = group.devices.map((device) => `${deviceNameOnLine(device)} ${settingsOf(device)}`).join(" · ");
    return `<div class="strip err">${t("dev.mixedSettings", { line: h(group.line), list: h(list) })}</div>`;
  }).join("");
}

function deviceRow(state, device, wires) {
  const active = device.key === state.selected ? " active" : "";
  const settings = settingsOf(device);
  const off = settings && settings !== wires.get(portOf(device));
  // the wire, then its setting; a setting off the wire's usual one stands out
  const line = settings ? `${h(portOf(device))} <span class="${off ? "set-otherwise" : ""}"${off ? ` title="${t("tree.otherSettings")}"` : ""}>${h(settings)}</span>` : h(device.line);
  const maker = [device.vendor, device.product].filter(Boolean).join(" ");
  return `<div class="row mdevices${active}" data-act="select-device" data-key="${h(device.key)}" role="option" aria-selected="${Boolean(active)}">
    <span class="name">${h(deviceNameOnLine(device))}</span><span class="mono cut">${line}<span class="narrow-only">/${device.unit}</span></span><span class="mono num">${device.unit}</span>
    <span class="cut">${device.profile_name ? h(device.profile_name) : `<span class="chip">${t("dev.noProfile")}</span>`}</span><span class="cut">${h(maker)}</span></div>`;
}

function devicesHtml(state) {
  const line = state.selectedLine;
  const devices = line ? state.site.devices.filter((device) => portOf(device) === line) : state.site.devices;
  const mixed = mixedStrips(devices);
  const strips = `${mixed ? `<div class="strips">${mixed}</div>` : ""}${notesHtml(state.scan.notes)}`;
  const toolbar = `<div class="toolbar"><div class="end">
    <button type="button" class="quiet" data-act="add-device-menu" aria-expanded="${state.menu === "add-device"}">${t("dev.addByHand")}</button>
    <span class="menu-host"><button type="button" id="export-menu" class="quiet" data-act="export-menu" aria-expanded="${state.menu === "export"}">${t("export")}</button>${exportMenu(state)}</span>
    ${line ? `<button type="button" class="quiet" data-act="ask-remove-line">${t("dev.removeLine")}</button>` : ""}</div></div>
    ${state.menu === "add-device" ? addDeviceHtml(state) : ""}
    ${line && state.asking?.kind === "remove-line" ? `<div class="strips">${askHtml(t("dev.removeLineAsk", { line: h(line), n: devices.length }), t("dev.removeLineWhy"), [["remove-line", t("dev.removeLine")]])}</div>` : ""}`;
  if (!devices.length) {
    return `<section class="listwrap">${listBorders()}${toolbar}${strips}<div class="empty"><h2>${t("dev.emptyTitle")}</h2><p>${t("dev.emptyLead")}</p>
      <div class="actions"><button type="button" class="tonal" data-act="add-device-menu">${t("dev.addByHand")}</button></div></div></section>`;
  }
  const sort = state.sorts.devices;
  // each wire's usual setting, from all its devices, also when one line is shown
  const wires = new Map(lines(state.site.devices).map((group) => [group.line, group.settings]));
  const head = `<div class="row mdevices head" role="row">${sortHead(t("col.device"), "devices", "name", sort)}${sortHead(t("col.line"), "devices", "line", sort)}${sortHead(t("col.unit"), "devices", "unit", sort, "num")}${sortHead(t("col.profile"), "devices", "profile", sort)}${sortHead(t("col.vendor"), "devices", "vendor", sort)}</div>`;
  return `<section class="listwrap">${listBorders()}${toolbar}${strips}<div class="rows" data-keep="devices" role="listbox">${head}${sortRows(devices, sort, DEVICE_COLUMNS).map((device) => deviceRow(state, device, wires)).join("")}</div></section>`;
}

function exportMenu(state) {
  if (state.menu !== "export") return "";
  return `<div class="menu end exports" role="menu">
    <a id="export-devices-csv" class="quiet" href="/api/devices.csv" download>${t("export.devicesCsv")}</a>
    <a id="export-site-xlsx" class="quiet" href="/api/site.xlsx" download>${t("export.siteXlsx")}</a>
    <a id="export-site-json" class="quiet" href="/api/site.json" download>${t("export.siteFile")}</a></div>`;
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
      <button type="button" class="quiet" data-act="ask-remove-device">${t("dev.remove")}</button>
    </div>
    ${removing}
    <dl class="facts">${facts}</dl>
    ${device.profile ? `<div class="actions"><a class="button quiet" href="/api/points.csv?key=${encodeURIComponent(device.key)}" download>${t("export.pointsCsv")}</a><a class="button quiet" href="/api/points.xlsx?key=${encodeURIComponent(device.key)}" download>${t("export.pointsXlsx")}</a></div>` : ""}
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
    return `<section class="listwrap">${listBorders()}<div class="empty"><h2>${t("pts.noProfileTitle")}</h2><p>${t("pts.noProfileLead")}</p>
      <div class="actions"><button type="button" data-act="explore-device">${t("dev.explore")}</button><button type="button" class="tonal" data-act="import-for-device">${t("dev.importList")}</button></div></div></section>`;
  }
  if (!reading) {
    return `<section class="listwrap">${listBorders()}<div class="empty"><h2>${t("pts.notReadTitle")}</h2><p>${counted("pts.notReadLead", device.points)}</p>
      <div class="actions"><button type="button" data-act="read-device">${t("dev.read")}</button></div></div></section>`;
  }
  const points = filterPoints(reading.points, state.pointFilter);
  const toolbar = `<div class="toolbar"><button type="button" class="quiet phone-back" data-act="select-device" data-key="">← ${t("tab.workspace")}</button><input id="point-filter" type="search" placeholder="${t("pts.filter")}" value="${h(state.pointFilter)}" autocomplete="off">
    <span class="hint">${t("pts.count", { n: points.length, all: reading.points.length })}${reading.readAt ? ` · ${t("pts.readAt", { time: reading.readAt.toLocaleTimeString(currentLocale(), { hour12: false }) })}` : ""}</span>
    <div class="end"><button type="button" class="quiet narrow-only" data-act="open-device-detail">${t("dev.details")}</button><label class="check"><input type="checkbox" id="live" data-act="toggle-live"${state.live ? " checked" : ""}> ${t("pts.live")}</label>
    <button type="button" class="tonal" data-act="read-device">${t("pts.readAgain")}</button></div></div>`;
  const failed = reading.failed ? `<div class="strip warn">${counted("pts.failed", reading.failed)}</div>` : "";
  const strips = failed || reading.notes?.length ? `<div class="strips">${failed}${(reading.notes ?? []).map((note) => `<div class="strip info">${h(noteText(note))}</div>`).join("")}</div>` : "";
  const sort = state.sorts.points;
  const head = `<div class="row mpoints head" role="row">${sortHead(t("col.name"), "points", "name", sort)}${sortHead(t("col.address"), "points", "address", sort)}${sortHead(t("col.value"), "points", "value", sort, "num")}<span></span></div>`;
  // sorted within each group: the groups are the plant's parts and stay as they are
  const rows = groupPoints(points).map(([group, members]) =>
    `${group ? `<div class="row group" role="presentation"><span class="name">${h(group)}</span><span class="hint">${members.length}</span></div>` : ""}${sortRows(members, sort, POINT_COLUMNS).map((point) => pointRow(state, point)).join("")}`).join("");
  return `<section class="listwrap">${listBorders()}${toolbar}${strips}<div class="rows" data-keep="points" role="listbox">${head}${rows || `<div class="empty"><p>${t("pts.none")}</p></div>`}</div></section>`;
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
    ? [device.name ? `${h(device.line)} · ${t("unit")} ${device.unit}` : "", device.profile_name ? counted("ws.leadProfile", device.points, { profile: h(device.profile_name) }) : t("dev.noProfile")].filter(Boolean).join(" · ")
    : devices === 0 ? t("ws.leadEmpty") : devices === 1 ? t("ws.leadOne") : t("ws.lead", { n: devices });
  const line = !device && state.selectedLine;
  const lineCount = line ? state.site.devices.filter((item) => portOf(item) === line).length : 0;
  return {
    title: device ? deviceName(device) : line || state.site.name || t("tab.workspace"),
    lead: line ? (lineCount === 1 ? t("ws.lineOne") : t("ws.lineMany", { n: lineCount })) : lead,
    next: nextStep(state),
    html: `<div class="workspace">${treeHtml(state)}${table}${detail}</div>`,
  };
}

registerView("workspace", viewWorkspace);
