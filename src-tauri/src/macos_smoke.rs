//! macOS integration checks use official emulator bundles and no game dumps.

use crate::backends::{cemu, rpcs3};
use std::sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    Arc,
};

async fn check(app: &tauri::AppHandle) -> Result<(), String> {
    let cancel = Arc::new(AtomicBool::new(false));
    let rpc_version = rpcs3::install(app.clone(), cancel.clone()).await?;
    if rpcs3::detect_version(app).as_deref() != Some(&rpc_version) {
        return Err("RPCS3 bundle version detection disagrees with installation".into());
    }
    // Hosted Intel Macs abort inside the upstream RPCS3/MoltenVK startup.
    // Apple Silicon runners can run the official emulator version command.
    if cfg!(target_arch = "aarch64") {
        let output = rpcs3::command(&rpcs3::exe_path(app)?)
            .args(["--headless", "--version"])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success()
            || !String::from_utf8_lossy(&output.stdout).contains(&rpc_version)
        {
            return Err(format!(
                "RPCS3 runtime probe failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        println!("RPCS3 Apple Silicon executable version check passed");
    } else {
        println!(
            "RPCS3 Intel bundle checked; runtime needs a physical Mac with supported graphics"
        );
    }
    let cemu_version = cemu::install(app.clone(), cancel).await?;
    let exe = cemu::exe_path(app)?;
    let output = cemu::command(&exe)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim() != cemu_version {
        return Err(format!(
            "Cemu runtime version check failed: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let data = cemu::user_data(&cemu::install_dir(app)?);
    // Exercise wx/Cocoa's real path lookup, rather than merely inspecting
    // environment variables that an emulator may choose not to honor.
    let mut child = cemu::command(&exe).spawn().map_err(|e| e.to_string())?;
    let mut found_log = false;
    for _ in 0..60 {
        if data.join("log.txt").is_file() {
            found_log = true;
            break;
        }
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    crate::session::kill(child.id());
    let _ = child.wait();
    if !found_log {
        return Err(format!(
            "Cemu did not write its startup log in the private data directory: {}",
            data.display()
        ));
    }
    if !cemu::install_dir(app)?
        .join("Cemu.app/Contents/Resources/gameProfiles/default")
        .is_dir()
    {
        return Err("Cemu's shipped game profiles are missing from Resources".into());
    }
    println!("macOS emulator integration passed: RPCS3 {rpc_version}, Cemu {cemu_version}, private Cemu log verified");
    Ok(())
}

pub fn run() -> i32 {
    let outcome = Arc::new(AtomicI32::new(1));
    let result = outcome.clone();
    tauri::Builder::default()
        .setup(move |app| {
            let handle = app.handle().clone();
            let result = result.clone();
            tauri::async_runtime::spawn(async move {
                let code = match check(&handle).await {
                    Ok(()) => 0,
                    Err(error) => {
                        eprintln!("macOS emulator integration failed: {error}");
                        1
                    }
                };
                result.store(code, Ordering::SeqCst);
                handle.exit(code);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("macOS smoke-test runtime failed");
    outcome.load(Ordering::SeqCst)
}
