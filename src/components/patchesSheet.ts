import {
  gamePatches,
  refreshPatches,
  setPatchEnabled,
  type Patch,
} from "../api";

function toggle(on: boolean): HTMLButtonElement {
  const button = document.createElement("button");
  button.className = on ? "switch on" : "switch";
  button.setAttribute("role", "switch");
  button.setAttribute("aria-checked", String(on));
  const dot = document.createElement("span");
  dot.className = "switch-dot";
  button.appendChild(dot);
  return button;
}

function patchRow(
  patch: Patch,
  titleId: string,
  onChanged: () => void
): HTMLElement {
  const row = document.createElement("div");
  row.className = patch.applies ? "setting" : "setting spare";

  const left = document.createElement("div");
  const name = document.createElement("div");
  name.className = "setting-k";
  name.textContent = patch.name;
  left.appendChild(name);

  if (patch.fix) {
    const why = document.createElement("div");
    why.className = "setting-hint";
    why.textContent = `Omoio turns this on for this game. ${patch.fix}`;
    left.appendChild(why);
  }

  const by = [patch.author && `by ${patch.author}`, patch.version && `v${patch.version}`]
    .filter(Boolean)
    .join(" · ");
  if (by) {
    const line = document.createElement("div");
    line.className = "setting-path";
    line.textContent = by;
    left.appendChild(line);
  }

  if (patch.notes) {
    const notes = document.createElement("div");
    notes.className = "setting-hint";
    notes.textContent = patch.notes;
    left.appendChild(notes);
  }

  // A patch written for another release of the game would not be applied, so
  // say which one it wants rather than offering a switch that does nothing.
  if (!patch.applies) {
    const needs = document.createElement("div");
    needs.className = "setting-hint warn";
    needs.textContent = `Written for version ${patch.versions.join(", ")}.`;
    left.appendChild(needs);
  }

  const right = document.createElement("div");
  right.className = "row-actions";
  if (patch.applies) {
    const control = toggle(patch.enabled);
    control.onclick = async () => {
      const next = !patch.enabled;
      control.disabled = true;
      try {
        await setPatchEnabled(patch, titleId, next);
        patch.enabled = next;
        control.className = next ? "switch on" : "switch";
        control.setAttribute("aria-checked", String(next));
        onChanged();
      } finally {
        control.disabled = false;
      }
    };
    right.appendChild(control);
  }

  row.append(left, right);
  return row;
}

export async function openPatches(
  titleId: string,
  title: string,
  onChanged: () => void
): Promise<void> {
  const scrim = document.createElement("div");
  scrim.className = "scrim";
  const sheet = document.createElement("div");
  sheet.className = "sheet wide";
  scrim.appendChild(sheet);
  document.body.appendChild(scrim);
  requestAnimationFrame(() => scrim.classList.add("on"));

  function close() {
    scrim.classList.remove("on");
    setTimeout(() => scrim.remove(), 200);
  }
  scrim.onclick = (e) => {
    if (e.target === scrim) close();
  };

  const head = document.createElement("div");
  head.innerHTML = `
    <div class="sheet-h">Patches</div>
    <div class="sheet-p"></div>
  `;
  head.querySelector<HTMLElement>(".sheet-p")!.textContent =
    `Written by the RPCS3 community for ${title}. Omoio turns on a fix it knows this game needs and says why. The rest stay off until you turn them on.`;
  sheet.appendChild(head);

  const body = document.createElement("div");
  body.className = "settings-scroll";
  sheet.appendChild(body);

  const foot = document.createElement("div");
  foot.className = "sheet-foot";
  const note = document.createElement("span");
  note.className = "cfg-v";
  const actions = document.createElement("div");
  actions.className = "sheet-actions";
  const get = document.createElement("button");
  get.className = "btn ghost";
  const done = document.createElement("button");
  done.className = "btn solid";
  done.textContent = "Done";
  done.onclick = close;
  actions.append(get, done);
  foot.append(note, actions);
  sheet.appendChild(foot);

  async function show() {
    body.textContent = "";
    const { have_list, patches } = await gamePatches(titleId);
    get.textContent = have_list ? "Get the latest" : "Get patches";

    if (!have_list) {
      const empty = document.createElement("div");
      empty.className = "sheet-p";
      empty.textContent = "Get the patch list to see what has been written for this game.";
      body.appendChild(empty);
      note.textContent = "No patch list yet";
      return;
    }

    if (patches.length === 0) {
      const empty = document.createElement("div");
      empty.className = "sheet-p";
      empty.textContent = "Nobody has published a patch for this game.";
      body.appendChild(empty);
      note.textContent = "None for this game";
      return;
    }

    const fits = patches.filter((p) => p.applies).length;
    const on = patches.filter((p) => p.enabled).length;
    note.textContent = on === 0 ? `${fits} available` : `${on} of ${fits} on`;

    for (const patch of patches) {
      body.appendChild(patchRow(patch, titleId, () => {
        void show();
        onChanged();
      }));
    }
  }

  get.onclick = async () => {
    get.disabled = true;
    const was = get.textContent;
    get.textContent = "Getting…";
    try {
      await refreshPatches();
      await show();
    } catch (err) {
      note.textContent = typeof err === "string" ? err : "Couldn't get the patch list.";
      get.textContent = was;
    } finally {
      get.disabled = false;
    }
  };

  await show();
}
