// The open device's points as a page to print or save as PDF (after bacnetio's print.js): the
// groups and the search of the list, without modbusio's own controls around them.

import { h } from "./html.js";
import { codeText, currentLocale, localNumber, t } from "./i18n.js";
import { deviceName, selectedDevice, wrapView } from "./store.js";
import { areaShort, filterPoints, groupPoints } from "./workspace.js";

function factsHtml(state, device, reading, shown) {
  const filter = state.pointFilter.trim();
  const all = reading.points.length;
  const count = filter ? t("print.some", { what: h(`"${filter}"`), shown, n: all }) : all === 1 ? t("print.one") : t("print.all", { n: all });
  const maker = [device.vendor, device.product].filter(Boolean).join(" ");
  const facts = [
    [t("side.site"), h(state.site.name)],
    [t("col.device"), h(deviceName(device))],
    [t("col.address"), `<span class="mono">${h(device.key)}</span>`],
    device.profile_name ? [t("col.profile"), h(device.profile_name)] : null,
    maker ? [t("col.vendor"), h(maker)] : null,
    reading.readAt ? [t("print.read"), h(reading.readAt.toLocaleString(currentLocale()))] : null,
    [t("print.shown"), count],
    [t("print.made"), `${h(new Date().toLocaleString(currentLocale()))} · modbusio ${h(state.info?.version ?? "")}`],
  ].filter(Boolean);
  return `<dl class="print-facts">${facts.map(([term, value]) => `<dt>${term}</dt><dd>${value}</dd>`).join("")}</dl>`;
}

function rowHtml(point) {
  const reading = point.reading;
  const value = reading?.error ? "" : h(localNumber(reading?.text ?? ""));
  const notes = [reading?.error ? `<span class="chip err">${h(codeText(reading.error))}</span>` : "", point.writable ? `<span class="chip">${t("pts.writable")}</span>` : ""].join("");
  return `<tr><td class="mono">${areaShort(point.area)} ${point.address}</td><td>${h(point.name)}</td>
    <td>${h(point.description ?? "")}</td><td class="mono">${value}</td><td>${notes}</td></tr>`;
}

function tableHtml(points) {
  const head = `<thead><tr><th>${t("col.address")}</th><th>${t("col.name")}</th><th>${t("pt.description")}</th><th>${t("col.value")}</th><th></th></tr></thead>`;
  const bodies = groupPoints(points).map(([group, members]) =>
    `<tbody>${group ? `<tr class="print-group"><th colspan="5">${h(group)} <span class="sub">${members.length}</span></th></tr>` : ""}${members.map(rowHtml).join("")}</tbody>`).join("");
  return `<table class="print-table">${head}${bodies}</table>`;
}

function viewPrint(state, device, reading) {
  const points = filterPoints(reading.points, state.pointFilter);
  return {
    title: deviceName(device),
    printing: true,
    next: null,
    html: `<article class="print-page">
      <div class="print-acts"><button type="button" data-act="print-now">${t("print.now")}</button>
        <button type="button" class="quiet" data-act="print-close">${t("print.back")}</button>
        <span class="hint">${t("print.hint")}</span></div>
      <h1>${h(deviceName(device))}</h1>
      ${factsHtml(state, device, reading, points.length)}
      <div class="print-scroll">${points.length ? tableHtml(points) : `<p>${t("pts.none")}</p>`}</div>
    </article>`,
  };
}

/** Whether there is something to print: an open device whose points were read. */
export function printable(state) {
  const device = selectedDevice();
  return Boolean(state.area === "workspace" && device && state.readings[device.key]);
}

wrapView("workspace", (view) => (state) => {
  const device = selectedDevice();
  const reading = device && state.readings[device.key];
  return state.printing && reading ? viewPrint(state, device, reading) : view(state);
});
