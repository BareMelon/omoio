import { openUrl } from "@tauri-apps/plugin-opener";

/// RAWG's terms ask for an active link to them on every screen their images
/// appear on, so this goes wherever a RAWG cover is shown.
export function rawgCredit(): HTMLElement {
  const credit = document.createElement("button");
  credit.className = "link-btn rawg-credit";
  credit.textContent = "Covers from RAWG";
  credit.onclick = (event) => {
    event.stopPropagation();
    void openUrl("https://rawg.io/");
  };
  return credit;
}
