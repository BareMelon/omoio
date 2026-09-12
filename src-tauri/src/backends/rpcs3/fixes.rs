//! The short list of fixes Omoio switches on by itself.
//!
//! Only a fix tied to named games, with its reason written down, may be on
//! without being asked. Anything that changes a game rather than fixing it waits for
//! the user. Every entry here was measured on a real session, and its reason
//! says what was seen. A fix is applied once per game, so one the user
//! switches off afterwards stays off.

use tauri::AppHandle;

/// What a fix changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Switches on a patch from RPCS3's own patch list.
    Patch {
        hash: &'static str,
        name: &'static str,
    },
    /// Sets one of the game's own RPCS3 settings, keyed the way
    /// `game_config` keys them.
    Setting {
        key: &'static str,
        value: &'static str,
    },
}

#[derive(Debug)]
pub struct Fix {
    /// Recorded per game once applied, so it is never applied twice.
    pub id: &'static str,
    pub serials: &'static [&'static str],
    pub change: Change,
    /// Shown beside the patch or setting, so nobody has to wonder why it is on.
    pub reason: &'static str,
}

/// Every release of LittleBigPlanet 3 that RPCS3's patch list names, from its
/// `lbp3_126title` anchor.
const LBP3: &[&str] = &[
    "BCES01663", "BCES02068", "BCUS81138", "BCUS98362", "BCAS20322", "NPEA00515", "NPUA81116",
    "NPHA80277",
];

/// LittleBigPlanet 2 as RPCS3's patch list names it for update 01.33, then
/// LittleBigPlanet 3 as above. The MLAA patch's own notes name both games.
const LBP2_AND_3: &[&str] = &[
    "BCES00850", "BCES01086", "BCES01345", "BCES01346", "BCES01693", "BCES01694", "BCUS98245",
    "BCUS98372", "BCAS20113", "BCAS20201", "BCJS30058", "NPEA00324", "NPEA00437", "NPUA80662",
    "BCES01663", "BCES02068", "BCUS81138", "BCUS98362", "BCAS20322", "NPEA00515", "NPUA81116",
    "NPHA80277",
];

pub const FIXES: &[Fix] = &[
    Fix {
        id: "lbp-disable-spu-mlaa",
        serials: LBP2_AND_3,
        change: Change::Patch {
            hash: "SPU-702d0205a89d445d15dc0f96548546c4e2e7a59f",
            name: "Disable SPU MLAA - LittleBigPlanet 2, LittleBigPlanet 3, LittleBigPlanet Hub",
        },
        reason: "Stops LittleBigPlanet 3 freezing at a cutscene, where the picture waited on the game's edge smoothing.",
    },
    Fix {
        id: "lbp3-resolution-100",
        serials: LBP3,
        change: Change::Setting {
            key: "Video\nResolution Scale",
            value: "100",
        },
        reason: "Above 100% the emulator refuses the game's own image copies, which shows as dotted characters.",
    },
];

/// The fixes written for this game.
pub fn for_game(title_id: &str) -> Vec<&'static Fix> {
    FIXES
        .iter()
        .filter(|fix| fix.serials.iter().any(|serial| *serial == title_id))
        .collect()
}

/// Why Omoio switches this patch on for this game, when it does.
pub fn patch_reason(title_id: &str, hash: &str, name: &str) -> Option<&'static str> {
    for_game(title_id).into_iter().find_map(|fix| match fix.change {
        Change::Patch { hash: h, name: n } if h == hash && n == name => Some(fix.reason),
        _ => None,
    })
}

/// The settings Omoio sets for this game, each with its reason, keyed the way
/// `game_config` keys them.
pub fn setting_reasons(title_id: &str) -> Vec<(&'static str, &'static str)> {
    for_game(title_id)
        .into_iter()
        .filter_map(|fix| match fix.change {
            Change::Setting { key, .. } => Some((key, fix.reason)),
            Change::Patch { .. } => None,
        })
        .collect()
}

/// The fixes for this game that have not been applied yet.
pub fn due(title_id: &str, applied: &[String]) -> Vec<&'static Fix> {
    for_game(title_id)
        .into_iter()
        .filter(|fix| !applied.iter().any(|id| id == fix.id))
        .collect()
}

/// Applies the fixes due for this game and returns the ids it applied.
///
/// A patch is only switched on when the patch list is downloaded and the
/// patch covers the version that runs. Otherwise it is left for the next
/// launch rather than recorded as done.
pub fn apply(app: &AppHandle, title_id: &str, app_version: &str, applied: &[String]) -> Vec<&'static str> {
    let mut done = Vec::new();
    for fix in due(title_id, applied) {
        let ok = match fix.change {
            Change::Patch { hash, name } => super::patches::for_title(app, title_id, app_version)
                .into_iter()
                .find(|patch| patch.hash == hash && patch.name == name && patch.applies)
                .is_some_and(|patch| {
                    super::patches::set_enabled(app, &patch, title_id, app_version, true).is_ok()
                }),
            Change::Setting { key, value } => {
                let mut chosen = super::game_config::read(app, title_id);
                chosen.insert(key.to_string(), value.to_string());
                super::game_config::write(app, title_id, &chosen).is_ok()
            }
        };
        if ok {
            done.push(fix.id);
        }
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(fixes: &[&Fix]) -> Vec<&'static str> {
        fixes.iter().map(|fix| fix.id).collect()
    }

    #[test]
    fn lbp2_and_lbp3_get_the_mlaa_patch_and_lbp1_gets_nothing() {
        assert!(ids(&for_game("BCES01663")).contains(&"lbp-disable-spu-mlaa"));
        assert!(ids(&for_game("BCES00850")).contains(&"lbp-disable-spu-mlaa"));
        assert!(for_game("BCES00141").is_empty(), "LittleBigPlanet 1 needed no fix");
    }

    #[test]
    fn only_lbp3_gets_its_resolution_back_to_100() {
        assert!(ids(&for_game("NPEA00515")).contains(&"lbp3-resolution-100"));
        assert!(!ids(&for_game("BCES00850")).contains(&"lbp3-resolution-100"));
        assert_eq!(setting_reasons("BCES01663").len(), 1);
        assert_eq!(setting_reasons("BCES01663")[0].0, "Video\nResolution Scale");
    }

    #[test]
    fn a_fix_applied_once_is_not_due_again() {
        let applied = vec!["lbp-disable-spu-mlaa".to_string()];
        assert_eq!(ids(&due("BCES01663", &applied)), ["lbp3-resolution-100"]);
        let both = vec!["lbp-disable-spu-mlaa".to_string(), "lbp3-resolution-100".to_string()];
        assert!(due("BCES01663", &both).is_empty());
    }

    #[test]
    fn a_patch_is_matched_by_both_its_hash_and_its_name() {
        let hash = "SPU-702d0205a89d445d15dc0f96548546c4e2e7a59f";
        let name = "Disable SPU MLAA - LittleBigPlanet 2, LittleBigPlanet 3, LittleBigPlanet Hub";
        assert!(patch_reason("BCES01663", hash, name).is_some());
        assert!(patch_reason("BCES01663", hash, "Something else").is_none());
        assert!(patch_reason("BCES00141", hash, name).is_none(), "not for LittleBigPlanet 1");
    }

    #[test]
    fn every_fix_has_its_own_id_and_a_plain_reason() {
        let mut seen = std::collections::HashSet::new();
        for fix in FIXES {
            assert!(seen.insert(fix.id), "{} is listed twice", fix.id);
            assert!(!fix.serials.is_empty(), "{} names no game", fix.id);
            assert!(fix.reason.ends_with('.'), "{} should read as a sentence", fix.id);
            assert!(!fix.reason.contains('\u{2014}'), "{} uses an em dash", fix.id);
        }
    }
}
