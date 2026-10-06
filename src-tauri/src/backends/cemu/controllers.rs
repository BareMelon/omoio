//! Cemu's side of the controller layout: one profile file per player.
//!
//! A fresh Cemu has no controller for anyone, so a game starts and nothing
//! answers. Cemu reads one file per player, `controllerProfiles/controller<N>.xml`
//! in its config folder (`InputManager::load` in Cemu v2.6), and Omoio writes
//! them from the layout it keeps (core/pad_layout.rs):
//!
//! - Player 1 is the Wii U GamePad, which most games expect to be there, but a
//!   Pro Controller in the few games that need one (`PRO_FIRST`).
//! - Players 2 to 4 are Wii U Pro Controllers, which multiplayer games take.
//!
//! Each is an XInput pad, named by its slot. Cemu's XInput pad is known by the
//! slot number alone, so a file can name a slot before a pad is in it. A
//! player on any other kind of pad gets none in Cemu for now: Cemu knows those
//! by an SDL GUID that has not been checked against a real pad. Cemu reads
//! XInput without the Guide button, so Home is not offered.
//!
//! Cemu keeps one layout for every game. A game's own layout is RPCS3's alone.

use crate::core::pad_layout::{Player, PLAYERS};
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Games whose first player has to be a Wii U Pro Controller rather than the
/// GamePad until they have saved once, by their title ids. Skylanders Trap
/// Team asks for a language the first time it starts, and that screen answers
/// the GamePad's touch screen and a Pro Controller's buttons but never the
/// GamePad's buttons, so a pad standing in for the GamePad was stuck on the
/// flags. As a Pro Controller the same pad picked a flag and went on through
/// the logos, the save slots and into the opening film (European disc, 5
/// October 2026; 000500001017c600 is the American one). Only until then:
/// Cemu turns the stick into the presses menus move by for the GamePad but
/// not for a Pro Controller (`VPADController.cpp` against
/// `WPADController::KPADRead`, v2.6), so the stick moved nothing in the
/// menus.
pub const PRO_FIRST: [&str; 2] = ["0005000010181f00", "000500001017c600"];

/// Player 1's file as each kind of controller, kept beside the others so the
/// one a game needs can become `controller0.xml` as the game starts.
const FIRST_AS_GAMEPAD: &str = "Player 1 GamePad.xml";
const FIRST_AS_PRO: &str = "Player 1 Pro Controller.xml";

/// The Wii U's buttons with the place each sits, in the order of the
/// GamePad's `ButtonId`, which counts from 1. They sit where Nintendo's pads
/// have them, as Cemu's own XInput layout puts them: A on the right, B at the
/// bottom, X at the top, Y on the left. Wii U games are made for those places,
/// so jumping and attacking land on the bottom and left buttons of any pad.
/// Going by the letters instead put Swap Force's jump on Xbox B and its
/// attack on Xbox Y (13 September 2026). The price is that menus confirm with
/// the right-hand button, as they do on a Wii U.
pub const WII_U: [(&str, &str); 24] = [
    ("East", "A"),
    ("South", "B"),
    ("North", "X"),
    ("West", "Y"),
    ("LB", "L"),
    ("RB", "R"),
    ("LT", "ZL"),
    ("RT", "ZR"),
    ("Start", "Plus"),
    ("Back", "Minus"),
    ("Up", "D-pad up"),
    ("Down", "D-pad down"),
    ("Left", "D-pad left"),
    ("Right", "D-pad right"),
    ("LS", "Left stick press"),
    ("RS", "Right stick press"),
    ("LS Y+", "Left stick up"),
    ("LS Y-", "Left stick down"),
    ("LS X-", "Left stick left"),
    ("LS X+", "Left stick right"),
    ("RS Y+", "Right stick up"),
    ("RS Y-", "Right stick down"),
    ("RS X-", "Right stick left"),
    ("RS X+", "Right stick right"),
];

/// Cemu's number for an input on an XInput pad, its `Buttons2` in
/// `Controller.h`. Buttons are the bits of `XINPUT_GAMEPAD.wButtons`; the
/// sticks and triggers come after Cemu's 32 buttons and its own trigger and
/// d-pad entries, positive directions first. Guide has none, since Cemu never
/// reads it.
fn xinput_number(input: &str) -> Option<u64> {
    Some(match input {
        "Up" => 0,
        "Down" => 1,
        "Left" => 2,
        "Right" => 3,
        "Start" => 4,
        "Back" => 5,
        "LS" => 6,
        "RS" => 7,
        "LB" => 8,
        "RB" => 9,
        "South" => 12,
        "East" => 13,
        "West" => 14,
        "North" => 15,
        "LS X+" => 38,
        "LS Y+" => 39,
        "RS X+" => 40,
        "RS Y+" => 41,
        "LT" => 42,
        "RT" => 43,
        "LS X-" => 44,
        "LS Y-" => 45,
        "RS X-" => 46,
        "RS Y-" => 47,
        _ => return None,
    })
}

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

/// The XInput slot a player's pad is in, counted from zero, or `None` for a
/// pad that is not read through XInput.
fn xinput_slot(player: &Player) -> Option<u32> {
    if player.pad.handler != "XInput" {
        return None;
    }
    player
        .pad
        .device
        .strip_prefix("XInput Pad #")?
        .parse::<u32>()
        .ok()
        .filter(|number| (1..=4).contains(number))
        .map(|number| number - 1)
}

/// A player's file, as the `kind` of controller. A player Cemu cannot read
/// the pad of still gets the file, with no pad in it, so one left from before
/// does not keep driving them.
fn profile(kind: Kind, player: &Player) -> String {
    let controller = xinput_slot(player)
        .map(|slot| {
            let entries: String = WII_U
                .iter()
                .enumerate()
                .filter_map(|(at, (place, _))| {
                    let number = xinput_number(player.input(place))?;
                    Some(format!(
                        "\t\t\t<entry>\n\t\t\t\t<mapping>{}</mapping>\n\t\t\t\t<button>{number}</button>\n\t\t\t</entry>\n",
                        kind.button_id(at)
                    ))
                })
                .collect();
            format!(
                "\t<controller>\n\
                 \t\t<api>XInput</api>\n\
                 \t\t<uuid>{slot}</uuid>\n\
                 \t\t<display_name>Controller {}</display_name>\n\
                 \t\t<mappings>\n\
                 {entries}\
                 \t\t</mappings>\n\
                 \t</controller>\n",
                slot + 1
            )
        })
        .unwrap_or_default();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <emulated_controller>\n\
         \t<type>{}</type>\n\
         {controller}\
         </emulated_controller>\n",
        kind.cemu_name()
    )
}

/// One file per player, counted from 0 as Cemu names them, and player 1's
/// again as both kinds of controller.
fn write_all(dir: &Path, players: &[Player]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for (index, player) in players.iter().enumerate().take(PLAYERS) {
        std::fs::write(dir.join(format!("controller{index}.xml")), profile(Kind::for_player(index), player))?;
    }
    if let Some(first) = players.first() {
        std::fs::write(dir.join(FIRST_AS_GAMEPAD), profile(Kind::GamePad, first))?;
        std::fs::write(dir.join(FIRST_AS_PRO), profile(Kind::Pro, first))?;
    }
    Ok(())
}

/// Whether the game with this title id has saved in Cemu, which puts its
/// first start behind it.
pub fn has_saved(app: &AppHandle, title_id: &str) -> bool {
    super::install_dir(app).is_ok_and(|dir| saved_in(&dir.join("portable").join("mlc01"), title_id))
}

/// Whether there is any file in the game's save folder under `mlc`, which
/// Cemu keeps by the two halves of the title id.
fn saved_in(mlc: &Path, title_id: &str) -> bool {
    if title_id.len() != 16 {
        return false;
    }
    let (high, low) = title_id.split_at(8);
    has_file(&mlc.join("usr").join("save").join(high).join(low).join("user"))
}

fn has_file(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let path = entry.path();
            path.is_file() || (path.is_dir() && has_file(&path))
        })
    })
}

/// Makes player 1 the kind of controller the game about to start needs: a
/// Pro Controller for one in `PRO_FIRST` that hasn't saved yet, the GamePad
/// otherwise. Cemu reads `controller0.xml` as the game starts, and each start
/// sets it again.
pub fn first_player(app: &AppHandle, pro: bool) -> Result<(), String> {
    let dir = profile_dir(app)?;
    let from = dir.join(if pro { FIRST_AS_PRO } else { FIRST_AS_GAMEPAD });
    if !from.is_file() {
        return Ok(());
    }
    std::fs::copy(from, dir.join("controller0.xml"))
        .map(|_| ())
        .map_err(|_| "Couldn't save the controller settings for Cemu.".to_string())
}

fn profile_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?.join("portable").join("controllerProfiles"))
}

/// Writes the layout for every game where Cemu reads it. Nothing is written
/// before Cemu is installed, and a game's own layout is left to RPCS3.
pub fn write(app: &AppHandle, title_id: &str, players: &[Player]) -> Result<(), String> {
    if !title_id.is_empty() || !super::install_dir(app)?.join("Cemu.exe").is_file() {
        return Ok(());
    }
    write_all(&profile_dir(app)?, players)
        .map_err(|_| "Couldn't save the controller settings for Cemu.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pad_layout::Pad;
    use std::collections::BTreeMap;

    fn xinput(slot: u32) -> Pad {
        Pad {
            device: format!("XInput Pad #{slot}"),
            name: format!("Controller {slot}"),
            handler: "XInput".to_string(),
            family: "xbox".to_string(),
        }
    }

    fn id_of(kind: Kind, name: &str) -> u64 {
        let at = WII_U.iter().position(|(_, n)| *n == name).unwrap();
        kind.button_id(at)
    }

    #[test]
    fn button_numbers_follow_each_controllers_own_list() {
        assert_eq!(id_of(Kind::GamePad, "A"), 1);
        assert_eq!(id_of(Kind::GamePad, "Minus"), 10);
        assert_eq!(id_of(Kind::GamePad, "D-pad up"), 11);
        assert_eq!(id_of(Kind::GamePad, "Right stick right"), 24);
        assert_eq!(id_of(Kind::Pro, "Minus"), 10);
        assert_eq!(id_of(Kind::Pro, "D-pad up"), 12, "Home sits before it");
        assert_eq!(id_of(Kind::Pro, "Right stick right"), 25);
    }

    #[test]
    fn player_one_is_the_gamepad_and_the_rest_are_pro_controllers() {
        let one = profile(Kind::for_player(0), &Player::on(xinput(1)));
        assert!(one.contains("<type>Wii U GamePad</type>"));
        assert!(one.contains("<api>XInput</api>"));
        assert!(one.contains("<uuid>0</uuid>"));
        assert!(one.contains("<mapping>1</mapping>\n\t\t\t\t<button>13</button>"), "A is the right button");
        assert!(one.contains("<mapping>2</mapping>\n\t\t\t\t<button>12</button>"), "B, which games jump with, is the bottom one");

        let two = profile(Kind::for_player(1), &Player::on(xinput(2)));
        assert!(two.contains("<type>Wii U Pro Controller</type>"));
        assert!(two.contains("<uuid>1</uuid>"));
        assert!(two.contains("<display_name>Controller 2</display_name>"));
        assert_eq!(two.matches("<entry>").count(), 24);
    }

    #[test]
    fn a_changed_layout_moves_the_button() {
        let buttons = BTreeMap::from([
            ("East".to_string(), "South".to_string()),
            ("South".to_string(), "East".to_string()),
        ]);
        let text = profile(Kind::for_player(0), &Player::with_buttons(xinput(1), buttons));
        assert!(text.contains("<mapping>1</mapping>\n\t\t\t\t<button>12</button>"), "A now on the bottom button");
    }

    #[test]
    fn guide_is_never_written_since_cemu_cannot_read_it() {
        let buttons = BTreeMap::from([("Start".to_string(), "Guide".to_string())]);
        let text = profile(Kind::for_player(1), &Player::with_buttons(xinput(2), buttons));
        assert_eq!(text.matches("<entry>").count(), 23);
    }

    #[test]
    fn a_pad_cemu_cannot_read_leaves_the_player_without_one() {
        let pad = Pad {
            device: "DualSense Wireless Controller 0".to_string(),
            name: "DualSense Wireless Controller".to_string(),
            handler: "SDL".to_string(),
            family: "playstation".to_string(),
        };
        let text = profile(Kind::for_player(1), &Player::on(pad));
        assert!(text.contains("<type>Wii U Pro Controller</type>"));
        assert!(!text.contains("<controller>"));
    }

    #[test]
    fn a_game_has_saved_once_its_save_folder_holds_a_file() {
        let mlc = std::env::temp_dir().join(format!("omoio-cemu-mlc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&mlc);
        let user = mlc.join("usr").join("save").join("00050000").join("10181f00").join("user");
        assert!(!saved_in(&mlc, "0005000010181f00"));
        std::fs::create_dir_all(user.join("80000001")).unwrap();
        assert!(!saved_in(&mlc, "0005000010181f00"), "an empty folder is no save");
        std::fs::write(user.join("80000001").join("Save_Header"), b"x").unwrap();
        assert!(saved_in(&mlc, "0005000010181f00"));
        assert!(!saved_in(&mlc, "000500001017c600"));
        assert!(!saved_in(&mlc, "short"));
        let _ = std::fs::remove_dir_all(&mlc);
    }

    #[test]
    fn every_player_gets_a_file_and_player_one_a_pro_controller_too() {
        let dir = std::env::temp_dir().join(format!("omoio-cemu-pads-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let players: Vec<Player> = (1..=4).map(|slot| Player::on(xinput(slot))).collect();
        write_all(&dir, &players).unwrap();
        for index in 0..PLAYERS {
            assert!(dir.join(format!("controller{index}.xml")).is_file());
        }
        let pro = std::fs::read_to_string(dir.join(FIRST_AS_PRO)).unwrap();
        assert!(pro.contains("<type>Wii U Pro Controller</type>"));
        assert!(pro.contains("<uuid>0</uuid>"), "player 1's own pad");
        assert!(pro.contains("<mapping>12</mapping>\n\t\t\t\t<button>0</button>"), "D-pad up after Home: {pro}");
        let gamepad = std::fs::read_to_string(dir.join(FIRST_AS_GAMEPAD)).unwrap();
        assert_eq!(gamepad, std::fs::read_to_string(dir.join("controller0.xml")).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
