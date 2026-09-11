use serde::Serialize;

#[derive(Serialize)]
pub struct HardwareInfo {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpu: Option<GpuInfo>,
    pub display: Option<DisplayInfo>,
}

#[derive(Serialize)]
pub struct CpuInfo {
    pub brand: String,
    pub physical_cores: Option<usize>,
    pub logical_cores: usize,
}

#[derive(Serialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
}

#[derive(Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub dedicated_memory_bytes: u64,
}

#[derive(Serialize)]
pub struct DisplayInfo {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

/// A library entry as the interface needs it.
///
/// Two different absences, told apart on purpose. `set_up` is false for a game
/// added from the catalogue that has no files yet. `available` is false for a
/// game that does have files, on a drive that is not plugged in right now,
/// which is normal rather than broken.
#[derive(Serialize)]
pub struct GameEntry {
    #[serde(flatten)]
    pub game: crate::core::library::Game,
    pub available: bool,
    pub set_up: bool,
    /// The dump's own tile art, copied somewhere we control so it still shows
    /// when the drive holding the game is unplugged.
    pub cover: Option<String>,
    /// "dump" for the game's own icon, "rawg" for a RAWG cover.
    pub cover_source: Option<&'static str>,
}

#[derive(Clone, Serialize)]
pub struct Progress {
    pub stage: String,
    pub bytes: u64,
    pub total: u64,
}
