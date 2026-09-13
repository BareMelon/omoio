import "./styles/tokens.css";
import "./styles/portal.css";
import {
  closePortalMenu,
  figureCharacters,
  figures as listFigures,
  onPortalMenu,
  padsHeld,
  portalClear,
  portalCreate,
  portalFigures,
  portalLoad,
  portalMenuFamily,
  type Character,
  type Figure,
  type PadFamily,
} from "./api";
import { nameOf } from "./components/padNames";

/// The Skylanders menu, drawn by Omoio over the running game and used with
/// the pad alone: d-pad or left stick to move, the bottom face button to
/// choose, the left one to take a figure off, the right one to go back or
/// close. Mouse and keyboard work as well.
///
/// "Your figures" starts with New figure, which lists every character the
/// emulator can make. Picking one has the emulator make that figure and put
/// it on the portal, so nobody needs figure files of their own.

/// How many tiles sit side by side, which is also how far up or down moves.
const COLUMNS = 5;

/// The shoulder buttons move this many rows at once through the long list of
/// characters.
const PAGE_ROWS = 4;

/// A direction held down keeps moving after a pause, as on a console.
const REPEAT_AFTER = 380;
const REPEAT_EVERY = 140;

type Move = "up" | "down" | "left" | "right" | "page-up" | "page-down";

const MOVES: Record<string, Move> = {
  Up: "up",
  "LS Y+": "up",
  Down: "down",
  "LS Y-": "down",
  Left: "left",
  "LS X-": "left",
  Right: "right",
  "LS X+": "right",
  LB: "page-up",
  RB: "page-down",
};

type Section = "portal" | "mine";

const root = document.getElementById("portal")!;

let family: PadFamily = "generic";
let shown = false;
let busy = false;
let onPortal: string[] = [];
let mine: Figure[] = [];
let section: Section = "mine";
/// The selection in the main view. In "Your figures", 0 is New figure and
/// the user's files follow from 1.
let at = 0;
let status = "";

/// The list of characters, once asked for, and the selection in it.
let picking = false;
let characters: Character[] = [];
let pick = 0;

function placed(): { name: string; slot: number }[] {
  return onPortal.map((name, slot) => ({ name, slot })).filter((figure) => figure.name);
}

function count(which: Section): number {
  return which === "portal" ? placed().length : mine.length + 1;
}

/// Keeps the selection on something that is there. An empty portal row
/// hands it to "Your figures", which always has New figure.
function settle() {
  if (count(section) === 0) section = "mine";
  at = Math.max(0, Math.min(at, count(section) - 1));
  pick = Math.max(0, Math.min(pick, characters.length - 1));
}

function tile(label: string, detail: string, selected: boolean, act: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.className = selected ? "portal-item sel" : "portal-item";
  if (detail) {
    const small = document.createElement("span");
    small.className = "portal-detail";
    small.textContent = detail;
    button.appendChild(small);
  }
  const name = document.createElement("span");
  name.textContent = label;
  button.appendChild(name);
  button.onclick = act;
  return button;
}

function emptyLine(text: string): HTMLElement {
  const line = document.createElement("div");
  line.className = "portal-empty";
  line.textContent = text;
  return line;
}

function panel(title: string, keys: string, body: string): void {
  root.innerHTML = `
    <div class="portal-panel" role="dialog" aria-label="Portal">
      <div class="portal-head">
        <div class="portal-title"></div>
        <div class="portal-keys"></div>
      </div>
      ${body}
      <div class="portal-status" role="status"></div>
    </div>`;
  root.querySelector<HTMLElement>(".portal-title")!.textContent = title;
  root.querySelector<HTMLElement>(".portal-keys")!.textContent = keys;
  root.querySelector<HTMLElement>(".portal-status")!.textContent = status;
}

function render() {
  settle();
  if (picking) return renderCharacters();

  panel(
    "Portal",
    `${nameOf(family, "South")} to choose · ${nameOf(family, "West")} to take off · ${nameOf(family, "East")} to close`,
    `<div class="portal-sec">On the portal</div>
     <div class="portal-row"></div>
     <div class="portal-sec">Your figures</div>
     <div class="portal-grid"></div>`
  );

  const row = root.querySelector<HTMLElement>(".portal-row")!;
  const on = placed();
  if (on.length === 0) row.appendChild(emptyLine("Nothing on the portal."));
  on.forEach((figure, index) =>
    row.appendChild(
      tile(figure.name, `Slot ${figure.slot + 1}`, section === "portal" && index === at, () => {
        section = "portal";
        at = index;
        void choose();
      })
    )
  );

  const grid = root.querySelector<HTMLElement>(".portal-grid")!;
  grid.appendChild(
    tile("New figure", "Any character", section === "mine" && at === 0, () => {
      section = "mine";
      at = 0;
      void choose();
    })
  );
  mine.forEach((figure, index) =>
    grid.appendChild(
      tile(figure.name, "", section === "mine" && index + 1 === at, () => {
        section = "mine";
        at = index + 1;
        void choose();
      })
    )
  );

  root.querySelector<HTMLElement>(".sel")?.scrollIntoView({ block: "nearest" });
}

function renderCharacters() {
  panel(
    "New figure",
    `${nameOf(family, "South")} to make and place · ${nameOf(family, "LB")} and ${nameOf(family, "RB")} to page · ${nameOf(family, "East")} to go back`,
    `<div class="portal-grid"></div>`
  );
  const grid = root.querySelector<HTMLElement>(".portal-grid")!;
  characters.forEach((character, index) =>
    grid.appendChild(
      tile(character.name, "", index === pick, () => {
        pick = index;
        void make();
      })
    )
  );
  root.querySelector<HTMLElement>(".sel")?.scrollIntoView({ block: "nearest" });
}

function moveMain(move: Move) {
  const total = mine.length + 1;
  if (section === "portal") {
    if (move === "left") at -= 1;
    else if (move === "right") at += 1;
    else if (move === "down") {
      section = "mine";
      at = Math.min(at, COLUMNS - 1);
    }
  } else if (move === "left" && at % COLUMNS > 0) {
    at -= 1;
  } else if (move === "right" && at % COLUMNS < COLUMNS - 1 && at + 1 < total) {
    at += 1;
  } else if (move === "down" && at + COLUMNS < total) {
    at += COLUMNS;
  } else if (move === "up") {
    if (at >= COLUMNS) {
      at -= COLUMNS;
    } else if (placed().length > 0) {
      section = "portal";
      at = Math.min(at, placed().length - 1);
    }
  }
  render();
}

function moveCharacters(move: Move) {
  const last = characters.length - 1;
  const steps: Record<Move, number> = {
    left: pick % COLUMNS > 0 ? -1 : 0,
    right: pick % COLUMNS < COLUMNS - 1 ? 1 : 0,
    up: -COLUMNS,
    down: COLUMNS,
    "page-up": -COLUMNS * PAGE_ROWS,
    "page-down": COLUMNS * PAGE_ROWS,
  };
  pick = Math.max(0, Math.min(last, pick + steps[move]));
  render();
}

/// One change to the portal at a time, since each goes through the
/// emulator's own window and takes a moment.
async function change(saying: string, job: () => Promise<string[]>, said: (names: string[]) => string) {
  if (busy) return;
  busy = true;
  status = saying;
  render();
  try {
    onPortal = await job();
    status = said(onPortal);
    mine = await listFigures();
  } catch (err) {
    status = typeof err === "string" ? err : "That didn't work. Try again.";
  } finally {
    busy = false;
    render();
  }
}

/// The first empty slot, or -1 when the portal is full. Before the portal
/// has been read, slot 1.
function freeSlot(): number {
  return onPortal.length === 0 ? 0 : onPortal.indexOf("");
}

function portalFull() {
  status = "The portal is full. Take a figure off first.";
  render();
}

async function takeOff() {
  settle();
  if (section !== "portal") return;
  const figure = placed()[at];
  if (!figure) return;
  await change(`Taking ${figure.name} off…`, () => portalClear(figure.slot), () => `${figure.name} is off the portal.`);
}

async function choose() {
  settle();
  if (section === "portal") return takeOff();
  if (at === 0) return openCharacters();
  const figure = mine[at - 1];
  if (!figure) return;
  const slot = freeSlot();
  if (slot < 0) return portalFull();
  await change(
    `Putting ${figure.name} on the portal…`,
    () => portalLoad(slot, figure.path),
    (names) => `${names[slot] || figure.name} is on the portal.`
  );
}

/// The list of characters comes from the emulator's own figure maker, the
/// first time only; Omoio keeps it after that.
async function openCharacters() {
  picking = true;
  if (characters.length === 0) {
    status = "Getting the characters…";
    render();
    try {
      characters = (await figureCharacters()).sort((a, b) => a.name.localeCompare(b.name));
      status = "";
    } catch (err) {
      status = typeof err === "string" ? err : "Couldn't get the characters.";
    }
  }
  render();
}

async function make() {
  const character = characters[pick];
  if (!character || busy) return;
  const slot = freeSlot();
  if (slot < 0) {
    picking = false;
    return portalFull();
  }
  picking = false;
  section = "portal";
  await change(
    `Making ${character.name}…`,
    () => portalCreate(slot, character),
    (names) => `${names[slot] || character.name} is on the portal.`
  );
}

async function refresh() {
  status = "Reading the portal…";
  render();
  const [names, files] = await Promise.all([
    portalFigures().catch((err: unknown) => {
      status = typeof err === "string" ? err : "Couldn't read the portal.";
      return null;
    }),
    listFigures(),
  ]);
  if (names) {
    onPortal = names;
    status = "";
  }
  mine = files;
  render();
}

// ---- the pad ----

let held = new Set<string>();
const downSince = new Map<string, number>();
let lastRepeat = 0;
let reading = false;

async function readPads(): Promise<Set<string>> {
  try {
    return new Set(await padsHeld());
  } catch {
    return new Set();
  }
}

function back() {
  if (picking) {
    picking = false;
    render();
  } else {
    void closePortalMenu();
  }
}

function press(input: string) {
  const move = MOVES[input];
  if (move) return picking ? moveCharacters(move) : moveMain(move);
  if (input === "East") return back();
  if (busy) return;
  if (input === "South") void (picking ? make() : choose());
  else if (input === "West" && !picking) void takeOff();
}

window.setInterval(async () => {
  if (!shown || reading) return;
  reading = true;
  try {
    const now = await readPads();
    const time = Date.now();
    for (const input of now) {
      if (!held.has(input)) {
        downSince.set(input, time);
        press(input);
      } else if (
        MOVES[input] &&
        time - (downSince.get(input) ?? time) > REPEAT_AFTER &&
        time - lastRepeat > REPEAT_EVERY
      ) {
        lastRepeat = time;
        press(input);
      }
    }
    held = now;
  } finally {
    reading = false;
  }
}, 50);

document.addEventListener("keydown", (event) => {
  const keys: Record<string, () => void> = {
    ArrowUp: () => press("Up"),
    ArrowDown: () => press("Down"),
    ArrowLeft: () => press("Left"),
    ArrowRight: () => press("Right"),
    PageUp: () => press("LB"),
    PageDown: () => press("RB"),
    Enter: () => press("South"),
    Delete: () => press("West"),
    Backspace: () => press("West"),
    Escape: () => press("East"),
  };
  const act = keys[event.key];
  if (act) {
    event.preventDefault();
    act();
  }
});

/// Shown again: what is already held down, such as the button that opened
/// the menu, is not taken as a press.
async function show() {
  family = (await portalMenuFamily()) as PadFamily;
  held = await readPads();
  picking = false;
  shown = true;
  await refresh();
}

void onPortalMenu((state) => {
  family = state.family as PadFamily;
  if (state.open) void show();
  else shown = false;
});
void show();
