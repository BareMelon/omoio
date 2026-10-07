// This fork has no signing key or update feed. Never install upstream releases
// over it: they would remove Linux support. Emulator updates still work.
export function readyUpdate(): { version: string } | null { return null; }
export function onUpdateChange(_listener: () => void): () => void { return () => {}; }
export async function restartToUpdate(): Promise<boolean> { return false; }
export function startUpdateChecks(): void {}

export function updateControls(): HTMLElement[] {
  const link = document.createElement("a");
  link.href = "https://github.com/BareMelon/omoio/releases";
  link.target = "_blank";
  link.rel = "noopener noreferrer";
  link.textContent = "Download fork releases";
  return [link];
}
