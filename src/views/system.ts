import { getHardwareInfo, type HardwareInfo } from "../api";
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
  el.className = "hw";
  el.innerHTML = `
    <div class="sec">
      <div class="sec-h">Detected hardware</div>
      ${hwRow("CPU", hw.cpu.brand, cpuNote)}
      ${hwRow("GPU", gpuValue, gpuNote)}
      ${hwRow("Memory", `${bytesToGB(hw.memory.total_bytes)} GB`, "")}
      ${hwRow("Display", displayValue, displayNote)}
    </div>
  `;
  return el;
}

export async function renderSystem(): Promise<View> {
  const hw = await getHardwareInfo();
  return {
    title: "System",
    subtitle: hw.gpu ? `${hw.cpu.brand} · ${hw.gpu.name}` : hw.cpu.brand,
    content: renderHardware(hw),
  };
}
