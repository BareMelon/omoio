import { padsHeld } from "../api";

export type Move = "up" | "down" | "left" | "right";
export type Action = Move | "accept" | "back" | "menu" | "prev" | "next";
export type Source = "pad" | "keyboard";

/// Places on a pad, named the way Omoio reads them, and what each does.
const MOVES: Record<string, Move> = {
  Up: "up",
  "LS Y+": "up",
  Down: "down",
  "LS Y-": "down",
  Left: "left",
  "LS X-": "left",
  Right: "right",
  "LS X+": "right",
};

const BUTTONS: Record<string, Action> = {
  South: "accept",
  East: "back",
  LB: "prev",
  RB: "next",
};

const KEYS: Record<string, Action> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Enter: "accept",
  " ": "accept",
  Escape: "back",
  Backspace: "back",
  PageUp: "prev",
  PageDown: "next",
};

/// A direction held down keeps moving after a pause, as on a console.
const REPEAT_AFTER = 380;
const REPEAT_EVERY = 110;

/// Turns the keyboard and every pad into actions, while `active` says Big
/// Picture is the thing being used. `pressed` hears the bottom face button
/// go down and come up, for the feel of a button under the thumb. The pads
/// report nothing while another program is in front, so that needs no
/// check here.
export function listen(
  act: (action: Action, source: Source) => void,
  active: () => boolean,
  pressed: (down: boolean) => void
): void {
  document.addEventListener("keydown", (event) => {
    if (!active() || event.altKey || event.ctrlKey || event.metaKey) return;
    const action = KEYS[event.key];
    if (!action) return;
    // The page would also press a focused button on Enter or Space, which
    // would press it twice.
    event.preventDefault();
    if (event.repeat && (action === "accept" || action === "back")) return;
    act(action, "keyboard");
  });
  document.addEventListener("keyup", (event) => {
    if (active() && (event.key === " " || event.key === "Enter")) event.preventDefault();
  });

  let held = new Set<string>();
  const since = new Map<string, number>();
  let lastRepeat = 0;
  let reading = false;
  let fresh = true;
  // Menu counts when it is let go, and only if View wasn't held with it:
  // the two together bring a game back, which is not a request for the menu.
  let menuDown = false;
  let chord = false;

  window.setInterval(async () => {
    if (reading) return;
    if (!active()) {
      fresh = true;
      return;
    }
    reading = true;
    try {
      const now = new Set(await padsHeld().catch((): string[] => []));
      if (!active()) return;
      if (fresh) {
        // Whatever is held as Big Picture comes up, such as the two buttons
        // that brought it, is not a press.
        fresh = false;
        held = now;
        menuDown = false;
        return;
      }
      const time = Date.now();
      if (now.has("Start") && now.has("Back")) chord = true;
      for (const input of now) {
        if (!held.has(input)) {
          since.set(input, time);
          if (input === "Start") {
            menuDown = true;
            continue;
          }
          if (input === "South") pressed(true);
          const action = MOVES[input] ?? BUTTONS[input];
          if (action) act(action, "pad");
        } else if (
          MOVES[input] &&
          time - (since.get(input) ?? time) > REPEAT_AFTER &&
          time - lastRepeat > REPEAT_EVERY
        ) {
          lastRepeat = time;
          act(MOVES[input], "pad");
        }
      }
      if (held.has("South") && !now.has("South")) pressed(false);
      if (menuDown && !now.has("Start")) {
        if (!chord) act("menu", "pad");
        menuDown = false;
      }
      if (!now.has("Start") && !now.has("Back")) chord = false;
      held = now;
    } finally {
      reading = false;
    }
  }, 50);
}

/// Waits until the pads' face buttons are let go, for up to a second. A game
/// given the pad while a button is still down takes that button as a press.
export async function untilLetGo(): Promise<void> {
  const until = Date.now() + 1000;
  while (Date.now() < until) {
    const now = await padsHeld().catch((): string[] => []);
    if (!now.some((input) => input === "South" || input === "East")) return;
    await new Promise((resolve) => setTimeout(resolve, 30));
  }
}
