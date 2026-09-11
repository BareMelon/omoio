import { catalogueCover } from "../api";

/// Covers for catalogue tiles, asked for a few at a time.
///
/// A page is sixty tiles and each title RAWG has not been asked about is a
/// request against the user's own allowance, so a title found once is not
/// asked for again this session, no more than four are in flight, and a tile
/// that has left the screen by the time its turn comes is skipped.
const found = new Map<string, string>();
const asking = new Map<string, Promise<string | null>>();
const MOST_AT_ONCE = 4;
let running = 0;
const waiting: (() => void)[] = [];

async function turn<T>(job: () => Promise<T>): Promise<T> {
  if (running >= MOST_AT_ONCE) await new Promise<void>((go) => waiting.push(go));
  running += 1;
  try {
    return await job();
  } finally {
    running -= 1;
    waiting.shift()?.();
  }
}

export function coverFor(titleId: string, name: string, wanted: () => boolean): Promise<string | null> {
  const known = found.get(titleId);
  if (known) return Promise.resolve(known);
  let pending = asking.get(titleId);
  if (!pending) {
    pending = turn(() => (wanted() ? catalogueCover(titleId, name) : Promise.resolve(null)))
      .catch(() => null)
      .then((path) => {
        asking.delete(titleId);
        if (path) found.set(titleId, path);
        return path;
      });
    asking.set(titleId, pending);
  }
  return pending;
}

/// A cover already fetched this session, to draw straight away.
export function knownCover(titleId: string): string | undefined {
  return found.get(titleId);
}
