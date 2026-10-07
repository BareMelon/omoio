import { invoke } from "@tauri-apps/api/core";

interface PlatformInfo {
  os: string;
  embedded_games: boolean;
}

export const platform = await invoke<PlatformInfo>("platform_info").catch(() => ({
  os: "unknown",
  embedded_games: false,
}));
