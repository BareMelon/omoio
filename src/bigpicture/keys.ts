import type { PadFamily } from "../api";
import { capFace } from "../components/padArt";
import type { Source } from "./input";

const KEYBOARD: Record<string, string> = {
  South: "Enter",
  East: "Esc",
  LB: "PgUp",
  RB: "PgDn",
};

/// A button as printed on the pad in hand, or the key that does the same.
/// Null when the keyboard has no key for it.
export function keycap(input: string, family: PadFamily | null, source: Source): string | null {
  if (source === "keyboard" || !family) {
    const key = KEYBOARD[input];
    return key ? `<kbd class="bp-key">${key}</kbd>` : null;
  }
  return `<kbd class="bp-key">${capFace(family, input)}</kbd>`;
}
