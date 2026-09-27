import type { Move } from "./input";

/// Everything that can take the highlight in `layer`, skipping what isn't
/// drawn.
export function focusables(layer: HTMLElement): HTMLElement[] {
  return [...layer.querySelectorAll<HTMLElement>(".nav")].filter((el) => el.getClientRects().length > 0);
}

/// How far `b` is from `a` in one range, or 0 where the two overlap.
function gap(a1: number, a2: number, b1: number, b2: number): number {
  if (b2 < a1) return a1 - b2;
  if (b1 > a2) return b1 - a2;
  return 0;
}

/// How far a move from `a` to `b` goes, or null when `b` doesn't lie that
/// way. Left and right stay in the row: at the end of the top bar there is
/// nothing more to the right, rather than a tile in the row below. Up and
/// down may go to whatever is nearest, weighing distance across the move
/// three times as much as distance along it.
function distance(a: DOMRect, b: DOMRect, move: Move): number | null {
  const sideways = move === "left" || move === "right";
  const centre = (r: DOMRect) => (sideways ? r.left + r.width / 2 : r.top + r.height / 2);
  const crossCentre = (r: DOMRect) => (sideways ? r.top + r.height / 2 : r.left + r.width / 2);
  const ahead = move === "right" || move === "down" ? centre(b) - centre(a) : centre(a) - centre(b);
  if (ahead <= 1) return null;

  const along = {
    right: b.left - a.right,
    left: a.left - b.right,
    down: b.top - a.bottom,
    up: a.top - b.bottom,
  }[move];
  const across = sideways ? gap(a.top, a.bottom, b.top, b.bottom) : gap(a.left, a.right, b.left, b.right);
  if (sideways && across > 0) return null;
  return Math.max(0, along) + across * 3 + Math.abs(crossCentre(a) - crossCentre(b)) * 0.1;
}

/// The row each tile was last left in, so coming back into a row lands on
/// the tile left there, as a console does, rather than the nearest one.
const lastInGroup = new Map<string, string>();

export function remember(el: HTMLElement): void {
  const { group, key } = el.dataset;
  if (group && key) lastInGroup.set(group, key);
}

/// The element a move from `from` lands on, among `candidates`.
export function nearest(from: HTMLElement, candidates: HTMLElement[], move: Move): HTMLElement | null {
  const a = from.getBoundingClientRect();
  let best: HTMLElement | null = null;
  let bestScore = Infinity;
  for (const el of candidates) {
    if (el === from) continue;
    const score = distance(a, el.getBoundingClientRect(), move);
    if (score !== null && score < bestScore) {
      bestScore = score;
      best = el;
    }
  }
  const group = best?.dataset.group;
  if (!best || !group || group === from.dataset.group) return best;
  const key = lastInGroup.get(group);
  return candidates.find((el) => el.dataset.key === key) ?? best;
}

/// Slides a row of tiles along so the highlighted one is on screen with a
/// little of the next showing, rather than scrolling the page sideways.
export function revealInRow(tile: HTMLElement): void {
  const track = tile.parentElement;
  const rail = track?.parentElement;
  if (!track?.classList.contains("bp-track") || !rail) return;
  const width = rail.clientWidth;
  const margin = width * 0.08;
  let offset = Number(track.dataset.offset ?? 0);
  const left = tile.offsetLeft - offset;
  const right = left + tile.offsetWidth;
  if (right > width - margin) offset += right - (width - margin);
  else if (left < margin) offset -= margin - left;
  offset = Math.max(0, Math.min(offset, track.offsetWidth - width));
  track.dataset.offset = String(offset);
  track.style.transform = `translateX(${-offset}px)`;
}

/// Scrolls `scroller` so `el` sits clear of its top and bottom edges.
export function revealInColumn(el: HTMLElement, scroller: HTMLElement, smooth: boolean): void {
  const r = el.getBoundingClientRect();
  const s = scroller.getBoundingClientRect();
  const margin = s.height * 0.14;
  const behavior: ScrollBehavior = smooth ? "smooth" : "auto";
  if (r.top < s.top + margin) scroller.scrollBy({ top: r.top - s.top - margin, behavior });
  else if (r.bottom > s.bottom - margin) scroller.scrollBy({ top: r.bottom - s.bottom + margin, behavior });
}
