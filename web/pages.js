// The smaller pages: the watch list, the recording with its charts, the values changed by you,
// and the settings.

import { h } from "./html.js";
import { askHtml } from "./asking.js";
import { LANGUAGES, codeText, currentLocale, languageChoice, localNumber, t } from "./i18n.js";
import { chartHtml } from "./chart.js";
import { updateCheckOn } from "./update.js";
import { deviceName, readingText, registerView } from "./store.js";
import { areaShort } from "./workspace.js";
import { sortHead, sortRows } from "./sorting.js";

const deviceOf = (state, key) => state.site.devices.find((device) => device.key === key);
const timeText = (seconds) => new Date(seconds * 1000).toLocaleString(currentLocale(), { dateStyle: "short", timeStyle: "medium" });

function viewWatch(state) {
  const sort = state.sorts.watch;
  const columns = {
    name: (item) => item.point,
    device: (item) => {
      const device = deviceOf(state, item.key);
      return device ? deviceName(device) : item.key;
    },
    value: (item) => state.watchValues[`${item.key}|${item.point}`]?.reading?.value,
  };
  const rows = sortRows(state.watch, sort, columns).map((item) => {
    const device = deviceOf(state, item.key);
    const answer = state.watchValues[`${item.key}|${item.point}`];
    const value = answer?.error ? `<span class="chip err">${h(answer.error)}</span>`
      : answer?.reading?.error ? `<span class="chip err">${h(codeText(answer.reading.error))}</span>`
        : `<span class="num">${h(localNumber(readingText(answer?.reading)))}</span>`;
    return `<div class="row mwatch"><span class="name">${h(item.point)}<span class="sub narrow-only">${h(device ? deviceName(device) : item.key)}</span></span><span class="cut">${h(device ? deviceName(device) : item.key)}</span><span class="value">${value}</span>
      <button type="button" class="quiet" data-act="unwatch" data-key="${h(item.key)}" data-point="${h(item.point)}">${t("watch.remove")}</button></div>`;
  }).join("");
  const body = rows
    ? `<div class="rows" data-keep="watch"><div class="row mwatch head">${sortHead(t("col.name"), "watch", "name", sort)}${sortHead(t("col.device"), "watch", "device", sort)}${sortHead(t("col.value"), "watch", "value", sort, "num")}<span></span></div>${rows}</div>`
    : `<div class="empty"><h2>${t("watch.emptyTitle")}</h2><p>${t("watch.emptyLead")}</p></div>`;
  return {
    title: t("tab.watch"),
    lead: t("watch.lead"),
    next: null,
    html: `<div class="split single"><section class="listwrap">${body}</section></div>`,
  };
}

function viewRecording(state) {
  const recording = state.recording;
  const intervals = [5, 10, 15, 30, 60].map((seconds) => `<option value="${seconds}"${seconds === recording.interval ? " selected" : ""}>${seconds} s</option>`).join("");
  const series = recording.series.map((item) => {
    const last = item.samples.findLast(([, value]) => value !== null);
    return `<div class="row mrecord"><span class="name">${h(item.label)}</span><span class="num mono">${last ? h(localNumber(String(last[1]))) : "–"} ${h(item.unit)}</span><span class="mono num">${item.samples.length}</span>
      <button type="button" class="quiet" data-act="record-remove" data-key="${h(item.device)}" data-point="${h(item.point)}">${t("rec.remove")}</button></div>`;
  }).join("");
  const toolbar = `<div class="toolbar"><label class="form-field inline">${t("rec.interval")}<select id="record-interval"${recording.running ? " disabled" : ""}>${intervals}</select></label>
    <div class="end">${recording.running ? `<button type="button" class="tonal" data-act="record-stop">${t("rec.stop")}</button>` : `<button type="button" class="tonal" data-act="record-start"${recording.series.length ? "" : " disabled"}>${t("rec.start")}</button>`}
    <button type="button" class="quiet" data-act="record-clear">${t("rec.clear")}</button>
    <a class="button quiet" href="/api/recording.csv?offset=${-new Date().getTimezoneOffset()}" download>CSV</a><a class="button quiet" href="/api/recording.xlsx?offset=${-new Date().getTimezoneOffset()}" download>Excel</a></div></div>`;
  const body = recording.series.length
    ? `${chartHtml(recording.series)}<div class="rows" data-keep="recording"><div class="row mrecord head"><span>${t("col.name")}</span><span class="num">${t("rec.last")}</span><span class="num">${t("rec.samples")}</span><span></span></div>${series}</div>`
    : `<div class="empty"><h2>${t("rec.emptyTitle")}</h2><p>${t("rec.emptyLead")}</p></div>`;
  return {
    title: t("tab.recording"),
    lead: recording.running ? t("rec.leadRunning", { n: recording.interval }) : t("rec.lead"),
    next: recording.series.length && !recording.running ? { act: "record-start", label: t("rec.start") } : null,
    html: `<div class="split single"><section class="listwrap">${toolbar}${body}</section></div>`,
  };
}

function entryRow(state, entry) {
  const device = deviceOf(state, entry.device);
  const place = `${entry.point ? `${h(entry.point)} · ` : ""}<span class="mono whole">${areaShort(entry.area)} ${entry.address}</span>`;
  const outcome = entry.error ? `<span class="chip err" title="${h(entry.error)}">${t("log.refused")}</span>`
    : entry.kept_other ? `<span class="chip warn">${t("log.keptOther")}</span>`
      : entry.restores ? `<span class="chip">${t("log.restores", { n: entry.restores })}</span>`
        : entry.standing ? `<span class="chip warn">${t("log.standing")}</span>` : "";
  const restore = entry.standing ? `<button type="button" class="quiet" data-act="ask-restore" data-id="${entry.id}">${t("log.restore")}</button>` : "";
  return `<div class="row mlog"><span class="mono num">${entry.id}</span><span class="cut">${timeText(entry.time)}</span><span class="cut where">${h(device ? deviceName(device) : entry.device)} · ${place}</span>
    <span class="cut">${h(localNumber(entry.before_text ?? "–"))} → <b>${h(localNumber(entry.text))}</b></span><span class="chips">${outcome}${restore}</span></div>`;
}

/** What putting `entry` back writes and where, in words: the confirmation names it. */
function restoreText(state, entry) {
  const device = deviceOf(state, entry.device);
  const place = [device ? deviceName(device) : entry.device, entry.point, `${areaShort(entry.area)} ${entry.address}`].filter(Boolean).join(" · ");
  return h(t("log.restoreWhat", { value: localNumber(entry.original_text ?? "–"), place }));
}

function viewChanged(state) {
  const journal = state.journal;
  const standing = journal.entries.filter((entry) => entry.standing);
  const asked = state.asking?.kind === "restore" ? standing.filter((entry) => state.asking.all || entry.id === state.asking.id) : [];
  const asking = state.asking?.kind === "restore"
    ? askHtml(state.asking.all ? (standing.length === 1 ? t("log.restoreOneAsk") : t("log.restoreAllAsk", { n: standing.length })) : t("log.restoreAsk", { n: state.asking.id }),
      [...asked.map((entry) => restoreText(state, entry)), t("log.restoreWhy")].join("<br>"), [["restore", t("log.restore")]])
    : "";
  const unlock = state.asking?.kind === "restore" && !state.unlocked ? askHtml(t("unlock.title"), t("unlock.why"), [["unlock", t("unlock.allow")]]) : "";
  const toolbar = `<div class="toolbar"><span class="hint">${journal.entries.length === 1 ? t("log.countOne") : t("log.count", { n: journal.entries.length })}</span><div class="end">
    <a class="button quiet" href="/api/journal.csv?offset=${-new Date().getTimezoneOffset()}" download>CSV</a><a class="button quiet" href="/api/journal.xlsx?offset=${-new Date().getTimezoneOffset()}" download>Excel</a></div></div>`;
  const entries = [...journal.entries].reverse();
  const body = entries.length
    ? `<div class="rows" data-keep="log"><div class="row mlog head"><span class="num">${t("log.number")}</span><span>${t("log.time")}</span><span>${t("log.where")}</span><span>${t("log.change")}</span><span></span></div>${entries.map((entry) => entryRow(state, entry)).join("")}</div>`
    : `<div class="empty"><h2>${t("log.emptyTitle")}</h2><p>${t("log.emptyLead")}</p></div>`;
  return {
    title: t("tab.changed"),
    lead: standing.length === 1 ? t("log.leadStandingOne") : standing.length ? t("log.leadStanding", { n: standing.length }) : t("log.lead"),
    next: standing.length ? { act: "ask-restore-all", label: standing.length === 1 ? t("log.restoreOne") : t("log.restoreAll", { n: standing.length }) } : null,
    html: `<div class="split single"><section class="listwrap">${toolbar}${unlock || asking ? `<div class="strips">${unlock || asking}</div>` : ""}${body}</section></div>`,
  };
}

function sitesCard(state) {
  const rows = state.sites.map((site) => `<li class="site-line"><span class="cut">${h(site.name)}${site.open ? ` <span class="chip accent">${t("set.open")}</span>` : ""}</span><span class="hint">${t("set.devices", { n: site.devices })}</span>
    ${site.open ? "" : `<button type="button" class="quiet" data-act="site-open" data-id="${h(site.id)}">${t("set.openSite")}</button><button type="button" class="quiet" data-act="ask-delete-site" data-id="${h(site.id)}">${t("set.deleteSite")}</button>`}</li>`).join("");
  const deleting = state.asking?.kind === "delete-site" ? askHtml(t("set.deleteAsk"), t("set.deleteWhy"), [["delete-site", t("set.deleteSite")]]) : "";
  return `<section class="card"><h2>${t("set.sites")}</h2>
    <label class="form-field">${t("set.siteName")}<input id="site-name" value="${h(state.siteNameDraft ?? state.site.name)}"></label>
    <div class="actions"><button type="button" class="tonal" data-act="rename-site">${t("set.rename")}</button></div>
    <ul class="plain sites">${rows}</ul>${deleting}
    <label class="form-field">${t("set.newSite")}<input id="new-site" placeholder="${t("set.newSiteHint")}" value="${h(state.newSiteName ?? "")}"></label>
    <div class="actions"><button type="button" class="quiet" data-act="new-site">${t("set.create")}</button>
      <a class="button quiet" href="/api/site.json" download>${t("export.siteFile")}</a>
      <button type="button" class="quiet" data-act="import-site">${t("set.importSite")}</button></div></section>`;
}

/** What the look for a new version found, in words. */
function updateText(state) {
  if (!updateCheckOn()) return t("update.off");
  const update = state.update;
  if (!update || update.checking) return t("update.checking");
  if (update.error) return t("update.failed");
  if (update.newer) return `<a href="${h(update.url)}" target="_blank" rel="noopener">${t("update.new", { version: h(update.version) })}</a>`;
  return t("update.current");
}

/** The version with the commit it was built from: builds between two releases differ there. */
export function versionText(info) {
  if (!info?.version) return "";
  return info.commit ? `${info.version} · ${info.commit}` : info.version;
}

function viewSettings(state) {
  const languages = [["auto", t("set.system")], ...LANGUAGES].map(([code, name]) => `<option value="${code}"${languageChoice() === code ? " selected" : ""}>${h(name)}</option>`).join("");
  let theme = "auto";
  try {
    theme = localStorage.getItem("modbusio.theme") || "auto";
  } catch {
    // the system's theme then
  }
  const themes = [["auto", t("set.system")], ["light", t("set.light")], ["dark", t("set.dark")]].map(([code, name]) => `<option value="${code}"${theme === code ? " selected" : ""}>${name}</option>`).join("");
  const capture = state.capture;
  const captureButtons = capture.running
    ? `<button type="button" class="tonal" data-act="capture-stop">${t("cap.stop", { n: capture.frames ?? 0 })}</button>`
    : `<button type="button" class="tonal" data-act="capture-start">${t("cap.start")}</button>${capture.kept !== null && capture.kept !== undefined ? `<a class="button quiet" href="/api/capture.pcapng" download>${t("cap.download", { n: capture.kept })}</a>` : ""}`;
  const problems = (state.info?.problems ?? []).map((problem) => `<div class="strip warn">${h(problem)}</div>`).join("");
  return {
    title: t("tab.settings"),
    lead: t("set.lead"),
    next: null,
    html: `<div class="page"><div class="cards">
      ${sitesCard(state)}
      <section class="card"><h2>${t("set.look")}</h2>
        <label class="form-field">${t("set.language")}<select id="language">${languages}</select></label>
        <label class="form-field">${t("set.theme")}<select id="theme">${themes}</select></label></section>
      <section class="card"><h2>${t("set.writing")}</h2><p>${state.unlocked ? t("set.unlocked") : t("set.locked")}</p>
        <div class="actions">${state.unlocked ? `<button type="button" class="tonal" data-act="lock">${t("set.lock")}</button>` : ""}</div></section>
      <section class="card"><h2>${t("cap.title")}</h2><p>${t("cap.lead")}</p><p class="hint">${t("cap.rtuHint")}</p><div class="actions">${captureButtons}</div></section>
      <section class="card"><h2>${t("set.about")}</h2><dl class="facts"><dt>${t("set.version")}</dt><dd class="mono">${h(versionText(state.info))}</dd>
        <dt>${t("update.title")}</dt><dd>${updateText(state)}</dd>
        <dt>${t("set.dataDir")}</dt><dd class="mono cut">${h(state.info?.data_dir ?? t("set.noDataDir"))}</dd></dl>${problems}
        <label class="check"><input type="checkbox" id="update-check"${updateCheckOn() ? " checked" : ""}> ${t("update.switch")}</label>
        <p class="hint">${t("update.privacy")}</p>
        <h3>${t("set.keys")}</h3><p class="hint">${t("set.keysText")}</p></section>
    </div></div>`,
  };
}

registerView("watch", viewWatch);
registerView("recording", viewRecording);
registerView("changed", viewChanged);
registerView("settings", viewSettings);
