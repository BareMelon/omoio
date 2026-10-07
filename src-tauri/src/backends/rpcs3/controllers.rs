//! RPCS3's side of the controller layout: its per-player profile file.
//!
//! RPCS3 starts out with no pad for anyone. Its own dialog says a real
//! controller is recommended, but nothing happens until someone opens that
//! dialog and picks a handler, and every button in a fresh profile is empty.
//! So plugging in a pad and pressing Play does nothing at all.
//!
//! Omoio keeps the layout itself (core/pad_layout.rs) and writes it here in
//! RPCS3's form. The device name has to be the one RPCS3 will see: a name it
//! cannot find becomes a placeholder rather than binding to whatever is
//! plugged in.
//!
//! - An Xbox pad, or anything else speaking XInput, goes through RPCS3's
//!   XInput handler. It names pads by slot, "XInput Pad #1" to "#4", so the
//!   name is known the moment Windows says a slot is in use. No guessing.
//! - Anything else, a DualSense or a Switch Pro or an 8BitDo, goes through SDL,
//!   which reads the controller database RPCS3 ships. Its name is the pad's
//!   SDL mapping name, which gilrs reads from the same database.
//!
//! An XInput slot can be named before anything is in it (RPCS3's
//! `xinput_pad_handler::get_device` takes any of the four), and RPCS3 notices
//! when a pad arrives, so players can wait on the slots a second, third and
//! fourth pad will take.

use crate::core::pad_layout::{self, Pad, Player, PLAYERS};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tauri::AppHandle;

/// Each PS3 input by RPCS3's name for it, which is also its key in the file,
/// and the place it sits on a pad. In the order RPCS3's own dialog lists them.
const PS3: [(&str, &str); 25] = [
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
];

/// What the Controller screen calls each place for the PS3.
pub const BUTTON_NAMES: [(&str, &str); 25] = [
    ("South", "Cross"),
    ("East", "Circle"),
    ("West", "Square"),
    ("North", "Triangle"),
    ("LB", "L1"),
    ("RB", "R1"),
    ("LT", "L2"),
    ("RT", "R2"),
    ("LS", "L3"),
    ("RS", "R3"),
    ("Up", "D-pad up"),
    ("Down", "D-pad down"),
    ("Left", "D-pad left"),
    ("Right", "D-pad right"),
    ("Back", "Select"),
    ("Start", "Start"),
    ("Guide", "PS button"),
    ("LS Y+", "Left stick up"),
    ("LS Y-", "Left stick down"),
    ("LS X-", "Left stick left"),
    ("LS X+", "Left stick right"),
    ("RS Y+", "Right stick up"),
    ("RS Y-", "Right stick down"),
    ("RS X-", "Right stick left"),
    ("RS X+", "Right stick right"),
];

/// SDL names the face buttons by where they sit; XInput by what is printed on
/// them. Everything else is spelled the same by both.
const FACE: [(&str, &str); 4] = [("South", "A"), ("East", "B"), ("West", "X"), ("North", "Y")];

fn to_handler(input: &str, handler: &str) -> String {
    if handler == "XInput" {
        if let Some((_, xinput)) = FACE.iter().find(|(sdl, _)| *sdl == input) {
            return xinput.to_string();
        }
    }
    input.to_string()
}

fn from_handler(input: &str, handler: &str) -> String {
    if handler == "XInput" {
        if let Some((sdl, _)) = FACE.iter().find(|(_, xinput)| *xinput == input) {
            return sdl.to_string();
        }
    }
    input.to_string()
}

/// What the interface calls a pad. RPCS3's own names are "XInput Pad #2", or
/// an SDL name with an index on the end.
fn display_name(handler: &str, device: &str) -> String {
    if handler == "XInput" {
        let slot = device.rsplit('#').next().unwrap_or("1");
        format!("Controller {slot}")
    } else {
        device
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .trim_end()
            .to_string()
    }
}

fn profile_dir(app: &AppHandle, title_id: &str) -> Result<PathBuf, String> {
    let root = super::config_dir(app)?.join("input_configs");
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
/// counts as no profile.
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
            let buttons: BTreeMap<String, String> = PS3
                .iter()
                .filter_map(|(key, place)| {
                    read_value(section, key).map(|found| (place.to_string(), from_handler(&found, &handler)))
                })
                .collect();
            let pad = Pad {
                name: display_name(&handler, &device),
                family: if handler == "XInput" { "xbox" } else { "generic" }.to_string(),
                device,
                handler,
            };
            Some(Player::with_buttons(pad, buttons))
        })
        .collect()
}

/// The file RPCS3 reads. Every button is written, not just the ones that
/// differ, because a fresh profile has them all empty. Leaving one out means
/// leaving it unbound.
fn profile_text(players: &[Player]) -> String {
    let mut out = String::new();
    for (at, player) in players.iter().enumerate().take(PLAYERS) {
        let pad = &player.pad;
        out.push_str(&format!("Player {} Input:\n", at + 1));
        out.push_str(&format!("  Handler: {}\n", pad.handler));
        out.push_str(&format!("  Device: {}\n", quoted(&pad.device)));
        out.push_str("  Buddy Device: \"\"\n");
        out.push_str("  Config:\n");
        for (key, place) in PS3 {
            out.push_str(&format!(
                "    {key}: {}\n",
                quoted(&to_handler(player.input(place), &pad.handler))
            ));
        }
        // A profile's deadzone defaults to zero rather than to the handler's,
        // and at zero a worn stick drifts. These are the handlers' own
        // numbers: SDL's from its init_config, XInput's the constants in
        // Microsoft's XInput.h.
        let (left, right) = if pad.handler == "XInput" {
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

/// Writes the players' layout where RPCS3 reads it, for every game or for one.
pub fn write(app: &AppHandle, title_id: &str, players: &[Player]) -> Result<(), String> {
    let usable = |p: &Player| !p.pad.device.is_empty() && (matches!(p.pad.handler.as_str(), "XInput" | "SDL") || (!cfg!(windows) && p.pad.handler == "Null"));
    if !players.iter().all(usable) {
        return Err("Couldn't save the controller settings.".to_string());
    }
    std::fs::create_dir_all(profile_dir(app, title_id)?).map_err(|e| e.to_string())?;
    std::fs::write(profile_path(app, title_id)?, profile_text(players))
        .map_err(|_| "Couldn't save the controller settings.".to_string())
}

/// Takes a game's own profile away, so it goes back to the one for every game.
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

/// The profiles RPCS3 already has, the one for every game and each game's
/// own, read in the first time Omoio keeps a layout of its own.
pub fn existing(app: &AppHandle) -> Vec<(String, Vec<Player>)> {
    let mut scopes = vec![String::new()];
    if let Ok(entries) = super::config_dir(app)
        .map(|dir| dir.join("input_configs"))
        .and_then(|root| std::fs::read_dir(root).map_err(|e| e.to_string()))
    {
        scopes.extend(
            entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name != "global"),
        );
    }
    let spare = crate::pads::xinput_slots();
    scopes
        .into_iter()
        .filter_map(|scope| {
            let text = read_profile(app, &scope)?;
            Some((scope, pad_layout::fill(parse_players(&text), &spare)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(handler: &str, device: &str) -> Pad {
        Pad {
            device: device.to_string(),
            name: display_name(handler, device),
            handler: handler.to_string(),
            family: if handler == "XInput" { "xbox" } else { "generic" }.to_string(),
        }
    }

    #[test]
    fn every_ps3_input_has_a_place_of_its_own() {
        let mut places: Vec<&str> = PS3.iter().map(|(_, place)| *place).collect();
        assert!(places.iter().all(|place| pad_layout::INPUTS.contains(place)));
        places.sort();
        places.dedup();
        assert_eq!(places.len(), 25);
        assert_eq!(BUTTON_NAMES.len(), 25);
    }

    #[test]
    fn xinput_profiles_use_its_names_for_the_face_buttons() {
        let text = profile_text(&[Player::on(pad("XInput", "XInput Pad #1"))]);
        assert!(text.starts_with("Player 1 Input:\n"));
        assert!(text.contains("  Handler: XInput\n"));
        assert!(text.contains("  Device: \"XInput Pad #1\"\n"));
        assert!(text.contains("    Cross: \"A\"\n"));
        assert!(text.contains("    Triangle: \"Y\"\n"));
        assert!(text.contains("    L1: \"LB\"\n"));
        assert!(text.contains("    PS Button: \"Guide\"\n"));
        assert!(text.contains("    Left Stick Deadzone: 7849\n"));
    }

    #[test]
    fn sdl_profiles_keep_sdl_names() {
        let text = profile_text(&[Player::on(pad("SDL", "DualSense Wireless Controller 0"))]);
        assert!(text.contains("  Handler: SDL\n"));
        assert!(text.contains("    Cross: \"South\"\n"));
        assert!(text.contains("    Left Stick Deadzone: 8000\n"));
    }

    #[test]
    fn a_changed_button_lands_on_the_ps3_input_in_that_place() {
        let buttons = BTreeMap::from([("South".to_string(), "East".to_string())]);
        let text = profile_text(&[Player::with_buttons(pad("XInput", "XInput Pad #1"), buttons)]);
        assert!(text.contains("    Cross: \"B\"\n"));
        assert!(text.contains("    Circle: \"B\"\n"), "the other place keeps its own");
    }

    #[test]
    fn four_players_written_read_back_the_same() {
        let buttons = BTreeMap::from([("North".to_string(), "West".to_string())]);
        let mut players = pad_layout::default_players(
            &[pad("SDL", "DualSense Wireless Controller 0")],
            &crate::pads::xinput_slots(),
        );
        players[1] = Player::with_buttons(players[1].pad.clone(), buttons);
        let text = profile_text(&players);
        for number in 1..=4 {
            assert!(text.contains(&format!("Player {number} Input:\n")));
        }
        let back: Vec<Player> = parse_players(&text).into_iter().map(Option::unwrap).collect();
        assert_eq!(back, players);
    }

    #[test]
    fn a_buddy_device_is_not_read_as_the_device() {
        let text = "Player 2 Input:\n  Handler: XInput\n  Buddy Device: \"\"\n  Device: \"XInput Pad #2\"\n  Config:\n    Cross: \"B\"\n";
        let players = parse_players(text);
        assert!(players[0].is_none());
        let second = players[1].as_ref().unwrap();
        assert_eq!(second.pad.device, "XInput Pad #2");
        assert_eq!(second.input("South"), "East", "B is read back as its place");
    }

    #[test]
    fn pads_have_names_worth_reading() {
        assert_eq!(display_name("XInput", "XInput Pad #3"), "Controller 3");
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
