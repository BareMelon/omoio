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
  /// What the dump itself reports.
  version: string | null;
  /// The official update Omoio installed, if any. This is what actually runs.
  update_version: string | null;
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

export function onGameFullscreen(handler: (on: boolean) => void): Promise<UnlistenFn> {
  return listen<boolean>("game-fullscreen", (event) => handler(event.payload));
}

export interface Machine {
  rpcs3: string | null;
  cpu: string | null;
  os: string | null;
  gpu: string | null;
  renderer: string | null;
}

export interface PlaySession {
  title_id: string;
  title: string;
  started: string;
  seconds: number;
  ending: "stopped" | "closed" | "crashed";
  machine: Machine;
  problems: string[];
  log_file: string;
}

export function listSessions(titleId?: string): Promise<PlaySession[]> {
  return invoke("list_sessions", { titleId: titleId ?? null });
}

export function readSessionLog(path: string): Promise<string> {
  return invoke("read_session_log", { path });
}

export function sessionPrompt(logFile: string): Promise<string> {
  return invoke("session_prompt", { logFile });
}

export interface Places {
  data: string;
  library: string;
  settings: string;
  logs: string;
  covers: string;
  rpcs3: string;
  games_folder: string | null;
}

export function getPlaces(): Promise<Places> {
  return invoke("get_places");
}

export interface GameOption {
  /// The setting's full path through RPCS3's config, joined with newlines.
  /// A section name can contain a slash, so nothing gentler is safe.
  key: string;
  group: string;
  name: string;
  label: string;
  hint: string;
  kind: "choice" | "number" | "switch" | "text";
  choices: string[];
  default: string;
  min: number;
  max: number;
  common: boolean;
}

/// path -> value. Anything absent is RPCS3's own default.
export type ChosenSettings = Record<string, string>;

export function gameSettings(titleId: string): Promise<[GameOption[], ChosenSettings]> {
  return invoke("game_settings", { titleId });
}

export function setGameSettings(titleId: string, chosen: ChosenSettings): Promise<void> {
  return invoke("set_game_settings", { titleId, chosen });
}

export interface Settings {
  games_folder: string | null;
  start_fullscreen: boolean;
  keep_sessions: number;
}

export function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export function setStartFullscreen(on: boolean): Promise<void> {
  return invoke("set_start_fullscreen", { on });
}

export function setKeepSessions(keep: number): Promise<void> {
  return invoke("set_keep_sessions", { keep });
}

export function revealFolder(path: string): Promise<void> {
  return invoke("reveal_folder", { path });
}

export function forgetAllGames(): Promise<void> {
  return invoke("forget_all_games");
}

export function clearSessionLogs(): Promise<void> {
  return invoke("clear_session_logs");
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

export interface Compatibility {
  known: boolean;
  label: string;
  tone: "go" | "warn" | "bad" | "mute";
  explanation: string;
  checked: string;
  stale: boolean;
  have_list: boolean;
}

export function gameCompatibility(titleId: string): Promise<Compatibility> {
  return invoke("game_compatibility", { titleId });
}

export function refreshCompatibility(): Promise<number> {
  return invoke("refresh_compatibility");
}

export interface Patch {
  hash: string;
  name: string;
  game: string;
  author: string;
  notes: string;
  version: string;
  versions: string[];
  applies: boolean;
  enabled: boolean;
}

export interface PatchList {
  have_list: boolean;
  patches: Patch[];
}

export function gamePatches(titleId: string): Promise<PatchList> {
  return invoke("game_patches", { titleId });
}

export function setPatchEnabled(
  patch: Patch,
  titleId: string,
  enabled: boolean
): Promise<void> {
  return invoke("set_patch_enabled", { patch, titleId, enabled });
}

export function refreshPatches(): Promise<number> {
  return invoke("refresh_patches");
}

export interface ScanProgress {
  stage: "looking" | "reading";
  done: number;
  total: number;
  title: string;
}

export interface ScanResult {
  added: number;
  already_there: number;
  not_games: number;
  cancelled: boolean;
}

export function scanFolder(path: string): Promise<ScanResult> {
  return invoke("scan_folder", { path });
}

export function onScanProgress(handler: (progress: ScanProgress) => void): Promise<UnlistenFn> {
  return listen<ScanProgress>("scan-progress", (event) => handler(event.payload));
}

export interface GameUpdate {
  version: string;
  size: number;
  sha1: string;
  url: string;
  firmware: string;
}

export function gameUpdates(titleId: string): Promise<GameUpdate[]> {
  return invoke("game_updates", { titleId });
}

export function installUpdate(titleId: string, update: GameUpdate): Promise<void> {
  return invoke("install_update", { titleId, update });
}

export function cancelUpdate(): Promise<void> {
  return invoke("cancel_update");
}

export function onUpdateProgress(handler: (progress: ImportProgress) => void): Promise<UnlistenFn> {
  return listen<ImportProgress>("update-progress", (event) => handler(event.payload));
}

export interface SaveBackup {
  /// Seconds since the epoch, like the session logs use.
  made: number;
  bytes: number;
  folders: number;
}

export function gameSaves(titleId: string): Promise<[boolean, SaveBackup[]]> {
  return invoke("game_saves", { titleId });
}

export function backUpSaves(titleId: string): Promise<SaveBackup | null> {
  return invoke("back_up_saves", { titleId });
}

export function restoreSaves(titleId: string, made: number): Promise<void> {
  return invoke("restore_saves", { titleId, made });
}

export function forgetBackup(titleId: string, made: number): Promise<void> {
  return invoke("forget_backup", { titleId, made });
}

export interface Account {
  username: string;
  /// Empty when RPCS3 is set to a combination no region of ours describes.
  region: string;
}

export interface RegionChoice {
  id: string;
  name: string;
  language: string;
}

export function getAccount(): Promise<Account> {
  return invoke("get_account");
}

export function listRegions(): Promise<RegionChoice[]> {
  return invoke("list_regions");
}

export function setUsername(name: string): Promise<string> {
  return invoke("set_username", { name });
}

export function setRegion(id: string): Promise<void> {
  return invoke("set_region", { id });
}

export function needsSetup(): Promise<boolean> {
  return invoke("needs_setup");
}

export function finishSetup(): Promise<void> {
  return invoke("finish_setup");
}

export interface PendingUpdate {
  title_id: string;
  title: string;
  installed: string;
  newest: string;
}

export function pendingUpdates(): Promise<[boolean, PendingUpdate[]]> {
  return invoke("pending_updates");
}
