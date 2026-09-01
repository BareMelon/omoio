import {
  cancelUpdate,
  gameUpdates,
  installUpdate,
  listGames,
  onUpdateProgress,
  type GameUpdate,
  type ImportProgress,
} from "../api";
import { store } from "../state";

function formatSize(bytes: number): string {
  const gb = bytes / 1024 ** 3;
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${Math.round(bytes / 1024 ** 2)} MB`;
}

export async function openUpdates(
  titleId: string,
  title: string,
  installed: string | null,
  onChanged: () => void
): Promise<void> {
  const scrim = document.createElement("div");
  scrim.className = "scrim";
  const sheet = document.createElement("div");
  sheet.className = "sheet wide";
  scrim.appendChild(sheet);
  document.body.appendChild(scrim);
  requestAnimationFrame(() => scrim.classList.add("on"));

  let busy = false;
  function close() {
    if (busy) return;
    scrim.classList.remove("on");
    setTimeout(() => scrim.remove(), 200);
  }
  scrim.onclick = (e) => {
    if (e.target === scrim) close();
  };

  function showProgress(update: GameUpdate, progress: ImportProgress) {
    const pct =
      progress.total > 0 ? Math.min(100, Math.round((progress.bytes / progress.total) * 100)) : 0;
    sheet.innerHTML = `
      <div class="sheet-h">Installing ${update.version}</div>
      <div class="progress-row" style="margin-top:14px">
        <div class="progress-label">
          <span>Downloading ${formatSize(progress.bytes)} of ${formatSize(progress.total)}</span>
          <span class="pct">${pct}%</span>
        </div>
        <div class="progress"><div class="progress-fill" style="width:${pct}%"></div></div>
      </div>
      <div class="sheet-actions">
        <button class="btn ghost" id="cancel-update">Cancel</button>
      </div>
    `;
    sheet.querySelector<HTMLButtonElement>("#cancel-update")!.onclick = () => {
      cancelUpdate();
    };
  }

  async function run(update: GameUpdate) {
    busy = true;
    showProgress(update, { stage: "unpacking", bytes: 0, total: update.size });
    const unlisten = await onUpdateProgress((progress) => showProgress(update, progress));
    try {
      await installUpdate(titleId, update);
      store.setGames(await listGames());
      busy = false;
      onChanged();
      await show(`Version ${update.version} installed.`);
    } catch (err) {
      busy = false;
      const message =
        err === "cancelled"
          ? "Stopped. Nothing was installed."
          : typeof err === "string"
            ? err
            : "Couldn't install that update.";
      await show(message);
    } finally {
      unlisten();
    }
  }

  async function show(notice?: string) {
    sheet.innerHTML = `
      <div class="sheet-h">Official updates</div>
      <div class="sheet-p"></div>
      ${notice ? `<div class="notice" style="margin-top:14px"></div>` : ""}
      <div class="settings-scroll" id="update-list">
        <div class="sheet-p">Checking with Sony…</div>
      </div>
      <div class="sheet-foot">
        <span class="cfg-v" id="update-count"></span>
        <div class="sheet-actions"><button class="btn solid" id="updates-done">Done</button></div>
      </div>
    `;
    sheet.querySelector<HTMLElement>(".sheet-p")!.textContent = installed
      ? `${title} is on version ${installed}.`
      : `${title} has no update installed.`;
    if (notice) sheet.querySelector<HTMLElement>(".notice")!.textContent = notice;
    sheet.querySelector<HTMLButtonElement>("#updates-done")!.onclick = close;

    const list = sheet.querySelector<HTMLElement>("#update-list")!;
    const count = sheet.querySelector<HTMLElement>("#update-count")!;

    let updates: GameUpdate[];
    try {
      updates = await gameUpdates(titleId);
    } catch (err) {
      list.textContent = "";
      const failed = document.createElement("div");
      failed.className = "sheet-p";
      failed.textContent =
        typeof err === "string" ? err : "Couldn't reach Sony's update service.";
      list.appendChild(failed);
      return;
    }

    list.textContent = "";
    if (updates.length === 0) {
      const none = document.createElement("div");
      none.className = "sheet-p";
      none.textContent = "Sony never published an update for this game.";
      list.appendChild(none);
      count.textContent = "None published";
      return;
    }
    count.textContent = `${updates.length} published`;

    // Going back is possible, but a save written by a newer version may not
    // load on an older one, and that part cannot be undone. Said once, above
    // the list, rather than on every row.
    const warning = document.createElement("div");
    warning.className = "notice";
    warning.style.marginBottom = "14px";
    warning.textContent =
      "You can install an older version later, but a saved game made on a newer one may not load on it. Back up your saves before going back.";
    list.appendChild(warning);

    for (const update of updates) {
      const row = document.createElement("div");
      row.className = update.version === installed ? "setting changed" : "setting";

      const left = document.createElement("div");
      const name = document.createElement("div");
      name.className = "setting-k";
      name.textContent = `Version ${update.version}`;
      const detail = document.createElement("div");
      detail.className = "setting-path";
      detail.textContent = `${formatSize(update.size)} · needs firmware ${update.firmware}`;
      left.append(name, detail);

      const right = document.createElement("div");
      right.className = "row-actions";
      if (update.version === installed) {
        const mark = document.createElement("span");
        mark.className = "status go";
        mark.textContent = "Installed";
        right.appendChild(mark);
      } else {
        const install = document.createElement("button");
        install.className = "small-btn";
        install.textContent = "Install";
        install.onclick = () => run(update);
        right.appendChild(install);
      }

      row.append(left, right);
      list.appendChild(row);
    }
  }

  await show();
}
