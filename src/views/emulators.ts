import { open } from "@tauri-apps/plugin-dialog";
import {
  addCemuKeys,
  cancelCemuInstall,
  cemuKeys,
  emulatorVersions,
  installCemu,
  onCemuInstallProgress,
  type InstallProgress,
} from "../api";
import { store } from "../state";
import type { View } from "./view";
import rpcs3Icon from "../icons/emulators/rpcs3.svg";
import pcsx2Icon from "../icons/emulators/pcsx2.png";
import dolphinIcon from "../icons/emulators/dolphin.png";
import ppssppIcon from "../icons/emulators/ppsspp.png";
import duckstationIcon from "../icons/emulators/duckstation.png";
import cemuIcon from "../icons/emulators/cemu.png";
import mgbaIcon from "../icons/emulators/mgba.png";
import vita3kIcon from "../icons/emulators/vita3k.svg";
import melondsIcon from "../icons/emulators/melonds.svg";

type Emulator = {
  name: string;
  console: string;
  /// The emulator's own icon, unchanged from its project. Each keeps its
  /// project's licence; src/icons/emulators/NOTICE.md says whose and which.
  icon: string;
  /// One hue per family, so the grid reads by maker and kind at a glance.
  hue: number;
  needs?: string;
  /// The console Omoio runs it for, when Omoio can install it.
  runs?: "ps3" | "wiiu";
};

const SONY_HOME = 222;
const SONY_HANDHELD = 190;
const NINTENDO_HOME = 352;
const NINTENDO_HANDHELD = 268;

/// The most starred emulators on GitHub, counted on 11 September 2026.
/// Stars are the one measure of popularity anyone can check, which is why they
/// decide the order rather than a list of favourites.
///
/// Switch and 3DS emulators are absent on purpose. The big ones were shut down
/// after legal action, and they cannot run anything without decryption keys,
/// which Omoio never handles. shadPS4, the PS4's, is left out too: it runs only
/// games already decrypted, and a PS4 game someone bought is locked to Sony's
/// keys (Bertram dropped it, 2 October 2026).
const EMULATORS: Emulator[] = [
  { name: "RPCS3", console: "PlayStation 3", icon: rpcs3Icon, hue: SONY_HOME, runs: "ps3" },
  { name: "PCSX2", console: "PlayStation 2", icon: pcsx2Icon, hue: SONY_HOME, needs: "Your own BIOS" },
  { name: "Dolphin", console: "GameCube and Wii", icon: dolphinIcon, hue: NINTENDO_HOME },
  { name: "PPSSPP", console: "PSP", icon: ppssppIcon, hue: SONY_HANDHELD },
  { name: "DuckStation", console: "PlayStation", icon: duckstationIcon, hue: SONY_HOME, needs: "Your own BIOS" },
  { name: "Cemu", console: "Wii U", icon: cemuIcon, hue: NINTENDO_HOME, runs: "wiiu" },
  { name: "mGBA", console: "Game Boy Advance", icon: mgbaIcon, hue: NINTENDO_HANDHELD },
  { name: "Vita3K", console: "PS Vita", icon: vita3kIcon, hue: SONY_HANDHELD, needs: "Your own firmware" },
  { name: "melonDS", console: "Nintendo DS", icon: melondsIcon, hue: NINTENDO_HANDHELD },
];

const STAGE: Record<InstallProgress["stage"], string> = {
  checking: "Finding the newest release…",
  downloading: "Downloading…",
  verifying: "Checking the download…",
  extracting: "Unpacking…",
  done: "Done",
};

/// The emulator's icon on a faint tile in its family's colour, so the grid
/// still reads by maker and kind at a glance. The colours are worked out from
/// the hue, the same way the library's placeholder art is.
function badge(emulator: Emulator, size: "big" | "small"): HTMLElement {
  const tile = document.createElement("div");
  tile.className = `emu-badge ${size}`;
  tile.style.background = `radial-gradient(circle at 50% 35%, hsl(${emulator.hue} 55% 30% / 0.55), hsl(${emulator.hue} 60% 12% / 0.35))`;
  tile.style.borderColor = `hsl(${emulator.hue} 55% 42% / 0.4)`;
  const icon = document.createElement("img");
  icon.src = emulator.icon;
  // The name sits right beside it, so the picture says nothing more.
  icon.alt = "";
  tile.appendChild(icon);
  return tile;
}

function text(emulator: Emulator): HTMLElement {
  const box = document.createElement("div");
  box.className = "emu-text";
  const name = document.createElement("div");
  name.className = "emu-name";
  name.textContent = emulator.name;
  const sub = document.createElement("div");
  sub.className = "emu-sub";
  sub.textContent = emulator.console;
  box.append(name, sub);
  if (emulator.needs) {
    const needs = document.createElement("div");
    needs.className = "emu-needs";
    needs.textContent = emulator.needs;
    box.appendChild(needs);
  }
  return box;
}

/// Installs Cemu where the button was, with progress and a way to stop.
function cemuInstaller(box: HTMLElement): HTMLButtonElement {
  const install = document.createElement("button");
  install.className = "small-btn";
  install.textContent = "Install to Omoio";
  install.onclick = async () => {
    const bar = document.createElement("div");
    bar.className = "progress-row emu-progress";
    bar.innerHTML = `
      <div class="progress-label"><span class="stage"></span><span class="pct"></span></div>
      <div class="progress"><div class="progress-fill" style="width:4%"></div></div>
    `;
    const stage = bar.querySelector<HTMLElement>(".stage")!;
    const pct = bar.querySelector<HTMLElement>(".pct")!;
    const fill = bar.querySelector<HTMLElement>(".progress-fill")!;
    stage.textContent = STAGE.checking;
    const stop = document.createElement("button");
    stop.className = "link-btn";
    stop.textContent = "Cancel";
    stop.onclick = () => void cancelCemuInstall();
    box.querySelector(".note")?.remove();
    install.replaceWith(bar);
    bar.after(stop);

    const unlisten = await onCemuInstallProgress((progress) => {
      stage.textContent = STAGE[progress.stage];
      if (progress.stage === "downloading" && progress.total > 0) {
        const done = Math.min(100, Math.round((progress.bytes / progress.total) * 100));
        fill.style.width = `${done}%`;
        pct.textContent = `${done}%`;
      } else {
        pct.textContent = "";
      }
    });
    try {
      await installCemu();
      store.redraw();
    } catch (err) {
      bar.remove();
      stop.remove();
      const note = document.createElement("div");
      note.className = "note plain";
      note.textContent =
        err === "cancelled"
          ? "Stopped. Nothing was installed."
          : typeof err === "string"
            ? err
            : "Couldn't install Cemu.";
      box.append(install, note);
    } finally {
      unlisten();
    }
  };
  return install;
}

/// Cemu reads a Wii U disc image with that disc's key, from a keys file the
/// user brings. Omoio adds the file's keys where Cemu looks and nothing more.
async function keysBlock(): Promise<HTMLElement> {
  const box = document.createElement("div");
  box.className = "emu-keys";
  const said = document.createElement("span");
  said.className = "cfg-v";
  const show = (n: number) => {
    said.textContent = n === 0 ? "No keys added" : n === 1 ? "1 key" : `${n} keys`;
  };
  show(await cemuKeys());

  const add = document.createElement("button");
  add.className = "small-btn";
  add.textContent = "Add keys";
  add.onclick = async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      title: "Choose your keys file",
      filters: [{ name: "Keys file", extensions: ["txt"] }],
    });
    if (typeof picked !== "string") return;
    try {
      const added = await addCemuKeys(picked);
      show(await cemuKeys());
      if (added === 0) said.textContent += " · nothing new in that file";
    } catch (err) {
      said.textContent = typeof err === "string" ? err : "Couldn't add those keys.";
    }
  };

  const actions = document.createElement("div");
  actions.className = "row-actions";
  actions.append(said, add);
  const note = document.createElement("div");
  note.className = "note plain";
  note.textContent =
    "For Wii U disc images from discs you own. Omoio doesn't supply keys or say where to find them, and pirated or unlicensed games don't belong here. Keeping your copies legal is up to you.";
  box.append(actions, note);
  return box;
}

export async function renderEmulators(): Promise<View> {
  const versions = await emulatorVersions();
  const versionOf = (runs?: string) => versions.find((v) => v.console === runs)?.version ?? null;
  const content = document.createElement("div");
  content.className = "emu";

  // What is already here, on its own, so it does not read as one of ten.
  const mine = EMULATORS.filter((emulator) => versionOf(emulator.runs));
  const heading = document.createElement("div");
  heading.className = "sec-h";
  heading.textContent = "In Omoio";
  content.appendChild(heading);
  if (mine.length === 0) {
    const none = document.createElement("div");
    none.className = "note plain";
    none.textContent = "None yet. Install one below to play games for its console.";
    content.appendChild(none);
  }
  for (const emulator of mine) {
    const card = document.createElement("div");
    card.className = "emu-hero";
    const words = text(emulator);
    card.append(badge(emulator, "big"), words);
    if (emulator.runs === "wiiu") words.appendChild(await keysBlock());
    const side = document.createElement("div");
    side.className = "emu-side";
    side.innerHTML = `<span class="status go">Installed</span><span class="emu-ver"></span>`;
    side.querySelector<HTMLElement>(".emu-ver")!.textContent = versionOf(emulator.runs);
    card.appendChild(side);
    content.appendChild(card);
  }

  const more = document.createElement("div");
  more.className = "sec-h";
  more.style.marginTop = "24px";
  more.textContent = "More emulators";
  content.appendChild(more);

  const grid = document.createElement("div");
  grid.className = "emu-grid";
  for (const emulator of EMULATORS.filter((e) => !versionOf(e.runs))) {
    const card = document.createElement("div");
    card.className = "emu-card";
    const words = text(emulator);
    card.append(badge(emulator, "small"), words);
    if (emulator.runs === "wiiu") {
      words.appendChild(cemuInstaller(words));
    } else if (emulator.runs === "ps3") {
      // RPCS3's installer sits with its firmware on the System screen.
      const install = document.createElement("button");
      install.className = "small-btn";
      install.textContent = "Install to Omoio";
      install.onclick = () => store.setView("system");
      words.appendChild(install);
    } else {
      // Each gets its Install to Omoio button once its downloads and licence
      // have been checked. Until then it says so, rather than showing a
      // button that does nothing.
      const soon = document.createElement("span");
      soon.className = "emu-soon";
      soon.textContent = "Coming";
      card.appendChild(soon);
    }
    grid.appendChild(card);
  }
  content.appendChild(grid);

  const foot = document.createElement("div");
  foot.className = "note plain";
  foot.style.marginTop = "14px";
  foot.textContent =
    "Ordered by GitHub stars. Each one is added once its downloads and licence have been checked. The icons are the emulators' own, under their projects' licences.";
  content.appendChild(foot);

  return { title: "Emulators", subtitle: `${mine.length} installed`, content };
}
