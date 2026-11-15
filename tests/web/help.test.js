// The help's guides: the same in every language, naming only buttons and guides that
// exist (docs/concept-bacnetio-sync.md, step 6). After bacnetio's test.

import { test } from "node:test";
import assert from "node:assert/strict";

globalThis.document = { documentElement: {}, addEventListener() {} };
globalThis.window = { addEventListener() {} };
const { GUIDES, HELP_FILES, TARGETS, helpText } = await import("../../web/help.js");
const { LANGUAGE_FILES, setLanguage } = await import("../../web/i18n.js");

const english = HELP_FILES.en;
const texts = (guide) => [guide.intro, ...(guide.steps ?? []), ...(guide.causes ?? [])];

test("every language has every guide, with as many steps and causes", () => {
  assert.deepEqual(Object.keys(HELP_FILES).sort(), Object.keys(LANGUAGE_FILES).sort());
  for (const [language, guides] of Object.entries(HELP_FILES)) {
    assert.deepEqual(Object.keys(guides).sort(), GUIDES.map((guide) => guide.id).sort(), language);
    for (const { id } of GUIDES) {
      assert.ok(guides[id].title && guides[id].intro, `${language} ${id}`);
      assert.equal(guides[id].steps?.length, english[id].steps?.length, `${language} ${id} steps`);
      assert.equal(guides[id].causes?.length, english[id].causes?.length, `${language} ${id} causes`);
    }
  }
});

test("guides name only buttons and guides that exist, and lead only to known places", () => {
  for (const [language, guides] of Object.entries(HELP_FILES)) {
    for (const text of Object.values(guides).flatMap(texts)) {
      for (const [, key] of text.matchAll(/\{btn:([\w.]+)\}/g)) assert.ok(key in LANGUAGE_FILES.en.texts, `${language}: ${key}`);
      for (const [, id] of text.matchAll(/\{guide:(\w+)\}/g)) assert.ok(id in guides, `${language}: ${id}`);
      assert.doesNotMatch(text.replace(/\{(btn|guide):[\w.]+\}/g, ""), /[{}]/, `${language}: ${text}`);
    }
  }
  for (const guide of GUIDES.filter((one) => one.target)) assert.ok(TARGETS[guide.target], guide.id);
  for (const label of Object.values(TARGETS)) assert.ok(label in LANGUAGE_FILES.en.texts, label);
});

test("a guide shows the buttons by the words of the interface", () => {
  setLanguage("de");
  assert.equal(helpText("{btn:side.search} öffnen, siehe {guide:gateway}"),
    "<b>Sucheinstellungen</b> öffnen, siehe «Geräte hinter einem Gateway finden»");
  setLanguage("en");
  assert.equal(helpText("{guide:keys}: <{btn:search.start}>"), "«Keys»: &lt;<b>Search</b>&gt;");
});
