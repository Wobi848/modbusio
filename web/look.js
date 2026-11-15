// The look: theme, accent colour and the background of each theme (after bacnetio's settings.js).
// Each follows the system or its default until chosen in the settings, and applies at once.

import { t } from "./i18n.js";

const THEMES = ["auto", "light", "dark"];
const THEME_KEY = "modbusio.theme";
/** Accent colours to choose from; states keep their own colours whatever is chosen. */
const ACCENTS = ["blue", "crimson", "violet", "teal"];
const ACCENT_KEY = "modbusio.accent";
/** Backgrounds of each theme, the first as it was before there was a choice (theme.js). */
const BACKGROUNDS = { light: ["white", "grey", "warm"], dark: ["navy", "graphite", "black"] };
const BACKGROUND_KEY = { light: "modbusio.background.light", dark: "modbusio.background.dark" };

function stored(key) {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function keep(key, choice) {
  try {
    localStorage.setItem(key, choice);
  } catch {
    // the choice then lasts only until the page is reloaded
  }
  // applyTheme comes from theme.js, which runs before the first drawing
  globalThis.applyTheme();
}

const themeChoice = () => stored(THEME_KEY) || "auto";
const accentChoice = () => stored(ACCENT_KEY) || "blue";

function backgroundChoice(theme) {
  const chosen = stored(BACKGROUND_KEY[theme]);
  return BACKGROUNDS[theme].includes(chosen) ? chosen : BACKGROUNDS[theme][0];
}

export const setTheme = (choice) => keep(THEME_KEY, choice);
export const setAccent = (choice) => keep(ACCENT_KEY, choice);
export const setBackground = (theme, choice) => keep(BACKGROUND_KEY[theme], choice);

/** A row of backgrounds for the light or the dark theme, each with its own swatch. */
function backgroundsHtml(theme) {
  const buttons = BACKGROUNDS[theme].map((name) =>
    `<button type="button" data-act="background" data-scheme="${theme}" data-background="${name}" aria-pressed="${backgroundChoice(theme) === name}"><span class="swatch background-${name}"></span>${t(`set.background.${name}`)}</button>`).join("");
  return `<div class="choice-row"><span class="hint">${t("set.background")} · ${t(`set.background.${theme}`)}</span><span class="choices">${buttons}</span></div>`;
}

/** The choices of the look, for the settings' card. */
export function lookHtml() {
  const themes = THEMES.map((theme) =>
    `<button type="button" data-act="theme" data-theme="${theme}" aria-pressed="${themeChoice() === theme}">${t(theme === "auto" ? "set.system" : `set.${theme}`)}</button>`).join("");
  const accents = ACCENTS.map((accent) =>
    `<button type="button" data-act="accent" data-accent="${accent}" aria-pressed="${accentChoice() === accent}"><span class="swatch accent-${accent}"></span>${t(`set.accent.${accent}`)}</button>`).join("");
  return `<div class="choice-row"><span class="hint" id="theme-label">${t("set.theme")}</span><span class="seg" role="group" aria-labelledby="theme-label">${themes}</span></div>
    <div class="choice-row"><span class="hint">${t("set.accent")}</span><span class="choices">${accents}</span></div>
    ${backgroundsHtml("light")}${backgroundsHtml("dark")}`;
}
