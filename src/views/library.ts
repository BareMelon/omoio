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

function gameRow(game: Game): HTMLElement {
  const row = document.createElement("div");
  row.className = "game-row";

  const name = document.createElement("div");
  name.className = "game-name";
  name.textContent = game.title;

  const meta = document.createElement("div");
  meta.className = "game-meta";
  const bits = [game.title_id];
  if (game.version) bits.push(`v${game.version}`);
  const size = formatSize(game.size_bytes);
  if (size) bits.push(size);
  meta.textContent = bits.join("  ·  ");

  const left = document.createElement("div");
  left.append(name, meta);

  const right = document.createElement("div");
  right.className = "game-actions";

  const problem = document.createElement("span");
  problem.className = "game-missing";
  if (!game.available) {
    problem.textContent = "Folder not found";
  }
  right.appendChild(problem);

  const play = document.createElement("button");
  play.className = "play-btn";
  play.innerHTML = `<svg viewBox="0 0 12 14" fill="currentColor"><path d="M1 1l10 6-10 6z"/></svg>Play`;
  play.disabled = !game.available;
  play.onclick = async () => {
    problem.textContent = "";
    play.disabled = true;
    try {
      await launchGame(game.title_id);
    } catch (err) {
      problem.textContent = typeof err === "string" ? err : "Couldn't start this game.";
    } finally {
      play.disabled = !game.available;
    }
  };
  right.appendChild(play);

  const remove = document.createElement("button");
  remove.className = "small-btn";
  remove.textContent = "Remove";
  remove.onclick = async () => {
    remove.disabled = true;
    await removeGame(game.title_id);
    store.setGames(await listGames());
  };
  right.appendChild(remove);

  row.append(left, right);
  return row;
}

export function renderLibrary(): View {
  const { games } = store.get();

  const content = document.createElement("div");

  if (games.length === 0) {
    const empty = emptyState("No games yet", "Import a game to add it to your library.");
    const button = document.createElement("button");
    button.className = "small-btn";
    button.textContent = "Import game";
    button.style.marginTop = "14px";
    button.onclick = openImportSheet;
    empty.appendChild(button);
    content.appendChild(empty);
  } else {
    const list = document.createElement("div");
    list.className = "sec";
    games.forEach((game) => list.appendChild(gameRow(game)));
    content.appendChild(list);
  }

  const total = games.reduce((sum, g) => sum + g.size_bytes, 0);
  const subtitle =
    games.length === 0
      ? "0 games"
      : `${games.length} ${games.length === 1 ? "game" : "games"}${total > 0 ? ` · ${formatSize(total)}` : ""}`;

  return { title: "Library", subtitle, content };
}
