---
name: new-text
description: "Add or change words the modbusio interface shows, in every language (web/lang/en.js, de.js), including server notes, translated errors and error codes. Use for every change that puts new or different text in front of the user, even a single button label or error message, so no language is left behind."
---

# Texts in every language

One file per language in `web/lang/`: `en.js`, `de.js`, with the same keys. `tests/web/ui.test.js`
fails when a key is missing in a language, a key used in `web/` does not exist, the placeholders
differ, or a note of `src/diagnosis.rs` has no German text.

## Steps

1. Pick a key in the area's style (`dev.*`, `pts.*`, `ex.*`, `prof.*`, `imp.*`, `log.*` …); reuse
   one where the words mean the same.
2. Add it to **every** file under `texts`. Placeholders as `{name}`, the same in every language.
3. Words from the server:
   - a note (a variant of `Note` in `src/diagnosis.rs`): its text under `notes` in every language
     except English, which comes from the server
   - a known error: the pattern once in `ERROR_PATTERNS` in `web/i18n.js`, its text under `errors`
     in every language except English
   - an error code of a reading (`timeout`, `exception_02` …): under `codes`
4. Modbus words stay as the trade uses them: Holding-Register, Input-Register, Coil, Unit-ID,
   Gateway. German in Swiss spelling (ss).
5. Run `node --test tests/web/*.test.js` and the layout test in German
   (`node tests/ui/run.mjs settings`).

Never build sentences from pieces; give the whole sentence a key with placeholders, and a
singular its own key.
