//! Per-game emulator settings, as the settings sheet shows them for every
//! emulator. Which settings there are, and where they live, is each
//! emulator's own business; this is only the shape they are shown in.

use serde::Serialize;
use std::collections::BTreeMap;

/// A setting as the interface should present it: the emulator's own key and
/// default, plus our own words where we have them.
#[derive(Serialize)]
pub struct Setting {
    pub key: String,
    pub group: String,
    pub name: String,
    pub label: String,
    pub hint: String,
    pub kind: String,
    pub choices: &'static [&'static str],
    pub default: String,
    pub min: i64,
    pub max: i64,
    pub common: bool,
}

/// What the user has chosen for a game, keyed by the setting's full path.
/// Anything absent is the emulator's own default and is deliberately not
/// represented.
pub type Chosen = BTreeMap<String, String>;

/// Everything the settings sheet needs for one game.
#[derive(Serialize)]
pub struct GameSettings {
    /// The emulator's name, for "Cemu default" and the like.
    pub emulator: String,
    pub options: Vec<Setting>,
    pub chosen: Chosen,
    /// Why Omoio set any of them itself, by key.
    pub reasons: BTreeMap<String, String>,
    /// The groups the Common tab shows, in order.
    pub common_groups: Vec<String>,
}
