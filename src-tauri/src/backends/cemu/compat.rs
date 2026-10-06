//! What the Cemu wiki says about how well each Wii U game runs.
//!
//! Cemu's compatibility list is built from its wiki: one page per game, with
//! an infobox carrying the rating and the regions it was released in. The
//! wiki's MediaWiki API hands over those pages' own text, so the list is read
//! from there rather than from the HTML page made out of it. That was 1213
//! games when this was written, in 28 requests and about five seconds.
//!
//! The wiki states no licence for its text. What is kept is a game's name, its
//! rating and where it was released, which are facts, and the catalogue
//! credits the wiki with a link.

use crate::core::catalogue::{Entry, Status};
use crate::core::console::Console;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};

const API: &str = "https://wiki.cemu.info/api.php";

/// Every game page is built on this infobox, and only game pages are.
const INFOBOX: &str = "Template:Infobox VG";

/// The most pages MediaWiki sends with their text in one answer to anyone
/// who is not a bot.
const BATCH: usize = 50;

/// Bumped when a field is added, so an older copy is fetched again.
const CACHE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Cache {
    version: u32,
    fetched: u64,
    games: Vec<Game>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Game {
    page_id: u64,
    title: String,
    rating: String,
    /// "Virtual Console" for an older console's game sold again on the Wii U.
    kind: String,
    /// The wiki's own region codes, such as NA or PAL.
    released: Vec<String>,
}

/// What the wiki means by each rating, from its own Category:Rating page, in
/// fewer words. "Unknown" is the wiki saying nobody has rated the game.
pub fn describe(rating: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match rating {
        "Perfect" => Some((
            "Perfect",
            "go",
            "Runs with no glitches, crashes or bugs the original did not have.",
        )),
        "Playable" => Some((
            "Playable",
            "go",
            "Plays to the end, with minor glitches at most.",
        )),
        "Runs" => Some((
            "Runs",
            "warn",
            "Gets in game, but major glitches or bugs make it hard to finish.",
        )),
        "Loads" => Some((
            "Loads",
            "bad",
            "Gets to a screen or the menus, then crashes.",
        )),
        "Unplayable" => Some((
            "Unplayable",
            "bad",
            "Does not load. It crashes straight away or stays on a black screen.",
        )),
        _ => None,
    }
}

/// What a rating means for someone about to import the game, said after
/// "Cemu rates it Runs:". Empty for Perfect and Playable, which need no
/// warning.
pub fn caution(rating: &str) -> &'static str {
    match rating {
        "Runs" => "it gets into the game, but major glitches make it hard to finish.",
        "Loads" => "it gets as far as the menus, then crashes.",
        "Unplayable" => "it doesn't load.",
        _ => "",
    }
}

/// One infobox field's value, without the editing notes the wiki leaves in
/// some of them: `|rating = Playable <!-- Unplayable, Loads, ... -->`.
fn field<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let (key, value) = line.trim_start().strip_prefix('|')?.split_once('=')?;
        (key.trim() == name).then(|| value.split("<!--").next().unwrap_or_default().trim())
    })
}

/// The regions in `{{vgrelease|NA=June 3, 2014|PAL=October 1, 2015|JP=}}`
/// that have a date. A region left empty is one the page's editor had no date
/// for, which says nothing about a release there.
fn release_codes(text: &str) -> Vec<String> {
    const OPEN: &str = "{{vgrelease|";
    let Some(start) = text.find(OPEN) else {
        return Vec::new();
    };
    let body = &text[start + OPEN.len()..];
    let body = &body[..body.find("}}").unwrap_or(body.len())];
    body.split('|')
        .filter_map(|part| {
            let (code, date) = part.split_once('=')?;
            (!date.trim().is_empty()).then(|| code.trim().to_ascii_uppercase())
        })
        .collect()
}

/// The wiki's region codes as the catalogue's region labels. PAL covers
/// Europe and Australia, as the region letter in a PS3 title id does. WW is
/// the eShop's worldwide release.
fn regions_of(codes: &[String]) -> Vec<&'static str> {
    let mut regions = Vec::new();
    for code in codes {
        let labels: &[&'static str] = match code.as_str() {
            "NA" | "US" | "USA" => &["US"],
            "EU" | "PAL" | "UK" | "AU" | "AUS" | "EUR" => &["EU"],
            "JP" | "JPN" => &["JP"],
            "KR" | "KOR" => &["KR"],
            "WW" => &["EU", "US", "JP"],
            _ => &[],
        };
        for label in labels {
            if !regions.contains(label) {
                regions.push(*label);
            }
        }
    }
    regions
}

fn read_page(page_id: u64, title: &str, text: &str) -> Game {
    let kind = match field(text, "type") {
        Some(kind) if kind.eq_ignore_ascii_case("Virtual Console") => "Virtual Console",
        _ => "",
    };
    Game {
        page_id,
        title: title.to_string(),
        rating: field(text, "rating")
            .and_then(|value| value.split_whitespace().next())
            .unwrap_or_default()
            .to_string(),
        kind: kind.to_string(),
        released: release_codes(text),
    }
}

fn cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data = app.path().data_dir().map_err(|e| e.to_string())?;
    Ok(data.join("Omoio").join("cemu-compatibility.json"))
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
    (cache.version == CACHE_VERSION).then_some(cache)
}

/// Every game in the list, as the catalogue takes them. `None` until the list
/// has been downloaded.
pub fn entries(app: &AppHandle) -> Option<Vec<Entry>> {
    let cache = read_cache(app)?;
    Some(
        cache
            .games
            .into_iter()
            .map(|game| Entry {
                console: Console::WiiU,
                key: format!("WU{}", game.page_id),
                named: true,
                title_id: String::new(),
                regions: regions_of(&game.released),
                status: describe(&game.rating)
                    .map(|(label, tone, explanation)| Status {
                        label,
                        tone,
                        explanation,
                        caution: caution(&game.rating),
                    })
                    .unwrap_or_default(),
                kind: if game.kind == "Virtual Console" {
                    "Virtual Console"
                } else {
                    ""
                },
                name: game.title,
            })
            .collect(),
    )
}

fn progress(app: &AppHandle, done: u64, total: u64) {
    let _ = app.emit(
        "compat-progress",
        crate::core::types::Progress {
            stage: "wiiu".to_string(),
            bytes: done,
            total,
        },
    );
}

async fn get(client: &reqwest::Client, params: &[(&str, &str)]) -> Result<serde_json::Value, String> {
    let url = reqwest::Url::parse_with_params(API, params).map_err(|e| e.to_string())?;
    let response = client
        .get(url)
        .header("User-Agent", super::USER_AGENT)
        .send()
        .await
        .map_err(|_| "Couldn't reach the Cemu wiki.".to_string())?;
    if !response.status().is_success() {
        return Err("The Cemu wiki isn't answering right now.".into());
    }
    response
        .json()
        .await
        .map_err(|_| "The Cemu wiki answered in a form Omoio doesn't understand.".to_string())
}

/// The id of every page built on the game infobox, 500 to a request.
async fn game_pages(client: &reqwest::Client) -> Result<Vec<u64>, String> {
    // Three requests were enough when this was written. The loop stops when
    // the wiki says there is no more; this is here so a change at their end
    // cannot spin forever.
    const MOST_REQUESTS: usize = 20;
    let mut ids = Vec::new();
    let mut next: Option<String> = None;
    for _ in 0..MOST_REQUESTS {
        let from = next.take();
        let mut params = vec![
            ("action", "query"),
            ("list", "embeddedin"),
            ("eititle", INFOBOX),
            ("einamespace", "0"),
            ("eilimit", "500"),
            ("format", "json"),
            ("formatversion", "2"),
        ];
        if let Some(from) = &from {
            params.push(("eicontinue", from.as_str()));
        }
        let body = get(client, &params).await?;
        ids.extend(
            body.pointer("/query/embeddedin")
                .and_then(|pages| pages.as_array())
                .into_iter()
                .flatten()
                .filter_map(|page| page.get("pageid")?.as_u64()),
        );
        next = body
            .pointer("/continue/eicontinue")
            .and_then(|c| c.as_str())
            .map(String::from);
        if next.is_none() {
            break;
        }
    }
    Ok(ids)
}

/// Downloads the list and keeps what the catalogue shows. Returns how many
/// games it now knows about.
pub async fn refresh(app: &AppHandle, cancel: &AtomicBool) -> Result<usize, String> {
    let client = reqwest::Client::new();
    let ids = game_pages(&client).await?;
    if ids.is_empty() {
        return Err("The Cemu wiki listed no games.".into());
    }

    let batches = ids.len().div_ceil(BATCH) as u64;
    let mut games = Vec::with_capacity(ids.len());
    for (done, chunk) in ids.chunks(BATCH).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        progress(app, done as u64, batches);
        let pages = chunk.iter().map(u64::to_string).collect::<Vec<_>>().join("|");
        let body = get(
            &client,
            &[
                ("action", "query"),
                ("pageids", pages.as_str()),
                ("prop", "revisions"),
                ("rvprop", "content"),
                ("rvslots", "main"),
                ("format", "json"),
                ("formatversion", "2"),
            ],
        )
        .await?;
        for page in body
            .pointer("/query/pages")
            .and_then(|pages| pages.as_array())
            .into_iter()
            .flatten()
        {
            let page_id = page.get("pageid").and_then(|v| v.as_u64());
            let title = page.get("title").and_then(|v| v.as_str());
            let text = page
                .pointer("/revisions/0/slots/main/content")
                .and_then(|v| v.as_str());
            if let (Some(page_id), Some(title), Some(text)) = (page_id, title, text) {
                games.push(read_page(page_id, title, text));
            }
        }
    }
    progress(app, batches, batches);

    games.sort_by(|a, b| a.title.cmp(&b.title));
    let count = games.len();
    let path = cache_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let cache = Cache {
        version: CACHE_VERSION,
        fetched: now(),
        games,
    };
    std::fs::write(&path, serde_json::to_string(&cache).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of a real game page: the infobox, with the wiki's editing
    /// notes left in, and a test report below it carrying its own rating.
    const PAGE: &str = "{{stub}}
{{Infobox VG
|title = 1001 Spikes
|type = eShop
|released = {{vgrelease|NA=June 3, 2014|PAL=October 1, 2015|JP=November 25, 2015|}} <!-- Put them in chronological order -->
|rating = Playable <!-- Unplayable, Loads, Runs, Playable, Perfect -->
}}
{{testline|version=1.6.0|region=EUR|rating=Perfect|notes=}}";

    #[test]
    fn a_page_gives_its_rating_kind_and_regions() {
        let game = read_page(7, "1001 Spikes", PAGE);
        assert_eq!(game.rating, "Playable", "the infobox, not the test report");
        assert_eq!(game.kind, "");
        assert_eq!(game.released, ["NA", "PAL", "JP"]);
    }

    #[test]
    fn a_region_without_a_date_is_left_out() {
        let page = "|type = Virtual Console
|released = {{vgrelease|NA=Dec 31, 2015|PAL=Dec 31, 2015|JP=}}
|rating = Runs";
        let game = read_page(8, "1080° Snowboarding", page);
        assert_eq!(game.kind, "Virtual Console");
        assert_eq!(game.released, ["NA", "PAL"]);
        assert_eq!(game.rating, "Runs");
    }

    #[test]
    fn a_page_without_a_rating_has_none() {
        let page = "{{Infobox VG\n|title = X\n}}\n{{testline|rating=Perfect}}";
        assert_eq!(read_page(9, "X", page).rating, "");
    }

    #[test]
    fn region_codes_become_the_catalogues_labels() {
        let codes = |list: &[&str]| list.iter().map(|c| c.to_string()).collect::<Vec<_>>();
        assert_eq!(regions_of(&codes(&["NA", "PAL", "AUS", "JP"])), ["US", "EU", "JP"]);
        assert_eq!(regions_of(&codes(&["WW"])), ["EU", "US", "JP"]);
        assert!(regions_of(&codes(&["XX"])).is_empty());
    }

    #[test]
    fn every_rating_the_wiki_uses_has_words_of_our_own() {
        for rating in ["Perfect", "Playable", "Runs", "Loads", "Unplayable"] {
            let (label, tone, explanation) = describe(rating).unwrap();
            assert_eq!(label, rating);
            assert!(matches!(tone, "go" | "warn" | "bad"), "{rating} has tone {tone}");
            assert!(explanation.ends_with('.'), "{rating} should read as a sentence");
        }
        assert!(describe("Unknown").is_none());
    }

    #[test]
    fn every_rating_below_playable_has_a_caution_and_the_others_none() {
        for rating in ["Perfect", "Playable", "Runs", "Loads", "Unplayable"] {
            let (_, tone, _) = describe(rating).unwrap();
            let caution = caution(rating);
            if tone == "go" {
                assert_eq!(caution, "", "{rating}");
            } else {
                assert!(caution.starts_with("it ") && caution.ends_with('.'), "{rating}: {caution:?}");
            }
        }
    }
}
