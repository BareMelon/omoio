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
  /// False for a game noted from the catalogue that has no files yet. Not the
  /// same as `available`, which means the files exist but the drive is out.
  set_up: boolean;
  /// Where the cover came from: the dump's own icon, or RAWG.
  cover_source: "dump" | "rawg" | null;
  /// Which console the game is for, and so which emulator runs it.
  console: "ps3" | "wiiu";
  /// What its emulator can do beyond starting it.
  features: {
    updates: boolean;
    patches: boolean;
    settings: boolean;
    saves: boolean;
    compatibility: boolean;
  };
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
  /// Which console it ran on. Sessions kept before this was recorded were PS3.
  console?: "ps3" | "wiiu";
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

/// The options, what this game is set to, and why Omoio set any of them itself.
export function gameSettings(
  titleId: string
): Promise<[GameOption[], ChosenSettings, Record<string, string>]> {
  return invoke("game_settings", { titleId });
}

export function setGameSettings(titleId: string, chosen: ChosenSettings): Promise<void> {
  return invoke("set_game_settings", { titleId, chosen });
}

export interface Settings {
  games_folder: string | null;
  start_fullscreen: boolean;
  keep_sessions: number;
  /// The resolution scale Omoio set for this machine, once it has.
  tuned_scale: number | null;
  /// Real covers from RAWG in place of the generated tiles. Off until asked for.
  covers: boolean;
  /// The user's own RAWG key, kept on this machine only.
  rawg_key: string | null;
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

/// Every console's list, or only `console`'s.
export function refreshCompatibility(console?: Console): Promise<number> {
  return invoke("refresh_compatibility", { console: console ?? null });
}

export function cancelCompatibility(): Promise<void> {
  return invoke("cancel_compatibility");
}

export interface CompatProgress {
  stage: "names";
  bytes: number;
  total: number;
}

export function onCompatProgress(
  handler: (progress: CompatProgress) => void
): Promise<UnlistenFn> {
  return listen<CompatProgress>("compat-progress", (event) => handler(event.payload));
}

export interface Patch {
  hash: string;
  name: string;
  game: string;
  /// The serial the patch list files it under: the game's own title id, or
  /// "All" for a patch written for every game.
  serial: string;
  author: string;
  notes: string;
  version: string;
  versions: string[];
  applies: boolean;
  enabled: boolean;
  /// Why Omoio switches it on, when it is one of Omoio's own fixes.
  fix: string | null;
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

/// How far an update run has got, across every package in it.
export interface UpdateProgress {
  /// The version being downloaded or installed right now.
  version: string;
  /// Which package this is, counted from 1, and how many there are.
  step: number;
  steps: number;
  /// Bytes downloaded across the whole run, and the size of the whole run.
  bytes: number;
  total: number;
  /// True once this package is downloaded and is being installed.
  installing: boolean;
}

export function onUpdateProgress(handler: (progress: UpdateProgress) => void): Promise<UnlistenFn> {
  return listen<UpdateProgress>("update-progress", (event) => handler(event.payload));
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

export type Console = "ps3" | "wiiu";

export interface Release {
  title_id: string;
  region: string;
}

/// One game in the catalogue, however many times it was released.
export interface Listing {
  console: Console;
  console_name: string;
  /// Unique across the catalogue. Covers are cached under it.
  key: string;
  /// The game's name, or its title id when the list has none.
  name: string;
  named: boolean;
  /// The best any release of it is reported to do. Empty when nobody has.
  status: { label: string; tone: string; explanation: string };
  /// "Virtual Console" for an older console's game sold again, else empty.
  kind: string;
  regions: string[];
  /// Every release with a title id, the chosen region's first.
  releases: Release[];
  demo: boolean;
  owned: boolean;
  features: Game["features"];
}

export interface CatalogueFilter {
  query: string;
  console: Console | null;
  region: string;
  runs: string;
  hide_demos: boolean;
  sort: string;
  limit: number;
}

export interface CatalogueView {
  have_list: boolean;
  consoles: { console: Console; name: string }[];
  missing: Console[];
  total: number;
  shown: Listing[];
  sources: { label: string; url: string }[];
}

export function catalogue(filter: CatalogueFilter): Promise<CatalogueView> {
  return invoke("catalogue", { filter });
}

export function addToLibrary(console: Console, titleId: string, title: string): Promise<void> {
  return invoke("add_to_library", { console, titleId, title });
}

export interface InstalledPackage {
  title_id: string;
  title: string;
  version: string;
  size_bytes: number;
}

export function installedPackages(): Promise<InstalledPackage[]> {
  return invoke("installed_packages");
}

export function installPackage(path: string): Promise<void> {
  return invoke("install_package", { path });
}

export function removePackage(titleId: string): Promise<void> {
  return invoke("remove_package", { titleId });
}

export type DroppedKind = "folder" | "archive" | "unknown";

export function droppedKind(path: string): Promise<DroppedKind> {
  return invoke("dropped_kind", { path });
}

export interface ControllerInfo {
  /// What RPCS3 stores for the pad, e.g. "XInput Pad #1".
  device: string;
  name: string;
  handler: string;
}

export interface Binding {
  /// The PS3 input, e.g. "Cross" or "Left Stick Up".
  key: string;
  /// The physical button, by its SDL name, e.g. "South".
  button: string;
}

export interface PlayerSetup {
  controller: ControllerInfo;
  /// Whether that pad is plugged in right now.
  connected: boolean;
  bindings: Binding[];
}

export interface ControllerView {
  connected: ControllerInfo[];
  /// Players one to four, in order.
  players: PlayerSetup[];
  /// Every pad a player can be given, plugged in or not.
  pads: ControllerInfo[];
  /// False until the layout is written. The players shown are then the ones
  /// pressing Play will set up.
  saved: boolean;
  own: boolean;
  choices: string[];
}

export function controllerView(titleId: string): Promise<ControllerView> {
  return invoke("controller_view", { titleId });
}

export function setUpController(titleId: string): Promise<void> {
  return invoke("set_up_controller", { titleId });
}

/// `number` is the player, counted from 1.
export function saveController(
  titleId: string,
  number: number,
  controller: ControllerInfo,
  bindings: Binding[]
): Promise<void> {
  return invoke("save_controller", { titleId, number, controller, bindings });
}

export function forgetController(titleId: string): Promise<void> {
  return invoke("forget_controller", { titleId });
}

export function setCovers(on: boolean): Promise<void> {
  return invoke("set_covers", { on });
}

export function setRawgKey(key: string): Promise<void> {
  return invoke("set_rawg_key", { key });
}

/// Looks up covers for the library. Resolves to how many games have one.
export function fetchCovers(): Promise<number> {
  return invoke("fetch_covers");
}

/// The RAWG cover for a catalogue game, or null when there is none or covers
/// are off.
export function catalogueCover(key: string, name: string, console: Console): Promise<string | null> {
  return invoke("catalogue_cover", { key, name, console });
}

export interface EmulatorVersion {
  console: "ps3" | "wiiu";
  /// Null when it is not installed.
  version: string | null;
}

export function emulatorVersions(): Promise<EmulatorVersion[]> {
  return invoke("emulator_versions");
}

export function installCemu(): Promise<string> {
  return invoke("install_cemu");
}

export function cancelCemuInstall(): Promise<void> {
  return invoke("cancel_cemu_install");
}

export function onCemuInstallProgress(handler: (progress: InstallProgress) => void): Promise<UnlistenFn> {
  return listen<InstallProgress>("cemu-install-progress", (event) => handler(event.payload));
}
