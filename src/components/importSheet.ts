import { open } from "@tauri-apps/plugin-dialog";
import {
  cancelImport,
  droppedKind,
  getGamesFolder,
  importArchive,
  importGame,
  listGames,
  onImportProgress,
  onScanProgress,
  scanFolder,
  setGamesFolder,
  type ImportProgress,
  type ScanProgress,
  type ScanResult,
} from "../api";
import { store } from "../state";

function formatGB(bytes: number): string {
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

export function openImportSheet(): void {
  sheet();
}

/// Paths dropped on the window skip the choices: the user has already said what
/// they want imported.
///
/// Kept separate from `openImportSheet` so neither can be wired to `onclick`
/// by mistake, which would hand a PointerEvent in as a list of paths.
export function importDropped(paths: string[]): void {
  sheet(paths);
}

function sheet(dropped?: string[]): void {
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
      <div class="sheet-alt">
        <button class="link-btn" id="pick-scan">Scan a folder for games</button>
      </div>
    `;
    sheet.querySelector<HTMLButtonElement>("#pick-folder")!.onclick = pickFolder;
    sheet.querySelector<HTMLButtonElement>("#pick-archive")!.onclick = pickArchive;
    sheet.querySelector<HTMLButtonElement>("#pick-scan")!.onclick = pickScan;
  }

  /// `counter` is set only when more than one thing was dropped, so a single
  /// import does not get a needless "1 of 1".
  function showProgress(progress: ImportProgress, counter = "") {
    const pct =
      progress.stage === "unpacking" && progress.total > 0
        ? Math.min(100, Math.round((progress.bytes / progress.total) * 100))
        : null;
    const label =
      progress.stage === "identifying"
        ? "Reading the game…"
        : `Unpacking… ${formatGB(progress.bytes)} of ${formatGB(progress.total)}`;
    sheet.innerHTML = `
      <div class="sheet-h">Importing${counter ? ` ${counter}` : ""}</div>
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

  /// Unpacking needs somewhere to put 19 GB, so that is settled before an
  /// archive starts rather than after the wait. False when the user backed out.
  async function haveGamesFolder(): Promise<boolean> {
    if (await getGamesFolder()) return true;
    const chosen = await open({
      directory: true,
      multiple: false,
      title: "Choose where to keep your games",
    });
    if (typeof chosen !== "string") return false;
    await setGamesFolder(chosen);
    return true;
  }

  async function pickArchive() {
    const picked = await open({
      multiple: false,
      directory: false,
      title: "Choose a game archive",
      filters: [{ name: "Game archive", extensions: ["7z", "zip"] }],
    });
    if (typeof picked !== "string") return;
    if (!(await haveGamesFolder())) return;

    await run(() => importArchive(picked));
  }

  function showScanProgress(progress: ScanProgress) {
    // Finding the games and reading them are two different waits. Counting
    // folders found says something is happening before a total is knowable.
    const label =
      progress.stage === "looking"
        ? `Looking for games… ${progress.done} found`
        : `Reading ${progress.title}`;
    const pct =
      progress.stage === "reading" && progress.total > 0
        ? Math.min(100, Math.round((progress.done / progress.total) * 100))
        : null;
    sheet.innerHTML = `
      <div class="sheet-h">Scanning</div>
      <div class="progress-row" style="margin-top:14px">
        <div class="progress-label">
          <span class="scan-label"></span>
          ${pct !== null ? `<span class="pct">${progress.done} of ${progress.total}</span>` : ""}
        </div>
        <div class="progress">
          <div class="progress-fill${pct === null ? " indeterminate" : ""}"${
            pct !== null ? ` style="width:${pct}%"` : ""
          }></div>
        </div>
      </div>
      <div class="sheet-actions">
        <button class="btn ghost" id="cancel-scan">Cancel</button>
      </div>
    `;
    // A game's own title, so never through innerHTML.
    sheet.querySelector<HTMLElement>(".scan-label")!.textContent = label;
    sheet.querySelector<HTMLButtonElement>("#cancel-scan")!.onclick = () => {
      cancelImport();
    };
  }

  function showScanResult(result: ScanResult) {
    busy = false;
    const lines = [
      result.added === 0
        ? ""
        : result.added === 1
          ? "1 game added."
          : `${result.added} games added.`,
      result.already_there > 0 ? `${result.already_there} already in your library.` : "",
      result.cancelled ? "Stopped early." : "",
    ].filter(Boolean);
    sheet.innerHTML = `
      <div class="sheet-h">${result.added > 0 ? "Games added" : "Nothing new"}</div>
      <div class="sheet-p">${
        result.added === 0 && result.already_there === 0 && !result.cancelled
          ? "No games were found in that folder."
          : lines.join(" ")
      }</div>
      <div class="sheet-actions">
        <button class="btn solid" id="scan-done">Done</button>
      </div>
    `;
    sheet.querySelector<HTMLButtonElement>("#scan-done")!.onclick = close;
  }

  async function pickScan() {
    const picked = await open({
      directory: true,
      multiple: false,
      title: "Choose a folder to scan",
    });
    if (typeof picked !== "string") return;

    busy = true;
    showScanProgress({ stage: "looking", done: 0, total: 0, title: "" });
    const unlisten = await onScanProgress(showScanProgress);
    try {
      const result = await scanFolder(picked);
      store.setGames(await listGames());
      showScanResult(result);
    } catch (err) {
      busy = false;
      showChoices(typeof err === "string" ? err : "Couldn't scan that folder.");
    } finally {
      unlisten();
    }
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

  /// Imports what was dropped, one at a time so a failure part way through
  /// still leaves everything before it in the library.
  async function runDropped(paths: string[]) {
    busy = true;
    const counter = (at: number) => (paths.length > 1 ? `${at + 1} of ${paths.length}` : "");
    let added = 0;
    let problem = "";

    for (const [at, path] of paths.entries()) {
      // Anything that is not an archive goes to the emulators to read, so a
      // file they cannot take gets their reason rather than a general one.
      const kind = await droppedKind(path);
      if (kind === "archive" && !(await haveGamesFolder())) break;

      showProgress({ stage: "unpacking", bytes: 0, total: 0 }, counter(at));
      const unlisten = await onImportProgress((progress) => showProgress(progress, counter(at)));
      try {
        await (kind === "archive" ? importArchive(path) : importGame(path));
        added += 1;
      } catch (err) {
        // Cancelling stops the run rather than moving to the next one: the
        // Cancel button means this, not this one.
        if (err === "cancelled") break;
        problem ||= typeof err === "string" ? err : "Couldn't import that game.";
      } finally {
        unlisten();
      }
    }

    store.setGames(await listGames());
    busy = false;
    if (added > 0 && !problem) {
      close();
      return;
    }
    showChoices(problem || "Nothing was imported.");
  }

  if (dropped && dropped.length > 0) {
    void runDropped(dropped);
  } else {
    showChoices();
  }
}
