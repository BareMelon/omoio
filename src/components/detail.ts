import { convertFileSrc } from "@tauri-apps/api/core";
import {
  addToLibrary,
  figurePictures,
  gameCompatibility,
  gamePatches,
  gameSaves,
  gameSettings,
  gameUpdates,
  launchGame,
  listGames,
  padsHeld,
  portalButton,
  refreshCompatibility,
  getFigurePictures,
  onFigurePictures,
  removeGame,
  setPortalButton,
  stopFigurePictures,
  type Game,
} from "../api";
import { placeholderArt } from "./art";
import { nameOf } from "./padNames";
import type { CatalogueSelection } from "../state";
import { openGameSettings } from "./gameSettingsSheet";
import { openImportSheet } from "./importSheet";
import { rawgCredit } from "./rawgCredit";
import { knownCover } from "./catalogueCovers";
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
    : placeholderArt(game.title_id, game.title);
  if (game.cover_source === "rawg") hero.appendChild(rawgCredit());

  body.innerHTML = `
    <div class="d-title"></div>
    <div class="d-sub"></div>
    <button class="play" id="detail-play">
      <svg viewBox="0 0 12 14" fill="currentColor"><path d="M1 1l10 6-10 6z"/></svg>Play
    </button>
    <div class="note" id="detail-note"></div>
    <div class="sec" id="sec-details">
      <div class="sec-h">Details</div>
      ${row("Version", game.version ?? (game.console === "wiiu" ? "Known after first play" : "unknown"))}
      ${row("Size on disk", formatSize(game.size_bytes))}
      ${row(
        "Files",
        game.available ? "Available" : "Not found",
        game.available ? "" : "warn"
      )}
    </div>
    <div class="sec" id="sec-version">
      <div class="sec-h">Game version</div>
      <button class="small-btn wide" id="detail-update">Check for updates</button>
      <div class="note plain" id="detail-update-note"></div>
      <button class="link-btn gone" id="detail-update-more">Choose another version</button>
    </div>
    <div class="sec" id="sec-saves">
      <div class="sec-h">Saved games</div>
      <button class="small-btn wide" id="detail-saves">Back up and restore</button>
      <div class="note plain" id="detail-saves-note"></div>
    </div>
    <div class="sec" id="sec-compat">
      <div class="sec-h">How well it runs</div>
      <div class="compat" id="detail-compat">
        <span class="status" id="detail-compat-badge"></span>
        <button class="link-btn" id="detail-compat-get"></button>
      </div>
      <div class="note plain" id="detail-compat-note"></div>
    </div>
    <div class="sec" id="sec-emulator">
      <div class="sec-h">Emulator</div>
      <button class="small-btn wide" id="detail-settings">Change settings</button>
      <div class="note plain" id="detail-settings-note"></div>
      <button class="small-btn wide" id="detail-patches">Patches</button>
      <div class="note plain" id="detail-patches-note"></div>
    </div>
    <div class="sec gone" id="sec-portal">
      <div class="sec-h">Skylanders</div>
      <div class="portal-key">
        <span class="row-k">Keybind Skylander emulator</span>
        <button class="help-dot" id="detail-portal-help" aria-label="How the Skylander emulator works" aria-expanded="false">?</button>
        <button class="small-btn" id="detail-portal-button"></button>
      </div>
      <div class="note plain gone" id="detail-portal-about"></div>
      <div class="sec-h sec-sub">Advanced</div>
      <div class="portal-key">
        <span class="row-k">Figure pictures</span>
        <button class="small-btn" id="detail-pictures"></button>
      </div>
      <div class="progress-row gone" id="detail-pictures-bar">
        <div class="progress-label"><span>Reading the pictures from your game…</span><span class="pct"></span></div>
        <div class="progress"><div class="progress-fill" style="width:0%"></div></div>
      </div>
      <div class="note plain" id="detail-pictures-note"></div>
    </div>
    <div class="sec" id="sec-location">
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
  if (!game.set_up) {
    // Noted from the catalogue and never imported. It says what to do and
    // gives you the way to do it, rather than reporting a fault.
    note.textContent = "Import this game's files to play it.";
    const importIt = document.createElement("button");
    importIt.className = "small-btn wide";
    importIt.textContent = "Import game";
    importIt.onclick = openImportSheet;
    note.after(importIt);

    // Nothing about a size, a version, saves or the emulator means anything
    // until the files are here. How well it runs still does: it says whether
    // this is worth setting up at all. Location keeps its Remove button, since
    // taking the note back off the list has to stay possible.
    for (const id of ["details", "version", "saves", "emulator"]) {
      body.querySelector(`#sec-${id}`)?.classList.add("gone");
    }
    body.querySelector(".d-path")!.classList.add("gone");
  }
  // Only what this game's emulator can do. A section with nothing behind it
  // is left out rather than shown as a button that does nothing.
  const offers = game.features;
  if (!offers.updates) body.querySelector("#sec-version")?.classList.add("gone");
  if (!offers.saves) body.querySelector("#sec-saves")?.classList.add("gone");
  if (!offers.compatibility) body.querySelector("#sec-compat")?.classList.add("gone");
  for (const [on, id] of [
    [offers.settings, "settings"],
    [offers.patches, "patches"],
  ] as const) {
    if (!on) {
      body.querySelector(`#detail-${id}`)?.classList.add("gone");
      body.querySelector(`#detail-${id}-note`)?.classList.add("gone");
    }
  }
  if (!offers.settings && !offers.patches) body.querySelector("#sec-emulator")?.classList.add("gone");

  if (game.set_up && !game.available) {
    note.textContent = "Reconnect the drive this game is on to play it.";
  }

  // The button that opens the portal menu over a Skylanders game, shown only
  // where the game's emulator lets Omoio fill the portal.
  if (offers.portal && game.set_up && /skylanders/i.test(game.title)) {
    body.querySelector("#sec-portal")!.classList.remove("gone");
    const keyButton = body.querySelector<HTMLButtonElement>("#detail-portal-button")!;
    const help = body.querySelector<HTMLButtonElement>("#detail-portal-help")!;
    const about = body.querySelector<HTMLElement>("#detail-portal-about")!;
    about.textContent =
      "Skylanders games need a toy portal, and the emulator pretends one is plugged in. While playing, press this button to open the portal menu over the game. Pick any character under its element, or an item or adventure pack, and it goes on the portal and is saved with its progress. Several can be on at once. It all works with the pad. The home button is the best choice, since games don't use it. If Windows' Game Bar opens instead, switch off its controller button in Windows Settings, under Gaming, Xbox Game Bar.";
    help.onclick = () => {
      const open = !about.classList.toggle("gone");
      help.setAttribute("aria-expanded", String(open));
    };

    const label = (place: string) => (place === "Guide" ? "Home button" : nameOf("generic", place));
    const showKey = async () => {
      keyButton.textContent = label(await portalButton());
    };
    void showKey();

    // Waits a few seconds for a press on any pad. A stick pushed a little
    // is not a press, so a pad resting off centre never records.
    keyButton.onclick = async () => {
      keyButton.disabled = true;
      keyButton.textContent = "Press a button…";
      const down = new Set(await padsHeld());
      const until = Date.now() + 6000;
      let pressed: string | undefined;
      while (!pressed && Date.now() < until) {
        await new Promise((resolve) => setTimeout(resolve, 60));
        const now = await padsHeld();
        pressed = now.find((input) => !down.has(input) && !/^(LS|RS) [XY][+-]$/.test(input));
        for (const input of [...down]) if (!now.includes(input)) down.delete(input);
      }
      if (pressed) {
        try {
          await setPortalButton(pressed);
        } catch (err) {
          note.textContent = typeof err === "string" ? err : "Couldn't save that button.";
        }
      }
      keyButton.disabled = false;
      await showKey();
    };

    // Each figure's own picture, read out of the user's copy of the game by
    // a small program Omoio fetches the first time. Under Advanced, since
    // the menu works without them.
    const pictures = body.querySelector<HTMLButtonElement>("#detail-pictures")!;
    const picturesBar = body.querySelector<HTMLElement>("#detail-pictures-bar")!;
    const picturesFill = picturesBar.querySelector<HTMLElement>(".progress-fill")!;
    const picturesPct = picturesBar.querySelector<HTMLElement>(".pct")!;
    const picturesNote = body.querySelector<HTMLElement>("#detail-pictures-note")!;
    const showPictures = (count: number) => {
      pictures.textContent = count > 0 ? "Get them again" : "Get pictures";
      picturesNote.textContent =
        count > 0
          ? `${count} pictures from your copy of the game. The portal menu shows them.`
          : "Show each figure's own picture in the portal menu, read from your copy of the game. Takes under a minute.";
    };
    const countPictures = () =>
      figurePictures(game.title_id)
        .then((found) => found.names.length)
        .catch(() => 0);
    void countPictures().then(showPictures);

    let reading = false;
    pictures.onclick = async () => {
      if (reading) {
        await stopFigurePictures();
        return;
      }
      reading = true;
      pictures.textContent = "Stop";
      picturesNote.textContent = "";
      picturesFill.style.width = "0%";
      picturesPct.textContent = "";
      picturesBar.classList.remove("gone");
      const unlisten = onFigurePictures((progress) => {
        if (progress.title_id !== game.title_id || progress.of === 0) return;
        const done = Math.round((progress.done / progress.of) * 100);
        picturesFill.style.width = `${done}%`;
        picturesPct.textContent = `${done}%`;
      });
      try {
        showPictures(await getFigurePictures(game.title_id));
      } catch (err) {
        showPictures(await countPictures());
        picturesNote.textContent = typeof err === "string" ? err : "Couldn't read the pictures. Try again.";
      } finally {
        reading = false;
        picturesBar.classList.add("gone");
        void unlisten.then((stopListening) => stopListening());
      }
    };
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
  if (offers.saves) showSaves();
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
      await refreshCompatibility(game.console);
      await showCompat();
    } catch (err) {
      compatNote.textContent =
        typeof err === "string" ? err : "Couldn't get the compatibility list.";
    } finally {
      getList.disabled = false;
    }
  };
  if (offers.compatibility) showCompat();

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
  if (offers.patches) showPatchCount();
  body.querySelector<HTMLButtonElement>("#detail-patches")!.onclick = () =>
    openPatches(game.title_id, game.title, showPatchCount);

  const settingsNote = body.querySelector<HTMLElement>("#detail-settings-note")!;
  const settingsButton = body.querySelector<HTMLButtonElement>("#detail-settings")!;
  const showSettingsCount = async () => {
    try {
      const { emulator, chosen } = await gameSettings(game.title_id);
      const changed = Object.keys(chosen).length;
      settingsButton.disabled = false;
      settingsNote.textContent =
        changed === 0
          ? `Running with ${emulator}'s own settings.`
          : `${changed} setting${changed === 1 ? "" : "s"} changed for this game.`;
    } catch (err) {
      // A Wii U disc image's settings are filed under an id only known once
      // the game has run, which the message says.
      settingsButton.disabled = true;
      settingsNote.textContent = typeof err === "string" ? err : "Couldn't read this game's settings.";
    }
  };
  if (offers.settings) showSettingsCount();
  settingsButton.onclick = () => openGameSettings(game.title_id, game.title, showSettingsCount);

  body.querySelector<HTMLButtonElement>("#detail-remove")!.onclick = async () => {
    await removeGame(game.title_id);
    store.setSelected(null);
    store.setGames(await listGames());
  };
}

/// A game from the catalogue: maybe owned, maybe never seen, so everything
/// here is about whether it is worth getting and what is known about it.
function fillListing(body: HTMLElement, hero: HTMLElement, { listing }: CatalogueSelection): void {
  const cover = knownCover(listing.key);
  if (cover) {
    hero.innerHTML = `<img src="${convertFileSrc(cover)}" alt="">`;
    hero.appendChild(rawgCredit());
  } else {
    hero.innerHTML = placeholderArt(listing.key, listing.name);
  }
  body.innerHTML = `
    <div class="d-title"></div>
    <div class="d-sub"></div>
    <button class="play" id="listing-add"></button>
    <div class="note" id="listing-note"></div>
    <div class="sec">
      <div class="sec-h">How well it runs</div>
      <div class="compat"><span class="status" id="listing-compat"></span></div>
      <div class="note plain" id="listing-compat-note"></div>
    </div>
    <div class="sec gone" id="listing-updates-sec">
      <div class="sec-h">Official updates</div>
      <button class="small-btn wide" id="listing-updates">Check for updates</button>
      <div class="note plain" id="listing-updates-note"></div>
    </div>
    <div class="sec gone" id="listing-patches-sec">
      <div class="sec-h">Community patches</div>
      <div class="note plain" id="listing-patches"></div>
    </div>
    <div class="sec gone" id="listing-releases">
      <div class="sec-h">Releases</div>
      <div id="listing-releases-rows"></div>
    </div>
  `;
  // A game's own name, so never through innerHTML.
  body.querySelector<HTMLElement>(".d-title")!.textContent = listing.name;
  body.querySelector<HTMLElement>(".d-sub")!.textContent = [
    listing.console_name,
    listing.regions.join(" "),
    listing.kind,
  ]
    .filter(Boolean)
    .join(" · ");

  const note = body.querySelector<HTMLElement>("#listing-note")!;
  const add = body.querySelector<HTMLButtonElement>("#listing-add")!;
  // The release a region filter picked, or else the first one listed.
  const release = listing.releases[0];
  if (listing.owned) {
    add.textContent = "In your library";
    add.disabled = true;
  } else if (release) {
    add.textContent = "Add to library";
    add.onclick = async () => {
      add.disabled = true;
      try {
        await addToLibrary(listing.console, release.title_id, listing.name);
        add.textContent = "In your library";
        note.textContent = "Import its files from the library to play it.";
        store.setGames(await listGames());
      } catch (err) {
        note.textContent = typeof err === "string" ? err : "Couldn't add that game.";
        add.disabled = false;
      }
    };
  } else {
    // Without a title id there is nothing to note it down by, so the way in is
    // its own files.
    add.textContent = "Import game";
    add.onclick = () => openImportSheet();
  }

  const badge = body.querySelector<HTMLElement>("#listing-compat")!;
  const compatNote = body.querySelector<HTMLElement>("#listing-compat-note")!;
  badge.textContent = listing.status.label || "No result";
  badge.className = `status ${listing.status.tone || "mute"}`;
  compatNote.textContent = listing.status.explanation || "Nobody has reported on this game yet.";
  // Some lists also say when a release's result was last reported.
  if (release && listing.features.compatibility) {
    void gameCompatibility(release.title_id).then((compat) => {
      if (compat.checked && compat.label === listing.status.label) {
        compatNote.textContent = `${listing.status.explanation} Last reported ${compat.checked}.`;
      }
    });
  }

  if (release && listing.features.updates) {
    body.querySelector("#listing-updates-sec")!.classList.remove("gone");
    // Asked only when pressed, the same as for a game in the library: it is a
    // request to Sony, and opening a title should not quietly make one.
    const updates = body.querySelector<HTMLButtonElement>("#listing-updates")!;
    const updatesNote = body.querySelector<HTMLElement>("#listing-updates-note")!;
    updates.onclick = async () => {
      updates.disabled = true;
      updates.textContent = "Checking…";
      try {
        const found = await gameUpdates(release.title_id);
        updatesNote.textContent =
          found.length === 0
            ? "Sony never published an update for this game."
            : found.length === 1
              ? `Sony published one update, version ${found[0].version}.`
              : `Sony published ${found.length} updates, up to version ${found[0].version}.`;
      } catch (err) {
        updatesNote.textContent = typeof err === "string" ? err : "Couldn't reach Sony's update service.";
      } finally {
        updates.disabled = false;
        updates.textContent = "Check for updates";
      }
    };
  }

  if (release && listing.features.patches) {
    body.querySelector("#listing-patches-sec")!.classList.remove("gone");
    const patchesNote = body.querySelector<HTMLElement>("#listing-patches")!;
    void gamePatches(release.title_id).then(({ have_list, patches }) => {
      patchesNote.textContent = !have_list
        ? "No patch list yet."
        : patches.length === 0
          ? "None published for this game."
          : patches.length === 1
            ? "One published for this game."
            : `${patches.length} published for this game.`;
    });
  }

  if (listing.releases.length > 0) {
    body.querySelector("#listing-releases")!.classList.remove("gone");
    const rows = body.querySelector<HTMLElement>("#listing-releases-rows")!;
    rows.innerHTML = listing.releases.map((r) => row(r.region || "Other", r.title_id)).join("");
  }
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

  detail.querySelector<HTMLButtonElement>(".d-close")!.onclick = () => {
    store.setSelected(null);
    store.setCatalogueSelected(null);
  };

  let shownListing: string | null = null;

  // The content makes room for the panel at once, and the panel slides over
  // the gap, so the grid is laid out once rather than on every frame.
  const show = (open: boolean) => {
    detail.classList.toggle("hidden", !open);
    detail.parentElement?.classList.toggle("with-detail", open);
  };

  store.subscribe((state) => {
    if (!state.playing && state.view === "catalogue" && state.catalogueSelected) {
      show(true);
      // Filled once per pick. Typing in the search box notifies too, and
      // refilling would ask for the same answers again on every letter.
      const picked = state.catalogueSelected.listing.key;
      if (picked !== shownListing) {
        shownListing = picked;
        fillListing(body, hero, state.catalogueSelected);
      }
      return;
    }
    shownListing = null;

    // This panel belongs to the library. It stays out of the way while a game
    // runs, since the picture covers this side of the window, and while any
    // other screen is up, where a game selected earlier is not what you are
    // looking at.
    const game =
      state.playing || state.view !== "library"
        ? undefined
        : state.games?.find((g) => g.title_id === state.selected);
    show(Boolean(game));
    if (game) fill(body, hero, game);
  });

  return detail;
}
