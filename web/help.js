// The help: guides for the usual tasks, in the interface's language (after bacnetio's help.js;
// docs/concept-bacnetio-sync.md, step 6).

import { h } from "./html.js";
import { currentLanguage, t } from "./i18n.js";
import { registerView } from "./store.js";
import de from "./help/de.js";
import en from "./help/en.js";
import es from "./help/es.js";
import fr from "./help/fr.js";
import it from "./help/it.js";

/** The guides in every language of the interface; French, Italian and Spanish are drafts like their other texts. */
export const HELP_FILES = { en, de, fr, it, es };

/** The guides in the order of the list, each with its group and the place its first step is. */
export const GUIDES = [
  { id: "find", group: "start", target: "search" },
  { id: "gateway", group: "start", target: "search" },
  { id: "trunk", group: "start", target: "search" },
  { id: "profile", group: "points", target: "profiles" },
  { id: "explore", group: "points", target: "explorer" },
  { id: "write", group: "values", target: "changed" },
  { id: "record", group: "values", target: "recording" },
  { id: "analyser", group: "line", target: "analyser", serial: true },
  { id: "othersettings", group: "line", target: "search", serial: true },
  { id: "exports", group: "more", target: "settings" },
  { id: "keys", group: "more" },
];

/** Where a guide's button leads, and its words. */
export const TARGETS = {
  search: "help.goSearch", profiles: "help.goProfiles", explorer: "help.goExplorer", changed: "help.goChanged",
  recording: "help.goRecording", analyser: "help.goAnalyser", settings: "help.goSettings",
};

const words = () => HELP_FILES[currentLanguage()] ?? en;

/** A guide's text with the interface's own names of buttons and fields, and other guides' titles. */
export function helpText(text) {
  return h(text)
    .replace(/\{btn:([\w.]+)\}/g, (_, key) => `<b>${h(t(key))}</b>`)
    .replace(/\{guide:(\w+)\}/g, (_, id) => `«${h(words()[id].title)}»`);
}

/** The keys and what they do, in the order of the keyboard's use. */
function keysHtml(state) {
  const count = state.areas.length;
  const keys = [[`1 – ${count}`, t("keys.areas")], [String(count + 1), t("tab.settings")], ["?", t("help.title")], ["/", t("keys.filter")], ["↑ ↓", t("keys.move")],
    ["Home End ⇞ ⇟", t("keys.jump")], ["Enter", t("keys.open")], ["Tab", t("keys.tab")], ["Esc", t("keys.close")]];
  return `<dl class="facts">${keys.map(([key, what]) => `<dt><kbd>${key}</kbd></dt><dd>${what}</dd>`).join("")}</dl>`;
}

/** The guides this system can follow: those about a serial line need one (not on Android). */
const guidesFor = (state) => GUIDES.filter((guide) => !guide.serial || state.info?.serial !== false);

function listHtml(guides, chosen) {
  const groups = [...new Set(guides.map((guide) => guide.group))];
  return groups.map((group) => {
    const buttons = guides.filter((guide) => guide.group === group).map((guide) =>
      `<button type="button" class="quiet" data-act="help-guide" data-guide="${guide.id}" aria-pressed="${guide.id === chosen}">${h(words()[guide.id].title)}</button>`).join("");
    return `<h3 class="cap">${t(`help.group.${group}`)}</h3>${buttons}`;
  }).join("");
}

function guideHtml(state, guide) {
  const text = words()[guide.id];
  const steps = text.steps ? `<ol class="help-steps">${text.steps.map((step) => `<li>${helpText(step)}</li>`).join("")}</ol>` : "";
  const causes = text.causes ? `<h3>${t("help.causes")}</h3><ul class="help-causes">${text.causes.map((cause) => `<li>${helpText(cause)}</li>`).join("")}</ul>` : "";
  const target = guide.target ? `<div class="acts"><button type="button" class="tonal" data-act="help-go" data-target="${guide.target}">${t(TARGETS[guide.target])}</button></div>` : "";
  const keys = guide.id === "keys" ? keysHtml(state) : "";
  return `<article class="card help-guide" id="help-guide" aria-labelledby="help-guide-title">
    <h2 id="help-guide-title">${h(text.title)}</h2>
    <p>${helpText(text.intro)}</p>${steps}${keys}${target}${causes}</article>`;
}

function viewHelp(state) {
  const guides = guidesFor(state);
  const guide = guides.find((one) => one.id === state.helpGuide) ?? guides[0];
  return {
    title: t("help.title"),
    lead: t("help.lead"),
    next: null,
    html: `<div class="page"><div class="help">
      <nav class="card help-list" aria-label="${t("help.guides")}">${listHtml(guides, guide.id)}</nav>
      ${guideHtml(state, guide)}
    </div></div>`,
  };
}

registerView("help", viewHelp);
