#[cfg(target_os = "macos")]
fn main() {
    std::process::exit(omoio_lib::macos_smoke::run());
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This integration check runs on macOS.");
}
