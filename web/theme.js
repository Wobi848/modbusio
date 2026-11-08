// Sets the theme before the first drawing, otherwise the wrong one flashes briefly. Loaded as a
// plain script in <head>. "auto" follows the system. The accent colour and the background of each
// theme are chosen the same way.

const THEME_KEY = "modbusio.theme";
const ACCENT_KEY = "modbusio.accent";
const BACKGROUND_KEY = { light: "modbusio.background.light", dark: "modbusio.background.dark" };
const BACKGROUNDS = { light: ["white", "grey", "warm"], dark: ["navy", "graphite", "black"] };
const darkQuery = matchMedia("(prefers-color-scheme: dark)");

function storedTheme() {
  try {
    return localStorage.getItem(THEME_KEY) || "auto";
  } catch {
    return "auto";
  }
}

function storedAccent() {
  try {
    return localStorage.getItem(ACCENT_KEY) || "blue";
  } catch {
    return "blue";
  }
}

/** The background chosen for `theme`; the first of its list unless another was chosen. */
function storedBackground(theme) {
  try {
    const chosen = localStorage.getItem(BACKGROUND_KEY[theme]);
    return BACKGROUNDS[theme].includes(chosen) ? chosen : BACKGROUNDS[theme][0];
  } catch {
    return BACKGROUNDS[theme][0];
  }
}

function applyTheme() {
  const choice = storedTheme();
  const dark = choice === "dark" || (choice === "auto" && darkQuery.matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  document.documentElement.dataset.accent = storedAccent();
  document.documentElement.dataset.background = storedBackground(dark ? "dark" : "light");
}

applyTheme();
darkQuery.addEventListener("change", applyTheme);
