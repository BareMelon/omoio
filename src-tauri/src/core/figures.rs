//! Toy figures, as the emulators' own figure makers describe them.

use serde::{Deserialize, Serialize};

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
}
