// What the buttons of the bus analyser do (docs/concept-analyser.md). Listening runs on the server;
// the page asks once a second for what is new.

import { api } from "./api.js";
import { t } from "./i18n.js";
import { lastNumber, takeHeard } from "./analyser.js";
import { attempt, remember, render, state, toast } from "./store.js";

/** The start's request from the page's choices; an error text when one is missing. */
export function startRequest(analyser) {
  if (!analyser.port) return { error: t("search.needPort") };
  if (analyser.speed === "listen") return { body: { port: analyser.port } };
  return { body: { port: analyser.port, baud: Number(analyser.speed), frame: analyser.frame } };
}

/** What is new on the server; kept unless the view is paused. */
export async function pollAnalyser() {
  const held = state.analyser.heard;
  const answer = await api.analyser(lastNumber(held?.exchanges), lastNumber(held?.frames));
  const wasRunning = Boolean(held?.running);
  if (state.analyser.paused && answer.running && held?.started_ms === answer.started_ms) return;
  state.analyser.heard = takeHeard(held, answer);
  if (state.area === "analyser" || wasRunning !== Boolean(answer.running)) render();
}

/** Serial ports for the start, once the page is opened. */
export async function loadPorts() {
  if (!state.info?.serial) return;
  state.serialPorts = (await attempt(api.serialPorts)) ?? [];
  if (!state.analyser.port && state.serialPorts.length) state.analyser.port = state.serialPorts[0].name;
  render();
}

/** Fields of the analyser page; true when the page must be drawn anew. */
export function analyserField(element) {
  state.analyser[element.dataset.analyser] = element.value;
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
  "analyser-ports": () => loadPorts(),
  "analyser-pause": () => { state.analyser.paused = !state.analyser.paused; render(); },
  "analyser-view": (element) => { state.analyser.view = element.dataset.view; state.analyser.chosen = null; render(); },
  "analyser-choose": (element) => { state.analyser.chosen = element.dataset.chosen || null; render(); },
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
