// Copied from bacnetio (web/columns.js), adapted: modbusio switches its columns by the window's
// width (@media in modbus.css and analyser.css), not by the list's (@container), so the widths set
// here hold from 1000 px of window up; and a column a table ends with that no head names (chips,
// buttons) keeps a fixed track. Its own storage key and tables.
//
// Columns to resize: each table takes its column widths from the CSS by itself. A user who drags
// the border right of a column head sets the widths of that table in this browser; a double-click
// hands them back. Not on a window as narrow as a phone, where the columns stack.

import { t } from "./i18n.js";

const KEY = "modbusio.columns";
/** Below this width of the list the columns stack and keep their own widths. */
const NARROW_LIST = 560;
const NARROWEST = 40;
/** The first column takes what is left, so the table always fits its list. */
const FIRST_LEAST = 120;
const FIRST = `minmax(${FIRST_LEAST}px, 1fr)`;
/** A column the user has not set: a share of what is left. */
const UNSET = "minmax(60px, 1fr)";
/** Where the tables show all their columns (modbus.css, analyser.css). */
const WIDE = "(min-width: 1000px)";
/**
 * The columns of each table from left to right, by the keys of their heads; `{ fixed }` for a
 * column no head sorts, with the track it keeps.
 */
const LAYOUTS = {
  devices: [".rows .row.mdevices", ["name", "line", "unit", "profile", "vendor"]],
  points: [".rows .row.mpoints:not(.group)", ["name", "address", "value", { fixed: "minmax(90px, auto)" }]],
  profiles: [".rows .row.mprofiles", ["name", "vendor", "points", { fixed: "110px" }]],
  profilePoints: [".rows .row.mprofile-points", ["name", "group", "address", "type", "factor", "unit", { fixed: "80px" }]],
  watch: [".rows .row.mwatch", ["name", "device", "value", { fixed: "8rem" }]],
  units: [".rows .row.aunits", ["unit", "requests", "answers", "problems", "answer", "cycle"]],
  exchanges: [".rows .row.aexch", ["time", "unit", "request", "result", "ms"]],
  registers: [".rows .row.aregs", ["address", "point", "reads", "writes", "changes", "last", "range"]],
};

function track(set, column, index) {
  if (typeof column === "object") return column.fixed;
  if (index === 0) return FIRST;
  return set[column] ? `${set[column]}px` : UNSET;
}

/** The border right of a column head, to drag. */
export const columnBorder = (list, key) =>
  `<span class="col-border" data-list="${list}" data-key="${key}" role="separator" aria-orientation="vertical" title="${t("column.resize")}"></span>`;

function load() {
  try {
    return JSON.parse(localStorage.getItem(KEY)) ?? {};
  } catch {
    return {};
  }
}

function save(widths) {
  try {
    localStorage.setItem(KEY, JSON.stringify(widths));
  } catch {
    // without storage the widths last until the page is reloaded
  }
}

let widths = {};

function apply() {
  let style = document.getElementById("column-widths");
  if (!style) {
    style = document.createElement("style");
    style.id = "column-widths";
    document.head.append(style);
  }
  style.textContent = Object.entries(widths).filter(([list]) => LAYOUTS[list]).map(([list, set]) => {
    const [rows, columns] = LAYOUTS[list];
    return `@media ${WIDE} { ${rows} { grid-template-columns: ${columns.map((column, index) => track(set, column, index)).join(" ")}; } }`;
  }).join("\n");
}

let dragging = null;

/** Sets the remembered widths and lets the borders be dragged; once, when the page starts. */
export function installColumns() {
  widths = load();
  apply();

  document.addEventListener("pointerdown", (event) => {
    const border = event.target.closest?.(".col-border");
    if (!border || event.button !== 0) return;
    event.preventDefault();
    const head = border.closest(".row.head");
    if (head.closest(".listwrap").getBoundingClientRect().width <= NARROW_LIST) return;
    // the tracks as laid out, not the heads: a head aligned to the end is narrower than its column
    const tracks = getComputedStyle(head).gridTemplateColumns.split(" ").map((width) => Math.round(parseFloat(width)));
    const keys = [...head.children].filter((cell) => cell.offsetParent !== null).map((cell) => cell.querySelector(".col-border")?.dataset.key);
    const column = keys.indexOf(border.dataset.key);
    // the first column is the flexible one: its border moves the next column instead
    const changed = column === 0 ? 1 : column;
    if (column < 0 || !keys[changed]) return;
    const list = border.dataset.list;
    const set = { ...widths[list] };
    keys.forEach((key, index) => { if (key && index > 0) set[key] = tracks[index]; });
    widths[list] = set;
    dragging = { list, key: keys[changed], start: tracks[changed], room: tracks[0] - FIRST_LEAST, sign: column === 0 ? -1 : 1, from: event.clientX };
    document.documentElement.classList.add("resizing");
  });

  document.addEventListener("pointermove", (event) => {
    if (!dragging) return;
    const { list, key, start, room, sign, from } = dragging;
    // the first column gives the room, down to its least; the row never grows out of its list
    widths[list][key] = Math.min(start + room, Math.max(NARROWEST, start + sign * (event.clientX - from)));
    apply();
  });

  document.addEventListener("pointerup", () => {
    if (!dragging) return;
    dragging = null;
    document.documentElement.classList.remove("resizing");
    save(widths);
  });

  document.addEventListener("dblclick", (event) => {
    const border = event.target.closest?.(".col-border");
    if (!border) return;
    delete widths[border.dataset.list];
    apply();
    save(widths);
  });
}
