//! The line shadPS4 keeps open on its standard input for a launcher. Omoio
//! starts every game through it, puts figures on the emulated Skylanders
//! portal with it, and pauses the game while one of its menus is over it.
//!
//! Read out of `src/core/ipc/ipc.cpp` at tag v.0.18.0: shadPS4 listens when
//! started with SHADPS4_ENABLE_IPC set to "true", reads one word or value per
//! line, answers on its standard error, and closes itself unless it is told
//! RUN within five seconds. START then lets the game begin.

use std::io::Write;
use std::path::Path;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::Mutex;

/// How many figures the emulated portal holds (`MAX_SKYLANDERS`,
/// `skylander.h`).
pub const SLOTS: usize = 16;

/// A figure file is 64 blocks of 16 bytes. shadPS4 stops on an assertion
/// when given any other size (`SkylanderPortal::LoadFigure`), so a file is
/// measured before it is handed over.
const FIGURE_BYTES: u64 = 0x40 * 0x10;

const GONE: &str = "shadPS4 isn't answering. Close the game and start it again.";

struct Running {
    pid: u32,
    input: ChildStdin,
    /// The name of the figure Omoio put in each place, empty where there is
    /// none. shadPS4 can't be asked what is on its portal, and nothing but
    /// Omoio puts figures there.
    slots: [String; SLOTS],
    paused: bool,
}

/// The game running now. Omoio runs one game at a time.
static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

/// Starts shadPS4 with the line open and lets the game begin. Returns the
/// process id.
pub fn start(mut command: Command) -> Result<u32, String> {
    let mut child = command
        .env("SHADPS4_ENABLE_IPC", "true")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Couldn't start shadPS4.".to_string())?;
    let (Some(mut input), Some(mut answers)) = (child.stdin.take(), child.stderr.take()) else {
        return Err("Couldn't start shadPS4.".to_string());
    };
    // Its answers are read and dropped: a pipe nobody reads fills up, and then
    // shadPS4 would stop at its next line.
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut answers, &mut std::io::sink());
    });
    input
        .write_all(b"RUN\nSTART\n")
        .and_then(|_| input.flush())
        .map_err(|_| "shadPS4 closed as it started.".to_string())?;
    let pid = child.id();
    *RUNNING.lock().unwrap() = Some(Running {
        pid,
        input,
        slots: Default::default(),
        paused: false,
    });
    Ok(pid)
}

/// Works on the game `pid` is running, if it is still the one Omoio started.
fn with_game<T>(pid: u32, act: impl FnOnce(&mut Running) -> Result<T, String>) -> Result<T, String> {
    let mut running = RUNNING.lock().unwrap();
    let game = running.as_mut().filter(|game| game.pid == pid).ok_or(GONE)?;
    act(game)
}

/// One command and its values, a line each.
fn send(game: &mut Running, lines: &[&str]) -> Result<(), String> {
    let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
    game.input
        .write_all(text.as_bytes())
        .and_then(|_| game.input.flush())
        .map_err(|_| GONE.to_string())
}

/// What is on the portal, by place, empty where nothing is.
pub fn figures(pid: u32) -> Result<Vec<String>, String> {
    with_game(pid, |game| Ok(game.slots.to_vec()))
}

/// Puts the figure in `file` on the portal in `slot`.
pub fn load(pid: u32, slot: usize, file: &Path) -> Result<Vec<String>, String> {
    if slot >= SLOTS {
        return Err("The portal has no place for another figure.".to_string());
    }
    let size = std::fs::metadata(file)
        .map(|meta| meta.len())
        .map_err(|_| "That figure's file isn't there any more.".to_string())?;
    if size != FIGURE_BYTES {
        return Err("That file isn't a Skylanders figure shadPS4 can read.".to_string());
    }
    let name = file
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    with_game(pid, |game| {
        if !game.slots[slot].is_empty() {
            return Err("That place on the portal is taken.".to_string());
        }
        // The pad number is for toy pads with several sides; a Skylanders
        // portal has one, and shadPS4 ignores it.
        send(game, &["USB_LOAD_FIGURE", &file.to_string_lossy(), "0", &slot.to_string()])?;
        game.slots[slot] = name;
        Ok(game.slots.to_vec())
    })
}

/// Takes the figure in `slot` off the portal. shadPS4 writes what the game
/// changed in it back to its file as it goes.
pub fn clear(pid: u32, slot: usize) -> Result<Vec<String>, String> {
    with_game(pid, |game| {
        // A place Omoio never filled is left alone: shadPS4 would take
        // whatever figure sits first on its portal instead.
        if game.slots.get(slot).is_some_and(|name| !name.is_empty()) {
            send(game, &["USB_REMOVE_FIGURE", "0", &slot.to_string(), "1"])?;
            game.slots[slot].clear();
        }
        Ok(game.slots.to_vec())
    })
}

/// Pauses the game while one of Omoio's menus is over it, so it hears
/// nothing from the pad, and lets it carry on after. A figure put on while
/// it is paused is there when it carries on.
pub fn pause(pid: u32, paused: bool) -> Result<(), String> {
    with_game(pid, |game| {
        if game.paused != paused {
            send(game, &[if paused { "PAUSE" } else { "RESUME" }])?;
            game.paused = paused;
        }
        Ok(())
    })
}
