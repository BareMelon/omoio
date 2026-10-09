# macOS support in this fork

This fork adds experimental **macOS 15 or newer** support to
[Bertrram/omoio](https://github.com/Bertrram/omoio), with separate Apple Silicon
(`aarch64`) and Intel (`x86_64`) builds. The minimum follows the official RPCS3
Mac bundles currently shipped upstream. Emulator compatibility and performance
still depend on your hardware and the game.

## Install

Open the [Mac build workflow](https://github.com/BareMelon/omoio/actions/workflows/macos.yml).
Choose a successful run, then download `omoio-macos-aarch64` for an M-series Mac
or `omoio-macos-x86_64` for an Intel Mac. GitHub requires sign-in to download
workflow artifacts. The artifact contains a disk image and a zipped app.

Open the `.dmg`, drag **Omoio.app** to Applications, eject the disk image, and
launch Omoio. Alternatively, unpack the application ZIP and move Omoio.app to
Applications. These builds are ad-hoc signed, without an Apple Developer ID or
notarization. If Gatekeeper blocks the app, macOS may offer **Open Anyway** in
System Settings > Privacy & Security after the first attempt. Only approve the
fork build you intended to download.

On first start, select RPCS3, Cemu and/or Dolphin. Omoio downloads the official Mac
application bundles, verifies SHA-256 before extraction, and installs them into
its own data directory. RPCS3 is native to the selected architecture. Cemu 2.6
ships an Intel executable and needs Apple's **Rosetta** on Apple Silicon;
Omoio checks for it and reports a useful error if it is missing. Install Rosetta
through macOS before installing Cemu. Omoio does not accept its license for you.

PS3 firmware must still be downloaded from Sony's page and selected by you.
Games and console keys are not included. Cemu's Mac port is experimental
upstream; expect some game-specific and graphics limitations.

Dolphin 2609a uses the official **universal Apple Silicon/Intel** application.
Wii/GameCube imports, native launching, save backup/restore, SDL controller
profiles, compatibility entries and bundled patches/graphics mods are integrated.
For Skylanders, use Dolphin's own **Tools > Emulated USB Devices > Skylanders
Portal** menu. Its main window stays available for Skylanders games.

## Features and limits

- Game imports, library management, compatibility lists, community packs, game
  settings, PS3 firmware/package installation and PS3 save backups use the
  existing backends with Mac-specific paths.
- Games run in separate emulator windows. Starting from Big Picture or enabling
  start-in-fullscreen requests the emulator's own fullscreen mode. Stop asks
  Dolphin's exact running application to quit first, allowing it to flush saves;
  a process group is ended if it fails to quit. Omoio records the session log.
- Controller discovery and mappings use SDL names and GUIDs. SDL is compiled
  into the Mac app, so users do not need Homebrew or a separate SDL install.
  Physical USB/Bluetooth controllers still need testing on your Mac.
- Big Picture works as the focused launcher. The Windows in-game View+Menu
  switch, embedded game window, input suppression and automated Skylanders
  portal menu are unavailable. Use the emulator's own portal tools.
- The upstream Skylanders picture extractor is Windows-only.
- CPU and RAM detection works; automatic GPU memory/display tuning remains
  Windows-only.
- Install new fork packages manually. Launcher self-updates are disabled;
  emulator updates remain available.

## Private emulator data

Omoio's data root is `~/Library/Application Support/Omoio`:

| Purpose | Directory under the data root |
| --- | --- |
| RPCS3 application | `rpcs3/RPCS3.app` |
| RPCS3 firmware, virtual disk, settings and saves | `rpcs3/home/Library/Application Support/rpcs3` |
| RPCS3 cache and log | `rpcs3/home/Library/Caches/rpcs3` |
| Cemu application | `cemu/Cemu.app` |
| Cemu settings, keys, saves, packs and log | `cemu/home/Library/Application Support/Cemu` |
| Cemu cache | `cemu/home/Library/Caches/Cemu` |
| Dolphin application | `dolphin/Dolphin.app` |
| Dolphin settings, saves, logs and packs | `dolphin/User` |

RPCS3 and Cemu receive a private home directory for that subprocess. Cemu also
receives `CFFIXED_USER_HOME` for Cocoa's standard-path lookup. Updating an app
bundle preserves these separate data directories. Omoio's own home directory
and your shell environment are unchanged. Dolphin receives its private data
directory through its native `--user` option.

## Build from source

Use macOS 15+, Xcode Command Line Tools, Node.js 22+, stable Rust and CMake.
Build on the architecture you intend to distribute; Cemu needs Rosetta for the
Apple Silicon integration check. No Apple signing credentials are required.

```bash
npm ci
npm run tauri icon Images/Icon/omoio-icon-1024.png
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo run --locked --manifest-path src-tauri/Cargo.toml --example macos_smoke
npm run tauri build
```

The integration example installs real emulators into Omoio's data directory.
Run it on a disposable test account or machine. For development use
`npm run tauri dev`. Tauri automatically merges `src-tauri/tauri.macos.conf.json`.
Packages appear under `src-tauri/target/release/bundle/macos` and `bundle/dmg`.

## Verification scope

The Mac workflow runs Rust tests on Intel and Apple Silicon, exercises Omoio's
real emulator installers, checks Cemu startup and its private data location,
builds application and disk-image packages, verifies the app signature, and
checks packaged desktop startup. The official RPCS3 executable version command
is exercised on Apple Silicon.

The Dolphin check starts its native GUI, checks shipped
packs, verifies its private data directory, and requires graceful shutdown.
The official macOS DMG does not include DolphinTool; automatic disc extraction
for figure pictures is unavailable on macOS.
These checks use no commercial games, console firmware or physical controllers.

The initial official Intel RPCS3 probe aborted with SIGABRT on GitHub's hosted
Intel runner during startup, after MoltenVK initialization output. Its cause has
not been established. The Intel workflow verifies the installed RPCS3 bundle
and version metadata but does **not** claim a working Intel RPCS3 runtime.
Actual games, PS3 firmware installation, physical controllers, Gatekeeper after
a browser download, and Intel RPCS3 on a physical Mac still need manual testing.

## Upstream references

- [Dolphin's official downloads](https://dolphin-emu.org/download/), version
  2609a, `dolphin-2609a-universal.dmg`, SHA-256:
  `9a810043538f21b53cf8f8f747b1672df74895d3d6eb30940d96042823f20ecb`.

- [RPCS3 Apple Silicon releases](https://github.com/RPCS3/rpcs3-binaries-mac-arm64/releases)
  and [Intel releases](https://github.com/RPCS3/rpcs3-binaries-mac/releases).
  Omoio requires the GitHub release asset's SHA-256 digest.
- RPCS3 `Utilities/File.cpp` defines its Mac home-relative data and cache paths;
  the app's Info.plist supplies the version without initializing graphics.
- [Cemu 2.6](https://github.com/cemu-project/Cemu/releases/tag/v2.6), asset
  `cemu-2.6-macos-12-x64.dmg`, SHA-256 measured from the official download:
  `698c4b298f94983e4d6c30e9687ba8ff05094dd3930837c5104cddc0b0a49e4e`.
- Cemu v2.6 `src/gui/CemuApp.cpp` and `src/Cemu/Logging/CemuLogging.cpp`
  define its Mac paths and game log; default game profiles live in the
  application bundle's `Contents/SharedSupport/gameProfiles/default` directory
  (Cemu v2.6 `src/CMakeLists.txt`).
