//! Each player's pad and layout, kept by Omoio in controllers.json and handed
//! to every emulator in its own form. What a layout is lives in
//! core/pad_layout.rs; this is keeping it and passing it on.

use crate::backends::EmulatorBackend;
use crate::core::pad_layout::{self, Layouts, Pad, Player, PLAYERS};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let data = app.path().data_dir().map_err(|e| e.to_string())?;
    Ok(data.join("Omoio").join("controllers.json"))
}

/// What is kept. Until something is, what the emulators already had is read
/// in, so a layout set up before Omoio kept its own is not lost.
fn load(app: &AppHandle) -> Layouts {
    if let Some(kept) = path(app).ok().and_then(|file| Layouts::load(&file)) {
        return kept;
    }
    let mut layouts = Layouts::default();
    for backend in crate::backends::all() {
        for (title_id, players) in backend.existing_layouts(app) {
            if layouts.get(&title_id).is_none() {
                layouts.set(&title_id, players);
            }
        }
    }
    layouts
}

fn store(app: &AppHandle, layouts: &Layouts) -> Result<(), String> {
    layouts
        .save(&path(app)?)
        .map_err(|_| "Couldn't save the controller settings.".to_string())
}

/// Hands a layout to every emulator. One failing does not stop the others;
/// the first failure is what is reported.
fn hand_over(app: &AppHandle, title_id: &str, players: &[Player]) -> Result<(), String> {
    let mut failed = None;
    for backend in crate::backends::all() {
        if let Err(err) = backend.write_layout(app, title_id, players) {
            failed.get_or_insert(err);
        }
    }
    failed.map_or(Ok(()), Err)
}

pub struct Current {
    pub players: Vec<Player>,
    /// Whether this game has a layout of its own.
    pub own: bool,
    /// Whether the players shown have been kept, rather than being who
    /// pressing Play would set up.
    pub saved: bool,
}

/// The four players a game plays with: its own layout, else the one for
/// every game, else who pressing Play would set up.
pub fn current(app: &AppHandle, title_id: &str, connected: &[Pad]) -> Current {
    let layouts = load(app);
    let own = !title_id.is_empty() && layouts.games.contains_key(title_id);
    let found = layouts.get(if own { title_id } else { "" }).cloned();
    let saved = found.is_some();
    let spare = crate::pads::xinput_slots();
    let mut players = match found {
        Some(players) => pad_layout::fill(players.into_iter().map(Some).collect(), &spare),
        None => pad_layout::default_players(connected, &spare),
    };
    pad_layout::refresh(&mut players, connected);
    Current { players, own, saved }
}

/// Gives player `number`, counted from 1, this pad and layout. A game's first
/// change starts from the layout for every game, which is what it was
/// playing with until then.
pub fn save_player(app: &AppHandle, title_id: &str, number: usize, player: Player) -> Result<(), String> {
    let at = number
        .checked_sub(1)
        .filter(|at| *at < PLAYERS)
        .ok_or("Couldn't save the controller settings.")?;
    let mut players = current(app, title_id, &crate::pads::connected()).players;
    pad_layout::give(&mut players, at, player);
    let mut layouts = load(app);
    layouts.set(title_id, players.clone());
    store(app, &layouts)?;
    hand_over(app, title_id, &players)
}

/// Gives all four players their pad's own layout, pads plugged in first.
pub fn restore_defaults(app: &AppHandle, title_id: &str) -> Result<(), String> {
    let players = pad_layout::default_players(&crate::pads::connected(), &crate::pads::xinput_slots());
    let mut layouts = load(app);
    layouts.set(title_id, players.clone());
    store(app, &layouts)?;
    hand_over(app, title_id, &players)
}

/// Takes a game's own layout away, so it goes back to the one for every game.
pub fn forget(app: &AppHandle, title_id: &str) -> Result<(), String> {
    if title_id.is_empty() {
        return Err("Couldn't remove that.".to_string());
    }
    let mut layouts = load(app);
    layouts.games.remove(title_id);
    store(app, &layouts)?;
    for backend in crate::backends::all() {
        backend.forget_layout(app, title_id)?;
    }
    Ok(())
}

/// Called before a game starts, so plugging in and pressing Play is enough.
/// A pad plugged in that no player has takes the place of one whose pad is
/// not there, buttons someone chose never change, and the layout goes to the
/// game's emulator.
pub fn before_launch(app: &AppHandle, backend: &dyn EmulatorBackend, title_id: &str) {
    let connected = crate::pads::connected();
    let Current { mut players, own, saved } = current(app, title_id, &connected);
    let moved = pad_layout::seat(&mut players, &connected);
    let scope = if own { title_id } else { "" };
    if moved || !saved {
        let mut layouts = load(app);
        layouts.set(scope, players.clone());
        let _ = store(app, &layouts);
    }
    let _ = backend.write_layout(app, scope, &players);
}
