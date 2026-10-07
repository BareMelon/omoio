#[cfg(target_os = "linux")]
fn main() {
    std::process::exit(omoio_lib::linux_smoke::run());
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This integration check runs on Linux.");
}
