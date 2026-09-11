import { catalogueCover, type Console } from "../api";

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

export function coverFor(
  key: string,
  name: string,
  console: Console,
  wanted: () => boolean
): Promise<string | null> {
  const known = found.get(key);
  if (known) return Promise.resolve(known);
  let pending = asking.get(key);
  if (!pending) {
    pending = turn(() => (wanted() ? catalogueCover(key, name, console) : Promise.resolve(null)))
      .catch(() => null)
      .then((path) => {
        asking.delete(key);
        if (path) found.set(key, path);
        return path;
      });
    asking.set(key, pending);
  }
  return pending;
}

/// A cover already fetched this session, to draw straight away.
export function knownCover(key: string): string | undefined {
  return found.get(key);
}
