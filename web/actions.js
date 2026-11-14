// What the buttons do: search, site and devices, reading, writing, the watch list, the recording,
// the log and the settings. Profiles, explorer and import are in profile-actions.js.

import { api } from "./api.js";
import { codeText, setLanguage, t } from "./i18n.js";
import { attempt, deviceName, go, pickFile, remember, render, selectedDevice, state, toast } from "./store.js";
import { profileActions } from "./profile-actions.js";
import { analyserActions, analyserField } from "./analyser-actions.js";
import { refreshSession } from "./session.js";
import { setUpdateCheck } from "./update.js";
import { nextSort } from "./sorting.js";

export async function loadSite() {
  const [site, sites, profiles] = await Promise.all([api.site(), api.sites(), api.profiles()]);
  // another site's name replaces one typed for the site before
  if (site.id !== state.site?.id) state.siteNameDraft = null;
  state.site = site;
  state.sites = sites;
  state.profiles = profiles;
  if (state.selected && !site.devices.some((device) => device.key === state.selected)) state.selected = null;
}

export async function loadJournal() {
  state.journal = await api.journal();
}

/** The search's request from the dialog's fields; an error text when a field is wrong. */
export function searchRequest(search) {
  const number = (text, fallback) => (String(text).trim() === "" ? fallback : Number(text));
  const timeout = number(search.timeout, undefined);
  if (timeout !== undefined && !(timeout >= 50 && timeout <= 10000)) return { error: t("search.badTimeout") };
  if (search.mode === "tcp") {
    const port = number(search.port, 502);
    if (!Number.isInteger(port) || port < 1 || port > 65535) return { error: t("search.badPort") };
    return { kind: "tcp", body: { subnet: search.subnet.trim() || undefined, interface: search.interface || undefined, port, timeout_ms: timeout } };
  }
  const from = number(search.from, 1);
  const to = number(search.to, 247);
  if (!Number.isInteger(from) || !Number.isInteger(to) || from < 1 || to > 247 || from > to) return { error: t("search.badRange") };
  if (search.mode === "gateway") {
    const address = search.gateway.trim();
    if (!address) return { error: t("search.needGateway") };
    const withPort = address.includes(":") ? address : `${address}:502`;
    return { kind: "units", body: { endpoint: `${search.gatewayRtu ? "rtu-tcp" : "tcp"}:${withPort}`, from, to, timeout_ms: timeout } };
  }
  if (!search.serialPort) return { error: t("search.needPort") };
  const retries = number(search.retries, 0);
  if (!Number.isInteger(retries) || retries < 0 || retries > 5) return { error: t("search.badRetries") };
  return { kind: "units", body: { endpoint: `serial:${search.serialPort}:${search.baud}:${search.frame}`, from, to, timeout_ms: timeout, retries } };
}

/** The search for devices at other settings from the dialog's fields: the speed and frame there
 * are the trunk's. An error text when a field is wrong. */
export function otherSettingsRequest(search) {
  const request = searchRequest({ ...search, mode: "serial" });
  if (request.error) return request;
  const { from, to, timeout_ms: timeoutMs } = request.body;
  return { body: { port: search.serialPort, baud: Number(search.baud), frame: search.frame, from, to, timeout_ms: timeoutMs, all_speeds: Boolean(search.allSpeeds) } };
}

/** Asks how long the search at other settings would take, and puts the question in the dialog. */
async function askOtherSettings() {
  const request = otherSettingsRequest(state.search);
  if (request.error) { toast(request.error, true); return; }
  const check = await attempt(() => api.otherSettings({ ...request.body, check: true }));
  if (!check) return;
  state.findingSettings = false;
  state.asking = { kind: "other-settings", minutes: Math.max(1, Math.round(check.estimate_seconds / 60)), settings: check.settings };
  render();
}

export async function pollScan() {
  state.scan = await api.scan();
  if (!state.scan.finished) {
    render();
    setTimeout(() => attempt(pollScan), 400);
    return;
  }
  // the settings search tells what it found: take it over for the unit search
  if (state.scan.settings) {
    const [baud, frame] = state.scan.settings.split(" ");
    Object.assign(state.search, { baud, frame });
    remember();
  }
  await loadSite();
  render();
}

/** Starts a search in the background. The settings search keeps the dialog open: its result is
 * the next step there (`keepDialog`); the others close it and fill the device list. */
async function startSearch(start, keepDialog = false) {
  if (await attempt(start) === null) return;
  state.findingSettings = keepDialog;
  if (!keepDialog) state.menu = null;
  state.scan = { ...state.scan, finished: false, phase: "", done: 0, total: 0 };
  render();
  await attempt(pollScan);
}

/** Reads the chosen device's points and keeps them for the table. */
export async function readDevice(quiet = false) {
  const device = selectedDevice();
  if (!device?.profile) return;
  if (!quiet) {
    state.busy = t("status.reading", { name: device.name || device.line });
    render();
  }
  const answer = await attempt(() => api.read(device.key));
  state.busy = null;
  if (answer) state.readings[device.key] = answer;
  render();
}

async function writePoint() {
  const device = selectedDevice();
  const asking = state.asking;
  const entry = await attempt(() => api.write({ key: device.key, point: asking.point, value: asking.value }));
  state.asking = null;
  if (entry?.error) toast(t("write.refused", { reason: entry.error }), true);
  else if (entry) toast(t("write.done", { value: entry.text, before: entry.before_text ?? "–" }));
  await attempt(loadJournal);
  await readDevice(true);
}

const pointOf = (name) => state.readings[state.selected]?.points.find((point) => point.name === name);

/** Chooses a device of the site, or none; the next render shows it. */
function selectDevice(key) {
  state.selected = key;
  state.selectedPoint = null;
  state.deviceDetailOpen = false;
  state.pointFilter = "";
  state.asking = null;
  state.live = false;
  remember();
}

export const actions = {
  ...profileActions,
  ...analyserActions,
  go: (element) => go(element.dataset.area),
  sort: (element) => {
    state.sorts[element.dataset.list] = nextSort(state.sorts[element.dataset.list], element.dataset.key);
    render();
  },
  "close-menu": () => { state.menu = null; state.findingSettings = false; render(); },
  "site-menu": () => { state.menu = state.menu === "sites" ? null : "sites"; render(); },
  "site-open": async (element) => {
    if (await attempt(() => api.openSite(element.dataset.id)) === null) return;
    state.menu = null;
    state.selected = null;
    state.readings = {};
    await attempt(loadSite);
    render();
  },
  "search-dialog": async () => {
    state.menu = "search";
    render();
    const [adapters, ports] = await Promise.all([attempt(api.adapters), state.info?.serial ? attempt(api.serialPorts) : []]);
    state.adapters = adapters ?? [];
    state.serialPorts = ports ?? [];
    if (!state.search.serialPort && state.serialPorts.length) state.search.serialPort = state.serialPorts[0].name;
    render();
  },
  "search-mode": (element) => { state.search.mode = element.dataset.mode; state.findingSettings = false; remember(); render(); },
  search: () => {
    const request = searchRequest(state.search);
    if (request.error) { toast(request.error, true); return; }
    remember();
    return startSearch(() => (request.kind === "tcp" ? api.scanTcp(request.body) : api.scanUnits(request.body)));
  },
  "find-settings": () => {
    if (!state.search.serialPort) { toast(t("search.needPort"), true); return; }
    state.listened = null;
    return startSearch(() => api.findSettings({ port: state.search.serialPort }), true);
  },
  listen: async () => {
    if (!state.search.serialPort) { toast(t("search.needPort"), true); return; }
    state.busy = t("status.listening");
    render();
    state.listened = await attempt(() => api.listen(state.search.serialPort, 1.5));
    state.busy = null;
    render();
  },
  "stop-search": () => attempt(api.stopScan),
  "other-settings-ask": () => askOtherSettings(),
  "other-settings": () => {
    const request = otherSettingsRequest(state.search);
    if (request.error) { toast(request.error, true); return; }
    state.asking = null;
    state.listened = null;
    remember();
    return startSearch(() => api.otherSettings(request.body), true);
  },
  unlock: async () => {
    const session = await attempt(() => api.setSession(true));
    if (session) state.unlocked = session.unlocked;
    if (state.asking?.kind === "write" || state.asking?.kind === "raw-write") state.asking.step = "value";
    render();
  },
  lock: async () => {
    const session = await attempt(() => api.setSession(false));
    if (session) state.unlocked = session.unlocked;
    render();
  },
  "cancel-ask": () => { state.asking = null; render(); },
  "select-device": (element) => { selectDevice(element.dataset.key || null); render(); },
  "add-device-menu": () => { state.menu = state.menu === "add-device" ? null : "add-device"; render(); },
  "add-device": async () => {
    const key = document.getElementById("add-key").value.trim();
    const name = document.getElementById("add-name").value.trim();
    const before = new Set(state.site.devices.map((device) => device.key));
    const answer = await attempt(() => api.addDevice(key, name));
    if (!answer) return;
    const { check, ...site } = answer;
    state.site = site;
    state.menu = null;
    state.addKey = "";
    state.addName = "";
    // the new device is chosen, so its next step (read, explore) is at hand
    const added = site.devices.find((device) => !before.has(device.key));
    if (added) selectDevice(added.key);
    const named = added ? deviceName(added) : key;
    if (check?.answered === false) toast(t("dev.addedSilent", { name: named, reason: codeText(check.error) }), true);
    else toast(t("dev.added", { name: named }));
    render();
  },
  "export-menu": () => { state.menu = state.menu === "export" ? null : "export"; render(); },
  "read-device": () => readDevice(),
  identify: async () => {
    const device = selectedDevice();
    if (!device) return;
    const answer = await attempt(() => api.identify(device.key));
    if (answer) toast(answer.objects.map((object) => object.text).join(" · "));
    await attempt(loadSite);
    render();
  },
  "ask-remove-device": () => { state.asking = { kind: "remove-device" }; render(); },
  "remove-device": async () => {
    const site = await attempt(() => api.removeDevice(state.selected));
    if (!site) return;
    state.site = site;
    state.selected = null;
    state.asking = null;
    remember();
    render();
  },
  "select-point": (element) => { state.selectedPoint = element.dataset.point; state.asking = null; render(); },
  "close-point": () => { state.selectedPoint = null; state.deviceDetailOpen = false; state.asking = null; render(); },
  "open-device-detail": () => { state.deviceDetailOpen = true; state.selectedPoint = null; render(); },
  "toggle-live": () => { state.live = !state.live; render(); },
  "toggle-watch": () => {
    const item = { key: state.selected, point: state.selectedPoint };
    const index = state.watch.findIndex((one) => one.key === item.key && one.point === item.point);
    if (index >= 0) state.watch.splice(index, 1);
    else state.watch.push(item);
    remember();
    render();
  },
  "toggle-record": async () => {
    const recorded = state.recording.series.some((series) => series.device === state.selected && series.point === state.selectedPoint);
    const recording = await attempt(() => (recorded ? api.recordRemove(state.selected, state.selectedPoint) : api.recordAdd(state.selected, state.selectedPoint)));
    if (recording) state.recording = recording;
    render();
  },
  "ask-write": async () => {
    await refreshSession();
    const point = pointOf(state.selectedPoint);
    state.asking = { kind: "write", point: state.selectedPoint, step: "value", value: point?.reading?.value ?? "" };
    render();
    document.getElementById("write-value")?.focus();
  },
  "write-check": async () => {
    if (String(state.asking.value).trim() === "") { toast(t("write.empty"), true); return; }
    const checked = await attempt(() => api.write({ key: selectedDevice().key, point: state.asking.point, value: state.asking.value, check: true }));
    if (!checked) return;
    state.asking.step = "confirm";
    render();
  },
  write: () => writePoint(),
  unwatch: (element) => {
    state.watch = state.watch.filter((item) => !(item.key === element.dataset.key && item.point === element.dataset.point));
    remember();
    render();
  },
  "record-start": async () => {
    const interval = Number(document.getElementById("record-interval")?.value ?? state.recording.interval);
    const recording = await attempt(() => api.recordStart(interval));
    if (recording) state.recording = recording;
    render();
  },
  "record-stop": async () => { state.recording = (await attempt(api.recordStop)) ?? state.recording; render(); },
  "record-clear": async () => { state.recording = (await attempt(api.recordClear)) ?? state.recording; render(); },
  "record-remove": async (element) => {
    const recording = await attempt(() => api.recordRemove(element.dataset.key, element.dataset.point));
    if (recording) state.recording = recording;
    render();
  },
  "ask-restore": async (element) => {
    await refreshSession();
    state.asking = { kind: "restore", id: Number(element.dataset.id) };
    render();
  },
  "ask-restore-all": async () => {
    await refreshSession();
    state.asking = { kind: "restore", all: true };
    if (state.area !== "changed") go("changed");
    else render();
  },
  restore: async () => {
    const request = state.asking.all ? { all: true } : { ids: [state.asking.id] };
    const answer = await attempt(() => api.restore(request));
    state.asking = null;
    const failed = (answer?.results ?? []).filter((result) => result.error || result.entry?.error);
    if (answer && failed.length) toast(t("log.restoreFailed", { n: failed.length }), true);
    else if (answer) toast(t("log.restored", { n: answer.results.length }));
    await attempt(loadJournal);
    render();
  },
  "rename-site": async () => {
    const site = await attempt(() => api.renameSite(state.siteNameDraft ?? state.site.name));
    if (site) { state.site = site; state.siteNameDraft = null; }
    state.sites = (await attempt(api.sites)) ?? state.sites;
    render();
  },
  "new-site": async () => {
    const name = (state.newSiteName ?? "").trim();
    if (!name) { toast(t("set.needName"), true); return; }
    if (await attempt(() => api.newSite(name)) === null) return;
    state.newSiteName = "";
    state.selected = null;
    state.readings = {};
    await attempt(loadSite);
    render();
  },
  "import-site": () => pickFile(".json", async (file) => {
    const text = await file.text();
    let parsed;
    try {
      parsed = JSON.parse(text);
    } catch {
      toast(t("set.notSiteFile"), true);
      return;
    }
    if (await attempt(() => api.importSite(parsed)) === null) return;
    state.selected = null;
    state.readings = {};
    await attempt(loadSite);
    render();
  }),
  "ask-delete-site": (element) => { state.asking = { kind: "delete-site", id: element.dataset.id }; render(); },
  "delete-site": async () => {
    const sites = await attempt(() => api.deleteSite(state.asking.id));
    state.asking = null;
    if (sites) await attempt(loadSite);
    render();
  },
  "capture-start": async () => { state.capture = (await attempt(api.captureStart)) ?? state.capture; render(); },
  "capture-stop": async () => { state.capture = (await attempt(api.captureStop)) ?? state.capture; render(); },
};

/** Fields that change the state as they are typed or chosen. */
export function onField(element) {
  const { dataset } = element;
  if (dataset.search) { state.search[dataset.search] = element.value; remember(); return false; }
  if (dataset.searchCheck) {
    state.search[dataset.searchCheck] = element.checked;
    remember();
    // the question names the time, which depends on the speeds asked
    if (state.asking?.kind === "other-settings") attempt(askOtherSettings);
    return false;
  }
  if (dataset.analyser) return analyserField(element);
  if (element.id === "point-filter") { state.pointFilter = element.value; return true; }
  if (element.id === "write-value" && state.asking) { state.asking.value = element.value; return false; }
  if (element.id === "add-key") { state.addKey = element.value; return false; }
  if (element.id === "add-name") { state.addName = element.value; return false; }
  // kept as typed, so a redraw (the page starting, a live value) does not lose them
  if (element.id === "site-name") { state.siteNameDraft = element.value; return false; }
  if (element.id === "new-site") { state.newSiteName = element.value; return false; }
  return profileActions.onField(element);
}

/** Fields that act when their choice is made. */
export async function onChange(element) {
  const { dataset } = element;
  if (dataset.deviceName !== undefined) {
    const site = await attempt(() => api.updateDevice({ key: dataset.deviceName, name: element.value }));
    if (site) state.site = site;
    render();
    return;
  }
  if (dataset.deviceProfile !== undefined) {
    const site = await attempt(() => api.updateDevice({ key: dataset.deviceProfile, profile: element.value }));
    if (site) state.site = site;
    delete state.readings[dataset.deviceProfile];
    render();
    return;
  }
  if (element.id === "language") { setLanguage(element.value); render(); return; }
  if (element.id === "update-check") { setUpdateCheck(element.checked); return; }
  if (element.id === "theme") {
    try {
      localStorage.setItem("modbusio.theme", element.value);
    } catch {
      // the theme then lasts until the page is reloaded
    }
    const dark = element.value === "dark" || (element.value === "auto" && matchMedia("(prefers-color-scheme: dark)").matches);
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    return;
  }
  await profileActions.onChange(element);
}
