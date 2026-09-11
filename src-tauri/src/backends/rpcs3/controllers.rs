//! Making a controller work without the user configuring anything.
//!
//! RPCS3 starts out bound to the keyboard. Its own dialog says a real
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

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;

/// A pad Omoio can see right now, and how RPCS3 will address it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Controller {
    /// What goes in the profile's `Device` line.
    pub device: String,
    /// What the interface calls it.
    pub name: String,
    /// RPCS3's handler for it: "XInput" or "SDL".
    pub handler: String,
}

/// One PS3 input and the physical button it is bound to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    /// RPCS3's name for the PS3 input, which is also the key in its file.
    pub key: String,
    /// The physical button, by its SDL name. XInput calls the face buttons
    /// A, B, X and Y instead; that is translated when the file is written, so
    /// the interface only ever deals in one set of names.
    pub button: String,
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
    let mut found: Vec<Controller> = slots
        .iter()
        .map(|slot| Controller {
            device: format!("XInput Pad #{}", slot + 1),
            name: if slots.len() > 1 {
                format!("Xbox controller {}", slot + 1)
            } else {
                "Xbox controller".to_string()
            },
            handler: "XInput".to_string(),
        })
        .collect();

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

/// An empty file is what RPCS3 itself writes for an untouched keyboard profile,
/// so it does not count as set up.
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
/// values are short strings. A YAML parser would be a dependency for twenty
/// lines of flat key-value.
fn read_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| line.starts_with(key) && line[key.len()..].starts_with(':'))
        .map(|line| line[key.len() + 1..].trim().trim_matches('"').to_string())
}

/// The controller a profile is bound to, as written in it.
pub fn bound(app: &AppHandle, title_id: &str) -> Option<Controller> {
    let text = read_profile(app, title_id)?;
    let handler = read_value(&text, "Handler")?;
    let device = read_value(&text, "Device").filter(|d| !d.is_empty())?;
    Some(Controller {
        name: device.clone(),
        device,
        handler,
    })
}

/// The bindings in a profile, or RPCS3's own defaults when there is none.
pub fn bindings(app: &AppHandle, title_id: &str) -> Vec<Binding> {
    let mut bindings = default_bindings();
    let Some(text) = read_profile(app, title_id) else {
        return bindings;
    };
    let handler = read_value(&text, "Handler").unwrap_or_default();
    for binding in &mut bindings {
        if let Some(found) = read_value(&text, &binding.key) {
            binding.button = from_handler(&found, &handler);
        }
    }
    bindings
}

/// Writes the profile RPCS3 reads on its next launch.
///
/// Every button is written, not just the ones that differ, because a fresh
/// profile has them all empty. Leaving one out means leaving it unbound.
pub fn write_profile(
    app: &AppHandle,
    title_id: &str,
    controller: &Controller,
    bindings: &[Binding],
) -> Result<(), String> {
    if controller.device.is_empty() {
        return Err("Plug in a controller first.".to_string());
    }
    if controller.handler != "XInput" && controller.handler != "SDL" {
        return Err("Couldn't save the controller settings.".to_string());
    }
    std::fs::create_dir_all(profile_dir(app, title_id)?).map_err(|e| e.to_string())?;
    std::fs::write(profile_path(app, title_id)?, profile_text(controller, bindings))
        .map_err(|_| "Couldn't save the controller settings.".to_string())
}

fn profile_text(controller: &Controller, bindings: &[Binding]) -> String {
    let mut out = String::from("Player 1 Input:\n");
    out.push_str(&format!("  Handler: {}\n", controller.handler));
    out.push_str(&format!("  Device: {}\n", quoted(&controller.device)));
    out.push_str("  Buddy Device: \"\"\n");
    out.push_str("  Config:\n");
    for binding in bindings {
        out.push_str(&format!(
            "    {}: {}\n",
            binding.key,
            quoted(&to_handler(&binding.button, &controller.handler))
        ));
    }
    // A profile's deadzone defaults to zero rather than to the handler's, and
    // at zero a worn stick drifts. These are the handlers' own numbers: SDL's
    // from its init_config, XInput's the constants in Microsoft's XInput.h.
    let (left, right) = if controller.handler == "XInput" {
        (7849, 8689)
    } else {
        (8000, 8000)
    };
    out.push_str(&format!("    Left Stick Deadzone: {left}\n"));
    out.push_str(&format!("    Right Stick Deadzone: {right}\n"));
    out
}

/// A name can hold anything the hardware reports, including a colon, which
/// would end the key early.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Binds the first pad it finds with RPCS3's own layout. Returns the pad used.
pub fn set_up(app: &AppHandle, title_id: &str) -> Result<Controller, String> {
    let controller = connected()
        .into_iter()
        .next()
        .ok_or("No controller found. Plug one in and try again.")?;
    write_profile(app, title_id, &controller, &default_bindings())?;
    Ok(controller)
}

/// Called before a game starts. If nothing has been set up and a pad is
/// plugged in, it is set up now, so plugging in and pressing Play is enough.
/// A profile the user has already made is never touched.
pub fn set_up_if_needed(app: &AppHandle) {
    if !have_profile(app, "") {
        let _ = set_up(app, "");
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
            name: device.to_string(),
            handler: handler.to_string(),
        }
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
        let text = profile_text(&pad("XInput", "XInput Pad #1"), &default_bindings());
        assert!(text.contains("  Handler: XInput\n"));
        assert!(text.contains("  Device: \"XInput Pad #1\"\n"));
        assert!(text.contains("    Cross: \"A\"\n"));
        assert!(text.contains("    Triangle: \"Y\"\n"));
        assert!(text.contains("    L1: \"LB\"\n"));
        assert!(text.contains("    Left Stick Deadzone: 7849\n"));
    }

    #[test]
    fn sdl_profiles_keep_sdl_names() {
        let text = profile_text(&pad("SDL", "DualSense Wireless Controller 0"), &default_bindings());
        assert!(text.contains("  Handler: SDL\n"));
        assert!(text.contains("    Cross: \"South\"\n"));
        assert!(text.contains("    Left Stick Deadzone: 8000\n"));
    }

    #[test]
    fn a_written_xinput_profile_reads_back_in_sdl_names() {
        let text = profile_text(&pad("XInput", "XInput Pad #1"), &default_bindings());
        let handler = read_value(&text, "Handler").unwrap();
        let cross = read_value(&text, "Cross").unwrap();
        assert_eq!(from_handler(&cross, &handler), "South");
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
