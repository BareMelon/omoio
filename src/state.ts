import type { Game, Listing, Playing } from "./api";

/// A catalogue title opened in the side panel, with the other regions' releases
/// of the same game that were on screen when it was picked.
export interface CatalogueSelection {
  listing: Listing;
  siblings: Listing[];
}

export type ViewId =
  | "library"
  | "catalogue"
  | "homebrew"
  | "controller"
  | "emulators"
  | "updates"
  | "system"
  | "logs"
  | "settings";

interface AppState {
  view: ViewId;
  /// `undefined` until the first check comes back. `null` means it came back
  /// and there is nothing installed, which reads very differently.
  rpcs3Version: string | null | undefined;
  firmwareVersion: string | null | undefined;
  /// `undefined` until the library has been read. An empty array means it was
  /// read and there is nothing in it, which is what "No games yet" is for.
  games: Game[] | undefined;
  /// The library search box.
  search: string;
  /// The catalogue has its own, over every PS3 game rather than yours.
  catalogueQuery: string;
  /// Which region the catalogue is showing, or empty for all of them.
  catalogueRegion: string;
  /// Whose controller layout is on screen: empty for every game, or a title id.
  controllerScope: string;
  /// The catalogue title open in the side panel, if any.
  catalogueSelected: CatalogueSelection | null;
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
    rpcs3Version: undefined,
    firmwareVersion: undefined,
    games: undefined,
    search: "",
    catalogueQuery: "",
    catalogueRegion: "",
    controllerScope: "",
    catalogueSelected: null,
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

  /// Redraw without anything having changed. The catalogue needs it after
  /// fetching the list, where the state is the same but the answer is not.
  redraw(): void {
    this.notify();
  }

  setCatalogueSelected(catalogueSelected: CatalogueSelection | null): void {
    this.state = { ...this.state, catalogueSelected };
    this.notify();
  }

  setControllerScope(controllerScope: string): void {
    if (controllerScope === this.state.controllerScope) return;
    this.state = { ...this.state, controllerScope };
    this.notify();
  }

  setCatalogueRegion(catalogueRegion: string): void {
    if (catalogueRegion === this.state.catalogueRegion) return;
    this.state = { ...this.state, catalogueRegion };
    this.notify();
  }

  setCatalogueQuery(catalogueQuery: string): void {
    if (catalogueQuery === this.state.catalogueQuery) return;
    this.state = { ...this.state, catalogueQuery };
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
