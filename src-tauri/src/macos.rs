//! macOS application bundles, installed without Homebrew or archive tools.

use std::path::{Path, PathBuf};
use std::process::Command;

pub fn plist_value(path: &Path, key: &str) -> Result<String, String> {
    let output = Command::new("/usr/bin/plutil")
        .args(["-extract", key, "raw", "-o", "-"])
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("Couldn't read {key} from the app bundle."));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run(command: &mut Command, action: &str) -> Result<(), String> {
    let output = command.output().map_err(|e| format!("{action}: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{action}: {}",
        String::from_utf8_lossy(&output.stderr)
            .chars()
            .take(2000)
            .collect::<String>()
    ))
}

fn staging(dest: &Path) -> Result<PathBuf, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let path = dest.join(format!("installing-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

fn replace_bundle(source: &Path, dest: &Path, name: &str, binary: &str) -> Result<(), String> {
    if !source.join("Contents/MacOS").join(binary).is_file() {
        return Err(format!(
            "The download did not contain a valid {name} application."
        ));
    }
    let current = dest.join(name);
    let previous = dest.join(format!("{name}.previous"));
    if previous.exists() {
        std::fs::remove_dir_all(&previous).map_err(|e| e.to_string())?;
    }
    if current.exists() {
        std::fs::rename(&current, &previous).map_err(|e| e.to_string())?;
    }
    if let Err(error) = std::fs::rename(source, &current) {
        if previous.exists() {
            let _ = std::fs::rename(&previous, &current);
        }
        return Err(error.to_string());
    }
    let _ = std::fs::remove_dir_all(previous);
    Ok(())
}

pub fn install_7z(archive: &Path, dest: &Path, name: &str, binary: &str) -> Result<(), String> {
    let stage = staging(dest)?;
    // macOS ships bsdtar with 7z support. It preserves the executable bits
    // and framework symlinks required by a signed macOS application bundle.
    let result = run(
        Command::new("/usr/bin/tar")
            .arg("-xf")
            .arg(archive)
            .arg("-C")
            .arg(&stage),
        "Couldn't extract the macOS emulator",
    )
    .and_then(|()| replace_bundle(&stage.join(name), dest, name, binary));
    let _ = std::fs::remove_dir_all(stage);
    result
}

pub fn install_dmg(image: &Path, dest: &Path, name: &str, binary: &str) -> Result<(), String> {
    let stage = staging(dest)?;
    let mount = stage.join("mount");
    run(
        Command::new("/usr/bin/hdiutil")
            .args(["attach", "-readonly", "-nobrowse", "-mountpoint"])
            .arg(&mount)
            .arg(image),
        "Couldn't open the emulator disk image",
    )?;
    let copied = stage.join(name);
    let copy = run(
        Command::new("/usr/bin/ditto")
            .arg(mount.join(name))
            .arg(&copied),
        "Couldn't copy the emulator application",
    );
    // Never remove the mount directory while the disk image is attached.
    run(
        Command::new("/usr/bin/hdiutil").arg("detach").arg(&mount),
        "Couldn't eject the emulator disk image",
    )?;
    let result = copy.and_then(|()| replace_bundle(&copied, dest, name, binary));
    let _ = std::fs::remove_dir_all(stage);
    result
}

pub fn require_cemu_architecture() -> Result<(), String> {
    if cfg!(target_arch = "aarch64") {
        let available = Command::new("/usr/bin/arch")
            .args(["-x86_64", "/usr/bin/true"])
            .output()
            .is_ok_and(|output| output.status.success());
        if !available {
            return Err("Cemu 2.6 needs Apple's Rosetta on Apple Silicon. Install Rosetta through macOS, then try again.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_children_use_a_stoppable_process_group() {
        let mut child = crate::platform::command(Path::new("/bin/sleep"))
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id();
        assert_eq!(unsafe { libc::getpgid(pid as i32) }, pid as i32);
        assert!(crate::session::still_running(pid));
        crate::session::kill(pid);
        assert!(!child.wait().unwrap().success());
        assert!(!crate::session::still_running(pid));
    }
}
