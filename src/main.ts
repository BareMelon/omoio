import "./styles/tokens.css";
import "./styles/app.css";

import {
  getFirmwareVersion,
  getRpcs3Version,
  listGames,
  onGameFullscreen,
  onGameStarted,
  onGameStopped,
  playingGame,
} from "./api";
import { renderPlayingBar } from "./components/playingBar";
import { store, type ViewId } from "./state";
import { renderTitlebar } from "./components/titlebar";
import { renderSidebar } from "./components/sidebar";
import { renderDetail } from "./components/detail";
import { renderLibrary } from "./views/library";
import { renderCatalogue } from "./views/catalogue";
import { renderHomebrew } from "./views/homebrew";
import { renderUpdates } from "./views/updates";
import { renderSystem } from "./views/system";
import { renderLogs } from "./views/logs";
import { renderSettings } from "./views/settings";
import type { View } from "./views/view";

const VIEWS: Record<ViewId, () => View | Promise<View>> = {
  library: renderLibrary,
  catalogue: renderCatalogue,
  homebrew: renderHomebrew,
  updates: renderUpdates,
  system: renderSystem,
  logs: renderLogs,
  settings: renderSettings,
};

const app = document.getElementById("app")!;

const topbar = document.createElement("div");
topbar.className = "topbar";
topbar.innerHTML = `
  <div>
    <div class="view-title"></div>
    <div class="view-sub"></div>
  </div>
  <div class="search">
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5L14 14"/></svg>
    <input type="text" placeholder="Search your games…" />
  </div>
`;
const viewTitle = topbar.querySelector<HTMLElement>(".view-title")!;
const viewSub = topbar.querySelector<HTMLElement>(".view-sub")!;

const searchBox = topbar.querySelector<HTMLInputElement>(".search input")!;
searchBox.oninput = () => store.setSearch(searchBox.value);

const content = document.createElement("div");
content.className = "content";

const main = document.createElement("main");
main.className = "main";
main.append(topbar, content);

const shell = document.createElement("div");
shell.className = "shell";
shell.append(renderSidebar(), main, renderDetail());

app.append(renderTitlebar(), shell);

const topbarNormal = [...topbar.children];

let renderToken = 0;
store.subscribe((state) => {
  // While a game runs, its picture covers the content area, so the top bar
  // becomes the controls for it and the view underneath is left alone.
  if (state.playing) {
    topbar.replaceChildren(renderPlayingBar(state.playing));
    content.replaceChildren();
    return;
  }
  topbar.replaceChildren(...topbarNormal);

  const token = ++renderToken;
  Promise.resolve(VIEWS[state.view]()).then((view) => {
    if (token !== renderToken || store.get().playing) return;
    viewTitle.textContent = view.title;
    viewSub.textContent = view.subtitle;
    content.replaceChildren(view.content);
  });
});

getRpcs3Version().then((version) => store.setRpcs3Version(version));
getFirmwareVersion().then((version) => store.setFirmwareVersion(version));
listGames().then((games) => store.setGames(games));
playingGame().then((playing) => store.setPlaying(playing));

onGameStarted((playing) => store.setPlaying(playing));
onGameFullscreen((on) => store.setGameFullscreen(on));
onGameStopped(() => {
  store.setGameFullscreen(false);
  store.setPlaying(null);
});
