// What the buttons of the explorer, the profiles and the import do.

import { api } from "./api.js";
import { t } from "./i18n.js";
import { parseStates } from "./profiles.js";
import { refreshSession } from "./session.js";
import { attempt, deviceName, go, remember, render, selectedDevice, state, toast } from "./store.js";

const NUMBER_FIELDS = ["address", "factor", "offset", "scale_register", "min", "max"];
const OPTIONAL_FIELDS = ["scale_register", "min", "max"];
const isBits = (area) => area === "coil" || area === "discrete_input";

/** A number as typed, with a decimal comma or point; NaN when it is none. */
export const typedNumber = (text) => (String(text).trim() === "" ? NaN : Number(String(text).trim().replace(",", ".")));

/** Why a profile cannot be saved as it is, or null: every number field holds a number. */
export function draftProblem(draft) {
  if (!draft.name.trim()) return t("prof.needName");
  const names = new Set();
  for (const point of draft.points) {
    if (!point.name.trim()) return t("prof.needPointName");
    if (names.has(point.name)) return t("prof.twice", { name: point.name });
    names.add(point.name);
    for (const field of NUMBER_FIELDS) {
      const value = point[field];
      if (OPTIONAL_FIELDS.includes(field) && (value === null || value === undefined)) continue;
      if (typeof value !== "number" || !Number.isFinite(value)) return t("prof.badNumber", { point: point.name, field: t(`role.${field}`) });
    }
    if (!Number.isInteger(point.address) || point.address < 0 || point.address > 65535) return t("prof.badAddress", { point: point.name });
  }
  return null;
}

/** A point from the explorer's form, ready for the profile. */
export function pointFromExplorer(adding, explorer, row, profileOrder) {
  const order = explorer.order === profileOrder ? undefined : explorer.order;
  return {
    name: adding.name.trim(),
    group: adding.group.trim(),
    area: explorer.area,
    address: row.address,
    type: isBits(explorer.area) ? "bool" : adding.type,
    ...(order && !isBits(explorer.area) ? { order } : {}),
    factor: typedNumber(adding.factor || "1"),
    offset: 0,
    unit: adding.unit.trim(),
    description: "",
    states: [],
    writable: Boolean(adding.writable),
  };
}

async function runExplore() {
  const explorer = state.explorer;
  const start = Number(explorer.start);
  const count = Number(explorer.count);
  if (!Number.isInteger(start) || start < 0 || start > 65535 || !Number.isInteger(count) || count < 1 || count > 2000) {
    toast(t("ex.badRange"), true);
    return;
  }
  explorer.busy = true;
  explorer.row = null;
  explorer.adding = null;
  render();
  const result = await attempt(() => api.explore({ key: explorer.key, area: explorer.area, start, count }));
  explorer.busy = false;
  if (result) explorer.result = result;
  remember();
  render();
}

async function savePoint() {
  const explorer = state.explorer;
  const row = explorer.result?.rows.find((item) => item.address === explorer.row);
  const device = state.site.devices.find((item) => item.key === explorer.key);
  if (!row || !device) return;
  if (!explorer.adding.name.trim()) { toast(t("prof.needPointName"), true); return; }
  let profile;
  if (device.profile) {
    const loaded = await attempt(() => api.profile(device.profile));
    if (!loaded) return;
    profile = loaded.profile;
  } else {
    profile = { id: "", name: deviceName(device), vendor: device.vendor ?? "", model: device.product ?? "", origin: "own", source: "", order: explorer.order, points: [] };
  }
  const point = pointFromExplorer(explorer.adding, explorer, row, profile.order);
  if (!Number.isFinite(point.factor) || point.factor === 0) { toast(t("prof.badFactor"), true); return; }
  // the same profile open in the editor: what is typed there and not saved counts too
  const draft = profile.id && state.profileDraft?.id === profile.id ? state.profileDraft : null;
  const typed = draft && state.profileDirty;
  if ([...profile.points, ...(typed ? draft.points : [])].some((item) => item.name === point.name)) { toast(t("prof.twice", { name: point.name }), true); return; }
  profile.points.push(point);
  const saved = await attempt(() => api.saveProfile(profile));
  if (!saved) return;
  // the editor takes the point as well, or saving there would write the profile without it
  if (typed) draft.points.push(saved.points.find((item) => item.name === point.name));
  else if (draft) state.profileDraft = saved;
  if (!device.profile) await attempt(() => api.updateDevice({ key: device.key, profile: saved.id }));
  // the site names each device's point count, which this point changed
  state.site = (await attempt(api.site)) ?? state.site;
  state.profiles = (await attempt(api.profiles)) ?? state.profiles;
  delete state.readings[device.key];
  explorer.adding = null;
  toast(t("ex.pointSaved", { name: point.name, profile: saved.name }));
  render();
}

/** The write of a register without a profile point, as the explorer asks it. */
function rawRequest() {
  const explorer = state.explorer;
  const asking = state.asking;
  const type = isBits(explorer.area) ? "bool" : "u16";
  return {
    key: explorer.key, confirm_raw: true, value: asking.value,
    raw: { area: explorer.area, address: asking.address, type, order: explorer.order, factor: 1 },
  };
}

async function rawWrite() {
  const entry = await attempt(() => api.write(rawRequest()));
  state.asking = null;
  if (entry?.error) toast(t("write.refused", { reason: entry.error }), true);
  else if (entry) toast(t("write.done", { value: entry.text, before: entry.before_text ?? "–" }));
  state.journal = (await attempt(api.journal)) ?? state.journal;
  await runExplore();
}

async function openProfile(id) {
  const loaded = await attempt(() => api.profile(id));
  if (!loaded) return;
  state.profileId = id;
  state.profileDraft = loaded.profile;
  state.profileDirty = false;
  state.pointIndex = null;
  state.asking = null;
  render();
}

function fileBase64(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result).replace(/^data:[^,]*,/, ""));
    reader.onerror = () => reject(new Error(t("imp.unreadable")));
    reader.readAsDataURL(file);
  });
}

async function runImport() {
  const importing = state.importing;
  const options = { ...importing.options };
  const result = await attempt(() => api.importList({
    file_name: importing.fileName,
    content_base64: importing.content,
    options,
    key: importing.key || undefined,
    order: importing.order,
  }));
  if (result) importing.result = result;
  render();
}

function startImport(key) {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".csv,.txt,.xlsx,.xlsm,.xls,.ods";
  input.addEventListener("change", async () => {
    const file = input.files?.[0];
    if (!file) return;
    const content = await attempt(() => fileBase64(file));
    if (!content) return;
    const device = state.site.devices.find((item) => item.key === key);
    state.importing = {
      fileName: file.name, content, key: key ?? "", order: "ABCD", result: null, options: {},
      name: file.name.replace(/\.[^.]+$/, ""), forDevice: Boolean(device),
    };
    render();
    await runImport();
  });
  input.click();
}

function draftPoint() {
  return state.profileDraft?.points[state.pointIndex];
}

/** Marks the profile changed; true for the first change, which enables *Save profile*. */
function changed() {
  const first = !state.profileDirty;
  state.profileDirty = true;
  return first;
}

/** Whether closing the open profile would lose something typed. A new profile nobody typed in loses nothing. */
export function unsavedChanges(state) {
  const draft = state.profileDraft;
  if (!draft || !state.profileDirty) return false;
  return Boolean(draft.id || draft.name || draft.vendor || draft.model || draft.points.length);
}

function closeProfile() {
  state.profileDraft = null;
  state.profileId = null;
  state.pointIndex = null;
  state.asking = null;
  render();
}

export const profileActions = {
  "explore-device": () => {
    const device = selectedDevice();
    if (device) state.explorer.key = device.key;
    if (!state.explorer.key && state.site.devices.length) state.explorer.key = state.site.devices[0].key;
    go("explorer");
  },
  explore: () => runExplore(),
  "explore-row": (element) => {
    state.explorer.row = Number(element.dataset.address);
    state.explorer.adding = null;
    state.asking = null;
    render();
  },
"explore-close": () => { state.explorer.row = null; state.explorer.adding = null; state.asking = null; render(); },
  "toggle-missing": () => { state.explorer.showMissing = !state.explorer.showMissing; render(); },
  "explore-order": (element) => { state.explorer.order = element.dataset.order; remember(); render(); },
  "add-to-profile": () => {
    const explorer = state.explorer;
    explorer.adding = { name: "", group: "", type: isBits(explorer.area) ? "bool" : "u16", factor: "1", unit: "", writable: false };
    render();
    document.getElementById("add-point-name")?.focus();
  },
  "cancel-add": () => { state.explorer.adding = null; render(); },
  "save-point": () => savePoint(),
  "ask-raw-write": async () => {
    await refreshSession();
    const row = state.explorer.result?.rows.find((item) => item.address === state.explorer.row);
    state.asking = { kind: "raw-write", address: state.explorer.row, step: "value", value: row?.value ?? "" };
    render();
  },
  "raw-write-check": async () => {
    if (String(state.asking.value).trim() === "") { toast(t("write.empty"), true); return; }
    if (!(await attempt(() => api.write({ ...rawRequest(), check: true })))) return;
    state.asking.step = "confirm";
    render();
  },
  "raw-write": () => rawWrite(),
  "open-profile": (element) => openProfile(element.dataset.id),
  "new-profile": () => {
    state.profileId = null;
    state.profileDraft = { id: "", name: "", vendor: "", model: "", origin: "own", source: "", order: "ABCD", points: [] };
    state.profileDirty = true;
    state.pointIndex = null;
    render();
    document.getElementById("profile-name")?.focus();
  },
  "close-profile": () => {
    if (unsavedChanges(state)) { state.asking = { kind: "close-profile" }; render(); return; }
    closeProfile();
  },
  "discard-profile": () => closeProfile(),
  "save-profile": async () => {
    const problem = draftProblem(state.profileDraft);
    if (problem) { toast(problem, true); return; }
    const saved = await attempt(() => api.saveProfile(state.profileDraft));
    if (!saved) return;
    state.profileDraft = saved;
    state.profileId = saved.id;
    state.profileDirty = false;
    state.asking = null;
    state.profiles = (await attempt(api.profiles)) ?? state.profiles;
    for (const device of state.site.devices.filter((item) => item.profile === saved.id)) delete state.readings[device.key];
    state.site = (await attempt(api.site)) ?? state.site;
    toast(t("prof.saved", { name: saved.name }));
    render();
  },
  "add-profile-point": () => {
    const last = state.profileDraft.points.at(-1);
    const next = last ? last.address + (last.type?.match(/32/) ? 2 : last.type?.match(/64/) ? 4 : 1) : 0;
    state.profileDraft.points.push({ name: "", group: last?.group ?? "", area: last?.area ?? "holding_register", address: Math.min(next, 65535), type: last?.type ?? "u16", factor: 1, offset: 0, unit: "", description: "", states: [], writable: false });
    state.pointIndex = state.profileDraft.points.length - 1;
    changed();
    render();
    document.getElementById("point-name")?.focus();
  },
"close-point-form": () => { state.pointIndex = null; render(); },
  "edit-point": (element) => { state.pointIndex = Number(element.dataset.index); render(); },
  "duplicate-point": () => {
    const point = draftPoint();
    if (!point) return;
    state.profileDraft.points.splice(state.pointIndex + 1, 0, { ...structuredClone(point), name: `${point.name} 2` });
    state.pointIndex += 1;
    changed();
    render();
  },
  "remove-point": () => {
    state.profileDraft.points.splice(state.pointIndex, 1);
    state.pointIndex = null;
    changed();
    render();
  },
  "ask-delete-profile": () => { state.asking = { kind: "delete-profile" }; render(); },
  "delete-profile": async () => {
    const list = await attempt(() => api.deleteProfile(state.profileDraft.id));
    state.asking = null;
    if (!list) { render(); return; }
    state.profiles = list;
    state.profileDraft = null;
    state.profileId = null;
    render();
  },
  "import-list": () => startImport(state.selected ?? ""),
  "import-for-device": () => startImport(state.selected),
  "close-import": () => { state.importing = null; render(); },
  "import-again": () => runImport(),
  "import-save": async () => {
    const importing = state.importing;
    const imported = importing.result?.imported;
    if (!imported?.points.length) return;
    const profile = { id: "", name: importing.name.trim() || importing.fileName, vendor: "", model: "", origin: "imported", source: importing.fileName, order: importing.order, points: imported.points };
    const saved = await attempt(() => api.saveProfile(profile));
    if (!saved) return;
    if (importing.key) {
      const site = await attempt(() => api.updateDevice({ key: importing.key, profile: saved.id }));
      if (site) state.site = site;
      delete state.readings[importing.key];
    }
    state.profiles = (await attempt(api.profiles)) ?? state.profiles;
    state.importing = null;
    toast(t("imp.saved", { name: saved.name, n: saved.points.length }));
    render();
  },

  /** Fields of the explorer, the profile editor and the import, as they are typed. */
  onField(element) {
    const { dataset } = element;
    if (dataset.explorer) { state.explorer[dataset.explorer] = element.value; remember(); return dataset.explorer === "order" || dataset.explorer === "area"; }
    if (dataset.adding) { state.explorer.adding[dataset.adding] = element.value; return false; }
    if (dataset.addingCheck) { state.explorer.adding[dataset.addingCheck] = element.checked; return false; }
    if (dataset.profile) { state.profileDraft[dataset.profile] = element.value; return changed() || dataset.profile === "order"; }
    if (dataset.pointCheck) { draftPoint()[dataset.pointCheck] = element.checked; changed(); return true; }
    if (dataset.pointField) {
      const point = draftPoint();
      const field = dataset.pointField;
      if (field === "states") point.states = parseStates(element.value);
      else if (field === "order") point.order = element.value || undefined;
      else if (NUMBER_FIELDS.includes(field)) {
        const number = typedNumber(element.value);
        point[field] = OPTIONAL_FIELDS.includes(field) && element.value.trim() === "" ? undefined : Number.isNaN(number) ? element.value : number;
      } else point[field] = element.value;
      if (field === "area" && isBits(point.area)) point.type = "bool";
      return changed() || field === "area" || field === "type";
    }
    if (dataset.import === "name") { state.importing.name = element.value; return false; }
    return false;
  },

  /** Choices of the import that read the list again. */
  async onChange(element) {
    const { dataset } = element;
    if (dataset.role && state.importing?.result) {
      const columns = { ...state.importing.result.imported.columns };
      if (element.value === "") delete columns[dataset.role];
      else columns[dataset.role] = Number(element.value);
      state.importing.options.columns = columns;
      state.importing.options.header_row = state.importing.result.imported.header_row;
      await runImport();
      return;
    }
    if (!dataset.import || dataset.import === "name") return;
    const importing = state.importing;
    if (dataset.import === "convention") importing.options.convention = element.value;
    if (dataset.import === "area") importing.options.area = element.value || undefined;
    if (dataset.import === "key") importing.key = element.value;
    if (dataset.import === "order") importing.order = element.value;
    await runImport();
  },
};
