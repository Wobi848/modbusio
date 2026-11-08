// Whether a new version is out: asked at most once a day and only while the user lets it; the
// answer is kept in this browser until the next check. Copied from bacnetio and adapted.

import { api } from "./api.js";
import { render, state } from "./store.js";

const RESULT_KEY = "modbusio.update";
/** Whether the page may ask for a new version; on unless switched off. */
const CHOICE_KEY = "modbusio.updateCheck";
const DAY_MS = 24 * 60 * 60 * 1000;

function keep(key, value) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    // without storage the page asks again at the next start
  }
}

function lastResult() {
  try {
    return JSON.parse(localStorage.getItem(RESULT_KEY));
  } catch {
    return null;
  }
}

export function updateCheckOn() {
  try {
    return localStorage.getItem(CHOICE_KEY) !== "off";
  } catch {
    return true;
  }
}

/** Asks the server unless a check of the last day by this version is kept, or `now` is set. */
export async function checkForUpdate(now = false) {
  if (!updateCheckOn()) return;
  const version = state.info?.version;
  const kept = lastResult();
  // after an update the kept answer is the old version's: it would offer the version now running
  if (!now && kept && kept.current === version && Date.now() - kept.at < DAY_MS) {
    state.update = kept;
    render();
    return;
  }
  state.update = { ...state.update, checking: true };
  render();
  try {
    const answer = await api.update();
    state.update = { at: Date.now(), current: version, newer: answer.newer, version: answer.latest?.version ?? null, url: answer.latest?.url ?? null };
  } catch (error) {
    state.update = { at: Date.now(), current: version, error: error.message };
  }
  keep(RESULT_KEY, JSON.stringify(state.update));
  render();
}

export function setUpdateCheck(on) {
  keep(CHOICE_KEY, on ? null : "off");
  if (on) {
    checkForUpdate(true);
    return;
  }
  keep(RESULT_KEY, null);
  state.update = null;
  render();
}
