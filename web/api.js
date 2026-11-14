// Talks to modbusio's JSON API (docs/api.md). Errors come back as Error with the server's text in
// the page's language.

import { errorText, t } from "./i18n.js";

async function call(method, path, body) {
  const options = { method, headers: {} };
  if (body !== undefined) {
    options.headers["content-type"] = "application/json";
    options.body = JSON.stringify(body);
  }
  let response;
  try {
    response = await fetch(path, options);
  } catch {
    throw new Error(t("notRunning"));
  }
  const text = await response.text();
  let data = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    // not an answer of this modbusio's API: a proxy, or a program older than this page
    throw new Error(t("badAnswer", { path }));
  }
  if (!response.ok) {
    const error = new Error(data?.error ? errorText(data.error) : `${response.status} ${response.statusText}`);
    error.status = response.status;
    throw error;
  }
  return data;
}

const query = (key) => `key=${encodeURIComponent(key)}`;

export const api = {
  info: () => call("GET", "/api/info"),
  update: () => call("GET", "/api/update"),
  adapters: () => call("GET", "/api/adapters"),
  serialPorts: () => call("GET", "/api/serial-ports"),
  scan: () => call("GET", "/api/scan"),
  scanTcp: (request) => call("POST", "/api/scan/tcp", request),
  scanUnits: (request) => call("POST", "/api/scan/units", request),
  findSettings: (request) => call("POST", "/api/scan/settings", request),
  stopScan: () => call("POST", "/api/scan/stop"),
  listen: (port, seconds) => call("POST", "/api/listen", { port, seconds }),
  site: () => call("GET", "/api/site"),
  renameSite: (name) => call("POST", "/api/site/rename", { name }),
  sites: () => call("GET", "/api/sites"),
  newSite: (name) => call("POST", "/api/sites/new", { name }),
  openSite: (id) => call("POST", "/api/sites/open", { id }),
  deleteSite: (id) => call("POST", "/api/sites/delete", { id }),
  importSite: (file) => call("POST", "/api/site.json", file),
  addDevice: (key, name) => call("POST", "/api/devices/add", { key, name }),
  updateDevice: (change) => call("POST", "/api/devices/update", change),
  removeDevice: (key) => call("POST", "/api/devices/remove", { key }),
  identify: (key) => call("POST", "/api/devices/identify", { key }),
  read: (key, points) => call("POST", "/api/devices/read", points ? { key, points } : { key }),
  values: (items) => call("POST", "/api/values", { items }),
  explore: (request) => call("POST", "/api/explore", request),
  profiles: () => call("GET", "/api/profiles"),
  profile: (id) => call("GET", `/api/profiles/get?id=${encodeURIComponent(id)}`),
  saveProfile: (profile) => call("POST", "/api/profiles", profile),
  deleteProfile: (id) => call("POST", "/api/profiles/delete", { id }),
  importList: (request) => call("POST", "/api/profiles/import", request),
  session: () => call("GET", "/api/session"),
  setSession: (unlocked) => call("POST", "/api/session", { unlocked }),
  write: (request) => call("POST", "/api/write", request),
  journal: () => call("GET", "/api/journal"),
  restore: (request) => call("POST", "/api/restore", request),
  recording: (after) => call("GET", after ? `/api/recording?after=${after}` : "/api/recording"),
  recordAdd: (key, point) => call("POST", "/api/recording/add", { key, point }),
  recordRemove: (key, point) => call("POST", "/api/recording/remove", { key, point }),
  recordStart: (interval) => call("POST", "/api/recording/start", { interval }),
  recordStop: () => call("POST", "/api/recording/stop"),
  recordClear: () => call("POST", "/api/recording/clear"),
  analyser: (exchanges, frames) => {
    const after = [exchanges === undefined ? "" : `exchanges=${exchanges}`, frames === undefined ? "" : `frames=${frames}`].filter(Boolean).join("&");
    return call("GET", after ? `/api/analyser?${after}` : "/api/analyser");
  },
  analyserStart: (request) => call("POST", "/api/analyser/start", request),
  analyserStop: () => call("POST", "/api/analyser/stop"),
  analyserOpen: (fileName, content) => call("POST", "/api/analyser/open", { file_name: fileName, content_base64: content }),
  analyserUnit: (unit) => call("GET", `/api/analyser/unit?unit=${unit}`),
  analyserRegister: (unit, area, address) => call("GET", `/api/analyser/register?unit=${unit}&area=${area}&address=${address}`),
  analyserClear: () => call("POST", "/api/analyser/clear"),
  capture: () => call("GET", "/api/capture"),
  captureStart: () => call("POST", "/api/capture/start"),
  captureStop: () => call("POST", "/api/capture/stop"),
};

/** Links to files; the time zone goes along so times are the viewer's. */
const zone = () => `offset=${-new Date().getTimezoneOffset()}`;
export const files = {
  pointsCsv: (key) => `/api/points.csv?${query(key)}`,
  pointsXlsx: (key) => `/api/points.xlsx?${query(key)}`,
  devicesCsv: () => "/api/devices.csv",
  siteXlsx: () => "/api/site.xlsx",
  siteJson: () => "/api/site.json",
  profileCsv: (id) => `/api/profile.csv?id=${encodeURIComponent(id)}`,
  profileXlsx: (id) => `/api/profile.xlsx?id=${encodeURIComponent(id)}`,
  journalCsv: () => `/api/journal.csv?${zone()}`,
  journalXlsx: () => `/api/journal.xlsx?${zone()}`,
  recordingCsv: () => `/api/recording.csv?${zone()}`,
  recordingXlsx: () => `/api/recording.xlsx?${zone()}`,
  capture: () => "/api/capture.pcapng",
};
