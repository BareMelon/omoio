import { openImportSheet } from "./importSheet";
import { store, type ViewId } from "../state";

interface NavItem {
  id: ViewId;
  label: string;
  icon: string;
}

const GAMES_NAV: NavItem[] = [
  { id: "library", label: "Library", icon: '<rect x="2" y="2.5" width="12" height="11" rx="1.5"/><path d="M5.5 2.5v11"/>' },
  { id: "catalogue", label: "Catalogue", icon: '<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5L14 14"/>' },
  { id: "homebrew", label: "Homebrew", icon: '<path d="M8 2v8M5 7l3 3 3-3M3 13h10"/>' },
];

const MAINTENANCE_NAV: NavItem[] = [
  { id: "updates", label: "Updates", icon: '<path d="M13.5 8a5.5 5.5 0 1 1-1.9-4.2M13 2v3.5h-3.5"/>' },
  {
    id: "system",
    label: "System",
    icon: '<circle cx="8" cy="8" r="2.2"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M12.6 3.4l-1.4 1.4M4.8 11.2l-1.4 1.4"/>',
  },
];

function navLabel(text: string): HTMLElement {
  const el = document.createElement("div");
  el.className = "nav-label";
  el.textContent = text;
  return el;
}

function navButton(item: NavItem): HTMLButtonElement {
  const btn = document.createElement("button");
  btn.className = "nav-item";
  btn.dataset.view = item.id;
  btn.innerHTML = `
    <svg class="nav-ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">${item.icon}</svg>
    <span>${item.label}</span>
  `;
  btn.onclick = () => store.setView(item.id);
  return btn;
}

export function renderSidebar(): HTMLElement {
  const side = document.createElement("aside");
  side.className = "side";

  side.appendChild(navLabel("Games"));
  GAMES_NAV.forEach((item) => side.appendChild(navButton(item)));

  side.appendChild(navLabel("Maintenance"));
  MAINTENANCE_NAV.forEach((item) => side.appendChild(navButton(item)));

  const foot = document.createElement("div");
  foot.className = "side-foot";
  foot.innerHTML = `
    <div class="stat"><span>RPCS3</span><span id="rpcs3-stat">Not installed</span></div>
    <div class="stat"><span>Firmware</span><span id="firmware-stat">Not installed</span></div>
    <button class="import-btn" id="import-game">
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M8 3.5v9M3.5 8h9"/></svg>
      Import game
    </button>
  `;
  side.appendChild(foot);

  const rpcs3Stat = foot.querySelector<HTMLElement>("#rpcs3-stat")!;
  const firmwareStat = foot.querySelector<HTMLElement>("#firmware-stat")!;
  foot.querySelector<HTMLButtonElement>("#import-game")!.onclick = openImportSheet;

  store.subscribe((state) => {
    side.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
      btn.classList.toggle("on", btn.dataset.view === state.view);
    });
    rpcs3Stat.textContent = state.rpcs3Version ?? "Not installed";
    firmwareStat.textContent = state.firmwareVersion ?? "Not installed";
  });

  return side;
}
