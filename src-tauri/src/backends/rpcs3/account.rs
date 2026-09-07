//! The name games call the player, and the region they run as.
//!
//! Both belong to RPCS3 and were read off a real install rather than assumed:
//!
//! - The name is a plain text file, `dev_hdd0/home/<user>/localusername`.
//! - The region is two settings in `config/config.yml`: `License Area`, which
//!   is the console's territory, and `Language`, which is what most games
//!   actually read to pick their text.
//!
//! The values come from RPCS3's own cellSysutil.cpp, so what Omoio writes is
//! what the emulator expects to read back.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;

/// RPCS3's default user. It supports more, but nothing in Omoio creates them,
/// and inventing a second one would only split someone's saves in half.
const USER: &str = "00000001";

const K_AREA: &str = "License Area";
const K_LANGUAGE: &str = "Language";

/// A place to play from, as somebody choosing one would think of it.
///
/// Each pairs the console territory with the language games start in. The
/// spellings are RPCS3's, from cellSysutil.cpp; the names are ours.
pub struct Region {
    pub id: &'static str,
    pub name: &'static str,
    pub area: &'static str,
    pub language: &'static str,
}

pub const REGIONS: &[Region] = &[
    Region { id: "eu-en", name: "Europe", area: "SCEE", language: "English (UK)" },
    Region { id: "us", name: "North America", area: "SCEA", language: "English (US)" },
    Region { id: "jp", name: "Japan", area: "SCEJ", language: "Japanese" },
    Region { id: "eu-fr", name: "France", area: "SCEE", language: "French" },
    Region { id: "eu-de", name: "Germany", area: "SCEE", language: "German" },
    Region { id: "eu-es", name: "Spain", area: "SCEE", language: "Spanish" },
    Region { id: "eu-it", name: "Italy", area: "SCEE", language: "Italian" },
    Region { id: "eu-nl", name: "Netherlands", area: "SCEE", language: "Dutch" },
    Region { id: "eu-pt", name: "Portugal", area: "SCEE", language: "Portuguese (Portugal)" },
    Region { id: "eu-da", name: "Denmark", area: "SCEE", language: "Danish" },
    Region { id: "eu-sv", name: "Sweden", area: "SCEE", language: "Swedish" },
    Region { id: "eu-no", name: "Norway", area: "SCEE", language: "Norwegian" },
    Region { id: "eu-fi", name: "Finland", area: "SCEE", language: "Finnish" },
    Region { id: "eu-pl", name: "Poland", area: "SCEE", language: "Polish" },
    Region { id: "eu-ru", name: "Russia", area: "SCEE", language: "Russian" },
    Region { id: "eu-tr", name: "Turkey", area: "SCEE", language: "Turkish" },
    Region { id: "br", name: "Brazil", area: "SCEA", language: "Portuguese (Brazil)" },
    Region { id: "kr", name: "Korea", area: "SCEK", language: "Korean" },
    Region { id: "hk", name: "Hong Kong and Taiwan", area: "SCEH", language: "Chinese (Traditional)" },
    Region { id: "cn", name: "China", area: "SCH", language: "Chinese (Simplified)" },
];

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub username: String,
    /// The id of the closest matching region, or empty when the emulator is
    /// set to a combination none of ours describes.
    pub region: String,
}

#[derive(Serialize)]
pub struct RegionChoice {
    pub id: &'static str,
    pub name: &'static str,
    pub language: &'static str,
}

pub fn regions() -> Vec<RegionChoice> {
    REGIONS
        .iter()
        .map(|r| RegionChoice { id: r.id, name: r.name, language: r.language })
        .collect()
}

fn username_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?
        .join("dev_hdd0")
        .join("home")
        .join(USER)
        .join("localusername"))
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?.join("config").join("config.yml"))
}

/// PS3 names are short and plain. Anything else is trimmed rather than
/// refused, so a name is never rejected for a reason nobody explained.
fn tidy(name: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
        .collect();
    kept.trim().chars().take(16).collect()
}

pub fn read(app: &AppHandle) -> Account {
    let username = username_path(app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|name| name.trim().to_string())
        .unwrap_or_default();

    let config = config_path(app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    let area = setting(&config, K_AREA).unwrap_or_default();
    let language = setting(&config, K_LANGUAGE).unwrap_or_default();

    let region = REGIONS
        .iter()
        .find(|r| r.area == area && r.language == language)
        .map(|r| r.id.to_string())
        .unwrap_or_default();

    Account { username, region }
}

pub fn set_username(app: &AppHandle, name: &str) -> Result<String, String> {
    let tidied = tidy(name);
    if tidied.is_empty() {
        return Err("Pick a name with at least one letter or number in it.".into());
    }
    let path = username_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &tidied).map_err(|e| e.to_string())?;
    Ok(tidied)
}

pub fn set_region(app: &AppHandle, id: &str) -> Result<(), String> {
    let region = REGIONS
        .iter()
        .find(|r| r.id == id)
        .ok_or("That isn't a region Omoio knows.")?;

    let path = config_path(app)?;
    let existing = std::fs::read_to_string(&path).map_err(|_| {
        "RPCS3 hasn't written its settings yet. Start a game once first.".to_string()
    })?;

    let updated = replace_setting(&existing, K_AREA, region.area);
    let updated = replace_setting(&updated, K_LANGUAGE, region.language);
    std::fs::write(&path, updated).map_err(|e| e.to_string())
}

/// Reads one indented `Key: value` out of RPCS3's config.
fn setting(config: &str, key: &str) -> Option<String> {
    config.lines().find_map(|line| {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix(key)?.strip_prefix(':')?;
        Some(rest.trim().trim_matches('"').to_string())
    })
}

/// Rewrites one line and leaves the rest of the file exactly as it was.
///
/// The global config holds lists and nested maps that our own reader is not
/// built to reproduce, so it is never parsed and re-emitted. Touching the one
/// line means nothing else can be lost on the way through.
fn replace_setting(config: &str, key: &str, value: &str) -> String {
    let mut out = String::with_capacity(config.len() + 32);
    let mut done = false;

    for line in config.lines() {
        let trimmed = line.trim_start();
        if !done && trimmed.starts_with(key) && trimmed[key.len()..].starts_with(':') {
            let indent = &line[..line.len() - trimmed.len()];
            out.push_str(&format!("{indent}{key}: {value}"));
            done = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "\
Core:
  PPU Decoder: Recompiler (LLVM)
  Libraries Control:
    []
System:
  License Area: SCEA
  Language: English (US)
  Keyboard Type: English keyboard (US standard)
Video:
  Renderer: Vulkan
";

    #[test]
    fn reads_the_settings_a_region_is_made_of() {
        assert_eq!(setting(CONFIG, K_AREA).as_deref(), Some("SCEA"));
        assert_eq!(setting(CONFIG, K_LANGUAGE).as_deref(), Some("English (US)"));
        assert_eq!(setting(CONFIG, "Nothing Like This"), None);
    }

    #[test]
    fn changes_one_line_and_leaves_the_rest_alone() {
        let out = replace_setting(CONFIG, K_LANGUAGE, "Danish");

        assert!(out.contains("  Language: Danish"));
        assert!(!out.contains("English (US)"));
        // Everything the global config holds that our reader cannot reproduce
        // has to survive untouched.
        assert!(out.contains("  Libraries Control:\n    []"));
        assert!(out.contains("  Keyboard Type: English keyboard (US standard)"));
        assert!(out.contains("  Renderer: Vulkan"));
        assert_eq!(out.lines().count(), CONFIG.lines().count());
    }

    #[test]
    fn only_the_first_match_is_rewritten() {
        // "Renderer" appears under both Video and Audio in the real file, so a
        // blind replace-all would change a setting nobody asked about.
        let both = "Video:\n  Renderer: Vulkan\nAudio:\n  Renderer: Cubeb\n";
        let out = replace_setting(both, "Renderer", "OpenGL");
        assert!(out.contains("  Renderer: OpenGL"));
        assert!(out.contains("  Renderer: Cubeb"), "the audio one must not move");
    }

    #[test]
    fn a_region_is_a_territory_and_a_language_together() {
        let denmark = REGIONS.iter().find(|r| r.id == "eu-da").unwrap();
        assert_eq!(denmark.area, "SCEE");
        assert_eq!(denmark.language, "Danish");

        // Every region must name values RPCS3 actually accepts.
        const AREAS: &[&str] = &["SCEJ", "SCEA", "SCEE", "SCEH", "SCEK", "SCH", "Other"];
        for region in REGIONS {
            assert!(AREAS.contains(&region.area), "{} has an unknown area", region.id);
            assert!(!region.language.is_empty());
        }
    }

    #[test]
    fn region_ids_are_unique() {
        let mut ids: Vec<&str> = REGIONS.iter().map(|r| r.id).collect();
        let before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(before, ids.len());
    }

    #[test]
    fn a_name_is_tidied_rather_than_refused() {
        assert_eq!(tidy("  Bertram  "), "Bertram");
        assert_eq!(tidy("Bertram!!!"), "Bertram");
        assert_eq!(tidy("a name that is far too long to fit"), "a name that is f");
        assert_eq!(tidy("!!!"), "", "nothing usable left is the one case we refuse");
    }
}
