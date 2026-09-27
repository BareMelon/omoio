import "../styles/bigpicture.css";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  gameCompatibility,
  getSettings,
  launchGame,
  listSessions,
  onGameStopped,
  padsConnected,
  resumeGame,
  setBigPicture,
  setStartInBigPicture,
  stopGame,
  type Compatibility,
  type Console,
  type Game,
  type PadFamily,
} from "../api";
import { placeholderArt, tint } from "../components/art";
import { store } from "../state";
import { focusables, nearest, remember, revealInColumn, revealInRow } from "./focus";
import { inFront, listen, untilLetGo, type Action, type Source } from "./input";
import { keycap } from "./keys";

/// Big Picture: Omoio for the sofa. The whole screen, big type, and every
/// part of it reachable with the d-pad or left stick: the bottom face button
/// chooses, the right one goes back, Menu opens the side menu, the bumpers
/// change tabs. Keyboard and mouse work too.
///
/// While a game runs it covers all of this. View and Menu together bring
/// Big Picture back over it with the game still running, which is handled
/// by the backend since the game has the keyboard at that moment.

type Section = "home" | "library";

interface Screen {
  section: Section;
  draw(): HTMLElement;
  /// The key of what is highlighted when the screen opens.
  first(): string | undefined;
  /// The bumpers, for a screen with tabs of its own.
  tab?(step: number): void;
  /// What was highlighted when the screen was last on show.
  left?: string;
}

interface Question {
  title: string;
  text: string;
  confirm: string;
  run: () => void;
}

const SHORT: Record<Console, string> = { ps3: "PS3", wiiu: "Wii U" };

const MARK = `<svg viewBox="0 0 1254 1254" fill="currentColor" fill-rule="evenodd" aria-hidden="true">
  <path d="M342 222l-60 26-40 27-16 27-6 24v610l6 24 16 27 40 27 61 26-14-24-9-30-1-30V282l1-30 9-30z"/>
  <path d="M396 205l-30 29-10 40v686l10 40 30 29 56 12h439l56-12 45-29 30-40 11-40v-82h-49l-25-5-16-21v-95l16-21 25-5h49V404h-52l-25-5-16-21v-95l16-21 25-5h52v-77l-11-40-30-40-45-29-56-12H452zM575 429h20l33 12 249 147 22 26 8 30v17l-8 30-22 26-249 147-33 12h-20l-33-12-22-26-8-30V475l8-30 22-26z"/>
</svg>`;

const PLAY_ICON = `<svg viewBox="0 0 12 14" fill="currentColor" aria-hidden="true"><path d="M1.5 1.2 11 7l-9.5 5.8z"/></svg>`;

// ---- small helpers ----

function h<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text?: string): HTMLElementTagNameMap[K] {
  const made = document.createElement(tag);
  if (className) made.className = className;
  if (text !== undefined) made.textContent = text;
  return made;
}

/// Something the highlight can land on. `key` finds it again after the
/// screen is drawn anew; `group` is the row it belongs to.
function navButton(className: string, key: string, run: () => void, group?: string): HTMLButtonElement {
  const button = h("button", `${className} nav`);
  button.dataset.key = key;
  if (group) button.dataset.group = group;
  button.onclick = () => {
    if (button.getAttribute("aria-disabled") !== "true") run();
  };
  return button;
}

function artFor(game: Game): string {
  return game.cover
    ? `<img src="${convertFileSrc(game.cover)}" alt="" decoding="async">`
    : placeholderArt(game.title_id, game.title);
}

function formatSize(bytes: number): string {
  if (bytes <= 0) return "";
  const gb = bytes / 1024 ** 3;
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${Math.round(bytes / 1024 ** 2)} MB`;
}

/// When a game was last played, in the words a person would use.
function lastPlayedText(seconds: number): string {
  const then = new Date(seconds * 1000);
  const today = new Date();
  const days = Math.round(
    (new Date(today.toDateString()).getTime() - new Date(then.toDateString()).getTime()) / 86_400_000
  );
  if (days <= 0) return "Played today";
  if (days === 1) return "Played yesterday";
  if (days < 7) return `Played ${days} days ago`;
  return `Played ${then.toLocaleDateString(undefined, { day: "numeric", month: "long" })}`;
}

function reducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function playable(game: Game): boolean {
  return game.set_up && game.available;
}

// ---- state ----

const root = h("div", "bp");
root.setAttribute("aria-label", "Big Picture");

const backdrop = h("div", "bp-backdrop");
const layers = [h("div", "bp-backdrop-layer"), h("div", "bp-backdrop-layer")];
backdrop.append(...layers);

const topBar = h("header", "bp-top");
const mainLayer = h("div", "bp-main");
const screenHost = h("div", "bp-host");
const hintsBar = h("footer", "bp-hints");
const menu = h("div", "bp-menu");
const dialogLayer = h("div", "bp-dialog");
const startingLayer = h("div", "bp-starting");
const toast = h("div", "bp-toast");
toast.setAttribute("role", "status");

mainLayer.append(topBar, screenHost);
root.append(backdrop, mainLayer, hintsBar, menu, dialogLayer, startingLayer, toast);

const stack: Screen[] = [];
let showing = false;
let current: HTMLElement | null = null;
let menuOpen = false;
let menuReturn: string | undefined;
let question: Question | null = null;
let questionReturn: string | undefined;
let starting: Game | null = null;
let stopping = false;
let quitting = false;
let source: Source = "pad";
let family: PadFamily | null = null;
let startInBigPicture = false;
let libraryConsole: Console | "" = "";
let drawnFor = "";
/// When each game was last started, from the kept session logs.
let lastPlayed = new Map<string, number>();
const compat = new Map<string, Promise<Compatibility>>();

function top(): Screen {
  return stack[stack.length - 1];
}

function games(): Game[] {
  return store.get().games ?? [];
}

function gameById(titleId: string | undefined): Game | undefined {
  return games().find((game) => game.title_id === titleId);
}

function compatibility(game: Game): Promise<Compatibility> | null {
  if (!game.features.compatibility) return null;
  let known = compat.get(game.title_id);
  if (!known) {
    known = gameCompatibility(game.title_id);
    compat.set(game.title_id, known);
  }
  return known;
}

/// The layer the highlight moves within: whatever sits on top.
function layer(): HTMLElement {
  if (starting) return startingLayer;
  if (question) return dialogLayer;
  if (menuOpen) return menu;
  return mainLayer;
}

function byKey(key: string | undefined, within: HTMLElement = layer()): HTMLElement | null {
  if (!key) return null;
  return focusables(within).find((el) => el.dataset.key === key) ?? null;
}

// ---- the backdrop: the highlighted game's picture, blurred, behind everything ----

let backdropGame = "";
let backdropTimer: number | undefined;
let backdropAt = 0;

function showBackdrop(titleId: string): void {
  if (titleId === backdropGame) return;
  backdropGame = titleId;
  window.clearTimeout(backdropTimer);
  // After a pause, so running along a row doesn't flicker through every
  // picture on the way.
  backdropTimer = window.setTimeout(() => void swapBackdrop(titleId), 140);
}

async function swapBackdrop(titleId: string): Promise<void> {
  const game = gameById(titleId);
  const incoming = layers[1 - backdropAt];
  // RAWG's pictures are large enough to show sharp across a TV; a dump's
  // own icon is 320 pixels wide and is only ever shown blurred.
  incoming.classList.toggle("sharp", game?.cover_source === "rawg");
  if (game?.cover) {
    const img = h("img");
    img.alt = "";
    img.src = convertFileSrc(game.cover);
    await img.decode().catch(() => {});
    incoming.replaceChildren(img);
    incoming.style.background = "";
  } else {
    const { back, front } = tint(titleId);
    incoming.replaceChildren();
    incoming.style.background = `radial-gradient(ellipse at 70% 20%, ${front}, ${back} 70%)`;
  }
  if (backdropGame !== titleId) return;
  layers[backdropAt].classList.remove("on");
  incoming.classList.add("on");
  backdropAt = 1 - backdropAt;
}

// ---- the highlight ----

function focus(el: HTMLElement | null | undefined, animate = true): void {
  if (!el) return;
  if (current !== el) current?.classList.remove("focused");
  current = el;
  el.classList.add("focused");
  if (document.activeElement !== el) el.focus({ preventScroll: true });
  remember(el);
  if (mainLayer.contains(el) && screenHost.contains(el)) top().left = el.dataset.key;

  const track = el.parentElement;
  if (track?.classList.contains("bp-track")) {
    if (!animate) track.style.transition = "none";
    revealInRow(el);
    if (!animate) {
      void track.offsetWidth;
      track.style.transition = "";
    }
  }
  const scroller = el.closest<HTMLElement>(".bp-screen");
  if (scroller) revealInColumn(el, scroller, animate && !reducedMotion());

  if (el.dataset.game) {
    showBackdrop(el.dataset.game);
    spotlight(el.dataset.game);
  }
  drawHints();
}

function move(action: Action): void {
  const candidates = focusables(layer());
  if (!current || !candidates.includes(current)) {
    focus(candidates[0]);
    return;
  }
  if (action === "up" || action === "down" || action === "left" || action === "right") {
    focus(nearest(current, candidates, action));
  }
}

/// A short press shape on the highlighted thing while the button is down.
function pressed(down: boolean): void {
  current?.classList.toggle("pressed", down);
}

// ---- actions ----

function act(action: Action, from: Source): void {
  if (from !== source) {
    source = from;
    drawHints();
  }
  switch (action) {
    case "up":
    case "down":
    case "left":
    case "right":
      move(action);
      return;
    case "accept":
      if (current && layer().contains(current)) current.click();
      else focus(focusables(layer())[0]);
      return;
    case "back":
      if (starting) void cancelStart();
      else if (question) closeQuestion();
      else if (menuOpen) closeMenu();
      else back();
      return;
    case "menu":
      if (starting || question) return;
      if (menuOpen) closeMenu();
      else openMenu();
      return;
    case "prev":
    case "next":
      if (!starting && !question && !menuOpen) top().tab?.(action === "next" ? 1 : -1);
  }
}

function back(): void {
  if (stack.length > 1) {
    stack.pop();
    render(true);
  } else if (top().section !== "home") {
    go("home");
  } else {
    openMenu();
  }
}

function go(section: Section): void {
  stack.length = 0;
  stack.push(section === "home" ? home() : library());
  render(true);
}

function open(screen: Screen): void {
  stack.push(screen);
  render(true);
}

function openGame(titleId: string): void {
  open(gamePage(titleId, top().section));
}

let toastTimer: number | undefined;

function say(text: string): void {
  toast.textContent = text;
  toast.classList.add("on");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => toast.classList.remove("on"), 5000);
}

async function play(game: Game): Promise<void> {
  const { playing } = store.get();
  if (playing?.title_id === game.title_id) return resume();
  if (!playable(game)) return;
  if (playing) {
    ask({
      title: `Quit ${playing.title}?`,
      text: `${game.title} starts in its place. Anything not saved in ${playing.title} is lost.`,
      confirm: "Quit and play",
      run: () => void start(game),
    });
    return;
  }
  await start(game);
}

async function start(game: Game): Promise<void> {
  starting = game;
  stopping = false;
  drawStarting();
  try {
    await launchGame(game.title_id);
    // Starting a game ends the one before it, and the new one is on screen.
    if (store.get().playing) store.setPlaying(null);
    store.setBigPicture({ on: true, suspended: false });
  } catch (err) {
    starting = null;
    drawStarting();
    say(typeof err === "string" ? err : "Couldn't start this game.");
  }
}

async function cancelStart(): Promise<void> {
  if (!starting || stopping) return;
  stopping = true;
  drawStarting();
  await stopGame();
}

async function resume(): Promise<void> {
  // The game takes a button still held down as a press of its own.
  await untilLetGo();
  await resumeGame();
}

function quitGame(): void {
  const { playing } = store.get();
  if (!playing) return;
  ask({
    title: `Quit ${playing.title}?`,
    text: "Anything not saved in the game is lost.",
    confirm: "Quit game",
    run: () => {
      quitting = true;
      render(false);
      void stopGame();
    },
  });
}

function quitOmoio(): void {
  const { playing } = store.get();
  const close = () => void getCurrentWindow().close();
  if (!playing) return close();
  ask({
    title: "Quit Omoio?",
    text: `${playing.title} is closed too. Anything not saved in it is lost.`,
    confirm: "Quit Omoio",
    run: close,
  });
}

// ---- the side menu ----

function menuItem(key: string, label: string, run: () => void, value?: string): HTMLButtonElement {
  const item = navButton("bp-menu-item", key, run);
  item.append(h("span", "", label));
  if (value !== undefined) item.append(h("span", "bp-menu-value", value));
  return item;
}

function drawMenu(): void {
  const panel = h("nav", "bp-menu-panel");
  panel.setAttribute("aria-label", "Menu");
  const head = h("div", "bp-menu-head");
  head.innerHTML = MARK;
  head.append(h("span", "", "Omoio"));
  panel.append(
    head,
    menuItem("menu:home", "Home", () => {
      closeMenu();
      go("home");
    }),
    menuItem("menu:library", "Library", () => {
      closeMenu();
      go("library");
    }),
    h("div", "bp-menu-rule"),
    menuItem(
      "menu:start",
      "Open Omoio in Big Picture",
      async () => {
        startInBigPicture = !startInBigPicture;
        await setStartInBigPicture(startInBigPicture);
        drawMenu();
        focus(byKey("menu:start", menu));
      },
      startInBigPicture ? "On" : "Off"
    ),
    menuItem("menu:exit", "Exit Big Picture", () => void setBigPicture(false)),
    menuItem("menu:quit", "Quit Omoio", quitOmoio)
  );
  const scrim = h("div", "bp-menu-scrim");
  scrim.onclick = closeMenu;
  menu.replaceChildren(scrim, panel);
}

function openMenu(): void {
  menuOpen = true;
  menuReturn = current?.dataset.key;
  drawMenu();
  menu.classList.add("open");
  focus(byKey("menu:home", menu));
}

function closeMenu(): void {
  menuOpen = false;
  menu.classList.remove("open");
  focus(byKey(menuReturn, mainLayer) ?? focusables(mainLayer)[0]);
}

// ---- questions ----

function ask(next: Question): void {
  question = next;
  questionReturn = current?.dataset.key;
  const card = h("div", "bp-dialog-card");
  card.setAttribute("role", "alertdialog");
  card.setAttribute("aria-label", next.title);
  const actions = h("div", "bp-dialog-actions");
  const yes = navButton("bp-btn primary", "question:yes", () => {
    closeQuestion();
    next.run();
  });
  yes.textContent = next.confirm;
  const no = navButton("bp-btn", "question:no", closeQuestion);
  no.textContent = "Cancel";
  actions.append(yes, no);
  card.append(h("div", "bp-dialog-title", next.title), h("div", "bp-dialog-text", next.text), actions);
  dialogLayer.replaceChildren(card);
  dialogLayer.classList.add("open");
  focus(yes);
}

function closeQuestion(): void {
  question = null;
  dialogLayer.classList.remove("open");
  dialogLayer.replaceChildren();
  focus(byKey(questionReturn, menuOpen ? menu : mainLayer) ?? focusables(layer())[0]);
}

// ---- starting a game ----

function drawStarting(): void {
  startingLayer.classList.toggle("open", Boolean(starting));
  if (!starting) {
    startingLayer.replaceChildren();
    focus(byKey(top()?.left) ?? focusables(layer())[0], false);
    return;
  }
  const card = h("div", "bp-starting-card");
  const art = h("div", "bp-starting-art");
  art.innerHTML = artFor(starting);
  const cancel = navButton("bp-btn", "starting:cancel", () => void cancelStart());
  cancel.textContent = stopping ? "Stopping…" : "Cancel";
  cancel.setAttribute("aria-disabled", String(stopping));
  card.append(
    art,
    h("div", "bp-starting-title", starting.title),
    h("div", "bp-spinner"),
    h("div", "bp-starting-note", stopping ? "Stopping the game." : "Starting the game."),
    chordNote("while you play brings you back here."),
    cancel
  );
  startingLayer.replaceChildren(card);
  focus(cancel);
}

/// "View + Menu while you play…", with the buttons of the pad in hand.
function chordNote(rest: string): HTMLElement {
  const note = h("div", "bp-chord");
  const back = keycap("Back", family, "pad");
  const menuKey = keycap("Start", family, "pad");
  if (back && menuKey) note.innerHTML = `${back}<span>+</span>${menuKey}`;
  note.append(h("span", "", rest));
  return note;
}

// ---- the top bar and the hints ----

const clock = h("span", "bp-clock");
const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

function tick(): void {
  clock.textContent = timeFormat.format(new Date());
}

function drawTop(): void {
  const brand = navButton("bp-brand", "top:menu", openMenu);
  brand.setAttribute("aria-label", "Menu");
  brand.innerHTML = `${MARK}<span>Omoio</span>`;

  const sections = h("nav", "bp-sections");
  for (const [section, label] of [
    ["home", "Home"],
    ["library", "Library"],
  ] as const) {
    const tab = navButton(`bp-section${top().section === section ? " on" : ""}`, `top:${section}`, () => go(section));
    tab.textContent = label;
    sections.append(tab);
  }

  const right = h("div", "bp-top-right");
  const { playing } = store.get();
  if (playing) {
    const now = navButton("bp-now", "top:now", () => openGame(playing.title_id));
    now.dataset.game = playing.title_id;
    now.append(h("span", "bp-now-dot"), h("span", "", playing.title));
    right.append(now);
  }
  tick();
  right.append(clock);
  topBar.replaceChildren(brand, sections, right);
}

function hint(input: string, words: string): HTMLElement | null {
  const cap = keycap(input, family, source);
  if (!cap) return null;
  const el = h("span", "bp-hint");
  el.innerHTML = cap;
  el.append(h("span", "", words));
  return el;
}

function drawHints(): void {
  const shown: (HTMLElement | null)[] = [];
  if (starting) {
    shown.push(hint("East", "Cancel"));
  } else if (question) {
    shown.push(hint("South", "Select"), hint("East", "Cancel"));
  } else if (menuOpen) {
    shown.push(hint("South", "Select"), hint("East", "Close"));
  } else {
    const deeper = stack.length > 1 || top()?.section !== "home";
    shown.push(hint("South", current?.dataset.hint ?? "Select"));
    if (deeper) shown.push(hint("East", "Back"));
    else if (source === "keyboard") shown.push(hint("East", "Menu"));
    if (top()?.tab && consoles().length > 1) {
      const bumpers = h("span", "bp-hint");
      bumpers.innerHTML = `${keycap("LB", family, source) ?? ""}${keycap("RB", family, source) ?? ""}`;
      bumpers.append(h("span", "", "Console"));
      shown.push(bumpers);
    }
    shown.push(hint("Start", "Menu"));
  }
  const left = h("div", "bp-hints-left");
  if (store.get().suspended && source === "pad" && family && !starting) left.append(chordNote("Back to game"));
  hintsBar.replaceChildren(left, ...shown.filter((el): el is HTMLElement => el !== null));
}

// ---- tiles ----

function tile(game: Game, group: string, withMeta: boolean): HTMLButtonElement {
  const { playing } = store.get();
  const open = () => openGame(game.title_id);
  const card = navButton(`bp-tile${playable(game) ? "" : " ghost"}`, `${group}:${game.title_id}`, open, group);
  card.dataset.game = game.title_id;
  card.dataset.hint = "Open";
  card.title = game.title;
  const art = h("span", "bp-art");
  art.innerHTML = artFor(game);
  const badge =
    playing?.title_id === game.title_id
      ? ["Playing", "go"]
      : !game.set_up
        ? ["Not set up", ""]
        : !game.available
          ? ["Offline", "warn"]
          : null;
  if (badge) art.append(h("span", `bp-badge ${badge[1]}`, badge[0]));
  card.append(art, h("span", "bp-tile-name", game.title));
  if (withMeta) {
    const meta = [SHORT[game.console], game.set_up ? formatSize(game.size_bytes) : ""].filter(Boolean).join(" · ");
    card.append(h("span", "bp-tile-meta", meta));
  }
  return card;
}

function consoles(): Console[] {
  return [...new Set(games().map((game) => game.console))];
}

// ---- Home ----

/// The game on the spotlight under the row on Home: its name and what is
/// known about it, for whichever tile is highlighted.
function spotlight(titleId: string): void {
  const spot = screenHost.querySelector<HTMLElement>(".bp-spot");
  const game = gameById(titleId);
  if (!spot || !game || spot.dataset.game === titleId) return;
  spot.dataset.game = titleId;
  const meta = h("div", "bp-spot-meta");
  meta.append(h("span", "", SHORT[game.console]));
  const when = lastPlayed.get(titleId);
  if (when) meta.append(h("span", "", lastPlayedText(when)));
  spot.replaceChildren(h("div", "bp-spot-title", game.title), meta);
  void compatibility(game)?.then((result) => {
    if (spot.dataset.game !== titleId || !result.known) return;
    meta.append(h("span", `bp-status ${result.tone}`, result.label));
  });
}

function nowPlaying(): HTMLElement | null {
  const { playing } = store.get();
  if (!playing) return null;
  const game = gameById(playing.title_id);
  const banner = h("section", "bp-now-banner");
  const art = h("div", "bp-now-art");
  if (game) art.innerHTML = artFor(game);
  const words = h("div", "bp-now-words");
  words.append(h("div", "bp-now-label", "Now playing"), h("div", "bp-now-title", playing.title));
  const actions = h("div", "bp-now-actions");
  const resumeButton = navButton("bp-btn primary", "now:resume", () => void resume(), "now");
  resumeButton.innerHTML = `${PLAY_ICON}<span>Resume</span>`;
  resumeButton.dataset.hint = "Resume";
  const quit = navButton("bp-btn", "now:quit", quitGame, "now");
  quit.textContent = quitting ? "Quitting…" : "Quit game";
  if (quitting) {
    resumeButton.setAttribute("aria-disabled", "true");
    quit.setAttribute("aria-disabled", "true");
  }
  for (const button of [resumeButton, quit]) button.dataset.game = playing.title_id;
  actions.append(resumeButton, quit);
  banner.append(art, words, actions);
  return banner;
}

function emptyLibrary(): HTMLElement {
  const empty = h("div", "bp-empty");
  const exit = navButton("bp-btn", "empty:exit", () => void setBigPicture(false));
  exit.textContent = "Exit Big Picture";
  empty.append(
    h("div", "bp-empty-title", "No games yet"),
    h(
      "div",
      "bp-empty-text",
      "Games are added from the desktop: drop a game onto Omoio's window, or use Import game."
    ),
    exit
  );
  return empty;
}

function home(): Screen {
  return {
    section: "home",
    first() {
      const { playing, suspended } = store.get();
      if (playing && suspended) return "now:resume";
      const first = homeGames()[0];
      return first ? `home:${first.title_id}` : "empty:exit";
    },
    draw() {
      const screen = h("div", "bp-screen bp-home");
      const banner = nowPlaying();
      if (banner) screen.append(banner);
      const list = homeGames();
      if (list.length === 0) {
        screen.append(emptyLibrary());
        return screen;
      }
      const shelf = h("section", "bp-shelf");
      const rail = h("div", "bp-rail");
      const track = h("div", "bp-track");
      track.append(...list.map((game) => tile(game, "home", false)));
      rail.append(track);
      // A wheel moves along the row, since the row itself doesn't scroll.
      rail.addEventListener(
        "wheel",
        (event) => {
          if (!current || !track.contains(current)) return;
          event.preventDefault();
          move(event.deltaY + event.deltaX > 0 ? "right" : "left");
        },
        { passive: false }
      );
      shelf.append(h("h2", "bp-shelf-h", "Your games"), rail);
      screen.append(shelf, h("div", "bp-spot"));
      return screen;
    },
  };
}

/// Games that can be played, the ones played most lately first.
function homeGames(): Game[] {
  return games()
    .filter((game) => game.set_up)
    .sort(
      (a, b) =>
        (lastPlayed.get(b.title_id) ?? 0) - (lastPlayed.get(a.title_id) ?? 0) || a.title.localeCompare(b.title)
    );
}

// ---- Library ----

function library(): Screen {
  const screen: Screen = {
    section: "library",
    first() {
      const first = libraryGames()[0];
      return first ? `library:${first.title_id}` : "empty:exit";
    },
    draw() {
      const page = h("div", "bp-screen bp-library");
      const head = h("div", "bp-library-head");
      const all = games();
      const shown = libraryGames();
      head.append(
        h("h1", "bp-page-title", "Library"),
        h("span", "bp-count", `${shown.length} ${shown.length === 1 ? "game" : "games"}`)
      );
      const kinds = consoles();
      if (kinds.length > 1) {
        // Changed with the bumpers, and clicked with a mouse; the highlight
        // stays on the games.
        const tabs = h("div", "bp-tabs");
        tabs.innerHTML = keycap("LB", family, source) ?? "";
        for (const kind of ["", ...kinds] as (Console | "")[]) {
          const tab = h("button", `bp-tab${kind === libraryConsole ? " on" : ""}`, kind ? SHORT[kind] : "All");
          tab.tabIndex = -1;
          tab.onclick = () => {
            libraryConsole = kind;
            screen.left = undefined;
            render(false);
          };
          tabs.append(tab);
        }
        tabs.insertAdjacentHTML("beforeend", keycap("RB", family, source) ?? "");
        head.append(tabs);
      }
      page.append(head);
      if (all.length === 0) {
        page.append(emptyLibrary());
        return page;
      }
      const grid = h("div", "bp-grid");
      grid.append(...shown.map((game) => tile(game, "library", true)));
      page.append(grid);
      return page;
    },
    tab(step) {
      const kinds: (Console | "")[] = ["", ...consoles()];
      if (kinds.length < 3) return;
      const at = kinds.indexOf(libraryConsole);
      libraryConsole = kinds[(at + step + kinds.length) % kinds.length];
      screen.left = undefined;
      render(false);
    },
  };
  return screen;
}

/// Every game, those that can be played first, each group by name.
function libraryGames(): Game[] {
  return games()
    .filter((game) => !libraryConsole || game.console === libraryConsole)
    .sort((a, b) => Number(b.set_up) - Number(a.set_up) || a.title.localeCompare(b.title));
}

// ---- a game's page ----

function gamePage(titleId: string, section: Section): Screen {
  return {
    section,
    first: () => "game:primary",
    draw() {
      const page = h("div", "bp-screen bp-page");
      const game = gameById(titleId);
      if (!game) {
        page.append(h("div", "bp-empty-title", "This game isn't in your library any more."));
        return page;
      }
      const { playing } = store.get();
      const running = playing?.title_id === titleId;

      const art = h("div", "bp-page-art");
      art.innerHTML = artFor(game);

      const info = h("div", "bp-page-info");
      const meta = h("div", "bp-page-meta");
      meta.append(h("span", "", SHORT[game.console]), h("span", "bp-mono", game.title_id));
      const version = game.update_version ?? game.version;
      if (version) meta.append(h("span", "bp-mono", `v${version}`));
      if (game.set_up && game.size_bytes > 0) meta.append(h("span", "bp-mono", formatSize(game.size_bytes)));

      const actions = h("div", "bp-page-actions");
      const primary = navButton("bp-btn primary big", "game:primary", () =>
        running ? void resume() : void play(game)
      );
      primary.dataset.game = titleId;
      primary.innerHTML = `${PLAY_ICON}<span>${running ? "Resume" : "Play"}</span>`;
      primary.dataset.hint = running ? "Resume" : "Play";
      if (!running && !playable(game)) primary.setAttribute("aria-disabled", "true");
      actions.append(primary);
      if (running) {
        const quit = navButton("bp-btn big", "game:quit", quitGame);
        quit.dataset.game = titleId;
        quit.textContent = quitting ? "Quitting…" : "Quit game";
        if (quitting) {
          quit.setAttribute("aria-disabled", "true");
          primary.setAttribute("aria-disabled", "true");
        }
        actions.append(quit);
      }

      const notes = h("div", "bp-page-notes");
      const note = !game.set_up
        ? "Import this game's files from the desktop to play it."
        : !game.available
          ? "Connect the drive this game is on to play it."
          : running && !game.features.quiet_behind
            ? "The game still hears the controller while Big Picture is open."
            : "";
      if (note) notes.append(h("p", "bp-page-note", note));
      const when = lastPlayed.get(titleId);
      if (when) notes.append(h("p", "bp-page-quiet", lastPlayedText(when)));
      const runs = h("div", "bp-page-runs");
      notes.append(runs);
      void compatibility(game)?.then((result) => {
        if (!result.known || !runs.isConnected) return;
        runs.append(h("span", `bp-status ${result.tone}`, result.label), h("span", "", result.explanation));
      });
      if (playable(game) && family) notes.append(chordNote("while you play brings you back here."));

      info.append(meta, h("h1", "bp-page-title", game.title), actions, notes);
      page.append(art, info);
      return page;
    },
  };
}

// ---- drawing ----

/// Draws the screen on top again. `entering` plays the way in, for a screen
/// newly opened; a redraw of the same screen keeps still and keeps its place.
function render(entering: boolean): void {
  const screen = top();
  if (!screen) return;
  const scrolled = entering ? 0 : (screenHost.querySelector(".bp-screen")?.scrollTop ?? 0);
  root.classList.toggle("hero", stack.length === 1 && screen.section === "home");
  drawTop();
  const el = screen.draw();
  if (entering && !reducedMotion()) el.classList.add("entering");
  screenHost.replaceChildren(el);
  el.scrollTop = scrolled;
  drawnFor = drawnFrom();
  if (layer() === mainLayer) {
    focus(byKey(screen.left) ?? byKey(screen.first()) ?? focusables(mainLayer)[0], entering);
  } else {
    drawHints();
  }
}

/// What the screens are drawn from, so a change elsewhere in the app that
/// doesn't touch them never redraws them.
function drawnFrom(): string {
  const { games: all, playing, suspended } = store.get();
  return JSON.stringify([
    all?.map((game) => [game.title_id, game.title, game.cover, game.available, game.set_up]),
    playing?.title_id,
    suspended,
    quitting,
  ]);
}

async function readSessions(): Promise<void> {
  const sessions = await listSessions().catch(() => []);
  const next = new Map<string, number>();
  for (const session of sessions) {
    const at = Number(session.started);
    if (at > (next.get(session.title_id) ?? 0)) next.set(session.title_id, at);
  }
  lastPlayed = next;
}

async function readPads(): Promise<void> {
  const pads = await padsConnected().catch(() => []);
  const next = pads[0]?.family ?? null;
  if (next !== family) {
    family = next;
    if (showing) drawHints();
  }
}

let arriving = false;

/// Big Picture coming up: over a game waiting behind it, on that game's page
/// with Resume ready, otherwise on Home.
async function arrive(): Promise<void> {
  if (arriving) return;
  arriving = true;
  const [, settings] = await Promise.all([readSessions(), getSettings().catch(() => null), readPads()]);
  arriving = false;
  const { bigPicture: on, playing, suspended } = store.get();
  if (!on) return;
  startInBigPicture = settings?.start_in_big_picture ?? false;
  stack.length = 0;
  stack.push(home());
  if (playing && suspended) stack.push(gamePage(playing.title_id, "home"));
  showing = true;
  render(true);
}

export function renderBigPicture(): HTMLElement {
  listen(
    act,
    () => {
      const { bigPicture, playing, suspended } = store.get();
      return bigPicture && showing && !(playing && !suspended && !starting);
    },
    pressed
  );

  // The mouse and Tab move the highlight too.
  root.addEventListener("focusin", (event) => {
    const target = (event.target as HTMLElement).closest<HTMLElement>(".nav");
    if (target && target !== current && layer().contains(target)) focus(target);
  });

  window.setInterval(() => {
    if (showing) tick();
  }, 10_000);
  window.setInterval(() => {
    if (showing && inFront()) void readPads();
  }, 4000);

  let wasSuspended = false;
  store.subscribe((state) => {
    if (!state.bigPicture) {
      showing = false;
      menuOpen = false;
      menu.classList.remove("open");
      wasSuspended = false;
      return;
    }
    if (!showing) {
      wasSuspended = state.suspended;
      void arrive();
      return;
    }
    if (starting && state.playing?.title_id === starting.title_id) {
      starting = null;
      drawStarting();
    }
    // Brought up over a game that was on the screen: straight to its page.
    if (state.suspended && !wasSuspended && state.playing) {
      wasSuspended = true;
      if (menuOpen) closeMenu();
      if (question) closeQuestion();
      const page = top().section;
      stack.length = 0;
      stack.push(home(), gamePage(state.playing.title_id, page));
      render(true);
      return;
    }
    wasSuspended = state.suspended;
    if (drawnFrom() !== drawnFor) render(false);
  });

  void onGameStopped(() => {
    const cancelled = stopping;
    quitting = false;
    stopping = false;
    if (starting) {
      starting = null;
      drawStarting();
      if (!cancelled) say("The game closed before it started. Its log is under Logs on the desktop.");
    }
    void readSessions().then(() => {
      if (showing) render(false);
    });
  });

  return root;
}
