import {
  cancelCemuInstall,
  emulatorVersions,
  installCemu,
  onCemuInstallProgress,
  type InstallProgress,
} from "../api";
import { store } from "../state";
import type { View } from "./view";

type Emulator = {
  name: string;
  console: string;
  /// The console as it is usually shortened. A fact about the hardware, shown
  /// in our own lettering, rather than anyone's logo.
  badge: string;
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

/// The ten most starred emulators on GitHub, counted on 11 September 2026.
/// Stars are the one measure of popularity anyone can check, which is why they
/// decide the order rather than a list of favourites.
///
/// Switch and 3DS emulators are absent on purpose. The big ones were shut down
/// after legal action, and they cannot run anything without decryption keys,
/// which Omoio never handles.
const EMULATORS: Emulator[] = [
  { name: "shadPS4", console: "PlayStation 4", badge: "PS4", hue: SONY_HOME },
  { name: "RPCS3", console: "PlayStation 3", badge: "PS3", hue: SONY_HOME, runs: "ps3" },
  { name: "PCSX2", console: "PlayStation 2", badge: "PS2", hue: SONY_HOME, needs: "Your own BIOS" },
  { name: "Dolphin", console: "GameCube and Wii", badge: "Wii", hue: NINTENDO_HOME },
  { name: "PPSSPP", console: "PSP", badge: "PSP", hue: SONY_HANDHELD },
  { name: "DuckStation", console: "PlayStation", badge: "PS1", hue: SONY_HOME, needs: "Your own BIOS" },
  { name: "Cemu", console: "Wii U", badge: "Wii U", hue: NINTENDO_HOME, runs: "wiiu" },
  { name: "mGBA", console: "Game Boy Advance", badge: "GBA", hue: NINTENDO_HANDHELD },
  { name: "Vita3K", console: "PS Vita", badge: "Vita", hue: SONY_HANDHELD, needs: "Your own firmware" },
  { name: "melonDS", console: "Nintendo DS", badge: "DS", hue: NINTENDO_HANDHELD },
];

const STAGE: Record<InstallProgress["stage"], string> = {
  checking: "Finding the newest release…",
  downloading: "Downloading…",
  verifying: "Checking the download…",
  extracting: "Unpacking…",
  done: "Done",
};

/// A tile in the family's colour with the console's short name on it. The
/// colours are worked out from the hue, the same way the library's
/// placeholder art is, so nothing here is a fixed colour.
function badge(emulator: Emulator, size: "big" | "small"): HTMLElement {
  const tile = document.createElement("div");
  tile.className = `emu-badge ${size}`;
  tile.style.background = `linear-gradient(150deg, hsl(${emulator.hue} 58% 30%), hsl(${emulator.hue} 62% 13%))`;
  tile.style.borderColor = `hsl(${emulator.hue} 55% 42% / 0.55)`;
  tile.textContent = emulator.badge;
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
    card.append(badge(emulator, "big"), text(emulator));
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
    "Ordered by GitHub stars. Each one is added once its downloads and licence have been checked.";
  content.appendChild(foot);

  return { title: "Emulators", subtitle: `${mine.length} installed`, content };
}
