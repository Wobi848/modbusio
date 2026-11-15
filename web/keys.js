// The keyboard, after bacnetio's keys.js: Escape closes, arrows move through lists and menus,
// digits choose the areas, Tab stays inside a dialog.

import { actions } from "./actions.js";
import { cursorRow, drawCursor, setCursor } from "./cursor.js";
import { printable } from "./print.js";
import { attempt, go, render, state } from "./store.js";

/** Rows whose choice replaces the list: the arrows mark, Enter opens. */
const LEADS_AWAY = new Set(["select-device", "open-profile", "found-open"]);
const PAGE_ROWS = 10;
const MOVE_KEYS = { ArrowDown: 1, ArrowUp: -1, PageDown: PAGE_ROWS, PageUp: -PAGE_ROWS, Home: -Infinity, End: Infinity };

/** The list the keys move in: the one with a chosen or marked row, else the first with rows. */
function listRows() {
  const lists = [...document.querySelectorAll("#work .rows[role=listbox]")]
    .map((list) => [...list.querySelectorAll(".row[data-act]")])
    .filter((rows) => rows.length);
  return lists.find((rows) => rows.some((row) => row.matches(".active, .cursor"))) ?? lists[0] ?? [];
}

/** Moves the chosen (or marked) row of the list on screen by that many rows. */
function moveSelection(step) {
  const rows = listRows();
  if (!rows.length) return;
  const index = rows.findIndex((row) => row.matches(".active, .cursor"));
  const next = rows[Math.max(0, Math.min(rows.length - 1, index + step))];
  if (LEADS_AWAY.has(next.dataset.act)) {
    rows[index]?.classList.remove("cursor");
    setCursor(next);
    drawCursor();
  } else {
    // draws the page anew before it waits for anything, so the chosen row is there to scroll to
    attempt(() => actions[next.dataset.act](next));
  }
  document.querySelector("#work .rows .row.active, #work .rows .row.cursor")?.scrollIntoView({ block: "nearest" });
}

/** Up and down move through an open list menu. */
function moveInMenu(event) {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return false;
  // an open list menu takes the arrows wherever the focus is, as long as nothing is typed
  const menu = document.querySelector(".menu[role=listbox], .menu[role=menu]");
  if (!menu || event.target.closest?.("input, select, textarea")) return false;
  const items = [...menu.querySelectorAll("button, a")];
  const at = items.indexOf(document.activeElement);
  const next = event.key === "ArrowDown" ? Math.min(at + 1, items.length - 1) : Math.max(at - 1, 0);
  items[next]?.focus();
  return true;
}

/** Tab stays inside an open dialog. */
function keepInDialog(event) {
  const box = document.querySelector(".modal-box");
  if (!box || event.key !== "Tab") return false;
  const fields = [...box.querySelectorAll("button, select, input, a[href], summary")].filter((field) => !field.disabled && field.offsetParent);
  const [first, last] = [fields[0], fields[fields.length - 1]];
  if (!box.contains(document.activeElement)) first?.focus();
  else if (event.shiftKey && document.activeElement === first) last.focus();
  else if (!event.shiftKey && document.activeElement === last) first.focus();
  else return false;
  return true;
}

/** Closes what lies on top; false when nothing was open. */
function closeTopmost() {
  if (state.printing) state.printing = false;
  else if (state.compare) state.compare = null;
  else if (state.importing) state.importing = null;
  else if (state.menu) state.menu = null;
  else if (state.asking) state.asking = null;
  else if (state.selectedPoint) state.selectedPoint = null;
  else return false;
  return true;
}

/** A key that belongs to the control it was pressed on, not to the list. */
const ownKeys = (target) => Boolean(target.closest?.("button, a, summary, [role=separator], [tabindex]:not(.rows)"));

document.addEventListener("keydown", (event) => {
  const typing = event.target.closest?.("input, select, textarea");
  if (event.key === "Escape") {
    if (closeTopmost()) render();
    return;
  }
  if (moveInMenu(event) || keepInDialog(event)) {
    event.preventDefault();
    return;
  }
  if (event.key === "Enter" && event.target.id === "write-value") {
    attempt(() => actions[state.asking?.kind === "raw-write" ? "raw-write-check" : "write-check"]());
    return;
  }
  // the browser would print the list with modbusio around it: the print view is made for paper
  if ((event.ctrlKey || event.metaKey) && event.key === "p" && printable(state) && !state.printing) {
    event.preventDefault();
    state.printing = true;
    render();
    window.print();
    return;
  }
  if (typing || event.ctrlKey || event.metaKey || event.altKey) return;
  const number = Number(event.key);
  if (number >= 1 && number <= state.areas.length) { go(state.areas[number - 1]); return; }
  if (number === state.areas.length + 1) { go("settings"); return; }
  if (event.key === "?") { go("help"); return; }
  // / searches the whole site, f the list on screen
  const field = { "/": "site-search", f: "point-filter" }[event.key];
  if (field && document.getElementById(field)) {
    event.preventDefault();
    document.getElementById(field).focus();
    return;
  }
  // a dialog or a question lies over the list
  if (state.menu === "search" || state.importing || ownKeys(event.target)) return;
  if (MOVE_KEYS[event.key]) {
    event.preventDefault();
    moveSelection(MOVE_KEYS[event.key]);
    return;
  }
  const marked = event.key === "Enter" ? cursorRow() : null;
  if (marked) attempt(() => actions[marked.dataset.act](marked));
});
