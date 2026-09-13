//! The user's own keys for Wii U disc images, handed to Cemu.
//!
//! A disc image, `.wud` or `.wux`, is encrypted. Cemu reads it with the disc's
//! key from `keys.txt` in its user data folder (`KeyCache.cpp`), which for
//! Omoio's portable Cemu is `portable/keys.txt`. Omoio only adds the keys from
//! a file the user brings; Cemu does the decrypting. Omoio never supplies a
//! key and never says where to find one. This is the one exception to the rule
//! against getting round copy protection, granted by Bertram on 13 September
//! 2026.

use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// The key Cemu writes into a new keys.txt as an example. It opens no disc,
/// so it does not count.
const EXAMPLE: &str = "541b9889519b27d363cd21604b97c67a";

/// What Cemu writes at the top of a keys.txt it makes itself.
const HEADER: &str = "# this file contains keys needed for decryption of disc file system data (WUD/WUX)\r\n\
# 1 key per line, any text after a '#' character is considered a comment\r\n\
# the emulator will automatically pick the right key\r\n";

/// The keys in a keys file, read the way Cemu reads them: anything after `#`
/// or `;` is a comment, spaces, tabs, `-` and `_` are dropped, and a line of
/// 32 hex characters is a key. Other lengths Cemu ignores, and so does this.
/// A line that is not hex Cemu shows in an error box, possibly over a game, so
/// the first one is returned to be refused rather than passed on.
fn read(text: &str) -> (Vec<String>, Option<usize>) {
    let mut keys = Vec::new();
    let mut bad = None;
    for (number, line) in text.lines().enumerate() {
        let before_comment = line.split(['#', ';']).next().unwrap_or("");
        let packed: String = before_comment
            .chars()
            .filter(|c| !matches!(c, ' ' | '\t' | '-' | '_' | '\r'))
            .collect();
        if packed.is_empty() {
            continue;
        }
        if !packed.chars().all(|c| c.is_ascii_hexdigit()) {
            bad.get_or_insert(number + 1);
            continue;
        }
        if packed.len() == 32 {
            let key = packed.to_ascii_lowercase();
            if key != EXAMPLE && !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    (keys, bad)
}

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?.join("portable").join("keys.txt"))
}

/// How many keys Cemu has, not counting its example.
pub fn count(app: &AppHandle) -> usize {
    path(app)
        .ok()
        .and_then(|file| std::fs::read_to_string(file).ok())
        .map(|text| read(&text).0.len())
        .unwrap_or(0)
}

/// Adds the keys in the user's file to Cemu's, leaving those already there.
/// Returns how many were new.
pub fn add(app: &AppHandle, from: &Path) -> Result<usize, String> {
    if !super::install_dir(app)?.join("Cemu.exe").is_file() {
        return Err("Install Cemu first, then add your keys.".to_string());
    }
    let text = std::fs::read_to_string(from).map_err(|_| "Couldn't read that file.".to_string())?;
    merged(path(app)?.as_path(), &text)
}

fn merged(file: &Path, text: &str) -> Result<usize, String> {
    let (new, bad) = read(text);
    if let Some(line) = bad {
        return Err(format!(
            "Line {line} of that file isn't a key. A keys file has one key of 32 letters and digits per line."
        ));
    }
    if new.is_empty() {
        // Cemu makes this file itself, with one example key, so it is the
        // one people most often pick by mistake.
        return Err(if text.to_ascii_lowercase().contains(EXAMPLE) {
            "That's Cemu's own starting file. Its one key is an example that opens no disc, so there is nothing to add."
                .to_string()
        } else {
            "That file has no keys in it.".to_string()
        });
    }
    let existing = std::fs::read_to_string(file).unwrap_or_default();
    let (have, _) = read(&existing);
    let fresh: Vec<String> = new.into_iter().filter(|key| !have.contains(key)).collect();
    if fresh.is_empty() {
        return Ok(0);
    }
    let mut out = if existing.trim().is_empty() {
        HEADER.to_string()
    } else {
        existing.clone()
    };
    if !out.ends_with('\n') {
        out.push_str("\r\n");
    }
    for key in &fresh {
        out.push_str(key);
        out.push_str("\r\n");
    }
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "Couldn't save the keys.".to_string())?;
    }
    std::fs::write(file, out).map_err(|_| "Couldn't save the keys.".to_string())?;
    Ok(fresh.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "00112233445566778899aabbccddeeff";

    #[test]
    fn keys_are_read_the_way_cemu_reads_them() {
        let text = "# a comment\r\n\
                    0011-2233 4455_6677 8899AABBCCDDEEFF # my disc\r\n\
                    ; another comment\r\n\
                    abcd\r\n\
                    541b9889519b27d363cd21604b97c67a # example key (can be deleted)\r\n";
        let (keys, bad) = read(text);
        assert_eq!(keys, [KEY], "separators and case dropped, short line and example ignored");
        assert_eq!(bad, None);
    }

    #[test]
    fn a_line_that_is_not_hex_is_reported() {
        let (_, bad) = read(&format!("{KEY}\nnot a key\n"));
        assert_eq!(bad, Some(2));
    }

    #[test]
    fn new_keys_are_added_to_cemus_and_old_ones_kept() {
        let dir = std::env::temp_dir().join(format!("omoio-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("keys.txt");

        assert_eq!(merged(&file, KEY).unwrap(), 1);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("# this file contains keys"), "Cemu's own header");
        assert!(text.contains(KEY));

        assert_eq!(merged(&file, KEY).unwrap(), 0, "already there");
        let other = "ffeeddccbbaa99887766554433221100";
        assert_eq!(merged(&file, &format!("{KEY}\n{other}\n")).unwrap(), 1);
        assert_eq!(read(&std::fs::read_to_string(&file).unwrap()).0.len(), 2);

        assert!(merged(&file, "no keys here\n").is_err());
        assert!(merged(&file, "# only a comment\n").is_err());
        let starting_file = format!("{HEADER}{EXAMPLE} # example key (can be deleted)\r\n");
        assert!(merged(&file, &starting_file).unwrap_err().contains("Cemu's own starting file"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
