//! Which console a game is for, and what its emulator can do for it.
//!
//! These live in `core` rather than beside the emulators because the library
//! and the interface need them, and `core` must not depend on `backends`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Console {
    /// Every library written before consoles were recorded holds PS3 games
    /// only, so that is what a missing value means.
    #[default]
    Ps3,
    WiiU,
}

impl Console {
    /// The short name people use, for messages like "It takes PS3 games".
    pub fn short(self) -> &'static str {
        match self {
            Self::Ps3 => "PS3",
            Self::WiiU => "Wii U",
        }
    }
}

/// What a game's emulator can do beyond starting it. The interface shows a
/// section only when the emulator offers it, rather than a button that does
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Features {
    /// Official updates from the console's maker.
    pub updates: bool,
    /// Community patches.
    pub patches: bool,
    /// Per-game emulator settings.
    pub settings: bool,
    /// Backing saves up and putting them back.
    pub saves: bool,
    /// A compatibility result for the game. RPCS3's list covers the PS3 only.
    pub compatibility: bool,
    /// The toy portal of a Skylanders game, filled from Omoio's portal menu.
    pub portal: bool,
    /// The game stops reading the pad while Omoio's window is in front, so
    /// presses in Big Picture never reach a game waiting behind it.
    pub quiet_behind: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consoles_are_written_the_way_the_interface_reads_them() {
        assert_eq!(serde_json::to_string(&Console::Ps3).unwrap(), "\"ps3\"");
        assert_eq!(serde_json::to_string(&Console::WiiU).unwrap(), "\"wiiu\"");
        assert_eq!(serde_json::from_str::<Console>("\"wiiu\"").unwrap(), Console::WiiU);
    }

    #[test]
    fn nothing_recorded_means_ps3() {
        assert_eq!(Console::default(), Console::Ps3);
    }
}
