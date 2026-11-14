// Copied from bacnetio (web/sorting.js) unchanged.
//
// Sorting a list by a column: numbers as numbers, addresses and names with numbers in them in their
// natural order, empty cells last. Works on values a column's getter gives.

import { columnBorder } from "./columns.js";

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

/** The number at the start of a text such as "21.4 °C", or null. */
function leadingNumber(value) {
  if (typeof value === "number") return value;
  const number = /^\s*(-?\d+(\.\d+)?)/.exec(value ?? "");
  return number ? Number(number[1]) : null;
}

const isEmpty = (value) => value === null || value === undefined || value === "";

function compare(a, b) {
  const [first, second] = [leadingNumber(a), leadingNumber(b)];
  if (first !== null && second !== null && first !== second) return first - second;
  return collator.compare(String(a), String(b));
}

/** A copy sorted by `sort` ({ key, descending }), or the list itself when no column is chosen. */
export function sortRows(rows, sort, columns) {
  const column = sort && columns[sort.key];
  if (!column) return rows;
  const direction = sort.descending ? -1 : 1;
  return [...rows].sort((a, b) => {
    const [first, second] = [column(a), column(b)];
    // empty cells stay last in both directions
    if (isEmpty(first) || isEmpty(second)) return isEmpty(first) - isEmpty(second);
    return direction * compare(first, second);
  });
}

/** Ascending, then descending, then back to the device's own order. */
export function nextSort(sort, key) {
  if (sort?.key !== key) return { key, descending: false };
  return sort.descending ? null : { key, descending: true };
}

/** A column head that sorts its list, saying how it is sorted now. */
export function sortHead(label, list, key, sort, className = "") {
  const mark = sort?.key === key ? (sort.descending ? "↓" : "↑") : "↕";
  const aria = sort?.key === key ? (sort.descending ? "descending" : "ascending") : "none";
  return `<span class="${className}" role="columnheader" aria-sort="${aria}"><button type="button" class="sort${sort?.key === key ? " on" : ""}" data-act="sort" data-list="${list}" data-key="${key}">${label} <span aria-hidden="true">${mark}</span></button>${columnBorder(list, key)}</span>`;
}
