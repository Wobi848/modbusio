// The frame around every page: sidebar (site, search, areas, settings), page head with the one
// next step, status bar, and the search settings as a dialog. Markup as bacnetio's.

import { h } from "./html.js";
import { noteText, t } from "./i18n.js";

const ICON = (path) => `<svg class="nav-i" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${path}</svg>`;
const AREA_ICONS = {
  workspace: ICON('<rect x="3" y="3" width="8" height="8" rx="2"/><rect x="13" y="3" width="8" height="5" rx="2"/><rect x="13" y="11" width="8" height="10" rx="2"/><rect x="3" y="14" width="8" height="7" rx="2"/>'),
  explorer: ICON('<circle cx="11" cy="11" r="6.5"/><path d="M20 20l-4.2-4.2"/><path d="M8.5 11h5M11 8.5v5"/>'),
  profiles: ICON('<rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 8h8M8 12h8M8 16h5"/>'),
  watch: ICON('<path d="M2.5 12S6 5 12 5s9.5 7 9.5 7-3.5 7-9.5 7-9.5-7-9.5-7z"/><circle cx="12" cy="12" r="2.5"/>'),
  recording: ICON('<path d="M3 17l5-6 4 3 5-7 4 4"/><path d="M3 21h18"/>'),
  changed: ICON('<path d="M14.5 4.5l5 5L9 20H4v-5z"/><path d="M12 7l5 5"/>'),
};
const SETTINGS_ICON = ICON('<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M2 12h3M19 12h3M4.9 19.1L7 17M17 7l2.1-2.1"/>');
const SLIDERS_ICON = ICON('<path d="M4 7h10M18 7h2M4 17h2M10 17h10"/><circle cx="16" cy="7" r="2"/><circle cx="8" cy="17" r="2"/>');
const CHEVRON = '<svg class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 9l6 6 6-6"/></svg>';

function badge(state, area) {
  if (area === "workspace") return `<span class="n">${state.site.devices.length || ""}</span>`;
  if (area === "profiles") return `<span class="n">${state.profiles.length || ""}</span>`;
  if (area === "watch") return `<span class="n">${state.watch.length || ""}</span>`;
  if (area === "recording") return `<span class="n">${state.recording.running ? "●" : ""}</span>`;
  if (area === "changed") return `<span class="n warn">${state.journal.standing || ""}</span>`;
  return "";
}

function tab(state, area, key) {
  const current = state.area === area ? ' aria-current="page"' : "";
  return `<a href="#/${area}" data-area="${area}"${current} title="${t(`tab.${area}`)} (${key})">${AREA_ICONS[area]}<span>${t(`tab.${area}`)}</span>${badge(state, area)}</a>`;
}

/** How the search stands, in a few words with a dot. */
export function searchState(state) {
  if (!state.scan.finished) return `<span class="state"><span class="dot busy"></span>${t("conn.searching")}</span>`;
  const n = state.site.devices.length;
  if (!n) return `<span class="state"><span class="dot"></span>${t("conn.ready")}</span>`;
  return `<span class="state"><span class="dot on"></span>${n === 1 ? t("side.oneDevice") : t("side.devices", { n })}</span>`;
}

function siteMenu(state) {
  if (state.menu !== "sites") return "";
  const sites = state.sites.map((site) => {
    const act = site.open ? 'data-act="close-menu"' : `data-act="site-open" data-id="${h(site.id)}"`;
    return `<button type="button" class="quiet" role="option" aria-checked="${Boolean(site.open)}" ${act}><span class="cut">${h(site.name)}</span></button>`;
  }).join("");
  return `<div class="menu site-choices" role="listbox" aria-label="${t("side.site")}">${sites}
    <hr role="separator"><button type="button" class="quiet all-sites" data-act="go" data-area="settings">${t("side.allSites")}</button></div>`;
}

export function sideHtml(state) {
  const areas = state.areas.map((area, index) => tab(state, area, index + 1)).join("");
  const settingsCurrent = state.area === "settings" ? ' aria-current="page"' : "";
  return `<a class="brand" href="#/workspace"><img src="icon.svg" alt=""><span class="brand-name">modbusio</span></a>
    <div class="head-more open" id="head-more">
    <div class="side-card"><section class="side-part site-group" aria-label="${t("side.site")}"><span class="cap">${t("side.site")}</span>
      <div class="menu-host site-host"><button type="button" id="site-switch" class="quiet site-switch" data-act="site-menu" aria-haspopup="listbox" aria-expanded="${state.menu === "sites"}" title="${h(state.site.name)}"><span class="site-now">${h(state.site.name)}</span>${CHEVRON}</button>${siteMenu(state)}</div>
      <div class="side-meta"><span class="side-detail">${searchState(state)}</span>
        <button type="button" id="search-settings" class="quiet side-tool" data-act="search-dialog" aria-label="${t("side.search")}" title="${t("side.search")}">${SLIDERS_ICON}<span class="side-tool-label">${t("side.search")}</span></button></div>
    </section></div>
    </div>
    <nav class="tabs" aria-label="${t("areas")}">${areas}</nav>
    <a class="quiet settings-link" href="#/settings" data-area="settings"${settingsCurrent} title="${t("tab.settings")} (${state.areas.length + 1})">${SETTINGS_ICON}<span class="side-label">${t("tab.settings")}</span></a>`;
}

/** Title, one sentence and the one next step: every page says where you are. */
export function nextbarHtml(view) {
  const next = view.next ? `<button type="button" id="next" data-act="${view.next.act}"${view.next.data ?? ""}>${h(view.next.label)}</button>` : "";
  return `<div class="pagehead"><h1>${h(view.title)}</h1><p class="lead">${view.lead ?? ""}</p></div>${next}`;
}

function activity(state) {
  const scan = state.scan;
  if (!scan.finished) {
    const phase = t(`phase.${scan.phase || "hosts"}`);
    return `${phase} · ${scan.done ?? 0}/${scan.total ?? 0}`;
  }
  if (state.busy) return state.busy;
  return t("status.ready");
}

export function statusHtml(state) {
  const busy = !state.scan.finished || Boolean(state.busy);
  const changed = state.journal.standing
    ? `<button type="button" class="chip warn" data-act="go" data-area="changed">${t("chip.changed", { n: state.journal.standing })}</button>` : "";
  const recording = state.recording.running ? `<button type="button" class="chip accent" data-act="go" data-area="recording">● ${t("chip.recording")}</button>` : "";
  const capture = state.capture.running ? `<button type="button" class="chip accent" data-act="go" data-area="settings">● ${t("chip.capture", { n: state.capture.frames ?? 0 })}</button>` : "";
  const unlocked = state.unlocked ? `<button type="button" class="chip warn" data-act="lock">${t("chip.unlocked")}</button>` : "";
  return `<span class="state"><span class="dot ${busy ? "busy" : "on"}"></span>${h(activity(state))}</span>
    ${!state.scan.finished ? `<button type="button" class="quiet" data-act="stop-search">${t("stop")}</button>` : ""}
    <span class="chips">${unlocked}${changed}${recording}${capture}</span>`;
}

const field = (id, label, input, hint = "") => `<dt><label for="${id}">${label}</label></dt><dd>${input}</dd>${hint ? `<dd class="hint">${hint}</dd>` : ""}`;

function adapterOptions(state) {
  const options = state.adapters.map((adapter) => {
    const selected = (state.search.interface || state.adapters.find((one) => one.default)?.ip) === adapter.ip ? " selected" : "";
    return `<option value="${h(adapter.ip)}"${selected}>${h(`${adapter.name} · ${adapter.ip}/${adapter.prefix_len}`)}</option>`;
  }).join("");
  return options || `<option value="">${t("search.noAdapter")}</option>`;
}

function portOptions(state) {
  const ports = state.serialPorts.map((port) => {
    const label = port.description ? `${port.name} · ${port.description}` : port.name;
    return `<option value="${h(port.name)}"${port.name === state.search.serialPort ? " selected" : ""}>${h(label)}</option>`;
  }).join("");
  return ports || `<option value="">${t("search.noPort")}</option>`;
}

function modeChoice(state) {
  const modes = [["tcp", t("search.tcp")], ["gateway", t("search.gateway")]];
  if (state.info?.serial) modes.push(["serial", t("search.serial")]);
  return `<dt><span id="mode-label">${t("search.mode")}</span></dt><dd><span class="seg" role="group" aria-labelledby="mode-label">${modes.map(([mode, label]) =>
    `<button type="button" id="mode-${mode}" data-act="search-mode" data-mode="${mode}" aria-pressed="${state.search.mode === mode}">${label}</button>`).join("")}</span></dd>`;
}

const input = (id, key, state, placeholder = "", mono = true) =>
  `<input id="${id}" data-search="${key}" class="${mono ? "mono" : ""}" placeholder="${h(placeholder)}" value="${h(state.search[key])}">`;

function tcpFields(state) {
  return `${field("search-interface", t("search.adapter"), `<select id="search-interface" data-search="interface">${adapterOptions(state)}</select>`)}
    ${field("search-subnet", t("search.subnet"), input("search-subnet", "subnet", state, "192.168.10.0/24"), t("search.subnetHint"))}
    ${field("search-port", t("search.port"), input("search-port", "port", state, "502"))}`;
}

function rangeFields(state, timeoutHint) {
  return `${field("search-from", t("search.from"), input("search-from", "from", state, "1"))}
    ${field("search-to", t("search.to"), input("search-to", "to", state, "247"))}
    ${field("search-timeout", t("search.timeout"), input("search-timeout", "timeout", state, timeoutHint), t("search.timeoutHint"))}`;
}

function gatewayFields(state) {
  return `${field("search-gateway", t("search.gatewayAddress"), input("search-gateway", "gateway", state, "192.168.10.50:502"))}
    <dt>${t("search.framing")}</dt><dd><label class="check"><input type="checkbox" id="search-rtu" data-search-check="gatewayRtu"${state.search.gatewayRtu ? " checked" : ""}> ${t("search.rtuOverTcp")}</label></dd>
    <dd class="hint">${t("search.rtuOverTcpHint")}</dd>
    ${rangeFields(state, "500")}`;
}

function listenedHtml(state) {
  const listened = state.listened;
  if (!listened) return "";
  const trials = listened.listened.trials.map((trial) => `<li><span class="mono">${h(trial.settings)}</span>: ${t("listen.trial", { frames: trial.frames, stray: trial.stray_bytes })}</li>`).join("");
  return `<div class="strip ${listened.listened.active ? "warn" : "info"}">${h(noteText(listened.note))}</div><details><summary>${t("listen.details")}</summary><ul class="plain">${trials}</ul></details>`;
}

function serialFields(state) {
  const bauds = ["1200", "2400", "4800", "9600", "19200", "38400", "57600", "115200"].map((baud) =>
    `<option${baud === state.search.baud ? " selected" : ""}>${baud}</option>`).join("");
  const frames = ["8E1", "8N1", "8N2", "8O1"].map((frame) => `<option${frame === state.search.frame ? " selected" : ""}>${frame}</option>`).join("");
  return `${field("search-port-serial", t("search.serialPort"), `<select id="search-port-serial" data-search="serialPort">${portOptions(state)}</select>`)}
    ${field("search-baud", t("search.baud"), `<select id="search-baud" data-search="baud">${bauds}</select>`)}
    ${field("search-frame", t("search.frame"), `<select id="search-frame" data-search="frame">${frames}</select>`, t("search.frameHint"))}
    ${rangeFields(state, "300")}
    ${field("search-retries", t("search.retries"), input("search-retries", "retries", state, "0"))}`;
}

/** The search settings: where to look, and the search itself as the one main button. */
export function dialogHtml(state) {
  if (state.menu !== "search") return "";
  const mode = state.search.mode;
  const fields = mode === "serial" ? serialFields(state) : mode === "gateway" ? gatewayFields(state) : tcpFields(state);
  const serialTools = mode === "serial" ? `<button type="button" class="tonal" data-act="listen">${t("search.listen")}</button><button type="button" class="tonal" data-act="find-settings">${t("search.findSettings")}</button>` : "";
  return `<div class="modal"><div class="modal-box search-settings" role="dialog" aria-modal="true" aria-labelledby="search-title">
    <h2 id="search-title">${t("search.title")}</h2>
    <p class="sub">${t(`search.lead.${mode}`)}</p>
    <dl class="facts">${modeChoice(state)}${fields}</dl>
    ${mode === "serial" ? listenedHtml(state) : ""}
    <div class="acts"><button type="button" class="quiet" data-act="close-menu">${t("cancel")}</button>${serialTools}<button type="button" data-act="search">${t("search.start")}</button></div>
  </div></div>`;
}
