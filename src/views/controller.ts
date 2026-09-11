import {
  controllerView,
  forgetController,
  saveController,
  setUpController,
  type Binding,
  type ControllerInfo,
  type ControllerView,
} from "../api";
import { store } from "../state";
import { emptyState, type View } from "./view";

/// What each physical input is called on screen. The keys are SDL's names,
/// which is what the backend deals in whatever the pad is.
const PHYSICAL: Record<string, string> = {
  South: "A",
  East: "B",
  West: "X",
  North: "Y",
  LB: "LB",
  LT: "LT",
  LS: "Left stick press",
  RB: "RB",
  RT: "RT",
  RS: "Right stick press",
  Start: "Start",
  Back: "Back",
  Guide: "Guide",
  Up: "D-pad up",
  Down: "D-pad down",
  Left: "D-pad left",
  Right: "D-pad right",
  "LS X-": "Left stick left",
  "LS X+": "Left stick right",
  "LS Y+": "Left stick up",
  "LS Y-": "Left stick down",
  "RS X-": "Right stick left",
  "RS X+": "Right stick right",
  "RS Y+": "Right stick up",
  "RS Y-": "Right stick down",
};

/// The PS3 side, grouped the way the pad is laid out: left hand, middle,
/// right hand. Labels drop the group name where the heading already says it.
type Group = { title: string; rows: [key: string, label: string][] };

const LEFT: Group[] = [
  { title: "Left shoulder", rows: [["L1", "L1"], ["L2", "L2"], ["L3", "L3"]] },
  { title: "D-pad", rows: [["Up", "Up"], ["Down", "Down"], ["Left", "Left"], ["Right", "Right"]] },
  {
    title: "Left stick",
    rows: [
      ["Left Stick Up", "Up"],
      ["Left Stick Down", "Down"],
      ["Left Stick Left", "Left"],
      ["Left Stick Right", "Right"],
    ],
  },
];

const RIGHT: Group[] = [
  { title: "Right shoulder", rows: [["R1", "R1"], ["R2", "R2"], ["R3", "R3"]] },
  {
    title: "Buttons",
    rows: [["Triangle", "Triangle"], ["Circle", "Circle"], ["Cross", "Cross"], ["Square", "Square"]],
  },
  {
    title: "Right stick",
    rows: [
      ["Right Stick Up", "Up"],
      ["Right Stick Down", "Down"],
      ["Right Stick Left", "Left"],
      ["Right Stick Right", "Right"],
    ],
  },
];

const MIDDLE: Group = {
  title: "Middle",
  rows: [["Select", "Select"], ["PS Button", "PS button"], ["Start", "Start"]],
};

/// Which part of the drawing a row lights up. The four directions of a stick
/// all belong to the stick.
function partFor(key: string): string {
  if (key.startsWith("Left Stick")) return "L3";
  if (key.startsWith("Right Stick")) return "R3";
  return key;
}

/// A PS3 pad, drawn from the tokens so it follows the rest of the app. Every
/// input carries `data-part` so hovering a row can point at it.
const PAD_ART = `
  <svg class="pad-art" viewBox="0 0 320 200" aria-hidden="true">
    <rect data-part="L2" x="60" y="6" width="48" height="14" rx="6"/>
    <rect data-part="R2" x="212" y="6" width="48" height="14" rx="6"/>
    <rect data-part="L1" x="54" y="20" width="60" height="14" rx="7"/>
    <rect data-part="R1" x="206" y="20" width="60" height="14" rx="7"/>
    <path class="pad-body" d="M70 32H250C288 32 310 70 312 118C314 164 296 190 272 188C251 186 239 162 223 144H97C81 162 69 186 48 188C24 190 6 164 8 118C10 70 32 32 70 32Z"/>
    <rect data-part="Up" x="74" y="58" width="16" height="17" rx="3"/>
    <rect data-part="Down" x="74" y="89" width="16" height="17" rx="3"/>
    <rect data-part="Left" x="57" y="74" width="17" height="16" rx="3"/>
    <rect data-part="Right" x="90" y="74" width="17" height="16" rx="3"/>
    <circle data-part="Triangle" cx="238" cy="62" r="10"/>
    <circle data-part="Circle" cx="258" cy="82" r="10"/>
    <circle data-part="Cross" cx="238" cy="102" r="10"/>
    <circle data-part="Square" cx="218" cy="82" r="10"/>
    <rect data-part="Select" x="132" y="76" width="18" height="8" rx="4"/>
    <rect data-part="Start" x="170" y="76" width="18" height="8" rx="4"/>
    <circle data-part="PS Button" cx="160" cy="104" r="8"/>
    <circle data-part="L3" cx="118" cy="130" r="19"/>
    <circle data-part="R3" cx="202" cy="130" r="19"/>
  </svg>`;

/// Nothing in RPCS3's own name for a pad is worth reading. An XInput slot is
/// an Xbox pad; an SDL name ends in the index RPCS3 adds.
function padName(controller: ControllerInfo): string {
  if (controller.handler === "XInput") {
    const slot = controller.device.match(/#(\d+)$/)?.[1];
    return slot && slot !== "1" ? `Xbox controller ${slot}` : "Xbox controller";
  }
  return controller.device.replace(/ \d+$/, "");
}

function connectedList(view: ControllerView): HTMLElement {
  const box = document.createElement("div");
  box.className = "pads";
  for (const pad of view.connected) {
    const chip = document.createElement("span");
    chip.className = "pad-chip";
    chip.innerHTML = `<span class="dot"></span><span class="pad-chip-name"></span>`;
    chip.querySelector<HTMLElement>(".pad-chip-name")!.textContent = pad.name;
    box.appendChild(chip);
  }
  return box;
}

function scopePicker(scope: string): HTMLElement {
  const games = (store.get().games ?? []).filter((game) => game.set_up);
  const select = document.createElement("select");
  select.className = "select";
  const every = new Option("Every game", "");
  select.add(every);
  for (const game of games) select.add(new Option(game.title, game.title_id));
  select.value = scope;
  select.onchange = () => store.setControllerScope(select.value);
  return select;
}

export async function renderController(): Promise<View> {
  const scope = store.get().controllerScope ?? "";
  const view = await controllerView(scope);
  const content = document.createElement("div");
  content.className = "controller";

  const note = document.createElement("div");
  note.className = "note plain";

  if (view.connected.length === 0 && !view.bound) {
    const empty = emptyState(
      "No controller plugged in",
      "Plug one in and it sets itself up. Xbox, PlayStation, Switch Pro and 8BitDo pads all work."
    );
    const again = document.createElement("button");
    again.className = "small-btn";
    again.style.marginTop = "14px";
    again.textContent = "Check again";
    again.onclick = () => store.redraw();
    empty.appendChild(again);
    content.appendChild(empty);
    return { title: "Controller", subtitle: "Nothing plugged in", content };
  }

  // The pad this layout drives: whatever it was set up with, or else the
  // first one plugged in, which is what a change will bind to.
  const target = view.bound ?? view.connected[0];
  let bindings: Binding[] = view.bindings;

  const save = async () => {
    if (!target) {
      note.textContent = "Plug in a controller first.";
      return;
    }
    try {
      await saveController(scope, target, bindings);
      note.textContent = "";
      if (!view.own) store.redraw();
    } catch (err) {
      note.textContent = typeof err === "string" ? err : "Couldn't save the controller settings.";
    }
  };

  // Status and the pads that are in.
  const head = document.createElement("div");
  head.className = "pad-head";
  const status = document.createElement("div");
  status.className = "pad-status";
  if (view.bound) {
    status.textContent =
      scope && !view.own
        ? `Using the layout for every game, on ${padName(view.bound)}.`
        : `Set up on ${padName(view.bound)}.`;
  } else {
    status.textContent = "Not set up yet. It sets itself up the first time you press Play.";
  }
  head.append(status, connectedList(view));

  const actions = document.createElement("div");
  actions.className = "row-actions";
  actions.appendChild(scopePicker(scope));
  if (!view.bound) {
    const now = document.createElement("button");
    now.className = "small-btn";
    now.textContent = "Set up now";
    now.onclick = async () => {
      now.disabled = true;
      try {
        await setUpController(scope);
        store.redraw();
      } catch (err) {
        note.textContent = typeof err === "string" ? err : "Couldn't set up the controller.";
        now.disabled = false;
      }
    };
    actions.appendChild(now);
  } else {
    const reset = document.createElement("button");
    reset.className = "small-btn";
    reset.textContent = "Restore defaults";
    reset.onclick = async () => {
      try {
        await setUpController(scope);
        store.redraw();
      } catch (err) {
        note.textContent = typeof err === "string" ? err : "Couldn't set up the controller.";
      }
    };
    actions.appendChild(reset);
  }
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

  if (scope && !view.own) {
    const hint = document.createElement("div");
    hint.className = "note plain";
    hint.textContent = "Changing a button here gives this game a layout of its own.";
    content.appendChild(hint);
  }

  // The map itself: the pad in the middle, its inputs either side.
  const map = document.createElement("div");
  map.className = "pad-map";

  const art = document.createElement("div");
  art.className = "pad-mid";
  art.innerHTML = PAD_ART;
  const svg = art.querySelector("svg")!;

  const light = (key: string, on: boolean) => {
    svg.querySelector(`[data-part="${partFor(key)}"]`)?.classList.toggle("hot", on);
  };

  const row = ([key, label]: [string, string]): HTMLElement => {
    const line = document.createElement("label");
    line.className = "map-row";
    const name = document.createElement("span");
    name.className = "map-k";
    name.textContent = label;

    const select = document.createElement("select");
    select.className = "map-v";
    for (const choice of view.choices) select.add(new Option(PHYSICAL[choice] ?? choice, choice));
    select.value = bindings.find((b) => b.key === key)?.button ?? "";
    select.onchange = () => {
      bindings = bindings.map((b) => (b.key === key ? { ...b, button: select.value } : b));
      void save();
    };

    line.onmouseenter = () => light(key, true);
    line.onmouseleave = () => light(key, false);
    select.onfocus = () => light(key, true);
    select.onblur = () => light(key, false);
    line.append(name, select);
    return line;
  };

  const column = (groups: Group[]): HTMLElement => {
    const col = document.createElement("div");
    col.className = "map-col";
    for (const group of groups) {
      const box = document.createElement("div");
      box.className = "map-group";
      const heading = document.createElement("div");
      heading.className = "sec-h";
      heading.textContent = group.title;
      box.appendChild(heading);
      group.rows.forEach((entry) => box.appendChild(row(entry)));
      col.appendChild(box);
    }
    return col;
  };

  const middle = document.createElement("div");
  middle.className = "map-group";
  MIDDLE.rows.forEach((entry) => middle.appendChild(row(entry)));
  art.appendChild(middle);

  map.append(column(LEFT), art, column(RIGHT));
  content.appendChild(map);

  const subtitle = view.connected.length
    ? view.connected.length === 1
      ? "1 controller plugged in"
      : `${view.connected.length} controllers plugged in`
    : "Nothing plugged in right now";
  return { title: "Controller", subtitle, content };
}
