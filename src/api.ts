import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface CpuInfo {
  brand: string;
  physical_cores: number | null;
  logical_cores: number;
}

export interface MemoryInfo {
  total_bytes: number;
}

export interface GpuInfo {
  name: string;
  dedicated_memory_bytes: number;
}

export interface DisplayInfo {
  width: number;
  height: number;
  refresh_hz: number;
}

export interface HardwareInfo {
  cpu: CpuInfo;
  memory: MemoryInfo;
  gpu: GpuInfo | null;
  display: DisplayInfo | null;
}

export function getHardwareInfo(): Promise<HardwareInfo> {
  return invoke("get_hardware_info");
}

export interface InstallProgress {
  stage: "checking" | "downloading" | "verifying" | "extracting" | "done";
  bytes: number;
  total: number;
}

export function getRpcs3Version(): Promise<string | null> {
  return invoke("get_rpcs3_version");
}

export function installRpcs3(): Promise<string> {
  return invoke("install_rpcs3");
}

export function cancelRpcs3Install(): Promise<void> {
  return invoke("cancel_rpcs3_install");
}

export function onRpcs3InstallProgress(handler: (progress: InstallProgress) => void): Promise<UnlistenFn> {
  return listen<InstallProgress>("rpcs3-install-progress", (event) => handler(event.payload));
}

export function getFirmwareVersion(): Promise<string | null> {
  return invoke("get_firmware_version");
}

export function installFirmware(path: string): Promise<string> {
  return invoke("install_firmware", { path });
}
