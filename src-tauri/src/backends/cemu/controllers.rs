//! Making pads work in Cemu without anyone opening its input settings.
//!
//! A fresh Cemu has no controller for anyone, so a game starts and nothing
//! answers. Cemu reads one file per player, `controllerProfiles/controller<N>.xml`
//! in its config folder (`InputManager::load` in Cemu v2.6), and Omoio writes
//! them before a game starts:
//!
//! - Player 1 is the Wii U GamePad, which most games expect to be there.
//! - Players 2 to 4 are Wii U Pro Controllers, which multiplayer games take.
//!
//! Each is an XInput pad, named by its slot. Cemu's XInput pad is known by the
//! slot number alone, so a file can name a slot before a pad is in it, and a
//! second pad plugged in is player 2 without anything being written again.
//!
//! A file that is already there is left alone, so a layout someone set up in
//! Cemu's own window is never overwritten.

use std::path::{Path, PathBuf};
use tauri::AppHandle;

const PLAYERS: usize = 4;

/// Cemu's `Buttons2` for an XInput pad. The buttons are the bits of
/// `XINPUT_GAMEPAD.wButtons`; the sticks and triggers come after Cemu's 32
/// buttons and its own trigger and d-pad entries, positive directions first.
mod xinput {
    pub const DPAD_UP: u64 = 0;
    pub const DPAD_DOWN: u64 = 1;
    pub const DPAD_LEFT: u64 = 2;
    pub const DPAD_RIGHT: u64 = 3;
    pub const START: u64 = 4;
    pub const BACK: u64 = 5;
    pub const LEFT_THUMB: u64 = 6;
    pub const RIGHT_THUMB: u64 = 7;
    pub const LEFT_SHOULDER: u64 = 8;
    pub const RIGHT_SHOULDER: u64 = 9;
    pub const A: u64 = 12;
    pub const B: u64 = 13;
    pub const X: u64 = 14;
    pub const Y: u64 = 15;
    pub const LS_RIGHT: u64 = 38;
    pub const LS_UP: u64 = 39;
    pub const RS_RIGHT: u64 = 40;
    pub const RS_UP: u64 = 41;
    pub const LT: u64 = 42;
    pub const RT: u64 = 43;
    pub const LS_LEFT: u64 = 44;
    pub const LS_DOWN: u64 = 45;
    pub const RS_LEFT: u64 = 46;
    pub const RS_DOWN: u64 = 47;
}

/// Cemu's own layout for an XInput pad, the same for the GamePad and the Pro
/// Controller (`VPADController.cpp`, `ProController.cpp`). Nintendo's A is the
/// right-hand face button, where it sits on their pads, so it is Xbox B. In
/// the order of the GamePad's `ButtonId`, which counts from 1.
const LAYOUT: [(&str, u64); 24] = [
    ("A", xinput::B),
    ("B", xinput::A),
    ("X", xinput::Y),
    ("Y", xinput::X),
    ("L", xinput::LEFT_SHOULDER),
    ("R", xinput::RIGHT_SHOULDER),
    ("ZL", xinput::LT),
    ("ZR", xinput::RT),
    ("Plus", xinput::START),
    ("Minus", xinput::BACK),
    ("Up", xinput::DPAD_UP),
    ("Down", xinput::DPAD_DOWN),
    ("Left", xinput::DPAD_LEFT),
    ("Right", xinput::DPAD_RIGHT),
    ("Left stick press", xinput::LEFT_THUMB),
    ("Right stick press", xinput::RIGHT_THUMB),
    ("Left stick up", xinput::LS_UP),
    ("Left stick down", xinput::LS_DOWN),
    ("Left stick left", xinput::LS_LEFT),
    ("Left stick right", xinput::LS_RIGHT),
    ("Right stick up", xinput::RS_UP),
    ("Right stick down", xinput::RS_DOWN),
    ("Right stick left", xinput::RS_LEFT),
    ("Right stick right", xinput::RS_RIGHT),
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    GamePad,
    Pro,
}

impl Kind {
    fn for_player(player: usize) -> Kind {
        if player == 0 {
            Kind::GamePad
        } else {
            Kind::Pro
        }
    }

    /// As `EmulatedController::type_from_string` reads it.
    fn cemu_name(self) -> &'static str {
        match self {
            Kind::GamePad => "Wii U GamePad",
            Kind::Pro => "Wii U Pro Controller",
        }
    }

    /// The button's number in this controller's `ButtonId`. The two lists
    /// match except that the Pro Controller has Home straight after Minus, so
    /// from Up onwards its numbers are one higher.
    fn button_id(self, at: usize) -> u64 {
        let id = at as u64 + 1;
        match self {
            Kind::Pro if id > 10 => id + 1,
            _ => id,
        }
    }
}

/// Player `player`'s file, counted from 0 as Cemu names them.
fn profile(player: usize) -> String {
    let kind = Kind::for_player(player);
    let entries: String = LAYOUT
        .iter()
        .enumerate()
        .map(|(at, (_, input))| {
            format!(
                "\t\t\t<entry>\n\t\t\t\t<mapping>{}</mapping>\n\t\t\t\t<button>{input}</button>\n\t\t\t</entry>\n",
                kind.button_id(at)
            )
        })
        .collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <emulated_controller>\n\
         \t<type>{}</type>\n\
         \t<controller>\n\
         \t\t<api>XInput</api>\n\
         \t\t<uuid>{player}</uuid>\n\
         \t\t<display_name>Controller {}</display_name>\n\
         \t\t<mappings>\n\
         {entries}\
         \t\t</mappings>\n\
         \t</controller>\n\
         </emulated_controller>\n",
        kind.cemu_name(),
        player + 1
    )
}

fn profile_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?.join("portable").join("controllerProfiles"))
}

fn write_missing(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for player in 0..PLAYERS {
        let path = dir.join(format!("controller{player}.xml"));
        if !path.exists() {
            std::fs::write(&path, profile(player))?;
        }
    }
    Ok(())
}

/// Called before a game starts, so plugging in and pressing Play is enough.
pub fn set_up_if_needed(app: &AppHandle) {
    if let Ok(dir) = profile_dir(app) {
        let _ = write_missing(&dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id_of(kind: Kind, name: &str) -> u64 {
        let at = LAYOUT.iter().position(|(n, _)| *n == name).unwrap();
        kind.button_id(at)
    }

    #[test]
    fn button_numbers_follow_each_controllers_own_list() {
        assert_eq!(id_of(Kind::GamePad, "A"), 1);
        assert_eq!(id_of(Kind::GamePad, "Minus"), 10);
        assert_eq!(id_of(Kind::GamePad, "Up"), 11);
        assert_eq!(id_of(Kind::GamePad, "Right stick right"), 24);
        assert_eq!(id_of(Kind::Pro, "Minus"), 10);
        assert_eq!(id_of(Kind::Pro, "Up"), 12, "Home sits before it");
        assert_eq!(id_of(Kind::Pro, "Right stick right"), 25);
    }

    #[test]
    fn player_one_is_the_gamepad_and_the_rest_are_pro_controllers() {
        let one = profile(0);
        assert!(one.contains("<type>Wii U GamePad</type>"));
        assert!(one.contains("<api>XInput</api>"));
        assert!(one.contains("<uuid>0</uuid>"));
        assert!(one.contains("<mapping>1</mapping>\n\t\t\t\t<button>13</button>"), "A is Xbox B");

        let two = profile(1);
        assert!(two.contains("<type>Wii U Pro Controller</type>"));
        assert!(two.contains("<uuid>1</uuid>"));
        assert!(two.contains("<display_name>Controller 2</display_name>"));
        assert_eq!(two.matches("<entry>").count(), 24);
    }

    #[test]
    fn a_layout_already_there_is_kept() {
        let dir = std::env::temp_dir().join(format!("omoio-cemu-pads-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("controller0.xml"), "mine").unwrap();

        write_missing(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("controller0.xml")).unwrap(), "mine");
        for player in 1..PLAYERS {
            assert!(dir.join(format!("controller{player}.xml")).is_file());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
