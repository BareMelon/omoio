import {
  controllerView,
  forgetController,
  padInput,
  saveController,
  setUpController,
  type ControllerView,
  type PadFamily,
} from "../api";
import { store } from "../state";
import type { View } from "./view";

/// What each place is called on each kind of pad. A place a kind does not
/// name differently falls through to COMMON.
const FAMILY_NAMES: Record<PadFamily, Record<string, string>> = {
  xbox: {
    South: "A",
    East: "B",
    West: "X",
    North: "Y",
    LB: "LB",
    RB: "RB",
    LT: "LT",
    RT: "RT",
    Back: "View",
    Start: "Menu",
    Guide: "Guide",
  },
  playstation: {
    South: "Cross",
    East: "Circle",
    West: "Square",
    North: "Triangle",
    LB: "L1",
    RB: "R1",
    LT: "L2",
    RT: "R2",
    LS: "L3",
    RS: "R3",
    Back: "Share",
    Start: "Options",
    Guide: "Home",
  },
  nintendo: {
    South: "B",
    East: "A",
    West: "Y",
    North: "X",
    LB: "L",
    RB: "R",
    LT: "ZL",
    RT: "ZR",
    Back: "Minus",
    Start: "Plus",
    Guide: "Home",
  },
  generic: {
    South: "Bottom button",
    East: "Right button",
    West: "Left button",
    North: "Top button",
    LB: "Left bumper",
    RB: "Right bumper",
    LT: "Left trigger",
    RT: "Right trigger",
    Back: "Back",
    Start: "Start",
    Guide: "Home",
  },
};

const COMMON: Record<string, string> = {
  LS: "Left stick press",
  RS: "Right stick press",
  Up: "D-pad up",
  Down: "D-pad down",
  Left: "D-pad left",
  Right: "D-pad right",
  "LS Y+": "Left stick up",
  "LS Y-": "Left stick down",
  "LS X-": "Left stick left",
  "LS X+": "Left stick right",
  "RS Y+": "Right stick up",
  "RS Y-": "Right stick down",
  "RS X-": "Right stick left",
  "RS X+": "Right stick right",
};

function nameOf(family: PadFamily, input: string): string {
  return FAMILY_NAMES[family][input] ?? COMMON[input] ?? input;
}

/// Under a d-pad or stick heading, the direction alone says enough.
const DIRECTION: Record<string, string> = {
  Up: "Up",
  Down: "Down",
  Left: "Left",
  Right: "Right",
  "LS Y+": "Up",
  "LS Y-": "Down",
  "LS X-": "Left",
  "LS X+": "Right",
  "RS Y+": "Up",
  "RS Y-": "Down",
  "RS X-": "Left",
  "RS X+": "Right",
};

/// The places in two columns, grouped the way hands find them. Directions
/// mean the same on every console, so only the other groups say what each
/// console calls a place.
type Group = { title: string; places: string[]; directions?: boolean };

const COLUMNS: Group[][] = [
  [
    { title: "Face buttons", places: ["South", "East", "West", "North"] },
    { title: "D-pad", places: ["Up", "Down", "Left", "Right"], directions: true },
    { title: "Middle", places: ["Back", "Start", "Guide"] },
  ],
  [
    { title: "Shoulders", places: ["LB", "RB", "LT", "RT"] },
    { title: "Stick presses", places: ["LS", "RS"] },
    { title: "Left stick", places: ["LS Y+", "LS Y-", "LS X-", "LS X+"], directions: true },
    { title: "Right stick", places: ["RS Y+", "RS Y-", "RS X-", "RS X+"], directions: true },
  ],
];

/// The part of the drawing an input belongs to. A stick's four directions
/// are all the stick.
function partOf(input: string): string {
  if (input.startsWith("LS")) return "LS";
  if (input.startsWith("RS")) return "RS";
  return input;
}

/// Where things sit. Xbox and Nintendo pads put the left stick above the
/// d-pad; PlayStation pads put both sticks low and side by side, with a
/// touchpad between the middle buttons. A pad of no known kind gets the
/// first, the more common shape.
type Spots = {
  ls: [number, number];
  rs: [number, number];
  dpad: [number, number];
  face: [number, number];
  back: [number, number];
  start: [number, number];
  guide: [number, number];
  guideSize: number;
  touchpad: boolean;
};

const OFFSET: Spots = {
  ls: [118, 100],
  rs: [246, 150],
  dpad: [156, 152],
  face: [292, 98],
  back: [173, 98],
  start: [227, 98],
  guide: [200, 66],
  guideSize: 13,
  touchpad: false,
};

const SIDE_BY_SIDE: Spots = {
  ls: [150, 154],
  rs: [250, 154],
  dpad: [104, 102],
  face: [296, 102],
  back: [140, 62],
  start: [260, 62],
  guide: [200, 142],
  guideSize: 9,
  touchpad: true,
};

const TRIGGERS: Record<PadFamily, [string, string]> = {
  xbox: ["LT", "RT"],
  playstation: ["L2", "R2"],
  nintendo: ["ZL", "ZR"],
  generic: ["LT", "RT"],
};

/// What is printed on a face button: a letter, or on a PlayStation pad a
/// shape. A pad of no known kind gets nothing rather than a guess.
function faceMark(family: PadFamily, place: string, x: number, y: number): string {
  if (family === "playstation") {
    switch (place) {
      case "North":
        return `<path class="pad-glyph" d="M${x} ${y - 5.5}L${x + 5.5} ${y + 4}H${x - 5.5}Z"/>`;
      case "East":
        return `<circle class="pad-glyph" cx="${x}" cy="${y}" r="5"/>`;
      case "South":
        return `<path class="pad-glyph" d="M${x - 4.5} ${y - 4.5}L${x + 4.5} ${y + 4.5}M${x + 4.5} ${y - 4.5}L${x - 4.5} ${y + 4.5}"/>`;
      default:
        return `<rect class="pad-glyph" x="${x - 4.5}" y="${y - 4.5}" width="9" height="9"/>`;
    }
  }
  if (family === "generic") return "";
  return `<text class="pad-face" x="${x}" y="${y + 4}">${FAMILY_NAMES[family][place]}</text>`;
}

/// The kind of pad this player has, drawn from the tokens so it follows the
/// rest of the app. Each input carries its place as `data-part`, so a row or
/// a press can light it.
function padArt(family: PadFamily): string {
  const spots = family === "playstation" ? SIDE_BY_SIDE : OFFSET;
  const [lt, rt] = TRIGGERS[family];
  const [dx, dy] = spots.dpad;
  const [fx, fy] = spots.face;
  const face: [string, number, number][] = [
    ["North", fx, fy - 22],
    ["East", fx + 22, fy],
    ["South", fx, fy + 22],
    ["West", fx - 22, fy],
  ];
  const pill = (part: string, [x, y]: [number, number]) =>
    `<rect data-part="${part}" x="${x - 9}" y="${y - 5}" width="18" height="10" rx="5"/>`;
  return `
    <svg class="pad-art" viewBox="0 0 400 260" aria-hidden="true">
      <rect data-part="LT" x="86" y="6" width="54" height="24" rx="9"/>
      <rect data-part="RT" x="260" y="6" width="54" height="24" rx="9"/>
      <path data-part="LB" d="M60 52C68 36 96 28 150 30L152 43C106 43 86 47 74 58Z"/>
      <path data-part="RB" d="M340 52C332 36 304 28 250 30L248 43C294 43 314 47 326 58Z"/>
      <text class="pad-label" x="113" y="22">${lt}</text>
      <text class="pad-label" x="287" y="22">${rt}</text>
      <path class="pad-body" d="M88 44C120 32 280 32 312 44C350 58 372 100 384 150C396 200 392 238 360 246C334 252 314 232 296 206C284 190 272 184 256 184H144C128 184 116 190 104 206C86 232 66 252 40 246C8 238 4 200 16 150C28 100 50 58 88 44Z"/>
      ${spots.touchpad ? `<rect class="pad-plain" x="160" y="46" width="80" height="44" rx="8"/>` : ""}
      <circle data-part="Guide" cx="${spots.guide[0]}" cy="${spots.guide[1]}" r="${spots.guideSize}"/>
      ${pill("Back", spots.back)}
      ${pill("Start", spots.start)}
      <circle data-part="LS" cx="${spots.ls[0]}" cy="${spots.ls[1]}" r="23"/>
      <circle data-part="RS" cx="${spots.rs[0]}" cy="${spots.rs[1]}" r="23"/>
      <rect class="pad-plain" x="${dx - 10}" y="${dy - 10}" width="20" height="20"/>
      <rect data-part="Up" x="${dx - 10}" y="${dy - 28}" width="20" height="19" rx="3"/>
      <rect data-part="Down" x="${dx - 10}" y="${dy + 9}" width="20" height="19" rx="3"/>
      <rect data-part="Left" x="${dx - 28}" y="${dy - 10}" width="19" height="20" rx="3"/>
      <rect data-part="Right" x="${dx + 9}" y="${dy - 10}" width="19" height="20" rx="3"/>
      ${face.map(([place, x, y]) => `<circle data-part="${place}" cx="${x}" cy="${y}" r="11"/>`).join("")}
      ${face.map(([place, x, y]) => faceMark(family, place, x, y)).join("")}
    </svg>`;
}

/// The player whose buttons are on screen. Kept across redraws, so giving a
/// player a pad does not jump back to player 1.
let shownPlayer = 0;

/// How long a row waits for a press before giving up.
const LISTEN_MS = 6000;

function scopePicker(scope: string): HTMLElement {
  const games = (store.get().games ?? []).filter((game) => game.set_up);
  const select = document.createElement("select");
  select.className = "select";
  select.add(new Option("Every game", ""));
  for (const game of games) select.add(new Option(game.title, game.title_id));
  select.value = scope;
  select.onchange = () => store.setControllerScope(select.value);
  return select;
}

/// One player: who they are, which pad is theirs, and whether it is plugged
/// in. The pad can be chosen before it is.
function playerCard(view: ControllerView, index: number, scope: string, note: HTMLElement): HTMLElement {
  const player = view.players[index];
  const card = document.createElement("div");
  card.className = index === shownPlayer ? "player-card on" : "player-card";
  card.tabIndex = 0;
  card.setAttribute("role", "button");
  card.setAttribute("aria-pressed", String(index === shownPlayer));
  const show = () => {
    shownPlayer = index;
    store.redraw();
  };
  card.onclick = show;
  card.onkeydown = (event) => {
    if (event.target === card && (event.key === "Enter" || event.key === " ")) {
      event.preventDefault();
      show();
    }
  };

  card.innerHTML = `
    <div class="player-top">
      <span class="player-num">${index + 1}</span>
      <span class="player-label">Player ${index + 1}</span>
    </div>
    <div class="player-pad"><span class="dot"></span><span class="player-pad-name"></span></div>
    <div class="player-state"></div>
  `;
  card.querySelector<HTMLElement>(".player-pad-name")!.textContent = player.pad.name;
  card.querySelector(".dot")!.classList.toggle("on", player.connected);
  card.querySelector<HTMLElement>(".player-state")!.textContent = player.connected
    ? "Plugged in"
    : "Waiting for this pad";

  const select = document.createElement("select");
  select.className = "select player-select";
  select.setAttribute("aria-label", `Pad for player ${index + 1}`);
  for (const pad of view.pads) {
    const plugged = view.connected.some((c) => c.device === pad.device);
    select.add(new Option(plugged ? `${pad.name} (plugged in)` : pad.name, pad.device));
  }
  select.value = player.pad.device;
  select.onclick = (event) => event.stopPropagation();
  select.onchange = async () => {
    const pad = view.pads.find((p) => p.device === select.value);
    if (!pad) return;
    shownPlayer = index;
    try {
      await saveController(scope, index + 1, pad, player.buttons);
      note.textContent = "";
    } catch (err) {
      note.textContent = typeof err === "string" ? err : "Couldn't save the controller settings.";
    }
    store.redraw();
  };
  card.appendChild(select);
  return card;
}

export async function renderController(): Promise<View> {
  const scope = store.get().controllerScope ?? "";
  const view = await controllerView(scope);
  if (shownPlayer >= view.players.length) shownPlayer = 0;
  const content = document.createElement("div");
  content.className = "controller";

  const note = document.createElement("div");
  note.className = "note plain";

  const head = document.createElement("div");
  head.className = "pad-head";
  const status = document.createElement("div");
  status.className = "pad-status";
  const plugged = view.players.filter((p) => p.connected).length;
  status.textContent = !view.saved
    ? "Nothing saved yet. Pressing Play sets up these four players."
    : scope && !view.own
      ? "This game uses the layout for every game."
      : plugged === 0
        ? "No pads plugged in. Each player waits for theirs."
        : `${plugged} of ${view.players.length} players have their pad plugged in.`;
  head.appendChild(status);

  const actions = document.createElement("div");
  actions.className = "row-actions";
  actions.appendChild(scopePicker(scope));
  const reset = document.createElement("button");
  reset.className = "small-btn";
  reset.textContent = view.saved ? "Restore defaults" : "Save these players";
  reset.onclick = async () => {
    try {
      await setUpController(scope);
      store.redraw();
    } catch (err) {
      note.textContent = typeof err === "string" ? err : "Couldn't set up the controllers.";
    }
  };
  actions.appendChild(reset);
  if (scope && view.own) {
    const back = document.createElement("button");
    back.className = "link-btn";
    back.textContent = "Use the layout for every game";
    back.onclick = async () => {
      await forgetController(scope);
      store.redraw();
    };
    actions.appendChild(back);
  }
  head.appendChild(actions);
  content.append(head, note);

  const players = document.createElement("div");
  players.className = "players";
  view.players.forEach((_, index) => players.appendChild(playerCard(view, index, scope, note)));
  content.appendChild(players);

  const how = document.createElement("div");
  how.className = "note plain";
  how.textContent =
    "One layout works in every emulator. A pad plugged in that no player has takes the place of the first player whose pad is missing when you press Play.";
  content.appendChild(how);

  if (scope && !view.own) {
    const hint = document.createElement("div");
    hint.className = "note plain";
    hint.textContent = "Changing a button here gives this game a layout of its own.";
    content.appendChild(hint);
  }

  // The chosen player's buttons: their pad on the left, each place on it to
  // the right with what it does on every console.
  const player = view.players[shownPlayer];
  const pad = player.pad;
  const family = pad.family;
  const buttons: Record<string, string> = { ...player.buttons };
  const save = async () => {
    try {
      await saveController(scope, shownPlayer + 1, pad, buttons);
      note.textContent = "";
      if (!view.own || !view.saved) store.redraw();
    } catch (err) {
      note.textContent = typeof err === "string" ? err : "Couldn't save the controller settings.";
    }
  };

  /// Gives `place` this input. An input another place had is swapped over,
  /// so no button ever does two things by accident.
  const assign = (place: string, input: string) => {
    const was = buttons[place];
    for (const other of Object.keys(buttons)) {
      if (other !== place && buttons[other] === input) buttons[other] = was;
    }
    buttons[place] = input;
  };

  const title = document.createElement("div");
  title.className = "map-title";
  title.textContent = `Buttons for player ${shownPlayer + 1}`;
  content.appendChild(title);

  const map = document.createElement("div");
  map.className = "pad-map";

  const stage = document.createElement("div");
  stage.className = "pad-stage";
  stage.innerHTML = padArt(family);
  const svg = stage.querySelector("svg")!;
  const test = document.createElement("div");
  test.className = "pad-test";
  test.textContent = player.connected
    ? `Press anything on ${pad.name} and it lights up here.`
    : `Switch on ${pad.name} to test it and record buttons. Until then, choose from a list.`;
  stage.appendChild(test);

  const part = (input: string) => svg.querySelector(`[data-part="${partOf(input)}"]`);

  const chips = new Map<string, HTMLButtonElement>();
  const rows = new Map<string, HTMLElement>();
  const paint = () => {
    for (const [place, chip] of chips) {
      if (!chip.classList.contains("listening")) chip.textContent = nameOf(family, buttons[place]);
    }
  };

  let listening: { place: string; chip: HTMLButtonElement; until: number } | null = null;
  const stopListening = () => {
    listening?.chip.classList.remove("listening");
    listening = null;
    paint();
  };

  /// Without the pad to press, a row opens a list of every input instead.
  const choose = (place: string, chip: HTMLButtonElement) => {
    const select = document.createElement("select");
    select.className = "bind-v";
    for (const input of view.inputs) select.add(new Option(nameOf(family, input), input));
    select.value = buttons[place];
    select.onchange = () => {
      assign(place, select.value);
      void save();
      select.replaceWith(chip);
      paint();
    };
    select.onblur = () => {
      if (select.isConnected) select.replaceWith(chip);
    };
    chip.replaceWith(select);
    select.focus();
  };

  const row = (group: Group, place: string): HTMLElement => {
    const line = document.createElement("div");
    line.className = "bind-row";
    const label = group.directions ? DIRECTION[place] : nameOf(family, place);
    const left = document.createElement("div");
    const name = document.createElement("div");
    name.className = "bind-k";
    name.textContent = label;
    left.appendChild(name);
    if (!group.directions) {
      const said = view.consoles
        .filter((c) => c.buttons[place] && c.buttons[place] !== label)
        .map((c) => `${c.name} ${c.buttons[place]}`)
        .join(" · ");
      if (said) {
        const sub = document.createElement("div");
        sub.className = "bind-sub";
        sub.textContent = said;
        left.appendChild(sub);
      }
    }

    const chip = document.createElement("button");
    chip.className = "bind-v";
    chip.setAttribute("aria-label", `${label}: change button`);
    chip.onclick = () => {
      if (!player.connected) return choose(place, chip);
      stopListening();
      listening = { place, chip, until: Date.now() + LISTEN_MS };
      chip.classList.add("listening");
      chip.textContent = "Press a button…";
    };
    chip.onkeydown = (event) => {
      if (event.key === "Escape" && listening?.chip === chip) stopListening();
    };
    chips.set(place, chip);
    rows.set(place, line);

    const hot = (on: boolean) => part(buttons[place])?.classList.toggle("hot", on);
    line.onmouseenter = () => hot(true);
    line.onmouseleave = () => hot(false);
    chip.onfocus = () => hot(true);
    chip.onblur = () => hot(false);

    line.append(left, chip);
    return line;
  };

  const columns = document.createElement("div");
  columns.className = "bind-cols";
  for (const groups of COLUMNS) {
    const col = document.createElement("div");
    for (const group of groups) {
      const box = document.createElement("div");
      box.className = "bind-group";
      const heading = document.createElement("div");
      heading.className = "sec-h";
      heading.textContent = group.title;
      box.appendChild(heading);
      group.places.forEach((place) => box.appendChild(row(group, place)));
      col.appendChild(box);
    }
    columns.appendChild(col);
  }
  paint();

  map.append(stage, columns);
  content.appendChild(map);

  // While this screen is up, the pad is read several times a second: what is
  // held lights on the drawing, and a row waiting for a press takes the first
  // button that goes down. A pad switched on or off draws the screen again,
  // so the card, the note and recording follow. It all stops once the screen
  // is replaced. A pad that is not there is asked about less often.
  let held = new Set<string>();
  let busy = false;
  const timer = window.setInterval(async () => {
    if (!content.isConnected) {
      window.clearInterval(timer);
      return;
    }
    if (busy) return;
    busy = true;
    try {
      const answer = await padInput(pad.device);
      if ((answer !== null) !== player.connected) {
        window.clearInterval(timer);
        store.redraw();
        return;
      }
      if (answer === null) return;
      const now = new Set(answer);
      for (const el of svg.querySelectorAll("[data-part]")) {
        el.classList.toggle("down", [...now].some((input) => partOf(input) === el.getAttribute("data-part")));
      }
      for (const [place, line] of rows) line.classList.toggle("flash", now.has(buttons[place]));
      if (listening) {
        const pressed = [...now].find((input) => !held.has(input));
        if (pressed) {
          assign(listening.place, pressed);
          stopListening();
          void save();
        } else if (Date.now() > listening.until) {
          stopListening();
        }
      }
      held = now;
    } finally {
      busy = false;
    }
  }, player.connected ? 60 : 500);

  const subtitle =
    view.connected.length === 0
      ? "Nothing plugged in right now"
      : view.connected.length === 1
        ? "1 controller plugged in"
        : `${view.connected.length} controllers plugged in`;
  return { title: "Controller", subtitle, content };
}
