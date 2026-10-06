import { check, type Update } from "@tauri-apps/plugin-updater";
import { playingGame } from "../api";
import { store } from "../state";

/// Omoio's own updates. GitHub is asked a little after the library loads and
/// every few hours while Omoio stays open. A newer release is downloaded in
/// the background and checked against the public key in tauri.conf.json
/// before it is offered. Installing it closes Omoio, so it is only ever
/// offered, and never while a game is running.

const FIRST_LOOK_MS = 8_000;
const LOOK_EVERY_MS = 4 * 60 * 60 * 1000;

export type UpdatePhase = "idle" | "checking" | "downloading";

let phase: UpdatePhase = "idle";
let ready: Update | null = null;
let looking: Promise<Update | null> | null = null;
/// "Later" was pressed. It lasts until Omoio next starts.
let later = false;
const listeners = new Set<() => void>();

function changed(): void {
  for (const listener of listeners) listener();
}

function setPhase(next: UpdatePhase): void {
  phase = next;
  changed();
}

/// The update downloaded and waiting for a restart, if there is one.
export function readyUpdate(): Update | null {
  return ready;
}

/// Runs whenever a check starts or ends. The function returned stops it.
export function onUpdateChange(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/// Resolves to the downloaded update, or null when Omoio is up to date.
/// Rejects when GitHub can't be reached or the download fails its check.
/// A second call while one is out waits for the same answer.
export function lookForUpdate(): Promise<Update | null> {
  looking ??= look().finally(() => {
    looking = null;
    setPhase("idle");
  });
  return looking;
}

async function look(): Promise<Update | null> {
  setPhase("checking");
  const found = await check({ timeout: 30_000 });
  if (!found) {
    // Only possible with an update waiting when the release behind it was
    // taken down, and then it shouldn't be installed either.
    void ready?.close();
    ready = null;
    return null;
  }
  if (found.version === ready?.version) {
    void found.close();
    return ready;
  }
  setPhase("downloading");
  try {
    await found.download(undefined, { timeout: 15 * 60_000 });
  } catch (err) {
    void found.close();
    throw err;
  }
  void ready?.close();
  ready = found;
  return found;
}

/// Closes Omoio and runs the downloaded installer, which shows its progress
/// and starts Omoio again when it is done. On Windows the promise never
/// settles when it works, because Omoio has exited. False means a game is
/// running and nothing was done.
export async function restartToUpdate(): Promise<boolean> {
  if (!ready) return false;
  // The store can be a moment behind a game that is only now starting.
  if (store.get().playing || (await playingGame())) return false;
  await ready.install();
  return true;
}

const COULD_NOT_INSTALL = "Couldn't start the update. Try again, or download it from omoio.app.";

/// A quiet card in the corner once an update has downloaded. Hidden while a
/// game runs and in Big Picture, and by the stylesheet while a sheet or an
/// emulator update is on screen, since restarting would cut those short.
function renderPrompt(): HTMLElement {
  const box = document.createElement("div");
  box.className = "omoio-update";
  box.setAttribute("role", "status");
  box.hidden = true;
  box.innerHTML = `
    <div class="omoio-update-h"></div>
    <div class="omoio-update-p"></div>
    <div class="sheet-actions">
      <button class="btn solid">Restart to update</button>
      <button class="btn ghost">Later</button>
    </div>
  `;
  const heading = box.querySelector<HTMLElement>(".omoio-update-h")!;
  const notes = box.querySelector<HTMLElement>(".omoio-update-p")!;
  const [restart, dismiss] = box.querySelectorAll<HTMLButtonElement>("button");

  let showing = "";
  const refresh = () => {
    const { playing, bigPicture } = store.get();
    box.hidden = !ready || later || !!playing || bigPicture;
    if (ready && ready.version !== showing) {
      showing = ready.version;
      heading.textContent = `Omoio ${ready.version} is ready`;
      notes.textContent = ready.body ?? "";
    }
  };

  restart.onclick = async () => {
    restart.disabled = dismiss.disabled = true;
    restart.textContent = "Restarting…";
    try {
      await restartToUpdate();
    } catch (err) {
      console.warn("Couldn't install the update:", err);
      notes.textContent = COULD_NOT_INSTALL;
    }
    restart.disabled = dismiss.disabled = false;
    restart.textContent = "Restart to update";
    refresh();
  };
  dismiss.onclick = () => {
    later = true;
    refresh();
  };

  onUpdateChange(refresh);
  store.subscribe(refresh);
  return box;
}

/// Starts the checks and puts the prompt in place. Called once, after the
/// library has loaded. A failed check is only logged: being offline is not
/// something to tell anyone about.
export function startUpdateChecks(): void {
  document.body.appendChild(renderPrompt());
  const quietly = () => {
    lookForUpdate().catch((err) => console.warn("Couldn't check for an Omoio update:", err));
  };
  window.setTimeout(quietly, FIRST_LOOK_MS);
  window.setInterval(quietly, LOOK_EVERY_MS);
}

/// The Settings line: what is happening, and a button that checks now or,
/// once an update is ready, restarts to install it.
export function updateControls(): HTMLElement[] {
  const said = document.createElement("span");
  said.className = "update-said";
  const button = document.createElement("button");
  button.className = "small-btn";
  /// What the last press came to. A failure stays until the next press; "Up
  /// to date" gives way to an update found later.
  let answer = "";
  let failed = false;
  let seen = false;

  const refresh = () => {
    // The listener outlives the Settings screen it was made for.
    if (button.isConnected) seen = true;
    else if (seen) {
      stop();
      return;
    }
    button.disabled = phase !== "idle";
    button.textContent = ready ? "Restart to update" : "Check for updates";
    if (phase === "checking") said.textContent = "Checking…";
    else if (phase === "downloading") said.textContent = "Downloading the update…";
    else if (ready && !failed) said.textContent = `${ready.version} is ready`;
    else said.textContent = answer;
  };
  const stop = onUpdateChange(refresh);

  button.onclick = async () => {
    answer = "";
    failed = false;
    if (ready) {
      button.disabled = true;
      said.textContent = "Restarting…";
      try {
        await restartToUpdate();
      } catch (err) {
        console.warn("Couldn't install the update:", err);
        answer = COULD_NOT_INSTALL;
        failed = true;
      }
    } else {
      try {
        if (!(await lookForUpdate())) answer = "Up to date";
      } catch (err) {
        console.warn("Couldn't check for an Omoio update:", err);
        answer = "Couldn't reach GitHub. Check your internet connection and try again.";
        failed = true;
      }
    }
    refresh();
  };

  refresh();
  return [said, button];
}
