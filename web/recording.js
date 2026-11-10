// What the page holds of the recording. It asks only for the samples it does not have yet
// (GET /api/recording?after=): a day of samples every two seconds is too much to send and to draw.

/** The time after which to ask. The devices of one round are read one after another, so a series
 * may be a round behind the others: the earliest of the last samples. 0 asks for everything. */
export function heldUntil(recording) {
  if (!recording.series.length) return 0;
  return Math.min(...recording.series.map((series) => series.samples.at(-1)?.[0] ?? 0));
}

/** `held` with what `answer` brings since. A series without news keeps its list of samples, so the
 * page sees what changed. Null where the two do not fit: points added or removed meanwhile, or
 * samples the page missed. */
export function withNewSamples(held, answer) {
  if (answer.series.length !== held.series.length) return null;
  const series = [];
  for (const [index, fresh] of answer.series.entries()) {
    const known = held.series[index];
    if (known.device !== fresh.device || known.point !== fresh.point) return null;
    const last = known.samples.at(-1)?.[0] ?? -Infinity;
    const added = fresh.samples.filter(([time]) => time > last);
    const count = known.samples.length + added.length;
    if (count < fresh.total) return null;
    // modbusio drops the oldest when a series is full, and all of them when the recording is emptied
    const changed = added.length > 0 || count > fresh.total;
    series.push({ ...fresh, samples: changed ? [...known.samples, ...added].slice(count - fresh.total) : known.samples });
  }
  return { ...answer, series };
}

/** Whether two states of the recording show the same: nothing to draw anew. */
export function sameRecording(one, other) {
  return one.running === other.running && one.interval === other.interval && one.series.length === other.series.length
    && one.series.every((series, index) => series.samples === other.series[index].samples);
}
