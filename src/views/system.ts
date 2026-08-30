import { emptyState, type View } from "./view";

export function renderSystem(): View {
  return {
    title: "System",
    subtitle: "Not checked yet",
    content: emptyState("Hardware hasn't been checked yet", "Detection runs the first time you set up RPCS3."),
  };
}
