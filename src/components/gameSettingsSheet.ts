import {
  gameSettings,
  setGameSettings,
  type ChosenSettings,
  type GameOption,
} from "../api";

const SECTION_TITLES: Record<string, string> = {
  Video: "Picture",
  Core: "Processor",
  Audio: "Sound",
};

/// Every control offers "RPCS3 default" and starts there. Choosing it removes
/// the setting rather than writing a value, so Omoio never states a preference
/// the user did not express.
const DEFAULT = "";

function control(option: GameOption, current: string, onChange: (value: string) => void): HTMLElement {
  if (option.kind === "switch") {
    const select = document.createElement("select");
    select.className = "select";
    for (const [value, label] of [[DEFAULT, "RPCS3 default"], ["true", "On"], ["false", "Off"]]) {
      const opt = document.createElement("option");
      opt.value = value;
      opt.textContent = label;
      opt.selected = value === current;
      select.appendChild(opt);
    }
    select.onchange = () => onChange(select.value);
    return select;
  }

  if (option.kind === "choice") {
    const select = document.createElement("select");
    select.className = "select";
    const blank = document.createElement("option");
    blank.value = DEFAULT;
    blank.textContent = "RPCS3 default";
    blank.selected = current === DEFAULT;
    select.appendChild(blank);
    for (const choice of option.choices) {
      const opt = document.createElement("option");
      opt.value = choice;
      opt.textContent = choice;
      opt.selected = choice === current;
      select.appendChild(opt);
    }
    select.onchange = () => onChange(select.value);
    return select;
  }

  const wrap = document.createElement("div");
  wrap.className = "row-actions";
  const input = document.createElement("input");
  input.type = "number";
  input.className = "number";
  input.min = String(option.min);
  input.max = String(option.max);
  input.placeholder = "default";
  input.value = current;
  input.onchange = () => {
    if (input.value === "") return onChange(DEFAULT);
    const clamped = Math.min(option.max, Math.max(option.min, Number(input.value)));
    input.value = String(clamped);
    onChange(String(clamped));
  };
  wrap.appendChild(input);
  return wrap;
}

export async function openGameSettings(titleId: string, title: string): Promise<void> {
  const [options, saved] = await gameSettings(titleId);
  // Worked on as a copy, so Cancel really does leave things as they were.
  const chosen: ChosenSettings = JSON.parse(JSON.stringify(saved));

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

  const body = document.createElement("div");
  body.className = "settings-scroll";
  sheet.appendChild(body);

  const changedCount = document.createElement("span");
  changedCount.className = "cfg-v";

  function refreshCount() {
    const n = Object.values(chosen).reduce((sum, keys) => sum + Object.keys(keys).length, 0);
    changedCount.textContent = n === 0 ? "Nothing changed" : `${n} changed`;
  }

  for (const sectionName of ["Video", "Core", "Audio"]) {
    const inSection = options.filter((o) => o.section === sectionName);
    if (inSection.length === 0) continue;

    const sec = document.createElement("div");
    sec.className = "sec";
    const heading = document.createElement("div");
    heading.className = "sec-h";
    heading.textContent = SECTION_TITLES[sectionName] ?? sectionName;
    sec.appendChild(heading);

    for (const option of inSection) {
      const current = chosen[option.section]?.[option.key] ?? DEFAULT;
      const row = document.createElement("div");
      row.className = "setting";

      const left = document.createElement("div");
      const name = document.createElement("div");
      name.className = "setting-k";
      name.textContent = option.label;
      const hint = document.createElement("div");
      hint.className = "setting-hint";
      hint.textContent = option.hint;
      left.append(name, hint);

      const right = document.createElement("div");
      right.className = "row-actions";
      right.appendChild(
        control(option, current, (value) => {
          if (value === DEFAULT) {
            delete chosen[option.section]?.[option.key];
            if (chosen[option.section] && Object.keys(chosen[option.section]).length === 0) {
              delete chosen[option.section];
            }
          } else {
            chosen[option.section] ??= {};
            chosen[option.section][option.key] = value;
          }
          refreshCount();
        })
      );

      row.append(left, right);
      sec.appendChild(row);
    }
    body.appendChild(sec);
  }

  const actions = document.createElement("div");
  actions.className = "sheet-actions";
  const reset = document.createElement("button");
  reset.className = "btn ghost";
  reset.textContent = "Reset all";
  reset.onclick = async () => {
    await setGameSettings(titleId, {});
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
    close();
  };

  const foot = document.createElement("div");
  foot.className = "sheet-foot";
  foot.append(changedCount, actions);
  actions.append(reset, cancel, save);
  sheet.appendChild(foot);

  refreshCount();
}
