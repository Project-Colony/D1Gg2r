use sysinfo::{System, Disks, Networks, Components, RefreshKind, CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::Path;

/// Static system information that never changes at runtime.
/// Wrapped in Arc to avoid cloning on every tick.
#[derive(Clone, Debug)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
}

/// A snapshot of system metrics at a point in time.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub timestamp: f64,
    pub cpu_usage_per_core: Vec<f32>,
    pub cpu_usage_global: f32,
    pub cpu_name: String,
    pub cpu_core_count: usize,
    pub cpu_frequency_mhz: u64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub disks: Vec<DiskInfo>,
    pub disk_io: DiskIoSnapshot,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
    pub net_interfaces: Vec<NetIfaceInfo>,
    pub temperatures: Vec<TempInfo>,
    pub processes: Vec<ProcessInfo>,
    pub gpu: crate::gpu::GpuSnapshot,
    pub uptime_secs: u64,
    pub process_count: usize,
    /// Static system info (shared via Arc, zero-cost clone).
    pub sys_info: Arc<SystemInfo>,
    /// System load averages (1m, 5m, 15m). On unsupported platforms, all zeros.
    pub load_avg: [f64; 3],
}

/// Lightweight point for the live rolling charts (no allocations).
#[derive(Clone, Copy, Debug)]
pub struct LivePoint {
    pub cpu: f32,
    pub mem_pct: f32,
    pub net_rx: u64,
    pub net_tx: u64,
    pub disk_read: u64,
    pub disk_write: u64,
}

#[derive(Clone, Debug)]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub fs_type: String,
    pub total: u64,
    pub available: u64,
    pub is_removable: bool,
}

/// Lightweight point for the live rolling charts including disk I/O.
#[derive(Clone, Copy, Debug)]
pub struct DiskIoSnapshot {
    pub read_bytes: u64,
    pub write_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct TempInfo {
    pub label: String,
    pub temp_c: f32,
}

#[derive(Clone, Debug)]
pub struct NetIfaceInfo {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub cmd: Vec<String>,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub virtual_memory_bytes: u64,
    pub uid: u32,
    pub is_desktop_app: bool,
    /// Number of tasks/threads for this process.
    pub thread_count: u32,
    /// Process status: R(unning), S(leeping), Z(ombie), D(isk-wait), etc.
    pub status: char,
}

pub struct Collector {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    /// Cached set of desktop app binary names (loaded once at startup).
    desktop_app_names: HashSet<String>,
    /// Configurable process list limit.
    process_limit: usize,
    /// Cached system info (doesn't change at runtime, shared via Arc).
    pub sys_info: Arc<SystemInfo>,
    /// Tick counter — used to skip expensive refreshes on most ticks.
    tick_count: u64,
    /// Cached disk info (disks rarely change).
    cached_disks: Vec<DiskInfo>,
    /// Tick at which disks were last refreshed.
    disks_last_refresh: u64,
}

/// Scan all .desktop files from standard XDG directories and extract
/// the binary name from the Exec= line. This lets us identify which
/// running processes are "real" desktop applications (like Firefox,
/// Steam, etc.) vs background services.
///
/// On non-Linux platforms this returns an empty set since .desktop
/// files are a freedesktop.org convention.
fn load_desktop_app_names() -> HashSet<String> {
    #[allow(unused_mut)]
    let mut names = HashSet::new();

    #[cfg(target_os = "linux")]
    {
        let dirs = [
            "/usr/share/applications",
            "/usr/local/share/applications",
        ];
        // Also add user-local .desktop files
        let home_apps = dirs::data_dir()
            .map(|d| d.join("applications"))
            .unwrap_or_default();
        let flatpak_dirs = [
            "/var/lib/flatpak/exports/share/applications",
        ];
        let snap_dirs = [
            "/var/lib/snapd/desktop/applications",
        ];

        let all_dirs: Vec<&Path> = dirs.iter().map(Path::new)
            .chain(std::iter::once(home_apps.as_path()))
            .chain(flatpak_dirs.iter().map(Path::new))
            .chain(snap_dirs.iter().map(Path::new))
            .collect();

        for dir in all_dirs {
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(&path) else { continue };
                for line in content.lines() {
                    let line = line.trim();
                    if let Some(exec) = line.strip_prefix("Exec=") {
                        // Extract the binary name: "Exec=/usr/bin/firefox %u" → "firefox"
                        // Also handle: "Exec=env VAR=x /usr/bin/foo" by skipping env vars
                        let parts: Vec<&str> = exec.split_whitespace().collect();
                        for part in &parts {
                            // Skip env-like prefixes (VAR=value, env)
                            if *part == "env" || part.contains('=') {
                                continue;
                            }
                            // Extract basename
                            if let Some(basename) = Path::new(part).file_name() {
                                let name = basename.to_string_lossy().to_string();
                                if !name.is_empty() {
                                    names.insert(name);
                                }
                            }
                            break;
                        }
                        break; // Only use first Exec= line
                    }
                }
            }
        }
    }

    // macOS: scan /Applications for .app bundles and extract the executable name
    // from Contents/Info.plist (CFBundleExecutable key) or fall back to the
    // bundle directory name.
    #[cfg(target_os = "macos")]
    {
        let app_dirs = [
            "/Applications",
            "/System/Applications",
        ];
        if let Some(home) = dirs::home_dir() {
            let user_apps = home.join("Applications");
            let all: Vec<&Path> = app_dirs.iter().map(Path::new)
                .chain(std::iter::once(user_apps.as_path()))
                .collect();
            for dir in all {
                let Ok(entries) = std::fs::read_dir(dir) else { continue };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("app") {
                        continue;
                    }
                    // Try to read CFBundleExecutable from Info.plist (plain text scan)
                    let plist = path.join("Contents").join("Info.plist");
                    if let Ok(content) = std::fs::read_to_string(&plist) {
                        // Simple extraction: find <key>CFBundleExecutable</key> then next <string>...</string>
                        let key = "CFBundleExecutable";
                        if let Some(pos) = content.find(key) {
                            let after = &content[pos..];
                            if let Some(s_start) = after.find("<string>") {
                                let val_start = s_start + 8;
                                if let Some(s_end) = after[val_start..].find("</string>") {
                                    let exec_name = &after[val_start..val_start + s_end];
                                    if !exec_name.is_empty() {
                                        names.insert(exec_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                    // Also add the bundle name without .app as fallback
                    if let Some(stem) = path.file_stem() {
                        let name = stem.to_string_lossy().to_string();
                        if !name.is_empty() {
                            names.insert(name);
                        }
                    }
                }
            }
        } else {
            // No home dir — just scan system-wide
            for dir in app_dirs.iter().map(Path::new) {
                let Ok(entries) = std::fs::read_dir(dir) else { continue };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(stem) = path.file_stem() {
                        if path.extension().and_then(|e| e.to_str()) == Some("app") {
                            names.insert(stem.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    names
}

/// Save the cached desktop app names to a local cache file for fast reloads.
fn save_desktop_cache(names: &HashSet<String>) {
    if let Some(cache_dir) = dirs::cache_dir() {
        let cache_path = cache_dir.join("digger").join("desktop_apps.txt");
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content: String = names.iter().cloned().collect::<Vec<_>>().join("\n");
        let _ = std::fs::write(&cache_path, content);
    }
}

/// Try to load desktop app names from cache. Returns None if cache is stale or missing.
fn load_desktop_cache() -> Option<HashSet<String>> {
    let cache_dir = dirs::cache_dir()?;
    let cache_path = cache_dir.join("digger").join("desktop_apps.txt");
    let metadata = std::fs::metadata(&cache_path).ok()?;
    let modified = metadata.modified().ok()?;
    // Cache is valid for 1 hour
    if modified.elapsed().ok()? > std::time::Duration::from_secs(3600) {
        return None;
    }
    let content = std::fs::read_to_string(&cache_path).ok()?;
    Some(content.lines().filter(|l| !l.is_empty()).map(String::from).collect())
}

/// Windows: Enumerate all visible top-level windows and return the set of
/// owning PIDs. A process that owns at least one visible window is considered
/// an "application" (like in the Windows Task Manager).
#[cfg(target_os = "windows")]
fn get_windowed_pids() -> HashSet<u32> {
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };

    static PIDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

    unsafe extern "system" fn enum_callback(hwnd: HWND, _: LPARAM) -> BOOL {
        if IsWindowVisible(hwnd) != 0 {
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid != 0 {
                if let Ok(mut pids) = PIDS.lock() {
                    pids.push(pid);
                }
            }
        }
        1 // continue enumeration
    }

    if let Ok(mut pids) = PIDS.lock() {
        pids.clear();
    }
    unsafe {
        EnumWindows(Some(enum_callback), 0);
    }
    let result = if let Ok(pids) = PIDS.lock() {
        pids.iter().copied().collect()
    } else {
        HashSet::new()
    };
    result
}

/// Windows: Names of processes that are core OS components.
/// Only processes matching this list AND running under a system SID AND
/// located under the Windows directory are classified as "System".
/// Third-party services (AnyDesk, antivirus, etc.) run under SYSTEM but
/// are NOT OS components and should appear as Background processes instead.
#[cfg(target_os = "windows")]
fn is_known_windows_system_process(name: &str) -> bool {
    const KNOWN: &[&str] = &[
        // Kernel & core
        "System", "Registry", "smss.exe", "csrss.exe", "wininit.exe",
        "services.exe", "lsass.exe", "lsaiso.exe", "svchost.exe",
        "fontdrvhost.exe", "dwm.exe", "conhost.exe", "sihost.exe",
        "taskhostw.exe", "RuntimeBroker.exe", "ShellExperienceHost.exe",
        "StartMenuExperienceHost.exe", "SearchHost.exe", "TextInputHost.exe",
        "ctfmon.exe", "dllhost.exe", "WmiPrvSE.exe",
        // Security & updates
        "MsMpEng.exe", "NisSrv.exe", "SecurityHealthService.exe",
        "SecurityHealthSystray.exe", "SgrmBroker.exe", "MpDefenderCoreService.exe",
        "wuauclt.exe", "TrustedInstaller.exe", "TiWorker.exe",
        // System services
        "spoolsv.exe", "SearchIndexer.exe", "SearchProtocolHost.exe",
        "SearchFilterHost.exe", "audiodg.exe", "WUDFHost.exe",
        "dasHost.exe", "CompPkgSrv.exe", "SystemSettingsBroker.exe",
        "SettingSyncHost.exe", "backgroundTaskHost.exe", "UserOOBEBroker.exe",
        // Networking
        "lsm.exe", "iphlpsvc.exe", "mDNSResponder.exe",
        // Graphics / display
        "winlogon.exe", "LogonUI.exe", "LockApp.exe",
        // Memory / storage
        "MemCompression", "vmmem", "System Idle Process",
        "wbengine.exe", "vds.exe",
        // Misc core
        "sppsvc.exe", "SppExtComObj.Exe", "uhssvc.exe",
        "WaaSMedicAgent.exe", "AgentService.exe", "MoUsoCoreWorker.exe",
        "musNotificationUx.exe", "MusNotifyIcon.exe",
    ];
    // sysinfo may return names with or without the .exe suffix depending
    // on the Windows version, so strip it for comparison on both sides.
    fn strip_exe(s: &str) -> &str {
        s.strip_suffix(".exe")
            .or_else(|| s.strip_suffix(".EXE"))
            .or_else(|| s.strip_suffix(".Exe"))
            .unwrap_or(s)
    }
    let base = strip_exe(name);
    KNOWN.iter().any(|&k| strip_exe(k).eq_ignore_ascii_case(base))
}

/// Windows: Check if an executable path is under the Windows directory
/// (typically C:\Windows). Third-party apps installed in Program Files
/// or other locations are not system processes even if they run as SYSTEM.
#[cfg(target_os = "windows")]
fn is_windows_system_path(exe_path: &str) -> bool {
    if exe_path.is_empty() {
        return false;
    }
    let lower = exe_path.to_ascii_lowercase();
    // Matches any drive letter: c:\windows\, d:\windows\, etc.
    (lower.len() > 3 && &lower[1..3] == ":\\" && lower[3..].starts_with("windows\\"))
        || lower.starts_with("\\systemroot\\")
}

/// Windows: Check if a process is owned by the current interactive user.
/// Returns true when the process token SID matches the caller's SID.
/// Processes that cannot be inspected (access denied) are assumed non-user.
#[cfg(target_os = "windows")]
fn is_current_user_process(pid: u32) -> bool {
    use std::ptr;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        EqualSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        // Get the current process' user SID for comparison
        let mut our_token: HANDLE = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut our_token) == 0 {
            return false;
        }
        let mut our_buf = vec![0u8; 256];
        let mut needed = 0u32;
        if GetTokenInformation(
            our_token, TokenUser, our_buf.as_mut_ptr().cast(),
            our_buf.len() as u32, &mut needed,
        ) == 0 {
            CloseHandle(our_token);
            return false;
        }
        let our_sid = (*(our_buf.as_ptr() as *const TOKEN_USER)).User.Sid;

        // Open the target process
        let process: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            CloseHandle(our_token);
            return false; // can't open → not user-owned
        }
        let mut token: HANDLE = ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            CloseHandle(process);
            CloseHandle(our_token);
            return false;
        }

        let mut buf = vec![0u8; 256];
        let ok = GetTokenInformation(
            token, TokenUser, buf.as_mut_ptr().cast(),
            buf.len() as u32, &mut needed,
        );
        let is_ours = if ok != 0 {
            let their_sid = (*(buf.as_ptr() as *const TOKEN_USER)).User.Sid;
            EqualSid(our_sid, their_sid) != 0
        } else {
            false
        };

        CloseHandle(token);
        CloseHandle(process);
        CloseHandle(our_token);
        is_ours
    }
}

/// Windows: Determine if a process should be classified as "System".
///
/// A process is "System" if it does NOT belong to the current user AND
/// is either a known OS component or its executable lives under the
/// Windows directory. Third-party services (AnyDesk, antivirus, etc.)
/// that run under service accounts are classified as Background instead.
#[cfg(target_os = "windows")]
fn is_system_process(pid: u32, name: &str, exe_path: &str) -> bool {
    if is_current_user_process(pid) {
        return false;
    }
    // Known OS process name → system
    if is_known_windows_system_process(name) {
        return true;
    }
    // Executable lives under C:\Windows → system
    if is_windows_system_path(exe_path) {
        return true;
    }
    // Non-user process but third-party → Background, not System
    false
}

/// Windows: Batch-check which PIDs are system processes.
#[cfg(target_os = "windows")]
fn get_system_pids(procs: &[(u32, String, String)]) -> HashSet<u32> {
    procs.iter()
        .filter(|(pid, name, exe_path)| is_system_process(*pid, name, exe_path))
        .map(|(pid, _, _)| *pid)
        .collect()
}

impl Collector {
    pub fn with_process_limit(limit: usize) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        // Try cached desktop names first, fall back to scanning
        let desktop_app_names = load_desktop_cache().unwrap_or_else(|| {
            let names = load_desktop_app_names();
            save_desktop_cache(&names);
            names
        });

        let sys_info = Arc::new(SystemInfo {
            os_name: System::name().unwrap_or_else(|| "Unknown".into()),
            os_version: System::os_version().unwrap_or_else(|| "Unknown".into()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".into()),
            hostname: System::host_name().unwrap_or_else(|| "Unknown".into()),
        });

        let disks = Disks::new_with_refreshed_list();
        let cached_disks = disks.iter().map(|d| DiskInfo {
            name: d.name().to_string_lossy().to_string(),
            mount: d.mount_point().to_string_lossy().to_string(),
            fs_type: d.file_system().to_string_lossy().to_string(),
            total: d.total_space(),
            available: d.available_space(),
            is_removable: d.is_removable(),
        }).collect();

        Self {
            sys,
            disks,
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            desktop_app_names,
            process_limit: limit,
            sys_info,
            tick_count: 0,
            cached_disks,
            disks_last_refresh: 0,
        }
    }

    pub fn collect(&mut self) -> Snapshot {
        self.tick_count += 1;

        // Opt #1 & #8: Only refresh what we actually use.
        // CPU frequency rarely changes — refresh it every 10 ticks.
        let cpu_refresh = if self.tick_count % 10 == 0 {
            CpuRefreshKind::everything()
        } else {
            CpuRefreshKind::new().with_cpu_usage()
        };

        // Opt #1: Only refresh process fields we need (cpu, memory, disk usage).
        let proc_refresh = ProcessRefreshKind::new()
            .with_cpu()
            .with_memory()
            .with_disk_usage();

        self.sys.refresh_specifics(
            RefreshKind::new()
                .with_cpu(cpu_refresh)
                .with_memory(MemoryRefreshKind::everything())
                .with_processes(proc_refresh),
        );
        self.networks.refresh();
        self.components.refresh();

        // Opt #9: Only rebuild DiskInfo every 30 ticks (disks rarely change).
        if self.tick_count - self.disks_last_refresh >= 30 {
            self.disks_last_refresh = self.tick_count;
            self.disks.refresh();
            self.cached_disks = self.disks.iter().map(|d| DiskInfo {
                name: d.name().to_string_lossy().to_string(),
                mount: d.mount_point().to_string_lossy().to_string(),
                fs_type: d.file_system().to_string_lossy().to_string(),
                total: d.total_space(),
                available: d.available_space(),
                is_removable: d.is_removable(),
            }).collect();
        } else {
            // Just refresh available space (cheap)
            self.disks.refresh();
            for (cached, live) in self.cached_disks.iter_mut().zip(self.disks.iter()) {
                cached.available = live.available_space();
            }
        }

        let cpu_usage_per_core: Vec<f32> = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let cpu_usage_global = if cpu_usage_per_core.is_empty() {
            0.0
        } else {
            cpu_usage_per_core.iter().sum::<f32>() / cpu_usage_per_core.len() as f32
        };

        let (mut rx, mut tx) = (0u64, 0u64);
        let mut net_interfaces = Vec::new();
        for (name, data) in self.networks.iter() {
            rx += data.received();
            tx += data.transmitted();
            net_interfaces.push(NetIfaceInfo {
                name: name.clone(),
                rx_bytes: data.received(),
                tx_bytes: data.transmitted(),
            });
        }

        #[allow(unused_mut)]
        let mut temperatures: Vec<TempInfo> = self
            .components
            .iter()
            .map(|c| TempInfo {
                label: c.label().to_string(),
                temp_c: c.temperature(),
            })
            .collect();

        // On Windows, sysinfo may return no components — fall back to WMI
        #[cfg(target_os = "windows")]
        if temperatures.is_empty() {
            temperatures = collect_wmi_temperatures();
        }

        let cpus = self.sys.cpus();
        let num_cpus = cpus.len().max(1) as f32;
        let cpu_name = cpus.first().map(|c| c.brand().to_string()).unwrap_or_default();
        let cpu_frequency_mhz = cpus.first().map(|c| c.frequency()).unwrap_or(0);
        let cpu_core_count = cpus.len();
        let process_count = self.sys.processes().values().filter(|p| p.thread_kind().is_none()).count();
        let uptime_secs = System::uptime();

        // Opt #2: Pre-build thread count map in O(n) instead of O(n²).
        let mut thread_counts: HashMap<sysinfo::Pid, u32> = HashMap::new();
        // Opt #6: Aggregate disk I/O in the same pass.
        let mut total_disk_read = 0u64;
        let mut total_disk_write = 0u64;
        for p in self.sys.processes().values() {
            let du = p.disk_usage();
            total_disk_read += du.read_bytes;
            total_disk_write += du.written_bytes;
            if let (Some(parent), Some(_thread_kind)) = (p.parent(), p.thread_kind()) {
                *thread_counts.entry(parent).or_insert(0) += 1;
            }
        }

        // Windows: get PIDs with visible windows and system PIDs for grouping
        #[cfg(target_os = "windows")]
        let windowed_pids = get_windowed_pids();
        #[cfg(target_os = "windows")]
        let all_procs: Vec<(u32, String, String)> = self.sys.processes().values()
            .filter(|p| p.thread_kind().is_none())
            .map(|p| (
                p.pid().as_u32(),
                p.name().to_string_lossy().to_string(),
                p.exe().map(|e| e.to_string_lossy().to_string()).unwrap_or_default(),
            ))
            .collect();
        #[cfg(target_os = "windows")]
        let system_pids = get_system_pids(&all_procs);

        let mut processes: Vec<ProcessInfo> = self
            .sys
            .processes()
            .values()
            .filter(|p| p.thread_kind().is_none())
            .map(|p| {
                let name = p.name().to_string_lossy().to_string();
                let pid_u32 = p.pid().as_u32();

                // Determine if this is a desktop app:
                // - Linux/macOS: match binary name against .desktop/.app list
                // - Windows: check if process owns a visible window (like Task Manager)
                let is_desktop_app = {
                    #[cfg(not(target_os = "windows"))]
                    { self.desktop_app_names.contains(&name) }
                    #[cfg(target_os = "windows")]
                    { windowed_pids.contains(&pid_u32) }
                };

                let status_char = match p.status() {
                    sysinfo::ProcessStatus::Run => 'R',
                    sysinfo::ProcessStatus::Sleep => 'S',
                    sysinfo::ProcessStatus::Zombie => 'Z',
                    sysinfo::ProcessStatus::Idle => 'I',
                    sysinfo::ProcessStatus::Stop => 'T',
                    _ => 'S',
                };
                // O(1) thread count lookup instead of O(n) inner loop
                let task_count = thread_counts.get(&p.pid()).copied().unwrap_or(0) + 1;

                // UID: used for grouping (user vs system processes)
                // - Linux: real UID from /proc
                // - Windows: 0 = user process, 1 = system process (sentinel values)
                // - macOS: 0 for all (no grouping by owner)
                let uid = {
                    #[cfg(target_os = "linux")]
                    { p.user_id().map(|u| **u).unwrap_or(0) }
                    #[cfg(target_os = "windows")]
                    { if system_pids.contains(&pid_u32) { 1u32 } else { 0u32 } }
                    #[cfg(target_os = "macos")]
                    { 0u32 }
                };

                ProcessInfo {
                    pid: pid_u32,
                    parent_pid: p.parent().map(|pid| pid.as_u32()),
                    name,
                    cmd: p.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect(),
                    cpu_usage: p.cpu_usage() / num_cpus,
                    memory_bytes: p.memory(),
                    virtual_memory_bytes: p.virtual_memory(),
                    uid,
                    is_desktop_app,
                    thread_count: task_count,
                    status: status_char,
                }
            })
            .collect();

        // Use partial sort: only find top N by CPU usage instead of sorting everything.
        let limit = self.process_limit.min(processes.len());
        if limit < processes.len() {
            processes.select_nth_unstable_by(limit, |a, b| {
                b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal)
            });
            processes.truncate(limit);
        }
        // Sort the top N for display
        processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));

        let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;

        // Load averages (Linux/macOS); zeros on unsupported platforms
        let load_avg = read_load_avg();

        Snapshot {
            timestamp: now,
            cpu_usage_per_core,
            cpu_usage_global,
            cpu_name,
            cpu_core_count,
            cpu_frequency_mhz,
            memory_used: self.sys.used_memory(),
            memory_total: self.sys.total_memory(),
            swap_used: self.sys.used_swap(),
            swap_total: self.sys.total_swap(),
            disks: self.cached_disks.clone(),
            disk_io: DiskIoSnapshot {
                read_bytes: total_disk_read,
                write_bytes: total_disk_write,
            },
            net_rx_bytes: rx,
            net_tx_bytes: tx,
            net_interfaces,
            temperatures,
            processes,
            gpu: crate::gpu::collect_gpu_info(),
            uptime_secs,
            process_count,
            sys_info: Arc::clone(&self.sys_info),
            load_avg,
        }
    }
}

/// Read system load averages (1m, 5m, 15m).
fn read_load_avg() -> [f64; 3] {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let mut avg: [libc::c_double; 3] = [0.0; 3];
        // SAFETY: getloadavg is a standard POSIX function that writes load
        // averages into the provided array. We pass a valid array of 3 elements.
        let ret = unsafe { libc::getloadavg(avg.as_mut_ptr(), 3) };
        if ret == 3 {
            return [avg[0], avg[1], avg[2]];
        }
    }
    [0.0, 0.0, 0.0]
}

/// Windows temperature sensors via native WMI — no PowerShell subprocess.
///
/// Tries multiple sources in order:
/// 1. LibreHardwareMonitor/OpenHardwareMonitor WMI namespace (richest)
/// 2. Win32_PerfFormattedData_Counters_ThermalZoneInformation (no admin, Win10+)
/// 3. MSAcpi_ThermalZoneTemperature in root\WMI namespace (requires admin)
#[cfg(target_os = "windows")]
fn collect_wmi_temperatures() -> Vec<TempInfo> {
    use std::sync::Mutex;
    use std::time::Instant;

    static CACHE: Mutex<Option<(Instant, Vec<TempInfo>)>> = Mutex::new(None);
    static REFRESH_RUNNING: Mutex<bool> = Mutex::new(false);
    const TTL_SECS: u64 = 3;

    let cached = if let Ok(guard) = CACHE.lock() {
        if let Some((ts, ref data)) = *guard {
            if ts.elapsed().as_secs() < TTL_SECS {
                return data.clone();
            }
            Some(data.clone())
        } else {
            None
        }
    } else {
        None
    };

    // Run WMI on a background thread to avoid COM apartment conflicts
    // with the UI thread (winit/iced requires STA, WMI uses MTA)
    let already_running = REFRESH_RUNNING.lock().map(|g| *g).unwrap_or(false);

    if !already_running {
        if let Ok(mut g) = REFRESH_RUNNING.lock() {
            *g = true;
        }
        std::thread::spawn(move || {
            let temps = collect_wmi_temperatures_native();
            if let Ok(mut guard) = CACHE.lock() {
                *guard = Some((Instant::now(), temps));
            }
            if let Ok(mut g) = REFRESH_RUNNING.lock() {
                *g = false;
            }
        });
    }

    cached.unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn collect_wmi_temperatures_native() -> Vec<TempInfo> {
    use std::collections::HashMap;
    use wmi::{COMLibrary, Variant, WMIConnection};

    // Use without_security() because CoInitializeSecurity is process-wide
    // and winit/iced may have already called it on the UI thread.
    let com_lib = match COMLibrary::without_security() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // Helper: extract f64 from common WMI Variant types
    fn variant_f64(v: &Variant) -> Option<f64> {
        match v {
            Variant::UI8(n) => Some(*n as f64),
            Variant::UI4(n) => Some(*n as f64),
            Variant::UI2(n) => Some(*n as f64),
            Variant::UI1(n) => Some(*n as f64),
            Variant::I4(n) => Some(*n as f64),
            Variant::I2(n) => Some(*n as f64),
            Variant::R8(n) => Some(*n),
            Variant::R4(n) => Some(*n as f64),
            Variant::String(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    fn variant_string(v: &Variant) -> Option<String> {
        match v {
            Variant::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    // 1. Try LibreHardwareMonitor / OpenHardwareMonitor (all sensors)
    for ns in &["ROOT\\LibreHardwareMonitor", "ROOT\\OpenHardwareMonitor"] {
        if let Ok(con) = WMIConnection::with_namespace_path(ns, com_lib) {
            if let Ok(results) = con.raw_query::<HashMap<String, Variant>>(
                "SELECT Name, SensorType, Value FROM Sensor WHERE SensorType = 'Temperature'",
            ) {
                let temps: Vec<TempInfo> = results
                    .iter()
                    .filter_map(|row| {
                        let label = variant_string(row.get("Name")?)?;
                        let temp_c = variant_f64(row.get("Value")?)? as f32;
                        if temp_c > 0.0 && temp_c < 150.0 {
                            Some(TempInfo { label, temp_c })
                        } else {
                            None
                        }
                    })
                    .collect();

                if !temps.is_empty() {
                    return temps;
                }
            }
        }
    }

    // 2. Thermal zone performance counters (Kelvin → Celsius, no admin)
    //    Use raw_query for maximum compatibility across Windows versions.
    if let Ok(con) = WMIConnection::new(com_lib) {
        if let Ok(results) = con.raw_query::<HashMap<String, Variant>>(
            "SELECT Name, Temperature FROM Win32_PerfFormattedData_Counters_ThermalZoneInformation",
        ) {
            let temps: Vec<TempInfo> = results
                .iter()
                .filter_map(|row| {
                    let kelvin = variant_f64(row.get("Temperature")?)? as u32;
                    if kelvin == 0 {
                        return None;
                    }
                    let celsius = kelvin as f32 - 273.15;
                    if celsius > 0.0 && celsius < 150.0 {
                        let label = row
                            .get("Name")
                            .and_then(variant_string)
                            .unwrap_or_else(|| "Thermal Zone".into());
                        Some(TempInfo {
                            label,
                            temp_c: celsius,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            if !temps.is_empty() {
                return temps;
            }
        }
    }

    // 3. ACPI thermal zones (requires admin, ROOT\WMI namespace)
    if let Ok(con) = WMIConnection::with_namespace_path("ROOT\\WMI", com_lib) {
        if let Ok(results) = con.raw_query::<HashMap<String, Variant>>(
            "SELECT InstanceName, CurrentTemperature FROM MSAcpi_ThermalZoneTemperature",
        ) {
            let temps: Vec<TempInfo> = results
                .iter()
                .filter_map(|row| {
                    let raw = variant_f64(row.get("CurrentTemperature")?)?;
                    let celsius = (raw - 2732.0) / 10.0;
                    if celsius > 0.0 && celsius < 150.0 {
                        let label = row
                            .get("InstanceName")
                            .and_then(variant_string)
                            .unwrap_or_else(|| "ACPI Thermal Zone".into());
                        Some(TempInfo {
                            label,
                            temp_c: celsius as f32,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            if !temps.is_empty() {
                return temps;
            }
        }
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_app_names_loads() {
        // Just ensure it doesn't panic
        let names = load_desktop_app_names();
        // On CI with no desktop, this may be empty — that's fine
        let _ = names.len();
    }

    #[test]
    fn test_desktop_cache_roundtrip() {
        let mut names = HashSet::new();
        names.insert("firefox".into());
        names.insert("code".into());
        save_desktop_cache(&names);
        if let Some(loaded) = load_desktop_cache() {
            assert!(loaded.contains("firefox"));
            assert!(loaded.contains("code"));
        }
    }

    #[test]
    fn test_process_limit() {
        // Verify the limit field is stored correctly
        let collector = Collector::with_process_limit(50);
        assert_eq!(collector.process_limit, 50);
    }
}
