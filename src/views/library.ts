import { emptyState, type View } from "./view";

export function renderLibrary(): View {
  return {
    title: "Library",
    subtitle: "0 games",
    content: emptyState("No games yet", "Import a game to add it to your library."),
  };
}
