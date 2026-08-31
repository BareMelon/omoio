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

export interface Game {
  title_id: string;
  title: string;
  version: string | null;
  path: string;
  size_bytes: number;
  /// False when the folder isn't reachable right now, e.g. an external drive.
  available: boolean;
  /// Cached copy of the dump's own ICON0.PNG, or null if it had none.
  cover: string | null;
}

export function listGames(): Promise<Game[]> {
  return invoke("list_games");
}

export function importGame(path: string): Promise<Game> {
  return invoke("import_game", { path });
}

export function removeGame(titleId: string): Promise<void> {
  return invoke("remove_game", { titleId });
}

export function launchGame(titleId: string): Promise<void> {
  return invoke("launch_game", { titleId });
}

export interface Playing {
  title_id: string;
  title: string;
}

export function stopGame(): Promise<void> {
  return invoke("stop_game");
}

export function playingGame(): Promise<Playing | null> {
  return invoke("playing_game");
}

export function setGameFullscreen(fullscreen: boolean): Promise<void> {
  return invoke("set_game_fullscreen", { fullscreen });
}

export function onGameStarted(handler: (playing: Playing) => void): Promise<UnlistenFn> {
  return listen<Playing>("game-started", (event) => handler(event.payload));
}

export function onGameStopped(handler: () => void): Promise<UnlistenFn> {
  return listen("game-stopped", () => handler());
}

export function getGamesFolder(): Promise<string | null> {
  return invoke("get_games_folder");
}

export function setGamesFolder(path: string): Promise<void> {
  return invoke("set_games_folder", { path });
}

export function importArchive(path: string): Promise<Game> {
  return invoke("import_archive", { path });
}

export function cancelImport(): Promise<void> {
  return invoke("cancel_import");
}

export interface ImportProgress {
  stage: "unpacking" | "identifying";
  bytes: number;
  total: number;
}

export function onImportProgress(handler: (progress: ImportProgress) => void): Promise<UnlistenFn> {
  return listen<ImportProgress>("import-progress", (event) => handler(event.payload));
}

export function getFirmwareVersion(): Promise<string | null> {
  return invoke("get_firmware_version");
}

export function installFirmware(path: string): Promise<string> {
  return invoke("install_firmware", { path });
}
