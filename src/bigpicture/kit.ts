/// What every Big Picture screen is built from, and what a screen may ask of
/// Big Picture around it.

export type Section = "home" | "library" | "settings";

export interface Screen {
  section: Section;
  draw(): HTMLElement;
  /// The key of what is highlighted when the screen opens.
  first(): string | undefined;
  /// The bumpers, for a screen with tabs of its own.
  tab?(step: number): void;
  /// What the bumpers move between, for the hint, or null while there is
  /// only one.
  tabName?(): string | null;
  /// Told whenever the highlight lands on something of the screen's. Not
  /// the page's own focus event, which a pad doesn't cause while the page
  /// itself isn't focused.
  landed?(el: HTMLElement): void;
  /// What was highlighted when the screen was last on show.
  left?: string;
}

export interface Question {
  title: string;
  text: string;
  confirm: string;
  run: () => void;
}

export interface Choice {
  value: string;
  label: string;
}

export interface Kit {
  open(screen: Screen): void;
  /// Draws the screen on top again, if `screen` is still the one on top,
  /// keeping the highlight where it was.
  redraw(screen: Screen): void;
  ask(question: Question): void;
  /// A list to choose from, the current one ticked. Null when backed out of.
  pick(title: string, choices: Choice[], current: string): Promise<string | null>;
  /// The next button pressed, by place: on `device` if given, else on any
  /// pad. Null if none came.
  record(title: string, text: string, device?: string): Promise<string | null>;
  say(text: string): void;
}

export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className = "",
  text?: string
): HTMLElementTagNameMap[K] {
  const made = document.createElement(tag);
  if (className) made.className = className;
  if (text !== undefined) made.textContent = text;
  return made;
}

/// Something the highlight can land on. `key` finds it again after the
/// screen is drawn anew; `group` is the row it belongs to. A button marked
/// aria-disabled can still be landed on, so what it says can be read, but
/// does nothing.
export function navButton(className: string, key: string, run: () => void, group?: string): HTMLButtonElement {
  const button = h("button", `${className} nav`);
  button.dataset.key = key;
  if (group) button.dataset.group = group;
  button.onclick = () => {
    if (button.getAttribute("aria-disabled") !== "true") run();
  };
  return button;
}

/// One line in a list of settings: its name, a line about it, and its value
/// or switch on the right. Without `run` it only tells, but can still be
/// landed on, so a long list can be read to the end with the pad.
export function row(
  key: string,
  label: string,
  value: string | HTMLElement,
  run?: () => void,
  hint?: string
): HTMLButtonElement {
  const line = navButton(`bp-row${run ? "" : " info"}`, key, run ?? (() => {}), "rows");
  if (!run) line.dataset.hint = "";
  const words = h("span", "bp-row-words");
  words.append(h("span", "bp-row-label", label));
  if (hint) words.append(h("span", "bp-row-hint", hint));
  const right = typeof value === "string" ? h("span", "bp-row-value", value) : value;
  line.append(words, right);
  return line;
}

function toggle(on: boolean): HTMLElement {
  const track = h("span", `bp-switch${on ? " on" : ""}`);
  track.setAttribute("role", "img");
  track.setAttribute("aria-label", on ? "On" : "Off");
  track.append(h("span", "bp-switch-dot"));
  return track;
}

/// A row with a switch. The switch moves the moment it is pressed rather
/// than after the save, and moves back if the save fails.
export function switchRow(
  key: string,
  label: string,
  on: boolean,
  change: (next: boolean) => Promise<void>,
  hint?: string
): HTMLButtonElement {
  let state = on;
  const knob = toggle(on);
  const show = () => {
    knob.classList.toggle("on", state);
    knob.setAttribute("aria-label", state ? "On" : "Off");
  };
  return row(
    key,
    label,
    knob,
    async () => {
      state = !state;
      show();
      try {
        await change(state);
      } catch {
        state = !state;
        show();
      }
    },
    hint
  );
}

export function heading(text: string): HTMLElement {
  return h("h2", "bp-list-h", text);
}

/// A screen that is a list: a small line naming what it belongs to, a big
/// title, and the list itself.
export function listPage(over: string, title: string): { page: HTMLElement; list: HTMLElement } {
  const page = h("div", "bp-screen bp-scroll bp-listpage");
  const head = h("div", "bp-listpage-head");
  if (over) head.append(h("div", "bp-listpage-over", over));
  head.append(h("h1", "bp-page-title", title));
  const list = h("div", "bp-list");
  page.append(head, list);
  return { page, list };
}
