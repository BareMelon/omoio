// This fork has no signing key or update feed. Never install upstream releases
// over it: they would replace this fork. Emulator updates still work.
import { openUrl } from "@tauri-apps/plugin-opener";
import { platform } from "../platform";
export function readyUpdate(): { version: string } | null { return null; }
export function onUpdateChange(_listener: () => void): () => void { return () => {}; }
export async function restartToUpdate(): Promise<boolean> { return false; }
export function startUpdateChecks(): void {}

export function updateControls(): HTMLElement[] {
  const link = document.createElement("a");
  link.href = `https://github.com/BareMelon/omoio/actions/workflows/${platform.os === "linux" ? "linux" : platform.os === "macos" ? "macos" : "build"}.yml`;
  link.target = "_blank";
  link.rel = "noopener noreferrer";
  link.textContent = "Download fork builds";
  link.onclick = (event) => {
    event.preventDefault();
    void openUrl(link.href);
  };
  return [link];
}
