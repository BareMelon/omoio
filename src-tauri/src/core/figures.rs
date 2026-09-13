//! Toy figures, as the emulators' own figure makers describe them, and the
//! facts Omoio keeps about each id to sort them: the element, what kind of
//! figure it is, and the game it came out with. The emulators list names and
//! numbers only. These facts were checked against Dolphin's figure list,
//! which has them for every figure; the table here is Omoio's own.

use serde::{Deserialize, Serialize};

/// The element a figure belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Element {
    Air,
    Earth,
    Fire,
    Water,
    Life,
    Undead,
    Magic,
    Tech,
    Light,
    Dark,
}

/// What a figure is, which decides where the menu lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Character,
    Item,
    Trap,
    Adventure,
    Vehicle,
    Trophy,
}

/// The games in the order they came out. A game reads the figures of its own
/// year and of every earlier one, never those of a later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Game {
    Spyro,
    Giants,
    SwapForce,
    TrapTeam,
    SuperChargers,
    Imaginators,
}

/// Which game a title is, from its name. `None` for one Omoio can't tell,
/// which then shows every figure.
pub fn game_from_title(title: &str) -> Option<Game> {
    let title = title.to_lowercase();
    [
        ("imaginators", Game::Imaginators),
        ("superchargers", Game::SuperChargers),
        ("trap team", Game::TrapTeam),
        ("swap force", Game::SwapForce),
        ("giants", Game::Giants),
        ("spyro", Game::Spyro),
    ]
    .into_iter()
    .find(|(name, _)| title.contains(name))
    .map(|(_, game)| game)
}

/// The game a figure id came out with. The sidekicks of the first two games
/// share ids with the minis of Trap Team, so they count from the first.
fn id_game(id: u16) -> Game {
    match id {
        505 | 514 | 519 | 526 => Game::Spyro,
        540..=543 => Game::Giants,
        0..=99 | 200..=207 | 300..=304 | 400..=449 => Game::Spyro,
        100..=199 | 208..=209 => Game::Giants,
        210..=299 | 305..=399 | 450..=999 => Game::TrapTeam,
        1000..=3219 | 3300..=3399 => Game::SwapForce,
        3220..=3299 | 3400..=3599 => Game::SuperChargers,
        _ => Game::Imaginators,
    }
}

/// The game a variant came out with, kept in the variant's top four bits:
/// 0x0000 for the first game, 0x1801 for a Giants re-release, 0x2805 for a
/// Swap Force one, and so on.
fn variant_game(variant: u16) -> Game {
    match variant >> 12 {
        0 => Game::Spyro,
        1 => Game::Giants,
        2 => Game::SwapForce,
        3 => Game::TrapTeam,
        4 => Game::SuperChargers,
        _ => Game::Imaginators,
    }
}

/// Whether `game` reads a figure with this id and variant.
pub fn reads(game: Game, id: u16, variant: u16) -> bool {
    id_game(id) <= game && variant_game(variant) <= game
}

pub fn kind(id: u16) -> Kind {
    match id {
        210..=229 => Kind::Trap,
        200..=299 | 3200..=3219 => Kind::Item,
        300..=399 | 3300..=3399 => Kind::Adventure,
        3220..=3299 => Kind::Vehicle,
        3500..=3599 => Kind::Trophy,
        _ => Kind::Character,
    }
}

pub fn element(id: u16) -> Option<Element> {
    use Element::*;
    // Swap Force numbers its bottom halves from 1000, its top halves from
    // 2000 and its whole figures from 3000, two to an element in one order.
    if (1..=3).contains(&(id / 1000)) && id % 1000 < 16 {
        return Some([Air, Earth, Fire, Life, Magic, Tech, Undead, Water][usize::from(id % 1000 / 2)]);
    }
    Some(match id {
        // Spyro's Adventure
        0..=3 => Air,
        4..=7 => Earth,
        8..=11 => Fire,
        12..=15 => Water,
        16..=18 | 23 | 28 => Magic,
        19..=22 => Tech,
        24..=27 => Life,
        29..=32 => Undead,
        // Giants, a giant and a core figure to each element
        100..=101 => Air,
        102..=103 => Earth,
        104..=105 => Fire,
        106..=107 => Water,
        108..=109 => Magic,
        110..=111 => Tech,
        112..=113 => Life,
        114..=115 => Undead,
        // Trap Team's traps, one id to an element
        210 => Magic,
        211 => Water,
        212 => Air,
        213 => Undead,
        214 => Tech,
        215 => Fire,
        216 => Earth,
        217 => Life,
        218 => Dark,
        219 => Light,
        // Legendary figures of the first game
        404 => Earth,
        416 => Magic,
        419 => Tech,
        430 => Undead,
        // Trap Team, four to an element and two each for Light and Dark
        450..=453 => Air,
        454..=457 => Earth,
        458..=461 => Fire,
        462..=465 => Water,
        466..=469 => Magic,
        470..=473 => Tech,
        474..=477 => Life,
        478..=481 => Undead,
        482..=483 => Light,
        484..=485 => Dark,
        // Minis and sidekicks
        502 | 505 => Earth,
        503 | 542 => Magic,
        504 | 543 => Undead,
        506 | 508 => Air,
        507 | 509 => Fire,
        510 | 519 => Tech,
        514 | 541 => Water,
        526 | 540 => Life,
        // SuperChargers vehicles
        3220 | 3232 | 3233 => Air,
        3221 | 3227 => Undead,
        3222 | 3231 => Water,
        3223 | 3224 => Fire,
        3225 | 3226 => Earth,
        3228 | 3241 => Life,
        3234 | 3235 | 3240 => Tech,
        3236 => Light,
        3237 => Dark,
        3238 | 3239 => Magic,
        // SuperChargers characters
        3400 | 3417 => Undead,
        3401 | 3414 => Tech,
        3402 | 3420 => Magic,
        3406 | 3413 => Air,
        3411 | 3416 => Earth,
        3412 | 3421 | 3424 => Fire,
        3415 | 3423 | 3428 => Life,
        3422 | 3425 => Water,
        3426 => Light,
        3427 => Dark,
        _ => return None,
    })
}

/// A character an emulator can make a figure of: the name it shows, and the
/// id and variant the figure carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub id: u16,
    pub variant: u16,
}

impl Character {
    /// From the number Cemu's figure maker keeps with each name in its list:
    /// the id in the high half and the variant in the low one. `None` for
    /// its "---Select---" line, which carries 0xFFFFFFFF.
    pub fn from_item(name: &str, data: u64) -> Option<Self> {
        if data >= 0xFFFF_FFFF || name.trim().is_empty() {
            return None;
        }
        Some(Self {
            name: name.trim().to_string(),
            id: (data >> 16) as u16,
            variant: (data & 0xFFFF) as u16,
        })
    }
}

/// A character as the menu lists it, with where it belongs.
#[derive(Debug, Clone, Serialize)]
pub struct Offer {
    #[serde(flatten)]
    pub character: Character,
    pub element: Option<Element>,
    pub kind: Kind,
}

/// The characters `game` reads, each with its element and kind. Every one
/// when the game isn't known.
pub fn offers(characters: Vec<Character>, game: Option<Game>) -> Vec<Offer> {
    characters
        .into_iter()
        .filter(|c| game.is_none_or(|game| reads(game, c.id, c.variant)))
        .map(|character| Offer {
            element: element(character.id),
            kind: kind(character.id),
            character,
        })
        .collect()
}

/// A file name for a new figure that is not taken yet: the character's
/// name, then " 2", " 3" and so on. Characters Windows does not allow in a
/// file name are left out.
pub fn free_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    let clean: String = name
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        .collect();
    let clean = clean.trim().trim_end_matches('.').to_string();
    let base = if clean.is_empty() { "Figure".to_string() } else { clean };
    let first = format!("{base}.sky");
    if !taken(&first) {
        return first;
    }
    (2..)
        .map(|n| format!("{base} {n}.sky"))
        .find(|file| !taken(file))
        .unwrap_or(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_item_gives_its_id_and_variant() {
        let whirlwind = Character::from_item("Whirlwind", 0).unwrap();
        assert_eq!((whirlwind.id, whirlwind.variant), (0, 0));
        let dark = Character::from_item("Dark Spyro", (404 << 16) | 0x1206).unwrap();
        assert_eq!((dark.id, dark.variant), (404, 0x1206));
        assert!(Character::from_item("---Select---", 0xFFFF_FFFF).is_none());
    }

    #[test]
    fn a_new_figure_never_takes_a_name_already_used() {
        let taken = ["Spyro.sky", "Spyro 2.sky"];
        assert_eq!(free_name("Spyro", |f| taken.contains(&f)), "Spyro 3.sky");
        assert_eq!(free_name("Whirlwind", |_| false), "Whirlwind.sky");
        assert_eq!(free_name("Who?: Me*", |_| false), "Who Me.sky");
        assert_eq!(free_name("???", |_| false), "Figure.sky");
    }

    #[test]
    fn figures_are_sorted_by_their_element_and_kind() {
        assert_eq!(element(0), Some(Element::Air)); // Whirlwind
        assert_eq!(element(23), Some(Element::Magic)); // Wrecking Ball
        assert_eq!(element(28), Some(Element::Magic)); // Dark Spyro
        assert_eq!(element(101), Some(Element::Air)); // Swarm
        assert_eq!(element(1007), Some(Element::Life)); // Grilla Drilla, bottom half
        assert_eq!(element(2015), Some(Element::Water)); // Wash Buckler, top half
        assert_eq!(element(3000), Some(Element::Air)); // Scratch
        assert_eq!(element(3012), Some(Element::Undead)); // Roller Brawl
        assert_eq!(element(482), Some(Element::Light)); // Knight Light
        assert_eq!(element(485), Some(Element::Dark)); // Blackout
        assert_eq!(element(201), None); // Hidden Treasure
        assert_eq!(kind(201), Kind::Item);
        assert_eq!(kind(203), Kind::Item); // Ghost Pirate Swords
        assert_eq!(kind(3200), Kind::Item); // Battle Hammer
        assert_eq!(kind(220), Kind::Trap); // Kaos
        assert_eq!(kind(300), Kind::Adventure); // Dragon's Peak
        assert_eq!(kind(3300), Kind::Adventure); // Sheep Wreck Island
        assert_eq!(kind(3220), Kind::Vehicle); // Jet Stream
        assert_eq!(kind(3500), Kind::Trophy); // Sky Trophy
        assert_eq!(kind(16), Kind::Character); // Spyro
    }

    #[test]
    fn a_game_reads_its_own_figures_and_older_ones_only() {
        let swap = Game::SwapForce;
        assert!(reads(swap, 0, 0x0000)); // Whirlwind
        assert!(reads(swap, 0, 0x2805)); // Horn Blast Whirlwind
        assert!(!reads(swap, 0, 0x3810)); // Eon's Elite Whirlwind, from Trap Team
        assert!(reads(swap, 3000, 0x2000)); // Scratch
        assert!(reads(swap, 505, 0x0000)); // Terrabite, a sidekick of the first game
        assert!(!reads(swap, 505, 0x3000)); // Terrabite, the Trap Team mini
        assert!(!reads(swap, 450, 0x3000)); // Gusto
        assert!(!reads(swap, 230, 0x3000)); // Hand of Fate
        assert!(!reads(Game::Giants, 3000, 0x2000));
        assert!(reads(Game::SuperChargers, 3400, 0x4100)); // Fiesta
    }

    #[test]
    fn the_game_is_told_by_its_title() {
        assert_eq!(game_from_title("Skylanders - Swap Force"), Some(Game::SwapForce));
        assert_eq!(game_from_title("Skylanders SWAP Force"), Some(Game::SwapForce));
        assert_eq!(game_from_title("Skylanders Spyro's Adventure"), Some(Game::Spyro));
        assert_eq!(game_from_title("Skylanders Giants"), Some(Game::Giants));
        assert_eq!(game_from_title("Skylanders: Trap Team"), Some(Game::TrapTeam));
        assert_eq!(game_from_title("Skylanders SuperChargers"), Some(Game::SuperChargers));
        assert_eq!(game_from_title("LittleBigPlanet 3"), None);
    }

    #[test]
    fn the_menu_gets_what_the_game_reads_with_its_element() {
        let list = vec![
            Character { name: "Whirlwind".into(), id: 0, variant: 0 },
            Character { name: "Gusto".into(), id: 450, variant: 0x3000 },
        ];
        let offered = offers(list.clone(), Some(Game::SwapForce));
        assert_eq!(offered.len(), 1);
        let json = serde_json::to_value(&offered[0]).unwrap();
        assert_eq!(json["name"], "Whirlwind");
        assert_eq!(json["element"], "air");
        assert_eq!(json["kind"], "character");
        assert_eq!(offers(list, None).len(), 2);
    }
}
