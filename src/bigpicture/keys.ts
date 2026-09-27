import type { PadFamily } from "../api";
import { nameOf } from "../components/padNames";
import type { Source } from "./input";

const MENU_ICON =
  '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"><path d="M3.5 4.5h9M3.5 8h9M3.5 11.5h9"/></svg>';
const VIEW_ICON =
  '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><rect x="2.5" y="5.5" width="7.5" height="6" rx="1"/><path d="M6 3.5h6.5a1 1 0 0 1 1 1V9"/></svg>';

/// The four face buttons as a diamond with one filled in, for a pad whose
/// markings Omoio doesn't know.
function diamond(place: string): string {
  const spots: [string, number, number][] = [
    ["North", 8, 3.2],
    ["East", 12.8, 8],
    ["South", 8, 12.8],
    ["West", 3.2, 8],
  ];
  const dots = spots
    .map(([at, x, y]) =>
      at === place
        ? `<circle cx="${x}" cy="${y}" r="2.4" fill="currentColor"/>`
        : `<circle cx="${x}" cy="${y}" r="2" fill="none" stroke="currentColor" stroke-width="1.1"/>`
    )
    .join("");
  return `<svg viewBox="0 0 16 16">${dots}</svg>`;
}

const KEYBOARD: Record<string, string> = {
  South: "Enter",
  East: "Esc",
  LB: "PgUp",
  RB: "PgDn",
};

const FACES = ["South", "East", "West", "North"];

/// What goes inside the cap for one place on the pad in hand.
function face(input: string, family: PadFamily): string {
  if (family === "xbox" && input === "Start") return MENU_ICON;
  if (family === "xbox" && input === "Back") return VIEW_ICON;
  if (family === "nintendo" && input === "Start") return "+";
  if (family === "nintendo" && input === "Back") return "−";
  if (family === "generic" && FACES.includes(input)) return diamond(input);
  if (family === "generic" && (input === "LB" || input === "RB")) return input;
  return nameOf(family, input);
}

/// A button as printed on the pad in hand, or the key that does the same.
/// Null when the keyboard has no key for it.
export function keycap(input: string, family: PadFamily | null, source: Source): string | null {
  if (source === "keyboard" || !family) {
    const key = KEYBOARD[input];
    return key ? `<kbd class="bp-key">${key}</kbd>` : null;
  }
  return `<kbd class="bp-key">${face(input, family)}</kbd>`;
}
