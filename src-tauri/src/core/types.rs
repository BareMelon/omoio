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

#[derive(Clone, Serialize)]
pub struct Progress {
    pub stage: String,
    pub bytes: u64,
    pub total: u64,
}
