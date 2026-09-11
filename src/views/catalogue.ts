import { convertFileSrc } from "@tauri-apps/api/core";
import {
  addToLibrary,
  cancelCompatibility,
  catalogue,
  getSettings,
  listGames,
  onCompatProgress,
  refreshCompatibility,
  type Listing,
} from "../api";
import { placeholderArt } from "../components/art";
import { coverFor, knownCover } from "../components/catalogueCovers";
import { rawgCredit } from "../components/rawgCredit";
import { store } from "../state";
import { emptyState, type View } from "./view";

const TONE: Record<string, string> = {
  Playable: "go",
  Ingame: "warn",
  Intro: "warn",
  Loadable: "bad",
  Nothing: "bad",
};

function card(
  listing: Listing,
  already: boolean,
  siblings: Listing[],
  covers: boolean,
  onAdded: () => void
): HTMLElement {
  // A div rather than a button: it holds the Add button, and a button cannot
  // hold another. Role and keys make it act like one.
  const card = document.createElement("div");
  card.className = "card";
  card.tabIndex = 0;
  card.setAttribute("role", "button");
  const open = () => store.setCatalogueSelected({ listing, siblings });
  card.onclick = open;
  card.onkeydown = (event) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      open();
    }
  };

  const tone = TONE[listing.status] ?? "";
  card.innerHTML = `
    <div class="art">
      ${placeholderArt(listing.title_id, listing.name)}
      <span class="art-name"></span>
    </div>
    <div class="meta">
      <span class="id">${listing.title_id}</span>
      ${listing.region ? `<span class="region">${listing.region}</span>` : ""}
      ${listing.status ? `<span class="status ${tone} tiny">${listing.status}</span>` : ""}
    </div>
  `;
  // A game's own name, so never through innerHTML.
  card.querySelector<HTMLElement>(".art-name")!.textContent = listing.name;

  if (covers) {
    // The same treatment as a library tile: the whole picture over a blurred
    // copy of itself, since RAWG's art is wide and the tile is not.
    const art = card.querySelector<HTMLElement>(".art")!;
    const show = (path: string) => {
      art.querySelector("svg")?.remove();
      const src = convertFileSrc(path);
      art.insertAdjacentHTML(
        "afterbegin",
        `<img class="art-back" src="${src}" alt="" aria-hidden="true"><img class="art-fit" src="${src}" alt="">`
      );
    };
    const known = knownCover(listing.title_id);
    if (known) {
      show(known);
    } else {
      // After this tick, once the view is on screen, so a tile that is about
      // to be replaced by the next keystroke is never asked about.
      setTimeout(() => {
        void coverFor(listing.title_id, listing.name, () => card.isConnected).then((path) => {
          if (path && card.isConnected) show(path);
        });
      }, 0);
    }
  }

  const action = document.createElement("button");
  action.className = "small-btn wide";
  if (already) {
    action.textContent = "In your library";
    action.disabled = true;
  } else {
    action.textContent = "Add to library";
    action.onclick = async (event) => {
      event.stopPropagation();
      action.disabled = true;
      try {
        await addToLibrary(listing.title_id, listing.name);
        action.textContent = "In your library";
        store.setGames(await listGames());
        onAdded();
      } catch {
        action.textContent = "Already there";
      }
    };
  }
  card.appendChild(action);

  return card;
}

/// The regions a title id can say it was sold in, plus everything. Console will
/// join this row when there is a second one to choose between.
const REGIONS: [string, string][] = [
  ["", "All"],
  ["EU", "EU"],
  ["US", "US"],
  ["JP", "JP"],
  ["Asia", "Asia"],
  ["KR", "KR"],
];

function regionFilter(current: string): HTMLElement {
  const row = document.createElement("div");
  row.className = "filters";
  for (const [value, label] of REGIONS) {
    const button = document.createElement("button");
    button.className = value === current ? "chip on" : "chip";
    button.textContent = label;
    button.onclick = () => store.setCatalogueRegion(value);
    row.appendChild(button);
  }
  return row;
}

/// The search box is the shared one in the top bar rather than one of this
/// screen's own. This whole view is rebuilt on every keystroke, and a field
/// rebuilt under the cursor loses focus and what was typed into it.
export async function renderCatalogue(): Promise<View> {
  const { catalogueQuery, catalogueRegion } = store.get();
  const query = catalogueQuery ?? "";
  const [view, settings] = await Promise.all([catalogue(query, catalogueRegion ?? ""), getSettings()]);
  const covers = settings.covers && Boolean(settings.rawg_key);

  if (!view.have_list) {
    const content = emptyState(
      "No list yet",
      "The catalogue is RPCS3's compatibility list. Get it once and it works offline."
    );
    const get = document.createElement("button");
    get.className = "small-btn";
    get.textContent = "Get the list";
    get.style.marginTop = "14px";

    // Names come a page at a time and take about twenty seconds, so this says
    // how far along it is and can be stopped.
    const bar = document.createElement("div");
    bar.className = "progress-row gone";
    bar.style.maxWidth = "360px";
    bar.style.margin = "16px auto 0";
    bar.innerHTML = `
      <div class="progress-label"><span>Getting the list…</span><span class="pct"></span></div>
      <div class="progress"><div class="progress-fill" style="width:0%"></div></div>
    `;
    const fill = bar.querySelector<HTMLElement>(".progress-fill")!;
    const pct = bar.querySelector<HTMLElement>(".pct")!;

    const stop = document.createElement("button");
    stop.className = "link-btn gone";
    stop.textContent = "Stop";
    stop.style.marginTop = "10px";
    stop.onclick = () => void cancelCompatibility();

    get.onclick = async () => {
      get.classList.add("gone");
      bar.classList.remove("gone");
      stop.classList.remove("gone");
      const unlisten = await onCompatProgress((progress) => {
        const done = Math.min(100, Math.round((progress.bytes / progress.total) * 100));
        fill.style.width = `${done}%`;
        pct.textContent = `${done}%`;
      });
      try {
        await refreshCompatibility();
        store.redraw();
      } catch {
        bar.classList.add("gone");
        stop.classList.add("gone");
        get.classList.remove("gone");
        get.textContent = "Couldn't get it";
      } finally {
        unlisten();
      }
    };
    content.append(get, bar, stop);
    return { title: "Catalogue", subtitle: "Nothing to browse yet", content };
  }

  const content = document.createElement("div");
  content.appendChild(regionFilter(catalogueRegion ?? ""));

  if (view.shown.length === 0) {
    const none = document.createElement("div");
    none.className = "sheet-p";
    none.style.marginTop = "18px";
    none.textContent = query
      ? "No game by that name."
      : "No games from that region in the list.";
    content.appendChild(none);
    return { title: "Catalogue", subtitle: "No matches", content };
  }

  const grid = document.createElement("div");
  grid.className = "grid";
  const owned = new Set(view.in_library);
  for (const listing of view.shown) {
    grid.appendChild(
      card(
        listing,
        owned.has(listing.title_id),
        // The same game's other regions among what is on screen.
        view.shown.filter((other) => other.name === listing.name && other.title_id !== listing.title_id),
        covers,
        () => store.redraw()
      )
    );
  }
  content.appendChild(grid);
  if (covers) content.appendChild(rawgCredit());

  const subtitle =
    view.total > view.shown.length
      ? `${view.shown.length} of ${view.total.toLocaleString()}`
      : `${view.total.toLocaleString()} ${view.total === 1 ? "game" : "games"}`;

  return { title: "Catalogue", subtitle, content };
}
