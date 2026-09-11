//! Making controllers work without the user configuring anything.
//!
//! RPCS3 starts out with no pad for anyone. Its own dialog says a real
//! controller is recommended, but nothing happens until someone opens that
//! dialog and picks a handler, and every button in a fresh profile is empty.
//! So plugging in a pad and pressing Play does nothing at all.
//!
//! Omoio writes the profile instead, and the device name in it has to be the
//! one RPCS3 will see: a name it cannot find becomes a placeholder rather than
//! binding to whatever is plugged in.
//!
//! - An Xbox pad, or anything else speaking XInput, goes through RPCS3's
//!   XInput handler. It names pads by slot, "XInput Pad #1" to "#4", so the
//!   name is known the moment Windows says a slot is in use. No guessing.
//! - Anything else, a DualSense or a Switch Pro or an 8BitDo, goes through SDL,
//!   which reads the controller database RPCS3 ships. Its name is the pad's
//!   SDL mapping name, which gilrs reads from the same database.
//!
//! Four players are set up from the start. A player the file leaves out has
//! no pad, so a second pad plugged in used to do nothing. An XInput slot can
//! be named before anything is in it (RPCS3's `xinput_pad_handler::get_device`
//! takes any of the four), and RPCS3 notices when a pad arrives, so players
//! two to four wait on the slots a second, third and fourth pad will take.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;

/// How many players Omoio sets up. RPCS3 takes seven, as the PS3 did, but
/// games stop at four.
pub const PLAYERS: usize = 4;

/// A pad, and how RPCS3 will address it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    /// What goes in the profile's `Device` line.
    pub device: String,
    /// What the interface calls it.
    pub name: String,
    /// RPCS3's handler for it: "XInput" or "SDL".
    pub handler: String,
}

/// One PS3 input and the physical button it is bound to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    /// RPCS3's name for the PS3 input, which is also the key in its file.
    pub key: String,
    /// The physical button, by its SDL name. XInput calls the face buttons
    /// A, B, X and Y instead; that is translated when the file is written, so
    /// the interface only ever deals in one set of names.
    pub button: String,
}

/// One player's pad and layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub controller: Controller,
    pub bindings: Vec<Binding>,
}

/// RPCS3's own defaults, from `sdl_pad_handler::init_config`. XInput's are the
/// same once its face buttons are translated.
fn default_bindings() -> Vec<Binding> {
    [
        ("Cross", "South"),
        ("Circle", "East"),
        ("Square", "West"),
        ("Triangle", "North"),
        ("Up", "Up"),
        ("Down", "Down"),
        ("Left", "Left"),
        ("Right", "Right"),
        ("L1", "LB"),
        ("L2", "LT"),
        ("L3", "LS"),
        ("R1", "RB"),
        ("R2", "RT"),
        ("R3", "RS"),
        ("Start", "Start"),
        ("Select", "Back"),
        ("PS Button", "Guide"),
        ("Left Stick Left", "LS X-"),
        ("Left Stick Right", "LS X+"),
        ("Left Stick Up", "LS Y+"),
        ("Left Stick Down", "LS Y-"),
        ("Right Stick Left", "RS X-"),
        ("Right Stick Right", "RS X+"),
        ("Right Stick Up", "RS Y+"),
        ("Right Stick Down", "RS Y-"),
    ]
    .into_iter()
    .map(|(key, button)| Binding {
        key: key.to_string(),
        button: button.to_string(),
    })
    .collect()
}

/// The physical inputs a pad can send, by SDL name, for the picker.
pub fn choices() -> Vec<&'static str> {
    vec![
        "South", "East", "West", "North", "Up", "Down", "Left", "Right", "LB", "LT", "LS", "RB",
        "RT", "RS", "Start", "Back", "Guide", "LS X-", "LS X+", "LS Y-", "LS Y+", "RS X-", "RS X+",
        "RS Y-", "RS Y+",
    ]
}

/// SDL names the face buttons by where they sit; XInput by what is printed on
/// them. Everything else is spelled the same by both.
const FACE: [(&str, &str); 4] = [("South", "A"), ("East", "B"), ("West", "X"), ("North", "Y")];

fn to_handler(button: &str, handler: &str) -> String {
    if handler == "XInput" {
        if let Some((_, xinput)) = FACE.iter().find(|(sdl, _)| *sdl == button) {
            return xinput.to_string();
        }
    }
    button.to_string()
}

fn from_handler(button: &str, handler: &str) -> String {
    if handler == "XInput" {
        if let Some((sdl, _)) = FACE.iter().find(|(_, xinput)| *xinput == button) {
            return sdl.to_string();
        }
    }
    button.to_string()
}

/// What the interface calls a pad. RPCS3's own names are "XInput Pad #2", or
/// an SDL name with an index on the end.
fn display_name(handler: &str, device: &str) -> String {
    if handler == "XInput" {
        let slot = device.rsplit('#').next().unwrap_or("1");
        format!("Xbox controller {slot}")
    } else {
        device
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .trim_end()
            .to_string()
    }
}

/// One of the four XInput slots, counted from zero.
fn xinput_slot(slot: usize) -> Controller {
    let device = format!("XInput Pad #{}", slot + 1);
    Controller {
        name: display_name("XInput", &device),
        device,
        handler: "XInput".to_string(),
    }
}

/// Which of the four XInput slots have a pad in them, the same slots RPCS3's
/// XInput handler walks.
#[cfg(windows)]
fn xinput_slots() -> Vec<u32> {
    use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
    (0..4)
        .filter(|&slot| {
            let mut state = XINPUT_STATE::default();
            // A call into a Windows DLL, which Rust cannot check, so it is
            // marked unsafe. All it does is fill in the struct we hand it.
            // Zero means a pad answered in that slot.
            unsafe { XInputGetState(slot, &mut state) == 0 }
        })
        .collect()
}

#[cfg(not(windows))]
fn xinput_slots() -> Vec<u32> {
    Vec::new()
}

/// Microsoft's USB vendor id. Its pads already answered through XInput, so
/// they are not listed a second time as SDL devices.
const MICROSOFT: u16 = 0x045E;

/// The pads plugged in right now, XInput ones first since that is what most
/// people have.
pub fn connected() -> Vec<Controller> {
    let slots = xinput_slots();
    let mut found: Vec<Controller> = slots.iter().map(|&slot| xinput_slot(slot as usize)).collect();

    if let Ok(gilrs) = gilrs::Gilrs::new() {
        // RPCS3 numbers pads of the same name from zero, which is how two
        // identical controllers are told apart in its file.
        let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for (_, pad) in gilrs.gamepads() {
            if !slots.is_empty() && pad.vendor_id() == Some(MICROSOFT) {
                continue;
            }
            let name = pad.map_name().unwrap_or(pad.name()).to_string();
            let at = seen.entry(name.clone()).or_insert(0);
            found.push(Controller {
                device: format!("{name} {at}"),
                name: name.clone(),
                handler: "SDL".to_string(),
            });
            *at += 1;
        }
    }

    found
}

fn profile_dir(app: &AppHandle, title_id: &str) -> Result<PathBuf, String> {
    let root = super::install_dir(app)?.join("config").join("input_configs");
    Ok(if title_id.is_empty() {
        root.join("global")
    } else {
        root.join(title_id)
    })
}

/// Where RPCS3 reads the profile from. An empty `title_id` is the profile that
/// applies to every game; a title id is that game's own.
fn profile_path(app: &AppHandle, title_id: &str) -> Result<PathBuf, String> {
    Ok(profile_dir(app, title_id)?.join("Default.yml"))
}

/// An empty file is what RPCS3 itself writes for an untouched profile, so it
/// does not count as set up.
pub fn have_profile(app: &AppHandle, title_id: &str) -> bool {
    profile_path(app, title_id)
        .map(|path| std::fs::metadata(&path).is_ok_and(|meta| meta.len() > 0))
        .unwrap_or(false)
}

fn read_profile(app: &AppHandle, title_id: &str) -> Option<String> {
    std::fs::read_to_string(profile_path(app, title_id).ok()?)
        .ok()
        .filter(|text| !text.trim().is_empty())
}

/// Reads one `Key: value` out of the profile.
///
/// The file is ours: we write it, RPCS3 rewrites it in the same shape, and the
/// values are short strings. A YAML parser would be a dependency for a few
/// dozen lines of flat key-value.
fn read_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| line.starts_with(key) && line[key.len()..].starts_with(':'))
        .map(|line| line[key.len() + 1..].trim().trim_matches('"').to_string())
}

/// The players in a profile, by number. A player the file leaves out, or
/// gives a handler Omoio does not write, comes back as `None`.
fn parse_players(text: &str) -> Vec<Option<Player>> {
    let mut sections = vec![String::new(); PLAYERS];
    let mut current: Option<usize> = None;
    for line in text.lines() {
        if let Some(number) = line
            .strip_prefix("Player ")
            .and_then(|rest| rest.trim_end().strip_suffix(" Input:"))
        {
            current = number
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|n| (1..=PLAYERS).contains(n))
                .map(|n| n - 1);
            continue;
        }
        // Any other line at the left edge starts something that is not a
        // player.
        if !line.starts_with(' ') {
            current = None;
        }
        if let Some(at) = current {
            sections[at].push_str(line);
            sections[at].push('\n');
        }
    }

    sections
        .iter()
        .map(|section| {
            let handler = read_value(section, "Handler")?;
            let device = read_value(section, "Device").filter(|d| !d.is_empty())?;
            if handler != "XInput" && handler != "SDL" {
                return None;
            }
            let mut bindings = default_bindings();
            for binding in &mut bindings {
                if let Some(found) = read_value(section, &binding.key) {
                    binding.button = from_handler(&found, &handler);
                }
            }
            Some(Player {
                controller: Controller {
                    name: display_name(&handler, &device),
                    device,
                    handler,
                },
                bindings,
            })
        })
        .collect()
}

/// The file RPCS3 reads. Every button is written, not just the ones that
/// differ, because a fresh profile has them all empty. Leaving one out means
/// leaving it unbound.
fn profile_text(players: &[Player]) -> String {
    let mut out = String::new();
    for (at, player) in players.iter().enumerate().take(PLAYERS) {
        let controller = &player.controller;
        out.push_str(&format!("Player {} Input:\n", at + 1));
        out.push_str(&format!("  Handler: {}\n", controller.handler));
        out.push_str(&format!("  Device: {}\n", quoted(&controller.device)));
        out.push_str("  Buddy Device: \"\"\n");
        out.push_str("  Config:\n");
        for binding in &player.bindings {
            out.push_str(&format!(
                "    {}: {}\n",
                binding.key,
                quoted(&to_handler(&binding.button, &controller.handler))
            ));
        }
        // A profile's deadzone defaults to zero rather than to the handler's,
        // and at zero a worn stick drifts. These are the handlers' own
        // numbers: SDL's from its init_config, XInput's the constants in
        // Microsoft's XInput.h.
        let (left, right) = if controller.handler == "XInput" {
            (7849, 8689)
        } else {
            (8000, 8000)
        };
        out.push_str(&format!("    Left Stick Deadzone: {left}\n"));
        out.push_str(&format!("    Right Stick Deadzone: {right}\n"));
    }
    out
}

/// A name can hold anything the hardware reports, including a colon, which
/// would end the key early.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn write_players(app: &AppHandle, title_id: &str, players: &[Player]) -> Result<(), String> {
    let usable = |p: &Player| {
        !p.controller.device.is_empty() && matches!(p.controller.handler.as_str(), "XInput" | "SDL")
    };
    if !players.iter().all(usable) {
        return Err("Couldn't save the controller settings.".to_string());
    }
    std::fs::create_dir_all(profile_dir(app, title_id)?).map_err(|e| e.to_string())?;
    std::fs::write(profile_path(app, title_id)?, profile_text(players))
        .map_err(|_| "Couldn't save the controller settings.".to_string())
}

/// Who gets which pad when nothing has been chosen. Pads plugged in now take
/// the first players, in the order they are listed, and the XInput slots not
/// taken fill the rest and wait for a pad.
fn default_players(connected: &[Controller]) -> Vec<Player> {
    let mut pads: Vec<Controller> = connected.to_vec();
    for slot in 0..4 {
        let pad = xinput_slot(slot);
        if !pads.iter().any(|known| known.device == pad.device) {
            pads.push(pad);
        }
    }
    pads.truncate(PLAYERS);
    pads.into_iter()
        .map(|controller| Player {
            controller,
            bindings: default_bindings(),
        })
        .collect()
}

/// The players a profile has, with any it leaves out given a pad no other
/// player has.
fn fill(found: Vec<Option<Player>>, connected: &[Controller]) -> Vec<Player> {
    let taken: Vec<String> = found
        .iter()
        .flatten()
        .map(|player| player.controller.device.clone())
        .collect();
    let mut spare = default_players(connected)
        .into_iter()
        .filter(|player| !taken.contains(&player.controller.device));
    found
        .into_iter()
        .enumerate()
        .map(|(at, player)| {
            player.or_else(|| spare.next()).unwrap_or_else(|| Player {
                controller: xinput_slot(at),
                bindings: default_bindings(),
            })
        })
        .collect()
}

/// Gives each pad that is plugged in but belongs to no player the place of
/// the first player whose own pad is not plugged in. Their buttons stay as
/// they were. Returns whether anyone moved.
fn seat(players: &mut [Player], connected: &[Controller]) -> bool {
    let mut moved = false;
    for pad in connected {
        if players.iter().any(|p| p.controller.device == pad.device) {
            continue;
        }
        let waiting = players
            .iter()
            .position(|p| !connected.iter().any(|c| c.device == p.controller.device));
        if let Some(at) = waiting {
            players[at].controller = pad.clone();
            moved = true;
        }
    }
    moved
}

/// The four players as they stand for a profile, whether or not it has been
/// written yet.
pub fn current_players(app: &AppHandle, title_id: &str, connected: &[Controller]) -> Vec<Player> {
    let found = read_profile(app, title_id)
        .map(|text| parse_players(&text))
        .unwrap_or_else(|| vec![None; PLAYERS]);
    fill(found, connected)
}

/// Every pad a player can be given: the four XInput slots, which can be chosen
/// before anything is in them, whatever else is plugged in, and any pad a
/// player already has that is not plugged in right now.
pub fn pads(players: &[Player], connected: &[Controller]) -> Vec<Controller> {
    let mut pads: Vec<Controller> = (0..4).map(xinput_slot).collect();
    for pad in connected.iter().chain(players.iter().map(|p| &p.controller)) {
        if !pads.iter().any(|known| known.device == pad.device) {
            pads.push(pad.clone());
        }
    }
    pads
}

/// Gives player `number`, counted from 1, this pad and layout. A pad another
/// player had is swapped over, so no pad ever drives two players.
///
/// A game's first change starts from the layout for every game, which is what
/// it was playing with until then.
pub fn save_player(app: &AppHandle, title_id: &str, number: usize, player: Player) -> Result<(), String> {
    let at = number
        .checked_sub(1)
        .filter(|at| *at < PLAYERS)
        .ok_or("Couldn't save the controller settings.")?;
    let source = if have_profile(app, title_id) { title_id } else { "" };
    let mut players = current_players(app, source, &connected());
    if let Some(other) = players
        .iter()
        .position(|p| p.controller.device == player.controller.device)
    {
        if other != at {
            players[other].controller = players[at].controller.clone();
        }
    }
    players[at] = player;
    write_players(app, title_id, &players)
}

/// Gives all four players RPCS3's own layout, pads plugged in first.
pub fn set_up(app: &AppHandle, title_id: &str) -> Result<(), String> {
    write_players(app, title_id, &default_players(&connected()))
}

/// Called before a game starts, so plugging in and pressing Play is enough.
/// Players missing from the profile are added, and a pad plugged in that no
/// player has takes the place of one whose pad is not there. Buttons someone
/// chose are never changed.
pub fn set_up_if_needed(app: &AppHandle) {
    let connected = connected();
    let found = read_profile(app, "")
        .map(|text| parse_players(&text))
        .unwrap_or_else(|| vec![None; PLAYERS]);
    let missing = found.iter().any(Option::is_none);
    let mut players = fill(found, &connected);
    let moved = seat(&mut players, &connected);
    if missing || moved {
        let _ = write_players(app, "", &players);
    }
}

/// Takes a game's own layout away, so it goes back to the one for every game.
pub fn forget(app: &AppHandle, title_id: &str) -> Result<(), String> {
    if title_id.is_empty() || title_id.contains(['/', '\\', ':', '.']) {
        return Err("Couldn't remove that.".to_string());
    }
    let dir = profile_dir(app, title_id)?;
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir).map_err(|_| "Couldn't remove that.".to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(handler: &str, device: &str) -> Controller {
        Controller {
            device: device.to_string(),
            name: display_name(handler, device),
            handler: handler.to_string(),
        }
    }

    fn player(handler: &str, device: &str) -> Player {
        Player {
            controller: pad(handler, device),
            bindings: default_bindings(),
        }
    }

    fn devices(players: &[Player]) -> Vec<&str> {
        players.iter().map(|p| p.controller.device.as_str()).collect()
    }

    #[test]
    fn every_ps3_input_has_a_default_the_picker_offers() {
        let bindings = default_bindings();
        assert_eq!(bindings.len(), 25);
        for binding in &bindings {
            assert!(
                choices().contains(&binding.button.as_str()),
                "{} maps to {}, which the picker does not offer",
                binding.key,
                binding.button
            );
        }
    }

    #[test]
    fn xinput_profiles_use_its_names_for_the_face_buttons() {
        let text = profile_text(&[player("XInput", "XInput Pad #1")]);
        assert!(text.starts_with("Player 1 Input:\n"));
        assert!(text.contains("  Handler: XInput\n"));
        assert!(text.contains("  Device: \"XInput Pad #1\"\n"));
        assert!(text.contains("    Cross: \"A\"\n"));
        assert!(text.contains("    Triangle: \"Y\"\n"));
        assert!(text.contains("    L1: \"LB\"\n"));
        assert!(text.contains("    Left Stick Deadzone: 7849\n"));
    }

    #[test]
    fn sdl_profiles_keep_sdl_names() {
        let text = profile_text(&[player("SDL", "DualSense Wireless Controller 0")]);
        assert!(text.contains("  Handler: SDL\n"));
        assert!(text.contains("    Cross: \"South\"\n"));
        assert!(text.contains("    Left Stick Deadzone: 8000\n"));
    }

    #[test]
    fn four_players_written_read_back_the_same() {
        let players = default_players(&[pad("SDL", "DualSense Wireless Controller 0")]);
        let text = profile_text(&players);
        for number in 1..=4 {
            assert!(text.contains(&format!("Player {number} Input:\n")));
        }
        let back: Vec<Player> = parse_players(&text).into_iter().map(Option::unwrap).collect();
        assert_eq!(back, players);
    }

    #[test]
    fn with_nothing_plugged_in_four_players_wait_on_the_xinput_slots() {
        assert_eq!(
            devices(&default_players(&[])),
            ["XInput Pad #1", "XInput Pad #2", "XInput Pad #3", "XInput Pad #4"]
        );
    }

    #[test]
    fn pads_plugged_in_take_the_first_players() {
        let players = default_players(&[
            pad("XInput", "XInput Pad #2"),
            pad("SDL", "DualSense Wireless Controller 0"),
        ]);
        assert_eq!(
            devices(&players),
            ["XInput Pad #2", "DualSense Wireless Controller 0", "XInput Pad #1", "XInput Pad #3"]
        );
    }

    #[test]
    fn a_one_player_profile_gains_three_more() {
        let text = profile_text(&[player("SDL", "DualSense Wireless Controller 0")]);
        let players = fill(parse_players(&text), &[]);
        assert_eq!(
            devices(&players),
            ["DualSense Wireless Controller 0", "XInput Pad #1", "XInput Pad #2", "XInput Pad #3"]
        );
    }

    #[test]
    fn a_pad_nobody_has_takes_the_place_of_one_not_plugged_in() {
        let mut players = default_players(&[]);
        let connected = [
            pad("XInput", "XInput Pad #1"),
            pad("SDL", "DualSense Wireless Controller 0"),
        ];
        assert!(seat(&mut players, &connected));
        assert_eq!(
            devices(&players),
            ["XInput Pad #1", "DualSense Wireless Controller 0", "XInput Pad #3", "XInput Pad #4"]
        );
        assert!(!seat(&mut players, &connected), "nothing moves the second time");
    }

    #[test]
    fn a_buddy_device_is_not_read_as_the_device() {
        let text = "Player 2 Input:\n  Handler: XInput\n  Buddy Device: \"\"\n  Device: \"XInput Pad #2\"\n  Config:\n    Cross: \"B\"\n";
        let players = parse_players(text);
        assert!(players[0].is_none());
        let second = players[1].as_ref().unwrap();
        assert_eq!(second.controller.device, "XInput Pad #2");
        assert_eq!(second.bindings[0].button, "East", "B is read back in SDL's name");
    }

    #[test]
    fn pads_have_names_worth_reading() {
        assert_eq!(display_name("XInput", "XInput Pad #3"), "Xbox controller 3");
        assert_eq!(display_name("SDL", "DualSense Wireless Controller 0"), "DualSense Wireless Controller");
        assert_eq!(display_name("SDL", "8BitDo Pro 2 1"), "8BitDo Pro 2");
    }

    #[test]
    fn a_name_with_punctuation_survives_being_written() {
        assert_eq!(quoted("Pad: v2"), "\"Pad: v2\"");
        assert_eq!(quoted("say \"hi\""), "\"say \\\"hi\\\"\"");
    }

    /// "Left Stick Left" starts with "Left", so a looser match would answer
    /// one with the other's value.
    #[test]
    fn a_key_is_not_matched_by_a_longer_one_starting_the_same_way() {
        let text = "    Left Stick Left: \"LS X-\"\n    Left: \"Left\"\n";
        assert_eq!(read_value(text, "Left").as_deref(), Some("Left"));
        assert_eq!(read_value(text, "Left Stick Left").as_deref(), Some("LS X-"));
    }
}
