// Copied from bacnetio (web/menus.js), unchanged.
// The menus of the toolbars open inside the work area, which cuts off what reaches past it: an open
// menu is placed against the window instead, under its button, and is at most as tall as the room
// down to the window's edge; the rest scrolls inside it.

/** Room kept between a menu and the window's edges. */
const EDGE = 8;
/** The gap between a button and its menu, as in bars.css. */
const GAP = 4;

/** Places the open menu of the work area under its button, inside the window. */
export function placeMenus() {
  for (const menu of document.querySelectorAll("#work .menu-host > .menu")) {
    // a dialog on a phone has its own place already (layout.css)
    if (getComputedStyle(menu).position === "fixed") continue;
    const button = menu.parentElement.querySelector("button").getBoundingClientRect();
    // as wide as where it opened; placed against the window, it would take all the width there is
    const width = Math.min(menu.offsetWidth, window.innerWidth - 2 * EDGE);
    const top = button.bottom + GAP;
    Object.assign(menu.style, { position: "fixed", top: `${top}px`, right: "auto", width: `${width}px`, maxHeight: `${window.innerHeight - top - EDGE}px` });
    const wanted = menu.classList.contains("end") ? button.right - width : button.left;
    menu.style.left = `${Math.max(EDGE, Math.min(wanted, window.innerWidth - width - EDGE))}px`;
  }
}
