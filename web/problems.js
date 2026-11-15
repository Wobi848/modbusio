// Everything that needs a look across the devices read so far (after bacnetio's problems.js):
// devices that do not answer, points that could not be read, values you wrote and did not put
// back. A click opens the device, with the point chosen.

import { h } from "./html.js";
import { codeText, counted, localNumber, t } from "./i18n.js";
import { deviceName, registerView } from "./store.js";
import { areaShort } from "./workspace.js";

/** The devices with a profile that were not read yet: nothing is known about their points. */
export const unreadDevices = (state) => state.site.devices.filter((device) => device.profile && !state.readings[device.key]);

/** A device's reading in which every point failed for the same reason: the device, not its points. */
function silentReason(reading) {
  const errors = new Set(reading.points.map((point) => point.reading?.error ?? ""));
  return reading.points.length > 1 && errors.size === 1 && !errors.has("") ? [...errors][0] : null;
}

/** The problems on the site, the worst first: silent devices, failed points, values held. */
export function siteProblems(state) {
  const rows = [];
  for (const device of state.site.devices) {
    const reading = state.readings[device.key];
    if (!reading) continue;
    const silent = silentReason(reading);
    if (silent) {
      rows.push({ device, kind: "silent", severity: 0, text: codeText(silent) });
      continue;
    }
    for (const point of reading.points.filter((one) => one.reading?.error)) {
      rows.push({ device, point: point.name, place: `${areaShort(point.area)} ${point.address}`, kind: "failed", severity: 1, text: codeText(point.reading.error) });
    }
  }
  for (const entry of (state.journal.entries ?? []).filter((one) => one.standing)) {
    const device = state.site.devices.find((one) => one.key === entry.device);
    if (!device) continue;
    rows.push({ device, point: entry.point || null, place: `${areaShort(entry.area)} ${entry.address}`, kind: "held", severity: 2,
      text: `${localNumber(entry.before_text ?? "–")} → ${localNumber(entry.text)}` });
  }
  return rows.sort((first, second) => first.severity - second.severity);
}

const CHIP = { silent: "err", failed: "err", held: "warn" };

function rowHtml({ device, point, place, kind, text }) {
  const open = `data-act="found-open" data-key="${h(device.key)}"${point ? ` data-point="${h(point)}"` : ""}`;
  return `<div class="row mproblems" role="option" ${open}><span class="cut hint">${h(deviceName(device))}</span>
    <span class="name">${h(point ?? (kind === "silent" ? t("prob.device") : place))}</span><span class="mono cut">${h(text)}</span>
    <span class="chips"><span class="chip ${CHIP[kind]}">${t(`prob.${kind}`)}</span></span></div>`;
}

function viewProblems(state) {
  const rows = siteProblems(state);
  const unread = unreadDevices(state).length;
  const reading = Boolean(state.readingAll);
  const withProfile = state.site.devices.some((device) => device.profile);
  const toolbar = `<div class="toolbar"><span class="hint">${unread ? counted("prob.unread", unread) : t("prob.allRead")}</span>
    <div class="end"><button type="button" class="tonal" data-act="read-all"${reading || !withProfile ? " disabled" : ""}>${reading ? t("prob.reading", state.readingAll) : t("prob.readAll")}</button></div></div>`;
  const head = `<div class="row mproblems head" role="row"><span>${t("col.device")}</span><span>${t("col.name")}</span><span>${t("prob.what")}</span><span></span></div>`;
  return {
    title: t("tab.problems"),
    lead: rows.length ? counted("prob.lead", rows.length) : t("prob.leadNone"),
    next: null,
    html: `<div class="split single"><section class="listwrap">${toolbar}<div class="rows" data-keep="problems" role="listbox">${rows.length ? head + rows.map(rowHtml).join("") : `<div class="empty"><h2>${t("prob.noneTitle")}</h2><p>${withProfile ? t("prob.none") : t("prob.noProfile")}</p></div>`}</div></section></div>`,
  };
}

registerView("problems", viewProblems);
