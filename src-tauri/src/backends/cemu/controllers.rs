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
//! Player 1 is the one every game answers, so they are never left without a
//! pad while an XInput one is plugged in (`stand_in`), and a game does not
//! start with nobody to answer it (`nobody_answers`).
//!
//! Cemu keeps one layout for every game. A game's own layout is RPCS3's alone.

use crate::core::pad_layout::{self, Pad, Player, PLAYERS};
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Games whose first player has to be a Wii U Pro Controller rather than the
/// GamePad, by their title ids. Skylanders Trap Team asks for a language as
/// it starts, every time and not only the first (6 October 2026), and that
/// screen answers the GamePad's touch screen and a Pro Controller's buttons
/// but never the GamePad's buttons, so a pad standing in for the GamePad was
/// stuck on the flags. As a Pro Controller the same pad picked a flag and went
/// on through the logos, the save slots and into the opening film (European
/// disc, 5 October 2026; 000500001017c600 is the American one). Cemu turns
/// the stick into the presses menus move by for the GamePad but not for a
/// Pro Controller (`VPADController.cpp` against `WPADController::KPADRead`,
/// v2.6), so this Pro Controller takes its d-pad from the stick
/// (`dpad_from_stick`). Omoio shows only the TV picture, so nothing of the
/// GamePad's screen is lost.
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

/// The XInput slot a pad is in, counted from zero, or `None` for a pad that
/// is not read through XInput.
fn xinput_slot(pad: &Pad) -> Option<u32> {
    if pad.handler != "XInput" {
        return None;
    }
    pad.device
        .strip_prefix("XInput Pad #")?
        .parse::<u32>()
        .ok()
        .filter(|number| (1..=4).contains(number))
        .map(|number| number - 1)
}

/// The left stick's direction to take a d-pad direction from, for a Pro
/// Controller standing in for the GamePad. Two of Cemu's buttons may share
/// one of the pad's, so the stick still steers as a stick; the pad's own
/// d-pad then does nothing, which costs nothing in Trap Team, where it has no
/// job of its own (darkspyro.net's controls for the Wii U).
fn dpad_from_stick(place: &str) -> &str {
    match place {
        "Up" => "LS Y+",
        "Down" => "LS Y-",
        "Left" => "LS X-",
        "Right" => "LS X+",
        other => other,
    }
}

/// A player's file, as the `kind` of controller, with the d-pad taken from
/// the stick when `stick_dpad`. A player Cemu cannot read the pad of still
/// gets the file, with no pad in it, so one left from before does not keep
/// driving them.
fn profile(kind: Kind, player: &Player, stick_dpad: bool) -> String {
    let controller = xinput_slot(&player.pad)
        .map(|slot| {
            let entries: String = WII_U
                .iter()
                .enumerate()
                .filter_map(|(at, (place, _))| {
                    let place = if stick_dpad { dpad_from_stick(place) } else { place };
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

/// The players as Cemu gets them, with `plugged` the pads plugged in now.
/// Player 1 on a pad Cemu cannot read would leave every game without anyone
/// to answer it, so they play on an XInput pad that is plugged in instead,
/// keeping their own buttons: one no other player has, else another player's,
/// who takes player 1's pad in return, so no pad drives two players. A PS5
/// pad run through DS4Windows is seen twice, by gilrs and as an XInput pad,
/// and a layout made while DS4Windows was off has the PS5 pad as player 1 and
/// its XInput twin, plugged in later, as player 2; this plays it. Only Cemu's
/// files change; the layout Omoio keeps stays as it was chosen, since RPCS3
/// reads the other pads itself.
fn stand_in(players: &[Player], plugged: &[Pad]) -> Vec<Player> {
    let mut players = players.to_vec();
    let Some(first) = players.first() else {
        return players;
    };
    if xinput_slot(&first.pad).is_some() {
        return players;
    }
    let mut xinput = plugged.iter().filter(|pad| xinput_slot(pad).is_some());
    let free = xinput
        .clone()
        .find(|pad| !players.iter().any(|p| p.pad.device == pad.device));
    if let Some(pad) = free.or_else(|| xinput.next()) {
        let buttons = first.buttons.clone();
        pad_layout::give(&mut players, 0, Player { pad: pad.clone(), buttons });
    }
    players
}

/// Why a game started now would have nobody answering it: player 1's file,
/// `controller0.xml` in `dir`, names no pad. `first` is player 1 as Omoio
/// keeps them, for the pad's name. `None` when there is a pad.
fn nobody_answers(dir: &Path, first: Option<&Player>) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("controller0.xml")).unwrap_or_default();
    if text.contains("<controller>") {
        return None;
    }
    let whose = match first {
        Some(player) if player.pad.handler != "XInput" => {
            format!("Player 1's controller, {}, doesn't work in Wii U games yet.", player.pad.name)
        }
        _ => "Player 1 has no controller in Wii U games.".to_string(),
    };
    Some(format!(
        "{whose} Plug in an Xbox controller, or one that works as one, or choose another for player 1 on the Controller screen, then press Play again."
    ))
}

/// Says why player 1 would have no pad in the game about to start, after
/// `first_player` has set their file. `None` when they have one.
pub fn missing_first_player(app: &AppHandle) -> Option<String> {
    let dir = profile_dir(app).ok()?;
    let players = crate::controllers::current(app, "", &crate::pads::connected()).players;
    nobody_answers(&dir, players.first())
}

/// One file per player, counted from 0 as Cemu names them, and player 1's
/// again as both kinds of controller.
fn write_all(dir: &Path, players: &[Player]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for (index, player) in players.iter().enumerate().take(PLAYERS) {
        std::fs::write(dir.join(format!("controller{index}.xml")), profile(Kind::for_player(index), player, false))?;
    }
    if let Some(first) = players.first() {
        std::fs::write(dir.join(FIRST_AS_GAMEPAD), profile(Kind::GamePad, first, false))?;
        std::fs::write(dir.join(FIRST_AS_PRO), profile(Kind::Pro, first, true))?;
    }
    Ok(())
}

/// Makes player 1 the kind of controller the game about to start needs: a
/// Pro Controller for one in `PRO_FIRST`, the GamePad for any other. Cemu
/// reads `controller0.xml` as the game starts, and each start sets it again.
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
    write_all(&profile_dir(app)?, &stand_in(players, &crate::pads::connected()))
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
        let one = profile(Kind::for_player(0), &Player::on(xinput(1)), false);
        assert!(one.contains("<type>Wii U GamePad</type>"));
        assert!(one.contains("<api>XInput</api>"));
        assert!(one.contains("<uuid>0</uuid>"));
        assert!(one.contains("<mapping>1</mapping>\n\t\t\t\t<button>13</button>"), "A is the right button");
        assert!(one.contains("<mapping>2</mapping>\n\t\t\t\t<button>12</button>"), "B, which games jump with, is the bottom one");

        let two = profile(Kind::for_player(1), &Player::on(xinput(2)), false);
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
        let text = profile(Kind::for_player(0), &Player::with_buttons(xinput(1), buttons), false);
        assert!(text.contains("<mapping>1</mapping>\n\t\t\t\t<button>12</button>"), "A now on the bottom button");
    }

    #[test]
    fn guide_is_never_written_since_cemu_cannot_read_it() {
        let buttons = BTreeMap::from([("Start".to_string(), "Guide".to_string())]);
        let text = profile(Kind::for_player(1), &Player::with_buttons(xinput(2), buttons), false);
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
        let text = profile(Kind::for_player(1), &Player::on(pad), false);
        assert!(text.contains("<type>Wii U Pro Controller</type>"));
        assert!(!text.contains("<controller>"));
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
        // D-pad up, numbered after Home, is taken from the stick, which still
        // steers as a stick too.
        assert!(pro.contains("<mapping>12</mapping>\n\t\t\t\t<button>39</button>"), "d-pad up from the stick: {pro}");
        assert!(pro.contains("<mapping>18</mapping>\n\t\t\t\t<button>39</button>"), "stick up: {pro}");
        assert!(!pro.contains("<button>0</button>"), "the pad's own d-pad has no button: {pro}");
        let gamepad = std::fs::read_to_string(dir.join(FIRST_AS_GAMEPAD)).unwrap();
        assert_eq!(gamepad, std::fs::read_to_string(dir.join("controller0.xml")).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn ps5() -> Pad {
        Pad {
            device: "PS5 Controller 0".to_string(),
            name: "PS5 Controller".to_string(),
            handler: "SDL".to_string(),
            family: "playstation".to_string(),
        }
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("omoio-cemu-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn devices(players: &[Player]) -> Vec<&str> {
        players.iter().map(|p| p.pad.device.as_str()).collect()
    }

    #[test]
    fn a_ps5_pad_through_ds4windows_plays_player_one_as_its_xinput_twin() {
        // Set up while DS4Windows was off, then played with it on: the PS5
        // pad is player 1 and the XInput pad DS4Windows makes of it player 2.
        let buttons = BTreeMap::from([
            ("East".to_string(), "South".to_string()),
            ("South".to_string(), "East".to_string()),
        ]);
        let mut players = vec![Player::with_buttons(ps5(), buttons.clone())];
        players.extend((1..=3).map(|slot| Player::on(xinput(slot))));
        let cemu = stand_in(&players, &[xinput(1), ps5()]);
        assert_eq!(devices(&cemu), ["XInput Pad #1", "PS5 Controller 0", "XInput Pad #2", "XInput Pad #3"]);
        assert_eq!(cemu[0].buttons, buttons, "player 1 keeps their own buttons");
        assert!(cemu[1].buttons.is_empty(), "and player 2 theirs");
        assert_eq!(devices(&players)[0], "PS5 Controller 0", "the layout Omoio keeps is untouched");

        let dir = scratch("ds4windows");
        write_all(&dir, &cemu).unwrap();
        let one = std::fs::read_to_string(dir.join("controller0.xml")).unwrap();
        assert!(one.contains("<api>XInput</api>") && one.contains("<uuid>0</uuid>"), "{one}");
        assert!(one.contains("<mapping>1</mapping>\n\t\t\t\t<button>12</button>"), "A as player 1 set it: {one}");
        let two = std::fs::read_to_string(dir.join("controller1.xml")).unwrap();
        assert!(!two.contains("<controller>"), "the same pad does not drive player 2 too: {two}");
        let pro = std::fs::read_to_string(dir.join(FIRST_AS_PRO)).unwrap();
        assert!(pro.contains("<uuid>0</uuid>"), "Trap Team's player 1 gets it as well: {pro}");
        assert!(pro.contains("<mapping>12</mapping>\n\t\t\t\t<button>39</button>"), "d-pad from the stick: {pro}");
        assert_eq!(nobody_answers(&dir, players.first()), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_xinput_pad_no_one_has_is_taken_first() {
        let mut players = vec![Player::on(ps5())];
        players.extend((1..=3).map(|slot| Player::on(xinput(slot))));
        let cemu = stand_in(&players, &[xinput(1), ps5(), xinput(4)]);
        assert_eq!(devices(&cemu), ["XInput Pad #4", "XInput Pad #1", "XInput Pad #2", "XInput Pad #3"]);
    }

    #[test]
    fn player_one_on_xinput_is_left_alone() {
        let players = vec![Player::on(xinput(2)), Player::on(ps5())];
        assert_eq!(stand_in(&players, &[xinput(1), ps5()]), players, "even with their own pad not plugged in");
    }

    #[test]
    fn with_no_xinput_pad_plugged_in_the_game_says_why_it_cannot_start() {
        let mut players = vec![Player::on(ps5())];
        players.extend((1..=3).map(|slot| Player::on(xinput(slot))));
        let cemu = stand_in(&players, &[ps5()]);
        assert_eq!(cemu, players);
        let dir = scratch("no-xinput");
        write_all(&dir, &cemu).unwrap();
        let why = nobody_answers(&dir, players.first()).expect("player 1 has no pad in Cemu");
        assert!(why.starts_with("Player 1's controller, PS5 Controller, doesn't work in Wii U games yet."), "{why}");
        assert!(why.contains("Controller screen"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cemu_with_no_files_yet_has_nobody_to_answer() {
        let dir = scratch("no-files");
        let why = nobody_answers(&dir, Some(&Player::on(xinput(1)))).unwrap();
        assert!(why.starts_with("Player 1 has no controller in Wii U games."), "{why}");
    }
}
