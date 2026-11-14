// Starts the page, hands clicks and fields to the actions, and keeps live data fresh: the search's
// progress, live values, the watch list, the recording, the capture.

import { api } from "./api.js";
import { actions, loadJournal, loadSite, onChange, onField, pollScan, readDevice } from "./actions.js";
import { loadPorts, pollAnalyser } from "./analyser-actions.js";
import { dialogHtml, nextbarHtml, sideHtml, statusHtml } from "./shell.js";
import { importHtml } from "./profiles.js";
import { AREAS, areaOfHash, attempt, go, recall, registerFrame, render, selectedDevice, state } from "./store.js";
import "./workspace.js";
import "./explorer.js";
import "./pages.js";
import "./analyser.js";
import { heldUntil, sameRecording, withNewSamples } from "./recording.js";
import { checkForUpdate } from "./update.js";
import { installPanels } from "./panels.js";
import { installColumns } from "./columns.js";

state.areas = AREAS;

/** The bus analyser needs a serial port, which Android gives no program (concept section 10). */
function areasOf(info) {
  return AREAS.filter((area) => area !== "analyser" || info?.serial);
}
registerFrame({
  side: sideHtml,
  nextbar: nextbarHtml,
  status: statusHtml,
  dialog: (current) => dialogHtml(current) + importHtml(current),
});

document.addEventListener("click", (event) => {
  const target = event.target;
  if (state.menu && state.menu !== "search" && !target.closest(".menu, .ask, [data-act$='-menu'], .add-device")) {
    state.menu = null;
    render();
  }
  const element = target.closest("[data-act]");
  if (!element || element.disabled) return;
  // a checkbox acts through its change, not through the click on its label
  if (element.type === "checkbox") return;
  const action = actions[element.dataset.act];
  if (action) {
    event.preventDefault();
    attempt(() => action(element));
  }
});

document.addEventListener("change", (event) => {
  const element = event.target;
  if (element.type === "checkbox" && element.dataset.act) {
    attempt(() => actions[element.dataset.act]?.(element));
    return;
  }
  // a typed field took its value with each input already; its change comes while it loses the
  // focus, and redrawing then would remove it in the middle of that (the browser throws) and can
  // swallow the click that moved the focus
  const typed = element.tagName !== "SELECT" && element.type !== "checkbox";
  if (!typed && onField(element)) render();
  attempt(() => onChange(element));
});

document.addEventListener("input", (event) => {
  const element = event.target;
  if (element.tagName === "SELECT" || element.type === "checkbox") return;
  if (onField(element)) render();
});

/** Keys as in bacnetio: 1 to n the areas, the next one the settings, Escape closes, / the point filter. */
document.addEventListener("keydown", (event) => {
  const typing = event.target.closest("input, select, textarea");
  if (event.key === "Escape") {
    if (state.importing) state.importing = null;
    else if (state.menu) state.menu = null;
    else if (state.asking) state.asking = null;
    else if (state.selectedPoint) state.selectedPoint = null;
    else return;
    render();
    return;
  }
  if (event.key === "Enter" && event.target.id === "write-value") {
    attempt(() => actions[state.asking?.kind === "raw-write" ? "raw-write-check" : "write-check"]());
    return;
  }
  if (typing || event.ctrlKey || event.metaKey || event.altKey) return;
  const number = Number(event.key);
  if (number >= 1 && number <= state.areas.length) go(state.areas[number - 1]);
  else if (number === state.areas.length + 1) go("settings");
  else if (event.key === "/" && document.getElementById("point-filter")) {
    event.preventDefault();
    document.getElementById("point-filter").focus();
  }
});

window.addEventListener("hashchange", () => {
  const area = areaOfHash(location.hash);
  if (area !== state.area) go(area);
});

/**
 * The Android app's back key (android/…/MainActivity.kt): one step back in the page, or the
 * answer that there is none, with the values still written so the app can say so.
 */
window.modbusioBack = () => {
  let stepped = true;
  if (state.importing) state.importing = null;
  else if (state.menu) state.menu = null;
  else if (state.asking) state.asking = null;
  else if (state.selectedPoint || state.deviceDetailOpen) {
    state.selectedPoint = null;
    state.deviceDetailOpen = false;
  } else if (state.area === "workspace" && state.selected) state.selected = null;
  else if (state.area !== "workspace") go("workspace");
  else stepped = false;
  if (stepped) render();
  return JSON.stringify({ stepped, held: state.journal.standing });
};

window.addEventListener("beforeunload", (event) => {
  // values written and not put back: leaving should be a decision
  if (state.journal.standing) event.preventDefault();
});

/** The recording with what is new since `held`; all of it where the page's samples do not fit. */
async function latestRecording(held) {
  return withNewSamples(held, await api.recording(heldUntil(held))) ?? api.recording();
}

let ticking = false;
let ticks = 0;
/** Once a second: what the page shows live. Each part only while it is on screen or running. */
async function tick() {
  if (ticking) return;
  ticking = true;
  ticks += 1;
  try {
    const device = selectedDevice();
    if (state.area === "workspace" && state.live && device && state.readings[device.key] && ticks % 2 === 0) {
      await readDevice(true);
    }
    if (state.area === "watch" && state.watch.length && ticks % 2 === 0) {
      const answers = await api.values(state.watch);
      state.watchValues = Object.fromEntries(answers.map((answer) => [`${answer.key}|${answer.point}`, answer]));
      render();
    }
    if ((state.area === "recording" || state.recording.running) && ticks % 2 === 0) {
      const held = state.recording;
      const latest = await latestRecording(held);
      // a button's answer that came meanwhile is newer than this one
      if (state.recording === held) {
        state.recording = latest;
        if (state.area === "recording" ? !sameRecording(held, latest) : held.running !== latest.running) render();
      }
    }
    if (state.area === "analyser" || state.analyser.heard?.running) await pollAnalyser();
    if (state.capture.running && ticks % 2 === 0) {
      state.capture = await api.capture();
      render();
    }
  } catch {
    // the next tick tries again; a stopped modbusio shows in the actions' messages
  } finally {
    ticking = false;
  }
}

async function start() {
  recall();
  installPanels();
  installColumns();
  const hashed = areaOfHash(location.hash);
  if (location.hash) state.area = hashed;
  render();
  const info = await attempt(api.info);
  state.info = info;
  state.areas = areasOf(info);
  if (!state.areas.includes(state.area) && state.area !== "settings") state.area = "workspace";
  await attempt(loadSite);
  await attempt(loadJournal);
  const [session, recording, capture, scan] = await Promise.all([attempt(api.session), attempt(api.recording), attempt(api.capture), attempt(api.scan)]);
  state.unlocked = session?.unlocked ?? false;
  if (recording) state.recording = recording;
  if (capture) state.capture = capture;
  if (scan) state.scan = scan;
  if (!state.explorer.key && state.site.devices.length) state.explorer.key = state.site.devices[0].key;
  render();
  if (scan && !scan.finished) attempt(pollScan);
  await attempt(pollAnalyser);
  attempt(loadPorts);
  setInterval(tick, 1000);
  checkForUpdate();
}

start();
