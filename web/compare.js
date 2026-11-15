// Two devices with the same profile side by side, their points matched by name (after bacnetio's
// compare.js): what differs between two of a kind shows at a glance.

import { h } from "./html.js";
import { codeText, localNumber, t } from "./i18n.js";
import { deviceName, selectedDevice, wrapView } from "./store.js";
import { areaShort, groupPoints } from "./workspace.js";

const valueOf = (point) => (point?.reading?.error ? codeText(point.reading.error) : localNumber(point?.reading?.text ?? ""));

/** One row per point of the first device, with the second's point of the same name. */
export function compareRows(first, second) {
  const others = new Map((second?.points ?? []).map((point) => [point.name, point]));
  return (first?.points ?? []).map((point) => {
    const other = others.get(point.name);
    const [here, there] = [valueOf(point), valueOf(other)];
    return { point, here, there, differs: here !== there };
  });
}

function rowHtml({ point, here, there, differs }) {
  return `<div class="row mcompare${differs ? " differs" : ""}"><span class="name">${h(point.name)}</span><span class="mono cut">${areaShort(point.area)} ${point.address}</span>
    <span class="mono num">${h(here) || "–"}</span><span class="mono num">${h(there) || "–"}</span><span>${differs ? `<span class="chip warn">${t("compare.differs")}</span>` : ""}</span></div>`;
}

function viewCompare(state, device, other) {
  const compare = state.compare;
  const rows = compareRows(state.readings[device.key], state.readings[other.key]);
  const different = rows.filter((row) => row.differs);
  const shown = compare.onlyDifferent ? different : rows;
  const views = [["all", t("compare.all"), rows.length, false], ["different", t("compare.different"), different.length, true]].map(([view, label, count, only]) =>
    `<button type="button" id="compare-${view}" data-act="compare-view" data-view="${view}" aria-pressed="${compare.onlyDifferent === only}">${label} <span class="n">${count}</span></button>`).join("");
  const toolbar = `<div class="toolbar"><span class="seg" role="group" aria-label="${t("compare.show")}">${views}</span>
    <div class="end"><button type="button" class="tonal" data-act="compare-read"${compare.reading ? " disabled" : ""}>${t(compare.reading ? "compare.reading" : "compare.read")}</button>
    <button type="button" class="quiet" data-act="compare-close">${t("compare.close")}</button></div></div>`;
  const head = `<div class="row mcompare head" role="row"><span>${t("col.name")}</span><span>${t("col.address")}</span><span class="num">${h(deviceName(device))}</span><span class="num">${h(deviceName(other))}</span><span></span></div>`;
  const shownPoints = new Map(shown.map((row) => [row.point, row]));
  const body = groupPoints(shown.map((row) => row.point)).map(([group, members]) =>
    `${group ? `<div class="row group" role="presentation"><span class="name">${h(group)}</span><span class="hint">${members.length}</span></div>` : ""}${members.map((point) => rowHtml(shownPoints.get(point))).join("")}`).join("");
  const empty = compare.reading ? t("compare.reading") : rows.length ? t("compare.same") : t("compare.notRead");
  return {
    title: t("compare.title", { first: deviceName(device), second: deviceName(other) }),
    lead: t("compare.lead", { profile: h(device.profile_name ?? ""), n: different.length }),
    next: null,
    html: `<div class="split single"><section class="listwrap">${toolbar}<div class="rows" data-keep="compare">${head}${body || `<div class="empty"><p>${empty}</p></div>`}</div></section></div>`,
  };
}

wrapView("workspace", (view) => (state) => {
  const device = selectedDevice();
  const other = state.compare && state.site.devices.find((one) => one.key === state.compare.other);
  return device && other ? viewCompare(state, device, other) : view(state);
});
