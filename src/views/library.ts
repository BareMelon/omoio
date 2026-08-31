import { convertFileSrc } from "@tauri-apps/api/core";
import type { Game } from "../api";
import { launchGame, removeGame, listGames } from "../api";
import { openImportSheet } from "../components/importSheet";
import { store } from "../state";
import { emptyState, type View } from "./view";

function formatSize(bytes: number): string {
  if (bytes <= 0) return "";
  const gb = bytes / 1024 ** 3;
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${Math.round(bytes / 1024 ** 2)} MB`;
}

/// Every title id gives the same colours every time, so a game without art
/// still gets its own recognisable tile rather than a grey box.
function placeholderArt(game: Game): string {
  let hash = 0;
  for (const ch of game.title_id) hash = (hash * 31 + ch.charCodeAt(0)) & 0xffff;
  const hue = hash % 360;
  const back = `hsl(${hue} 32% 14%)`;
  const front = `hsl(${(hue + 40) % 360} 58% 52%)`;
  const initials = game.title.replace(/[^A-Za-z0-9]/g, "").slice(0, 2).toUpperCase();
  return `
    <svg viewBox="0 0 100 120" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      <rect width="100" height="120" fill="${back}"/>
      <circle cx="50" cy="52" r="26" fill="none" stroke="${front}" stroke-width="2.5" opacity=".8"/>
      <circle cx="50" cy="52" r="11" fill="${front}"/>
      <text x="50" y="103" text-anchor="middle" fill="${front}"
            font-size="15" font-weight="700" font-family="system-ui">${initials}</text>
    </svg>`;
}

function gameCard(game: Game): HTMLElement {
  const card = document.createElement("button");
  card.className = game.available ? "card" : "card ghost";
  card.title = game.available ? `Play ${game.title}` : "This game's folder isn't there";

  // A dump's ICON0 is 320x176, landscape, while the tile is portrait like the
  // box art a metadata service would eventually give us. Cropping to fill cuts
  // the logo in half, so the icon is shown whole over a blurred copy of itself.
  const art = game.cover
    ? `<img class="art-back" src="${convertFileSrc(game.cover)}" alt="" aria-hidden="true">
       <img class="art-fit" src="${convertFileSrc(game.cover)}" alt="" loading="lazy">`
    : placeholderArt(game);

  card.innerHTML = `
    <div class="art">
      ${art}
      ${game.available ? "" : `<span class="badge warn">Offline</span>`}
      <span class="art-name"></span>
    </div>
    <div class="meta">
      <span class="id">${game.title_id}</span>
      <span>· ${formatSize(game.size_bytes)}</span>
    </div>
  `;
  // Set through textContent so a game's own title can never be markup.
  card.querySelector<HTMLElement>(".art-name")!.textContent = game.title;

  if (game.available) {
    card.onclick = async () => {
      card.disabled = true;
      try {
        await launchGame(game.title_id);
      } catch (err) {
        store.setNotice(typeof err === "string" ? err : "Couldn't start this game.");
      } finally {
        card.disabled = false;
      }
    };
  }
  return card;
}

function detailsFor(game: Game): HTMLElement {
  const row = document.createElement("div");
  row.className = "card-actions";
  const remove = document.createElement("button");
  remove.className = "link-btn";
  remove.textContent = "Remove";
  remove.onclick = async (e) => {
    e.stopPropagation();
    remove.disabled = true;
    await removeGame(game.title_id);
    store.setGames(await listGames());
  };
  row.appendChild(remove);
  return row;
}

export function renderLibrary(): View {
  const { games, search, notice } = store.get();
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
    shown.forEach((game) => {
      const cell = document.createElement("div");
      cell.append(gameCard(game), detailsFor(game));
      grid.appendChild(cell);
    });
    content.appendChild(grid);
  }

  const total = games.reduce((sum, g) => sum + g.size_bytes, 0);
  const subtitle =
    games.length === 0
      ? "0 games"
      : `${games.length} ${games.length === 1 ? "game" : "games"}${total > 0 ? ` · ${formatSize(total)}` : ""}`;

  return { title: "Library", subtitle, content };
}
