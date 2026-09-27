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
