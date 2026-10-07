//! Native launch helpers. Linux AppImages are extracted once at install time,
//! so starting an emulator needs neither FUSE nor a mount helper.

use std::path::Path;
use std::process::{Child, Command};

pub fn track(mut child: Child) -> u32 {
    let pid = child.id();
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    pid
}

#[cfg(target_os = "linux")]
pub fn start_fullscreen(app: &tauri::AppHandle) -> bool {
    use tauri::Manager;
    crate::big_picture::is_on()
        || app
            .path()
            .data_dir()
            .ok()
            .map(|dir| {
                crate::core::settings::Settings::load(&dir.join("Omoio/settings.json"))
                    .start_fullscreen
            })
            .unwrap_or(false)
}

pub fn command(exe: &Path) -> Command {
    let mut command = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // Own the process group so Stop also reaches emulator subprocesses.
        command.process_group(0);
        // Do not leak the launcher's own AppImage paths into a nested AppImage.
        for key in [
            "APPIMAGE",
            "APPDIR",
            "OWD",
            "ARGV0",
            "LD_LIBRARY_PATH",
            "LD_PRELOAD",
        ] {
            command.env_remove(key);
        }
    }
    command
}

#[cfg(target_os = "linux")]
pub fn executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
pub fn install_appimage(image: &Path, dest: &Path) -> Result<(), String> {
    executable(image)?;
    let staging = dest.join("extracting");
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let output = command(image)
        .arg("--appimage-extract")
        .current_dir(&staging)
        .output()
        .map_err(|e| format!("Couldn't extract the emulator AppImage: {e}"))?;
    let unpacked = staging.join("squashfs-root");
    if !output.status.success() || !unpacked.join("AppRun").is_file() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err("The emulator AppImage could not be extracted.".to_string());
    }
    let current = dest.join("AppDir");
    let previous = dest.join("AppDir.previous");
    if previous.exists() {
        std::fs::remove_dir_all(&previous).map_err(|e| e.to_string())?;
    }
    if current.exists() {
        std::fs::rename(&current, &previous).map_err(|e| e.to_string())?;
    }
    if let Err(error) = std::fs::rename(&unpacked, &current) {
        if previous.exists() {
            let _ = std::fs::rename(&previous, &current);
        }
        return Err(error.to_string());
    }
    let _ = std::fs::remove_dir_all(&previous);
    let _ = std::fs::remove_dir_all(&staging);
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn a_child_has_its_own_process_group_and_can_be_reaped() {
        let mut child = command(Path::new("/bin/sleep")).arg("30").spawn().unwrap();
        let pid = child.id();
        assert_eq!(unsafe { libc::getpgid(pid as i32) }, pid as i32);
        assert!(crate::session::still_running(pid));
        crate::session::kill(pid);
        let status = child.wait().unwrap();
        assert!(!status.success());
        assert!(!crate::session::still_running(pid));
    }
}
