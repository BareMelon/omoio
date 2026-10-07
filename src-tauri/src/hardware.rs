use crate::core::types::{CpuInfo, DisplayInfo, GpuInfo, HardwareInfo, MemoryInfo};
use sysinfo::System;
#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS,
};

pub fn detect() -> HardwareInfo {
    let (cpu, memory) = detect_cpu_and_memory();
    HardwareInfo {
        cpu,
        memory,
        gpu: detect_gpu(),
        display: detect_display(),
    }
}

fn detect_cpu_and_memory() -> (CpuInfo, MemoryInfo) {
    let mut sys = System::new_all();
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let brand = sys
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let cpu = CpuInfo {
        brand,
        physical_cores: System::physical_core_count(),
        logical_cores: sys.cpus().len(),
    };
    let memory = MemoryInfo {
        total_bytes: sys.total_memory(),
    };
    (cpu, memory)
}

// DXGI reports dedicated video memory as a 64-bit SIZE_T. The alternative,
// WMI's Win32_VideoController.AdapterRAM, is a 32-bit field that silently
// wraps on any card with more than 4 GB of VRAM - which is most of them.
#[cfg(windows)]
fn detect_gpu() -> Option<GpuInfo> {
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
        let mut best: Option<GpuInfo> = None;

        for index in 0.. {
            let Ok(adapter) = factory.EnumAdapters1(index) else {
                break;
            };
            let Ok(desc) = adapter.GetDesc1() else {
                continue;
            };
            if (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 {
                continue;
            }

            let name = String::from_utf16_lossy(&desc.Description)
                .trim_end_matches('\0')
                .to_string();
            let dedicated_memory_bytes = desc.DedicatedVideoMemory as u64;

            let is_better = best
                .as_ref()
                .is_none_or(|b| dedicated_memory_bytes > b.dedicated_memory_bytes);
            if is_better {
                best = Some(GpuInfo {
                    name,
                    dedicated_memory_bytes,
                });
            }
        }

        best
    }
}

#[cfg(windows)]
fn detect_display() -> Option<DisplayInfo> {
    let mut mode = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    let found = unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut mode) };
    if found.as_bool() {
        Some(DisplayInfo {
            width: mode.dmPelsWidth,
            height: mode.dmPelsHeight,
            refresh_hz: mode.dmDisplayFrequency,
        })
    } else {
        None
    }
}

#[cfg(not(windows))]
fn detect_gpu() -> Option<GpuInfo> {
    None
}

#[cfg(not(windows))]
fn detect_display() -> Option<DisplayInfo> {
    None
}
