/* What the service sent: the newest sample and the two history rings. */

import { settings } from "./store.js";

/** Seconds of per-core usage kept in the browser for the heat strip. */
const CORE_HISTORY = 300;
const HOUR_MS = 3600000;
const DAY_MS = 86400000;

export const data = {
  facts: null,
  snapshot: null,
  training: null,
  samples: [],
  minutes: [],
  /** Per-core usage since this page opened: [{ at_ms, cores }]. */
  coreHistory: [],
  /** The snapshot the page keeps showing while paused. */
  frozen: null,
};

export function receiveHello(message) {
  data.facts = message.facts;
  data.samples = message.history ?? [];
  data.minutes = message.minutes ?? [];
}

export function receiveSample(message) {
  const snapshot = message.snapshot;
  const point = snapshot.point;
  data.snapshot = snapshot;
  data.training = message.training ?? null;
  // The hello history and the latest snapshot can hold the same sample.
  if (!data.samples.length || data.samples.at(-1).at_ms < point.at_ms) data.samples.push(point);
  dropBefore(data.samples, point.at_ms - HOUR_MS * 2);
  dropBefore(data.minutes, point.at_ms - DAY_MS);
  addMinute(point);
  data.coreHistory.push({ at_ms: point.at_ms, cores: snapshot.cores });
  if (data.coreHistory.length > CORE_HISTORY) data.coreHistory.shift();
}

function dropBefore(list, at) {
  let drop = 0;
  while (drop < list.length && list[drop].at_ms < at) drop += 1;
  if (drop) list.splice(0, drop);
}

/**
 * While a page stays open the server's minute ring is not resent, so the page
 * thins its own samples: one point per minute, the newest of that minute. The
 * next reload brings the true averages again.
 */
function addMinute(point) {
  const last = data.minutes.at(-1);
  if (!last || Math.floor(last.at_ms / 60000) < Math.floor(point.at_ms / 60000)) data.minutes.push(point);
}

/** The index of the first element at or after `at`. */
function lowerBound(list, at) {
  let low = 0;
  let high = list.length;
  while (low < high) {
    const middle = (low + high) >> 1;
    if (list[middle].at_ms < at) low = middle + 1;
    else high = middle;
  }
  return low;
}

export const shownSnapshot = () => data.frozen ?? data.snapshot;

/** The points of the chosen time range, and when two points count as a gap. */
export function currentRange() {
  const end = shownSnapshot()?.point.at_ms ?? Date.now();
  const start = end - settings.range * 1000;
  const interval = data.facts?.interval_ms ?? 1000;
  const useMinutes = settings.range > 3600;
  const source = useMinutes ? data.minutes : data.samples;
  const points = source.slice(lowerBound(source, start), lowerBound(source, end + 1));
  return {
    points,
    start,
    end,
    minutes: useMinutes,
    gap: useMinutes ? 150000 : Math.max(3000, interval * 2.5),
  };
}

/** A short recent window for the small graphs, whatever the chosen range. */
export function recentPoints(seconds) {
  const end = shownSnapshot()?.point.at_ms ?? Date.now();
  return data.samples.slice(lowerBound(data.samples, end - seconds * 1000), lowerBound(data.samples, end + 1));
}

export const diskPoint = (point, name) => point.disks?.find(entry => entry[0] === name);
export const netPoint = (point, name) => point.nets?.find(entry => entry[0] === name);
export const gpuPoint = (point, index) => point.gpus?.find(entry => entry[0] === index);
