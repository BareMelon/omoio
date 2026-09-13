//! The game currently being played.
//!
//! RPCS3 runs as its own process. Omoio keeps hold of it so the picture can be
//! placed inside our window, so Stop actually stops it, and so a game never
//! outlives the app that started it.

use crate::backends::rpcs3::overlay;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

/// Where the game picture sits inside our window: right of the sidebar, and
/// below the title bar and the top bar, which keeps showing what is running
/// and the way to stop it. Same units the interface is laid out in.
const SIDEBAR: f64 = 212.0;
const CONTENT_TOP: f64 = 38.0 + 52.0;

#[derive(Clone, Serialize)]
pub struct Playing {
    pub title_id: String,
    pub title: String,
    /// Which console, so the right emulator's log is kept afterwards.
    pub console: crate::core::console::Console,
}

#[derive(Default)]
pub struct Session {
    inner: Mutex<Option<Running>>,
}

struct Running {
    pid: u32,
    playing: Playing,
    window: Option<isize>,
    fullscreen: bool,
    started: std::time::Instant,
    /// Set by Stop, so a session we ended is never reported as a crash.
    stopped_by_us: bool,
}

impl Session {
    pub fn begin(&self, pid: u32, playing: Playing) {
        *self.inner.lock().unwrap() = Some(Running {
            pid,
            playing,
            window: None,
            fullscreen: false,
            started: std::time::Instant::now(),
            stopped_by_us: false,
        });
    }

    fn started_at(&self) -> Option<std::time::Instant> {
        self.inner.lock().unwrap().as_ref().map(|r| r.started)
    }

    fn was_stopped_by_us(&self) -> bool {
        self.inner.lock().unwrap().as_ref().is_some_and(|r| r.stopped_by_us)
    }

    pub fn playing(&self) -> Option<Playing> {
        self.inner.lock().unwrap().as_ref().map(|r| r.playing.clone())
    }

    pub fn is_fullscreen(&self) -> bool {
        self.inner.lock().unwrap().as_ref().is_some_and(|r| r.fullscreen)
    }

    pub fn set_fullscreen(&self, on: bool) {
        if let Some(running) = self.inner.lock().unwrap().as_mut() {
            running.fullscreen = on;
        }
    }

    pub fn pid(&self) -> Option<u32> {
        self.inner.lock().unwrap().as_ref().map(|r| r.pid)
    }

    fn window(&self) -> Option<isize> {
        self.inner.lock().unwrap().as_ref().and_then(|r| r.window)
    }

    fn adopt_window(&self, window: isize) {
        if let Some(running) = self.inner.lock().unwrap().as_mut() {
            running.window = Some(window);
        }
    }

    fn end(&self) -> Option<Playing> {
        self.inner.lock().unwrap().take().map(|r| r.playing)
    }

    /// Ends the game if one is running. Killing the process is how RPCS3 is
    /// stopped from outside; it keeps nothing of ours that a clean exit would
    /// save.
    pub fn stop(&self) -> bool {
        let Some(pid) = self.pid() else {
            return false;
        };
        if let Some(running) = self.inner.lock().unwrap().as_mut() {
            running.stopped_by_us = true;
        }
        kill(pid);
        true
    }
}

#[cfg(windows)]
fn kill(pid: u32) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

#[cfg(not(windows))]
fn kill(_pid: u32) {}

fn still_running(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let Ok(out) = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        else {
            return false;
        };
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        false
    }
}

/// Keeps the game picture lined up with our window, and notices when the game
/// ends. Runs until the game is gone, then tells the interface.
pub fn watch(app: AppHandle, pid: u32) {
    tauri::async_runtime::spawn(async move {
        let mut attached = false;
        // The key state is remembered until read, so clear anything left over
        // from before the game started. Otherwise an F11 pressed elsewhere
        // minutes ago throws the game to fullscreen the moment it appears.
        overlay::fullscreen_key_pressed();

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;

            let session = app.state::<Session>();
            // A stop or a second launch means this watcher is finished.
            if session.pid() != Some(pid) {
                return;
            }

            if !still_running(pid) {
                // Read the log before anything else can overwrite it.
                let stopped_by_us = session.was_stopped_by_us();
                let seconds = session
                    .started_at()
                    .map(|s| s.elapsed().as_secs())
                    .unwrap_or(0);
                let playing = session.end();
                if let Some(playing) = playing {
                    keep_session_log(&app, &playing, seconds, stopped_by_us);
                    let _ = app.emit("game-stopped", playing);
                }
                return;
            }

            let Some(window) = app.get_webview_window("main") else {
                continue;
            };
            let host = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);

            // The game holds the keyboard, so this is the only way back out of
            // a picture that covers the screen. Read it even when we are not
            // going to act on it, so a press meant for another window is
            // consumed rather than saved up for later.
            let pressed = overlay::fullscreen_key_pressed();
            if let (true, Some(game)) = (pressed, session.window()) {
                if overlay::ours_has_focus(game, host) {
                    let now = !session.is_fullscreen();
                    session.set_fullscreen(now);
                    let _ = app.emit("game-fullscreen", now);
                }
            }

            if !attached {
                // The window only exists once RPCS3 has something to draw, so
                // this keeps looking while the game boots.
                if let Some(game) = overlay::find_window(pid) {
                    overlay::attach(game, host);
                    session.adopt_window(game);
                    attached = true;
                    let _ = app.emit("game-started", session.playing());
                }
            }

            if attached {
                place(&window, &session);
            }
        }
    });
}

/// Fills in a game's version from the emulator's log when the game's own
/// files couldn't give one, as with an encrypted Wii U disc image.
fn learn_version(app: &AppHandle, title_id: &str, version: String) {
    let Ok(data_dir) = app.path().data_dir() else {
        return;
    };
    let file = data_dir.join("Omoio").join("library.json");
    let mut library = crate::core::library::Library::load(&file);
    if let Some(game) = library.get_mut(title_id).filter(|game| game.version.is_none()) {
        game.version = Some(version);
        let _ = library.save(&file);
    }
}

/// Copies what the emulator said about this session somewhere it will survive,
/// and records how it ended. Emulators overwrite their own log on the next
/// launch, so this is the only chance to keep it, and to learn from it what
/// the game's own files couldn't say.
fn keep_session_log(app: &AppHandle, playing: &Playing, seconds: u64, stopped_by_us: bool) {
    use crate::core::playlog::{self, Ending, Session as LoggedSession};

    let backend = crate::backends::for_console(playing.console);
    let log = backend
        .as_ref()
        .and_then(|backend| backend.log_file(app))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    if let Some(version) = backend.as_ref().and_then(|backend| backend.version_from_log(&log)) {
        learn_version(app, &playing.title_id, version);
    }

    let ending = if stopped_by_us {
        Ending::Stopped
    } else if playlog::has_fatal(&log) {
        Ending::Crashed
    } else {
        Ending::Closed
    };

    let Ok(data_dir) = app.path().data_dir() else {
        return;
    };
    let logs = data_dir.join("Omoio").join("logs");
    if std::fs::create_dir_all(&logs).is_err() {
        return;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let log_file = logs.join(format!("{}-{stamp}.log", playing.title_id));
    let _ = std::fs::write(&log_file, &log);

    let session = LoggedSession {
        console: playing.console,
        title_id: playing.title_id.clone(),
        title: playing.title.clone(),
        started: stamp.to_string(),
        seconds,
        ending,
        machine: playlog::read_machine(&log),
        problems: playlog::read_problems(&log, 40),
        log_file: log_file.to_string_lossy().into_owned(),
    };

    let index = logs.join("sessions.json");
    let mut all: Vec<LoggedSession> = std::fs::read_to_string(&index)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    all.insert(0, session);

    // Keep as many as the user asked for and take the old logs with them, so
    // this never grows without bound on someone's disk.
    let keep = crate::core::settings::Settings::load(&data_dir.join("Omoio").join("settings.json"))
        .keep_sessions;
    for old in all.iter().skip(keep) {
        let _ = std::fs::remove_file(&old.log_file);
    }
    all.truncate(keep);
    if let Ok(text) = serde_json::to_string_pretty(&all) {
        let _ = std::fs::write(&index, text);
    }
}

/// Fills the content area, or the whole screen when the user asked for that.
pub fn place(window: &WebviewWindow, session: &Session) {
    let Some(game) = session.window() else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);

    if session.is_fullscreen() {
        if let Ok(Some(monitor)) = window.current_monitor() {
            let pos = monitor.position();
            let size = monitor.size();
            overlay::place(game, pos.x, pos.y, size.width as i32, size.height as i32);
        }
        return;
    }

    let (Ok(pos), Ok(size)) = (window.inner_position(), window.inner_size()) else {
        return;
    };
    let left = (SIDEBAR * scale).round() as i32;
    let top = (CONTENT_TOP * scale).round() as i32;
    overlay::place(
        game,
        pos.x + left,
        pos.y + top,
        size.width as i32 - left,
        size.height as i32 - top,
    );
}
