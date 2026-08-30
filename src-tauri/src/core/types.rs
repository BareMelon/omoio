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

/// A library entry as the interface needs it. `available` is false when the
/// folder is gone right now, which for a game on an external drive is a normal
/// state rather than a broken entry.
#[derive(Serialize)]
pub struct GameEntry {
    #[serde(flatten)]
    pub game: crate::core::library::Game,
    pub available: bool,
}

#[derive(Clone, Serialize)]
pub struct Progress {
    pub stage: String,
    pub bytes: u64,
    pub total: u64,
}
