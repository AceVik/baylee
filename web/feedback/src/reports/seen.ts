// Which reports this browser has opened, for the "new" marker in the list.
//
// The service keeps no per-admin state, so this is per device: a report
// counts as unread here until it was opened here, or until it leaves the
// `new` status. Kept in `localStorage`, bounded, and never sent anywhere.

const KEY = "baylee-feedback:seen";

/** How many ids are remembered; the oldest go first. */
export const SEEN_LIMIT = 2000;

interface Stored {
  ids: string[];
}

let cache: Set<string> | null = null;

function read(): Set<string> {
  if (cache !== null) return cache;
  let ids: string[] = [];
  try {
    const raw = localStorage.getItem(KEY);
    if (raw !== null) {
      const parsed: unknown = JSON.parse(raw);
      if (typeof parsed === "object" && parsed !== null && Array.isArray((parsed as Stored).ids)) {
        ids = (parsed as Stored).ids.filter((id): id is string => typeof id === "string");
      }
    }
  } catch {
    // No storage, or something else wrote there: start afresh.
  }
  cache = new Set(ids);
  return cache;
}

function write(ids: Set<string>): void {
  const kept = [...ids].slice(-SEEN_LIMIT);
  cache = new Set(kept);
  try {
    localStorage.setItem(KEY, JSON.stringify({ ids: kept } satisfies Stored));
  } catch {
    // Storage full or refused: the marker is a convenience.
  }
}

/** Whether `id` was opened on this device. */
export function isSeen(id: string): boolean {
  return read().has(id);
}

/** Remembers that `id` was opened here. */
export function markSeen(id: string): void {
  const ids = read();
  if (ids.has(id)) return;
  ids.add(id);
  write(ids);
}

/** Remembers all of `ids` at once (for "mark all as read"). */
export function markAllSeen(ids: readonly string[]): void {
  const seen = read();
  let changed = false;
  for (const id of ids) {
    if (!seen.has(id)) {
      seen.add(id);
      changed = true;
    }
  }
  if (changed) write(seen);
}

/** Forgets everything; for tests. */
export function forgetSeen(): void {
  cache = null;
  try {
    localStorage.removeItem(KEY);
  } catch {
    // Nothing to forget.
  }
}
