//! Trap Team's 46 villains, by the number the game's own files give each,
//! 1001 to 1046; a trap keeps the same number less 1000. The names are the
//! ones the game writes when a villain is beaten. The element is that of the
//! traps that hold the villain, which is also the colour of its picture's
//! frame. Kaos has none: only the Kaos trap holds him.

use crate::core::figures::Element;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Villain {
    pub id: u16,
    pub name: &'static str,
    /// `None` for Kaos.
    pub element: Option<Element>,
}

const fn villain(id: u16, name: &'static str, element: Option<Element>) -> Villain {
    Villain { id, name, element }
}

use Element::*;

pub const VILLAINS: [Villain; 46] = [
    villain(1001, "Chompy Mage", Some(Life)),
    villain(1002, "Dr. Krankcase", Some(Tech)),
    villain(1003, "Wolfgang", Some(Undead)),
    villain(1004, "Chef Pepper Jack", Some(Fire)),
    villain(1005, "Night Shade", Some(Dark)),
    villain(1006, "Luminous", Some(Light)),
    villain(1007, "Golden Queen", Some(Earth)),
    villain(1008, "Dreamcatcher", Some(Air)),
    villain(1009, "The Gulper", Some(Water)),
    villain(1010, "Kaos", None),
    villain(1011, "Cuckoo Clocker", Some(Life)),
    villain(1012, "Buzzer Beak", Some(Air)),
    villain(1013, "Shield Shredder", Some(Life)),
    villain(1014, "Cross Crow", Some(Water)),
    villain(1015, "Bone Chompy", Some(Undead)),
    villain(1016, "Brawl and Chain", Some(Water)),
    villain(1017, "Bomb Shell", Some(Magic)),
    villain(1018, "Masker Mind", Some(Undead)),
    villain(1019, "Chill Bill", Some(Water)),
    villain(1020, "Sheep Creep", Some(Life)),
    villain(1021, "Shrednaught", Some(Tech)),
    villain(1022, "Chomp Chest", Some(Earth)),
    villain(1023, "Broccoli Guy", Some(Life)),
    villain(1024, "Rage Mage", Some(Magic)),
    villain(1025, "Lob Goblin", Some(Light)),
    villain(1026, "Chompy", Some(Life)),
    villain(1027, "Fisticuffs", Some(Dark)),
    villain(1028, "Trolling Thunder", Some(Tech)),
    villain(1029, "Hood Sickle", Some(Undead)),
    villain(1030, "Bruiser Cruiser", Some(Tech)),
    villain(1031, "Brawlrus", Some(Tech)),
    villain(1032, "Tussle Sprout", Some(Earth)),
    villain(1033, "Krankenstein", Some(Air)),
    villain(1034, "Scrap Shooter", Some(Fire)),
    villain(1035, "Slobber Trap", Some(Water)),
    villain(1036, "Grinnade", Some(Fire)),
    villain(1037, "Bad Juju", Some(Air)),
    villain(1038, "Blaster-Tron", Some(Light)),
    villain(1039, "Tae Kwon Crow", Some(Dark)),
    villain(1040, "Pain-yatta", Some(Magic)),
    villain(1041, "Smoke Scream", Some(Fire)),
    villain(1042, "Eye Five", Some(Light)),
    villain(1043, "Grave Clobber", Some(Earth)),
    villain(1044, "Threatpack", Some(Water)),
    villain(1045, "Mab Lobs", Some(Tech)),
    villain(1046, "Eye Scream", Some(Dark)),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::figures;

    #[test]
    fn every_villain_is_numbered_in_order() {
        for (at, each) in VILLAINS.iter().enumerate() {
            assert_eq!(each.id, 1001 + at as u16, "{}", each.name);
        }
    }

    #[test]
    fn a_villain_fits_the_traps_of_its_element() {
        // The trap ids, one to an element, give the same elements.
        let trap = |id: u16| figures::element(id);
        let of = |id: u16| VILLAINS[usize::from(id - 1001)].element;
        assert_eq!(of(1001), trap(217)); // Chompy Mage, Life
        assert_eq!(of(1017), trap(210)); // Bomb Shell, Magic
        assert_eq!(of(1005), trap(218)); // Night Shade, Dark
        assert_eq!(of(1006), trap(219)); // Luminous, Light
        assert_eq!(of(1010), None); // Kaos, only in his own trap
        let count = |element| VILLAINS.iter().filter(|v| v.element == element).count();
        assert_eq!([count(Some(Air)), count(Some(Water)), count(Some(Magic)), count(None)], [4, 6, 3, 1]);
    }
}
