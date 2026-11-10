// UI logic without a browser: `node --test tests/web/*.test.js`. A stub stands in for the document.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";

globalThis.document = { documentElement: {} };
const { LANGUAGE_FILES, LANGUAGES, codeText, counted, errorText, noteText, setLanguage, t } = await import("../../web/i18n.js");
const { searchRequest } = await import("../../web/actions.js");
const { filterPoints, groupPoints, lines, viewWorkspace } = await import("../../web/workspace.js");
const { parseStates, statesText } = await import("../../web/profiles.js");
const { draftProblem, pointFromExplorer, typedNumber, unsavedChanges } = await import("../../web/profile-actions.js");
const { bounds, byUnit, pathData } = await import("../../web/chart.js");
const { deviceName, deviceNameOnLine, state } = await import("../../web/store.js");
const { pairHtml } = await import("../../web/explorer.js");
const { listenedHtml } = await import("../../web/shell.js");

const webDir = new URL("../../web/", import.meta.url);
const sources = readdirSync(webDir).filter((name) => name.endsWith(".js")).map((name) => readFileSync(new URL(name, webDir), "utf8"));

test("every language has every key, note, error and code of English, with the same placeholders", () => {
  const english = LANGUAGE_FILES.en;
  const placeholders = (text) => [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort().join(",");
  for (const [code, file] of Object.entries(LANGUAGE_FILES)) {
    for (const part of ["texts", "codes"]) {
      for (const [key, text] of Object.entries(english[part])) {
        assert.ok(key in file[part], `${code}: ${part}.${key} is missing`);
        assert.equal(placeholders(file[part][key]), placeholders(text), `${code}: ${part}.${key} has other placeholders`);
      }
    }
  }
});

test("every key the interface uses exists in English", () => {
  const used = new Set(sources.flatMap((source) => [...source.matchAll(/\bt\("([^"]+)"/g)].map((match) => match[1])));
  for (const key of used) assert.ok(key in LANGUAGE_FILES.en.texts, `${key} is used but not in en.js`);
  for (const prefix of ["tab.workspace", "tab.explorer", "tab.profiles", "tab.watch", "tab.recording", "tab.changed", "phase.hosts", "phase.units", "phase.settings", "phase.identify",
    "area.holding_register", "area.input_register", "area.coil", "area.discrete_input", "conv.zero_based", "conv.one_based", "conv.reference",
    "search.lead.tcp", "search.lead.gateway", "search.lead.serial"]) {
    assert.ok(prefix in LANGUAGE_FILES.en.texts, `${prefix} is built in code but not in en.js`);
  }
});

test("every note the server sends has a German text with its values", () => {
  const diagnosis = readFileSync(new URL("../../src/diagnosis.rs", import.meta.url), "utf8");
  const variants = [...diagnosis.matchAll(/^ {4}([A-Z]\w+)(?: \{|,)/gm)].map((match) => match[1].replace(/[A-Z]/g, (letter, at) => (at ? "_" : "") + letter.toLowerCase()));
  assert.ok(variants.length > 10);
  for (const [language, file] of Object.entries(LANGUAGE_FILES).filter(([code]) => code !== "en")) {
    for (const code of variants) assert.ok(code in file.notes, `${language}: note ${code} is missing`);
    for (const key of Object.keys(LANGUAGE_FILES.de.errors)) assert.ok(key in file.errors, `${language}: error ${key} is missing`);
  }
  setLanguage("de");
  assert.equal(noteText({ code: "gateway", address: "10.0.0.9:502", text: "x" }), "10.0.0.9:502 ist ein Gateway: die Units dahinter suchen.");
  assert.equal(errorText("12x is not a number"), "12x ist keine Zahl");
  assert.equal(codeText("exception_02"), "keine Daten an dieser Adresse");
  setLanguage("en");
  assert.equal(noteText({ code: "gateway", text: "the server's text" }), "the server's text");
});

test("French, Italian and Spanish are drafts: loaded, not in the menu", () => {
  assert.deepEqual(LANGUAGES.map(([code]) => code), ["en", "de"]);
  for (const code of ["fr", "it", "es"]) assert.ok(LANGUAGE_FILES[code], `${code} is not loaded`);
});

test("the search request is built from the dialog and refused where a field is wrong", () => {
  const base = { mode: "tcp", interface: "", subnet: "", port: "502", gateway: "", gatewayRtu: false, serialPort: "", baud: "9600", frame: "8E1", from: "1", to: "247", timeout: "", retries: "0" };
  assert.deepEqual(searchRequest(base), { kind: "tcp", body: { subnet: undefined, interface: undefined, port: 502, timeout_ms: undefined } });
  assert.ok(searchRequest({ ...base, port: "70000" }).error);
  const gateway = searchRequest({ ...base, mode: "gateway", gateway: "10.0.0.9", gatewayRtu: true, to: "10" });
  assert.equal(gateway.body.endpoint, "rtu-tcp:10.0.0.9:502");
  assert.equal(gateway.body.to, 10);
  assert.ok(searchRequest({ ...base, mode: "gateway" }).error, "a gateway needs its address");
  const serial = searchRequest({ ...base, mode: "serial", serialPort: "/dev/ttyUSB0", timeout: "200", retries: "1" });
  assert.equal(serial.body.endpoint, "serial:/dev/ttyUSB0:9600:8E1");
  assert.equal(serial.body.timeout_ms, 200);
  assert.ok(searchRequest({ ...base, mode: "serial", serialPort: "COM3", from: "0" }).error);
  assert.ok(searchRequest({ ...base, mode: "serial", serialPort: "COM3", timeout: "10" }).error);
});

test("devices group by line and points by group, those without a group last", () => {
  const devices = [{ line: "a", key: "1" }, { line: "b", key: "2" }, { line: "a", key: "3" }];
  assert.deepEqual(lines(devices).map((group) => [group.line, group.devices.length]), [["a", 2], ["b", 1]]);
  const points = [{ name: "x", group: "" }, { name: "y", group: "Pump" }, { name: "z", group: "Boiler" }];
  assert.deepEqual(groupPoints(points).map(([group]) => group), ["Boiler", "Pump", ""]);
});

test("the point filter needs every word somewhere in the point", () => {
  const points = [
    { name: "Flow temperature", group: "Boiler", area: "holding_register", address: 10, unit: "°C", description: "" },
    { name: "Return temperature", group: "Boiler", area: "input_register", address: 12, unit: "°C", description: "" },
  ];
  assert.equal(filterPoints(points, "").length, 2);
  assert.deepEqual(filterPoints(points, "flow °c").map((point) => point.name), ["Flow temperature"]);
  assert.deepEqual(filterPoints(points, "ir 12").map((point) => point.name), ["Return temperature"]);
});

test("states go to one line and back", () => {
  const states = [[0, "off"], [1, "heating"]];
  assert.equal(statesText(states), "0=off; 1=heating");
  assert.deepEqual(parseStates("0=off; 1: heating; nonsense"), states);
});

test("a profile with a wrong number or a name twice is not saved", () => {
  const point = { name: "Flow", group: "", area: "holding_register", address: 0, type: "u16", factor: 1, offset: 0, unit: "" };
  assert.equal(draftProblem({ name: "P", points: [point] }), null);
  assert.ok(draftProblem({ name: "", points: [point] }));
  assert.ok(draftProblem({ name: "P", points: [point, { ...point }] }));
  assert.ok(draftProblem({ name: "P", points: [{ ...point, factor: "0,x" }] }));
  assert.ok(draftProblem({ name: "P", points: [{ ...point, address: 70000 }] }));
  assert.equal(draftProblem({ name: "P", points: [{ ...point, min: undefined, scale_register: undefined }] }), null);
  assert.equal(typedNumber("0,1"), 0.1);
  assert.ok(Number.isNaN(typedNumber("")));
});

test("a point taken from the explorer keeps the order only where it differs from the profile's", () => {
  const adding = { name: " Flow ", group: "", type: "f32", factor: "1", unit: "°C", writable: false };
  const row = { address: 2 };
  const same = pointFromExplorer(adding, { area: "holding_register", order: "CDAB" }, row, "CDAB");
  assert.equal(same.order, undefined);
  assert.equal(same.name, "Flow");
  const other = pointFromExplorer(adding, { area: "holding_register", order: "CDAB" }, row, "ABCD");
  assert.equal(other.order, "CDAB");
  const coil = pointFromExplorer({ ...adding, type: "u16" }, { area: "coil", order: "CDAB" }, row, "ABCD");
  assert.equal(coil.type, "bool");
  assert.equal(coil.order, undefined);
});

test("charts group by unit, leave gaps and give a flat line room", () => {
  const series = [{ unit: "°C", samples: [[0, 20], [1000, null], [2000, 22]] }, { unit: "%", samples: [[0, 5], [2000, 5]] }, { unit: "°C", samples: [] }];
  assert.deepEqual(byUnit(series).map(([unit, members]) => [unit, members.length]), [["°C", 2], ["%", 1]]);
  const box = bounds([series[0]]);
  assert.deepEqual([box.low, box.high, box.first, box.last], [20, 22, 0, 2000]);
  assert.equal((pathData(series[0].samples, box).match(/M/g) ?? []).length, 2, "a failed reading breaks the line");
  const flat = bounds([series[1]]);
  assert.ok(flat.low < 5 && flat.high > 5);
  assert.equal(bounds([series[2]]), null);
});

test("the workspace offers the search first, then exploring, then reading", () => {
  Object.assign(state, { scan: { finished: true, notes: [] }, site: { name: "S", devices: [] }, selected: null, readings: {}, watch: [], recording: { series: [] }, menu: null, asking: null, profiles: [] });
  assert.equal(viewWorkspace(state).next.act, "search-dialog");
  state.site.devices = [{ key: "tcp:10.0.0.5:502/1", line: "10.0.0.5:502", unit: 1, name: "", profile: null, points: 0, limits: { most_registers: 125 } }];
  state.selected = "tcp:10.0.0.5:502/1";
  assert.equal(viewWorkspace(state).next.act, "explore-device");
  state.site.devices[0].profile = "p";
  assert.equal(viewWorkspace(state).next.act, "read-device");
  assert.match(viewWorkspace(state).html, new RegExp(t("pts.notReadTitle")));
});

test("a device without a name is not named after its line where the line stands beside it", () => {
  setLanguage("en");
  const device = { line: "127.0.0.3:5020", unit: 5 };
  assert.equal(deviceName(device), "127.0.0.3:5020 · unit 5");
  assert.equal(deviceNameOnLine(device), "Unit 5");
  assert.equal(deviceNameOnLine({ ...device, product: "EM-340" }), "EM-340");
  assert.equal(deviceName({ ...device, name: "Drive" }), "Drive");
});

test("only the chosen word order shows a float as plausible", () => {
  const badc = { order: "BADC", float: "-34.370117", u32: 3255401216, s32: -1039566080, plausible: true };
  assert.match(pairHtml(badc, "ABCD"), /<dd class="mono dim">/, "a plausible size in another order is no hint");
  assert.match(pairHtml(badc, "BADC"), /<dd class="mono">/);
  assert.match(pairHtml({ ...badc, plausible: false }, "BADC"), /<dd class="mono dim">/);
});

test("a count of one has its own words", () => {
  setLanguage("en");
  assert.equal(counted("prof.pointCount", 1), "1 point");
  assert.equal(counted("prof.pointCount", 2), "2 points");
  assert.equal(counted("ws.leadProfile", 1, { profile: "FU-7" }), "profile FU-7, 1 point");
  setLanguage("de");
  assert.equal(counted("pts.notReadLead", 1), "Das Profil nennt 1 Datenpunkt.");
  setLanguage("en");
});

test("a point's name reaches the write question as text, whatever a vendor's list put in it", () => {
  setLanguage("en");
  const key = "tcp:10.0.0.5:502/1";
  const name = `<img src=x onerror="fetch('/api/session')">`;
  const point = { name, group: "", area: "holding_register", address: 4, count: 1, type: "u16", unit: "", factor: 1, writable: true, reading: { text: "7", value: 7, raw: [7] } };
  Object.assign(state, {
    scan: { finished: true, notes: [] }, site: { name: "S", devices: [{ key, line: "10.0.0.5:502", unit: 1, name: "", profile: "p", points: 1, limits: { most_registers: 125 } }] },
    selected: key, selectedPoint: name, readings: { [key]: { points: [point], failed: 0, notes: [] } }, watch: [], recording: { series: [] }, pointFilter: "", profiles: [], menu: null, unlocked: true,
  });
  for (const step of ["value", "confirm"]) {
    state.asking = { kind: "write", point: name, step, value: "12" };
    const html = viewWorkspace(state).html;
    assert.match(html, /class="ask"/, `the ${step} step is shown`);
    assert.doesNotMatch(html, /<img/, `the ${step} step shows the name as text`);
  }
  Object.assign(state, { asking: null, selected: null, selectedPoint: null, readings: {}, unlocked: false });
});

test("closing a profile asks only when something typed would be lost", () => {
  const empty = { id: "", name: "", vendor: "", model: "", points: [] };
  assert.equal(unsavedChanges({ profileDraft: null, profileDirty: true }), false);
  assert.equal(unsavedChanges({ profileDraft: empty, profileDirty: true }), false, "a new profile nobody typed in");
  assert.equal(unsavedChanges({ profileDraft: { ...empty, name: "FU-7" }, profileDirty: true }), true);
  assert.equal(unsavedChanges({ profileDraft: { ...empty, id: "fu-7", name: "FU-7" }, profileDirty: false }), false, "a saved profile");
  assert.equal(unsavedChanges({ profileDraft: { ...empty, id: "fu-7" }, profileDirty: true }), true, "a saved profile whose name was emptied");
});

test("listening names the unit IDs that talked on the line", () => {
  setLanguage("en");
  const trial = (settings, frames, units) => ({ settings, frames, stray_bytes: 0, units });
  const heard = (trials, active) => ({ listened: { trials, active, quiet: !active }, note: { kind: "quiet", text: "The line is quiet." } });
  const busy = listenedHtml({ listened: heard([trial("9600 8N1", 0, []), trial("9600 8E1", 6, [12, 2, 5])], "9600 8E1") });
  assert.match(busy, /Unit IDs in the traffic heard: 2, 5, 12\./);
  assert.doesNotMatch(listenedHtml({ listened: heard([trial("9600 8E1", 0, [])], null) }), /Unit IDs/);
});
