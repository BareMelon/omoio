import {
  cancelRpcs3Install,
  getHardwareInfo,
  installRpcs3,
  onRpcs3InstallProgress,
  type HardwareInfo,
  type InstallProgress,
} from "../api";
import { store } from "../state";
import type { View } from "./view";

function bytesToGB(bytes: number): number {
  return Math.round(bytes / 1024 ** 3);
}

function hwRow(label: string, value: string, note: string): string {
  return `<div class="hw-row"><span class="hw-k">${label}</span><span class="hw-v">${value}</span><span class="hw-n">${note}</span></div>`;
}

function renderHardware(hw: HardwareInfo): HTMLElement {
  const cpuNote = hw.cpu.physical_cores
    ? `${hw.cpu.physical_cores} cores / ${hw.cpu.logical_cores} threads`
    : `${hw.cpu.logical_cores} threads`;
  const gpuValue = hw.gpu?.name ?? "Not detected";
  const gpuNote = hw.gpu ? `${bytesToGB(hw.gpu.dedicated_memory_bytes)} GB` : "";
  const displayValue = hw.display ? `${hw.display.width} × ${hw.display.height}` : "Not detected";
  const displayNote = hw.display ? `${hw.display.refresh_hz} Hz` : "";

  const el = document.createElement("div");
  el.className = "sec";
  el.innerHTML = `
    <div class="sec-h">Detected hardware</div>
    ${hwRow("CPU", hw.cpu.brand, cpuNote)}
    ${hwRow("GPU", gpuValue, gpuNote)}
    ${hwRow("Memory", `${bytesToGB(hw.memory.total_bytes)} GB`, "")}
    ${hwRow("Display", displayValue, displayNote)}
  `;
  return el;
}

const STAGE_LABEL: Record<InstallProgress["stage"], string> = {
  checking: "Checking for the latest build…",
  downloading: "Downloading…",
  verifying: "Verifying…",
  extracting: "Extracting…",
  done: "Done",
};

function renderEmulator(version: string | null): HTMLElement {
  const el = document.createElement("div");
  el.className = "sec";

  const status = document.createElement("div");
  status.className = "cfg-row";
  status.innerHTML = `
    <span class="cfg-k">RPCS3</span>
    <span class="cfg-v" id="rpcs3-version"></span>
  `;

  const action = document.createElement("div");
  action.className = "cfg-row";

  el.innerHTML = `<div class="sec-h">Emulator</div>`;
  el.appendChild(status);
  el.appendChild(action);

  const versionEl = status.querySelector<HTMLElement>("#rpcs3-version")!;

  function showIdle(currentVersion: string | null) {
    versionEl.textContent = currentVersion ?? "Not installed";
    action.innerHTML = currentVersion
      ? ""
      : `<button class="small-btn" id="install-rpcs3">Install RPCS3</button>`;
    action.querySelector<HTMLButtonElement>("#install-rpcs3")?.addEventListener("click", startInstall);
  }

  function showProgress(progress: InstallProgress) {
    const pct = progress.stage === "downloading" && progress.total > 0
      ? Math.round((progress.bytes / progress.total) * 100)
      : null;
    action.innerHTML = `
      <div class="progress-row" style="flex:1">
        <div class="progress-label">
          <span>${STAGE_LABEL[progress.stage]}</span>
          ${pct !== null ? `<span class="pct">${pct}%</span>` : ""}
        </div>
        <div class="progress"><div class="progress-fill" style="width:${pct ?? 8}%"></div></div>
      </div>
      <button class="small-btn" id="cancel-install">Cancel</button>
    `;
    action.querySelector<HTMLButtonElement>("#cancel-install")?.addEventListener("click", () => {
      cancelRpcs3Install();
    });
  }

  async function startInstall() {
    showProgress({ stage: "checking", bytes: 0, total: 0 });
    const unlisten = await onRpcs3InstallProgress(showProgress);
    try {
      const installed = await installRpcs3();
      store.setRpcs3Version(installed);
      showIdle(installed);
    } catch (err) {
      if (err !== "cancelled") {
        console.error("RPCS3 install failed:", err);
        action.innerHTML = `<div class="progress-error">Couldn't install RPCS3. Check your internet connection and try again.</div>`;
      } else {
        showIdle(null);
      }
    } finally {
      unlisten();
    }
  }

  showIdle(version);
  return el;
}

export async function renderSystem(): Promise<View> {
  const hw = await getHardwareInfo();
  const rpcs3Version = store.get().rpcs3Version;

  const content = document.createElement("div");
  content.className = "hw";
  content.appendChild(renderHardware(hw));
  content.appendChild(renderEmulator(rpcs3Version));

  return {
    title: "System",
    subtitle: hw.gpu ? `${hw.cpu.brand} · ${hw.gpu.name}` : hw.cpu.brand,
    content,
  };
}
