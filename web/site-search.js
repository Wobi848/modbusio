// The search over the whole site, from the sidebar (after bacnetio's site-search.js): devices by
// name, line, unit and maker, points of the devices read so far by name, group, address and
// description. A click opens what was found.

import { h } from "./html.js";
import { codeText, localNumber, t } from "./i18n.js";
import { deviceName, registerView } from "./store.js";
import { areaShort } from "./workspace.js";

/** Most rows on the page: the search is for finding one thing, not for listing a whole site. */
const MOST_RESULTS = 200;

const matches = (words, parts) => {
  const text = parts.filter(Boolean).join(" ").toLowerCase();
  return words.every((word) => text.includes(word));
};

/** The devices and the points of the site that carry every word of the search. */
export function searchSite(state) {
  const words = state.siteSearch.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return { devices: [], points: [] };
  const devices = state.site.devices.filter((device) =>
    matches(words, [device.name, device.line, device.key, device.vendor, device.product, device.profile_name]));
  const points = [];
  for (const device of state.site.devices) {
    for (const point of state.readings[device.key]?.points ?? []) {
      if (matches(words, [point.name, point.group, `${areaShort(point.area)} ${point.address}`, point.unit, point.description])) points.push({ device, point });
    }
  }
  return { devices, points };
}

function viewSearch(state) {
  const { devices, points } = searchSite(state);
  const count = devices.length + points.length;
  const deviceRows = devices.map((device) => `<div class="row found" role="option" data-act="found-open" data-key="${h(device.key)}"><span class="name">${h(deviceName(device))}</span>
    <span class="cut hint">${t("col.device")}${device.profile_name ? ` · ${h(device.profile_name)}` : ""}</span><span class="mono cut">${h(device.key)}</span></div>`);
  const pointRows = points.map(({ device, point }) => {
    const reading = point.reading;
    const value = reading?.error ? codeText(reading.error) : localNumber(reading?.text ?? "");
    return `<div class="row found" role="option" data-act="found-open" data-key="${h(device.key)}" data-point="${h(point.name)}"><span class="name">${h(point.name)}</span>
      <span class="cut hint">${h(deviceName(device))} · ${areaShort(point.area)} ${point.address}</span><span class="mono cut">${h(value)}</span></div>`;
  });
  const rows = [...deviceRows, ...pointRows].slice(0, MOST_RESULTS).join("");
  const unread = state.site.devices.filter((device) => device.profile && !state.readings[device.key]).length;
  const hint = unread ? `<div class="toolbar"><span class="hint">${t("found.unread", { n: unread })}</span></div>` : "";
  return {
    title: t("found.title"),
    lead: count > MOST_RESULTS ? t("found.leadMost", { n: count, most: MOST_RESULTS }) : t("found.lead", { n: count }),
    next: null,
    html: `<div class="split single"><section class="listwrap">${hint}<div class="rows" data-keep="found" role="listbox">${rows || `<div class="empty"><p>${t("found.none")}</p></div>`}</div></section></div>`,
  };
}

registerView("search", viewSearch);
