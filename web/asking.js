// Questions in the details before anything changes the plant (concept section 7): unlocking the
// session, the value, a last confirmation naming what happens. Never a browser dialog.

import { h } from "./html.js";
import { localNumber, t } from "./i18n.js";

/** A question with its reason and buttons; Cancel is always there. */
export function askHtml(title, why, buttons) {
  const acts = buttons.map(([act, label, data = ""]) => `<button type="button" data-act="${act}"${data}>${h(label)}</button>`).join("");
  return `<div class="ask" role="group"><b>${h(title)}</b><span>${why}</span>
    <div class="actions">${acts}<button type="button" class="quiet" data-act="cancel-ask">${t("cancel")}</button></div></div>`;
}

/** The unlock question, shared by every write. */
export const unlockHtml = () => askHtml(t("unlock.title"), t("unlock.why"), [["unlock", t("unlock.allow")]]);

/** What the point accepts, in words: its states, its limits, on or off. */
function acceptsText(point) {
  if (point.area === "coil" || point.type === "bool") return t("write.onOff");
  const parts = [];
  if (point.states?.length) parts.push(point.states.map(([number, word]) => `${number} = ${word}`).join(", "));
  if (point.min !== undefined || point.max !== undefined) {
    parts.push(t("write.range", { min: point.min !== undefined ? localNumber(point.min) : "…", max: point.max !== undefined ? localNumber(point.max) : "…" }));
  }
  if (point.unit) parts.push(t("write.unit", { unit: point.unit }));
  return parts.join(" · ");
}

/** Writing a profile point: a button, then unlock, value and confirmation in place. */
export function writeHtml(state, point) {
  const asking = state.asking;
  if (asking?.kind !== "write" || asking.point !== point.name) {
    return `<div class="actions"><button type="button" class="tonal" data-act="ask-write">${t("write.button")}</button></div>`;
  }
  if (!state.unlocked) return unlockHtml();
  if (asking.step === "confirm") {
    return askHtml(
      t("write.confirmTitle", { value: asking.value, point: point.name }),
      t("write.confirmWhy"),
      [["write", t("write.do")]],
    );
  }
  const accepts = acceptsText(point);
  return `<div class="ask" role="group"><b>${h(t("write.title", { point: point.name }))}</b>
    ${accepts ? `<span class="hint">${h(accepts)}</span>` : ""}
    <label class="form-field">${t("write.value")}<input id="write-value" class="mono" value="${h(asking.value ?? "")}" autocomplete="off"></label>
    <div class="actions"><button type="button" data-act="write-check">${t("write.next")}</button><button type="button" class="quiet" data-act="cancel-ask">${t("cancel")}</button></div></div>`;
}
