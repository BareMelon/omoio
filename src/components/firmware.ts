import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { installFirmware } from "../api";
import { store } from "../state";

// Sony publishes the firmware free but we never fetch it: the user downloads
// the PUP from here themselves and points us at it.
const SONY_FIRMWARE_PAGE = "https://www.playstation.com/en-us/support/hardware/ps3/system-software/";

// Kept outside any one prompt: the game's panel is drawn afresh whenever the
// store changes, and a prompt drawn mid-install has to show the install, not
// offer to start a second one.
let installing = false;
let failed: string | undefined;

/// The way to add the firmware from wherever a game turned out to need it,
/// with the same two steps as the System screen: Sony's page, then the file.
/// The store hears about the new version, so anything drawn from it, the
/// game's own panel included, redraws without the prompt.
export function firmwareActions(): HTMLElement {
  const actions = document.createElement("div");
  actions.className = "row-actions";

  function showProgress() {
    actions.innerHTML = `
      <div class="progress-row" style="flex:1">
        <div class="progress-label"><span>Installing firmware…</span></div>
        <div class="progress"><div class="progress-fill indeterminate"></div></div>
      </div>
    `;
  }

  function showIdle(note?: string) {
    actions.innerHTML = `
      <button class="small-btn" data-firmware="sony">Open Sony's download page</button>
      <button class="small-btn" data-firmware="pick">Choose PUP file…</button>
      ${note ? `<div class="progress-error"></div>` : ""}
    `;
    // Set through textContent, so the reason given is never treated as markup.
    if (note) actions.querySelector<HTMLElement>(".progress-error")!.textContent = note;
    actions.querySelector<HTMLButtonElement>('[data-firmware="sony"]')!.onclick = () => {
      openUrl(SONY_FIRMWARE_PAGE);
    };
    actions.querySelector<HTMLButtonElement>('[data-firmware="pick"]')!.onclick = choose;
  }

  async function choose() {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "PS3 firmware", extensions: ["pup"] }],
    });
    if (typeof selected !== "string" || installing) return;

    installing = true;
    failed = undefined;
    showProgress();
    try {
      const installed = await installFirmware(selected);
      installing = false;
      store.setFirmwareVersion(installed);
    } catch (err) {
      console.error("Firmware install failed:", err);
      installing = false;
      failed = typeof err === "string" ? err : "Couldn't install that firmware.";
      showIdle(failed);
      // A prompt drawn while this one was installing shows the reason too.
      store.redraw();
    }
  }

  if (installing) showProgress();
  else showIdle(failed);
  return actions;
}
