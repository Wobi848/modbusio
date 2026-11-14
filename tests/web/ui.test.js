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
const { heldUntil, sameRecording, withNewSamples } = await import("../../web/recording.js");
const { deviceName, deviceNameOnLine, state } = await import("../../web/store.js");
const { pairHtml } = await import("../../web/explorer.js");
const { versionText } = await import("../../web/pages.js");
const { listenedHtml } = await import("../../web/shell.js");
const { askedText, bytesHtml, filteredExchanges, heardChanged, resultText, takeHeard, KEPT } = await import("../../web/analyser.js");
const { startRequest } = await import("../../web/analyser-actions.js");
const { profileFrom, registersUnit, stepSamples } = await import("../../web/analyser-registers.js");

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
  for (const prefix of ["tab.workspace", "tab.explorer", "tab.profiles", "tab.watch", "tab.recording", "tab.changed", "tab.analyser", "an.role.request", "an.role.answer", "phase.hosts", "phase.units", "phase.settings", "phase.identify",
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

test("the recording takes up new samples and keeps what it holds", () => {
  const series = (point, samples, total = samples.length) => ({ device: "tcp:10.0.0.5:502/1", point, label: point, unit: "°C", samples, total });
  const recording = (...all) => ({ series: all, interval: 5, running: true });
  const held = recording(series("Flow", [[1000, 20], [2000, 21]]), series("Return", [[1000, 15]]));
  assert.equal(heldUntil(held), 1000, "the series a round behind decides what is asked for");
  assert.equal(heldUntil(recording()), 0);
  assert.equal(heldUntil(recording(series("Flow", []))), 0, "a series without samples: everything");

  // what modbusio answers to after=1000: Flow's second sample once more, and a new one of each
  const merged = withNewSamples(held, recording(series("Flow", [[2000, 21], [3000, null]], 3), series("Return", [[2000, 16]], 2)));
  assert.deepEqual(merged.series.map((item) => item.samples), [[[1000, 20], [2000, 21], [3000, null]], [[1000, 15], [2000, 16]]]);
  assert.ok(!sameRecording(held, merged));

  const quiet = withNewSamples(merged, recording(series("Flow", [], 3), series("Return", [[3000, 17]], 3)));
  assert.equal(quiet.series[0].samples, merged.series[0].samples, "a series without news keeps its list");
  assert.ok(!sameRecording(merged, quiet));
  const nothing = withNewSamples(quiet, recording(series("Flow", [], 3), series("Return", [], 3)));
  assert.ok(sameRecording(quiet, nothing), "nothing to draw anew");
  assert.ok(!sameRecording(quiet, { ...nothing, running: false }));

  // a full series loses its oldest; an emptied recording all it had
  const full = withNewSamples(quiet, recording(series("Flow", [[4000, 23]], 3), series("Return", [], 3)));
  assert.deepEqual(full.series[0].samples, [[2000, 21], [3000, null], [4000, 23]]);
  const emptied = withNewSamples(quiet, recording(series("Flow", [[9000, 1]], 1), series("Return", [], 0)));
  assert.deepEqual(emptied.series.map((item) => item.samples), [[[9000, 1]], []]);

  // what does not fit is asked for whole
  assert.equal(withNewSamples(quiet, recording(series("Flow", [], 3))), null, "a point removed meanwhile");
  assert.equal(withNewSamples(quiet, recording(series("Flow", [], 3), series("Speed", [], 3))), null, "another point in its place");
  assert.equal(withNewSamples(quiet, recording(series("Flow", [], 5), series("Return", [], 3))), null, "samples the page missed");
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

const exchange = (number, unit, outcome, asked = { area: "holding_register", address: 100, count: 2, write: false }) =>
  ({ number, at: number * 1000, unit, function: 3, asked, outcome, answer_micros: 20000, request: "", answer: "" });

test("the analyser's lists take what is new and start again for a new analysis", () => {
  const first = takeHeard(null, { running: true, started_ms: 1, units: [], exchanges: [exchange(0, 1, { kind: "written" })], frames: [] });
  const more = takeHeard(first, { running: true, started_ms: 1, units: [], exchanges: [exchange(1, 1, { kind: "written" })], frames: [] });
  assert.deepEqual(more.exchanges.map((item) => item.number), [0, 1]);
  const other = takeHeard(more, { running: true, started_ms: 2, units: [], exchanges: [exchange(5, 1, { kind: "written" })], frames: [] });
  assert.deepEqual(other.exchanges.map((item) => item.number), [5]);
  const many = Array.from({ length: KEPT + 10 }, (_, number) => exchange(number, 1, { kind: "written" }));
  assert.equal(takeHeard(null, { running: true, started_ms: 3, units: [], exchanges: many, frames: [] }).exchanges.length, KEPT);
  assert.equal(takeHeard(null, { running: false }), null);
});

test("an exchange is said in words, in German too", () => {
  setLanguage("en");
  assert.equal(askedText(exchange(0, 1, { kind: "written" })), "HR 100, 2 values");
  assert.equal(askedText({ ...exchange(0, 1, {}), asked: { area: "coil", address: 3, count: 1, write: true } }), "write coil 3");
  assert.equal(askedText({ ...exchange(0, 1, {}), asked: null, function: 0x41 }), "function 41");
  assert.equal(resultText(exchange(0, 1, { kind: "registers", values: [1, 2, 3, 4, 5, 6, 7] })), "1 2 3 4 5 6 …");
  assert.equal(resultText(exchange(0, 1, { kind: "exception", code: 2 })), "exception 02: no data at this address");
  setLanguage("de");
  assert.equal(askedText(exchange(0, 1, {})), "HR 100, 2 Werte");
  assert.equal(resultText(exchange(0, 1, { kind: "no_answer" })), "keine Antwort");
  assert.match(resultText(exchange(0, 1, { kind: "exception", code: 11 })), /^Ausnahme 0B: /);
  setLanguage("en");
});

test("the exchanges are filtered by unit, problems and writes, newest first", () => {
  const write = { area: "holding_register", address: 1, count: 1, write: true };
  const heard = { exchanges: [exchange(0, 1, { kind: "registers", values: [1] }), exchange(1, 12, { kind: "no_answer" }), exchange(2, 12, { kind: "written" }, write)] };
  const numbers = (filters) => filteredExchanges({ heard, unitFilter: "", problemsOnly: false, writesOnly: false, ...filters }).map((item) => item.number);
  assert.deepEqual(numbers({}), [2, 1, 0]);
  assert.deepEqual(numbers({ unitFilter: "12" }), [2, 1]);
  assert.deepEqual(numbers({ problemsOnly: true }), [1]);
  assert.deepEqual(numbers({ writesOnly: true }), [2]);
});

test("the analyser starts with a port, and with speed and frame unless it finds them by listening", () => {
  assert.ok(startRequest({ port: "", speed: "listen", frame: "8E1" }).error);
  assert.deepEqual(startRequest({ port: "COM3", speed: "listen", frame: "8E1" }).body, { port: "COM3" });
  assert.deepEqual(startRequest({ port: "COM3", speed: "19200", frame: "8N1" }).body, { port: "COM3", baud: 19200, frame: "8N1" });
});

test("a frame's bytes are shown with unit, function, data and CRC", () => {
  const html = bytesHtml("0C 03 00 64 00 02 84 C9");
  assert.match(html, /0C<small>Unit<\/small>/);
  assert.match(html, /00 64 00 02<small>Data<\/small>/);
  assert.match(html, /84 C9<small>CRC<\/small>/);
});

test("a register's changes are drawn as steps up to now", () => {
  assert.deepEqual(stepSamples([[0, 4200], [2000000, 4500]], 1000, 5000), [[1000, 4200], [3000, 4200], [3000, 4500], [5000, 4500]]);
  assert.deepEqual(stepSamples([], 1000, 5000), []);
});

test("the registers show the unit chosen, else the first heard", () => {
  const heard = { units: [{ unit: 1 }, { unit: 12 }] };
  assert.equal(registersUnit({ heard, registersUnit: "" }), 1);
  assert.equal(registersUnit({ heard, registersUnit: "12" }), 12);
  assert.equal(registersUnit({ heard: { units: [] }, registersUnit: "" }), null);
});

test("a write of the master names the value before it where it was read", () => {
  const write = { area: "holding_register", address: 1, count: 1, write: true, values: [4500] };
  assert.equal(resultText({ ...exchange(0, 12, { kind: "written" }, write), before: [4200] }), "written: 4200 → 4500");
  assert.equal(resultText(exchange(0, 12, { kind: "written" }, write)), "written: 4500");
});

test("a draft profile from the trunk: floats where the order says so, bits, writable where written", () => {
  const row = (area, address, extra = {}) => ({ area, reads: 3, writes: 0, reading: { address, pairs: [] }, ...extra });
  const data = {
    unit: 1, order: "CDAB", points: [],
    registers: [
      { ...row("holding_register", 0), reading: { address: 0, pairs: [{ order: "CDAB", plausible: true }] } },
      row("holding_register", 1),
      row("holding_register", 5, { writes: 2 }),
      row("coil", 3),
    ],
  };
  const draft = profileFrom(data, "Unit 1 from the trunk");
  assert.equal(draft.order, "CDAB");
  assert.deepEqual(draft.points.map((point) => [point.name, point.type, point.writable]), [["HR 0", "f32", false], ["HR 5", "u16", true], ["coil 3", "bool", false]]);
  assert.equal(draftProblem(draft), null);
});

test("the capture file's errors are said in German", () => {
  setLanguage("de");
  assert.equal(errorText("trunk.zip holds no RTU frames (3 packets of other kinds)"), "trunk.zip enthält keine RTU-Rahmen (3 Pakete anderer Art)");
  setLanguage("en");
});

test("the analyser's page is drawn anew only when something changed", () => {
  const idle = { running: false };
  assert.equal(heardChanged(null, idle), false);
  const running = { running: true, started_ms: 5, ended: null, exchanges: [], frames: [] };
  assert.equal(heardChanged(null, running), true);
  assert.equal(heardChanged(running, { ...running }), false);
  assert.equal(heardChanged(running, { ...running, frames: [{ number: 1 }] }), true);
  assert.equal(heardChanged(running, { ...running, running: false, ended: "stopped" }), true);
});

test("the version names the commit it was built from", () => {
  assert.equal(versionText({ version: "0.1.0", commit: "3f6d90e" }), "0.1.0 · 3f6d90e");
  assert.equal(versionText({ version: "0.1.0", commit: "" }), "0.1.0");
  assert.equal(versionText(null), "");
});
