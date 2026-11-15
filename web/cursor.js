// The keyboard's place in a list whose rows lead away when chosen (devices, profiles): the arrows
// move a mark, Enter opens the marked row. Lists whose choice stays on screen need none; there the
// chosen row is the place. No imports, so the drawing (store.js) and the keys (keys.js) both use it.

/** The list (`data-keep`) and the row the mark is on. */
const cursor = { list: null, row: null };

/** What tells one row from the others: everything its action reads. */
export const rowIdentity = (row) => JSON.stringify(row.dataset);

export function setCursor(row) {
  cursor.list = row.closest("[data-keep]")?.dataset.keep ?? null;
  cursor.row = rowIdentity(row);
}

/** The marked row of the list on screen, if it is still there. */
export function cursorRow() {
  if (!cursor.list) return null;
  const rows = document.querySelectorAll(`#work .rows[data-keep="${cursor.list}"] .row[data-act]`);
  return [...rows].find((row) => rowIdentity(row) === cursor.row) ?? null;
}

/** Marks the row again after the page was drawn anew. */
export function drawCursor() {
  cursorRow()?.classList.add("cursor");
}
