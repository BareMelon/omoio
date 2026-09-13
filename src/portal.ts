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
  type Figure,
  type FigureElement,
  type FigureKind,
  type Offer,
  type PadFamily,
} from "./api";
import { nameOf } from "./components/padNames";

/// The Skylanders menu, drawn by Omoio over the running game and used with
/// the pad alone. The shoulder buttons go through the tabs: Saved, then one
/// tab to each element, then items such as the treasure chest and the
/// swords, traps and adventure packs. The d-pad or left stick moves, the
/// bottom face button puts a figure on the portal, the left one takes it
/// off, the right one closes. Mouse and keyboard work as well.
///
/// A character is made by the emulator's own figure maker the first time it
/// is chosen, and saved. After that the saved figure goes on, so it keeps
/// what it has earned.

/// How many tiles sit side by side, which is also how far up or down moves.
const COLUMNS = 5;

/// A direction held down keeps moving after a pause, as on a console.
const REPEAT_AFTER = 380;
const REPEAT_EVERY = 140;

type Move = "up" | "down" | "left" | "right";

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

const ELEMENTS: [FigureElement, string][] = [
  ["air", "Air"],
  ["earth", "Earth"],
  ["fire", "Fire"],
  ["water", "Water"],
  ["life", "Life"],
  ["undead", "Undead"],
  ["magic", "Magic"],
  ["tech", "Tech"],
  ["light", "Light"],
  ["dark", "Dark"],
];

const KINDS: [FigureKind, string][] = [
  ["item", "Items"],
  ["trap", "Traps"],
  ["adventure", "Adventure packs"],
  ["vehicle", "Vehicles"],
  ["trophy", "Trophies"],
];

/// A mark for each element, and for the kinds that have none, in place of
/// pictures of the figures, which Omoio doesn't have.
const MARKS: Record<string, string> = {
  air: `<path d="M3 8h11a3 3 0 1 0-3-3M3 12h15a3 3 0 1 1-3 3M3 16h8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>`,
  earth: `<path d="M2 20 9 8l4 6 3-4 6 10z" fill="currentColor"/>`,
  fire: `<path d="M12 2c1 4 6 6.5 6 12a6 6 0 0 1-12 0c0-2.6 1.3-4 2.5-5 0 2 .8 3.3 2 3.8C10 9 10.8 5 12 2z" fill="currentColor"/>`,
  water: `<path d="M12 2.5c3.5 5 6.5 8.4 6.5 12a6.5 6.5 0 0 1-13 0c0-3.6 3-7 6.5-12z" fill="currentColor"/>`,
  life: `<path d="M4 20C4 11 9 4 20 4c0 11-7 16-16 16zM4 20l9-9" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>`,
  undead: `<path fill-rule="evenodd" d="M12 2.5a8 8 0 0 0-8 8c0 2.8 1.3 4.6 3.2 5.7V20a1 1 0 0 0 1 1h7.6a1 1 0 0 0 1-1v-3.8c1.9-1.1 3.2-2.9 3.2-5.7a8 8 0 0 0-8-8zM9 9.5a1.8 1.8 0 1 0 0 3.6 1.8 1.8 0 0 0 0-3.6zm6 0a1.8 1.8 0 1 0 0 3.6 1.8 1.8 0 0 0 0-3.6z" fill="currentColor"/>`,
  magic: `<path d="m12 2 2.6 6.6 7.1.5-5.5 4.6 1.8 6.9L12 16.8l-6 3.8 1.8-6.9-5.5-4.6 7.1-.5z" fill="currentColor"/>`,
  tech: `<circle cx="12" cy="12" r="4.5" fill="none" stroke="currentColor" stroke-width="2.5"/><path d="M12 2v4M12 18v4M2 12h4M18 12h4M4.9 4.9l2.8 2.8M16.3 16.3l2.8 2.8M4.9 19.1l2.8-2.8M16.3 7.7l2.8-2.8" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>`,
  light: `<circle cx="12" cy="12" r="4" fill="currentColor"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9 7 7M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>`,
  dark: `<path d="M15.5 3A9 9 0 1 0 21 17.5 7.5 7.5 0 0 1 15.5 3z" fill="currentColor"/>`,
  item: `<path d="M3 10h18v10H3zM3 10l2-5h14l2 5M10 14h4" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>`,
  adventure: `<path d="M5 21V3M5 4h13l-3 4 3 4H5" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>`,
  vehicle: `<circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="12" cy="12" r="2.5" fill="currentColor"/>`,
  trophy: `<path d="M7 3h10v5a5 5 0 0 1-10 0zM7 5H4a3 3 0 0 0 3.3 4M17 5h3a3 3 0 0 1-3.3 4M12 13v4M9 21h6" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>`,
  figure: `<circle cx="12" cy="8" r="4" fill="currentColor"/><path d="M4 21a8 8 0 0 1 16 0z" fill="currentColor"/>`,
};

/// One tile: a saved figure, a character the emulator can make, or both when
/// the character has been made before.
interface Entry {
  name: string;
  element: FigureElement | null;
  kind: FigureKind | null;
  figure?: Figure;
  offer?: Offer;
}

interface Tab {
  label: string;
  element?: FigureElement;
  entries: Entry[];
}

const root = document.getElementById("portal")!;

let family: PadFamily = "generic";
let shown = false;
let busy = false;
let asking = false;
let onPortal: string[] = [];
let mine: Figure[] = [];
let offers: Offer[] = [];
let tabs: Tab[] = [];
let tab = 0;
/// The selection: a tile in the grid, or a figure in the row of those on
/// the portal.
let zone: "grid" | "portal" = "grid";
let at = 0;
let chip = 0;
let status = "";

function placed(): { name: string; slot: number }[] {
  return onPortal.map((name, slot) => ({ name, slot })).filter((figure) => figure.name);
}

/// The name the portal shows for a tile's figure, which is the emulator's
/// name for the character rather than the file's.
function portalName(entry: Entry): string {
  const figure = entry.figure;
  const madeAs = figure && offers.find((offer) => offer.id === figure.id && offer.variant === figure.variant);
  return entry.offer?.name ?? madeAs?.name ?? entry.name;
}

function buildTabs() {
  const kept = tabs[tab]?.label;
  const byName = (a: Entry, b: Entry) => a.name.localeCompare(b.name);
  const entry = (offer: Offer): Entry => ({
    name: offer.name,
    element: offer.element,
    kind: offer.kind,
    offer,
    figure: mine.find((figure) => figure.id === offer.id && figure.variant === offer.variant),
  });
  const characters = offers.filter((offer) => offer.kind === "character");
  const next: Tab[] = [
    {
      label: "Saved",
      entries: mine.map((figure) => ({ name: figure.name, element: figure.element, kind: figure.kind, figure })),
    },
  ];
  for (const [element, label] of ELEMENTS) {
    const entries = characters.filter((offer) => offer.element === element).map(entry).sort(byName);
    if (entries.length > 0) next.push({ label, element, entries });
  }
  const others = characters.filter((offer) => !offer.element).map(entry).sort(byName);
  if (others.length > 0) next.push({ label: "Other", entries: others });
  for (const [kind, label] of KINDS) {
    const entries = offers.filter((offer) => offer.kind === kind).map(entry).sort(byName);
    if (entries.length > 0) next.push({ label, entries });
  }
  tabs = next;
  const again = tabs.findIndex((each) => each.label === kept);
  tab = again >= 0 ? again : Math.min(tab, tabs.length - 1);
}

/// Keeps the selection on something that is there. An empty portal row
/// hands it back to the grid.
function settle() {
  at = Math.max(0, Math.min(at, (tabs[tab]?.entries.length ?? 0) - 1));
  const on = placed().length;
  if (on === 0) zone = "grid";
  chip = Math.max(0, Math.min(chip, on - 1));
}

function node<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text?: string): HTMLElementTagNameMap[K] {
  const made = document.createElement(tag);
  made.className = className;
  if (text !== undefined) made.textContent = text;
  return made;
}

function mark(element: FigureElement | null, kind: FigureKind | null): HTMLElement {
  const badge = node("span", `portal-mark tint-${element ?? "none"}`);
  const shape = element ?? (kind && kind in MARKS ? kind : "figure");
  badge.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true">${MARKS[shape]}</svg>`;
  return badge;
}

function renderHead(): HTMLElement {
  const head = node("div", "portal-head");
  head.appendChild(node("div", "portal-title", "Portal"));
  const row = node("div", "portal-on");
  const on = placed();
  if (on.length === 0) row.appendChild(node("span", "portal-quiet", "Nothing on the portal."));
  on.forEach((figure, index) => {
    const known = offers.find((offer) => offer.name === figure.name);
    const button = node("button", zone === "portal" && index === chip ? "portal-chip sel" : "portal-chip");
    button.append(mark(known?.element ?? null, known?.kind ?? null), node("span", "", figure.name));
    button.onclick = () => {
      zone = "portal";
      chip = index;
      void takeOff();
    };
    row.appendChild(button);
  });
  head.appendChild(row);
  return head;
}

function renderTabs(): HTMLElement {
  const nav = node("div", "portal-nav");
  const bar = node("div", "portal-tabs");
  bar.setAttribute("role", "tablist");
  tabs.forEach((each, index) => {
    const button = node("button", index === tab ? "portal-tab sel" : "portal-tab");
    button.setAttribute("role", "tab");
    button.setAttribute("aria-selected", String(index === tab));
    if (each.element) button.appendChild(node("span", `portal-dot tint-${each.element}`));
    button.append(each.label);
    button.onclick = () => showTab(index);
    bar.appendChild(button);
  });
  nav.append(node("span", "portal-bumper", nameOf(family, "LB")), bar, node("span", "portal-bumper", nameOf(family, "RB")));
  return nav;
}

function renderBody(): HTMLElement {
  const body = node("div", "portal-body");
  const current = tabs[tab];
  if (!current || current.entries.length === 0) {
    body.appendChild(
      node(
        "div",
        "portal-quiet",
        asking
          ? "Getting the characters…"
          : "No saved figures yet. Choose a character under its element and it is saved here."
      )
    );
    return body;
  }
  const grid = node("div", "portal-grid");
  current.entries.forEach((entry, index) => {
    const on = onPortal.includes(portalName(entry));
    const tile = node("button", `portal-item${zone === "grid" && index === at ? " sel" : ""}${on ? " on" : ""}`);
    const words = node("span", "portal-words");
    words.appendChild(node("span", "portal-name", entry.name));
    const note = on ? "On the portal" : entry.offer && entry.figure ? "Saved" : "";
    if (note) words.appendChild(node("span", "portal-note", note));
    tile.append(mark(entry.element, entry.kind), words);
    tile.onclick = () => {
      zone = "grid";
      at = index;
      void choose();
    };
    grid.appendChild(tile);
  });
  body.appendChild(grid);
  return body;
}

function renderFoot(): HTMLElement {
  const foot = node("div", "portal-foot");
  const said = node("div", "portal-status", status);
  said.setAttribute("role", "status");
  const entry = tabs[tab]?.entries[at];
  const hints: [string, string][] = [];
  if (zone === "portal") {
    hints.push(["South", "Take off"]);
  } else {
    hints.push(["South", "Put on"]);
    if (entry && onPortal.includes(portalName(entry))) hints.push(["West", "Take off"]);
  }
  hints.push(["East", "Close"]);
  const row = node("div", "portal-hints");
  for (const [input, words] of hints) {
    const hint = node("span", "portal-hint");
    hint.append(node("kbd", "", nameOf(family, input)), words);
    row.appendChild(hint);
  }
  foot.append(said, row);
  return foot;
}

/// Draws the whole menu again, keeping where the grid and tabs were
/// scrolled so moving doesn't make the list jump.
function render() {
  settle();
  const scrolled = root.querySelector(".portal-body")?.scrollTop ?? 0;
  const tabsScrolled = root.querySelector(".portal-tabs")?.scrollLeft ?? 0;
  const panel = node("div", "portal-panel");
  panel.setAttribute("role", "dialog");
  panel.setAttribute("aria-label", "Portal");
  panel.append(renderHead(), renderTabs(), renderBody(), renderFoot());
  root.replaceChildren(panel);
  panel.querySelector<HTMLElement>(".portal-body")!.scrollTop = scrolled;
  panel.querySelector<HTMLElement>(".portal-tabs")!.scrollLeft = tabsScrolled;
  panel.querySelector(".portal-item.sel")?.scrollIntoView({ block: "nearest" });
  panel.querySelector(".portal-tab.sel")?.scrollIntoView({ block: "nearest", inline: "nearest" });
}

function say(text: string) {
  status = text;
  render();
}

function showTab(index: number) {
  if (tabs.length === 0) return;
  tab = (index + tabs.length) % tabs.length;
  at = 0;
  zone = "grid";
  render();
}

function moveGrid(move: Move) {
  const total = tabs[tab]?.entries.length ?? 0;
  const lastRow = Math.floor((total - 1) / COLUMNS);
  if (move === "left" && at % COLUMNS > 0) at -= 1;
  else if (move === "right" && at % COLUMNS < COLUMNS - 1 && at + 1 < total) at += 1;
  else if (move === "down" && at + COLUMNS < total) at += COLUMNS;
  else if (move === "down" && Math.floor(at / COLUMNS) < lastRow) at = total - 1;
  else if (move === "up" && at >= COLUMNS) at -= COLUMNS;
  else if (move === "up" && placed().length > 0) {
    zone = "portal";
    chip = Math.min(at, placed().length - 1);
  }
  render();
}

function movePortal(move: Move) {
  if (move === "left") chip -= 1;
  else if (move === "right") chip += 1;
  else if (move === "down") zone = "grid";
  render();
}

/// One change to the portal at a time, since each goes through the
/// emulator's own window and takes a moment.
async function change(saying: string, job: () => Promise<string[]>, said: (names: string[]) => string) {
  if (busy) return;
  busy = true;
  say(saying);
  try {
    onPortal = await job();
    status = said(onPortal);
    mine = await listFigures();
    buildTabs();
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

function takeOffSlot(slot: number, name: string) {
  return change(`Taking ${name} off…`, () => portalClear(slot), () => `${name} is off the portal.`);
}

/// Takes off the figure picked in the portal row, or the one the selected
/// tile stands for when it is on the portal.
async function takeOff() {
  settle();
  if (zone === "portal") {
    const figure = placed()[chip];
    if (figure) await takeOffSlot(figure.slot, figure.name);
    return;
  }
  const entry = tabs[tab]?.entries[at];
  const slot = entry ? onPortal.indexOf(portalName(entry)) : -1;
  if (slot >= 0) await takeOffSlot(slot, onPortal[slot]);
}

/// Puts the selected figure on the portal: the saved one when there is one,
/// otherwise a new one the emulator makes.
async function choose() {
  settle();
  if (zone === "portal") return takeOff();
  const entry = tabs[tab]?.entries[at];
  if (!entry || busy) return;
  const name = portalName(entry);
  if (onPortal.includes(name)) return say(`${name} is already on the portal.`);
  const slot = freeSlot();
  if (slot < 0) return say("The portal is full. Take a figure off first.");
  const { figure, offer } = entry;
  if (figure) {
    await change(
      `Putting ${entry.name} on the portal…`,
      () => portalLoad(slot, figure.path),
      (names) => `${names[slot] || entry.name} is on the portal.`
    );
  } else if (offer) {
    await change(
      `Making ${offer.name}…`,
      () => portalCreate(slot, offer),
      (names) => `${names[slot] || offer.name} is on the portal.`
    );
  }
}

/// The characters come from the emulator's own figure maker the first time
/// only; Omoio keeps the list after that, so later openings are instant.
async function loadOffers() {
  if (offers.length > 0 || asking) return;
  asking = true;
  busy = true;
  say("Getting the characters…");
  try {
    offers = await figureCharacters();
    status = "";
  } catch (err) {
    status = typeof err === "string" ? err : "Couldn't get the characters.";
  } finally {
    asking = false;
    busy = false;
    buildTabs();
    // With nothing saved yet, open on the first element, not an empty tab.
    if (mine.length === 0 && tab === 0 && tabs.length > 1) tab = 1;
    render();
  }
}

async function refresh() {
  busy = true;
  say("Reading the portal…");
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
  busy = false;
  buildTabs();
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
  const move = MOVES[input];
  if (move) return zone === "portal" ? movePortal(move) : moveGrid(move);
  if (input === "LB") return showTab(tab - 1);
  if (input === "RB") return showTab(tab + 1);
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
  zone = "grid";
  shown = true;
  await refresh();
  await loadOffers();
}

void onPortalMenu((state) => {
  family = state.family as PadFamily;
  if (state.open) void show();
  else shown = false;
});
void show();
