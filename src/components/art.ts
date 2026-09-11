/// A tile for a game with no picture of its own.
///
/// Every title id gives the same colours every time, so a game without art is
/// still recognisable rather than a grey box, and stays the same tile wherever
/// it appears. Shared between the library and the catalogue so one game does
/// not look like two.
///
/// The initials sit inside the ring rather than below it: the game's own name
/// is drawn across the bottom of the tile, and anything down there collides
/// with it as soon as the name runs to two lines.
export function placeholderArt(titleId: string, name: string): string {
  let hash = 0;
  for (const ch of titleId) hash = (hash * 31 + ch.charCodeAt(0)) & 0xffff;
  const hue = hash % 360;
  const back = `hsl(${hue} 32% 14%)`;
  const front = `hsl(${(hue + 40) % 360} 58% 52%)`;
  const initials = name.replace(/[^A-Za-z0-9]/g, "").slice(0, 2).toUpperCase();
  return `
    <svg viewBox="0 0 100 120" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      <rect width="100" height="120" fill="${back}"/>
      <circle cx="50" cy="52" r="26" fill="none" stroke="${front}" stroke-width="2.5" opacity=".8"/>
      <text x="50" y="60" text-anchor="middle" fill="${front}"
            font-family="system-ui" font-size="21" font-weight="700">${initials}</text>
    </svg>`;
}
