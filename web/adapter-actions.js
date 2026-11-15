// The network adapters and serial ports the search can use, read again while its settings are
// open (after bacnetio's adapter-actions.js): a VPN that came up, a cable or a USB adapter plugged
// in shows without opening the settings again.

import { api } from "./api.js";
import { render, state } from "./store.js";

const POLL_MS = 5000;

/** Reads both lists and drops a choice that is gone; true when something changed. */
export async function loadSearchChoices() {
  const [adapters, ports] = await Promise.all([api.adapters(), state.info?.serial ? api.serialPorts() : []]);
  const before = JSON.stringify([state.adapters, state.serialPorts, state.search.interface, state.search.serialPort]);
  state.adapters = adapters;
  state.serialPorts = ports;
  const search = state.search;
  // an adapter that is gone hands the choice back to the default one
  if (search.interface && !adapters.some((adapter) => adapter.ip === search.interface)) search.interface = "";
  if (!ports.some((port) => port.name === search.serialPort)) search.serialPort = ports[0]?.name ?? "";
  return before !== JSON.stringify([state.adapters, state.serialPorts, search.interface, search.serialPort]);
}

async function refresh() {
  if (state.menu !== "search" || document.visibilityState !== "visible") return;
  // an open list of choices would close under the pointer; the next round catches up
  if (["search-interface", "search-port-serial"].includes(document.activeElement?.id)) return;
  try {
    if (await loadSearchChoices()) render();
  } catch {
    // modbusio was closed; the next round tries again
  }
}

/** Every few seconds while the search settings are open, and at once when the page comes back. */
export function watchSearchChoices() {
  setInterval(refresh, POLL_MS);
  window.addEventListener("focus", refresh);
  document.addEventListener("visibilitychange", refresh);
}
