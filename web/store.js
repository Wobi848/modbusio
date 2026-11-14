// The page's state and its drawing. Every action changes `state` and calls render(); the views
// are functions of the state. Scroll positions and the focused field survive a redraw.

import { codeText, t } from "./i18n.js";

export const AREAS = ["workspace", "explorer", "profiles", "watch", "recording", "changed", "analyser"];
const MEMORY_KEY = "modbusio.memory";

export const state = {
  area: "workspace",
  info: null,
  /** What the last look for a new version found (update.js). */
  update: null,
  site: { id: "", name: "", devices: [] },
  sites: [],
  profiles: [],
  adapters: [],
  serialPorts: [],
  /** The search's report from the server; `running` while it goes on. */
  scan: { finished: true, devices: [], hosts: [], notes: [] },
  search: {
    mode: "tcp", interface: "", subnet: "", port: "502",
    gateway: "", gatewayRtu: false,
    serialPort: "", baud: "9600", frame: "8E1",
    from: "1", to: "247", timeout: "", retries: "0",
  },
  listened: null,
  /** The device chosen in the workspace, by key; its points as read. */
  selected: null,
  readings: {},
  pointFilter: "",
  selectedPoint: null,
  live: false,
  explorer: { key: "", area: "holding_register", start: "0", count: "100", order: "ABCD", result: null, busy: false, row: null, adding: null },
  profileId: null,
  profileDraft: null,
  pointIndex: null,
  importing: null,
  watch: [],
  watchValues: {},
  recording: { series: [], interval: 10, running: false },
  journal: { entries: [], standing: 0 },
  unlocked: false,
  capture: { running: false, frames: null, kept: null },
  /** The bus analyser: the start's choices, what the server heard (`heard`), the page's view. */
  analyser: {
    port: "", speed: "listen", frame: "8E1", heard: null, busy: false, paused: false,
    view: "units", chosen: null, unitFilter: "", problemsOnly: false, writesOnly: false,
    registersUnit: "", register: null, unitData: null, history: null,
  },
  /** A question in the details: `{ kind, ... }` for writing, restoring, removing. */
  asking: null,
  menu: null,
  busy: null,
  notes: [],
  error: null,
};

/** What survives a reload: the area, the chosen device, the search settings, the watch list. */
export function remember() {
  try {
    localStorage.setItem(MEMORY_KEY, JSON.stringify({
      area: state.area, selected: state.selected, search: state.search, watch: state.watch,
      explorer: { key: state.explorer.key, area: state.explorer.area, start: state.explorer.start, count: state.explorer.count, order: state.explorer.order },
      analyser: { port: state.analyser.port, speed: state.analyser.speed, frame: state.analyser.frame },
    }));
  } catch {
    // without storage the page starts fresh after a reload
  }
}

export function recall() {
  try {
    const memory = JSON.parse(localStorage.getItem(MEMORY_KEY) ?? "{}");
    if (AREAS.includes(memory.area) || memory.area === "settings") state.area = memory.area;
    state.selected = memory.selected ?? null;
    Object.assign(state.search, memory.search ?? {});
    state.watch = Array.isArray(memory.watch) ? memory.watch : [];
    Object.assign(state.explorer, memory.explorer ?? {});
    Object.assign(state.analyser, memory.analyser ?? {});
  } catch {
    // a broken memory is ignored
  }
}

const views = {};
/** Each page registers its view: `(state) => { title, lead, next, html }`. */
export function registerView(area, view) {
  views[area] = view;
}

let frame = { side: () => "", nextbar: () => "", status: () => "", dialog: () => "" };
export function registerFrame(parts) {
  frame = parts;
}

/** On a phone the areas are a row that scrolls sideways: keep the current one in sight, also after
 * the row is drawn anew. */
function showCurrentArea(side) {
  const row = side.querySelector(".tabs");
  const current = row?.querySelector("[aria-current=page]");
  if (!current || row.scrollWidth <= row.clientWidth) return;
  const left = current.getBoundingClientRect().left - row.getBoundingClientRect().left + row.scrollLeft;
  row.scrollLeft = left - (row.clientWidth - current.offsetWidth) / 2;
}

function keepScroll(root) {
  const kept = {};
  for (const element of root.querySelectorAll("[data-keep]")) kept[element.dataset.keep] = element.scrollTop;
  return kept;
}

function restoreScroll(root, kept) {
  for (const element of root.querySelectorAll("[data-keep]")) {
    if (kept[element.dataset.keep] !== undefined) element.scrollTop = kept[element.dataset.keep];
  }
}

export function render() {
  const view = (views[state.area] ?? views.workspace)(state);
  const focused = document.activeElement?.id;
  const selection = focused && "selectionStart" in document.activeElement
    ? [document.activeElement.selectionStart, document.activeElement.selectionEnd] : null;
  const work = document.getElementById("work");
  const kept = keepScroll(document.body);
  const side = document.getElementById("side");
  side.innerHTML = frame.side(state);
  showCurrentArea(side);
  document.getElementById("nextbar").innerHTML = frame.nextbar(view);
  work.innerHTML = view.html;
  document.getElementById("status").innerHTML = frame.status(state);
  document.getElementById("dialog").innerHTML = frame.dialog(state);
  document.title = `${view.title} · modbusio`;
  restoreScroll(document.body, kept);
  if (focused) {
    const again = document.getElementById(focused);
    if (again && again !== document.activeElement) {
      again.focus({ preventScroll: true });
      if (selection && "setSelectionRange" in again) {
        try {
          again.setSelectionRange(...selection);
        } catch {
          // fields like number inputs have no selection
        }
      }
    }
  }
}

let toastTimer = null;
/** A short message at the bottom; errors stay a little longer. */
export function toast(text, error = false) {
  const element = document.getElementById("toast");
  element.textContent = text;
  element.hidden = false;
  element.classList.toggle("error", error);
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { element.hidden = true; }, error ? 7000 : 3500);
}

/** Runs an action, showing its error as a toast; returns its result or null. */
export async function attempt(action) {
  try {
    return await action();
  } catch (error) {
    toast(error.message, true);
    return null;
  }
}

export function go(area) {
  // the explorer starts with the device chosen in the workspace, unless it shows another one's result
  if (area === "explorer" && state.selected && !state.explorer.result && !state.explorer.busy) state.explorer.key = state.selected;
  state.area = area;
  state.menu = null;
  state.asking = null;
  if (location.hash !== `#/${area}`) history.replaceState(null, "", `#/${area}`);
  remember();
  render();
}

export const areaOfHash = (hash) => {
  const area = hash.replace(/^#\//, "");
  return AREAS.includes(area) || area === "settings" ? area : "workspace";
};

/** The chosen device of the site, or null. */
export const selectedDevice = () => state.site.devices.find((device) => device.key === state.selected) ?? null;

/** A device's name for people: its own, else the product, else its line and unit. */
export function deviceName(device) {
  if (!device) return "";
  return device.name || device.product ? deviceNameOnLine(device) : `${device.line} · ${t("unit")} ${device.unit}`;
}

/** A device's name where its line is shown beside it already (the tree, the device list). */
export function deviceNameOnLine(device) {
  return device.name || device.product || `${t("col.unit")} ${device.unit}`;
}

/** A reading's value for people, or its error in words. */
export function readingText(reading) {
  if (!reading) return "";
  return reading.error ? codeText(reading.error) : reading.text;
}
