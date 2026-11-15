// Copied from bacnetio (web/analysis.js), without its grouping by unit (chart.js has modbusio's).
// Looking at recorded values: a window of time, the key figures of a series, and fewer samples to draw
// when there are more than a chart can show. No screen and no network in here.

/** The samples inside a window of time; the whole series without one. */
/** Lowest and highest; a recording is too long to spread into Math.min. */
export function extent(values) {
  return values.reduce(([low, high], value) => [Math.min(low, value), Math.max(high, value)], [Infinity, -Infinity]);
}

export function inWindow(samples, range) {
  if (!range) return samples;
  return samples.filter(([ms]) => ms >= range.from && ms <= range.to);
}

/**
 * Figures of a series: lowest, highest, mean, last, how many samples and how many without an answer.
 * The mean weighs each value by how long it held, so an uneven interval does not skew it. A stepped
 * series (binary, states) also tells the share of time spent on and how often it changed.
 */
export function figures(samples, stepped = false) {
  const known = samples.filter(([, value]) => value !== null);
  const result = { count: samples.length, gaps: samples.length - known.length, low: null, high: null, mean: null, last: null, onShare: null, changes: null };
  if (!known.length) return result;
  [result.low, result.high] = extent(known.map(([, value]) => value));
  result.last = known.at(-1)[1];
  let weighted = 0;
  let total = 0;
  let on = 0;
  for (let index = 0; index + 1 < known.length; index++) {
    const held = known[index + 1][0] - known[index][0];
    weighted += known[index][1] * held;
    total += held;
    if (known[index][1] > 0) on += held;
  }
  result.mean = total ? weighted / total : known[0][1];
  if (stepped) {
    result.onShare = total ? on / total : known[0][1] > 0 ? 1 : 0;
    result.changes = known.filter(([, value], index) => index > 0 && value !== known[index - 1][1]).length;
  }
  return result;
}

/**
 * At most about `buckets` samples to draw, keeping each stretch's lowest and highest, so that a spike
 * stays visible, and gaps stay gaps. Short series come back as they are.
 */
export function thinned(samples, buckets = 600) {
  if (samples.length <= buckets * 2) return samples;
  const size = samples.length / buckets;
  const out = [];
  for (let bucket = 0; bucket < buckets; bucket++) {
    const part = samples.slice(Math.floor(bucket * size), Math.floor((bucket + 1) * size));
    const gap = part.find(([, value]) => value === null);
    const known = part.filter(([, value]) => value !== null);
    if (gap) out.push(gap);
    if (!known.length) continue;
    const low = known.reduce((best, sample) => (sample[1] < best[1] ? sample : best));
    const high = known.reduce((best, sample) => (sample[1] > best[1] ? sample : best));
    out.push(...(low[0] <= high[0] ? [low, high] : [high, low]));
  }
  return out;
}
