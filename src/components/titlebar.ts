import { getCurrentWindow } from "@tauri-apps/api/window";

export function renderTitlebar(): HTMLElement {
  const bar = document.createElement("div");
  bar.className = "titlebar";
  bar.setAttribute("data-tauri-drag-region", "");
  bar.innerHTML = `
    <div class="wordmark">
      <svg viewBox="0 0 1254 1254" fill="#126BFC" fill-rule="evenodd">
        <path d="M342 222l-60 26-40 27-16 27-6 24v610l6 24 16 27 40 27 61 26-14-24-9-30-1-30V282l1-30 9-30z"/>
        <path d="M396 205l-30 29-10 40v686l10 40 30 29 56 12h439l56-12 45-29 30-40 11-40v-82h-49l-25-5-16-21v-95l16-21 25-5h49V404h-52l-25-5-16-21v-95l16-21 25-5h52v-77l-11-40-30-40-45-29-56-12H452zM575 429h20l33 12 249 147 22 26 8 30v17l-8 30-22 26-249 147-33 12h-20l-33-12-22-26-8-30V475l8-30 22-26z"/>
      </svg>
      Omoio
    </div>
    <div class="tb-spacer"></div>
    <button class="tb-btn" id="tb-minimize" aria-label="Minimize">
      <svg viewBox="0 0 10 10"><path d="M0 5h10" stroke="currentColor" stroke-width="1.2"/></svg>
    </button>
    <button class="tb-btn" id="tb-maximize" aria-label="Maximize">
      <svg viewBox="0 0 10 10"><rect x="1" y="1" width="8" height="8" fill="none" stroke="currentColor" stroke-width="1.2"/></svg>
    </button>
    <button class="tb-btn close" id="tb-close" aria-label="Close">
      <svg viewBox="0 0 10 10"><path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" stroke-width="1.2"/></svg>
    </button>
  `;

  const win = getCurrentWindow();
  bar.querySelector<HTMLButtonElement>("#tb-minimize")!.onclick = () => win.minimize();
  bar.querySelector<HTMLButtonElement>("#tb-maximize")!.onclick = () => win.toggleMaximize();
  bar.querySelector<HTMLButtonElement>("#tb-close")!.onclick = () => win.close();

  return bar;
}
