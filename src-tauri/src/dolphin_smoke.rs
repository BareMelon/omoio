//! Real package, GUI, private-storage and orderly-exit checks, without ROMs.
use crate::backends::{self, dolphin, EmulatorBackend};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

pub async fn check(app: &tauri::AppHandle) -> Result<(), String> {
    let version = dolphin::install(app.clone(), Arc::new(AtomicBool::new(false))).await?;
    if dolphin::install::detect_version(app).as_deref() != Some(&version) {
        return Err("Dolphin version detection disagrees with installation".into());
    }
    let sys = dolphin::install::sys_dir(app)?;
    if !sys.join("GameSettings").is_dir() || !sys.join("Load/GraphicMods").is_dir() {
        return Err(format!("Dolphin's bundled packs are missing from {}", sys.display()));
    }
    let tool = dolphin::install::tool_path(app)?;
    let output = dolphin::install::command(&tool).arg("--help").output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("DolphinTool didn't start: {} {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)));
    }
    let user = dolphin::install::user_dir(app)?;
    dolphin::settings::prepare(&user.join("Config"), true, &user.join("Figures")).map_err(|e| e.to_string())?;
    let child = dolphin::install::command(&dolphin::install::exe_path(app)?)
        .arg("--user").arg(&user).spawn().map_err(|e| e.to_string())?;
    let pid = crate::platform::track(child);
    std::thread::sleep(Duration::from_secs(12));
    if !crate::session::still_running(pid) {
        return Err("Dolphin exited during its GUI startup check".into());
    }
    let closed = backends::close(&dolphin::WII, pid);
    if !closed {
        crate::session::kill(pid);
        return Err("Dolphin's native graceful-close check failed".into());
    }
    if !user.join("Config/Qt.ini").is_file() {
        return Err(format!("Dolphin didn't persist its GUI settings in the private user directory: {}", user.display()));
    }
    if dolphin::WII.features().portal {
        return Err("Native Dolphin must not advertise Windows portal automation".into());
    }
    println!("Dolphin integration passed: {version}, native GUI and tool, bundled packs, private data, graceful shutdown");
    Ok(())
}
