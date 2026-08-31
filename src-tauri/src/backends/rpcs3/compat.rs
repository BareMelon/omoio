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
use tauri::{AppHandle, Manager};

/// Taken from RPCS3's own game_compatibility.cpp rather than guessed.
const EXPORT_URL: &str = "https://rpcs3.net/compatibility?api=v1&export";

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
}

#[derive(Serialize, Deserialize)]
struct Cache {
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
    serde_json::from_str(&text).ok()
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

/// Downloads the list and keeps the parts worth keeping. Returns how many
/// titles it now knows about.
pub async fn refresh(app: &AppHandle) -> Result<usize, String> {
    let response = reqwest::Client::new()
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
            Entry { status, date: text("date"), update: text("update") },
        );
    }

    let count = titles.len();
    let path = cache_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let cache = Cache { fetched: now(), titles };
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
    fn a_status_is_never_shown_without_a_tone_and_an_explanation() {
        for status in ["Playable", "Ingame", "Intro", "Loadable", "Nothing"] {
            let (label, tone, explanation) = describe(status).unwrap();
            assert!(!label.is_empty());
            assert!(matches!(tone, "go" | "warn" | "bad"), "{status} has tone {tone}");
            assert!(explanation.ends_with('.'), "{status} should read as a sentence");
        }
    }
}
