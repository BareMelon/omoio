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

On first start, choose RPCS3 and/or Cemu. Omoio installs their **official native
Linux AppImages**, verifies SHA-256, and extracts them once. Emulator launches
therefore do not need FUSE and do not use Wine. PS3 firmware still comes from
Sony's page and must be selected by you.

## Behavior and limits

- Game imports, the library, compatibility lists, community packs, game settings,
  PS3 firmware/package installs and PS3 save backups use the existing backends.
- Games open in separate emulator windows on Linux. Use the emulator's own
  fullscreen shortcut. Omoio's start-in-fullscreen option and games launched
  from Big Picture request fullscreen when starting the emulator.
- Stop terminates the emulator's process group. Omoio reaps child processes,
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
- ARM Linux and macOS are not supported by this port.

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

CI runs native Rust tests, installs and checks both official Linux emulator
binaries through Omoio's installer code, builds both packages and checks that
the desktop process stays running under Xvfb. This is build/startup evidence, not a completed
playthrough with games, physical controllers or a Wayland desktop.

## Source references

- RPCS3 Linux builds: `RPCS3/rpcs3-binaries-linux` releases, checked against
  GitHub's asset `digest` before extracting or executing.
- RPCS3 paths: `Utilities/File.cpp` (`get_config_dir`, `get_log_dir`) in RPCS3.
- Cemu v2.6 AppImage SHA-256:
  `0c20c4aeb800bb13d9bab9474ef45a6f8fcde6402cad9b32ac2a1bbd03186313`,
  from the official release's GitHub asset digest on 7 October 2026.
- Cemu paths: `src/gui/CemuApp.cpp`, tag v2.6.
- Cemu SDL identifiers and driver hints: `src/input/api/SDL/SDLControllerProvider.cpp`,
  tag v2.6. A controller profile names the actual SDL GUID and its ordinal.
