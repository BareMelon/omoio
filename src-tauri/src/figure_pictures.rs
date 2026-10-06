//! Pictures of the Skylanders figures for the portal menu, read out of the
//! user's own copy of the game. The reading is done by a separate program,
//! omoio-portraits, kept in its own repository so that the game-format
//! readers it needs don't live in Omoio. Omoio downloads it only when the
//! user asks for the pictures, and checks it against the fingerprint below
//! every time before it runs, so nothing else can stand in for it.

use crate::backends::EmulatorBackend;
use crate::core::library::Game;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

const READER_URL: &str = "https://github.com/Bertrram/omoio-portraits/releases/download/v0.2.0/omoio-portraits.exe";
const READER_SHA256: &str = "cc5ee1c6cc71c53ff37d39e27da3c46e4f359c67c6f591ad54386a51fc0df2a9";
const READER: &str = "omoio-portraits.exe";

const NO_COPY: &str = "Omoio reads the pictures from a copy of the game that Cemu makes, and there isn't one yet. In Cemu's Title Manager, right-click the game, choose Convert to compressed Wii U archive, and save it in the game's folder.";

/// The reader while it runs, so it can be stopped.
static RUNNING: Mutex<Option<Child>> = Mutex::new(None);

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().data_dir().map_err(|e| e.to_string())?.join("Omoio"))
}

/// A title id becomes a folder name here, so only letters and digits pass.
fn is_plain_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 16 && id.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Where a game's pictures are kept: one folder per game, so each shows its
/// figures as that game draws them.
fn folder(app: &AppHandle, title_id: &str) -> Result<PathBuf, String> {
    if !is_plain_id(title_id) {
        return Err("That game isn't in the library any more.".to_string());
    }
    Ok(data_dir(app)?.join("figure-pictures").join(title_id))
}

#[derive(serde::Serialize)]
pub struct Pictures {
    folder: String,
    /// Each picture's name without `.png`: `<id>-<variant>`, the variant as
    /// four hex digits.
    names: Vec<String>,
}

/// The pictures a game has so far.
pub fn pictures(app: &AppHandle, title_id: &str) -> Result<Pictures, String> {
    let folder = folder(app, title_id)?;
    let names = std::fs::read_dir(&folder)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str()?.strip_suffix(".png").map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(Pictures {
        folder: folder.to_string_lossy().into_owned(),
        names,
    })
}

fn fingerprint(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// The reader, downloaded the first time and checked every time.
async fn reader(app: &AppHandle) -> Result<PathBuf, String> {
    let path = app
        .path()
        .local_data_dir()
        .map_err(|e| e.to_string())?
        .join("Omoio")
        .join("tools")
        .join(READER);
    if std::fs::read(&path).is_ok_and(|bytes| fingerprint(&bytes) == READER_SHA256) {
        return Ok(path);
    }
    let offline = |_| "Couldn't download the picture reader. Check the internet connection and try again.".to_string();
    let bytes = reqwest::Client::new()
        .get(READER_URL)
        .header("User-Agent", "Omoio")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(offline)?
        .bytes()
        .await
        .map_err(offline)?;
    if fingerprint(&bytes) != READER_SHA256 {
        return Err("The picture reader that came down isn't the one Omoio expects, so it wasn't run.".to_string());
    }
    let unwritable = |_| "Couldn't keep the picture reader in Omoio's folder.".to_string();
    std::fs::create_dir_all(path.parent().ok_or("Couldn't keep the picture reader.")?).map_err(unwritable)?;
    std::fs::write(&path, &bytes).map_err(unwritable)?;
    Ok(path)
}

fn run(reader: &Path) -> Command {
    let mut command = Command::new(reader);
    // The reader writes to a console, and Windows would open a window for it.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Which game an archive holds, as the reader finds it in the archive's own
/// meta.xml.
fn title_of(reader: &Path, copy: &Path) -> Option<String> {
    let out = run(reader).arg("title").arg(copy).output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("title ").map(str::to_string))
}

#[derive(Clone, serde::Serialize)]
struct Progress {
    title_id: String,
    done: usize,
    of: usize,
}

/// Reads the figures' pictures out of the game and keeps them, in place of
/// any from before. Returns how many there are. Progress goes out as
/// `figure-pictures` events.
pub async fn get(app: AppHandle, backend: &'static dyn EmulatorBackend, game: Game) -> Result<usize, String> {
    let reader = reader(&app).await?;
    let folder = folder(&app, &game.title_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let copy = backend
            .readable_copy(&app, &game, &|path| title_of(&reader, path))
            .ok_or(NO_COPY)?;
        // Written beside and swapped in at the end, so a run that fails or is
        // stopped leaves the pictures from before as they were.
        let fresh = folder.with_extension("new");
        let _ = std::fs::remove_dir_all(&fresh);
        let mut child = run(&reader)
            .arg("pictures")
            .arg(&copy)
            .arg(&fresh)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| "Couldn't start the picture reader.".to_string())?;
        let (out, err) = (child.stdout.take(), child.stderr.take());
        *RUNNING.lock().unwrap() = Some(child);

        let mut written = 0;
        for line in out.map(BufReader::new).into_iter().flat_map(BufRead::lines).map_while(Result::ok) {
            if let Some((done, of)) = line.strip_prefix("progress ").and_then(|rest| rest.split_once(' ')) {
                let progress = Progress {
                    title_id: game.title_id.clone(),
                    done: done.parse().unwrap_or(0),
                    of: of.parse().unwrap_or(0),
                };
                let _ = app.emit("figure-pictures", progress);
            } else if let Some(count) = line.strip_prefix("done ") {
                written = count.parse().unwrap_or(0);
            }
        }
        let Some(mut child) = RUNNING.lock().unwrap().take() else {
            let _ = std::fs::remove_dir_all(&fresh);
            return Err("Stopped. The pictures from before are kept.".to_string());
        };
        let finished = child.wait().is_ok_and(|status| status.success());
        if !finished {
            let mut said = String::new();
            let _ = err.map(|mut err| err.read_to_string(&mut said));
            let _ = std::fs::remove_dir_all(&fresh);
            let said = said.trim();
            return Err(if said.is_empty() { "Couldn't read the pictures out of the game." } else { said }.to_string());
        }
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::rename(&fresh, &folder).map_err(|_| "Couldn't keep the pictures in Omoio's folder.".to_string())?;
        Ok(written)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Stops the reader if it is running.
pub fn stop() {
    if let Some(mut child) = RUNNING.lock().unwrap().take() {
        let _ = child.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_ids_become_folders() {
        assert!(is_plain_id("WUD87E51FD0F7F95"));
        assert!(is_plain_id("0005000010140400"));
        assert!(!is_plain_id("..\\..\\Windows"));
        assert!(!is_plain_id(""));
    }

    #[test]
    fn the_fingerprint_is_sha256_in_hex() {
        assert_eq!(fingerprint(b"hello"), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }
}
