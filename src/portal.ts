import "./styles/tokens.css";
import "./styles/portal.css";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  closePortalMenu,
  figureCharacters,
  figurePictures,
  figures as listFigures,
  onPortalMenu,
  padsHeld,
  portalClear,
  portalCreate,
  portalFigures,
  portalLoad,
  portalMenuFamily,
  villains as listVillains,
  type Figure,
  type FigureElement,
  type FigureKind,
  type Movement,
  type Offer,
  type PadFamily,
  type Villain,
} from "./api";
import { nameOf } from "./components/padNames";
import adventureIcon from "./icons/adventures.svg";
import itemIcon from "./icons/items.svg";
import swapperIcon from "./icons/swappers.svg";

/// The Skylanders menu, drawn by Omoio over the running game and used with
/// the pad alone. The shoulder buttons go through the tabs: Saved, traps and
/// the villains they hold, then one tab to each element, then swappers, items
/// such as the treasure chest and the swords, and adventure packs. The d-pad
/// or left stick moves, the bottom face button puts a figure on the portal,
/// the left one takes it off, the right one closes. Mouse and keyboard work
/// as well.
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

const ELEMENT_NAMES = new Map(ELEMENTS);

const KINDS: [FigureKind, string][] = [
  ["item", "Items"],
  ["trap", "Traps"],
  ["adventure", "Adventure packs"],
  ["vehicle", "Vehicles"],
  ["trophy", "Trophies"],
];

/// What a figure without an element is called under its name.
const KIND_NAMES: Partial<Record<FigureKind, string>> = {
  item: "Item",
  trap: "Trap",
  adventure: "Adventure pack",
  vehicle: "Vehicle",
  trophy: "Trophy",
};

/// Omoio's own icons for the tabs without an element and for the figures of
/// those kinds, drawn in the style of the game's element symbols: shapes,
/// painted in the kind's colour as the symbols are in the element's.
const ICONS = { item: itemIcon, adventure: adventureIcon, swapper: swapperIcon };

type Icon = keyof typeof ICONS;

const MOVEMENT_NAMES: Record<Movement, string> = {
  bounce: "Bounce",
  climb: "Climb",
  dig: "Dig",
  rocket: "Rocket",
  sneak: "Sneak",
  speed: "Speed",
  spin: "Spin",
  teleport: "Teleport",
};

/// Omoio's own drawing for each element, and for the kinds that have no
/// icon. Used until Omoio has read the game's own symbols out of the game,
/// and for an element that game doesn't have. A child who can't read yet
/// goes by the element's shape and colour, which the games use too.
const MARKS: Record<string, string> = {
  air: `<path d="M3 8h11a3 3 0 1 0-3-3M3 12h15a3 3 0 1 1-3 3M3 16h8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>`,
  earth: `<path d="M2 20 9 8l4 6 3-4 6 10z" fill="currentColor"/>`,
  fire: `<path d="M12 2c1 4 6 6.5 6 12a6 6 0 0 1-12 0c0-2.6 1.3-4 2.5-5 0 2 .8 3.3 2 3.8C10 9 10.8 5 12 2z" fill="currentColor"/>`,
  water: `<path d="M12 2.5c3.5 5 6.5 8.4 6.5 12a6.5 6.5 0 0 1-13 0c0-3.6 3-7 6.5-12z" fill="currentColor"/>`,
  life: `<path d="M4 20C4 11 9 4 20 4c0 11-7 16-16 16zM4 20l9-9" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>`,
  undead: `<path fill-rule="evenodd" d="M12 2.5a8 8 0 0 0-8 8c0 2.8 1.3 4.6 3.2 5.7V20a1 1 0 0 0 1 1h7.6a1 1 0 0 0 1-1v-3.8c1.9-1.1 3.2-2.9 3.2-5.7a8 8 0 0 0-8-8zM9 9.5a1.8 1.8 0 1 0 0 3.6 1.8 1.8 0 0 0 0-3.6zm6 0a1.8 1.8 0 1 0 0 3.6 1.8 1.8 0 0 0 0-3.6z" fill="currentColor"/>`,
  magic: `<path d="m12 2 2.6 6.6 7.1.5-5.5 4.6 1.8 6.9L12 16.8l-6 3.8 1.8-6.9-5.5-4.6 7.1-.5z" fill="currentColor"/>`,
  tech: `<circle cx="12" cy="12" r="4.5" fill="none" stroke="currentColor" stroke-width="2.5"/><path d="M12 2v4M12 18v4M2 12h4M18 12h4M4.9 4.9l2.8 2.8M16.3 16.3l2.8 2.8M4.9 19.1l2.8-2.8M16.3 7.7l2.8-2.8" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>`,
  light: `<path d="M12 6.2a5.8 5.8 0 1 1 0 11.6a5.8 5.8 0 1 1 0-11.6ZM10.05 5.69L12.00 0.70L13.95 5.69ZM15.08 6.16L19.99 4.01L17.84 8.92ZM18.31 10.05L23.30 12.00L18.31 13.95ZM17.84 15.08L19.99 19.99L15.08 17.84ZM13.95 18.31L12.00 23.30L10.05 18.31ZM8.92 17.84L4.01 19.99L6.16 15.08ZM5.69 13.95L0.70 12.00L5.69 10.05ZM6.16 8.92L4.01 4.01L8.92 6.16Z" fill="currentColor"/>`,
  dark: `<path d="M14.6 2.4A10 10 0 1 0 21.8 16.6A8.3 8.3 0 0 1 14.6 2.4Z" fill="currentColor"/>`,
  trap: `<path d="M12 2.5 17.5 9 12 17 6.5 9z" fill="currentColor"/><path d="M8 19.5h8M12 17v2.5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>`,
  villain: `<path d="M12 2.5 20.5 7.3v9.4L12 21.5 3.5 16.7V7.3z" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/>`,
  vehicle: `<circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="12" cy="12" r="2.5" fill="currentColor"/>`,
  trophy: `<path d="M7 3h10v5a5 5 0 0 1-10 0zM7 5H4a3 3 0 0 0 3.3 4M17 5h3a3 3 0 0 1-3.3 4M12 13v4M9 21h6" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>`,
  figure: `<circle cx="12" cy="8" r="4" fill="currentColor"/><path d="M4 21a8 8 0 0 1 16 0z" fill="currentColor"/>`,
};

/// The corner of a tile says when its figure is on the portal, saved, or
/// picked as a swapper's top, with a shape as well as words.
const BADGES = {
  on: ["On the portal", `<path d="m5 12.5 4.5 4.5L19 7.5" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>`],
  saved: ["Saved", `<path d="M6 3h12v18l-6-4.5L6 21z" fill="currentColor"/>`],
  picked: ["Top picked", `<path d="M12 3.5 20 13h-5v7.5H9V13H4z" fill="currentColor"/>`],
} as const;

/// One tile: a saved figure, a character the emulator can make, or both when
/// the character has been made before.
interface Entry {
  name: string;
  element: FigureElement | null;
  kind: FigureKind | null;
  figure?: Figure;
  offer?: Offer;
  /// A Swap Force swapper: both halves of one character.
  swap?: { top: Offer; bottom: Offer };
}

interface Tab {
  label: string;
  element?: FigureElement;
  icon?: Icon;
  /// One of Omoio's own drawings, for a tab with neither element nor icon.
  mark?: keyof typeof MARKS;
  entries: Entry[];
  /// The villains tab, laid out as a collector's tray rather than a grid.
  tray?: true;
}

/// The tray's columns, one to each element, as the villains fit the traps,
/// and one for Kaos, who has a trap of his own.
const TRAY: [FigureElement | null, string][] = [...ELEMENTS, [null, "Kaos"]];

const root = document.getElementById("portal")!;

let family: PadFamily = "generic";
let shown = false;
let busy = false;
let asking = false;
let onPortal: string[] = [];
let mine: Figure[] = [];
let offers: Offer[] = [];
/// Trap Team's villains, which the user has caught and which trap holds each.
let villainList: Villain[] = [];
/// The villains caught when the menu was last open, so one caught since
/// stands out once. `null` until the menu has been open.
let caughtBefore: Set<number> | null = null;
let caughtNew = new Set<number>();
let tabs: Tab[] = [];
let tab = 0;
/// The selection: a tile in the grid, or a figure in the row of those on
/// the portal.
let zone: "grid" | "portal" = "grid";
let at = 0;
let chip = 0;
/// Whether the menu has been filled once. Until then the page keeps the
/// "Loading the portal" it was written with.
let ready = false;
/// The top half picked for a swapper, while waiting for its bottom.
let pickedTop: { name: string; top: Offer } | null = null;
/// The figures' pictures Omoio has read out of this game, by name, and for
/// each figure id the picture of its lowest variant.
let pictures: { folder: string; names: Set<string>; firstOf: Map<number, string> } | null = null;

/// A figure's picture, or its plain version's when its variant has none of
/// its own, or failing that any version's: Trap Team names its pictures by
/// the game's own variants, which Cemu's list doesn't give its Trap Masters.
/// `null` when Omoio has no picture of it.
function pictureOf(id: number | null | undefined, variant: number | null | undefined): string | null {
  if (id == null) return null;
  const four = (value: number) => value.toString(16).padStart(4, "0");
  return fileOf(`${id}-${four(variant ?? 0)}`) ?? fileOf(`${id}-0000`) ?? fileOf(pictures?.firstOf.get(id) ?? "");
}

/// For each figure id, the name of its picture with the lowest variant.
function firstPictures(names: string[]): Map<number, string> {
  const first = new Map<number, string>();
  for (const name of [...names].sort()) {
    const match = /^(\d+)-[0-9a-f]{4}$/.exec(name);
    if (match && !first.has(Number(match[1]))) first.set(Number(match[1]), name);
  }
  return first;
}

/// One of the pictures Omoio read out of the game, by name, when it has it.
function fileOf(name: string): string | null {
  return pictures?.names.has(name) ? convertFileSrc(`${pictures.folder}\\${name}.png`) : null;
}

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

/// A swapper half's character, without the "(Top)" or "(Bottom)" the
/// emulator's list adds.
function baseName(name: string): string {
  return name.replace(/\s*\((Top|Bottom)\)\s*$/i, "").trim();
}

function savedFor(offer: Offer): Figure | undefined {
  return mine.find((figure) => figure.id === offer.id && figure.variant === offer.variant);
}

function isOn(entry: Entry): boolean {
  if (entry.swap) return [entry.swap.top, entry.swap.bottom].some((half) => onPortal.includes(half.name));
  return onPortal.includes(portalName(entry));
}

function buildTabs() {
  const kept = tabs[tab]?.label;
  const byName = (a: Entry, b: Entry) => a.name.localeCompare(b.name);
  const entry = (offer: Offer): Entry => ({
    name: offer.name,
    element: offer.element,
    kind: offer.kind,
    offer,
    figure: savedFor(offer),
  });
  const characters = offers.filter((offer) => offer.kind === "character" && !offer.half);
  const next: Tab[] = [
    {
      label: "Saved",
      entries: mine.map((figure) => ({ name: figure.name, element: figure.element, kind: figure.kind, figure })),
    },
  ];
  // Traps go on and off all through a Trap Team game, so they sit next to
  // Saved, and the villains they hold right after.
  const traps = offers.filter((offer) => offer.kind === "trap").map(entry).sort(byName);
  if (traps.length > 0) next.push({ label: "Traps", mark: "trap", entries: traps });
  if (traps.length > 0 && villainList.length > 0) next.push({ label: "Villains", mark: "villain", entries: [], tray: true });
  for (const [element, label] of ELEMENTS) {
    const entries = characters.filter((offer) => offer.element === element).map(entry).sort(byName);
    if (entries.length > 0) next.push({ label, element, entries });
  }
  const others = characters.filter((offer) => !offer.element).map(entry).sort(byName);
  if (others.length > 0) next.push({ label: "Other", entries: others });
  const halves = offers.filter((offer) => offer.half);
  const swappers: Entry[] = halves
    .filter((offer) => offer.half === "top")
    .flatMap((top) => {
      const bottom = halves.find((offer) => offer.half === "bottom" && baseName(offer.name) === baseName(top.name));
      return bottom ? [{ name: baseName(top.name), element: top.element, kind: top.kind, swap: { top, bottom } }] : [];
    })
    .sort(byName);
  if (swappers.length > 0) next.push({ label: "Swappers", icon: "swapper", entries: swappers });
  for (const [kind, label] of KINDS.filter(([kind]) => kind !== "trap")) {
    const entries = offers.filter((offer) => offer.kind === kind).map(entry).sort(byName);
    if (entries.length > 0) next.push({ label, icon: kindIcon(kind) ?? undefined, entries });
  }
  tabs = next;
  const again = tabs.findIndex((each) => each.label === kept);
  tab = again >= 0 ? again : Math.min(tab, tabs.length - 1);
}

/// The tray's columns with the villains in each, in the game's order.
function trayColumns(): { element: FigureElement | null; label: string; villains: Villain[] }[] {
  return TRAY.map(([element, label]) => ({ element, label, villains: villainList.filter((v) => v.element === element) }));
}

/// Every villain in the tray, column by column: what the selection counts in.
function trayOrder(): Villain[] {
  return trayColumns().flatMap((column) => column.villains);
}

/// How many things the selection can be on in the open tab.
function count(): number {
  const current = tabs[tab];
  return current?.tray ? trayOrder().length : (current?.entries.length ?? 0);
}

/// Keeps the selection on something that is there. An empty portal row
/// hands it back to the grid.
function settle() {
  at = Math.max(0, Math.min(at, count() - 1));
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

function drawing(element: FigureElement | null, kind: FigureKind | null): string {
  const shape = element ?? (kind && kind in MARKS ? kind : "figure");
  return `<svg viewBox="0 0 24 24" aria-hidden="true">${MARKS[shape]}</svg>`;
}

/// The kinds of figure with an icon of their own.
function kindIcon(kind: FigureKind | null): Icon | null {
  return kind === "item" || kind === "adventure" ? kind : null;
}

/// The colour a figure is shown in: its element's, or its kind's icon's.
function tintOf(element: FigureElement | null, kind: FigureKind | null): string {
  return `tint-${element ?? kindIcon(kind) ?? "none"}`;
}

function image(source: string): HTMLImageElement {
  const made = node("img", "");
  made.src = source;
  made.alt = "";
  made.decoding = "async";
  return made;
}

/// A shape painted in a colour through it, as the game's element symbols
/// and Omoio's own icons are.
function painted(source: string, tint: string): HTMLElement {
  const shape = node("span", `portal-symbol ${tint}`);
  const mask = `url("${source}")`;
  shape.style.maskImage = mask;
  shape.style.webkitMaskImage = mask;
  return shape;
}

/// The game's own symbol for an element, a white shape read out of the
/// game. `null` until Omoio has it.
function symbol(element: FigureElement | null): HTMLElement | null {
  const source = element && fileOf(`element-${element}`);
  return source ? painted(source, `tint-${element}`) : null;
}

function icon(kind: Icon): HTMLElement {
  return painted(ICONS[kind], `tint-${kind} portal-icon`);
}

/// An element's shape: the game's own symbol when Omoio has it, its drawing
/// when not. A figure with no element gets its kind's icon or drawing.
function emblem(into: HTMLElement, element: FigureElement | null, kind: FigureKind | null) {
  const own = element ? null : kindIcon(kind);
  const shape = own ? icon(own) : symbol(element);
  if (shape) into.appendChild(shape);
  else into.insertAdjacentHTML("beforeend", drawing(element, kind));
}

function mark(element: FigureElement | null, kind: FigureKind | null, source: string | null): HTMLElement {
  const badge = node("span", `portal-mark ${tintOf(element, kind)}`);
  if (source) badge.appendChild(image(source));
  else emblem(badge, element, kind);
  return badge;
}

/// How a swapper moves: the game's own Swap Zone badge when Omoio has it,
/// with the word, which a badge alone doesn't give someone new to them.
function movement(moves: Movement): HTMLElement {
  const part = node("span", "portal-move");
  const badge = fileOf(`movement-${moves}`);
  if (badge) part.appendChild(image(badge));
  part.append(MOVEMENT_NAMES[moves]);
  return part;
}

/// The picture spot at the top of a tile, with its corner badge: the
/// figure's own picture from the game when Omoio has it, its element's
/// drawing when not. A swapper is its bottom with a top laid over it: its
/// own, or while a top is picked, that one, so each bottom shows the mix.
function picture(entry: Entry, badge: keyof typeof BADGES | null): HTMLElement {
  const spot = node("span", `portal-art ${tintOf(entry.element, entry.kind)}`);
  const sources = entry.swap
    ? [entry.swap.bottom, pickedTop?.top ?? entry.swap.top].map((half) => pictureOf(half.id, half.variant))
    : [pictureOf(entry.offer?.id ?? entry.figure?.id, entry.offer?.variant ?? entry.figure?.variant)];
  if (sources.every((source) => source)) {
    for (const source of sources) spot.appendChild(image(source!));
  } else {
    emblem(spot, entry.element, entry.kind);
  }
  if (badge) {
    const [words, shape] = BADGES[badge];
    const corner = node("span", `portal-badge ${badge}`);
    corner.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true">${shape}</svg>`;
    corner.append(words);
    spot.appendChild(corner);
  }
  return spot;
}

/// The line under a tile's name: the element in its colour and shape, or
/// the kind of figure when it has no element, the series where the name
/// doesn't give it, and how a swapper moves, which its bottom decides.
function kindLine(entry: Entry): HTMLElement {
  const line = node("span", `portal-kind ${tintOf(entry.element, entry.kind)}`);
  if (entry.element) {
    emblem(line, entry.element, null);
    line.append(ELEMENT_NAMES.get(entry.element) ?? "");
  } else if (entry.kind) {
    const own = kindIcon(entry.kind);
    if (own) line.appendChild(icon(own));
    line.append(KIND_NAMES[entry.kind] ?? "");
  }
  const series = entry.offer?.series ?? entry.figure?.series;
  if (series) line.appendChild(node("span", "portal-series", `Series ${series}`));
  const moves = entry.swap ? entry.swap.bottom.movement : (entry.offer?.movement ?? entry.figure?.movement);
  if (moves) line.appendChild(movement(moves));
  return line;
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
    button.append(
      mark(known?.element ?? null, known?.kind ?? null, pictureOf(known?.id, known?.variant)),
      node("span", "", figure.name)
    );
    // A trap says which villain it brings with it.
    const villain = heldOnPortal(figure.name);
    if (villain) {
      button.append(node("span", "portal-with", "with"), node("span", "", villain.name));
      const face = fileOf(`villain-${villain.id}`);
      if (face) {
        const picture = image(face);
        picture.className = "portal-chip-villain";
        button.appendChild(picture);
      }
    }
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
    if (each.element) button.appendChild(symbol(each.element) ?? node("span", `portal-dot tint-${each.element}`));
    else if (each.icon) button.appendChild(icon(each.icon));
    else if (each.mark) button.insertAdjacentHTML("beforeend", `<svg class="portal-tab-mark ${each.mark}" viewBox="0 0 24 24" aria-hidden="true">${MARKS[each.mark]}</svg>`);
    button.append(each.label);
    button.onclick = () => showTab(index);
    bar.appendChild(button);
  });
  nav.append(node("span", "portal-bumper", nameOf(family, "LB")), bar, node("span", "portal-bumper", nameOf(family, "RB")));
  return nav;
}

function renderBody(): HTMLElement {
  if (tabs[tab]?.tray) return renderTray();
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
    const on = isOn(entry);
    const chosen = Boolean(entry.swap) && pickedTop?.name === entry.name;
    const tile = node(
      "button",
      `portal-item${zone === "grid" && index === at ? " sel" : ""}${on ? " on" : ""}${chosen ? " picked" : ""}`
    );
    const badge = chosen ? "picked" : on ? "on" : entry.offer && entry.figure ? "saved" : null;
    tile.append(picture(entry, badge), node("span", "portal-name", entry.name), kindLine(entry));
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

// ---- the villains ----

/// An empty slot in the tray: the hollow a villain sits in once caught.
const SLOT = `<svg class="portal-slot" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 1.6 21 6.8v10.4L12 22.4 3 17.2V6.8z"/><text x="12" y="15.3" text-anchor="middle">?</text></svg>`;

/// The shapes the tally's three counts go by, as well as by their words.
const TALLY_SHAPES = {
  trapped: `<path d="M12 2.5 20.5 7.3v9.4L12 21.5 3.5 16.7V7.3z" fill="currentColor"/>`,
  loose: `<circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" stroke-width="2"/>`,
  none: `<path d="M12 2.5 20.5 7.3v9.4L12 21.5 3.5 16.7V7.3z" fill="none" stroke="currentColor" stroke-width="1.6" stroke-dasharray="2.6 2.2"/>`,
};

/// The saved trap figure that holds a villain.
function trapFigure(villain: Villain): Figure | undefined {
  return mine.find((figure) => figure.path === villain.trap?.path);
}

/// The name the portal gives the trap holding a villain.
function trapName(villain: Villain): string {
  const figure = trapFigure(villain);
  return figure ? portalName({ name: figure.name, element: null, kind: null, figure }) : (villain.trap?.name ?? "");
}

function trapOn(villain: Villain): boolean {
  return Boolean(villain.trap) && onPortal.includes(trapName(villain));
}

/// The villain a figure on the portal holds, when it is one saved trap that
/// holds one.
function heldOnPortal(name: string): Villain | undefined {
  const holding = villainList.filter((villain) => villain.trap && trapName(villain) === name);
  return holding.length === 1 ? holding[0] : undefined;
}

/// A villain's picture: in its trap's frame when caught, or as it escaped
/// when caught before and in no trap now.
function villainPicture(villain: Villain): string | null {
  if (!villain.caught) return null;
  const own = fileOf(`villain-${villain.id}`);
  return villain.trap ? own : (fileOf(`villain-${villain.id}-loose`) ?? own);
}

function elementWords(element: FigureElement | null): string {
  return element ? (ELEMENT_NAMES.get(element) ?? "") : "Kaos";
}

/// How many villains are caught, how many of those sit in a trap, and a bar
/// that fills as the collection does.
function renderTally(): HTMLElement {
  const total = villainList.length;
  const trapped = villainList.filter((villain) => villain.trap).length;
  const caught = villainList.filter((villain) => villain.caught).length;
  const tally = node("div", "portal-tally");
  const said = node("div", "portal-count");
  said.append(node("b", "", String(caught)), node("span", "", `of ${total} villains caught`));
  const bar = node("div", "portal-bar");
  bar.setAttribute("aria-hidden", "true");
  for (const [part, share] of [["trapped", trapped], ["loose", caught - trapped]] as const) {
    const fill = node("i", part);
    fill.style.width = `${(share / total) * 100}%`;
    bar.appendChild(fill);
  }
  const legend = node("div", "portal-legend");
  const counts: [keyof typeof TALLY_SHAPES, string, number][] = [
    ["trapped", "In a trap", trapped],
    ["loose", "Caught, not in a trap", caught - trapped],
    ["none", "Not caught yet", total - caught],
  ];
  for (const [part, words, number] of counts) {
    const item = node("span", part);
    item.insertAdjacentHTML("beforeend", `<svg viewBox="0 0 24 24" aria-hidden="true">${TALLY_SHAPES[part]}</svg>`);
    item.append(words, node("b", "", String(number)));
    legend.appendChild(item);
  }
  tally.append(said, bar, legend);
  return tally;
}

/// One villain in its column: its picture in the slot once caught, with the
/// trap that holds it in the corner.
function villainTile(villain: Villain, index: number): HTMLElement {
  const state = villain.trap ? "trapped" : villain.caught ? "loose" : "none";
  const fresh = caughtNew.has(villain.id);
  const tile = node("button", `portal-villain ${state}${zone === "grid" && index === at ? " sel" : ""}${fresh ? " new" : ""}`);
  const where = villain.trap ? `in ${trapName(villain)}` : villain.caught ? "caught, not in a trap" : "not caught yet";
  tile.setAttribute("aria-label", `${villain.name}, ${where}`);
  const art = node("span", "portal-villain-art");
  art.insertAdjacentHTML("beforeend", SLOT);
  const face = villainPicture(villain);
  if (face) art.appendChild(image(face));
  if (villain.trap) {
    const badge = node("span", "portal-villain-trap");
    const trap = pictureOf(villain.trap.id, villain.trap.variant);
    if (trap) badge.appendChild(image(trap));
    else badge.insertAdjacentHTML("beforeend", `<svg viewBox="0 0 24 24" aria-hidden="true">${MARKS.trap}</svg>`);
    art.appendChild(badge);
  }
  if (fresh) art.appendChild(node("span", "portal-villain-new", "New"));
  tile.append(art, node("span", "portal-villain-name", villain.name));
  tile.onclick = () => {
    zone = "grid";
    at = index;
    void choose();
  };
  return tile;
}

/// The villain picked in the tray: its picture, its element, the trap that
/// holds it, and the rest of its element.
function renderVillain(villain: Villain | undefined): HTMLElement {
  const card = node("div", `portal-villain-card tint-${villain?.element ?? "kaos"}`);
  if (!villain) return card;
  const art = node("div", "portal-villain-card-art");
  const face = villainPicture(villain);
  if (face) art.appendChild(image(face));
  else art.insertAdjacentHTML("beforeend", SLOT);
  const line = node("div", "portal-kind");
  if (villain.element) emblem(line, villain.element, null);
  line.append(villain.element ? `${elementWords(villain.element)} villain` : "Kaos");
  card.append(art, node("div", "portal-villain-card-name", villain.name), line);
  if (villain.trap) {
    card.appendChild(node("div", "portal-label", "In this trap"));
    const held = node("div", "portal-held");
    const picture = node("span", "portal-held-art");
    const trap = pictureOf(villain.trap.id, villain.trap.variant);
    if (trap) picture.appendChild(image(trap));
    const words = node("div", "portal-held-words");
    words.append(node("b", "", trapName(villain)), node("span", "", `${elementWords(villain.element)} trap`));
    if (trapOn(villain)) {
      const on = node("span", "portal-held-on");
      on.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true">${BADGES.on[1]}</svg>`;
      on.append(BADGES.on[0]);
      words.appendChild(on);
    }
    held.append(picture, words);
    card.appendChild(held);
  } else {
    const fits = villain.element ? `a ${elementWords(villain.element)} trap` : "the Kaos trap";
    card.appendChild(
      node("p", "portal-villain-note", villain.caught ? "Not in one of your traps now." : `Not caught yet. Only ${fits} can hold it.`)
    );
  }
  const kin = villainList.filter((other) => other.element === villain.element);
  if (kin.length > 1) {
    card.appendChild(node("div", "portal-label", `${elementWords(villain.element)} villains`));
    const row = node("div", "portal-kin");
    for (const other of kin) {
      const spot = node("span", other.id === villain.id ? "portal-kin-villain me" : "portal-kin-villain");
      const face = villainPicture(other);
      if (face) spot.appendChild(image(face));
      else spot.insertAdjacentHTML("beforeend", SLOT);
      row.appendChild(spot);
    }
    card.appendChild(row);
  }
  return card;
}

/// The villains tab, laid out as a collector's tray: a column to each
/// element, every villain in it, caught or not, and the one picked beside.
function renderTray(): HTMLElement {
  const body = node("div", "portal-body portal-tray");
  const order = trayOrder();
  const main = node("div", "portal-tray-main");
  const columns = node("div", "portal-tray-columns");
  for (const column of trayColumns()) {
    const tint = `tint-${column.element ?? "kaos"}`;
    const shown = node("div", `portal-tray-column ${tint}`);
    const name = node("div", "portal-tray-column-name");
    if (column.element) emblem(name, column.element, null);
    name.append(column.label);
    const got = column.villains.filter((villain) => villain.caught).length;
    const all = column.villains.length;
    shown.append(name, node("div", got === all ? "portal-tray-column-n full" : "portal-tray-column-n", `${got} of ${all}`));
    for (const villain of column.villains) shown.appendChild(villainTile(villain, order.indexOf(villain)));
    columns.appendChild(shown);
  }
  main.append(renderTally(), columns);
  body.append(main, renderVillain(order[at]));
  return body;
}

function renderFoot(): HTMLElement {
  const foot = node("div", "portal-foot");
  const entry = tabs[tab]?.entries[at];
  const hints: [string, string][] = [];
  if (zone === "portal") {
    hints.push(["South", "Take off"]);
  } else if (tabs[tab]?.tray) {
    const villain = trayOrder()[at];
    if (villain?.trap && trapOn(villain)) hints.push(["West", `Take ${trapName(villain)} off`]);
    else if (villain?.trap) hints.push(["South", `Put ${trapName(villain)} on`]);
  } else {
    hints.push(["South", entry?.swap ? (pickedTop ? "Pick bottom" : "Pick top") : "Put on"]);
    if (entry && isOn(entry)) hints.push(["West", "Take off"]);
  }
  hints.push(["East", pickedTop ? "Back" : "Close"]);
  const row = node("div", "portal-hints");
  for (const [input, words] of hints) {
    const hint = node("span", "portal-hint");
    hint.append(node("kbd", "", nameOf(family, input)), words);
    row.appendChild(hint);
  }
  foot.append(row);
  return foot;
}

/// Draws the whole menu again, keeping where the grid and tabs were
/// scrolled so moving doesn't make the list jump.
function render() {
  if (!ready) return;
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
  panel.querySelector(".portal-item.sel, .portal-villain.sel")?.scrollIntoView({ block: "nearest" });
  panel.querySelector(".portal-tab.sel")?.scrollIntoView({ block: "nearest", inline: "nearest" });
}

// ---- notices ----

/// What the menu is doing or has done, shown as a card in the corner of the
/// screen like a Windows notification, large enough to read from a sofa.
/// Work under way and hints stay until something replaces them; a finished
/// job, a problem and a word about why nothing happened go by themselves.
interface Notice {
  kind: "working" | "done" | "problem" | "hint" | "info";
  title: string;
  detail?: string;
  /// The figure it is about, its picture's layers bottom first: a swapper
  /// is two. Left out when Omoio has no picture of a layer.
  picture?: (string | null)[];
}

const NOTICE_LASTS: Partial<Record<Notice["kind"], number>> = { done: 3500, problem: 8000, info: 5000 };

const NOTICE_SHAPES: Record<Exclude<Notice["kind"], "working">, string> = {
  done: BADGES.on[1],
  problem: `<path d="M12 6v7.5M12 18h.01" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"/>`,
  hint: BADGES.picked[1],
  info: `<path d="M12 11v6.5M12 6.5h.01" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"/>`,
};

const card = node("div", "portal-notice");
card.setAttribute("role", "status");
document.body.appendChild(card);
let notice: Notice | null = null;
let noticeTimer = 0;

/// The kind's mark: a spinner while working, a shape otherwise.
function noticeMark(kind: Notice["kind"]): HTMLElement {
  if (kind === "working") return node("span", "portal-spinner");
  const mark = node("span", "portal-notice-shape");
  mark.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true">${NOTICE_SHAPES[kind]}</svg>`;
  return mark;
}

/// Shows a notice in place of the one up now, or with `null` takes it away.
function notify(next: Notice | null) {
  window.clearTimeout(noticeTimer);
  notice = next;
  if (!next) {
    card.classList.remove("shown");
    return;
  }
  const art = node("span", "portal-notice-art");
  const layers = next.picture ?? [];
  if (layers.length > 0 && layers.every(Boolean)) {
    for (const source of layers) art.appendChild(image(source!));
    art.appendChild(node("span", "portal-notice-corner")).appendChild(noticeMark(next.kind));
  } else {
    art.appendChild(noticeMark(next.kind));
  }
  const words = node("span", "portal-notice-words");
  words.append(node("strong", "", next.title));
  if (next.detail) words.append(node("span", "", next.detail));
  card.replaceChildren(art, words);
  card.className = `portal-notice ${next.kind} shown`;
  const lasts = NOTICE_LASTS[next.kind];
  if (lasts) noticeTimer = window.setTimeout(() => notify(null), lasts);
}

function problem(err: unknown, otherwise: string): Notice {
  return { kind: "problem", title: typeof err === "string" ? err : otherwise };
}

/// The picture of a figure on the portal, by the name the portal gives it.
function pictureNamed(name: string): (string | null)[] {
  const known = offers.find((offer) => offer.name === name);
  return [pictureOf(known?.id, known?.variant)];
}

function showTab(index: number) {
  if (tabs.length === 0) return;
  tab = (index + tabs.length) % tabs.length;
  at = 0;
  zone = "grid";
  // Leaving a tab drops a picked top, and the notice asking for its bottom.
  if (pickedTop) notify(null);
  pickedTop = null;
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

/// Up and down a column of the tray, across to the next column at the same
/// height or its last villain, and up off the top to the portal row.
function moveTray(move: Move) {
  const sizes = trayColumns().map((column) => column.villains.length);
  let column = 0;
  let row = at;
  while (column < sizes.length - 1 && row >= sizes[column]) row -= sizes[column++];
  const first = (index: number) => sizes.slice(0, index).reduce((sum, size) => sum + size, 0);
  if (move === "left" && column > 0) at = first(column - 1) + Math.min(row, sizes[column - 1] - 1);
  else if (move === "right" && column < sizes.length - 1) at = first(column + 1) + Math.min(row, sizes[column + 1] - 1);
  else if (move === "down" && row + 1 < sizes[column]) at += 1;
  else if (move === "up" && row > 0) at -= 1;
  else if (move === "up" && placed().length > 0) {
    zone = "portal";
    chip = Math.min(column, placed().length - 1);
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
async function change(saying: Notice, job: () => Promise<string[]>, said: (names: string[]) => Notice) {
  if (busy) return;
  busy = true;
  notify(saying);
  try {
    onPortal = await job();
    notify(said(onPortal));
    mine = await listFigures(true);
    buildTabs();
  } catch (err) {
    notify(problem(err, "That didn't work. Try again."));
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
  const picture = pictureNamed(name);
  return change(
    { kind: "working", title: `Taking ${name} off…`, picture },
    () => portalClear(slot),
    () => ({ kind: "done", title: `${name} is off the portal`, picture })
  );
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
  if (tabs[tab]?.tray) {
    const villain = trayOrder()[at];
    const slot = villain?.trap ? onPortal.indexOf(trapName(villain)) : -1;
    if (villain && slot >= 0) await takeOffSlot(slot, trapName(villain));
    return;
  }
  const entry = tabs[tab]?.entries[at];
  if (!entry) return;
  const names = entry.swap ? [entry.swap.top.name, entry.swap.bottom.name] : [portalName(entry)];
  for (const name of names) {
    const slot = onPortal.indexOf(name);
    if (slot >= 0) await takeOffSlot(slot, name);
  }
}

/// Puts one figure on the portal in the first free slot: the saved one when
/// there is one, otherwise a new one the emulator makes. Says whether it is
/// on now.
async function putOn(name: string, figure: Figure | undefined, offer: Offer | undefined): Promise<boolean> {
  const slot = freeSlot();
  if (slot < 0) {
    notify({ kind: "problem", title: "The portal is full", detail: "Take a figure off first." });
    return false;
  }
  const picture = [pictureOf(offer?.id ?? figure?.id, offer?.variant ?? figure?.variant)];
  if (figure) {
    await change(
      { kind: "working", title: `Putting ${name} on the portal…`, picture },
      () => portalLoad(slot, figure.path),
      (names) => ({ kind: "done", title: `${names[slot] || name} is on the portal`, picture })
    );
  } else if (offer) {
    await change(
      { kind: "working", title: `Making ${offer.name}…`, detail: "A new figure, kept for next time.", picture },
      () => portalCreate(slot, offer),
      (names) => ({ kind: "done", title: `${names[slot] || offer.name} is on the portal`, picture })
    );
  }
  return Boolean(onPortal[slot]);
}

/// Both halves of a swapper, one after the other, each the saved figure when
/// there is one. The top and bottom may be of different characters.
async function putSwapper(top: Offer, bottom: Offer) {
  const halves = [top, bottom].filter((half) => !onPortal.includes(half.name));
  const free = onPortal.length === 0 ? halves.length : onPortal.filter((name) => !name).length;
  if (free < halves.length) {
    return notify({ kind: "problem", title: "A swapper needs two free places", detail: "Take a figure off first." });
  }
  for (const half of halves) {
    if (!(await putOn(half.name, savedFor(half), half))) return;
  }
  const same = baseName(top.name) === baseName(bottom.name);
  notify({
    kind: "done",
    title: same
      ? `${baseName(top.name)} is on the portal`
      : `${baseName(top.name)} and ${baseName(bottom.name)} are on the portal`,
    picture: [bottom, top].map((half) => pictureOf(half.id, half.variant)),
  });
}

/// Puts the trap that holds the picked villain on the portal, which brings
/// the villain into the game with it. A villain in no trap says why.
async function chooseVillain() {
  const villain = trayOrder()[at];
  if (!villain || busy) return;
  if (!villain.trap) {
    const fits = villain.element ? `a ${elementWords(villain.element)} trap` : "the Kaos trap";
    return notify(
      villain.caught
        ? { kind: "info", title: `${villain.name} isn't in one of your traps`, picture: [villainPicture(villain)] }
        : { kind: "info", title: `${villain.name} isn't caught yet`, detail: `Only ${fits} can hold it.` }
    );
  }
  const name = trapName(villain);
  if (onPortal.includes(name)) return notify({ kind: "done", title: `${name} is already on the portal`, picture: pictureNamed(name) });
  await putOn(name, trapFigure(villain), undefined);
}

/// Puts the selected figure on the portal: the saved one when there is one,
/// otherwise a new one the emulator makes.
async function choose() {
  settle();
  if (zone === "portal") return takeOff();
  if (tabs[tab]?.tray) return chooseVillain();
  const entry = tabs[tab]?.entries[at];
  if (!entry || busy) return;
  if (entry.swap) {
    if (!pickedTop) {
      const top = entry.swap.top;
      pickedTop = { name: entry.name, top };
      notify({ kind: "hint", title: "Now pick the bottom", detail: `Top: ${entry.name}`, picture: [pictureOf(top.id, top.variant)] });
      return render();
    }
    const top = pickedTop.top;
    pickedTop = null;
    return putSwapper(top, entry.swap.bottom);
  }
  const name = portalName(entry);
  if (onPortal.includes(name)) return notify({ kind: "done", title: `${name} is already on the portal`, picture: pictureNamed(name) });
  await putOn(entry.name, entry.figure, entry.offer);
}

/// The characters come from the emulator's own figure maker the first time
/// only; Omoio keeps the list after that, so later openings are instant.
async function loadOffers() {
  if (offers.length > 0 || asking) return;
  asking = true;
  busy = true;
  const getting: Notice = { kind: "working", title: "Getting the characters…" };
  // While the page still says it is loading, that says enough.
  if (ready) notify(getting);
  render();
  try {
    offers = await figureCharacters();
    if (notice === getting) notify(null);
  } catch (err) {
    notify(problem(err, "Couldn't get the characters."));
  } finally {
    asking = false;
    busy = false;
    buildTabs();
    // With nothing saved yet, open on the first element, not an empty tab.
    if (mine.length === 0 && tab === 0 && tabs.length > 1) tab = 1;
    render();
  }
}

/// Reads what is on the portal. Usually quick, so the notice only comes up
/// when the emulator takes its time.
async function refresh() {
  busy = true;
  const looking: Notice = { kind: "working", title: "Reading the portal…" };
  const slow = window.setTimeout(() => ready && notify(looking), 400);
  const [names, files, found] = await Promise.all([
    portalFigures().catch((err: unknown) => {
      notify(problem(err, "Couldn't read the portal."));
      return null;
    }),
    listFigures(true),
    listVillains().catch(() => [] as Villain[]),
  ]);
  window.clearTimeout(slow);
  if (names) {
    onPortal = names;
    if (notice === looking) notify(null);
  }
  mine = files;
  villainList = found;
  // A villain caught since the menu was last open stands out until the next time.
  const caught = new Set(found.filter((villain) => villain.caught).map((villain) => villain.id));
  caughtNew = caughtBefore ? new Set([...caught].filter((id) => !caughtBefore!.has(id))) : new Set();
  caughtBefore = caught;
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
  if (move) return zone === "portal" ? movePortal(move) : tabs[tab]?.tray ? moveTray(move) : moveGrid(move);
  if (input === "LB") return showTab(tab - 1);
  if (input === "RB") return showTab(tab + 1);
  if (input === "East") {
    if (!pickedTop) return void closePortalMenu();
    pickedTop = null;
    notify(null);
    return render();
  }
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
  notify(null);
  try {
    family = (await portalMenuFamily()) as PadFamily;
    held = await readPads();
    zone = "grid";
    shown = true;
    // Read again each time, since pictures can be got while the game runs.
    pictures = await figurePictures()
      .then((found) => ({ folder: found.folder, names: new Set(found.names), firstOf: firstPictures(found.names) }))
      .catch(() => null);
    await refresh();
    await loadOffers();
  } finally {
    // Filled once, the menu draws itself from then on, a problem or not.
    if (!ready) {
      ready = true;
      render();
    }
  }
}

void onPortalMenu((state) => {
  family = state.family as PadFamily;
  if (state.open) void show();
  else shown = false;
});
void show();
