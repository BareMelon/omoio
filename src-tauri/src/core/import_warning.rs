//! Whether a game about to be imported is worth a warning, and whether
//! another console Omoio runs has a version of it that plays well.
//!
//! Only what the compatibility lists already say is used, from the copies
//! Omoio keeps, so nothing here waits on the network. A game nobody has rated
//! gets no warning: there is nothing true to say about it.

use crate::core::catalogue::{fold, same_game, tone_rank, Entry, Status};
use crate::core::console::Console;
use crate::core::figures::has_portal_menu;
use crate::core::library::Game;
use serde::Serialize;

/// A game as far as the check needs to know it. An archive's game is known
/// this far before it is unpacked.
#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    pub console: Console,
    /// Empty when the game has no id its console's list could know it by.
    pub title_id: String,
    pub title: String,
}

impl Imported {
    pub fn of(game: &Game) -> Self {
        Self {
            console: game.console,
            title_id: game.title_id.clone(),
            title: game.title.clone(),
        }
    }
}

/// One console's emulator and every title its list rates, with each name
/// folded once rather than once per game checked, since a scan checks
/// hundreds against a list of thousands.
pub struct List {
    console: Console,
    emulator: &'static str,
    titles: Vec<(Entry, String, String)>,
}

impl List {
    pub fn new(console: Console, emulator: &'static str, entries: Vec<Entry>) -> Self {
        let titles = entries
            .into_iter()
            .filter(|entry| entry.named && !entry.status.tone.is_empty())
            .map(|entry| {
                let (folded, key) = (fold(&entry.name), same_game(&entry.name));
                (entry, folded, key)
            })
            .collect();
        Self { console, emulator, titles }
    }

    /// The best rated of the titles that pass `same`.
    fn best(&self, same: impl Fn(&str, &str) -> bool) -> Option<&Entry> {
        self.titles
            .iter()
            .filter(|(_, folded, key)| same(folded, key))
            .map(|(entry, _, _)| entry)
            .min_by_key(|entry| tone_rank(entry.status.tone))
    }

    /// How this list rates the game: its own release when the list has it,
    /// then the game by its name, then by the looser name used across
    /// consoles. Releases of one game are rated apart, and the catalogue
    /// shows the best of them, so this does too.
    fn result(&self, game: &Imported) -> Option<Status> {
        let own = (!game.title_id.is_empty())
            .then(|| self.titles.iter().find(|(entry, _, _)| entry.title_id == game.title_id))
            .flatten()
            .map(|(entry, _, _)| entry.status);
        let folded = fold(&game.title);
        let key = same_game(&game.title);
        own.or_else(|| self.best(|name, _| !folded.is_empty() && name == folded).map(|e| e.status))
            .or_else(|| self.best(|_, other| !key.is_empty() && other == key).map(|e| e.status))
    }
}

/// What to tell someone about to import a game that may not run well.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Warning {
    pub title: String,
    pub console: Console,
    pub console_name: &'static str,
    /// "RPCS3 rates it Ingame: it starts, but you may hit problems before the end."
    pub rating: String,
    /// "The Wii U version is rated Playable in Cemu." Empty when no other
    /// console has a version rated to play well.
    pub better: String,
}

/// The warning for a game whose own emulator rates it below playing well,
/// or `None` when it plays well or nobody has rated it.
pub fn warning(game: &Imported, lists: &[List]) -> Option<Warning> {
    let own = lists.iter().find(|list| list.console == game.console)?;
    let status = own.result(game)?;
    if !matches!(status.tone, "warn" | "bad") {
        return None;
    }

    let rating = if status.caution.is_empty() {
        format!("{} rates it {}.", own.emulator, status.label)
    } else {
        format!("{} rates it {}: {}", own.emulator, status.label, status.caution)
    };

    let key = same_game(&game.title);
    let better = lists
        .iter()
        .filter(|list| list.console != game.console && !key.is_empty())
        .find_map(|list| {
            list.best(|_, other| other == key)
                .filter(|entry| entry.status.tone == "go")
                .map(|entry| (list, entry))
        })
        .map(|(list, entry)| {
            let portal = if has_portal_menu(list.console, &entry.name) {
                ", and the portal menu works in it"
            } else {
                ""
            };
            format!(
                "The {} version is rated {} in {}{portal}.",
                list.console.short(),
                entry.status.label,
                list.emulator
            )
        })
        .unwrap_or_default();

    Some(Warning {
        title: game.title.clone(),
        console: game.console,
        console_name: game.console.short(),
        rating,
        better,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rated(label: &'static str) -> Status {
        let (tone, caution) = match label {
            "Playable" | "Perfect" => ("go", ""),
            "Ingame" => ("warn", "it starts, but you may hit problems before the end."),
            "Runs" => ("warn", "it gets into the game, but major glitches make it hard to finish."),
            "Nothing" => ("bad", "it doesn't start."),
            _ => ("", ""),
        };
        Status { label, tone, explanation: "", caution }
    }

    fn entry(console: Console, title_id: &str, name: &str, label: &'static str) -> Entry {
        Entry {
            console,
            key: if title_id.is_empty() { name.to_string() } else { title_id.to_string() },
            name: name.to_string(),
            named: true,
            title_id: title_id.to_string(),
            regions: Vec::new(),
            status: rated(label),
            kind: "",
        }
    }

    /// What RPCS3's list and the Cemu wiki said about the Skylanders games
    /// on 6 October 2026.
    fn lists() -> Vec<List> {
        let ps3 = |id, name, label| entry(Console::Ps3, id, name, label);
        let wii_u = |name, label| entry(Console::WiiU, "", name, label);
        vec![
            List::new(
                Console::Ps3,
                "RPCS3",
                vec![
                    ps3("BLES01272", "Skylanders Spyro's Adventure", "Ingame"),
                    ps3("BLES01689", "Skylanders Giants", "Playable"),
                    ps3("BLES01860", "Skylanders SWAP Force", "Ingame"),
                    ps3("BLES02055", "Skylanders Trap Team", "Ingame"),
                    ps3("BLUS31545", "Skylanders SuperChargers", "Ingame"),
                    ps3("BLES02240", "Skylanders Imaginators", "Ingame"),
                    ps3("BLES00001", "Unrated Game", ""),
                ],
            ),
            List::new(
                Console::WiiU,
                "Cemu",
                vec![
                    wii_u("Skylanders: Spyro's Adventure", "Perfect"),
                    wii_u("Skylanders: Giants", "Playable"),
                    wii_u("Skylanders: Swap Force", "Runs"),
                    wii_u("Skylanders: Trap Team", "Playable"),
                    wii_u("Skylanders: SuperChargers", "Perfect"),
                    wii_u("Skylanders: Imaginators", "Perfect"),
                ],
            ),
        ]
    }

    fn ps3_game(title_id: &str, title: &str) -> Imported {
        Imported { console: Console::Ps3, title_id: title_id.into(), title: title.into() }
    }

    #[test]
    fn a_game_with_a_better_version_elsewhere_is_told_about_it() {
        let trap_team = warning(&ps3_game("BLES02055", "Skylanders Trap Team™"), &lists()).unwrap();
        assert_eq!(trap_team.title, "Skylanders Trap Team™");
        assert_eq!(trap_team.console_name, "PS3");
        assert_eq!(trap_team.rating, "RPCS3 rates it Ingame: it starts, but you may hit problems before the end.");
        assert_eq!(
            trap_team.better,
            "The Wii U version is rated Playable in Cemu, and the portal menu works in it."
        );

        let superchargers = warning(&ps3_game("BLUS31545", "Skylanders SuperChargers"), &lists()).unwrap();
        assert_eq!(superchargers.better, "The Wii U version is rated Perfect in Cemu.");
    }

    #[test]
    fn no_better_version_is_offered_when_the_other_one_has_problems_too() {
        let warning = warning(&ps3_game("BLES01860", "Skylanders SWAP Force"), &lists()).unwrap();
        assert_eq!(warning.better, "", "Cemu rates the Wii U version Runs");
    }

    #[test]
    fn a_game_that_plays_well_or_has_no_rating_gets_no_warning() {
        assert_eq!(warning(&ps3_game("BLES01689", "Skylanders Giants"), &lists()), None);
        assert_eq!(warning(&ps3_game("BLES00001", "Unrated Game"), &lists()), None);
        assert_eq!(warning(&ps3_game("BLES99999", "A Game Nobody Listed"), &lists()), None);
        assert_eq!(warning(&ps3_game("BLES01860", "Skylanders SWAP Force"), &[]), None, "no lists yet");
    }

    #[test]
    fn a_wii_u_game_is_found_by_its_name() {
        let game = Imported {
            console: Console::WiiU,
            title_id: "WUD87E51FD0F7F95".into(),
            title: "Skylanders - Swap Force".into(),
        };
        let warning = warning(&game, &lists()).unwrap();
        assert_eq!(warning.console_name, "Wii U");
        assert_eq!(
            warning.rating,
            "Cemu rates it Runs: it gets into the game, but major glitches make it hard to finish."
        );
        assert_eq!(warning.better, "", "RPCS3 rates the PS3 version Ingame");
    }

    #[test]
    fn a_release_the_list_lacks_is_rated_like_the_rest_of_its_game() {
        let warning = warning(&ps3_game("BLUS31442", "Skylanders Trap Team"), &lists()).unwrap();
        assert!(warning.rating.starts_with("RPCS3 rates it Ingame"));
    }

    #[test]
    fn the_portal_menu_is_only_mentioned_where_it_works() {
        // The Wii U's Giants plays, but the menu has only been played with
        // the PS3's.
        let lists = vec![
            List::new(Console::Ps3, "RPCS3", vec![entry(Console::Ps3, "BLES01689", "Skylanders Giants", "Nothing")]),
            List::new(Console::WiiU, "Cemu", vec![entry(Console::WiiU, "", "Skylanders: Giants", "Playable")]),
        ];
        let warning = warning(&ps3_game("BLES01689", "Skylanders Giants"), &lists).unwrap();
        assert_eq!(warning.rating, "RPCS3 rates it Nothing: it doesn't start.");
        assert_eq!(warning.better, "The Wii U version is rated Playable in Cemu.");
    }
}
