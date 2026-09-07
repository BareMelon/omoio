import {
  finishSetup,
  getAccount,
  listRegions,
  needsSetup,
  setRegion,
  setUsername,
  type RegionChoice,
} from "../api";

/// Asked once, on the first run. Both answers stay changeable in Settings, and
/// the sheet says so rather than making the choice feel final.
export async function openSetupIfNeeded(): Promise<void> {
  if (!(await needsSetup())) return;

  const [account, regions] = await Promise.all([getAccount(), listRegions()]);

  const scrim = document.createElement("div");
  scrim.className = "scrim";
  const sheet = document.createElement("div");
  sheet.className = "sheet";
  scrim.appendChild(sheet);
  document.body.appendChild(scrim);
  requestAnimationFrame(() => scrim.classList.add("on"));

  sheet.innerHTML = `
    <div class="sheet-h">Before you play</div>
    <div class="sheet-p">Both can be changed later in Settings.</div>

    <div class="setting">
      <div>
        <div class="setting-k">Username</div>
        <div class="setting-hint">The name games show for you.</div>
      </div>
      <div class="row-actions">
        <input class="text-in" id="setup-name" maxlength="16" spellcheck="false">
      </div>
    </div>

    <div class="setting">
      <div>
        <div class="setting-k">Region</div>
        <div class="setting-hint">Sets the language games start in.</div>
      </div>
      <div class="row-actions">
        <select class="select" id="setup-region"></select>
      </div>
    </div>

    <div class="note plain" id="setup-note"></div>
    <div class="sheet-actions">
      <button class="btn solid" id="setup-done">Save and continue</button>
    </div>
  `;

  const name = sheet.querySelector<HTMLInputElement>("#setup-name")!;
  name.value = account.username || "User";

  const region = sheet.querySelector<HTMLSelectElement>("#setup-region")!;
  for (const choice of regions as RegionChoice[]) {
    const option = document.createElement("option");
    option.value = choice.id;
    option.textContent = `${choice.name} · ${choice.language}`;
    option.selected = choice.id === account.region;
    region.appendChild(option);
  }
  // Nothing matched, so start somewhere rather than on a blank.
  if (!account.region) region.value = "eu-en";

  const note = sheet.querySelector<HTMLElement>("#setup-note")!;
  const done = sheet.querySelector<HTMLButtonElement>("#setup-done")!;

  await new Promise<void>((finished) => {
    done.onclick = async () => {
      done.disabled = true;
      try {
        await setUsername(name.value);
        await setRegion(region.value);
        await finishSetup();
        scrim.classList.remove("on");
        setTimeout(() => scrim.remove(), 200);
        finished();
      } catch (err) {
        note.textContent = typeof err === "string" ? err : "Couldn't save that.";
        done.disabled = false;
      }
    };
  });
}
