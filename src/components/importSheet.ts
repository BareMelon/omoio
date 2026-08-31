import { open } from "@tauri-apps/plugin-dialog";
import {
  cancelImport,
  getGamesFolder,
  importArchive,
  importGame,
  listGames,
  onImportProgress,
  setGamesFolder,
  type ImportProgress,
} from "../api";
import { store } from "../state";

function formatGB(bytes: number): string {
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

export function openImportSheet(): void {
  const scrim = document.createElement("div");
  scrim.className = "scrim";
  const sheet = document.createElement("div");
  sheet.className = "sheet";
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
  document.addEventListener("keydown", function onKey(e) {
    if (e.key === "Escape" && !busy) {
      document.removeEventListener("keydown", onKey);
      close();
    }
  });

  function showChoices(note?: string) {
    busy = false;
    sheet.innerHTML = `
      <div class="sheet-h">Import a game</div>
      <div class="sheet-p">Point Omoio at a folder you've already unpacked, or at a .7z or .zip archive.</div>
      ${note ? `<div class="notice" style="margin-top:16px">${note}</div>` : ""}
      <div class="sheet-actions">
        <button class="btn ghost" id="pick-folder">Choose a folder</button>
        <button class="btn solid" id="pick-archive">Choose an archive</button>
      </div>
    `;
    sheet.querySelector<HTMLButtonElement>("#pick-folder")!.onclick = pickFolder;
    sheet.querySelector<HTMLButtonElement>("#pick-archive")!.onclick = pickArchive;
  }

  function showProgress(progress: ImportProgress) {
    const pct =
      progress.stage === "unpacking" && progress.total > 0
        ? Math.min(100, Math.round((progress.bytes / progress.total) * 100))
        : null;
    const label =
      progress.stage === "identifying"
        ? "Reading the game…"
        : `Unpacking… ${formatGB(progress.bytes)} of ${formatGB(progress.total)}`;
    sheet.innerHTML = `
      <div class="sheet-h">Importing</div>
      <div class="progress-row" style="margin-top:14px">
        <div class="progress-label">
          <span>${label}</span>
          ${pct !== null ? `<span class="pct">${pct}%</span>` : ""}
        </div>
        <div class="progress">
          <div class="progress-fill${pct === null ? " indeterminate" : ""}"${
            pct !== null ? ` style="width:${pct}%"` : ""
          }></div>
        </div>
      </div>
      <div class="sheet-actions">
        <button class="btn ghost" id="cancel-import">Cancel</button>
      </div>
    `;
    sheet.querySelector<HTMLButtonElement>("#cancel-import")!.onclick = () => {
      cancelImport();
    };
  }

  async function pickFolder() {
    const picked = await open({ directory: true, multiple: false, title: "Choose a game folder" });
    if (typeof picked !== "string") return;
    await run(() => importGame(picked));
  }

  async function pickArchive() {
    const picked = await open({
      multiple: false,
      directory: false,
      title: "Choose a game archive",
      filters: [{ name: "Game archive", extensions: ["7z", "zip"] }],
    });
    if (typeof picked !== "string") return;

    // Unpacking needs somewhere to put 19 GB, so settle that before starting.
    let folder = await getGamesFolder();
    if (!folder) {
      const chosen = await open({
        directory: true,
        multiple: false,
        title: "Choose where to keep your games",
      });
      if (typeof chosen !== "string") return;
      await setGamesFolder(chosen);
      folder = chosen;
    }

    await run(() => importArchive(picked));
  }

  async function run(job: () => Promise<unknown>) {
    busy = true;
    showProgress({ stage: "unpacking", bytes: 0, total: 0 });
    const unlisten = await onImportProgress(showProgress);
    try {
      await job();
      store.setGames(await listGames());
      busy = false;
      close();
    } catch (err) {
      busy = false;
      if (err === "cancelled") {
        showChoices();
      } else {
        showChoices(typeof err === "string" ? err : "Couldn't import that game.");
      }
    } finally {
      unlisten();
    }
  }

  showChoices();
}
