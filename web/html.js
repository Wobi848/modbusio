// Text into HTML: everything from the network or the user goes through this.

/** Text into HTML with what a search found marked. */
export function marked(text, words) {
  const raw = String(text ?? "");
  const lower = raw.toLowerCase();
  const spans = [];
  for (const word of words.filter(Boolean)) {
    for (let at = lower.indexOf(word); at !== -1; at = lower.indexOf(word, at + word.length)) spans.push([at, at + word.length]);
  }
  if (!spans.length) return h(raw);
  spans.sort((first, second) => first[0] - second[0]);
  let out = "";
  let done = 0;
  for (const [from, to] of spans) {
    if (from < done) continue;
    out += `${h(raw.slice(done, from))}<mark>${h(raw.slice(from, to))}</mark>`;
    done = to;
  }
  return out + h(raw.slice(done));
}

export const h = (text) => String(text ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
