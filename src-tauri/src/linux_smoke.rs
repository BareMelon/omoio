//! Online integration harness for CI, invoked by the linux_smoke example.
//! It downloads official emulators but uses no firmware or game dumps.

use crate::backends::{cemu, rpcs3};
use std::sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    Arc,
};

async fn check(app: &tauri::AppHandle) -> Result<(), String> {
    let cancel = Arc::new(AtomicBool::new(false));
    let rpc_version = rpcs3::install(app.clone(), cancel.clone()).await?;
    if rpcs3::detect_version(app).as_deref() != Some(&rpc_version) {
        return Err("RPCS3 version detection disagrees with the installed binary".into());
    }
    let cemu_version = cemu::install(app.clone(), cancel).await?;
    let output = cemu::command(&cemu::exe_path(app)?)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || text.trim() != cemu_version {
        return Err(format!(
            "Native Cemu version check failed: {} {}",
            text,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let rpc_command = rpcs3::command(&rpcs3::exe_path(app)?);
    let config_home = rpc_command
        .get_envs()
        .find(|(key, _)| *key == "XDG_CONFIG_HOME")
        .and_then(|(_, value)| value)
        .ok_or("RPCS3 has no private XDG config home")?;
    if std::path::Path::new(config_home).join("rpcs3") != rpcs3::config_dir(app)? {
        return Err("RPCS3's launch environment does not match the paths Omoio writes".into());
    }
    let cemu_command = cemu::command(&cemu::exe_path(app)?);
    let data_home = cemu_command
        .get_envs()
        .find(|(key, _)| *key == "XDG_DATA_HOME")
        .and_then(|(_, value)| value)
        .ok_or("Cemu has no private XDG data home")?;
    if std::path::Path::new(data_home).join("Cemu") != cemu::user_data(&cemu::install_dir(app)?) {
        return Err("Cemu's launch environment does not match the paths Omoio writes".into());
    }
    println!("Native emulator installation passed: RPCS3 {rpc_version}, Cemu {cemu_version}");
    crate::dolphin_smoke::check(app).await?;
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
                        eprintln!("Linux emulator integration failed: {error}");
                        1
                    }
                };
                result.store(code, Ordering::SeqCst);
                handle.exit(code);
            });
            Ok(())
        })
        .run(crate::app_context())
        .expect("Linux smoke-test runtime failed");
    outcome.load(Ordering::SeqCst)
}
