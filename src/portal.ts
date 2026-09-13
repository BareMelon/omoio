import "./styles/tokens.css";
import "./styles/portal.css";
import {
  closePortalMenu,
  figures as listFigures,
  onPortalMenu,
  padsHeld,
  portalClear,
  portalFigures,
  portalLoad,
  portalMenuFamily,
  type Figure,
  type PadFamily,
} from "./api";
import { nameOf } from "./components/padNames";

/// The Skylanders menu, drawn by Omoio over the running game and used with
/// the pad alone: d-pad or left stick to move, the bottom face button to
/// choose, the left one to take a figure off, the right one to close. Mouse
/// and keyboard work as well.

/// How many figure tiles sit side by side, which is also how far up or down
/// moves in the grid.
const COLUMNS = 5;

/// A direction held down keeps moving after a pause, as on a console.
const REPEAT_AFTER = 380;
const REPEAT_EVERY = 140;

const DIRECTIONS: Record<string, "up" | "down" | "left" | "right"> = {
  Up: "up",
  "LS Y+": "up",
  Down: "down",
  "LS Y-": "down",
  Left: "left",
  "LS X-": "left",
  Right: "right",
  "LS X+": "right",
};

type Section = "portal" | "mine";

const root = document.getElementById("portal")!;

let family: PadFamily = "generic";
let shown = false;
let busy = false;
let onPortal: string[] = [];
let mine: Figure[] = [];
let section: Section = "mine";
let at = 0;
let status = "";

function placed(): { name: string; slot: number }[] {
  return onPortal.map((name, slot) => ({ name, slot })).filter((figure) => figure.name);
}

function count(which: Section): number {
  return which === "portal" ? placed().length : mine.length;
}

/// Keeps the selection on something that is there. An empty section hands it
/// to the other one.
function settle() {
  if (count(section) === 0) {
    const other: Section = section === "portal" ? "mine" : "portal";
    if (count(other) > 0) section = other;
  }
  at = Math.max(0, Math.min(at, count(section) - 1));
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

function render() {
  settle();
  root.innerHTML = `
    <div class="portal-panel" role="dialog" aria-label="Portal">
      <div class="portal-head">
        <div class="portal-title">Portal</div>
        <div class="portal-keys"></div>
      </div>
      <div class="portal-sec">On the portal</div>
      <div class="portal-row"></div>
      <div class="portal-sec">Your figures</div>
      <div class="portal-grid"></div>
      <div class="portal-status" role="status"></div>
    </div>`;
  root.querySelector<HTMLElement>(".portal-keys")!.textContent =
    `${nameOf(family, "South")} to choose · ${nameOf(family, "West")} to take off · ${nameOf(family, "East")} to close`;

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
  if (mine.length === 0) {
    grid.appendChild(emptyLine("No figure files yet. Add yours in Omoio, under Settings, Toy figures."));
  }
  mine.forEach((figure, index) =>
    grid.appendChild(
      tile(figure.name, "", section === "mine" && index === at, () => {
        section = "mine";
        at = index;
        void choose();
      })
    )
  );

  root.querySelector<HTMLElement>(".portal-status")!.textContent = status;
  root.querySelector<HTMLElement>(".sel")?.scrollIntoView({ block: "nearest" });
}

function move(direction: "up" | "down" | "left" | "right") {
  if (section === "portal") {
    if (direction === "left") at -= 1;
    else if (direction === "right") at += 1;
    else if (direction === "down" && mine.length > 0) {
      section = "mine";
      at = Math.min(at, COLUMNS - 1);
    }
  } else if (direction === "left" && at % COLUMNS > 0) {
    at -= 1;
  } else if (direction === "right" && at % COLUMNS < COLUMNS - 1 && at + 1 < mine.length) {
    at += 1;
  } else if (direction === "down" && at + COLUMNS < mine.length) {
    at += COLUMNS;
  } else if (direction === "up") {
    if (at >= COLUMNS) {
      at -= COLUMNS;
    } else if (placed().length > 0) {
      section = "portal";
      at = Math.min(at, placed().length - 1);
    }
  }
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
  const figure = mine[at];
  if (!figure) return;
  const slot = onPortal.length === 0 ? 0 : onPortal.indexOf("");
  if (slot < 0) {
    status = "The portal is full. Take a figure off first.";
    render();
    return;
  }
  await change(
    `Putting ${figure.name} on the portal…`,
    () => portalLoad(slot, figure.path),
    (names) => `${names[slot] || figure.name} is on the portal.`
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

function press(input: string) {
  const direction = DIRECTIONS[input];
  if (direction) return move(direction);
  if (input === "East") return void closePortalMenu();
  if (busy) return;
  if (input === "South") void choose();
  else if (input === "West") void takeOff();
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
        DIRECTIONS[input] &&
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
    ArrowUp: () => move("up"),
    ArrowDown: () => move("down"),
    ArrowLeft: () => move("left"),
    ArrowRight: () => move("right"),
    Enter: () => void choose(),
    Delete: () => void takeOff(),
    Backspace: () => void takeOff(),
    Escape: () => void closePortalMenu(),
  };
  const act = keys[event.key];
  if (act) {
    event.preventDefault();
    act();
  }
});

/// Shown again: what is already held down, such as the Guide button that
/// opened the menu, is not taken as a press.
async function show() {
  family = (await portalMenuFamily()) as PadFamily;
  held = await readPads();
  shown = true;
  await refresh();
}

void onPortalMenu((state) => {
  family = state.family as PadFamily;
  if (state.open) void show();
  else shown = false;
});
void show();
