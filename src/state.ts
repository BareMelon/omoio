import type { Game } from "./api";

export type ViewId = "library" | "catalogue" | "homebrew" | "updates" | "system";

interface AppState {
  view: ViewId;
  rpcs3Version: string | null;
  firmwareVersion: string | null;
  games: Game[];
}

type Listener = (state: AppState) => void;

// One small observable store for the whole app. No external state library:
// views subscribe and re-render themselves when the view changes. This is
// enough for the ~15 components in the app; revisit only if that stops
// being true.
class Store {
  private state: AppState = {
    view: "library",
    rpcs3Version: null,
    firmwareVersion: null,
    games: [],
  };
  private listeners = new Set<Listener>();

  get(): AppState {
    return this.state;
  }

  setView(view: ViewId): void {
    if (view === this.state.view) return;
    this.state = { ...this.state, view };
    this.notify();
  }

  setRpcs3Version(version: string | null): void {
    this.state = { ...this.state, rpcs3Version: version };
    this.notify();
  }

  setFirmwareVersion(version: string | null): void {
    this.state = { ...this.state, firmwareVersion: version };
    this.notify();
  }

  setGames(games: Game[]): void {
    this.state = { ...this.state, games };
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
