//! The community patches RPCS3 publishes, per game.
//!
//! Two files, both RPCS3's, both in its install:
//!
//! - `patches/patch.yml` is the catalogue. It is real YAML, with anchors and
//!   aliases and the same key repeated at the top level, which is why this is
//!   the one place Omoio uses a YAML parser rather than reading lines.
//! - `config/patch_config.yml` is what is switched on. RPCS3 writes only the
//!   patches that are enabled, so switching one off means removing its entry,
//!   not writing `false`.
//!
//! Both shapes were read out of RPCS3's bin_patch.cpp rather than guessed, and
//! a test loads the real published file to check this still reads it.
//!
//! Nothing is ever enabled on the user's behalf.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;
use yaml_rust2::{yaml::Hash, Yaml, YamlEmitter, YamlLoader};

/// From RPCS3's patch_manager_dialog.cpp. The version is the patch engine's,
/// declared in bin_patch.h, and the server returns a file for that version.
const PATCH_ENGINE_VERSION: &str = "1.2";

fn patch_url() -> String {
    format!("https://rpcs3.net/compatibility?patch&api=v1&v={PATCH_ENGINE_VERSION}")
}

/// Keys as RPCS3 spells them, from patch_key in bin_patch.h.
const K_GAMES: &str = "Games";
const K_AUTHOR: &str = "Author";
const K_NOTES: &str = "Notes";
const K_PATCH_VERSION: &str = "Patch Version";
const K_ENABLED: &str = "Enabled";
const K_ALL: &str = "All";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    /// RPCS3 keys everything on the hash of the executable, so it travels with
    /// these values and is not ours to shorten.
    pub hash: String,
    pub name: String,
    pub game: String,
    pub author: String,
    pub notes: String,
    pub version: String,
    /// The game versions this patch was written for.
    pub versions: Vec<String>,
    /// Whether it covers the copy of the game actually installed.
    pub applies: bool,
    pub enabled: bool,
}

pub fn catalogue_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?.join("patches").join("patch.yml"))
}

/// `config/patch_config.yml`, next to RPCS3's other settings. On Windows
/// RPCS3 asks for its config subdirectory here, which its own File.cpp
/// resolves to `config/` beside the executable.
pub fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?.join("config").join("patch_config.yml"))
}

pub fn have_catalogue(app: &AppHandle) -> bool {
    catalogue_path(app).is_ok_and(|p| p.exists())
}

/// A scalar as it was written. Version numbers like `01.33` would come back
/// from a number-aware reader as 1.33, and RPCS3 matches these as text, so the
/// original spelling is the only safe thing to keep.
fn scalar(value: &Yaml) -> Option<String> {
    match value {
        Yaml::String(s) => Some(s.clone()),
        Yaml::Real(s) => Some(s.clone()),
        Yaml::Integer(i) => Some(i.to_string()),
        Yaml::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Looks a key up the way yaml-cpp resolves it: the last one written wins.
///
/// Keys repeated inside a mapping have been padded apart so the document parses
/// (see `make_keys_unique`), and one patch in the published file really does
/// name `Notes` twice. Reading the last match keeps us agreeing with RPCS3
/// about what the file says.
fn last<'a>(map: &'a Hash, key: &str) -> Option<&'a Yaml> {
    map.iter()
        .filter(|(k, _)| k.as_str().is_some_and(|k| k.trim_end() == key))
        .next_back()
        .map(|(_, value)| value)
}

fn text(map: &Hash, key: &str) -> String {
    last(map, key).and_then(scalar).unwrap_or_default()
}

/// The published file names the same key twice in the same mapping, in two
/// places: 166 times at the top level, once for every `Anchors` block and for
/// ten real entries, and inside patches too, where one names `Notes` twice.
/// yaml-cpp, which RPCS3 reads it with, keeps the last and carries on. Our
/// parser refuses the document outright.
///
/// Dropping the earlier ones is not the same thing, because yaml-cpp still
/// reads them and every anchor they define stays defined. So the repeats are
/// padded apart with trailing spaces instead. Everything parses, every anchor
/// survives, and `last` then reads the one RPCS3 would have used.
///
/// This is safe to do by line because the file has no block scalars, so no
/// indented text can be mistaken for a key.
fn make_keys_unique(text: &str) -> String {
    // One counter per nesting level, dropped as soon as that level closes.
    let mut levels: Vec<(usize, std::collections::HashMap<String, usize>)> = Vec::new();
    let mut out = String::with_capacity(text.len() + 8192);

    for line in text.lines() {
        let Some((indent, key, rest)) = split_key(line) else {
            out.push_str(line);
            out.push('\n');
            continue;
        };

        while levels.last().is_some_and(|(at, _)| *at > indent) {
            levels.pop();
        }
        if levels.last().map(|(at, _)| *at) != Some(indent) {
            levels.push((indent, std::collections::HashMap::new()));
        }

        let seen = &mut levels.last_mut().expect("just pushed").1;
        let count = seen.entry(key.to_string()).or_insert(0);
        *count += 1;

        out.push_str(&" ".repeat(indent));
        if *count > 1 {
            out.push_str(&format!("\"{}{}\"", key, " ".repeat(*count - 1)));
        } else {
            out.push_str(&format!("\"{key}\""));
        }
        out.push_str(rest);
        out.push('\n');
    }
    out
}

/// Splits a mapping line into its indent, the key's text, and everything from
/// the colon onwards. `None` for anything that is not a key: list items,
/// comments, and blank lines.
fn split_key(line: &str) -> Option<(usize, &str, &str)> {
    let indent = line.len() - line.trim_start().len();
    let body = &line[indent..];
    if body.is_empty() || body.starts_with('#') || body.starts_with('-') {
        return None;
    }

    // A key may be quoted, and one of them really is "32:9", so the colon
    // inside it must not be mistaken for the separator.
    if let Some(unquoted) = body.strip_prefix('"') {
        let end = unquoted.find('"')?;
        let rest = &unquoted[end + 1..];
        if !rest.starts_with(':') {
            return None;
        }
        return Some((indent, &unquoted[..end], rest));
    }

    let colon = body.find(':')?;
    let key = &body[..colon];
    if key.is_empty() || key.contains('\'') {
        return None;
    }
    Some((indent, key, &body[colon..]))
}

fn load(path: &PathBuf) -> Result<Yaml, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse(&raw)
}

fn parse(raw: &str) -> Result<Yaml, String> {
    let mut docs = YamlLoader::load_from_str(&make_keys_unique(raw))
        .map_err(|_| "Couldn't read the patch list.".to_string())?;
    if docs.is_empty() {
        return Err("The patch list is empty.".into());
    }
    Ok(docs.remove(0))
}

/// Every patch published for this game, whether or not it fits the copy
/// installed. One that needs another version is still worth showing, because
/// otherwise it looks like no patch exists.
pub fn for_title(app: &AppHandle, title_id: &str, app_version: &str) -> Vec<Patch> {
    let Ok(root) = catalogue_path(app).and_then(|p| load(&p)) else {
        return Vec::new();
    };
    let Some(top) = root.as_hash() else {
        return Vec::new();
    };
    let enabled = read_enabled(app);
    let mut found = Vec::new();

    for (hash, entries) in patch_entries(top) {
        for (name, info) in entries_last_wins(entries) {
            let Some(info) = info.as_hash() else { continue };
            let Some(games) = last(info, K_GAMES).and_then(Yaml::as_hash) else {
                continue;
            };

            for (game, serials) in entries_last_wins(games) {
                let Some(serials) = serials.as_hash() else { continue };
                let Some(versions) = last(serials, title_id).and_then(Yaml::as_vec) else {
                    continue;
                };
                let versions: Vec<String> = versions.iter().filter_map(scalar).collect();
                let applies = versions.iter().any(|v| v == K_ALL || v == app_version);

                found.push(Patch {
                    applies,
                    enabled: enabled.iter().any(|(h, n)| *h == hash && *n == name),
                    hash: hash.clone(),
                    name: name.clone(),
                    game: game.clone(),
                    author: text(info, K_AUTHOR),
                    notes: text(info, K_NOTES),
                    version: text(info, K_PATCH_VERSION),
                    versions,
                });
            }
        }
    }

    // Ones that fit first, then by name, so the list does not reshuffle.
    found.sort_by(|a, b| b.applies.cmp(&a.applies).then_with(|| a.name.cmp(&b.name)));
    found
}

/// The top-level entries that actually hold patches, keyed by hash with the
/// padding stripped back off.
///
/// Which prefixes RPCS3 uses is not worth guessing at: the file has PPU, SPU,
/// OVL and PRX today and could gain another. An entry is a patch entry when it
/// contains something that names the games it is for, which is what makes it
/// usable at all. Repeats keep the last, as yaml-cpp does.
fn patch_entries(top: &Hash) -> Vec<(String, &Hash)> {
    entries_last_wins(top)
        .into_iter()
        .filter_map(|(hash, value)| Some((hash, value.as_hash()?)))
        .filter(|(_, entries)| {
            entries
                .values()
                .any(|info| info.as_hash().is_some_and(|i| last(i, K_GAMES).is_some()))
        })
        .collect()
}

/// A mapping's entries with repeats collapsed to the last one written, and the
/// padding that kept them apart stripped back off.
fn entries_last_wins(map: &Hash) -> Vec<(String, &Yaml)> {
    let mut out: Vec<(String, &Yaml)> = Vec::new();
    for (key, value) in map {
        let Some(key) = key.as_str() else { continue };
        let key = key.trim_end().to_string();
        match out.iter_mut().find(|(seen, _)| *seen == key) {
            Some(slot) => slot.1 = value,
            None => out.push((key, value)),
        }
    }
    out
}

/// Which patches are switched on, as hash and name. RPCS3 writes an entry only
/// while a patch is enabled, so presence is the whole answer.
fn read_enabled(app: &AppHandle) -> Vec<(String, String)> {
    let Ok(root) = config_path(app).and_then(|p| load(&p)) else {
        return Vec::new();
    };
    let Some(top) = root.as_hash() else {
        return Vec::new();
    };
    let mut on = Vec::new();
    for (hash_key, entries) in top {
        let (Some(hash), Some(entries)) = (hash_key.as_str(), entries.as_hash()) else {
            continue;
        };
        for (name_key, _) in entries {
            if let Some(name) = name_key.as_str() {
                on.push((hash.to_string(), name.to_string()));
            }
        }
    }
    on
}

fn nested<'a>(map: &'a mut Hash, key: &str) -> &'a mut Hash {
    let key = Yaml::String(key.into());
    if !matches!(map.get(&key), Some(Yaml::Hash(_))) {
        map.insert(key.clone(), Yaml::Hash(Hash::new()));
    }
    match map.get_mut(&key) {
        Some(Yaml::Hash(inner)) => inner,
        _ => unreachable!("just inserted a map"),
    }
}

/// Switches one patch on or off for one game, leaving every other entry in
/// RPCS3's file alone.
pub fn set_enabled(
    app: &AppHandle,
    patch: &Patch,
    title_id: &str,
    app_version: &str,
    on: bool,
) -> Result<(), String> {
    let path = config_path(app)?;
    let mut root = match load(&path) {
        Ok(Yaml::Hash(hash)) => hash,
        _ => Hash::new(),
    };

    // RPCS3 records the version the patch is being used against. "All"
    // patches have no version of their own, so the game's own is what applies.
    let version = if patch.versions.iter().any(|v| v == K_ALL) && !patch.versions.iter().any(|v| v == app_version)
    {
        K_ALL.to_string()
    } else {
        app_version.to_string()
    };

    if on {
        let by_name = nested(&mut root, &patch.hash);
        let by_game = nested(by_name, &patch.name);
        let by_serial = nested(by_game, &patch.game);
        let by_version = nested(by_serial, title_id);
        let leaf = nested(by_version, &version);
        leaf.insert(Yaml::String(K_ENABLED.into()), Yaml::Boolean(true));
    } else {
        remove(&mut root, &[&patch.hash, &patch.name, &patch.game, title_id, &version]);
    }

    if root.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut out = String::new();
    YamlEmitter::new(&mut out)
        .dump(&Yaml::Hash(root))
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, out).map_err(|e| e.to_string())
}

/// Takes one entry out and tidies up behind it, so switching a patch off does
/// not leave a trail of empty maps RPCS3 has to wade through.
fn remove(map: &mut Hash, path: &[&str]) {
    let Some((head, rest)) = path.split_first() else {
        return;
    };
    let key = Yaml::String((*head).into());
    if rest.is_empty() {
        map.remove(&key);
        return;
    }
    if let Some(Yaml::Hash(inner)) = map.get_mut(&key) {
        remove(inner, rest);
        if inner.is_empty() {
            map.remove(&key);
        }
    }
}

/// Downloads the catalogue into RPCS3's own patches folder, where RPCS3 reads
/// it from. Returns how many patch entries it holds.
pub async fn refresh(app: &AppHandle) -> Result<usize, String> {
    let response = reqwest::Client::new()
        .get(patch_url())
        .header("User-Agent", super::USER_AGENT)
        .send()
        .await
        .map_err(|_| "Couldn't reach the patch list.".to_string())?;

    if !response.status().is_success() {
        return Err("The patch list isn't available right now.".into());
    }

    // The file arrives wrapped in JSON, with its own checksum beside it.
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "The patch list came back in a form we don't understand.".to_string())?;

    if body.get("return_code").and_then(|c| c.as_i64()).unwrap_or(-255) != 0 {
        return Err("The patch list isn't available right now.".into());
    }
    let yaml = body
        .get("patch")
        .and_then(|p| p.as_str())
        .ok_or("The patch list came back empty.")?;

    if let Some(expected) = body.get("sha256").and_then(|s| s.as_str()) {
        if !matches_checksum(yaml, expected) {
            return Err("The patch list arrived damaged. Try again.".into());
        }
    }

    let path = catalogue_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, yaml).map_err(|e| e.to_string())?;

    Ok(count_entries(yaml))
}

fn matches_checksum(text: &str, expected: &str) -> bool {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let got: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    got.eq_ignore_ascii_case(expected)
}

fn count_entries(yaml: &str) -> usize {
    yaml.lines()
        .filter(|line| line.starts_with("PPU") || line.starts_with("SPU"))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_version_spelled_the_way_it_was_written() {
        // Read as a number, 01.33 comes back 1.33 and stops matching.
        let docs = YamlLoader::load_from_str("v: [ 01.33, 1.00, All ]").unwrap();
        let list = docs[0]["v"].as_vec().unwrap();
        let versions: Vec<String> = list.iter().filter_map(scalar).collect();
        assert_eq!(versions, vec!["01.33", "1.00", "All"]);
    }

    #[test]
    fn resolves_the_aliases_the_patch_file_is_built_from() {
        // Every real entry names its games through an anchor like this one.
        let text = "\
Anchors:
  lbp: &lbp
    \"LittleBigPlanet 2\":
      BCES00850: [ 01.33 ]
PPU-abc:
  \"Unlock FPS\":
    Games: *lbp
    Author: \"someone\"
";
        let docs = YamlLoader::load_from_str(text).unwrap();
        let games = &docs[0]["PPU-abc"]["Unlock FPS"][K_GAMES];
        assert!(games.as_hash().is_some(), "alias was not resolved");
        assert!(games["LittleBigPlanet 2"]["BCES00850"].as_vec().is_some());
    }

    #[test]
    fn switching_a_patch_off_leaves_nothing_behind() {
        let mut root = Hash::new();
        {
            let a = nested(&mut root, "PPU-abc");
            let b = nested(a, "Unlock FPS");
            let c = nested(b, "LittleBigPlanet 2");
            let d = nested(c, "BCES00850");
            let leaf = nested(d, "01.33");
            leaf.insert(Yaml::String(K_ENABLED.into()), Yaml::Boolean(true));
        }
        remove(
            &mut root,
            &["PPU-abc", "Unlock FPS", "LittleBigPlanet 2", "BCES00850", "01.33"],
        );
        assert!(root.is_empty(), "empty maps were left behind");
    }

    #[test]
    fn switching_one_off_leaves_the_others_alone() {
        let mut root = Hash::new();
        {
            let leaf = nested(
                nested(nested(nested(nested(&mut root, "PPU-abc"), "One"), "Game"), "SER"),
                "01.00",
            );
            leaf.insert(Yaml::String(K_ENABLED.into()), Yaml::Boolean(true));
            let other = nested(
                nested(nested(nested(nested(&mut root, "PPU-abc"), "Two"), "Game"), "SER"),
                "01.00",
            );
            other.insert(Yaml::String(K_ENABLED.into()), Yaml::Boolean(true));
        }
        remove(&mut root, &["PPU-abc", "One", "Game", "SER", "01.00"]);
        assert!(root.get(&Yaml::String("PPU-abc".into())).is_some());
        let kept = root[&Yaml::String("PPU-abc".into())].as_hash().unwrap();
        assert!(kept.get(&Yaml::String("Two".into())).is_some());
        assert!(kept.get(&Yaml::String("One".into())).is_none());
    }

    #[test]
    fn a_damaged_download_is_refused() {
        // sha256 of "hello"
        let good = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(matches_checksum("hello", good));
        assert!(!matches_checksum("hello world", good));
    }

    /// Reads the file RPCS3 actually publishes. Set OMOIO_PATCH_YML to a copy
    /// of it to run this.
    #[test]
    #[ignore]
    fn reads_the_published_patch_file() {
        let path = std::env::var("OMOIO_PATCH_YML").expect("set OMOIO_PATCH_YML");
        let root = load(&PathBuf::from(path)).expect("parsed");
        let top = root.as_hash().expect("a map at the top");
        let entries = patch_entries(top);
        assert!(entries.len() > 100, "only found {} patch entries", entries.len());

        // A repeated hash must appear once, not twice, or a patch RPCS3 never
        // applies would be offered as though it did.
        let mut names: Vec<&String> = entries.iter().map(|(hash, _)| hash).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(before, names.len(), "the same hash was kept twice");

        // No key should still be carrying the padding used to make it unique.
        assert!(!names.iter().any(|hash| hash.ends_with(' ')));

        // Nearly every patch names its games through an alias, so this fails
        // loudly if alias support ever goes away.
        let with_games: usize = entries
            .iter()
            .map(|(_, entry)| {
                entry
                    .values()
                    .filter(|info| {
                        info.as_hash()
                            .and_then(|i| i.get(&Yaml::String(K_GAMES.into())))
                            .and_then(Yaml::as_hash)
                            .is_some()
                    })
                    .count()
            })
            .sum();
        assert!(with_games > 100, "only {with_games} patches named their games");
    }

    #[test]
    fn a_repeated_key_parses_and_the_last_one_wins() {
        // Both halves of what the published file does: repeated Anchors blocks
        // whose anchors must all survive, and a repeated entry.
        let text = "\
Anchors:
  first: &first
    \"Game One\":
      SER00001: [ 01.00 ]
Anchors:
  second: &second
    \"Game Two\":
      SER00002: [ 01.00 ]
PPU-abc:
  \"Old\":
    Games: *first
PPU-abc:
  \"New\":
    Games: *second
";
        let root = parse(text).expect("a repeated key should not stop the file being read");
        let entries = patch_entries(root.as_hash().unwrap());

        assert_eq!(entries.len(), 1, "the repeated hash should appear once");
        let (hash, patches) = &entries[0];
        assert_eq!(hash, "PPU-abc", "the padding should be gone");
        // yaml-cpp keeps the last, so RPCS3 only ever applies "New".
        assert!(patches.contains_key(&Yaml::String("New".into())));
        assert!(!patches.contains_key(&Yaml::String("Old".into())));
        // The anchor from the first block still resolved, which is why the
        // blocks are renamed rather than dropped.
        assert!(root["Anchors"]["first"].as_hash().is_some());
    }
}
