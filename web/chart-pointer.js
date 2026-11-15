// The pointer on charts: the crosshair, and looking closer by drawing a span (after bacnetio's
// chart-pointer.js).

import { pointerLeft, pointerMoved, spanBegun, spanEnded } from "./chart.js";
import { render } from "./store.js";

document.addEventListener("pointerdown", (event) => {
  if (event.target.closest?.(".chart .draw")) spanBegun(event);
});

document.addEventListener("pointerup", (event) => {
  if (spanEnded(event)) render();
});

document.addEventListener("pointermove", (event) => {
  if (event.target.closest?.(".chart .draw")) pointerMoved(event);
});

document.addEventListener("pointerout", (event) => {
  if (event.target.closest?.(".chart .draw") && !event.relatedTarget?.closest?.(".chart .draw")) pointerLeft();
});
