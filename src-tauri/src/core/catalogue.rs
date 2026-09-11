//! The catalogue: every title the emulators' compatibility lists know about,
//! put together so one game is one entry however many times it was released.
//!
//! Nothing here reads a file or the network. Each backend hands over what its
//! list says, and this decides what is shown and in what order.

use crate::core::console::{Console, Features};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The regions a release can be from, in the order releases are listed.
const REGIONS: [&str; 5] = ["EU", "US", "JP", "Asia", "KR"];

/// How many games a page shows when nobody has asked for more.
const PAGE: usize = 60;

/// How well a title runs, in the words of the list it came from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Status {
    pub label: &'static str,
    /// "go", "warn" or "bad". Empty when nobody has reported on the title.
    pub tone: &'static str,
    pub explanation: &'static str,
}

/// One title as a list reports it, before the releases of a game are put
/// together.
#[derive(Debug, Clone)]
pub struct Entry {
    pub console: Console,
    /// Unique within its console's list. Covers are cached under it.
    pub key: String,
    pub name: String,
    /// False when the list had no name and `name` is the title id.
    pub named: bool,
    /// Empty when the list has no title id for the title.
    pub title_id: String,
    pub regions: Vec<&'static str>,
    pub status: Status,
    /// "Virtual Console" for an older console's game sold again, else empty.
    pub kind: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Release {
    pub title_id: String,
    pub region: &'static str,
}

/// One game in the catalogue, with every release of it the list knows.
#[derive(Debug, Clone, Serialize)]
pub struct Listing {
    pub console: Console,
    pub console_name: &'static str,
    pub key: String,
    pub name: String,
    pub named: bool,
    /// The best any release of it is reported to do.
    pub status: Status,
    pub kind: &'static str,
    pub regions: Vec<&'static str>,
    /// Every release with a title id, the chosen region's first.
    pub releases: Vec<Release>,
    pub demo: bool,
    /// Filled in by the caller, which knows the library.
    pub owned: bool,
    /// Filled in by the caller, which knows the backends.
    pub features: Features,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Filter {
    pub query: String,
    pub console: Option<Console>,
    /// One of the labels in `REGIONS`, or empty for every region.
    pub region: String,
    /// A tone, "go", "warn" or "bad", or empty for any result.
    pub runs: String,
    pub hide_demos: bool,
    /// "runs" puts what runs best first. Anything else sorts by name.
    pub sort: String,
    /// How many to return. Zero means one page.
    pub limit: usize,
}

/// A name with case and punctuation folded away, so "LittleBigPlanet™ 2" and
/// "LITTLEBIGPLANET 2" are the same game.
pub fn fold(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_lowercase().next().unwrap_or(c)
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Demos, trials and betas are their own titles with their own results, and
/// someone who owns one should find it, so they are marked rather than dropped.
fn is_demo(name: &str) -> bool {
    name.contains("体験版")
        || fold(name)
            .split(' ')
            .any(|word| matches!(word, "demo" | "trial" | "beta"))
}

fn tone_rank(tone: &str) -> u8 {
    match tone {
        "go" => 0,
        "warn" => 1,
        "bad" => 2,
        _ => 3,
    }
}

fn region_rank(region: &str) -> usize {
    REGIONS
        .iter()
        .position(|known| *known == region)
        .unwrap_or(REGIONS.len())
}

/// Puts the releases of each game together. The same game is sold once per
/// region and again on the store, so RPCS3's list holds four "Angry Birds
/// Trilogy" titles that are one game to anyone browsing.
pub fn group(entries: Vec<Entry>) -> Vec<Listing> {
    let mut index: HashMap<(Console, String), usize> = HashMap::new();
    let mut groups: Vec<Vec<Entry>> = Vec::new();
    for entry in entries {
        let folded = fold(&entry.name);
        // A title with no name is only ever itself.
        let same = if entry.named && !folded.is_empty() {
            folded
        } else {
            entry.key.clone()
        };
        match index.get(&(entry.console, same.clone())) {
            Some(&at) => groups[at].push(entry),
            None => {
                index.insert((entry.console, same), groups.len());
                groups.push(vec![entry]);
            }
        }
    }
    groups.into_iter().map(listing_of).collect()
}

fn listing_of(mut entries: Vec<Entry>) -> Listing {
    let rank = |entry: &Entry| {
        entry
            .regions
            .iter()
            .map(|region| region_rank(region))
            .min()
            .unwrap_or(REGIONS.len())
    };
    entries.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.title_id.cmp(&b.title_id)));

    let mut regions: Vec<&'static str> = entries
        .iter()
        .flat_map(|entry| entry.regions.iter().copied())
        .collect();
    regions.sort_by_key(|region| region_rank(region));
    regions.dedup();

    let releases = entries
        .iter()
        .filter(|entry| !entry.title_id.is_empty())
        .map(|entry| Release {
            title_id: entry.title_id.clone(),
            region: entry.regions.first().copied().unwrap_or(""),
        })
        .collect();

    // min_by_key keeps the first of equals, so a tie goes to the release
    // listed first.
    let status = entries
        .iter()
        .map(|entry| entry.status)
        .min_by_key(|status| tone_rank(status.tone))
        .unwrap_or_default();

    let first = &entries[0];
    Listing {
        console: first.console,
        console_name: first.console.short(),
        key: first.key.clone(),
        name: first.name.clone(),
        named: first.named,
        status,
        kind: first.kind,
        regions,
        releases,
        demo: is_demo(&first.name),
        owned: false,
        features: Features::default(),
    }
}

/// What the filter lets through, in the order asked for, and how many that
/// was before the page was cut.
pub fn pick(listings: Vec<Listing>, filter: &Filter) -> (usize, Vec<Listing>) {
    let needle = filter.query.trim().to_lowercase();
    let mut found: Vec<Listing> = listings
        .into_iter()
        .filter(|l| filter.console.map_or(true, |console| l.console == console))
        .filter(|l| filter.region.is_empty() || l.regions.iter().any(|r| *r == filter.region))
        .filter(|l| filter.runs.is_empty() || l.status.tone == filter.runs)
        .filter(|l| !(filter.hide_demos && l.demo))
        .filter(|l| {
            needle.is_empty()
                || l.name.to_lowercase().contains(&needle)
                || l.releases
                    .iter()
                    .any(|r| r.title_id.to_lowercase().contains(&needle))
        })
        .collect();

    // A name beginning with what was typed is what someone meant, and a title
    // with no name at all is the weakest match. Within that, the order asked
    // for.
    let best_first = filter.sort == "runs";
    found.sort_by_cached_key(|l| {
        let name = l.name.to_lowercase();
        let rank = if !l.named {
            3
        } else if name.starts_with(&needle) {
            0
        } else if name.split_whitespace().any(|word| word.starts_with(&needle)) {
            1
        } else {
            2
        };
        let runs = if best_first { tone_rank(l.status.tone) } else { 0 };
        (rank, runs, name)
    });

    let total = found.len();
    found.truncate(if filter.limit == 0 { PAGE } else { filter.limit });

    // The release a card adds is its first, so with a region chosen that is
    // the one from there. Stable, so the rest keep their order.
    if !filter.region.is_empty() {
        for listing in &mut found {
            listing.releases.sort_by_key(|r| r.region != filter.region);
        }
    }
    (total, found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titled(console: Console, id: &str, name: &str, regions: Vec<&'static str>, tone: &'static str) -> Entry {
        Entry {
            console,
            key: id.to_string(),
            name: name.to_string(),
            named: true,
            title_id: if console == Console::Ps3 { id.to_string() } else { String::new() },
            regions,
            status: Status {
                label: "Result",
                tone,
                explanation: "",
            },
            kind: "",
        }
    }

    fn ps3(id: &str, name: &str, region: &'static str, tone: &'static str) -> Entry {
        titled(Console::Ps3, id, name, vec![region], tone)
    }

    fn names(listings: &[Listing]) -> Vec<&str> {
        listings.iter().map(|l| l.name.as_str()).collect()
    }

    #[test]
    fn the_releases_of_one_game_become_one_listing() {
        let listings = group(vec![
            ps3("NPUB31054", "Angry Birds Trilogy", "US", "go"),
            ps3("BLES01732", "Angry Birds Trilogy", "EU", "go"),
            ps3("NPEB01235", "ANGRY BIRDS TRILOGY", "EU", "go"),
            ps3("BLUS31054", "Angry Birds Trilogy", "US", "go"),
        ]);
        assert_eq!(listings.len(), 1);
        let game = &listings[0];
        assert_eq!(game.regions, vec!["EU", "US"]);
        let ids: Vec<&str> = game.releases.iter().map(|r| r.title_id.as_str()).collect();
        assert_eq!(ids, ["BLES01732", "NPEB01235", "BLUS31054", "NPUB31054"]);
        assert_eq!(game.key, "BLES01732", "covers stay under the same release");
        assert_eq!(game.name, "Angry Birds Trilogy");
    }

    #[test]
    fn different_games_and_consoles_stay_apart() {
        let listings = group(vec![
            ps3("BLES01732", "Angry Birds Trilogy", "EU", "go"),
            ps3("BLES01943", "Angry Birds Star Wars", "EU", "go"),
            titled(Console::WiiU, "WU1", "Angry Birds Trilogy", vec!["EU", "US"], "go"),
        ]);
        assert_eq!(listings.len(), 3);
        let wii_u = listings.iter().find(|l| l.console == Console::WiiU).unwrap();
        assert!(wii_u.releases.is_empty(), "the wiki has no title ids");
        assert_eq!(wii_u.console_name, "Wii U");
    }

    #[test]
    fn the_best_result_among_releases_is_the_one_shown() {
        let listings = group(vec![
            ps3("BLES00001", "Some Game", "EU", "warn"),
            ps3("BLUS00001", "Some Game", "US", "go"),
        ]);
        assert_eq!(listings[0].status.tone, "go");
    }

    fn shelf() -> Vec<Listing> {
        group(vec![
            ps3("BLES01732", "Angry Birds Trilogy", "EU", "go"),
            ps3("NPUB30001", "Stacking Demo", "US", "go"),
            ps3("BLJM00001", "Rain", "JP", "bad"),
            ps3("BCES00797", "Heavy Rain", "EU", "warn"),
            titled(Console::WiiU, "WU2", "Mario Kart 8", vec!["EU", "US", "JP"], "go"),
        ])
    }

    #[test]
    fn each_filter_narrows_the_list() {
        let by = |filter: Filter| names(&pick(shelf(), &filter).1).join(", ");
        assert_eq!(by(Filter { region: "JP".into(), ..Default::default() }), "Mario Kart 8, Rain");
        assert_eq!(by(Filter { runs: "bad".into(), ..Default::default() }), "Rain");
        assert_eq!(by(Filter { console: Some(Console::WiiU), ..Default::default() }), "Mario Kart 8");
        assert!(!by(Filter { hide_demos: true, ..Default::default() }).contains("Demo"));
        assert!(by(Filter::default()).contains("Stacking Demo"));
    }

    #[test]
    fn a_name_starting_with_the_search_comes_first() {
        let (_, found) = pick(shelf(), &Filter { query: "rain".into(), ..Default::default() });
        assert_eq!(names(&found), ["Rain", "Heavy Rain"]);
    }

    #[test]
    fn runs_best_puts_what_plays_well_first() {
        let (_, found) = pick(shelf(), &Filter { sort: "runs".into(), ..Default::default() });
        assert_eq!(
            names(&found),
            ["Angry Birds Trilogy", "Mario Kart 8", "Stacking Demo", "Heavy Rain", "Rain"]
        );
    }

    #[test]
    fn a_page_is_cut_but_the_total_is_kept() {
        let (total, found) = pick(shelf(), &Filter { limit: 2, ..Default::default() });
        assert_eq!((total, found.len()), (5, 2));
        let (_, found) = pick(shelf(), &Filter::default());
        assert_eq!(found.len(), 5, "no limit means a whole page");
    }

    #[test]
    fn the_chosen_regions_release_comes_first() {
        let listings = group(vec![
            ps3("BLES01732", "Angry Birds Trilogy", "EU", "go"),
            ps3("BLUS31054", "Angry Birds Trilogy", "US", "go"),
        ]);
        let (_, found) = pick(listings, &Filter { region: "US".into(), ..Default::default() });
        assert_eq!(found[0].releases[0].title_id, "BLUS31054");
        assert_eq!(found[0].key, "BLES01732", "the cover does not change with the filter");
    }

    #[test]
    fn demos_are_told_by_whole_words() {
        for name in ["Stacking Demo", "1942: Joint Strike Trial", "LittleBigPlanet™ Beta", "ぼくのなつやすみ 体験版"] {
            assert!(is_demo(name), "{name}");
        }
        for name in ["Demon's Souls™", "Trials HD", "Alphabet"] {
            assert!(!is_demo(name), "{name}");
        }
    }

    #[test]
    fn folding_ignores_case_and_marks() {
        assert_eq!(fold("LittleBigPlanet™ 2"), "littlebigplanet 2");
        assert_eq!(fold("LITTLEBIGPLANET 2"), "littlebigplanet 2");
        assert_eq!(fold("  Mario   Kart 8 "), "mario kart 8");
    }
}
