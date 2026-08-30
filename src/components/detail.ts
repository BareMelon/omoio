// Structural shell for the detail panel. No game data exists yet to show
// here; later steps fill d-body and call a show() once a game is selected.
export function renderDetail(): HTMLElement {
  const detail = document.createElement("aside");
  detail.className = "detail hidden";
  detail.innerHTML = `
    <div class="d-hero">
      <div class="d-art"></div>
      <button class="d-close" aria-label="Close">
        <svg viewBox="0 0 10 10"><path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" stroke-width="1.4"/></svg>
      </button>
    </div>
    <div class="d-body"></div>
  `;
  detail.querySelector<HTMLButtonElement>(".d-close")!.onclick = () => {
    detail.classList.add("hidden");
  };
  return detail;
}
