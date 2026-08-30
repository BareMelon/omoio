export interface View {
  title: string;
  subtitle: string;
  content: HTMLElement;
}

export function emptyState(title: string, subtitle: string): HTMLElement {
  const el = document.createElement("div");
  el.className = "empty";
  el.innerHTML = `<div class="empty-t"></div><div class="empty-s"></div>`;
  el.querySelector(".empty-t")!.textContent = title;
  el.querySelector(".empty-s")!.textContent = subtitle;
  return el;
}
