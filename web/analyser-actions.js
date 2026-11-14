// What the buttons of the bus analyser do (docs/concept-analyser.md). Listening runs on the server;
// the page asks once a second for what is new.

import { api } from "./api.js";
import { t } from "./i18n.js";
import { heardChanged, lastNumber, takeHeard } from "./analyser.js";
import { profileFrom, registersUnit } from "./analyser-registers.js";
import { fileBase64 } from "./profile-actions.js";
import { attempt, go, pickFile, remember, render, state, toast } from "./store.js";

/** The start's request from the page's choices; an error text when one is missing. */
export function startRequest(analyser) {
  if (!analyser.port) return { error: t("search.needPort") };
  if (analyser.speed === "listen") return { body: { port: analyser.port } };
  return { body: { port: analyser.port, baud: Number(analyser.speed), frame: analyser.frame } };
}

/** A choice list of the page is open: drawing the page anew would close it under the pointer. */
const choosing = () => document.activeElement?.tagName === "SELECT" && document.getElementById("work")?.contains(document.activeElement);

/** What is new on the server; kept unless the view is paused. The page is drawn anew only when
 * something changed, and not while a choice list on it is open. */
export async function pollAnalyser() {
  const held = state.analyser.heard;
  const answer = await api.analyser(lastNumber(held?.exchanges), lastNumber(held?.frames));
  const wasRunning = Boolean(held?.running);
  if (state.analyser.paused && answer.running && held?.started_ms === answer.started_ms) return;
  if (!heardChanged(held, answer)) return;
  state.analyser.heard = takeHeard(held, answer);
  await loadDetails();
  const started = wasRunning !== Boolean(answer.running);
  if (started || (state.area === "analyser" && !choosing())) render();
}

/** The unit in focus (the registers' unit, or the one chosen) with its registers and answer times,
 * and the chosen register's changes. */
async function loadDetails() {
  const analyser = state.analyser;
  if (state.area !== "analyser" || !analyser.heard) return;
  const unit = analyser.view === "registers" ? registersUnit(analyser)
    : analyser.view === "units" && analyser.chosen?.startsWith("u") ? Number(analyser.chosen.slice(1)) : null;
  if (unit === null) return;
  analyser.unitData = await api.analyserUnit(unit);
  if (analyser.view !== "registers" || !analyser.register) return;
  const [area, address] = analyser.register.split(":");
  const answer = await api.analyserRegister(unit, area, address);
  analyser.history = { place: analyser.register, values: answer.history };
}

/** Serial ports for the start, once the page is opened. */
export async function loadPorts() {
  if (!state.info?.serial) return;
  state.serialPorts = (await attempt(api.serialPorts)) ?? [];
  if (!state.analyser.port && state.serialPorts.length) state.analyser.port = state.serialPorts[0].name;
  if (state.area === "analyser") render();
}

/** Fields of the analyser page; true when the page must be drawn anew. */
export function analyserField(element) {
  state.analyser[element.dataset.analyser] = element.value;
  // chosen: the list is closed, the live view may be drawn again
  element.blur();
  if (element.dataset.analyser === "registersUnit") {
    state.analyser.register = null;
    attempt(async () => { await loadDetails(); render(); });
  }
  if (element.dataset.analyser !== "unitFilter") remember();
  return true;
}

export const analyserActions = {
  "analyser-start": async () => {
    if (state.analyser.busy) return;
    const request = startRequest(state.analyser);
    if (request.error) { toast(request.error, true); return; }
    state.analyser.busy = true;
    state.busy = request.body.baud ? t("an.starting") : t("an.findingSpeed");
    render();
    const answer = await attempt(() => api.analyserStart(request.body));
    state.analyser.busy = false;
    state.busy = null;
    if (answer) {
      state.analyser.heard = takeHeard(null, answer);
      state.analyser.chosen = null;
      state.analyser.paused = false;
    }
    render();
  },
  "analyser-stop": async () => {
    const answer = await attempt(api.analyserStop);
    if (answer) state.analyser.heard = takeHeard(state.analyser.heard, answer);
    state.analyser.paused = false;
    render();
  },
  "analyser-clear": async () => {
    const answer = await attempt(api.analyserClear);
    if (answer) state.analyser.heard = takeHeard(null, answer);
    state.analyser.chosen = null;
    render();
  },
  // asked for: say what was found, also when the list did not change
  "analyser-ports": async () => {
    await loadPorts();
    const count = state.serialPorts.length;
    toast(count === 1 ? t("an.onePortFound") : count ? t("an.portsFound", { n: count }) : t("an.noPortFound"));
  },
  "analyser-open-file": () => pickFile(".pcapng,.pcap,.cap", async (file) => {
    const content = await attempt(() => fileBase64(file));
    if (!content) return;
    const answer = await attempt(() => api.analyserOpen(file.name, content));
    if (!answer) return;
    Object.assign(state.analyser, { heard: takeHeard(null, answer), chosen: null, register: null, unitData: null, view: "units" });
    render();
  }),
  "analyser-draft-profile": () => {
    const data = state.analyser.unitData;
    if (!data?.registers.length) return;
    state.profileId = null;
    state.profileDraft = profileFrom(data, t("an.draftName", { unit: data.unit }));
    state.profileDirty = true;
    state.pointIndex = null;
    go("profiles");
  },
  "analyser-pause": () => { state.analyser.paused = !state.analyser.paused; render(); },
  "analyser-view": async (element) => {
    Object.assign(state.analyser, { view: element.dataset.view, chosen: null, register: null });
    render();
    await loadDetails();
    render();
  },
  "analyser-choose": async (element) => {
    state.analyser.chosen = element.dataset.chosen || null;
    render();
    await loadDetails();
    render();
  },
  "analyser-back": () => { Object.assign(state.analyser, { chosen: null, register: null }); render(); },
  "analyser-register": async (element) => {
    state.analyser.register = element.dataset.place;
    render();
    await loadDetails();
    render();
  },
  "analyser-unit-registers": async (element) => {
    Object.assign(state.analyser, { view: "registers", registersUnit: element.dataset.unit, chosen: null, register: null });
    render();
    await loadDetails();
    render();
  },
  "analyser-check": (element) => { state.analyser[element.dataset.field] = element.checked; render(); },
  "analyser-unit-exchanges": (element) => {
    Object.assign(state.analyser, { view: "exchanges", unitFilter: element.dataset.unit, chosen: null });
    render();
  },
  "analyser-add-unit": async (element) => {
    const heard = state.analyser.heard;
    const [baud, frame] = heard.settings.split(" ");
    const key = `serial:${heard.port}:${baud}:${frame}/${element.dataset.unit}`;
    const answer = await attempt(() => api.addDevice(key, ""));
    if (!answer) return;
    const { check, ...site } = answer;
    state.site = site;
    toast(t("an.added", { unit: element.dataset.unit }));
    render();
  },
};
