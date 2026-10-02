import type { PadFamily } from "../api";
import { FAMILY_NAMES, nameOf } from "./padNames";

/// A point on the drawing, in the units of its viewBox.
export type Spot = [number, number];

/// The drawing's own size. Anything placed beside it, such as the labels
/// on the Controller screen, works in these units.
export const PAD_BOX = { x: -6, y: -8, width: 412, height: 292 };

/// Where things sit. Xbox and Nintendo pads put the left stick above the
/// d-pad; PlayStation pads put both sticks low and side by side, with a
/// touchpad between the middle buttons. A pad of no known kind gets the
/// first, the more common shape.
interface Layout {
  ls: Spot;
  rs: Spot;
  dpad: Spot;
  face: Spot;
  back: Spot;
  start: Spot;
  guide: Spot;
  touchpad: boolean;
}

const OFFSET: Layout = {
  ls: [112, 104],
  rs: [250, 166],
  dpad: [150, 166],
  face: [290, 104],
  back: [176, 106],
  start: [224, 106],
  guide: [200, 74],
  touchpad: false,
};

const SIDE_BY_SIDE: Layout = {
  ls: [148, 168],
  rs: [252, 168],
  dpad: [104, 106],
  face: [296, 106],
  back: [136, 60],
  start: [264, 60],
  guide: [200, 150],
  touchpad: true,
};

const FACE_GAP = 26;

function layoutOf(family: PadFamily): Layout {
  return family === "playstation" ? SIDE_BY_SIDE : OFFSET;
}

/// The middle of every place on the drawing, so a label can point at it.
/// A stick's directions all point at the stick, the d-pad's at their arm,
/// and `Dpad` and `Face` are the middles of the d-pad and the face buttons.
export function padSpots(family: PadFamily): Record<string, Spot> {
  const l = layoutOf(family);
  const [dx, dy] = l.dpad;
  const [fx, fy] = l.face;
  const spots: Record<string, Spot> = {
    // The shoulders by their outer ends, which a line reaches without
    // crossing the trigger's name.
    LT: [80, 22],
    RT: [320, 22],
    LB: [50, 50],
    RB: [350, 50],
    Back: l.back,
    Start: l.start,
    Guide: l.guide,
    Dpad: l.dpad,
    Face: l.face,
    LS: l.ls,
    RS: l.rs,
    Up: [dx, dy - 16],
    Down: [dx, dy + 16],
    Left: [dx - 16, dy],
    Right: [dx + 16, dy],
    North: [fx, fy - FACE_GAP],
    East: [fx + FACE_GAP, fy],
    South: [fx, fy + FACE_GAP],
    West: [fx - FACE_GAP, fy],
  };
  for (const stick of ["LS", "RS"]) {
    for (const way of ["X+", "X-", "Y+", "Y-"]) spots[`${stick} ${way}`] = spots[stick];
  }
  return spots;
}

/// The part of the drawing an input lights. A stick's four directions are
/// all the stick.
export function partOf(input: string): string {
  if (input.startsWith("LS")) return "LS";
  if (input.startsWith("RS")) return "RS";
  return input;
}

const SHAPE: Record<string, (x: number, y: number) => string> = {
  North: (x, y) => `<path d="M${x} ${y - 5.5}L${x + 5.8} ${y + 4.2}H${x - 5.8}Z"/>`,
  East: (x, y) => `<circle cx="${x}" cy="${y}" r="5.2"/>`,
  South: (x, y) => `<path d="M${x - 4.6} ${y - 4.6}L${x + 4.6} ${y + 4.6}M${x + 4.6} ${y - 4.6}L${x - 4.6} ${y + 4.6}"/>`,
  West: (x, y) => `<rect x="${x - 4.6}" y="${y - 4.6}" width="9.2" height="9.2" rx="0.6"/>`,
};

/// What is printed on a face button: a letter, or on a PlayStation pad a
/// shape. A pad of no known kind gets nothing rather than a guess.
function faceMark(family: PadFamily, place: string, x: number, y: number): string {
  if (family === "playstation") return `<g class="pad-glyph">${SHAPE[place](x, y)}</g>`;
  if (family === "generic") return "";
  return `<text class="pad-face" x="${x}" y="${y + 4.5}">${FAMILY_NAMES[family][place]}</text>`;
}

/// The marks on the two small middle buttons.
function middleMark(family: PadFamily, place: "Back" | "Start", [x, y]: Spot): string {
  if (family === "nintendo") {
    const bar = `M${x - 3.5} ${y}H${x + 3.5}`;
    return `<path class="pad-mark" d="${place === "Start" ? `${bar}M${x} ${y - 3.5}V${y + 3.5}` : bar}"/>`;
  }
  if (family === "xbox") {
    return place === "Start"
      ? `<path class="pad-mark" d="M${x - 3.5} ${y - 2.5}H${x + 3.5}M${x - 3.5} ${y}H${x + 3.5}M${x - 3.5} ${y + 2.5}H${x + 3.5}"/>`
      : `<path class="pad-mark" d="M${x - 3.4} ${y - 0.6}h4.6v3.6h-4.6zM${x - 1.6} ${y - 2.6}h4.8v3.8"/>`;
  }
  return "";
}

/// The body: wide across the top, with a grip under each hand. PlayStation
/// grips hang lower and further apart.
function bodyPath(family: PadFamily): string {
  if (family === "playstation") {
    return `M200 40C246 40 280 34 306 34C340 34 362 46 374 72C388 102 398 150 404 196C408 232 404 262 382 270C360 278 340 266 326 246C312 226 300 214 278 210C252 206 226 206 200 206C174 206 148 206 122 210C100 214 88 226 74 246C60 266 40 278 18 270C-4 262 -8 232 -4 196C2 150 12 102 26 72C38 46 60 34 94 34C120 34 154 40 200 40Z`;
  }
  return `M200 46C238 46 268 40 296 38C330 36 354 46 366 66C382 92 394 130 398 172C402 214 398 248 378 260C358 272 336 264 320 244C306 226 294 212 270 208C248 204 224 204 200 204C176 204 152 204 130 208C106 212 94 226 80 244C64 264 42 272 22 260C2 248 -2 214 2 172C6 130 18 92 34 66C46 46 70 36 104 38C132 40 162 46 200 46Z`;
}

function cross([x, y]: Spot): string {
  const w = 9;
  const l = 27;
  const r = 3;
  return `M${x - w} ${y - l + r}Q${x - w} ${y - l} ${x - w + r} ${y - l}H${x + w - r}Q${x + w} ${y - l} ${x + w} ${y - l + r}V${y - w}H${x + l - r}Q${x + l} ${y - w} ${x + l} ${y - w + r}V${y + w - r}Q${x + l} ${y + w} ${x + l - r} ${y + w}H${x + w}V${y + l - r}Q${x + w} ${y + l} ${x + w - r} ${y + l}H${x - w + r}Q${x - w} ${y + l} ${x - w} ${y + l - r}V${y + w}H${x - l + r}Q${x - l} ${y + w} ${x - l} ${y + w - r}V${y - w + r}Q${x - l} ${y - w} ${x - l + r} ${y - w}H${x - w}Z`;
}

/// A small arrow on each arm of the d-pad, pointing the arm's way.
function arrows([x, y]: Spot): string {
  const at = 19;
  return `<path class="pad-arrow" d="M${x} ${y - at - 3}l3 4h-6zM${x} ${y + at + 3}l3 -4h-6zM${x - at - 3} ${y}l4 3v-6zM${x + at + 3} ${y}l-4 3v-6z"/>`;
}

function stick(part: string, [x, y]: Spot): string {
  return `
    <circle class="pad-well" cx="${x}" cy="${y}" r="27"/>
    <circle data-part="${part}" class="pad-stick" cx="${x}" cy="${y}" r="20"/>
    <circle class="pad-ring" cx="${x}" cy="${y}" r="13.5"/>`;
}

let drawn = 0;

/// The kind of pad this player has, in the app's colours. Each input carries
/// its place as `data-part`, so a label or a press can light it.
export function padArt(family: PadFamily): string {
  const l = layoutOf(family);
  const spots = padSpots(family);
  // Each drawing gets its own gradient name, so one taken off the page never
  // leaves another without its shading.
  const shade = `pad-shade-${++drawn}`;
  const middle = (place: "Back" | "Start", at: Spot) =>
    family === "playstation"
      ? `<rect data-part="${place}" x="${at[0] - 4}" y="${at[1] - 8}" width="8" height="16" rx="4"/>`
      : `<circle data-part="${place}" cx="${at[0]}" cy="${at[1]}" r="8.5"/>${middleMark(family, place, at)}`;
  const faces = ["North", "East", "South", "West"];
  // A pad of no known kind spells its triggers out, which is too long to fit.
  const [trigL, trigR] = family === "generic" ? ["LT", "RT"] : [nameOf(family, "LT"), nameOf(family, "RT")];
  return `
    <svg class="pad-art" viewBox="${PAD_BOX.x} ${PAD_BOX.y} ${PAD_BOX.width} ${PAD_BOX.height}" aria-hidden="true">
      <defs>
        <linearGradient id="${shade}" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" class="pad-shade-top"/>
          <stop offset="1" class="pad-shade-foot"/>
        </linearGradient>
      </defs>
      <path data-part="LT" d="M74 40L77 8C78 1 84 -4 92 -4H118C126 -4 132 1 133 8L136 40Z"/>
      <path data-part="RT" d="M326 40L323 8C322 1 316 -4 308 -4H282C274 -4 268 1 267 8L264 40Z"/>
      <text class="pad-label" x="105" y="14">${trigL}</text>
      <text class="pad-label" x="295" y="14">${trigR}</text>
      <path class="pad-body" fill="url(#${shade})" d="${bodyPath(family)}"/>
      <path data-part="LB" d="M38 58C52 36 78 25 108 26C126 27 142 30 152 34L150 44C140 41 124 38 106 38C82 38 60 46 46 64Z"/>
      <path data-part="RB" d="M362 58C348 36 322 25 292 26C274 27 258 30 248 34L250 44C260 41 276 38 294 38C318 38 340 46 354 64Z"/>
      ${l.touchpad ? `<rect class="pad-plate" x="150" y="46" width="100" height="70" rx="12"/>` : ""}
      <circle data-part="Guide" cx="${l.guide[0]}" cy="${l.guide[1]}" r="${l.touchpad ? 8 : 13}"/>
      ${l.touchpad ? "" : `<circle class="pad-ring" cx="${l.guide[0]}" cy="${l.guide[1]}" r="8"/>`}
      ${middle("Back", l.back)}
      ${middle("Start", l.start)}
      ${stick("LS", l.ls)}
      ${stick("RS", l.rs)}
      <path class="pad-plate" d="${cross(l.dpad)}"/>
      ${["Up", "Down", "Left", "Right"]
        .map((way) => {
          const [x, y] = spots[way];
          const tall = way === "Up" || way === "Down";
          return `<rect data-part="${way}" class="pad-arm" x="${x - (tall ? 9 : 11)}" y="${y - (tall ? 11 : 9)}" width="${tall ? 18 : 22}" height="${tall ? 22 : 18}" rx="3"/>`;
        })
        .join("")}
      ${arrows(l.dpad)}
      ${faces.map((place) => `<circle data-part="${place}" cx="${spots[place][0]}" cy="${spots[place][1]}" r="12.5"/>`).join("")}
      ${faces.map((place) => faceMark(family, place, spots[place][0], spots[place][1])).join("")}
    </svg>`;
}

const MENU_ICON =
  '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"><path d="M3.5 4.5h9M3.5 8h9M3.5 11.5h9"/></svg>';
const VIEW_ICON =
  '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><rect x="2.5" y="5.5" width="7.5" height="6" rx="1"/><path d="M6 3.5h6.5a1 1 0 0 1 1 1V9"/></svg>';
const DPAD =
  '<svg viewBox="0 0 16 16"><path d="M6 2.5h4v3.5h3.5v4H10v3.5H6V10H2.5V6H6z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>';
const ARROW: Record<string, string> = { Up: "M8 4l4 6H4z", Down: "M8 12l4-6H4z", Left: "M4 8l6-4v8z", Right: "M12 8L6 4v8z" };
const WAY: Record<string, string> = { "Y+": "Up", "Y-": "Down", "X-": "Left", "X+": "Right" };

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

/// What goes on a key standing for one input of the pad in hand: what is
/// printed on the button, its shape on a PlayStation pad, an arrow for a
/// direction, or a cross for the whole d-pad (`Dpad`).
export function capFace(family: PadFamily, input: string): string {
  if (family === "xbox" && input === "Start") return MENU_ICON;
  if (family === "xbox" && input === "Back") return VIEW_ICON;
  if (family === "nintendo" && input === "Start") return "+";
  if (family === "nintendo" && input === "Back") return "−";
  if (family === "playstation" && SHAPE[input]) {
    return `<svg viewBox="0 0 16 16" class="cap-shape">${SHAPE[input](8, 8.4)}</svg>`;
  }
  if (family === "generic" && SHAPE[input]) return diamond(input);
  // Spelled out, a pad of no known kind's shoulders are too long for a key.
  if (family === "generic" && ["LB", "RB", "LT", "RT"].includes(input)) return input;
  if (input === "Dpad") return DPAD;
  if (ARROW[input]) return `<svg viewBox="0 0 16 16"><path d="${ARROW[input]}" fill="currentColor"/></svg>`;
  const [stickName, way] = input.split(" ");
  if (way && WAY[way]) {
    const press = nameOf(family, stickName);
    const short = press.length <= 3 ? press : stickName;
    return `${short}<svg viewBox="0 0 16 16"><path d="${ARROW[WAY[way]]}" fill="currentColor"/></svg>`;
  }
  if (input === "LS" || input === "RS") {
    const press = nameOf(family, input);
    return press.length <= 3 ? press : input;
  }
  return nameOf(family, input);
}
