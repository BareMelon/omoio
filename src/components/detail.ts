import { convertFileSrc } from "@tauri-apps/api/core";
import { gameSettings, launchGame, listGames, removeGame, type Game } from "../api";
import { openGameSettings } from "./gameSettingsSheet";
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
      <div class="sec-h">Emulator</div>
      <button class="small-btn wide" id="detail-settings">Change settings</button>
      <div class="note plain" id="detail-settings-note"></div>
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

  const settingsNote = body.querySelector<HTMLElement>("#detail-settings-note")!;
  gameSettings(game.title_id).then(([, chosen]) => {
    const changed = Object.values(chosen).reduce((n, keys) => n + Object.keys(keys).length, 0);
    settingsNote.textContent =
      changed === 0
        ? "Running with RPCS3's own settings."
        : `${changed} setting${changed === 1 ? "" : "s"} changed for this game.`;
  });
  body.querySelector<HTMLButtonElement>("#detail-settings")!.onclick = () =>
    openGameSettings(game.title_id, game.title);

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
