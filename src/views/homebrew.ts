import { emptyState, type View } from "./view";

export function renderHomebrew(): View {
  return {
    title: "Homebrew",
    subtitle: "Not built yet",
    content: emptyState("Homebrew isn't built yet", "Install freely distributable PS3 software in one step."),
  };
}
