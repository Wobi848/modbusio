// Switching watch or record on many points at once: press on one switch in the point list and
// drag over the other rows (after bacnetio's drag-toggle.js). Mouse only; touch scrolls, and a key
// or a tap stays a plain click (app.js).

import { isRecorded, isWatched, setRecorded, setWatched } from "./actions.js";
import { render, state } from "./store.js";

const SWITCHES = {
  "quick-watch": { isOn: isWatched, set: setWatched },
  "quick-record": { isOn: isRecorded, set: setRecorded },
};

let drag = null;
let swallowClick = false;

/** The row under the pointer takes the state of the first switch, once per drag. */
function pass(row) {
  const point = row?.dataset.point;
  if (!point || drag.passed.has(point)) return;
  drag.passed.add(point);
  // taken now: the drag may be over before the switches before this one have answered
  const { act, on } = drag;
  const device = state.selected;
  if (SWITCHES[act].isOn(device, point) === on) return;
  // one after the other: each answer of the recording is the whole of it
  drag.last = drag.last.then(() => SWITCHES[act].set(device, point, on)).then(render);
}

export function installDragToggle() {
  document.addEventListener("pointerdown", (event) => {
    if (event.pointerType !== "mouse" || event.button !== 0) return;
    const button = event.target.closest?.("button.quick[data-act]");
    if (!button || !SWITCHES[button.dataset.act]) return;
    // no text selection while dragging
    event.preventDefault();
    drag = { act: button.dataset.act, on: button.getAttribute("aria-pressed") !== "true", passed: new Set(), last: Promise.resolve() };
    swallowClick = true;
    pass(button.closest(".row[data-point]"));
  });
  document.addEventListener("pointermove", (event) => {
    // the row at the pointer's place: the row the press began on is drawn anew meanwhile, and a
    // browser may go on naming the old one as the target
    if (drag) pass(document.elementFromPoint(event.clientX, event.clientY)?.closest(".row[data-point]"));
  });
  document.addEventListener("pointerup", () => {
    if (!drag) return;
    drag = null;
    // the click that follows the release was already handled by the press
    setTimeout(() => { swallowClick = false; });
  });
  document.addEventListener("click", (event) => {
    if (!swallowClick) return;
    swallowClick = false;
    event.stopPropagation();
    event.preventDefault();
  }, true);
}
