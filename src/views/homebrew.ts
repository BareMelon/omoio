import { open } from "@tauri-apps/plugin-dialog";
import {
  installPackage,
  installedPackages,
  removePackage,
  type InstalledPackage,
} from "../api";
import { store } from "../state";
import { emptyState, type View } from "./view";

function formatSize(bytes: number): string {
  if (bytes <= 0) return "";
  const gb = bytes / 1024 ** 3;
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${Math.round(bytes / 1024 ** 2)} MB`;
}

function row(installed: InstalledPackage, note: HTMLElement): HTMLElement {
  const line = document.createElement("div");
  line.className = "setting";

  const left = document.createElement("div");
  const name = document.createElement("div");
  name.className = "setting-k";
  // Comes from the package's own PARAM.SFO, so never through innerHTML.
  name.textContent = installed.title;
  const detail = document.createElement("div");
  detail.className = "setting-path";
  detail.textContent = [
    installed.title_id,
    installed.version ? `version ${installed.version}` : "",
    formatSize(installed.size_bytes),
  ]
    .filter(Boolean)
    .join(" · ");
  left.append(name, detail);

  const right = document.createElement("div");
  right.className = "row-actions";
  const remove = document.createElement("button");
  remove.className = "link-btn";
  remove.textContent = "Remove";
  remove.onclick = async () => {
    remove.disabled = true;
    try {
      await removePackage(installed.title_id);
      store.redraw();
    } catch (err) {
      note.textContent = typeof err === "string" ? err : "Couldn't remove that.";
      remove.disabled = false;
    }
  };
  right.appendChild(remove);

  line.append(left, right);
  return line;
}

export async function renderHomebrew(): Promise<View> {
  const packages = await installedPackages();

  const content = document.createElement("div");
  const note = document.createElement("div");
  note.className = "note plain";

  const install = document.createElement("button");
  install.className = "small-btn";
  install.textContent = "Install a package";
  install.onclick = async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "PS3 package", extensions: ["pkg"] }],
    });
    if (typeof picked !== "string") return;

    install.disabled = true;
    install.textContent = "Installing…";
    note.textContent = "This takes a few seconds.";
    try {
      await installPackage(picked);
      note.textContent = "";
      store.redraw();
    } catch (err) {
      note.textContent = typeof err === "string" ? err : "Couldn't install that package.";
    } finally {
      install.disabled = false;
      install.textContent = "Install a package";
    }
  };

  if (packages.length === 0) {
    const empty = emptyState(
      "Nothing installed",
      "Pick a .pkg file to put it on the emulated console. Homebrew, demos and add-ons all install this way."
    );
    install.style.marginTop = "14px";
    empty.append(install, note);
    content.appendChild(empty);
    return { title: "Homebrew", subtitle: "Nothing installed", content };
  }

  const list = document.createElement("div");
  packages.forEach((one) => list.appendChild(row(one, note)));

  const bar = document.createElement("div");
  bar.className = "row-actions";
  bar.style.marginTop = "14px";
  bar.append(install, note);

  content.append(list, bar);

  const total = packages.reduce((sum, one) => sum + one.size_bytes, 0);
  const subtitle = `${packages.length} installed${total > 0 ? ` · ${formatSize(total)}` : ""}`;
  return { title: "Homebrew", subtitle, content };
}
