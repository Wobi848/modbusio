// The interface's language: one file per language in lang/ with the same keys, English as the
// fallback. The system's language is taken unless one was chosen. Built like bacnetio's.

import de from "./lang/de.js";
import en from "./lang/en.js";
import es from "./lang/es.js";
import fr from "./lang/fr.js";
import it from "./lang/it.js";

export const LANGUAGE_FILES = { en, de, fr, it, es };
/** Languages in the menu. French, Italian and Spanish wait for a review by someone from the trade;
 * reviewers open them with ?lang=fr, ?lang=it or ?lang=es. */
export const LANGUAGES = [["en", "English"], ["de", "Deutsch"]];
const STORAGE_KEY = "modbusio.language";

/** Server errors by pattern; each language names the key's text with $1, $2 for the parts. */
const ERROR_PATTERNS = [
  [/^a search is running; stop it first$/, "searchRunning"],
  [/^the bus analyser listens on (.+); stop it first$/, "analyserListens"],
  [/^writing is locked: unlock it for this session first$/, "locked"],
  [/^this device has no profile yet: .*$/, "noProfile"],
  [/^(.+) is not a number$/, "notNumber"],
  [/^(.+) is not on or off \(1 or 0\)$/, "notOnOff"],
  [/^(.+) is outside (.*) … (.*)$/, "outside"],
  [/^(.+) does not fit (.+) with factor (.+)$/, "noFit"],
  [/^(.+) is beyond (.+): so large a whole number is not written exactly$/, "notExact"],
  [/^(.+) is not marked writable in the profile$/, "notWritable"],
  [/^(.+) is not a subnet like 192\.168\.10\.0\/24$/, "notSubnet"],
  [/^(.+) is too large to ask address by address .*$/, "subnetTooLarge"],
  [/^no permission to open (\S+): .*$/, "permission"],
  [/^cannot open the connection: (\S+) is used by another program.*$/, "portUsed"],
  [/^cannot open the connection: (.*)$/, "cannotOpen"],
  [/^devices use this profile: (.*); give them another first$/, "profileUsed"],
  [/^two points have the same name; each needs its own$/, "sameName"],
  [/^a profile needs a name$/, "profileName"],
  [/^a point needs a name$/, "pointName"],
  [/^no column holds the address$/, "noAddressColumn"],
  [/^no row looks like column names .*$/, "noHeader"],
  [/^(.+) is on the site already$/, "onSite"],
  [/^add points to record first$/, "recordFirst"],
  [/^a recording holds at most (\d+) points$/, "recordMost"],
  [/^no capture yet: start one and stop it$/, "noCapture"],
  [/^the device does not answer Read Device Identification \(43\/14\)$/, "noIdentity"],
  [/^only pages served by this modbusio may use it$/, "foreignPage"],
  [/^(.+) has no unit ID after a \/$/, "noUnit"],
  [/^(.+) is not an IP address with an optional port$/, "notAddress"],
  [/^unit 0 is the broadcast: .*$/, "broadcast"],
];

function stored() {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

/** A language opened for review with ?lang=, kept like a choice from the menu. */
function previewed() {
  const code = new URLSearchParams(globalThis.location?.search ?? "").get("lang");
  if (!LANGUAGE_FILES[code]) return null;
  try {
    localStorage.setItem(STORAGE_KEY, code);
  } catch {
    // the review then lasts only until the page is reloaded without ?lang=
  }
  return code;
}

/** The first of the system's languages that is in the menu. */
function systemLanguage() {
  const tags = globalThis.navigator?.languages ?? [globalThis.navigator?.language];
  const offered = LANGUAGES.map(([code]) => code);
  return tags.map((tag) => tag?.slice(0, 2).toLowerCase()).find((code) => offered.includes(code));
}

const resolve = (choice) => (LANGUAGE_FILES[choice] ? choice : systemLanguage() ?? "en");

let choice = previewed() ?? stored() ?? "auto";
let language = resolve(choice);
if (globalThis.document) document.documentElement.lang = language;

const file = () => LANGUAGE_FILES[language];

export const currentLanguage = () => language;
/** Locale for dates and numbers: "de-CH", "en-GB". */
export const currentLocale = () => file().locale;
/** "auto" or a language code, as chosen in Settings. */
export const languageChoice = () => (LANGUAGE_FILES[choice] ? choice : "auto");

export function setLanguage(code) {
  choice = code;
  language = resolve(code);
  document.documentElement.lang = language;
  try {
    localStorage.setItem(STORAGE_KEY, code);
  } catch {
    // the choice then lasts only until the page is reloaded
  }
}

/** The text for `key` with {placeholders} filled in. */
export function t(key, values = {}) {
  const text = file().texts[key] ?? en.texts[key] ?? key;
  return text.replace(/\{(\w+)\}/g, (_, name) => values[name] ?? "");
}

/** A text with a count: its `…One` text for 1, the text with {n} for any other number. */
export function counted(key, n, values = {}) {
  return n === 1 ? t(`${key}One`, values) : t(key, { ...values, n });
}

/** A note from the server ({code, ...values, text}) in the current language. */
export function noteText(note) {
  const own = file().notes[note.code];
  if (!own) return note.text;
  return own.replace(/\{(\w+)\}/g, (_, name) => note[name] ?? "");
}

/** An error message from the server in the current language; unknown ones stay English. */
export function errorText(message) {
  for (const [pattern, key] of ERROR_PATTERNS) {
    const own = file().errors[key];
    if (own && pattern.test(message)) return message.replace(pattern, own);
  }
  return message;
}

/** A reading's error code (`timeout`, `exception_02` …) in words. */
export function codeText(code) {
  if (!code) return "";
  return file().codes[code] ?? en.codes[code] ?? code.replaceAll("_", " ");
}

/** A number as people of this language write it: decimal comma in German. */
export function localNumber(text) {
  return file().decimalComma ? String(text).replace(/^(-?\d+)\.(\d+)/, "$1,$2") : String(text);
}
