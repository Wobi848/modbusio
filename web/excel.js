// Links to the Excel files (after bacnetio's excel.js): the server builds them from the same rows
// as the CSV, and the page gives the column heads, the sheet names and a title line in the
// reader's language. The CSV keeps its English heads for scripts; a profile's Excel file too, as
// the import reads it back by them.

import { currentLocale, t } from "./i18n.js";

/** The English heads and sheet names of the server's tables, each with its text key. */
export const HEADS = {
  Name: "col.name", Line: "col.line", "Unit ID": "xlsx.unitId", Profile: "col.profile", Vendor: "col.vendor", Product: "dev.product",
  Version: "dev.revision", Key: "xlsx.key", Group: "pt.group", Area: "ex.area", Address: "col.address", Value: "col.value",
  Unit: "pt.unit", Raw: "xlsx.raw", Error: "xlsx.error", Number: "xlsx.number", Time: "log.time", Device: "col.device", Point: "an.point",
  Before: "xlsx.before", Written: "an.writesShort", "Read back": "xlsx.readBack", Restores: "xlsx.restores",
  Requests: "an.requests", Answers: "an.answers", "No answer": "an.noAnswers", Damaged: "xlsx.damaged", Exceptions: "an.exceptions",
  Writes: "xlsx.writes", "Answer ms min": "xlsx.answerMin", "Answer ms median": "xlsx.answerMedian", "Answer ms 95 %": "xlsx.answer95",
  "Answer ms max": "xlsx.answerMax", "Cycle s median": "xlsx.cycleMedian", Function: "an.functionPart", Count: "xlsx.count",
  Write: "xlsx.write", Result: "an.result", Values: "rec.samples", "Answer ms": "an.answerMs", Reads: "xlsx.reads", Last: "an.last",
  Changes: "xlsx.changes", Min: "xlsx.min", Max: "xlsx.max", "Last seen": "xlsx.lastSeen",
  Devices: "xlsx.devices", Units: "xlsx.units", Exchanges: "an.exchanges", Registers: "an.registers",
};

/** The line above the heads: the site, what is exported, when, and with which modbusio. */
function title(state, what) {
  const made = new Date().toLocaleString(currentLocale());
  return [state.site?.name, ...what, made, `modbusio ${state.info?.version ?? ""}`.trim()].filter(Boolean).join(" · ");
}

/** The address of an Excel file: `path` with the heads, the title and the viewer's time zone. */
export function excelHref(state, path, what = []) {
  const heads = Object.fromEntries(Object.entries(HEADS).map(([english, key]) => [english, t(key)]));
  const query = new URLSearchParams({ heads: JSON.stringify(heads), title: title(state, what), offset: String(-new Date().getTimezoneOffset()) });
  return `${path}${path.includes("?") ? "&" : "?"}${query}`.replaceAll("&", "&amp;");
}
