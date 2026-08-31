import type { Game, Playing } from "./api";

export type ViewId =
  | "library"
  | "catalogue"
  | "homebrew"
  | "updates"
  | "system"
  | "logs"
  | "settings";

interface AppState {
  view: ViewId;
  rpcs3Version: string | null;
  firmwareVersion: string | null;
  games: Game[];
  search: string;
  notice: string | null;
  playing: Playing | null;
  gameFullscreen: boolean;
  selected: string | null;
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
    search: "",
    notice: null,
    playing: null,
    gameFullscreen: false,
    selected: null,
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

  setSearch(search: string): void {
    if (search === this.state.search) return;
    this.state = { ...this.state, search };
    this.notify();
  }

  setNotice(notice: string | null): void {
    this.state = { ...this.state, notice };
    this.notify();
  }

  setPlaying(playing: Playing | null): void {
    this.state = { ...this.state, playing };
    this.notify();
  }

  setGameFullscreen(gameFullscreen: boolean): void {
    this.state = { ...this.state, gameFullscreen };
    this.notify();
  }

  setSelected(selected: string | null): void {
    this.state = { ...this.state, selected };
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
