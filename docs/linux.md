# Linux support in this fork

This is a Linux port of [Bertrram/omoio](https://github.com/Bertrram/omoio).
The supported package target is **x86_64 Ubuntu 24.04 or a compatible newer
distribution**. Other distributions may work with the matching WebKitGTK 4.1,
GTK 3, SDL 2.30 and udev runtime libraries, but are not covered by CI.

## Install

Open the fork's [Linux build workflow](https://github.com/BareMelon/omoio/actions/workflows/linux.yml).
Download `omoio-linux-x86_64` from a successful run and unpack it. GitHub requires
sign-in for workflow artifact downloads.

On Ubuntu, install the `.deb` with `sudo apt install ./Omoio_*.deb`. This installs
the desktop launcher and its runtime dependencies. Alternatively, make the
`.AppImage` executable with `chmod +x Omoio_*.AppImage` and run it. If your system
has no FUSE, run the launcher with `--appimage-extract-and-run`.

On first start, choose RPCS3, Cemu and/or Dolphin. Omoio installs RPCS3 and Cemu's **official native
Linux AppImages**, verifies SHA-256, and extracts them once. Emulator launches
therefore do not need FUSE and do not use Wine. PS3 firmware still comes from
Sony's page and must be selected by you.

Dolphin 2609a runs Wii and GameCube games using its **official x86_64 Flatpak**.
The Debian package depends on Flatpak; AppImage users must install `flatpak`
through their distribution first. The first Dolphin installation also downloads
the KDE runtime and graphics extensions, so it is larger and slower than the
emulator download alone. Omoio creates a private Flatpak installation under
`Omoio/dolphin/flatpak` and passes an explicit `--user` directory for Dolphin's
settings and saves. It does not update your separately installed Dolphin.

Dolphin currently requires X11 or XWayland (as its official Qt build does).
It launches in a separate window. Wii/GameCube imports, save backup/restore,
SDL controller profiles, compatibility entries and bundled patches/graphics
mods are integrated. For Skylanders, use **Tools > Emulated USB Devices >
Skylanders Portal** in Dolphin; Omoio's automated portal overlay is Windows-only.

## Behavior and limits

- Game imports, the library, compatibility lists, community packs, game settings,
  PS3 firmware/package installs and PS3 save backups use the existing backends.
- Games open in separate emulator windows on Linux. Use the emulator's own
  fullscreen shortcut. Omoio's start-in-fullscreen option and games launched
  from Big Picture request fullscreen when starting the emulator.
- Stop first asks Dolphin's native window to close so it can flush saves.
  Other emulators, or a Dolphin that fails to close, are stopped as a process group.
  Omoio reaps child processes,
  detects normal exits, and keeps session logs.
- Big Picture works as a controller-driven library while Omoio is focused.
  The Windows in-game View+Menu switch, embedded game window, input suppression,
  and automated Skylanders portal menu are unavailable on Linux. Use the
  emulator's own portal tools instead; Skylanders launches with RPCS3's GUI.
- Linux controllers use SDL names and GUIDs, including Xbox controllers. Four
  disconnected player placeholders use RPCS3's Null handler rather than XInput.
  Controller hardware still needs practical testing on your machine; Linux
  permissions, USB/Bluetooth connections and emulator SDL versions can differ.
- The upstream Skylanders picture extractor has a Windows executable only.
  This fork reports that limitation instead of downloading and running it on Linux.
- CPU and RAM information works on Linux. GPU memory and display autodetection
  currently remain Windows-only, so Linux does not automatically tune RPCS3's
  render scale.
- Launcher self-updates are disabled in this fork. Install new fork packages
  manually; emulator updates still work. No upstream signing key is used.
- ARM Linux is not supported. For Apple Silicon and Intel Macs, see the
  separate [macOS guide](macos.md).

## Private emulator data

Under `${XDG_DATA_HOME:-$HOME/.local/share}/Omoio`:

| Purpose | Directory |
| --- | --- |
| Library, covers, controller layouts and session logs | Omoio data root |
| RPCS3 firmware, virtual disk, settings and saves | `rpcs3/xdg-config/rpcs3` |
| RPCS3 cache and log | `rpcs3/xdg-cache/rpcs3` |
| Cemu settings, keys, saves, packs and cache | `cemu/xdg/Cemu` |
| Extracted emulator programs | each emulator's `AppDir` |

Each emulator receives private XDG directories. These do not replace the XDG
variables of Omoio or the shell and do not alter an existing RPCS3 or Cemu install.
Replacing the extracted program directory preserves the emulator's data.

## Build from source

Install Node.js 22+, stable Rust, and the Linux dependencies:

```bash
sudo apt update
sudo apt install build-essential curl file pkg-config libwebkit2gtk-4.1-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf libssl-dev \
  libudev-dev libsdl2-dev
npm ci
npm run tauri icon Images/Icon/omoio-icon-1024.png
npm run tauri dev
```

To test and package:

```bash
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

Tauri automatically merges `src-tauri/tauri.linux.conf.json`. Packages appear in
`src-tauri/target/release/bundle/deb` and `bundle/appimage`. No update signing
secret is needed.

CI runs native Rust tests, installs and checks all three official Linux emulator
binaries through Omoio's installer code, builds both packages and checks that
the desktop process stays running under Xvfb. This is build/startup evidence, not a completed
playthrough with games, physical controllers or a Wayland desktop. Dolphin's
check starts its GUI and command-line tool, verifies bundled packs and private
storage, and requires a graceful GUI shutdown rather than counting a forced kill
as success.

## Source references

- [Dolphin's official downloads](https://dolphin-emu.org/download/), version
  2609a, `dolphin-2609a-x86_64.flatpak`, SHA-256:
  `dcedfff449e8367944960dd61161b459f6163314c45c917cf75440514800d21a`.
  [Flatpak documents `FLATPAK_USER_DIR`](https://docs.flatpak.org/en/latest/flatpak-command-reference.html)
  for the isolated installation. Runtime packages come from signed Flathub repositories.

- RPCS3 Linux builds: `RPCS3/rpcs3-binaries-linux` releases, checked against
  GitHub's asset `digest` before extracting or executing.
- RPCS3 paths: `Utilities/File.cpp` (`get_config_dir`, `get_log_dir`) in RPCS3.
- Cemu v2.6 AppImage SHA-256:
  `0c20c4aeb800bb13d9bab9474ef45a6f8fcde6402cad9b32ac2a1bbd03186313`,
  from the official release's GitHub asset digest on 7 October 2026.
- Cemu paths: `src/gui/CemuApp.cpp`, tag v2.6.
- Cemu SDL identifiers and driver hints: `src/input/api/SDL/SDLControllerProvider.cpp`,
  tag v2.6. A controller profile names the actual SDL GUID and its ordinal.
