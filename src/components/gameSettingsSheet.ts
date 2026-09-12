import {
  gameSettings,
  setGameSettings,
  type ChosenSettings,
  type GameOption,
} from "../api";

const GROUP_TITLES: Record<string, string> = {
  Video: "Picture",
  Core: "Processor",
  Audio: "Sound",
};

/// Every control offers "RPCS3 default" and starts there. Choosing it removes
/// the setting rather than writing a value, so Omoio never states a preference
/// the user did not express.
const DEFAULT = "";

function control(
  option: GameOption,
  current: string,
  onChange: (value: string) => void
): HTMLElement {
  if (option.kind === "switch" || option.kind === "choice") {
    const select = document.createElement("select");
    select.className = "select";
    const values: [string, string][] =
      option.kind === "switch"
        ? [[DEFAULT, "RPCS3 default"], ["true", "On"], ["false", "Off"]]
        : [[DEFAULT, "RPCS3 default"], ...option.choices.map((c): [string, string] => [c, c])];
    for (const [value, label] of values) {
      const opt = document.createElement("option");
      opt.value = value;
      opt.textContent = label;
      opt.selected = value === current;
      select.appendChild(opt);
    }
    select.onchange = () => onChange(select.value);
    return select;
  }

  const input = document.createElement("input");
  input.className = option.kind === "number" ? "number" : "text-in";
  input.value = current;
  // RPCS3's own value, so leaving the field empty is visibly the same as
  // not setting it.
  input.placeholder = option.default === "" ? "default" : option.default;

  if (option.kind === "number") {
    input.type = "number";
    const bounded = option.max > option.min;
    if (bounded) {
      input.min = String(option.min);
      input.max = String(option.max);
    }
    input.onchange = () => {
      if (input.value === "") return onChange(DEFAULT);
      if (bounded) {
        input.value = String(Math.min(option.max, Math.max(option.min, Number(input.value))));
      }
      onChange(input.value);
    };
    return input;
  }

  input.type = "text";
  input.spellcheck = false;
  input.onchange = () => onChange(input.value.trim());
  return input;
}

function settingRow(
  option: GameOption,
  chosen: ChosenSettings,
  onChanged: () => void,
  showKey: boolean,
  reasons: Record<string, string>
): HTMLElement {
  const row = document.createElement("div");
  row.className = "setting";

  const left = document.createElement("div");
  const name = document.createElement("div");
  name.className = "setting-k";
  name.textContent = option.label;
  left.appendChild(name);

  // Where we have renamed a setting, its RPCS3 name is shown underneath,
  // because that is the name in any advice you have been given. Where we have
  // not, the name above is already RPCS3's own and repeating it says nothing.
  if (showKey && option.label !== option.name) {
    const path = document.createElement("div");
    path.className = "setting-path";
    path.textContent = option.key.split("\n").join(" / ");
    left.appendChild(path);
  }
  if (option.hint) {
    const hint = document.createElement("div");
    hint.className = "setting-hint";
    hint.textContent = option.hint;
    left.appendChild(hint);
  }
  const why = reasons[option.key];
  if (why) {
    const set = document.createElement("div");
    set.className = "setting-hint";
    set.textContent = `Omoio sets this for this game. ${why}`;
    left.appendChild(set);
  }

  const right = document.createElement("div");
  right.className = "row-actions";
  right.appendChild(
    control(option, chosen[option.key] ?? DEFAULT, (value) => {
      if (value === DEFAULT) delete chosen[option.key];
      else chosen[option.key] = value;
      row.classList.toggle("changed", option.key in chosen);
      onChanged();
    })
  );

  row.classList.toggle("changed", option.key in chosen);
  row.append(left, right);
  return row;
}

function groupInto(
  pane: HTMLElement,
  options: GameOption[],
  chosen: ChosenSettings,
  onChanged: () => void,
  showKeys: boolean,
  reasons: Record<string, string>
): { group: HTMLElement; rows: { row: HTMLElement; text: string }[] }[] {
  const built: { group: HTMLElement; rows: { row: HTMLElement; text: string }[] }[] = [];
  let current = "";
  let section: HTMLElement | null = null;
  let rows: { row: HTMLElement; text: string }[] = [];

  for (const option of options) {
    if (option.group !== current || !section) {
      current = option.group;
      section = document.createElement("div");
      section.className = "sec";
      const heading = document.createElement("div");
      heading.className = "sec-h";
      heading.textContent = showKeys
        ? current
        : GROUP_TITLES[current] ?? current;
      section.appendChild(heading);
      pane.appendChild(section);
      rows = [];
      built.push({ group: section, rows });
    }
    const row = settingRow(option, chosen, onChanged, showKeys, reasons);
    section.appendChild(row);
    rows.push({
      row,
      text: `${option.group} ${option.name} ${option.label}`.toLowerCase(),
    });
  }
  return built;
}

export async function openGameSettings(
  titleId: string,
  title: string,
  onSaved: () => void
): Promise<void> {
  const [options, saved, reasons] = await gameSettings(titleId);
  // Worked on as a copy, so Cancel really does leave things as they were.
  const chosen: ChosenSettings = { ...saved };

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
    <div class="sheet-h">Emulator settings</div>
    <div class="sheet-p"></div>
  `;
  head.querySelector<HTMLElement>(".sheet-p")!.textContent =
    `Only for ${title}. Anything left on default is left to RPCS3.`;
  sheet.appendChild(head);

  const changedCount = document.createElement("span");
  changedCount.className = "cfg-v";
  function refreshCount() {
    const n = Object.keys(chosen).length;
    changedCount.textContent = n === 0 ? "Nothing changed" : `${n} changed`;
  }

  const tabs = document.createElement("div");
  tabs.className = "tabs";
  const commonTab = document.createElement("button");
  commonTab.className = "tab on";
  commonTab.textContent = "Common";
  const advancedTab = document.createElement("button");
  advancedTab.className = "tab";
  advancedTab.textContent = `Advanced (${options.length})`;
  tabs.append(commonTab, advancedTab);
  sheet.appendChild(tabs);

  const commonPane = document.createElement("div");
  commonPane.className = "settings-scroll";
  const advancedPane = document.createElement("div");
  advancedPane.className = "settings-scroll gone";

  const search = document.createElement("input");
  search.type = "search";
  search.className = "text-in search";
  search.placeholder = "Search all settings";
  search.spellcheck = false;
  const searchWrap = document.createElement("div");
  searchWrap.className = "sheet-search gone";
  searchWrap.appendChild(search);
  sheet.append(searchWrap, commonPane, advancedPane);

  if (options.length === 0) {
    const empty = document.createElement("div");
    empty.className = "sheet-p";
    empty.textContent =
      "Start a game once and the full list of settings appears here.";
    commonPane.appendChild(empty);
  }

  // RPCS3 lists the processor first. Picture is what people come here to
  // change, so Common leads with it.
  const common = ["Video", "Core", "Audio"].flatMap((group) =>
    options.filter((o) => o.common && o.group === group)
  );
  groupInto(commonPane, common, chosen, refreshCount, false, reasons);
  const advanced = groupInto(advancedPane, options, chosen, refreshCount, true, reasons);

  const noMatch = document.createElement("div");
  noMatch.className = "sheet-p gone";
  noMatch.textContent = "No setting by that name.";
  advancedPane.appendChild(noMatch);

  search.oninput = () => {
    const q = search.value.trim().toLowerCase();
    let hits = 0;
    for (const { group, rows } of advanced) {
      let shown = 0;
      for (const { row, text } of rows) {
        const match = q === "" || text.includes(q);
        row.classList.toggle("gone", !match);
        if (match) shown += 1;
      }
      group.classList.toggle("gone", shown === 0);
      hits += shown;
    }
    noMatch.classList.toggle("gone", hits > 0);
  };

  function showTab(advancedOn: boolean) {
    commonTab.classList.toggle("on", !advancedOn);
    advancedTab.classList.toggle("on", advancedOn);
    commonPane.classList.toggle("gone", advancedOn);
    advancedPane.classList.toggle("gone", !advancedOn);
    searchWrap.classList.toggle("gone", !advancedOn);
    if (advancedOn) search.focus();
  }
  commonTab.onclick = () => showTab(false);
  advancedTab.onclick = () => showTab(true);

  const actions = document.createElement("div");
  actions.className = "sheet-actions";
  const reset = document.createElement("button");
  reset.className = "btn ghost";
  reset.textContent = "Reset all";
  reset.onclick = async () => {
    await setGameSettings(titleId, {});
    onSaved();
    close();
  };
  const cancel = document.createElement("button");
  cancel.className = "btn ghost";
  cancel.textContent = "Cancel";
  cancel.onclick = close;
  const save = document.createElement("button");
  save.className = "btn solid";
  save.textContent = "Save";
  save.onclick = async () => {
    save.disabled = true;
    await setGameSettings(titleId, chosen);
    onSaved();
    close();
  };

  const foot = document.createElement("div");
  foot.className = "sheet-foot";
  foot.append(changedCount, actions);
  actions.append(reset, cancel, save);
  sheet.appendChild(foot);

  refreshCount();
}
