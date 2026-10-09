//! Native Dolphin packaging and orderly shutdown. Linux uses the official
//! pinned Flatpak in Omoio's own installation; macOS uses the universal app.

#[cfg(target_os = "macos")]
pub fn ask_to_close(pid: u32) -> bool {
    // NSRunningApplication targets exactly the child PID, without GUI
    // scripting, Accessibility permission, or quitting another Dolphin.
    let script = format!("ObjC.import('AppKit'); var app = $.NSRunningApplication.runningApplicationWithProcessIdentifier({pid}); app.isNil() ? false : app.terminate;");
    crate::platform::command(std::path::Path::new("/usr/bin/osascript"))
        .args(["-l", "JavaScript", "-e", &script])
        .output()
        .is_ok_and(|out| out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "true")
}

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "linux")]
mod linux {
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    const APP: &str = "org.DolphinEmu.dolphin-emu";

    fn flatpak(root: &Path) -> Command {
        let mut command = crate::platform::command(Path::new("flatpak"));
        command.env("FLATPAK_USER_DIR", root.join("flatpak"));
        command.env("FLATPAK_FANCY_OUTPUT", "0");
        command.arg("--user");
        command
    }

    pub fn flatpak_command(exe: &Path) -> Command {
        // exe is the stable link inside Omoio's Dolphin directory. Its target
        // is verified at install time, but runs inside the matching runtime.
        let root = exe.parent().expect("Dolphin executable has an installation directory");
        let mut command = flatpak(root);
        command.arg("run");
        command.arg(format!("--command={}", exe.file_name().unwrap().to_string_lossy()));
        command.args(["--filesystem=host", "--filesystem=/tmp"]);
        // Also serves as an exact installation marker when locating children.
        command.arg(format!("--filesystem={}", root.display()));
        for (key, folder) in [("XDG_CONFIG_HOME", "config"), ("XDG_DATA_HOME", "data"), ("XDG_CACHE_HOME", "cache")] {
            command.arg(format!("--env={key}={}", root.join("runtime").join(folder).display()));
        }
        command.arg(APP);
        command
    }

    fn run(command: &mut Command, root: &Path, cancel: &AtomicBool) -> Result<(), String> {
        if cancel.load(Ordering::Relaxed) { return Err("cancelled".into()); }
        let log_path = root.join("flatpak-install.log");
        let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
        let mut child = command.stdin(Stdio::null())
            .stdout(log.try_clone().map_err(|e| e.to_string())?).stderr(log)
            .spawn().map_err(|e| format!("Couldn't start Flatpak: {e}"))?;
        loop {
            if cancel.load(Ordering::Relaxed) {
                crate::session::kill(child.id());
                let _ = child.wait();
                return Err("cancelled".into());
            }
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                if status.success() { return Ok(()); }
                let text = std::fs::read_to_string(&log_path).unwrap_or_default();
                let tail: String = text.chars().rev().take(1800).collect::<String>().chars().rev().collect();
                return Err(format!("Flatpak couldn't set up Dolphin: {tail}"));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn install_flatpak(bundle: &Path, root: &Path, cancel: &AtomicBool) -> Result<(), String> {
        // Only this private installation is changed. Flatpak verifies runtime
        // signatures; the Dolphin bundle's SHA-256 is checked before this.
        run(flatpak(root).args(["remote-add", "--if-not-exists", "--noninteractive", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo"]), root, cancel)?;
        run(flatpak(root).args(["install", "--noninteractive", "--assumeyes", "--or-update", "--bundle"]).arg(bundle), root, cancel)?;
        let output = flatpak(root).args(["info", "--show-location", APP]).output().map_err(|e| e.to_string())?;
        if !output.status.success() { return Err("Couldn't locate the installed Dolphin Flatpak.".into()); }
        let location = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
        let location = location.canonicalize().map_err(|e| e.to_string())?;
        let private = root.join("flatpak").canonicalize().map_err(|e| e.to_string())?;
        if !location.starts_with(&private) { return Err("Dolphin's Flatpak is outside its private installation.".into()); }
        for (name, target) in [
            ("dolphin-emu", location.join("files/bin/dolphin-emu")),
            ("dolphin-tool", location.join("files/bin/dolphin-tool")),
            ("Sys", location.join("files/share/dolphin-emu/sys")),
        ] {
            if !target.exists() { return Err(format!("Dolphin's package is missing {}", target.display())); }
            let link = root.join(name);
            if std::fs::symlink_metadata(&link).is_ok() { std::fs::remove_file(&link).map_err(|e| e.to_string())?; }
            std::os::unix::fs::symlink(&target, link).map_err(|e| e.to_string())?;
        }
        for folder in ["config", "data", "cache"] {
            std::fs::create_dir_all(root.join("runtime").join(folder)).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn is_descendant(mut candidate: u32, parent: u32) -> bool {
        for _ in 0..64 {
            if candidate == parent { return true; }
            if candidate <= 1 { return false; }
            let Ok(stat) = std::fs::read_to_string(format!("/proc/{candidate}/stat")) else { return false; };
            let Some(ppid) = stat.rsplit_once(") ").and_then(|(_, rest)| rest.split_whitespace().nth(1)).and_then(|s| s.parse().ok()) else { return false; };
            candidate = ppid;
        }
        false
    }

    pub fn ask_to_close(pid: u32) -> bool {
        use x11rb::connection::Connection;
        use x11rb::protocol::res::{ClientIdMask, ClientIdSpec, ConnectionExt as ResExt};
        use x11rb::protocol::xproto::{AtomEnum, ClientMessageEvent, ConnectionExt, EventMask};
        let Ok((conn, screen)) = x11rb::connect(None) else { return false; };
        let atom = |name: &[u8]| conn.intern_atom(false, name).ok()?.reply().ok().map(|reply| reply.atom);
        let (Some(protocols), Some(delete), Some(net_pid)) = (atom(b"WM_PROTOCOLS"), atom(b"WM_DELETE_WINDOW"), atom(b"_NET_WM_PID")) else { return false; };
        let mut pending = vec![(conn.setup().roots[screen].root, 0)];
        while let Some((window, depth)) = pending.pop() {
            // XRes reports the host PID even for Flatpak's PID namespace.
            // The window's own PID property is only a fallback, and must
            // still belong to this exact launched process tree.
            let owner = conn.res_query_client_ids(&[ClientIdSpec { client: window, mask: ClientIdMask::LOCAL_CLIENT_PID }]).ok()
                .and_then(|cookie| cookie.reply().ok()).and_then(|reply| reply.ids.into_iter().find_map(|id| id.value.first().copied()))
                .or_else(|| conn.get_property(false, window, net_pid, AtomEnum::CARDINAL, 0, 1).ok()?.reply().ok()?.value32()?.next());
            if owner.is_some_and(|owner| is_descendant(owner, pid)) {
                let supports_close = conn.get_property(false, window, protocols, AtomEnum::ATOM, 0, 64).ok()
                    .and_then(|cookie| cookie.reply().ok()).is_some_and(|reply| reply.value32().is_some_and(|mut values| values.any(|a| a == delete)));
                if supports_close {
                    let message = ClientMessageEvent::new(32, window, protocols, [delete, x11rb::CURRENT_TIME, 0, 0, 0]);
                    return conn.send_event(false, window, EventMask::NO_EVENT, message).is_ok() && conn.flush().is_ok();
                }
            }
            if depth < 4 {
                if let Some(tree) = conn.query_tree(window).ok().and_then(|cookie| cookie.reply().ok()) {
                    pending.extend(tree.children.into_iter().map(|child| (child, depth + 1)));
                }
            }
        }
        false
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn launch_uses_the_private_flatpak_and_native_tool() {
            let command = flatpak_command(Path::new("/tmp/Omoio test/dolphin/dolphin-tool"));
            let args: Vec<_> = command.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
            assert!(args.contains(&"--command=dolphin-tool".into()));
            assert!(args.contains(&"--filesystem=/tmp/Omoio test/dolphin".into()));
            assert_eq!(args.last().unwrap(), APP);
            assert!(command.get_envs().any(|(k,v)| k == "FLATPAK_USER_DIR" && v == Some(std::ffi::OsStr::new("/tmp/Omoio test/dolphin/flatpak"))));
        }

        #[test]
        fn shutdown_cannot_target_an_unrelated_process() {
            assert!(is_descendant(std::process::id(), std::process::id()));
            assert!(!is_descendant(1, std::process::id()));
            assert!(!is_descendant(std::process::id(), u32::MAX));
        }
    }
}
