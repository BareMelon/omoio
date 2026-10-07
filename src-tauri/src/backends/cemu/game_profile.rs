//! Cemu's settings for one game: its game profile, `gameProfiles/<title id>.ini`
//! in Cemu's config folder, which is `portable/` for Omoio's Cemu.
//!
//! Checked in Cemu v2.6 (`GameProfile.cpp`, `GameProfileWindow.cpp`,
//! `CemuConfig.h`):
//!
//! - Cemu reads the game's own profile when there is one, and otherwise the
//!   one it ships in `gameProfiles/default/`. It never merges the two, so a
//!   game's own profile starts as a copy of the shipped one, as Cemu's own
//!   profile window does, or the fixes Cemu ships for that game would be lost.
//! - The options are the ones Cemu's profile window offers, written the way
//!   Cemu writes them. Cemu also reads numbers for the graphics API and CPU
//!   mode, and older CPU mode names ("Singlecore-Recompiler"), which its
//!   shipped profiles still use.
//!
//! Omoio writes only what the user chose. Everything else in the file, such
//! as a controller profile picked in Cemu itself, is left as it was.

use crate::core::game_settings::{Chosen, Setting};
use std::path::{Path, PathBuf};

/// A section and key, joined as the settings sheet keys every setting.
const SEP: char = '\n';

struct Offered {
    section: &'static str,
    key: &'static str,
    label: &'static str,
    hint: &'static str,
    kind: &'static str,
    choices: &'static [&'static str],
    /// Cemu's own value when the profile says nothing, where there is one.
    default: &'static str,
    common: bool,
}

const OFFERED: &[Offered] = &[
    Offered {
        section: "Graphics",
        key: "graphics_api",
        label: "Graphics API",
        hint: "Vulkan suits nearly everything. OpenGL is worth trying when a game will not draw.",
        kind: "choice",
        choices: &["Vulkan", "OpenGL"],
        default: "",
        common: true,
    },
    Offered {
        section: "CPU",
        key: "cpuMode",
        label: "CPU mode",
        hint: "Auto lets Cemu decide. The single-core modes are slower, and only a few games need one.",
        kind: "choice",
        choices: &["Auto", "Multi-core recompiler", "Single-core recompiler", "Single-core interpreter"],
        default: "Auto",
        common: true,
    },
    Offered {
        section: "General",
        key: "startWithPadView",
        label: "Start on the GamePad screen",
        hint: "Shows the GamePad's own screen rather than the TV picture. Ctrl+Tab switches while playing.",
        kind: "switch",
        choices: &[],
        default: "false",
        common: true,
    },
    Offered {
        section: "Graphics",
        key: "accurateShaderMul",
        label: "Accurate shader multiplication",
        hint: "How exactly shaders multiply. Cemu recommends on.",
        kind: "switch",
        choices: &[],
        default: "true",
        common: false,
    },
    Offered {
        section: "CPU",
        key: "threadQuantum",
        label: "Thread quantum",
        hint: "The longest an emulated thread runs before the next gets a turn, in cycles. For experts.",
        kind: "choice",
        choices: &["20000", "45000", "60000", "80000", "100000"],
        default: "45000",
        common: false,
    },
    Offered {
        section: "General",
        key: "loadSharedLibraries",
        label: "Load shared libraries",
        hint: "Loads libraries from Cemu's cafeLibs folder. For experts.",
        kind: "switch",
        choices: &[],
        default: "",
        common: false,
    },
];

pub fn options() -> Vec<Setting> {
    OFFERED
        .iter()
        .map(|offered| Setting {
            key: format!("{}{SEP}{}", offered.section, offered.key),
            group: offered.section.to_string(),
            name: offered.key.to_string(),
            label: offered.label.to_string(),
            hint: offered.hint.to_string(),
            kind: offered.kind.to_string(),
            choices: offered.choices,
            default: offered.default.to_string(),
            min: 0,
            max: 0,
            common: offered.common,
        })
        .collect()
}

/// A value as the sheet shows it, from any spelling Cemu reads. `None` for
/// one Cemu itself would not take.
fn shown(key: &str, value: &str) -> Option<String> {
    let folded: String = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    let known = match key {
        "graphics_api" => match folded.as_str() {
            "0" => "OpenGL",
            "1" => "Vulkan",
            _ => return None,
        },
        // Cemu turns its old dual-core mode into multi-core when it reads it.
        "cpuMode" => match folded.as_str() {
            "0" | "singlecoreinterpreter" => "Single-core interpreter",
            "1" | "singlecorerecompiler" => "Single-core recompiler",
            "2" | "3" | "dualcorerecompiler" | "multicorerecompiler" => "Multi-core recompiler",
            "4" | "auto" => "Auto",
            _ => return None,
        },
        "startWithPadView" | "accurateShaderMul" | "loadSharedLibraries" => match folded.as_str() {
            "1" | "true" => "true",
            "0" | "false" => "false",
            _ => return None,
        },
        "threadQuantum" if !folded.is_empty() && folded.chars().all(|c| c.is_ascii_digit()) => return Some(folded),
        _ => return None,
    };
    Some(known.to_string())
}

/// A value as Cemu writes it.
fn stored(key: &str, value: &str) -> String {
    match (key, value) {
        ("graphics_api", "OpenGL") => "0".to_string(),
        ("graphics_api", "Vulkan") => "1".to_string(),
        _ => value.to_string(),
    }
}

fn own_path(install: &Path, title_id: &str) -> PathBuf {
    super::user_data(install).join("gameProfiles").join(format!("{title_id}.ini"))
}

fn shipped_path(install: &Path, title_id: &str) -> PathBuf {
    let resources = if cfg!(windows) { install.to_path_buf() } else { install.join("AppDir/usr/share/Cemu") };
    resources.join("gameProfiles").join("default").join(format!("{title_id}.ini"))
}

fn read_lines(path: &Path) -> Option<Vec<String>> {
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.lines().map(str::to_string).collect())
}

fn is_comment(line: &str) -> bool {
    let line = line.trim();
    line.starts_with('#') || line.starts_with(';')
}

fn section_name(line: &str) -> Option<&str> {
    line.trim().strip_prefix('[')?.strip_suffix(']').map(str::trim)
}

/// Where `key` is set in `section`, as a line number.
fn find(lines: &[String], section: &str, key: &str) -> Option<usize> {
    let mut current = "";
    for (at, line) in lines.iter().enumerate() {
        if let Some(name) = section_name(line) {
            current = name;
        } else if current.eq_ignore_ascii_case(section) && !is_comment(line) {
            if line.split_once('=').is_some_and(|(name, _)| name.trim() == key) {
                return Some(at);
            }
        }
    }
    None
}

fn value_of(lines: &[String], section: &str, key: &str) -> Option<String> {
    find(lines, section, key).and_then(|at| lines[at].split_once('=').map(|(_, value)| value.trim().to_string()))
}

/// Sets `key` in `section`, adding the line at the end of the section, and
/// the section at the end of the file, where they are missing.
fn set(lines: &mut Vec<String>, section: &str, key: &str, value: &str) {
    let entry = format!("{key} = {value}");
    if let Some(at) = find(lines, section, key) {
        lines[at] = entry;
        return;
    }
    let header = lines
        .iter()
        .position(|line| section_name(line).is_some_and(|name| name.eq_ignore_ascii_case(section)));
    let Some(header) = header else {
        if lines.last().is_some_and(|line| !line.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(format!("[{section}]"));
        lines.push(entry);
        return;
    };
    let mut end = lines[header + 1..]
        .iter()
        .position(|line| section_name(line).is_some())
        .map_or(lines.len(), |next| header + 1 + next);
    while end > header + 1 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    lines.insert(end, entry);
}

fn remove(lines: &mut Vec<String>, section: &str, key: &str) {
    if let Some(at) = find(lines, section, key) {
        lines.remove(at);
    }
}

/// What the user has chosen for this game: each option the game's own
/// profile sets to something other than what Cemu would use without it,
/// which is the shipped profile's value or else Cemu's default.
pub fn read(install: &Path, title_id: &str) -> Chosen {
    let Some(own) = read_lines(&own_path(install, title_id)) else {
        return Chosen::new();
    };
    let shipped = read_lines(&shipped_path(install, title_id)).unwrap_or_default();
    OFFERED
        .iter()
        .filter_map(|offered| {
            let value = value_of(&own, offered.section, offered.key).and_then(|v| shown(offered.key, &v))?;
            let without = value_of(&shipped, offered.section, offered.key)
                .and_then(|v| shown(offered.key, &v))
                .unwrap_or_else(|| offered.default.to_string());
            (value != without).then(|| (format!("{}{SEP}{}", offered.section, offered.key), value))
        })
        .collect()
}

/// Writes the game's own profile: the shipped one to start from when there is
/// no own one yet, each offered option set to what was chosen or put back to
/// the shipped value, and nothing else touched. Choosing nothing removes the
/// game's own profile, so Cemu goes back to the one it ships.
pub fn write(install: &Path, title_id: &str, title: &str, chosen: &Chosen) -> Result<(), String> {
    let own = own_path(install, title_id);
    if chosen.is_empty() {
        if own.exists() {
            std::fs::remove_file(&own).map_err(|_| "Couldn't reset this game's settings.".to_string())?;
        }
        return Ok(());
    }
    let shipped = read_lines(&shipped_path(install, title_id)).unwrap_or_default();
    let mut lines = read_lines(&own).unwrap_or_else(|| shipped.clone());
    if lines.is_empty() {
        lines.push(format!("# {title}"));
    }
    for offered in OFFERED {
        let key = format!("{}{SEP}{}", offered.section, offered.key);
        match (chosen.get(&key), value_of(&shipped, offered.section, offered.key)) {
            (Some(value), _) => set(&mut lines, offered.section, offered.key, &stored(offered.key, value)),
            (None, Some(value)) => set(&mut lines, offered.section, offered.key, &value),
            (None, None) => remove(&mut lines, offered.section, offered.key),
        }
    }
    if let Some(folder) = own.parent() {
        std::fs::create_dir_all(folder).map_err(|_| "Couldn't save this game's settings.".to_string())?;
    }
    std::fs::write(&own, lines.join("\n") + "\n").map_err(|_| "Couldn't save this game's settings.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0005000010101a00";

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("omoio-cemu-profile-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("gameProfiles").join("default")).unwrap();
        dir
    }

    fn chosen(pairs: &[(&str, &str)]) -> Chosen {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn values_are_read_in_every_spelling_cemu_takes() {
        assert_eq!(shown("cpuMode", "Singlecore-Recompiler").as_deref(), Some("Single-core recompiler"));
        assert_eq!(shown("cpuMode", "Multi-core recompiler").as_deref(), Some("Multi-core recompiler"));
        assert_eq!(shown("cpuMode", "Dualcore-Recompiler").as_deref(), Some("Multi-core recompiler"));
        assert_eq!(shown("cpuMode", "4").as_deref(), Some("Auto"));
        assert_eq!(shown("graphics_api", "1").as_deref(), Some("Vulkan"));
        assert_eq!(shown("startWithPadView", "1").as_deref(), Some("true"));
        assert_eq!(shown("threadQuantum", "45000").as_deref(), Some("45000"));
        assert_eq!(shown("cpuMode", "Turbo"), None);
        assert_eq!(stored("graphics_api", "OpenGL"), "0");
    }

    #[test]
    fn the_shipped_profile_is_the_start_and_its_fixes_stay() {
        let install = scratch("shipped");
        let shipped = "# LEGO City Undercover (USA)\n\n[CPU]\ncpuMode = Singlecore-Recompiler\n";
        std::fs::write(shipped_path(&install, ID), shipped).unwrap();

        write(&install, ID, "LEGO City Undercover", &chosen(&[("Graphics\ngraphics_api", "OpenGL")])).unwrap();
        let text = std::fs::read_to_string(own_path(&install, ID)).unwrap();
        assert!(text.contains("cpuMode = Singlecore-Recompiler"), "{text}");
        assert!(text.contains("[Graphics]\ngraphics_api = 0"), "{text}");
        assert_eq!(read(&install, ID), chosen(&[("Graphics\ngraphics_api", "OpenGL")]), "the shipped fix is no choice of ours");

        write(&install, ID, "LEGO City Undercover", &chosen(&[("CPU\ncpuMode", "Auto")])).unwrap();
        let text = std::fs::read_to_string(own_path(&install, ID)).unwrap();
        assert!(text.contains("cpuMode = Auto") && !text.contains("graphics_api"), "{text}");

        write(&install, ID, "LEGO City Undercover", &Chosen::new()).unwrap();
        assert!(!own_path(&install, ID).exists(), "nothing chosen is Cemu's own profile again");
        let _ = std::fs::remove_dir_all(&install);
    }

    #[test]
    fn what_omoio_does_not_offer_is_left_alone() {
        let install = scratch("others");
        std::fs::create_dir_all(own_path(&install, ID).parent().unwrap()).unwrap();
        std::fs::write(
            own_path(&install, ID),
            "[General]\nstartWithPadView = false\n\n[CPU]\nthreadQuantum = 45000\n\n[Controller]\ncontroller1 = mine\n",
        )
        .unwrap();
        assert!(read(&install, ID).is_empty(), "Cemu's own defaults, written out, are not choices");

        write(&install, ID, "Game", &chosen(&[("General\nstartWithPadView", "true")])).unwrap();
        let text = std::fs::read_to_string(own_path(&install, ID)).unwrap();
        assert!(text.contains("startWithPadView = true"), "{text}");
        assert!(text.contains("controller1 = mine"), "{text}");
        assert!(!text.contains("threadQuantum"), "put back to Cemu's own by leaving it out: {text}");
        let _ = std::fs::remove_dir_all(&install);
    }
}
