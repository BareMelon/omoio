import { emptyState, type View } from "./view";

export function renderCatalogue(): View {
  return {
    title: "Catalogue",
    subtitle: "Not built yet",
    content: emptyState("Catalogue isn't built yet", "Browse compatibility and patches for any PS3 title."),
  };
}
