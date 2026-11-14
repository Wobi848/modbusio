// Copied from bacnetio (web/panels.js), with its own storage key.
//
// Panels to resize: sidebar, tree and details take their width from the window by themselves
// (layout.css). A user who drags a border, or moves it with the arrow keys, sets that panel's width
// in this browser; a double-click hands it back to the automatic width.

import { t } from "./i18n.js";

const KEY = "modbusio.panels";
/** What the arrow keys move a border by. */
const STEP = 16;
/** The list in the middle never gets narrower than this while dragging. */
const LIST_MIN = 360;
/** Per panel: the CSS variable it sets, its limits, and whether it grows to the left. */
const PANELS = {
  side: { variable: "--side-width", min: 180, max: 360, selector: ".side" },
  tree: { variable: "--tree-width", min: 160, max: 480, selector: ".tree" },
  detail: { variable: "--detail-width", min: 280, max: 640, selector: ".detail", leftward: true },
};

function loadWidths() {
  try {
    return JSON.parse(localStorage.getItem(KEY)) ?? {};
  } catch {
    return {};
  }
}

function saveWidths(widths) {
  try {
    localStorage.setItem(KEY, JSON.stringify(widths));
  } catch {
    // without storage the width lasts until the page is reloaded
  }
}

/** The widths the user set, per panel; the others follow the window. */
let widths = {};

function apply(panel) {
  const style = document.documentElement.style;
  if (widths[panel]) style.setProperty(PANELS[panel].variable, `${widths[panel]}px`);
  else style.removeProperty(PANELS[panel].variable);
}

/** The border of a panel, to drag. `panel` is one of side, tree, detail. */
export function resizeHandle(panel) {
  // an id, so the keyboard focus stays on the border when the page is drawn again
  return `<div class="resize" id="resize-${panel}" data-panel="${panel}" role="separator" aria-orientation="vertical" tabindex="0" title="${t("panel.resize")}" aria-label="${t("panel.resize")}"></div>`;
}

/** The two borders of the list in the workspace: to the tree on its left, to the details on its right. */
export const listBorders = () => resizeHandle("tree") + resizeHandle("detail");

/** The border of a list without a tree, to the details on its right (modbusio's own pages). */
export const detailBorder = () => resizeHandle("detail");

/** The widest a panel may be now: within its limits, and the list keeps its room. */
function widest(panel, current) {
  const list = document.querySelector(".listwrap");
  const room = list ? list.getBoundingClientRect().width - LIST_MIN : Infinity;
  return Math.min(PANELS[panel].max, current + Math.max(room, 0));
}

function setWidth(panel, width) {
  const element = document.querySelector(PANELS[panel].selector);
  if (!element) return;
  const current = element.getBoundingClientRect().width;
  widths[panel] = Math.round(Math.max(PANELS[panel].min, Math.min(width, widest(panel, current))));
  apply(panel);
}

let dragging = null;

/** Sets the remembered widths and lets the borders be dragged; once, when the page starts. */
export function installPanels() {
  widths = loadWidths();
  Object.keys(PANELS).forEach(apply);

  document.addEventListener("pointerdown", (event) => {
    const handle = event.target.closest?.(".resize");
    if (!handle || event.button !== 0) return;
    event.preventDefault();
    const panel = handle.dataset.panel;
    const start = document.querySelector(PANELS[panel].selector).getBoundingClientRect().width;
    dragging = { panel, start, from: event.clientX };
    document.documentElement.classList.add("resizing");
  });

  document.addEventListener("pointermove", (event) => {
    if (!dragging) return;
    const { panel, start, from } = dragging;
    const moved = event.clientX - from;
    setWidth(panel, start + (PANELS[panel].leftward ? -moved : moved));
  });

  document.addEventListener("pointerup", () => {
    if (!dragging) return;
    dragging = null;
    document.documentElement.classList.remove("resizing");
    saveWidths(widths);
  });

  document.addEventListener("dblclick", (event) => {
    const handle = event.target.closest?.(".resize");
    if (!handle) return;
    delete widths[handle.dataset.panel];
    apply(handle.dataset.panel);
    saveWidths(widths);
  });

  document.addEventListener("keydown", (event) => {
    const handle = event.target.closest?.(".resize");
    if (!handle || (event.key !== "ArrowLeft" && event.key !== "ArrowRight")) return;
    event.preventDefault();
    const panel = handle.dataset.panel;
    const current = document.querySelector(PANELS[panel].selector).getBoundingClientRect().width;
    const wider = (event.key === "ArrowRight") !== Boolean(PANELS[panel].leftward);
    setWidth(panel, current + (wider ? STEP : -STEP));
    saveWidths(widths);
  });
}
