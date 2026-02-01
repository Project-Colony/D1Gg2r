//! GPU monitoring with platform-specific backends and optional NVML.
//!
//! Detection cascade:
//! 1. NVML (if `gpu` feature enabled) — richest data for NVIDIA
//! 2. Linux: sysfs (`/sys/class/drm/card*`) — works for AMD, Intel, partial NVIDIA
//! 3. Linux: nvidia-smi CLI — fills in gaps for NVIDIA when sysfs is incomplete
//! 4. Windows: nvidia-smi CLI — full NVIDIA data
//! 5. Windows: WMI (Win32_VideoController) — all GPUs including integrated

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::sync::RwLock;
#[cfg(target_os = "windows")]
use std::sync::Mutex;
use std::time::Instant;

#[derive(Clone, Debug, Default)]
pub struct GpuInfo {
    pub name: String,
    pub temperature: f32,
    pub utilization: u32, // 0-100%
    pub memory_used: u64,
    pub memory_total: u64,
    pub power_watts: f32,
}

#[derive(Clone, Debug, Default)]
pub struct GpuSnapshot {
    pub gpus: Vec<GpuInfo>,
}

/// Collect GPU information using the best available backend.
pub fn collect_gpu_info() -> GpuSnapshot {
    // 1. Try NVML (feature-gated, NVIDIA only)
    #[cfg(feature = "gpu")]
    {
        let snap = collect_nvml();
        if !snap.gpus.is_empty() {
            return snap;
        }
    }

    // 2. Try sysfs (Linux, all vendors)
    #[cfg(target_os = "linux")]
    {
        let mut snap = collect_sysfs();
        if !snap.gpus.is_empty() {
            // 3. For NVIDIA cards with incomplete sysfs data, enrich via nvidia-smi
            enrich_with_nvidia_smi(&mut snap);
            return snap;
        }

        // 4. No sysfs cards found — try nvidia-smi standalone (e.g. container without sysfs)
        let snap = collect_nvidia_smi();
        if !snap.gpus.is_empty() {
            return snap;
        }
    }

    // 5. Windows: try nvidia-smi, then WMI for all GPUs (including integrated)
    #[cfg(target_os = "windows")]
    {
        let snap = collect_nvidia_smi_windows();
        if !snap.gpus.is_empty() {
            return snap;
        }

        let snap = collect_wmi_gpu();
        if !snap.gpus.is_empty() {
            return snap;
        }
    }

    GpuSnapshot::default()
}

// ---------------------------------------------------------------------------
// nvidia-smi backend — parses CSV output from the CLI tool
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn collect_nvidia_smi() -> GpuSnapshot {
    let gpus = query_nvidia_smi();
    GpuSnapshot { gpus }
}

/// Enrich existing sysfs-detected GPUs with nvidia-smi data where sysfs is incomplete.
#[cfg(target_os = "linux")]
fn enrich_with_nvidia_smi(snap: &mut GpuSnapshot) {
    // Only bother if there's at least one GPU with missing data
    let needs_enrichment = snap.gpus.iter().any(|g| {
        g.name.contains("NVIDIA")
            && (g.temperature == 0.0
                || g.memory_total == 0
                || g.power_watts == 0.0)
    });
    if !needs_enrichment {
        return;
    }

    let smi_gpus = query_nvidia_smi();

    // Match by index (nvidia-smi lists GPUs in order)
    let mut smi_idx = 0;
    for gpu in &mut snap.gpus {
        if !gpu.name.contains("NVIDIA") {
            continue;
        }
        if smi_idx >= smi_gpus.len() {
            break;
        }
        let smi = &smi_gpus[smi_idx];
        smi_idx += 1;

        // Replace name with nvidia-smi's (e.g. "NVIDIA GeForce RTX 2080 SUPER")
        if !smi.name.is_empty() {
            gpu.name = smi.name.clone();
        }
        // Fill in missing fields
        if gpu.temperature == 0.0 && smi.temperature != 0.0 {
            gpu.temperature = smi.temperature;
        }
        if gpu.utilization == 0 && smi.utilization != 0 {
            gpu.utilization = smi.utilization;
        }
        if gpu.memory_total == 0 && smi.memory_total != 0 {
            gpu.memory_total = smi.memory_total;
            gpu.memory_used = smi.memory_used;
        }
        if gpu.power_watts == 0.0 && smi.power_watts != 0.0 {
            gpu.power_watts = smi.power_watts;
        }
    }
}

/// Opt #3: Cache nvidia-smi results with a 5-second TTL to avoid
/// spawning a subprocess every metrics tick.
/// Uses RwLock for multiple concurrent readers.
#[cfg(target_os = "linux")]
static NVIDIA_SMI_CACHE: RwLock<Option<(Instant, Vec<GpuInfo>)>> = RwLock::new(None);

#[cfg(target_os = "linux")]
const NVIDIA_SMI_TTL_SECS: u64 = 5;

/// Run nvidia-smi and parse the CSV output into GpuInfo structs (cached).
#[cfg(target_os = "linux")]
fn query_nvidia_smi() -> Vec<GpuInfo> {
    // Fast path: read-only check with RwLock (no writer contention)
    if let Ok(guard) = NVIDIA_SMI_CACHE.read() {
        if let Some((ts, ref cached)) = *guard {
            if ts.elapsed().as_secs() < NVIDIA_SMI_TTL_SECS {
                return cached.clone();
            }
        }
    }

    let result = query_nvidia_smi_uncached();

    if let Ok(mut guard) = NVIDIA_SMI_CACHE.write() {
        *guard = Some((Instant::now(), result.clone()));
    }

    result
}

#[cfg(target_os = "linux")]
fn query_nvidia_smi_uncached() -> Vec<GpuInfo> {
    use std::process::Command;

    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,temperature.gpu,utilization.gpu,memory.used,memory.total,power.draw",
            "--format=csv,noheader,nounits",
        ])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut gpus = Vec::new();

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if fields.len() < 6 {
            continue;
        }

        let name = fields[0].to_string();
        let temperature = fields[1].parse::<f32>().unwrap_or(0.0);
        let utilization = fields[2].parse::<u32>().unwrap_or(0);
        // nvidia-smi reports memory in MiB
        let memory_used = fields[3]
            .parse::<u64>()
            .map(|m| m * 1024 * 1024)
            .unwrap_or(0);
        let memory_total = fields[4]
            .parse::<u64>()
            .map(|m| m * 1024 * 1024)
            .unwrap_or(0);
        let power_watts = fields[5].parse::<f32>().unwrap_or(0.0);

        gpus.push(GpuInfo {
            name,
            temperature,
            utilization,
            memory_used,
            memory_total,
            power_watts,
        });
    }

    gpus
}

// ---------------------------------------------------------------------------
// sysfs backend (Linux) — works for AMD, Intel; partial for NVIDIA
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn collect_sysfs() -> GpuSnapshot {
    let mut gpus = Vec::new();

    let drm = Path::new("/sys/class/drm");
    let Ok(entries) = fs::read_dir(drm) else {
        return GpuSnapshot::default();
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Match card0, card1, … (skip card0-DP-1 etc.)
        if !name_str.starts_with("card") || name_str.contains('-') {
            continue;
        }

        let card_path = entry.path();
        let device_path = card_path.join("device");
        if !device_path.exists() {
            continue;
        }

        let gpu = read_gpu_from_sysfs(&card_path, &device_path);
        // Only include if we got at least a name or some data
        if gpu.name.is_empty()
            && gpu.temperature == 0.0
            && gpu.utilization == 0
            && gpu.memory_total == 0
        {
            continue;
        }
        gpus.push(gpu);
    }

    GpuSnapshot { gpus }
}

#[cfg(target_os = "linux")]
fn read_gpu_from_sysfs(card_path: &Path, device_path: &Path) -> GpuInfo {
    let name = read_gpu_name(device_path);
    let temperature = read_hwmon_temp(device_path);
    let utilization = read_gpu_utilization(device_path);
    let (memory_used, memory_total) = read_gpu_memory(card_path, device_path);
    let power_watts = read_gpu_power(device_path);

    GpuInfo {
        name,
        temperature,
        utilization,
        memory_used,
        memory_total,
        power_watts,
    }
}

#[cfg(target_os = "linux")]
fn read_sysfs_str(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

#[cfg(target_os = "linux")]
fn read_gpu_name(device_path: &Path) -> String {
    if let Some(label) = read_sysfs_str(&device_path.join("label")) {
        return label;
    }

    if let Some(uevent) = read_sysfs_str(&device_path.join("uevent")) {
        for line in uevent.lines() {
            if let Some(driver) = line.strip_prefix("DRIVER=") {
                let vendor =
                    read_sysfs_str(&device_path.join("vendor")).unwrap_or_default();
                let vendor_name = match vendor.as_str() {
                    "0x10de" => "NVIDIA",
                    "0x1002" => "AMD",
                    "0x8086" => "Intel",
                    _ => "GPU",
                };
                return format!("{vendor_name} ({driver})");
            }
        }
    }

    device_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown GPU".into())
}

#[cfg(target_os = "linux")]
fn read_hwmon_temp(device_path: &Path) -> f32 {
    let hwmon_dir = device_path.join("hwmon");
    let Ok(entries) = fs::read_dir(&hwmon_dir) else {
        return 0.0;
    };

    for entry in entries.flatten() {
        let temp_input = entry.path().join("temp1_input");
        if let Some(val) = read_sysfs_str(&temp_input) {
            if let Ok(millideg) = val.parse::<f64>() {
                return (millideg / 1000.0) as f32;
            }
        }
    }
    0.0
}

#[cfg(target_os = "linux")]
fn read_gpu_utilization(device_path: &Path) -> u32 {
    // AMD: gpu_busy_percent
    if let Some(val) = read_sysfs_str(&device_path.join("gpu_busy_percent")) {
        if let Ok(pct) = val.parse::<u32>() {
            return pct;
        }
    }
    0
}

#[cfg(target_os = "linux")]
fn read_gpu_memory(card_path: &Path, device_path: &Path) -> (u64, u64) {
    // AMD: mem_info_vram_total / mem_info_vram_used
    let total = read_sysfs_str(&device_path.join("mem_info_vram_total"))
        .and_then(|v| v.parse::<u64>().ok());
    let used = read_sysfs_str(&device_path.join("mem_info_vram_used"))
        .and_then(|v| v.parse::<u64>().ok());

    if let (Some(t), Some(u)) = (total, used) {
        return (u, t);
    }

    // Intel: gt_total_memory
    if let Some(val) = read_sysfs_str(&card_path.join("gt_total_memory")) {
        if let Ok(bytes) = val.parse::<u64>() {
            return (0, bytes);
        }
    }

    (0, 0)
}

#[cfg(target_os = "linux")]
fn read_gpu_power(device_path: &Path) -> f32 {
    let hwmon_dir = device_path.join("hwmon");
    let Ok(entries) = fs::read_dir(&hwmon_dir) else {
        return 0.0;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(val) = read_sysfs_str(&path.join("power1_average")) {
            if let Ok(microwatts) = val.parse::<f64>() {
                return (microwatts / 1_000_000.0) as f32;
            }
        }
        if let Some(val) = read_sysfs_str(&path.join("power1_input")) {
            if let Ok(microwatts) = val.parse::<f64>() {
                return (microwatts / 1_000_000.0) as f32;
            }
        }
    }
    0.0
}

// ---------------------------------------------------------------------------
// NVML backend (optional, NVIDIA only)
// ---------------------------------------------------------------------------

#[cfg(feature = "gpu")]
fn collect_nvml() -> GpuSnapshot {
    use nvml_wrapper::Nvml;

    let nvml = match Nvml::init() {
        Ok(n) => n,
        Err(_) => return GpuSnapshot::default(),
    };

    let count = match nvml.device_count() {
        Ok(c) => c,
        Err(_) => return GpuSnapshot::default(),
    };

    let mut gpus = Vec::new();
    for i in 0..count {
        let device = match nvml.device_by_index(i) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let name = device.name().unwrap_or_else(|_| format!("GPU {i}"));
        let temperature = device
            .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
            .map(|t| t as f32)
            .unwrap_or(0.0);
        let utilization = device.utilization_rates().map(|u| u.gpu).unwrap_or(0);
        let memory = device.memory_info().ok();
        let memory_used = memory.as_ref().map(|m| m.used).unwrap_or(0);
        let memory_total = memory.as_ref().map(|m| m.total).unwrap_or(0);
        let power_watts = device
            .power_usage()
            .map(|mw| mw as f32 / 1000.0)
            .unwrap_or(0.0);

        gpus.push(GpuInfo {
            name,
            temperature,
            utilization,
            memory_used,
            memory_total,
            power_watts,
        });
    }

    GpuSnapshot { gpus }
}

// ---------------------------------------------------------------------------
// Windows backends — native WMI (no PowerShell) + nvidia-smi CLI
// ---------------------------------------------------------------------------

/// nvidia-smi on Windows — non-blocking with background refresh.
#[cfg(target_os = "windows")]
static NVIDIA_SMI_CACHE_WIN: Mutex<Option<(Instant, Vec<GpuInfo>)>> = Mutex::new(None);

#[cfg(target_os = "windows")]
static NVIDIA_SMI_REFRESH_RUNNING: Mutex<bool> = Mutex::new(false);

#[cfg(target_os = "windows")]
const NVIDIA_SMI_TTL_SECS_WIN: u64 = 5;

#[cfg(target_os = "windows")]
fn collect_nvidia_smi_windows() -> GpuSnapshot {
    let cached = if let Ok(guard) = NVIDIA_SMI_CACHE_WIN.lock() {
        if let Some((ts, ref data)) = *guard {
            if ts.elapsed().as_secs() < NVIDIA_SMI_TTL_SECS_WIN {
                return GpuSnapshot { gpus: data.clone() };
            }
            Some(data.clone())
        } else {
            None
        }
    } else {
        None
    };

    let already_running = NVIDIA_SMI_REFRESH_RUNNING
        .lock()
        .map(|g| *g)
        .unwrap_or(false);

    if !already_running {
        if let Ok(mut g) = NVIDIA_SMI_REFRESH_RUNNING.lock() {
            *g = true;
        }
        std::thread::spawn(|| {
            let result = collect_nvidia_smi_windows_blocking();
            if let Ok(mut guard) = NVIDIA_SMI_CACHE_WIN.lock() {
                *guard = Some((Instant::now(), result));
            }
            if let Ok(mut g) = NVIDIA_SMI_REFRESH_RUNNING.lock() {
                *g = false;
            }
        });
    }

    match cached {
        Some(gpus) if !gpus.is_empty() => GpuSnapshot { gpus },
        _ => GpuSnapshot::default(),
    }
}

#[cfg(target_os = "windows")]
fn collect_nvidia_smi_windows_blocking() -> Vec<GpuInfo> {
    use std::process::Command;

    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,temperature.gpu,utilization.gpu,memory.used,memory.total,power.draw",
            "--format=csv,noheader,nounits",
        ])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut gpus = Vec::new();

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if fields.len() < 6 {
            continue;
        }

        gpus.push(GpuInfo {
            name: fields[0].to_string(),
            temperature: fields[1].parse().unwrap_or(0.0),
            utilization: fields[2].parse().unwrap_or(0),
            memory_used: fields[3].parse::<u64>().map(|m| m * 1024 * 1024).unwrap_or(0),
            memory_total: fields[4].parse::<u64>().map(|m| m * 1024 * 1024).unwrap_or(0),
            power_watts: fields[5].parse().unwrap_or(0.0),
        });
    }

    gpus
}

// ---------------------------------------------------------------------------
// Native WMI backend for Windows (all GPU vendors including integrated)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
static WMI_GPU_CACHE: Mutex<Option<(Instant, Vec<GpuInfo>)>> = Mutex::new(None);

#[cfg(target_os = "windows")]
static WMI_GPU_REFRESH_RUNNING: Mutex<bool> = Mutex::new(false);

#[cfg(target_os = "windows")]
const WMI_GPU_TTL_SECS: u64 = 3;

/// Collect GPU info via native WMI — runs on a background thread to avoid
/// COM apartment conflicts with the UI thread (winit requires STA/OLE,
/// while WMI uses MTA).
#[cfg(target_os = "windows")]
fn collect_wmi_gpu() -> GpuSnapshot {
    let cached = if let Ok(guard) = WMI_GPU_CACHE.lock() {
        if let Some((ts, ref data)) = *guard {
            if ts.elapsed().as_secs() < WMI_GPU_TTL_SECS {
                return GpuSnapshot { gpus: data.clone() };
            }
            Some(data.clone())
        } else {
            None
        }
    } else {
        None
    };

    // Spawn background thread for WMI (COM init must not be on the UI thread)
    let already_running = WMI_GPU_REFRESH_RUNNING
        .lock()
        .map(|g| *g)
        .unwrap_or(false);

    if !already_running {
        if let Ok(mut g) = WMI_GPU_REFRESH_RUNNING.lock() {
            *g = true;
        }
        std::thread::spawn(|| {
            let gpus = collect_wmi_gpu_native();
            if let Ok(mut guard) = WMI_GPU_CACHE.lock() {
                *guard = Some((Instant::now(), gpus));
            }
            if let Ok(mut g) = WMI_GPU_REFRESH_RUNNING.lock() {
                *g = false;
            }
        });
    }

    GpuSnapshot { gpus: cached.unwrap_or_default() }
}

#[cfg(target_os = "windows")]
fn collect_wmi_gpu_native() -> Vec<GpuInfo> {
    use serde::Deserialize;
    use std::collections::HashMap;
    use wmi::{COMLibrary, Variant, WMIConnection};

    // Use without_security() because CoInitializeSecurity is process-wide
    // and winit/iced may have already called it on the UI thread.
    let com_lib = match COMLibrary::without_security() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // 1. Get GPU name and adapter RAM from Win32_VideoController
    let wmi_con = match WMIConnection::new(com_lib) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    #[derive(Deserialize)]
    #[serde(rename = "Win32_VideoController")]
    struct VideoController {
        #[serde(rename = "Name")]
        name: Option<String>,
        #[serde(rename = "AdapterRAM")]
        adapter_ram: Option<u64>,
    }

    let controllers: Vec<VideoController> = wmi_con.query().unwrap_or_default();
    if controllers.is_empty() {
        return Vec::new();
    }

    let mut gpus: Vec<GpuInfo> = controllers
        .into_iter()
        .filter_map(|vc| {
            let name = vc.name.filter(|n| !n.is_empty())?;
            Some(GpuInfo {
                name,
                temperature: 0.0,
                utilization: 0,
                memory_used: 0,
                memory_total: vc.adapter_ram.unwrap_or(0),
                power_watts: 0.0,
            })
        })
        .collect();

    if gpus.is_empty() {
        return gpus;
    }

    // 2. GPU utilization via WMI perf counter class (Win10 1709+)
    //    Sum all 3D engine utilization percentages
    let util_query = "SELECT UtilizationPercentage FROM \
        Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine \
        WHERE Name LIKE '%engtype_3D'";
    if let Ok(results) = wmi_con.raw_query::<HashMap<String, Variant>>(util_query) {
        let total_util: f64 = results
            .iter()
            .filter_map(|row| match row.get("UtilizationPercentage") {
                Some(Variant::UI8(v)) => Some(*v as f64),
                Some(Variant::UI4(v)) => Some(*v as f64),
                Some(Variant::UI2(v)) => Some(*v as f64),
                Some(Variant::R8(v)) => Some(*v),
                Some(Variant::R4(v)) => Some(*v as f64),
                Some(Variant::String(s)) => s.parse::<f64>().ok(),
                _ => None,
            })
            .sum();
        let clamped = total_util.min(100.0);
        for gpu in &mut gpus {
            gpu.utilization = clamped.round() as u32;
        }
    }

    // 3. GPU memory usage via WMI perf counter class
    //    Try DedicatedUsage first (discrete GPUs), then SharedUsage (Intel iGPUs)
    let mem_query = "SELECT DedicatedUsage, SharedUsage FROM \
        Win32_PerfFormattedData_GPUPerformanceCounters_GPUAdapterMemory";
    if let Ok(results) = wmi_con.raw_query::<HashMap<String, Variant>>(mem_query) {
        let extract = |key: &str| -> u64 {
            results
                .iter()
                .filter_map(|row| match row.get(key) {
                    Some(Variant::UI8(v)) => Some(*v),
                    Some(Variant::UI4(v)) => Some(*v as u64),
                    Some(Variant::String(s)) => s.parse::<u64>().ok(),
                    _ => None,
                })
                .sum()
        };
        let dedicated = extract("DedicatedUsage");
        let shared = extract("SharedUsage");
        let used = if dedicated > 0 { dedicated } else { shared };
        if used > 0 {
            for gpu in &mut gpus {
                gpu.memory_used = used;
            }
        }
    }

    // 4. Temperature — try LHM/OHM WMI, then thermal zones, then ACPI
    enrich_gpu_temps_wmi(&wmi_con, &mut gpus);

    gpus
}

/// Try to get GPU temperatures from various WMI sources.
#[cfg(target_os = "windows")]
fn enrich_gpu_temps_wmi(
    default_con: &wmi::WMIConnection,
    gpus: &mut [GpuInfo],
) {
    use std::collections::HashMap;
    use wmi::{COMLibrary, Variant, WMIConnection};

    fn variant_f64(v: &Variant) -> Option<f64> {
        match v {
            Variant::UI8(n) => Some(*n as f64),
            Variant::UI4(n) => Some(*n as f64),
            Variant::UI2(n) => Some(*n as f64),
            Variant::UI1(n) => Some(*n as f64),
            Variant::I4(n) => Some(*n as f64),
            Variant::R8(n) => Some(*n),
            Variant::R4(n) => Some(*n as f64),
            Variant::String(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    let com_lib = match COMLibrary::without_security() {
        Ok(c) => c,
        Err(_) => return,
    };

    // 1. Try LibreHardwareMonitor / OpenHardwareMonitor
    for ns in &["ROOT\\LibreHardwareMonitor", "ROOT\\OpenHardwareMonitor"] {
        if let Ok(con) = WMIConnection::with_namespace_path(ns, com_lib) {
            if let Ok(results) = con.raw_query::<HashMap<String, Variant>>(
                "SELECT Name, SensorType, Value FROM Sensor WHERE SensorType = 'Temperature'",
            ) {
                let gpu_temps: Vec<f32> = results
                    .iter()
                    .filter(|row| {
                        matches!(row.get("Name"), Some(Variant::String(n)) if n.to_lowercase().contains("gpu"))
                    })
                    .filter_map(|row| Some(variant_f64(row.get("Value")?)? as f32))
                    .filter(|t| *t > 0.0 && *t < 150.0)
                    .collect();

                for (gpu, temp) in gpus.iter_mut().zip(gpu_temps.iter()) {
                    if gpu.temperature == 0.0 {
                        gpu.temperature = *temp;
                    }
                }
                if gpu_temps.iter().any(|t| *t != 0.0) {
                    return;
                }
            }
        }
    }

    // 2. Fallback: thermal zone perf counter (Kelvin → Celsius)
    //    For integrated GPUs on the CPU die, system thermal zone is a reasonable proxy.
    if let Ok(results) = default_con.raw_query::<HashMap<String, Variant>>(
        "SELECT Temperature FROM Win32_PerfFormattedData_Counters_ThermalZoneInformation",
    ) {
        // Use the highest thermal zone temp as GPU proxy (hottest zone ≈ CPU/iGPU die)
        let mut best_celsius: f32 = 0.0;
        for row in &results {
            if let Some(kelvin) = row.get("Temperature").and_then(variant_f64) {
                let celsius = kelvin as f32 - 273.15;
                if celsius > 0.0 && celsius < 150.0 && celsius > best_celsius {
                    best_celsius = celsius;
                }
            }
        }
        if best_celsius > 0.0 {
            for gpu in gpus.iter_mut() {
                if gpu.temperature == 0.0 {
                    gpu.temperature = best_celsius;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_snapshot_default() {
        let snap = GpuSnapshot::default();
        assert!(snap.gpus.is_empty());
    }

    #[test]
    fn test_collect_gpu_returns_something() {
        let snap = collect_gpu_info();
        let _ = snap.gpus.len();
    }

    #[test]
    fn test_parse_nvidia_smi_output() {
        // Simulate what query_nvidia_smi would parse
        let line = "NVIDIA GeForce RTX 2080 SUPER, 45, 3, 1024, 8192, 30.50";
        let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        assert_eq!(fields.len(), 6);
        assert_eq!(fields[0], "NVIDIA GeForce RTX 2080 SUPER");
        assert_eq!(fields[1].parse::<f32>().unwrap(), 45.0);
        assert_eq!(fields[2].parse::<u32>().unwrap(), 3);
        assert_eq!(fields[3].parse::<u64>().unwrap() * 1024 * 1024, 1024 * 1024 * 1024);
        assert_eq!(fields[4].parse::<u64>().unwrap() * 1024 * 1024, 8192 * 1024 * 1024);
        assert_eq!(fields[5].parse::<f32>().unwrap(), 30.50);
    }
}
