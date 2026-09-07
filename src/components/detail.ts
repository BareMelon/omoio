import { convertFileSrc } from "@tauri-apps/api/core";
import {
  gameCompatibility,
  gamePatches,
  gameSaves,
  gameSettings,
  gameUpdates,
  launchGame,
  listGames,
  refreshCompatibility,
  removeGame,
  type Game,
} from "../api";
import { openGameSettings } from "./gameSettingsSheet";
import { openPatches } from "./patchesSheet";
import { openSaves } from "./savesSheet";
import { openUpdates } from "./updatesSheet";
import { store } from "../state";

function formatSize(bytes: number): string {
  if (bytes <= 0) return "unknown";
  const gb = bytes / 1024 ** 3;
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${Math.round(bytes / 1024 ** 2)} MB`;
}

function row(label: string, value: string, tone = ""): string {
  return `<div class="row"><span class="row-k">${label}</span><span class="row-v ${tone}">${value}</span></div>`;
}

function fill(body: HTMLElement, hero: HTMLElement, game: Game): void {
  hero.innerHTML = game.cover
    ? `<img src="${convertFileSrc(game.cover)}" alt="">`
    : `<div class="d-art-blank"></div>`;

  body.innerHTML = `
    <div class="d-title"></div>
    <div class="d-sub"></div>
    <button class="play" id="detail-play">
      <svg viewBox="0 0 12 14" fill="currentColor"><path d="M1 1l10 6-10 6z"/></svg>Play
    </button>
    <div class="note" id="detail-note"></div>
    <div class="sec">
      <div class="sec-h">Details</div>
      ${row("Version", game.version ? game.version : "unknown")}
      ${row("Size on disk", formatSize(game.size_bytes))}
      ${row(
        "Files",
        game.available ? "Available" : "Not found",
        game.available ? "" : "warn"
      )}
    </div>
    <div class="sec">
      <div class="sec-h">Game version</div>
      <button class="small-btn wide" id="detail-update">Check for updates</button>
      <div class="note plain" id="detail-update-note"></div>
      <button class="link-btn gone" id="detail-update-more">Choose another version</button>
    </div>
    <div class="sec">
      <div class="sec-h">Saved games</div>
      <button class="small-btn wide" id="detail-saves">Back up and restore</button>
      <div class="note plain" id="detail-saves-note"></div>
    </div>
    <div class="sec">
      <div class="sec-h">How well it runs</div>
      <div class="compat" id="detail-compat">
        <span class="status" id="detail-compat-badge"></span>
        <button class="link-btn" id="detail-compat-get"></button>
      </div>
      <div class="note plain" id="detail-compat-note"></div>
    </div>
    <div class="sec">
      <div class="sec-h">Emulator</div>
      <button class="small-btn wide" id="detail-settings">Change settings</button>
      <div class="note plain" id="detail-settings-note"></div>
      <button class="small-btn wide" id="detail-patches">Patches</button>
      <div class="note plain" id="detail-patches-note"></div>
    </div>
    <div class="sec">
      <div class="sec-h">Location</div>
      <div class="d-path"></div>
      <button class="link-btn" id="detail-remove">Remove from library</button>
    </div>
  `;
  // Set through textContent so a game's own name or path is never treated as markup.
  body.querySelector<HTMLElement>(".d-title")!.textContent = game.title;
  body.querySelector<HTMLElement>(".d-sub")!.textContent = game.version
    ? `${game.title_id} · version ${game.version}`
    : game.title_id;
  body.querySelector<HTMLElement>(".d-path")!.textContent = game.path;

  const note = body.querySelector<HTMLElement>("#detail-note")!;
  const play = body.querySelector<HTMLButtonElement>("#detail-play")!;
  play.disabled = !game.available;
  if (!game.available) {
    note.textContent = "Reconnect the drive this game is on to play it.";
  }
  play.onclick = async () => {
    play.disabled = true;
    note.textContent = "Starting…";
    try {
      await launchGame(game.title_id);
      store.setSelected(null);
    } catch (err) {
      note.textContent = typeof err === "string" ? err : "Couldn't start this game.";
      play.disabled = false;
    }
  };

  const running = game.update_version ?? game.version;
  const updateButton = body.querySelector<HTMLButtonElement>("#detail-update")!;
  const updateNote = body.querySelector<HTMLElement>("#detail-update-note")!;
  const moreVersions = body.querySelector<HTMLButtonElement>("#detail-update-more")!;

  const reload = async () => store.setGames(await listGames());
  const openList = () =>
    openUpdates(game.title_id, game.title, running, () => {
      void reload();
      void showPatchCount();
    });

  updateNote.textContent = game.update_version
    ? `Running version ${game.update_version}, updated by Omoio.`
    : `Running version ${game.version ?? "unknown"}, as the game shipped.`;

  // Checked on demand rather than on every selection: it is a request to
  // Sony, and opening a game should not quietly make one.
  updateButton.onclick = async () => {
    updateButton.disabled = true;
    updateButton.textContent = "Checking…";
    try {
      const updates = await gameUpdates(game.title_id);
      const newest = updates[0];
      if (!newest) {
        updateNote.textContent = "Sony never published an update for this game.";
      } else if (newest.version === running) {
        updateNote.textContent = `Version ${newest.version} is the newest there is.`;
        moreVersions.classList.remove("gone");
      } else {
        updateButton.textContent = `Update to ${newest.version}`;
        updateButton.onclick = openList;
        updateNote.textContent = `Version ${newest.version} is available.`;
        moreVersions.classList.remove("gone");
      }
    } catch (err) {
      updateNote.textContent =
        typeof err === "string" ? err : "Couldn't reach Sony's update service.";
    } finally {
      updateButton.disabled = false;
      if (updateButton.textContent === "Checking…") {
        updateButton.textContent = "Check for updates";
      }
    }
  };
  moreVersions.onclick = openList;

  const savesNote = body.querySelector<HTMLElement>("#detail-saves-note")!;
  const showSaves = async () => {
    const [hasSaves, backups] = await gameSaves(game.title_id);
    savesNote.textContent = !hasSaves
      ? "Nothing saved yet."
      : backups.length === 0
        ? "No copies kept yet."
        : backups.length === 1
          ? "1 copy kept."
          : `${backups.length} copies kept.`;
  };
  showSaves();
  body.querySelector<HTMLButtonElement>("#detail-saves")!.onclick = () =>
    openSaves(game.title_id, game.title, showSaves);

  const badge = body.querySelector<HTMLElement>("#detail-compat-badge")!;
  const compatNote = body.querySelector<HTMLElement>("#detail-compat-note")!;
  const getList = body.querySelector<HTMLButtonElement>("#detail-compat-get")!;
  const showCompat = async () => {
    const compat = await gameCompatibility(game.title_id);
    badge.textContent = compat.label;
    badge.className = `status ${compat.tone}`;
    compatNote.textContent = compat.checked
      ? `${compat.explanation} Last reported ${compat.checked}.`
      : compat.explanation;
    // Only offered when it would do something: the list is missing, or old
    // enough that a game's result may have moved on.
    getList.classList.toggle("gone", !compat.stale);
    getList.textContent = compat.have_list ? "Check for newer results" : "Get the list";
  };
  getList.onclick = async () => {
    getList.disabled = true;
    getList.textContent = "Getting…";
    try {
      await refreshCompatibility();
      await showCompat();
    } catch (err) {
      compatNote.textContent =
        typeof err === "string" ? err : "Couldn't get the compatibility list.";
    } finally {
      getList.disabled = false;
    }
  };
  showCompat();

  const patchesNote = body.querySelector<HTMLElement>("#detail-patches-note")!;
  const showPatchCount = async () => {
    const { have_list, patches } = await gamePatches(game.title_id);
    const fits = patches.filter((p) => p.applies);
    const on = fits.filter((p) => p.enabled).length;
    patchesNote.textContent = !have_list
      ? "No patch list yet."
      : fits.length === 0
        ? "None published for this game."
        : on === 0
          ? `${fits.length} available, none on.`
          : `${on} of ${fits.length} on.`;
  };
  showPatchCount();
  body.querySelector<HTMLButtonElement>("#detail-patches")!.onclick = () =>
    openPatches(game.title_id, game.title, showPatchCount);

  const settingsNote = body.querySelector<HTMLElement>("#detail-settings-note")!;
  const showSettingsCount = async () => {
    const [, chosen] = await gameSettings(game.title_id);
    const changed = Object.keys(chosen).length;
    settingsNote.textContent =
      changed === 0
        ? "Running with RPCS3's own settings."
        : `${changed} setting${changed === 1 ? "" : "s"} changed for this game.`;
  };
  showSettingsCount();
  body.querySelector<HTMLButtonElement>("#detail-settings")!.onclick = () =>
    openGameSettings(game.title_id, game.title, showSettingsCount);

  body.querySelector<HTMLButtonElement>("#detail-remove")!.onclick = async () => {
    await removeGame(game.title_id);
    store.setSelected(null);
    store.setGames(await listGames());
  };
}

export function renderDetail(): HTMLElement {
  const detail = document.createElement("aside");
  detail.className = "detail hidden";
  detail.innerHTML = `
    <div class="d-hero">
      <div class="d-art"></div>
      <button class="d-close" aria-label="Close">
        <svg viewBox="0 0 10 10"><path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" stroke-width="1.4"/></svg>
      </button>
    </div>
    <div class="d-body"></div>
  `;
  const hero = detail.querySelector<HTMLElement>(".d-art")!;
  const body = detail.querySelector<HTMLElement>(".d-body")!;

  detail.querySelector<HTMLButtonElement>(".d-close")!.onclick = () => store.setSelected(null);

  store.subscribe((state) => {
    // The game picture covers this side of the window, so nothing is shown
    // here while one is running.
    const game = state.playing
      ? undefined
      : state.games.find((g) => g.title_id === state.selected);
    detail.classList.toggle("hidden", !game);
    if (game) fill(body, hero, game);
  });

  return detail;
}
