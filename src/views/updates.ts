import { emptyState, type View } from "./view";

export function renderUpdates(): View {
  return {
    title: "Updates",
    subtitle: "Nothing to check",
    content: emptyState("No updates to check", "Install RPCS3 and add a game to see updates here."),
  };
}
