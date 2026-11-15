// The bus analyser's notes above its tables: folded away or as tall as their lower border is
// dragged, kept in this browser. Someone who knows the notes wants the room for the tables.

import { t } from "./i18n.js";

const KEY = "modbusio.analyserNotes";
/** What the arrow keys move the border by. */
const STEP = 16;
const LEAST = 40;
/** The most of the list's height the notes may take, so the table under them keeps its room. */
const MOST_SHARE = 0.7;

function load() {
  try {
    return JSON.parse(localStorage.getItem(KEY)) ?? {};
  } catch {
    return {};
  }
}

/** What the user set: `hidden`, and `height` in px once the border was dragged. */
let choice = {};

function save() {
  try {
    localStorage.setItem(KEY, JSON.stringify(choice));
  } catch {
    // without storage the choice lasts until the page is reloaded
  }
}

/** The notes with their head to fold them and their border to drag; `strips` is their HTML. */
export function notesHtml(strips, count) {
  if (!strips) return "";
  const shown = !choice.hidden;
  const head = `<button type="button" id="notes-toggle" class="quiet notes-toggle" data-act="analyser-notes-toggle" aria-expanded="${shown}">${shown ? "▾" : "▸"} ${t(count === 1 ? "an.notesOne" : "an.notes", { n: count })}</button>`;
  if (!shown) return `<div class="notes-head">${head}</div>`;
  const size = choice.height ? ` style="height:${choice.height}px;max-height:none"` : "";
  return `<div class="notes-head">${head}</div><div class="strips analyser-notes" data-keep="analyser-notes"${size}>${strips}</div>
    <div class="resize-row" id="resize-notes" role="separator" aria-orientation="horizontal" tabindex="0" title="${t("an.notesResize")}" aria-label="${t("an.notesResize")}"></div>`;
}

export function toggleNotes() {
  choice.hidden = !choice.hidden;
  save();
}

function setHeight(height) {
  const notes = document.querySelector(".analyser-notes");
  if (!notes) return;
  const most = notes.closest(".listwrap").getBoundingClientRect().height * MOST_SHARE;
  choice.height = Math.round(Math.max(LEAST, Math.min(height, most)));
  Object.assign(notes.style, { height: `${choice.height}px`, maxHeight: "none" });
}

const heightNow = () => document.querySelector(".analyser-notes")?.getBoundingClientRect().height ?? 0;

let dragging = null;

/** Takes the remembered choice and lets the border be dragged; once, when the page starts. */
export function installAnalyserNotes() {
  choice = load();
  document.addEventListener("pointerdown", (event) => {
    if (!event.target.closest?.(".resize-row") || event.button !== 0) return;
    event.preventDefault();
    dragging = { start: heightNow(), from: event.clientY };
    document.documentElement.classList.add("resizing-row");
  });
  document.addEventListener("pointermove", (event) => {
    if (dragging) setHeight(dragging.start + event.clientY - dragging.from);
  });
  document.addEventListener("pointerup", () => {
    if (!dragging) return;
    dragging = null;
    document.documentElement.classList.remove("resizing-row");
    save();
  });
  document.addEventListener("dblclick", (event) => {
    if (!event.target.closest?.(".resize-row")) return;
    delete choice.height;
    save();
    Object.assign(document.querySelector(".analyser-notes").style, { height: "", maxHeight: "" });
  });
  document.addEventListener("keydown", (event) => {
    if (!event.target.closest?.(".resize-row") || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
    event.preventDefault();
    setHeight(heightNow() + (event.key === "ArrowDown" ? STEP : -STEP));
    save();
  });
}
