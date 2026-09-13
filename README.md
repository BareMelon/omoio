# Omoio

Omoio puts all your console games in one library and plays them without you ever
touching an emulator. Import your dumps, press Play, and Omoio sets up the
emulator, the controller and the picture for you. It's made for big collections
spread across systems, the kind that sits on a few drives in a home lab.

![The catalogue, searching for Grand Theft Auto V](design/screenshots/catalogue.png)

**Very early development.** PS3 (RPCS3) and Wii U (Cemu) work today, and eight
more are planned: PS4, PS2, GameCube and Wii, PSP, PS1, Game Boy Advance, PS Vita
and DS. Things will break, and feedback is really appreciated, so open an issue.

## What you get

- One library for every system. Import a folder, a zip or 7z, or scan a whole
  drive at once.
- Games play inside Omoio's window, fullscreen with F11. You never see the
  emulator.
- Emulators installed and kept up to date for you.
- One controller layout for every emulator, set up by pressing buttons, for up to
  four players.
- A catalogue of about 4,500 PS3 and Wii U games showing how well each one runs.
- PS3 updates, patches and save backups, and emulator settings for each game.
- A Skylanders portal menu for Wii U games that you use from the controller.
- Real cover art from RAWG if you want it, with a free key. Otherwise Omoio draws
  its own tiles.

## Install

Download `Omoio_x.y.z_x64-setup.exe` from Releases and run it. It doesn't need
admin rights. On first start it asks which emulators you want. PS3 games also
need Sony's free firmware: Omoio opens the download page and installs the file
you pick.

The installer isn't code signed yet, so Windows SmartScreen may warn you. Click
More info, then Run anyway.

## The small print

Omoio doesn't come with games and won't help you find any, so bring your own
dumps of games you own. It doesn't crack anything: Wii U disc images need your own
keys file, added on the Emulators screen. RPCS3 and Cemu are separate projects,
run as their official builds, and all credit for the emulation goes to them.

## Build

Needs Rust, Node 20 or newer, and the Tauri prerequisites for Windows.

```
npm install
npm run tauri dev      # run locally
npm run tauri build    # build the installer
```

## Licence

Not decided yet.
