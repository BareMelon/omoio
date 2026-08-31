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
}

impl Session {
    pub fn begin(&self, pid: u32, playing: Playing) {
        *self.inner.lock().unwrap() = Some(Running {
            pid,
            playing,
            window: None,
            fullscreen: false,
        });
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

    fn pid(&self) -> Option<u32> {
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

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;

            let session = app.state::<Session>();
            // A stop or a second launch means this watcher is finished.
            if session.pid() != Some(pid) {
                return;
            }

            if !still_running(pid) {
                let ended = session.end();
                if let Some(playing) = ended {
                    let _ = app.emit("game-stopped", playing);
                }
                return;
            }

            let Some(window) = app.get_webview_window("main") else {
                continue;
            };

            if !attached {
                // The window only exists once RPCS3 has something to draw, so
                // this keeps looking while the game boots.
                if let Some(game) = overlay::find_window(pid) {
                    overlay::attach(game, window.hwnd().map(|h| h.0 as isize).unwrap_or(0));
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
