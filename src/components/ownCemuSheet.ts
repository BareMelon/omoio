import { bringOwnCemu, lookAtOwnCemu, replaceWithOwnSave, type OwnCemu, type OwnCemuSave } from "../api";

function plural(n: number, one: string, many: string): string {
  return n === 1 ? `1 ${one}` : `${n} ${many}`;
}

/// Cemu shows a title id with a dash between its halves.
function shownId(titleId: string): string {
  return `${titleId.slice(0, 8)}-${titleId.slice(8)}`.toUpperCase();
}

const STATE: Record<OwnCemuSave["state"], [string, string]> = {
  new: ["To copy", "status go"],
  same: ["Already here", "status"],
  differs: ["Omoio has its own", "status warn"],
};

/// What the user's own Cemu has, and a way to copy it into Omoio's. Their
/// Cemu is only read. A game Omoio's Cemu already has a save for is left
/// alone unless they ask for theirs in its place, one game at a time.
export async function openOwnCemu(folder: string, onChanged: () => void): Promise<void> {
  let found: OwnCemu;
  try {
    found = await lookAtOwnCemu(folder);
  } catch (err) {
    throw typeof err === "string" ? err : "Couldn't read that folder.";
  }

  const scrim = document.createElement("div");
  scrim.className = "scrim";
  const sheet = document.createElement("div");
  sheet.className = "sheet wide";
  scrim.appendChild(sheet);
  document.body.appendChild(scrim);
  requestAnimationFrame(() => scrim.classList.add("on"));

  let busy = false;
  function close() {
    if (busy) return;
    scrim.classList.remove("on");
    setTimeout(() => scrim.remove(), 200);
  }
  scrim.onclick = (e) => {
    if (e.target === scrim) close();
  };

  const head = document.createElement("div");
  head.innerHTML = `
    <div class="sheet-h">Bring over your Cemu</div>
    <div class="sheet-p"></div>
  `;
  head.querySelector<HTMLElement>(".sheet-p")!.textContent =
    "Omoio runs a Cemu of its own, apart from yours. This copies your keys and saved games into Omoio's, once. Your Cemu is only read, and what you change in it afterwards stays there.";
  sheet.appendChild(head);

  const list = document.createElement("div");
  list.className = "settings-scroll";
  sheet.appendChild(list);

  const foot = document.createElement("div");
  foot.className = "sheet-foot";
  const note = document.createElement("span");
  note.className = "cfg-v";
  const actions = document.createElement("div");
  actions.className = "sheet-actions";
  const shut = document.createElement("button");
  shut.className = "btn ghost";
  shut.textContent = "Close";
  shut.onclick = close;
  const bring = document.createElement("button");
  bring.className = "btn solid";
  bring.textContent = "Bring over";
  actions.append(shut, bring);
  foot.append(note, actions);
  sheet.appendChild(foot);

  function row(title: string, detail: string, right: HTMLElement[]): HTMLElement {
    const line = document.createElement("div");
    line.className = "setting";
    const left = document.createElement("div");
    const k = document.createElement("div");
    k.className = "setting-k";
    k.textContent = title;
    const path = document.createElement("div");
    path.className = "setting-path";
    path.textContent = detail;
    left.append(k, path);
    const side = document.createElement("div");
    side.className = "row-actions";
    side.append(...right);
    line.append(left, side);
    return line;
  }

  function pill(text: string, className: string): HTMLElement {
    const tag = document.createElement("span");
    tag.className = className;
    tag.textContent = text;
    return tag;
  }

  function saveRow(save: OwnCemuSave): HTMLElement {
    const [said, className] = STATE[save.state];
    const right: HTMLElement[] = [pill(said, className)];
    if (save.state === "differs") {
      const use = document.createElement("button");
      use.className = "small-btn";
      use.textContent = "Use yours";
      use.onclick = async () => {
        // Replacing a save is the one thing here that overwrites anything, so
        // it asks first and the button says what it will do.
        if (use.textContent === "Use yours") {
          use.textContent = "Replace Omoio's save?";
          use.classList.add("danger");
          return;
        }
        busy = true;
        use.disabled = true;
        use.textContent = "Replacing…";
        try {
          await replaceWithOwnSave(folder, save.title_id);
          busy = false;
          onChanged();
          await show("Your save is in place. Omoio's was moved to the replaced-saves folder beside its Cemu.");
        } catch (err) {
          busy = false;
          await show(typeof err === "string" ? err : "Couldn't replace that save.");
        }
      };
      right.push(use);
    }
    return row(save.name ?? `Wii U game ${shownId(save.title_id)}`, shownId(save.title_id), right);
  }

  async function show(notice?: string) {
    if (notice) {
      try {
        found = await lookAtOwnCemu(folder);
      } catch (err) {
        notice = typeof err === "string" ? err : "Couldn't read that folder any more.";
      }
    }
    list.textContent = "";
    if (notice) {
      const told = document.createElement("div");
      told.className = "notice";
      told.style.marginBottom = "14px";
      told.textContent = notice;
      list.appendChild(told);
    }

    list.appendChild(
      row(
        "Keys",
        found.keys_problem ??
          (found.keys === 0 ? "Nothing new for Omoio's Cemu" : `${plural(found.keys, "key", "keys")} to add`),
        [pill(found.keys > 0 ? "To copy" : "Nothing new", found.keys > 0 ? "status go" : "status")]
      )
    );

    if (found.saves.length === 0) {
      const none = document.createElement("div");
      none.className = "sheet-p";
      none.textContent = "No saved games in this Cemu.";
      list.appendChild(none);
    }
    for (const save of found.saves) list.appendChild(saveRow(save));

    const fresh = found.saves.filter((s) => s.state === "new").length;
    const theirs = found.saves.filter((s) => s.state === "differs").length;
    bring.disabled = found.keys === 0 && fresh === 0;
    const parts = [`From ${found.folder}`];
    if (theirs > 0) parts.push(`${plural(theirs, "game", "games")} kept as Omoio has them`);
    note.textContent = parts.join(" · ");
  }

  bring.onclick = async () => {
    busy = true;
    bring.disabled = true;
    bring.textContent = "Copying…";
    try {
      const done = await bringOwnCemu(folder);
      busy = false;
      onChanged();
      const copied = [];
      if (done.keys > 0) copied.push(plural(done.keys, "key", "keys"));
      if (done.saves > 0) copied.push(plural(done.saves, "save", "saves"));
      await show(copied.length > 0 ? `Copied ${copied.join(" and ")}.` : "Nothing new to copy.");
    } catch (err) {
      busy = false;
      await show(typeof err === "string" ? err : "Couldn't copy from your Cemu.");
    } finally {
      bring.textContent = "Bring over";
    }
  };

  await show();
}
