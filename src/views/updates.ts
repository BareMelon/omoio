import { pendingUpdates, refreshCompatibility, type PendingUpdate } from "../api";
import { store } from "../state";
import { emptyState, type View } from "./view";

function updateRow(update: PendingUpdate): HTMLElement {
  const row = document.createElement("div");
  row.className = "setting";

  const left = document.createElement("div");
  const name = document.createElement("div");
  name.className = "setting-k";
  name.textContent = update.title;
  const detail = document.createElement("div");
  detail.className = "setting-path";
  detail.textContent = `${update.title_id} · ${update.installed} to ${update.newest}`;
  left.append(name, detail);

  const right = document.createElement("div");
  right.className = "row-actions";
  const open = document.createElement("button");
  open.className = "small-btn";
  open.textContent = "Open the game";
  // Installing happens in the game's own panel, where its saves and settings
  // are, rather than in a second place that does the same thing differently.
  open.onclick = () => {
    store.setView("library");
    store.setSelected(update.title_id);
  };
  right.appendChild(open);

  row.append(left, right);
  return row;
}

export async function renderUpdates(): Promise<View> {
  const [haveList, waiting] = await pendingUpdates();

  if (!haveList) {
    const content = emptyState(
      "Nothing checked yet",
      "Omoio checks your games against the list RPCS3 publishes. Get it once and this works offline."
    );
    const get = document.createElement("button");
    get.className = "small-btn";
    get.textContent = "Get the list";
    get.onclick = async () => {
      get.disabled = true;
      get.textContent = "Getting…";
      try {
        await refreshCompatibility();
        store.setView("updates");
      } catch {
        get.textContent = "Couldn't get it";
        get.disabled = false;
      }
    };
    content.appendChild(get);
    return { title: "Updates", subtitle: "No list yet", content };
  }

  if (waiting.length === 0) {
    return {
      title: "Updates",
      subtitle: "Nothing waiting",
      content: emptyState(
        "Everything is up to date",
        "None of your games has a newer version published."
      ),
    };
  }

  const content = document.createElement("div");
  content.className = "hw";
  for (const update of waiting) {
    content.appendChild(updateRow(update));
  }

  return {
    title: "Updates",
    subtitle: waiting.length === 1 ? "1 game" : `${waiting.length} games`,
    content,
  };
}
