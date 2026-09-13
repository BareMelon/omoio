import type { PadFamily } from "../api";

/// What each place on a pad is called on each kind of pad, for anything that
/// names a button to the player. A place a kind does not name differently
/// falls through to COMMON.
export const FAMILY_NAMES: Record<PadFamily, Record<string, string>> = {
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

export function nameOf(family: PadFamily, input: string): string {
  return FAMILY_NAMES[family][input] ?? COMMON[input] ?? input;
}
