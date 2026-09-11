//! What RPCS3's compatibility list says about a game.
//!
//! The list is published by RPCS3 as one export of every title they have a
//! result for, which was 6751 of them when this was written. Omoio downloads it
//! whole, keeps the few fields worth showing, and answers from that copy. One
//! download serves the entire library, and a game's status is then instant and
//! works offline.
//!
//! Nothing here blocks anything. A missing or stale list means the interface
//! says it does not know, which is honest and stays out of the way.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

/// Taken from RPCS3's own game_compatibility.cpp rather than guessed. One
/// request, every title, with the newest version Sony published for each.
///
/// What it does not carry is the game's name. The only name in it sits on an
/// update package, so 4121 of the 6753 titles come back nameless, including
/// every Skylanders game before Trap Team.
const EXPORT_URL: &str = "https://rpcs3.net/compatibility?api=v1&export";

/// The same API without `export`, which answers with a `title` for every entry
/// and is what rpcs3.net itself lists from. It is paged at about 40 titles and
/// takes no page-size parameter, so the whole list is 168 requests and around
/// 22 seconds. Measured, not guessed.
///
/// Worth it once: a game's name never changes, so this is fetched with the
/// list and then answers offline for good.
const NAMES_URL: &str = "https://rpcs3.net/compatibility?api=v1&p=";

/// The list moves slowly. A week old is still worth showing, and refreshing is
/// a 6 MB download nobody asked for.
const STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 60 * 60);

/// One title's entry, trimmed to what an interface can use. The export carries
/// far more per title, including whole update manifests, and keeping all of it
/// would mean re-reading megabytes to answer one question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub status: String,
    pub date: String,
    pub update: String,
    /// The game's name. Sony only publishes one alongside an update package,
    /// so a few hundred titles have none and are shown by their id.
    #[serde(default)]
    pub name: String,
}

/// Bumped when we start keeping a field we did not before, so an older cache
/// is refetched rather than answering with blanks.
const CACHE_VERSION: u32 = 4;

#[derive(Serialize, Deserialize)]
struct Cache {
    #[serde(default)]
    version: u32,
    fetched: u64,
    titles: BTreeMap<String, Entry>,
}

/// What RPCS3 means by each status, in their words, from the table in
/// game_compatibility.h. The colours are theirs too, so a status reads the
/// same here as it does anywhere else people discuss these games.
pub fn describe(status: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match status {
        "Playable" => Some((
            "Playable",
            "go",
            "Plays from start to finish.",
        )),
        "Ingame" => Some((
            "Ingame",
            "warn",
            "Runs, but cannot be finished, has serious glitches, or is too slow.",
        )),
        "Intro" => Some((
            "Intro",
            "warn",
            "Shows a picture but does not get past the menus.",
        )),
        "Loadable" => Some((
            "Loadable",
            "bad",
            "Reaches a black screen with a framerate, and no further.",
        )),
        "Nothing" => Some((
            "Nothing",
            "bad",
            "Does not start at all.",
        )),
        _ => None,
    }
}

/// The game's name, which the export only carries alongside an update package,
/// under `patchsets[].packages[].titles[]` with a type of `TITLE`. The other
/// entries there are the same name in Japanese and are not what to show.
///
/// A title Sony never patched has none, so a few hundred come back empty and
/// are shown by their id instead.
fn name_of(entry: &serde_json::Value) -> String {
    entry
        .get("patchsets")
        .and_then(|sets| sets.as_array())
        .into_iter()
        .flatten()
        .filter_map(|set| set.get("packages")?.as_array())
        .flatten()
        .filter_map(|package| package.get("titles")?.as_array())
        .flatten()
        .find(|title| title.get("type").and_then(|t| t.as_str()) == Some("TITLE"))
        .and_then(|title| title.get("title")?.as_str())
        .map(clean_name)
        .unwrap_or_default()
}

/// Sony names the update package, and for a few hundred titles that is the name
/// the export carries: "AC Revelations (Update)", "戦国無双３ Z パッチデータ".
/// The id is the game's, so only the tail is wrong and only the tail is cut.
///
/// Demos and betas keep their names. Those are separate titles with their own
/// id and their own result, and someone who owns the demo should find the demo.
fn clean_name(raw: &str) -> String {
    let folded = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(cut) = update_tail(&folded) else {
        return folded;
    };
    let kept = trim_leftovers(&folded[..cut]);
    if kept.is_empty() {
        folded
    } else {
        kept.to_string()
    }
}

/// Where the update wording starts, if it starts anywhere but the beginning.
/// A name that is only "Update" is left alone: cutting it would leave nothing.
fn update_tail(name: &str) -> Option<usize> {
    let mut earliest: Option<usize> = None;
    let mut note = |at: usize| {
        if at > 0 {
            earliest = Some(earliest.map_or(at, |e: usize| e.min(at)));
        }
    };

    // Whole words, so a game called "Patchwork" survives.
    let mut word_start: Option<usize> = None;
    for (i, ch) in name.char_indices().chain(std::iter::once((name.len(), ' '))) {
        if ch.is_alphanumeric() {
            word_start.get_or_insert(i);
        } else if let Some(from) = word_start.take() {
            if matches!(
                name[from..i].to_lowercase().as_str(),
                "update" | "updates" | "updated" | "patch"
            ) {
                note(from);
            }
        }
    }

    // Japanese runs together with no spaces, so these are matched as they are.
    for marker in ["アップデート", "パッチ"] {
        if let Some(at) = name.find(marker) {
            note(at);
        }
    }

    earliest
}

/// What is left dangling once the update wording is gone: punctuation, and the
/// words that only led into it.
fn trim_leftovers(head: &str) -> &str {
    const FILLER: [&str; 7] = ["and", "dlc", "add-on", "content", "title", "game", "data"];
    let mut kept = head;
    loop {
        let before = kept;
        kept = kept.trim_end_matches(|c: char| {
            c.is_whitespace() || matches!(c, '-' | '–' | '—' | ':' | ',' | '(' | '（' | '[' | '・' | '~' | '～')
        });
        for word in FILLER {
            if let Some(shorter) = strip_word(kept, word) {
                kept = shorter;
            }
        }
        if kept == before {
            return kept;
        }
    }
}

fn strip_word<'a>(head: &'a str, word: &str) -> Option<&'a str> {
    let start = head.len().checked_sub(word.len())?;
    if !head.is_char_boundary(start) || !head[start..].eq_ignore_ascii_case(word) {
        return None;
    }
    // Only a whole word, never the end of a longer one.
    match head[..start].chars().next_back() {
        Some(prev) if prev.is_alphanumeric() => None,
        _ => Some(&head[..start]),
    }
}

fn cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data = app.path().data_dir().map_err(|e| e.to_string())?;
    Ok(data.join("Omoio").join("compatibility.json"))
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_cache(app: &AppHandle) -> Option<Cache> {
    let text = std::fs::read_to_string(cache_path(app).ok()?).ok()?;
    let cache: Cache = serde_json::from_str(&text).ok()?;
    // An older copy is missing fields we now show, so it is treated as absent
    // and fetched again rather than answering with blanks.
    (cache.version == CACHE_VERSION).then_some(cache)
}

/// The status we have for a title, and whether the copy it came from is old
/// enough to be worth refreshing. `None` when we have never downloaded the
/// list, or when the list simply has no result for this title.
pub fn look_up(app: &AppHandle, title_id: &str) -> (Option<Entry>, bool) {
    let Some(cache) = read_cache(app) else {
        return (None, true);
    };
    let stale = now().saturating_sub(cache.fetched) > STALE_AFTER.as_secs();
    (cache.titles.get(title_id).cloned(), stale)
}

pub fn have_list(app: &AppHandle) -> bool {
    read_cache(app).is_some()
}

/// The third character of a title id is the region it was sold in, on both disc
/// ids (BLES, BLUS) and store ids (NPEB, NPUB). Checked against the export: of
/// its 2863 E titles not one carries a Japanese name, while 222 of the 916 J
/// titles do.
///
/// The same game is listed once per region, so without this the catalogue shows
/// two rows called "007 Legends" and no way to tell which is which.
fn region_of(title_id: &str) -> &'static str {
    match title_id.as_bytes().get(2) {
        Some(b'E') => "EU",
        Some(b'U') => "US",
        Some(b'J') => "JP",
        Some(b'K') => "KR",
        // A is Asia and H is Hong Kong, which nothing in the interface tells
        // apart. T and I are store demo discs and system apps, not a region.
        Some(b'A') | Some(b'H') => "Asia",
        _ => "",
    }
}

/// Every title in the list, as the catalogue takes them. `None` until the list
/// has been downloaded.
///
/// Read from the copy we already hold, so the catalogue answers instantly and
/// with the network off.
pub fn entries(app: &AppHandle) -> Option<Vec<crate::core::catalogue::Entry>> {
    let cache = read_cache(app)?;
    Some(
        cache
            .titles
            .into_iter()
            .map(|(title_id, entry)| {
                let region = region_of(&title_id);
                crate::core::catalogue::Entry {
                    console: crate::core::console::Console::Ps3,
                    key: title_id.clone(),
                    named: !entry.name.is_empty(),
                    name: if entry.name.is_empty() {
                        title_id.clone()
                    } else {
                        entry.name
                    },
                    regions: if region.is_empty() { Vec::new() } else { vec![region] },
                    status: describe(&entry.status)
                        .map(|(label, tone, explanation)| crate::core::catalogue::Status {
                            label,
                            tone,
                            explanation,
                        })
                        .unwrap_or_default(),
                    kind: "",
                    title_id,
                }
            })
            .collect(),
    )
}

/// Every title we know the newest version of, so the library can be checked
/// against it without asking Sony about each game in turn.
pub fn newest_versions(app: &AppHandle) -> BTreeMap<String, String> {
    read_cache(app)
        .map(|cache| {
            cache
                .titles
                .into_iter()
                .filter(|(_, entry)| !entry.update.is_empty())
                .map(|(id, entry)| (id, entry.update))
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `newer` is a later game version than `installed`.
///
/// Compared piece by piece as numbers. These look like decimals but are not:
/// 01.10 comes after 01.09, which comparing them as decimals gets backwards.
pub fn is_newer(newer: &str, installed: &str) -> bool {
    fn parts(version: &str) -> Vec<u32> {
        version
            .split('.')
            .map(|piece| piece.trim().parse().unwrap_or(0))
            .collect()
    }
    let (a, b) = (parts(newer), parts(installed));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x > y;
        }
    }
    false
}

fn progress(app: &AppHandle, done: u64, total: u64) {
    let _ = app.emit(
        "compat-progress",
        crate::core::types::Progress {
            stage: "names".to_string(),
            bytes: done,
            total,
        },
    );
}

/// Every title's name, read from the paged API a page at a time.
///
/// Stops on the first page that answers with nothing, since the API has no
/// count to work from. A page that fails is skipped rather than failing the
/// whole list: names are worth having and not worth refusing the list over.
async fn fetch_names(
    app: &AppHandle,
    client: &reqwest::Client,
    cancel: &std::sync::atomic::AtomicBool,
) -> BTreeMap<String, String> {
    // From the 168 pages the list needed when this was written, with room to
    // grow. The loop stops at the end of the data, not at this number; it is
    // here so a change at their end cannot spin forever.
    const MOST_PAGES: u32 = 400;
    let mut names = BTreeMap::new();

    for page in 1..=MOST_PAGES {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        progress(app, page as u64, 168);

        let Ok(response) = client
            .get(format!("{NAMES_URL}{page}"))
            .header("User-Agent", super::USER_AGENT)
            .send()
            .await
        else {
            continue;
        };
        let Ok(body) = response.json::<serde_json::Value>().await else {
            continue;
        };
        let Some(results) = body.get("results").and_then(|r| r.as_object()) else {
            break;
        };
        if results.is_empty() {
            break;
        }

        for (title_id, entry) in results {
            if let Some(title) = entry.get("title").and_then(|t| t.as_str()) {
                names.insert(title_id.clone(), clean_name(title));
            }
        }
    }

    names
}

/// Downloads the list and keeps the parts worth keeping. Returns how many
/// titles it now knows about.
pub async fn refresh(
    app: &AppHandle,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<usize, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(EXPORT_URL)
        .header("User-Agent", super::USER_AGENT)
        .send()
        .await
        .map_err(|_| "Couldn't reach the compatibility list.".to_string())?;

    if !response.status().is_success() {
        return Err("The compatibility list isn't available right now.".into());
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "The compatibility list came back in a form we don't understand.".to_string())?;

    // RPCS3 reports its own trouble in this field, and a returned error still
    // arrives as a perfectly valid response.
    if body.get("return_code").and_then(|c| c.as_i64()).unwrap_or(-255) != 0 {
        return Err("The compatibility list isn't available right now.".into());
    }

    let results = body
        .get("results")
        .and_then(|r| r.as_object())
        .ok_or("The compatibility list came back empty.")?;

    // The names come from a second, paged endpoint. The status list is what
    // matters, so this runs after it and a failure here costs names, not the
    // list. The export's own name is the fallback for anything it misses.
    let names = fetch_names(app, &client, cancel).await;

    let mut titles = BTreeMap::new();
    for (title_id, entry) in results {
        let text = |key: &str| {
            entry
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };
        let status = text("status");
        if status.is_empty() {
            continue;
        }
        titles.insert(
            title_id.clone(),
            Entry {
                status,
                date: text("date"),
                update: text("update"),
                name: names
                    .get(title_id)
                    .cloned()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| name_of(entry)),
            },
        );
    }

    let count = titles.len();
    let path = cache_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let cache = Cache { version: CACHE_VERSION, fetched: now(), titles };
    std::fs::write(
        &path,
        serde_json::to_string(&cache).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_status_rpcs3_publishes_has_words_of_our_own() {
        // The five in their table. Anything else is treated as unknown rather
        // than shown raw.
        for status in ["Playable", "Ingame", "Intro", "Loadable", "Nothing"] {
            assert!(describe(status).is_some(), "{status} has no description");
        }
        assert!(describe("Wishlist").is_none());
        assert!(describe("").is_none());
    }

    #[test]
    fn reads_game_versions_as_pieces_rather_than_decimals() {
        // The one that matters: as decimals 01.10 would look older than 01.09.
        assert!(is_newer("01.10", "01.09"));
        assert!(is_newer("01.33", "01.00"));
        assert!(is_newer("02.00", "01.99"));

        assert!(!is_newer("01.00", "01.33"));
        assert!(!is_newer("01.33", "01.33"), "the same version is not newer");

        // A missing piece counts as zero, so 01.33 and 01.33.0 are the same.
        assert!(!is_newer("01.33", "01.33.0"));
        assert!(is_newer("01.33.1", "01.33"));

        // Nothing to compare against is not an update.
        assert!(!is_newer("", ""));
    }

    #[test]
    fn a_status_is_never_shown_without_a_tone_and_an_explanation() {
        for status in ["Playable", "Ingame", "Intro", "Loadable", "Nothing"] {
            let (label, tone, explanation) = describe(status).unwrap();
            assert!(!label.is_empty());
            assert!(matches!(tone, "go" | "warn" | "bad"), "{status} has tone {tone}");
            assert!(explanation.ends_with('.'), "{status} should read as a sentence");
        }
    }

    /// Every one of these is a real name from the export, kept as a record of
    /// what the wording actually looks like.
    #[test]
    fn update_wording_is_cut_off_the_game_name() {
        for (raw, want) in [
            ("AC Revelations (Update)", "AC Revelations"),
            ("RIDGE RACER 7\n(UPDATE)", "RIDGE RACER 7"),
            ("Beyond: Two Souls™ Update Data (UK)", "Beyond: Two Souls™"),
            ("SingStar® Vol.3 update", "SingStar® Vol.3"),
            ("SEGA Superstars Tennis Update 1.02", "SEGA Superstars Tennis"),
            ("Mafia II - Updated data", "Mafia II"),
            ("ROBOTICS;NOTES-UPDATE", "ROBOTICS;NOTES"),
            ("RESIDENT EVIL 6  PATCH DATA", "RESIDENT EVIL 6"),
            ("DEFIANCE - Title Update (Disc)", "DEFIANCE"),
            ("The Sims™ 3\nDLC and Updates", "The Sims™ 3"),
            (
                "LEGO® STAR WARS™: The Force Awakens: Add-on Content and Patch Data",
                "LEGO® STAR WARS™: The Force Awakens",
            ),
            (
                "Karaoke Revolution Presents: American Idol Encore 2 Game Update",
                "Karaoke Revolution Presents: American Idol Encore 2",
            ),
        ] {
            assert_eq!(clean_name(raw), want, "{raw:?}");
        }
    }

    #[test]
    fn japanese_update_wording_is_cut_too() {
        for (raw, want) in [
            ("戦国無双３ Z パッチデータ", "戦国無双３ Z"),
            ("進撃の巨人-パッチデータ", "進撃の巨人"),
            (
                "テイルズ オブ ベルセリア (アップデートデータ)",
                "テイルズ オブ ベルセリア",
            ),
        ] {
            assert_eq!(clean_name(raw), want, "{raw:?}");
        }
    }

    /// A demo and a beta are their own titles with their own results. Renaming
    /// one to the full game would send someone to the wrong entry.
    #[test]
    fn demos_betas_and_ordinary_names_are_left_alone() {
        for name in [
            "Stacking Demo",
            "LittleBigPlanet™ Beta",
            "God of War: Ascension™ (Multiplayer Beta)",
            "AC Revelations - Multiplayer Beta",
            "Demon's Souls™",
            "Patchwork Heroes",
        ] {
            assert_eq!(clean_name(name), name);
        }
    }

    #[test]
    fn a_title_id_says_which_region_it_was_sold_in() {
        for (id, want) in [
            ("BLES01614", "EU"),
            ("BLUS30983", "US"),
            ("BLJM60180", "JP"),
            ("BCKS10276", "KR"),
            ("BCAS20014", "Asia"),
            ("NPHA80003", "Asia"),
            ("NPEB00174", "EU"),
            ("NPUB30174", "US"),
            // A store demo disc and a system app are not sold in a region.
            ("MRTC00002", ""),
            ("NPIA00002", ""),
            ("XX", ""),
        ] {
            assert_eq!(region_of(id), want, "{id}");
        }
    }

    #[test]
    fn a_name_that_is_only_update_wording_is_kept() {
        assert_eq!(clean_name("Update"), "Update");
        assert_eq!(clean_name("  Patch  Data "), "Patch Data");
    }
}
