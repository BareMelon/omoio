//! Telling RPCS3 about a game and starting it.

use crate::core::library::Game;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Where a dump keeps the binary RPCS3 boots. A disc dump puts it under
/// PS3_GAME; an installed PSN title keeps USRDIR at the top.
fn eboot_path(root: &Path) -> Option<PathBuf> {
    let candidates = [
        root.join("PS3_GAME").join("USRDIR").join("EBOOT.BIN"),
        root.join("USRDIR").join("EBOOT.BIN"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// RPCS3 keeps its own list of games as `TITLE_ID: path/`, one per line, with
/// forward slashes and a trailing slash. Read off a working install rather than
/// guessed at, and written back the same way so RPCS3 recognises its own file.
fn games_list_line(title_id: &str, root: &Path) -> String {
    let mut path = root.to_string_lossy().replace('\\', "/");
    if !path.ends_with('/') {
        path.push('/');
    }
    // Windows forbids quotes and backslashes in names, so the quoted form needs
    // no escaping, and it survives a folder called something like "Game #1"
    // which unquoted YAML would read as a comment.
    format!("{title_id}: \"{path}\"")
}

/// Replaces this game's line and leaves every other one untouched. The file
/// belongs to RPCS3 and may list games Omoio knows nothing about, so it is
/// edited rather than rewritten.
fn merged_list(existing: &str, title_id: &str, root: &Path) -> String {
    let prefix = format!("{title_id}:");
    let mut lines: Vec<&str> = existing
        .lines()
        .filter(|line| !line.trim_start().starts_with(&prefix))
        .filter(|line| !line.trim().is_empty())
        .collect();
    let ours = games_list_line(title_id, root);
    lines.push(&ours);

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

pub fn register(app: &AppHandle, game: &Game) -> Result<(), String> {
    let config_dir = super::install_dir(app)?.join("config");
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    let list = config_dir.join("games.yml");

    let existing = std::fs::read_to_string(&list).unwrap_or_default();
    let merged = merged_list(&existing, &game.title_id, &game.path);
    std::fs::write(&list, merged).map_err(|e| e.to_string())
}

/// RPCS3 greets a fresh install with a window about itself, its funding and
/// its piracy policy. The user asked for their game, so the greeting is turned
/// off the same way clicking its checkbox would, before the emulator ever runs.
fn with_welcome_disabled(existing: &str) -> String {
    const SECTION: &str = "[main_window]";
    const KEY: &str = "infoBoxEnabledWelcome";

    let mut lines: Vec<String> = Vec::new();
    let mut in_section = false;
    let mut written = false;

    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            // Leaving the section without having seen the key: add it here.
            if in_section && !written {
                lines.push(format!("{KEY}=false"));
                written = true;
            }
            in_section = trimmed == SECTION;
        } else if in_section && trimmed.starts_with(KEY) {
            lines.push(format!("{KEY}=false"));
            written = true;
            continue;
        }
        lines.push(line.to_string());
    }

    if !written {
        if !in_section {
            lines.push(String::new());
            lines.push(SECTION.to_string());
        }
        lines.push(format!("{KEY}=false"));
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn disable_welcome_screen(app: &AppHandle) -> Result<(), String> {
    let dir = super::install_dir(app)?.join("GuiConfigs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join("CurrentSettings.ini");
    let existing = std::fs::read_to_string(&file).unwrap_or_default();
    if existing.contains("infoBoxEnabledWelcome=false") {
        return Ok(());
    }
    std::fs::write(&file, with_welcome_disabled(&existing)).map_err(|e| e.to_string())
}

pub fn launch(app: &AppHandle, game: &Game) -> Result<u32, String> {
    let exe = super::exe_path(app)?;
    if !exe.exists() {
        return Err("Install RPCS3 first, then you can play.".to_string());
    }
    if super::firmware::detect_version(app).is_none() {
        return Err("Add PS3 firmware first, then you can play.".to_string());
    }
    if !game.path.is_dir() {
        return Err("This game's folder isn't there. Reconnect the drive it's on.".to_string());
    }
    let eboot = eboot_path(&game.path)
        .ok_or("Couldn't find the game's program file. This folder may be incomplete.")?;

    register(app, game)?;
    disable_welcome_screen(app)?;

    // --no-gui keeps RPCS3's own window out of the way: the user asked to play
    // a game, not to meet the emulator. Spawned rather than waited on, so
    // Omoio stays usable while the game runs.
    let child = super::command(&exe)
        .arg("--no-gui")
        .arg(&eboot)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(child.id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_line_the_way_rpcs3_does() {
        let line = games_list_line(
            "BCES00850",
            Path::new("C:\\Users\\Shadow\\Documents\\RPS3\\games\\LittleBigPlanet 2"),
        );
        assert_eq!(
            line,
            "BCES00850: \"C:/Users/Shadow/Documents/RPS3/games/LittleBigPlanet 2/\""
        );
    }

    #[test]
    fn does_not_double_the_trailing_slash() {
        let line = games_list_line("BCES00141", Path::new("D:/games/LBP/"));
        assert_eq!(line, "BCES00141: \"D:/games/LBP/\"");
    }

    #[test]
    fn turns_off_the_welcome_screen_in_an_existing_config() {
        let existing = "[GSFrame]\nvisibility=Windowed\n\n[main_window]\ninfoBoxEnabledWelcome=true\n\n[Meta]\nattachCommandLine=false\n";
        let out = with_welcome_disabled(existing);

        assert!(out.contains("infoBoxEnabledWelcome=false"));
        assert!(!out.contains("infoBoxEnabledWelcome=true"));
        // Everything else RPCS3 keeps in there survives.
        assert!(out.contains("[GSFrame]"));
        assert!(out.contains("visibility=Windowed"));
        assert!(out.contains("attachCommandLine=false"));
    }

    #[test]
    fn adds_the_setting_when_the_section_has_no_such_key() {
        let out = with_welcome_disabled("[main_window]\nsomethingElse=1\n\n[Meta]\nx=2\n");
        assert!(out.contains("infoBoxEnabledWelcome=false"));
        assert!(out.contains("somethingElse=1"));
        assert!(out.contains("[Meta]"));
        // It has to land inside main_window, not after Meta.
        let welcome = out.find("infoBoxEnabledWelcome").unwrap();
        assert!(welcome < out.find("[Meta]").unwrap());
    }

    #[test]
    fn writes_a_whole_config_when_there_is_none() {
        let out = with_welcome_disabled("");
        assert!(out.contains("[main_window]"));
        assert!(out.contains("infoBoxEnabledWelcome=false"));
    }

    #[test]
    fn keeps_games_it_did_not_put_there() {
        let existing = "BCES00141: C:/games/LBP/\nNPEA00243: C:/games/Sackboy/\n";
        let merged = merged_list(&existing, "BCES00850", Path::new("D:/games/LBP2"));

        assert!(merged.contains("BCES00141: C:/games/LBP/"));
        assert!(merged.contains("NPEA00243: C:/games/Sackboy/"));
        assert!(merged.contains("BCES00850: \"D:/games/LBP2/\""));
        assert_eq!(merged.lines().count(), 3);
    }

    #[test]
    fn replaces_a_game_instead_of_listing_it_twice() {
        let existing = "BCES00850: C:/old/place/\nBCES00141: C:/games/LBP/\n";
        let merged = merged_list(&existing, "BCES00850", Path::new("D:/new/place"));

        assert!(!merged.contains("C:/old/place"));
        assert!(merged.contains("BCES00850: \"D:/new/place/\""));
        assert_eq!(merged.lines().count(), 2);
    }

    #[test]
    fn writes_a_usable_list_from_nothing() {
        let merged = merged_list("", "BCES00850", Path::new("D:/games/LBP2"));
        assert_eq!(merged, "BCES00850: \"D:/games/LBP2/\"\n");
    }

    #[test]
    fn finds_the_binary_in_both_dump_shapes() {
        let root = std::env::temp_dir().join(format!("omoio-eboot-{}", std::process::id()));
        let disc = root.join("disc");
        let hdd = root.join("hdd");
        std::fs::create_dir_all(disc.join("PS3_GAME").join("USRDIR")).unwrap();
        std::fs::create_dir_all(hdd.join("USRDIR")).unwrap();
        std::fs::write(disc.join("PS3_GAME").join("USRDIR").join("EBOOT.BIN"), b"x").unwrap();
        std::fs::write(hdd.join("USRDIR").join("EBOOT.BIN"), b"x").unwrap();

        assert_eq!(
            eboot_path(&disc),
            Some(disc.join("PS3_GAME").join("USRDIR").join("EBOOT.BIN"))
        );
        assert_eq!(eboot_path(&hdd), Some(hdd.join("USRDIR").join("EBOOT.BIN")));
        assert_eq!(eboot_path(&root.join("nothing")), None);

        let _ = std::fs::remove_dir_all(&root);
    }
}
