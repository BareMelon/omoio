import { open } from "@tauri-apps/plugin-dialog";
import { importGame, listGames } from "./api";
import { store } from "./state";

export async function startImport(): Promise<void> {
  const picked = await open({
    directory: true,
    multiple: false,
    title: "Choose a game folder",
  });
  if (typeof picked !== "string") return;

  // The result lands in the library, so show it before the work starts.
  store.setView("library");
  store.setImporting(true);
  try {
    await importGame(picked);
    store.setGames(await listGames());
    store.setImporting(false);
  } catch (err) {
    // The backend's messages are already written for a person to read.
    store.setImportError(typeof err === "string" ? err : "Couldn't import that folder.");
  }
}
