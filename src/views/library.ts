import { convertFileSrc } from "@tauri-apps/api/core";
import type { Game } from "../api";
import { placeholderArt } from "../components/art";
import { rawgCredit } from "../components/rawgCredit";
import { openImportSheet } from "../components/importSheet";
import { store } from "../state";
import { emptyState, type View } from "./view";

/// How a console is written on a tile.
const SHORT: Record<Game["console"], string> = { ps3: "PS3", wiiu: "Wii U", ps4: "PS4" };

function formatSize(bytes: number): string {
  if (bytes <= 0) return "";
  const gb = bytes / 1024 ** 3;
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${Math.round(bytes / 1024 ** 2)} MB`;
}

function gameCard(game: Game, mixed: boolean, selected: boolean): HTMLElement {
  const card = document.createElement("button");
  card.title = game.title;

  // The pictures games have are wide: a dump's ICON0 is 320x176 and RAWG's
  // art is landscape too. So the tile is wide and the picture fills it,
  // where a portrait tile left a letterbox around every one of them.
  const art = game.cover
    ? `<img src="${convertFileSrc(game.cover)}" alt="" loading="lazy">`
    : placeholderArt(game.title_id, game.title);

  // Three states, and only one of them is a problem. A game with no files yet
  // is waiting to be imported; a game whose drive is out is fine and will come
  // back. Saying "Offline" for both would make the first look broken.
  const mark = !game.set_up
    ? `<span class="badge">Not set up</span>`
    : game.available
      ? ""
      : `<span class="badge warn">Offline</span>`;

  // Dimmed either way: neither can be played right now.
  card.className = `card${game.set_up && game.available ? "" : " ghost"}${selected ? " on" : ""}`;
  card.innerHTML = `
    <div class="art">
      ${art}
      ${mark}
    </div>
    <div class="card-name"></div>
    <div class="meta">
      <span class="id">${game.title_id}</span>
      <span>${game.set_up ? formatSize(game.size_bytes) : "no files yet"}</span>
      ${mixed ? `<span class="region">${SHORT[game.console]}</span>` : ""}
    </div>
  `;
  // Set through textContent so a game's own title can never be markup.
  card.querySelector<HTMLElement>(".card-name")!.textContent = game.title;

  // Opens the game rather than starting it: what it is, whether it can run,
  // and a Play button live in the panel.
  card.onclick = () => store.setSelected(game.title_id);
  return card;
}

export function renderLibrary(): View {
  const { games, search, notice, selected } = store.get();

  // The library has not been read yet. Showing "No games yet" here would tell
  // someone with a shelf full of games that they have none, for a moment, every
  // time the window opens.
  if (games === undefined) {
    return { title: "Library", subtitle: "", content: document.createElement("div") };
  }

  const query = search.trim().toLowerCase();
  const shown = query
    ? games.filter(
        (g) =>
          g.title.toLowerCase().includes(query) || g.title_id.toLowerCase().includes(query)
      )
    : games;

  const content = document.createElement("div");

  if (notice) {
    const banner = document.createElement("div");
    banner.className = "notice";
    banner.textContent = notice;
    content.appendChild(banner);
  }

  if (games.length === 0) {
    const empty = emptyState("No games yet", "Import a game to add it to your library.");
    const button = document.createElement("button");
    button.className = "small-btn";
    button.textContent = "Import game";
    button.style.marginTop = "14px";
    button.onclick = openImportSheet;
    empty.appendChild(button);
    content.appendChild(empty);
  } else if (shown.length === 0) {
    content.appendChild(
      emptyState(`No games match "${search.trim()}"`, "Try a different name or title ID.")
    );
  } else {
    const grid = document.createElement("div");
    grid.className = "grid";
    // The console is only worth saying once there is more than one.
    const mixed = new Set(games.map((game) => game.console)).size > 1;
    shown.forEach((game) => grid.appendChild(gameCard(game, mixed, game.title_id === selected)));
    content.appendChild(grid);
    if (shown.some((game) => game.cover_source === "rawg")) content.appendChild(rawgCredit());
  }

  const total = games.reduce((sum, g) => sum + g.size_bytes, 0);
  const subtitle =
    games.length === 0
      ? "0 games"
      : `${games.length} ${games.length === 1 ? "game" : "games"}${total > 0 ? ` · ${formatSize(total)}` : ""}`;

  return { title: "Library", subtitle, content };
}
