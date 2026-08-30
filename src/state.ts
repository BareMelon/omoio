export type ViewId = "library" | "catalogue" | "homebrew" | "updates" | "system";

interface AppState {
  view: ViewId;
}

type Listener = (state: AppState) => void;

// One small observable store for the whole app. No external state library:
// views subscribe and re-render themselves when the view changes. This is
// enough for the ~15 components in the app; revisit only if that stops
// being true.
class Store {
  private state: AppState = { view: "library" };
  private listeners = new Set<Listener>();

  get(): AppState {
    return this.state;
  }

  setView(view: ViewId): void {
    if (view === this.state.view) return;
    this.state = { ...this.state, view };
    this.notify();
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    listener(this.state);
    return () => this.listeners.delete(listener);
  }

  private notify(): void {
    for (const listener of this.listeners) listener(this.state);
  }
}

export const store = new Store();
