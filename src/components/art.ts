/// The two colours of a game's generated tile. Every title id gives the same
/// pair every time, so a game without art is still recognisable rather than a
/// grey box, and stays the same wherever it appears.
export function tint(titleId: string): { back: string; front: string } {
  let hash = 0;
  for (const ch of titleId) hash = (hash * 31 + ch.charCodeAt(0)) & 0xffff;
  const hue = hash % 360;
  return { back: `hsl(${hue} 32% 14%)`, front: `hsl(${(hue + 40) % 360} 58% 52%)` };
}

/// A tile for a game with no picture of its own, wide like the pictures games
/// do have, so every tile is the same shape. Shared by the library, the
/// catalogue and Big Picture so one game never looks like two.
///
/// The initials sit inside the ring, clear of anything drawn over the tile.
export function placeholderArt(titleId: string, name: string): string {
  const { back, front } = tint(titleId);
  const initials = name.replace(/[^A-Za-z0-9]/g, "").slice(0, 2).toUpperCase();
  return `
    <svg viewBox="0 0 160 90" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      <rect width="160" height="90" fill="${back}"/>
      <circle cx="80" cy="45" r="25" fill="none" stroke="${front}" stroke-width="2.2" opacity=".8"/>
      <text x="80" y="52.5" text-anchor="middle" fill="${front}"
            font-family="system-ui" font-size="20" font-weight="700">${initials}</text>
    </svg>`;
}

/// Narrower than this, a picture filling a 16:9 tile would lose too much of
/// itself. A Wii U game's icon is square; a PS3 icon, a Wii U boot picture
/// and RAWG's art are all wider and still fill the tile.
const NARROWEST_FILLING = 4 / 3;

/// Shows each cover in `root` that is narrower than its tile whole, centred
/// on a blurred, darkened copy of itself, rather than cut down to a strip.
/// A picture's shape is known only once it has loaded, so it is decided then.
export function fitCovers(root: ParentNode): void {
  for (const img of root.querySelectorAll<HTMLImageElement>("img.cover")) {
    const fit = () => {
      if (img.classList.contains("whole")) return;
      if (!img.naturalWidth || img.naturalWidth / img.naturalHeight >= NARROWEST_FILLING) return;
      const behind = img.cloneNode() as HTMLImageElement;
      behind.className = "cover-behind";
      img.classList.add("whole");
      img.before(behind);
    };
    if (img.complete) fit();
    else img.addEventListener("load", fit, { once: true });
  }
}
