//! The update loop.

use iced::keyboard;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::icons::*;
use crate::message::*;
use crate::metrics::{LivePoint, Snapshot};
use crate::preferences::{FONT_SCALES, TEXT_SCALES};
use crate::state::*;
use crate::theme::{AccentChoice, ThemeChoice};

impl Digger {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tick => {
                let snap = Arc::new(self.collector.collect());
                let now_ts = snap.timestamp;

                // Opt #10 + #11: Batch SQLite inserts in a single transaction — flush every 5 seconds.
                self.pending_snapshots.push(Arc::clone(&snap));
                if now_ts - self.last_db_flush >= 5.0 || self.last_db_flush == 0.0 {
                    let batch = std::mem::take(&mut self.pending_snapshots);
                    let refs: Vec<&Snapshot> = batch.iter().map(|a| a.as_ref()).collect();
                    self.history.record_batch(&refs);
                    self.last_db_flush = now_ts;
                }

                let mem_pct = if snap.memory_total > 0 {
                    snap.memory_used as f32 / snap.memory_total as f32 * 100.0
                } else {
                    0.0
                };
                self.live_buffer.push(LivePoint {
                    cpu: snap.cpu_usage_global,
                    mem_pct,
                    net_rx: snap.net_rx_bytes,
                    net_tx: snap.net_tx_bytes,
                    disk_read: snap.disk_io.read_bytes,
                    disk_write: snap.disk_io.write_bytes,
                });

                // Check alert thresholds
                if snap.cpu_usage_global >= self.cpu_alert_threshold {
                    self.status_message = Some(format!(
                        "{ICON_WARNING} CPU usage at {:.0}% (threshold: {:.0}%)",
                        snap.cpu_usage_global, self.cpu_alert_threshold
                    ));
                } else if mem_pct >= self.mem_alert_threshold {
                    self.status_message = Some(format!(
                        "{ICON_WARNING} Memory usage at {:.0}% (threshold: {:.0}%)",
                        mem_pct, self.mem_alert_threshold
                    ));
                } else {
                    if let Some(err) = &self.history.last_error {
                        self.status_message = Some(format!("{ICON_WARNING} {err}"));
                    } else {
                        self.status_message = None;
                    }
                }

                // ─── Anomaly detection & event logging (opt #5: bounded VecDeque) ───
                let now_str: Arc<str> =
                    Arc::from(chrono::Local::now().format("%H:%M:%S").to_string());

                // Helper closure: push to bounded event log
                let push_event = |log: &mut VecDeque<LogEvent>, event: LogEvent| {
                    if log.len() >= EVENT_LOG_MAX {
                        log.pop_front();
                    }
                    log.push_back(event);
                };

                // CPU spike: jumped more than 40% in one tick
                let cpu_delta = snap.cpu_usage_global - self.prev_cpu;
                if cpu_delta > 40.0 {
                    let msg = format!(
                        "CPU spike: {:.0}% → {:.0}% (+{:.0}%)",
                        self.prev_cpu, snap.cpu_usage_global, cpu_delta
                    );
                    send_notification("Digger: CPU Spike", &msg);
                    push_event(
                        &mut self.event_log,
                        LogEvent {
                            timestamp: Arc::clone(&now_str),
                            icon: ICON_BOLT,
                            message: msg,
                            severity: EventSeverity::Warning,
                        },
                    );
                }

                // Memory monotonic rise detection
                if mem_pct > self.prev_mem_pct + 2.0 && mem_pct > 80.0 {
                    push_event(
                        &mut self.event_log,
                        LogEvent {
                            timestamp: Arc::clone(&now_str),
                            icon: ICON_WARNING,
                            message: format!(
                                "Memory rising: {:.1}% → {:.1}%",
                                self.prev_mem_pct, mem_pct
                            ),
                            severity: EventSeverity::Warning,
                        },
                    );
                }

                // Critical thresholds
                if snap.cpu_usage_global >= self.cpu_alert_threshold
                    && self.prev_cpu < self.cpu_alert_threshold
                {
                    let msg = format!(
                        "CPU exceeded threshold: {:.0}% >= {:.0}%",
                        snap.cpu_usage_global, self.cpu_alert_threshold
                    );
                    send_notification("Digger: CPU Alert", &msg);
                    push_event(
                        &mut self.event_log,
                        LogEvent {
                            timestamp: Arc::clone(&now_str),
                            icon: ICON_WARNING,
                            message: msg,
                            severity: EventSeverity::Critical,
                        },
                    );
                }
                if mem_pct >= self.mem_alert_threshold
                    && self.prev_mem_pct < self.mem_alert_threshold
                {
                    let msg = format!(
                        "Memory exceeded threshold: {:.0}% >= {:.0}%",
                        mem_pct, self.mem_alert_threshold
                    );
                    send_notification("Digger: Memory Alert", &msg);
                    push_event(
                        &mut self.event_log,
                        LogEvent {
                            timestamp: Arc::clone(&now_str),
                            icon: ICON_WARNING,
                            message: msg,
                            severity: EventSeverity::Critical,
                        },
                    );
                }

                // Recovery events
                if snap.cpu_usage_global < self.cpu_alert_threshold
                    && self.prev_cpu >= self.cpu_alert_threshold
                {
                    push_event(
                        &mut self.event_log,
                        LogEvent {
                            timestamp: Arc::clone(&now_str),
                            icon: ICON_CHECK,
                            message: format!(
                                "CPU recovered: {:.0}% < {:.0}% threshold",
                                snap.cpu_usage_global, self.cpu_alert_threshold
                            ),
                            severity: EventSeverity::Info,
                        },
                    );
                }
                if mem_pct < self.mem_alert_threshold
                    && self.prev_mem_pct >= self.mem_alert_threshold
                {
                    push_event(
                        &mut self.event_log,
                        LogEvent {
                            timestamp: Arc::clone(&now_str),
                            icon: ICON_CHECK,
                            message: format!(
                                "Memory recovered: {:.0}% < {:.0}% threshold",
                                mem_pct, self.mem_alert_threshold
                            ),
                            severity: EventSeverity::Info,
                        },
                    );
                }

                // Temperature alerts
                let max_temp = snap
                    .temperatures
                    .iter()
                    .map(|t| t.temp_c)
                    .fold(0.0_f32, f32::max);
                if max_temp > 85.0 {
                    let temp_msg = format!("High temperature: {:.0}°C", max_temp);
                    send_notification("Digger: Temperature Alert", &temp_msg);
                    push_event(
                        &mut self.event_log,
                        LogEvent {
                            timestamp: now_str,
                            icon: ICON_TEMP,
                            message: format!("High temperature: {:.0}°C", max_temp),
                            severity: EventSeverity::Critical,
                        },
                    );
                }

                self.prev_cpu = snap.cpu_usage_global;
                self.prev_mem_pct = mem_pct;

                // ─── Heartbeat BPM ───
                self.health_score = compute_heartbeat_bpm(snap.cpu_usage_global, mem_pct);

                self.current = Some(snap);

                // Opt #7: Throttle History tab SQL reload to every 10s.
                if self.tab == Tab::History
                    && (now_ts - self.history_last_reload >= HISTORY_RELOAD_INTERVAL_SECS)
                {
                    self.history_last_reload = now_ts;
                    let range = HISTORY_RANGES[self.history_range_idx].0;
                    self.history_points = self.history.load_last_n_seconds_downsampled(range, 600);
                }
            }
            Message::AnimTick => {
                // Opt #4: Skip animation work when values have converged.
                let mut needs_anim = false;

                if let Some(snap) = &self.current {
                    let target_cpu = snap.cpu_usage_global;
                    let target_mem = if snap.memory_total > 0 {
                        snap.memory_used as f32 / snap.memory_total as f32 * 100.0
                    } else {
                        0.0
                    };

                    // Reduced motion snaps instead of tweening. The gauges still
                    // have to track the data — a frozen gauge is a broken
                    // readout, not a calmer one — so what stops is the travel
                    // between two values, not the values.
                    if self.reduced_motion {
                        self.anim_cpu = target_cpu;
                        self.anim_mem_pct = target_mem;
                        self.anim_cores = snap.cpu_usage_per_core.clone();
                        return;
                    }

                    // Only tween if not converged (threshold: 0.1%)
                    if (target_cpu - self.anim_cpu).abs() > 0.1 {
                        self.anim_cpu += (target_cpu - self.anim_cpu) * TWEEN_SPEED;
                        needs_anim = true;
                    } else {
                        self.anim_cpu = target_cpu;
                    }
                    if (target_mem - self.anim_mem_pct).abs() > 0.1 {
                        self.anim_mem_pct += (target_mem - self.anim_mem_pct) * TWEEN_SPEED;
                        needs_anim = true;
                    } else {
                        self.anim_mem_pct = target_mem;
                    }

                    // Per-core tweening
                    let cores = &snap.cpu_usage_per_core;
                    if self.anim_cores.len() != cores.len() {
                        self.anim_cores = cores.clone();
                        needs_anim = true;
                    } else {
                        for (anim, &target) in self.anim_cores.iter_mut().zip(cores.iter()) {
                            if (target - *anim).abs() > 0.1 {
                                *anim += (target - *anim) * TWEEN_SPEED;
                                needs_anim = true;
                            } else {
                                *anim = target;
                            }
                        }
                    }
                }

                // Pulse & heartbeat always advance (cheap arithmetic)
                self.pulse_phase += PULSE_SPEED;
                if self.pulse_phase > std::f32::consts::TAU {
                    self.pulse_phase -= std::f32::consts::TAU;
                }

                let dt = ANIM_TICK_MS as f32 / 1000.0;
                let freq = self.health_score / 60.0;
                self.heart_phase += std::f32::consts::TAU * freq * dt;
                if self.heart_phase > std::f32::consts::TAU {
                    self.heart_phase -= std::f32::consts::TAU;
                }

                let _ = needs_anim; // reserved for future: could skip redraw when false
            }
            Message::TabSelected(tab) => {
                self.prev_tab = self.tab;
                self.tab = tab;
                if tab == Tab::History {
                    // Force immediate reload on tab switch
                    self.history_last_reload = 0.0;
                    let range = HISTORY_RANGES[self.history_range_idx].0;
                    self.history_points = self.history.load_last_n_seconds_downsampled(range, 600);
                }
            }
            Message::OverviewSection(s) => {
                self.overview_panel = s;
            }
            Message::ProcessFilterChanged(f) => self.process_filter = f,
            Message::ToggleGrouped => {
                self.process_grouped = !self.process_grouped;
                self.save_prefs();
            }
            Message::SortBy(col) => {
                if self.process_sort == col {
                    self.process_sort_asc = !self.process_sort_asc;
                } else {
                    self.process_sort = col;
                    self.process_sort_asc = false;
                }
                self.save_prefs();
            }
            Message::HistoryRangeSelected(idx) => {
                self.history_range_idx = idx;
                let range = HISTORY_RANGES[idx].0;
                self.history_points = self.history.load_last_n_seconds_downsampled(range, 600);
            }
            Message::ToggleSettings => {
                self.prev_show_settings = self.show_settings;
                self.show_settings = !self.show_settings;
            }
            Message::SettingsPanelSelected(p) => {
                self.settings_panel = p;
            }
            Message::SetRefreshInterval(secs) => {
                self.refresh_interval_secs = secs;
                self.save_prefs();
            }
            Message::ToggleTempUnit => {
                self.temp_celsius = !self.temp_celsius;
                self.save_prefs();
            }
            Message::ToggleSection(section) => {
                if !self.collapsed_sections.remove(&section) {
                    self.collapsed_sections.insert(section);
                }
            }
            Message::SetTheme { family, variant } => {
                self.theme_variant = ThemeChoice { family, variant };
                self.refresh_appearance();
                self.save_prefs();
            }
            Message::SetAccent(accent) => {
                self.accent_color = AccentChoice(accent);
                self.refresh_appearance();
                self.save_prefs();
            }
            Message::ToggleDyslexicFont => {
                self.use_dyslexic_font = !self.use_dyslexic_font;
                self.refresh_appearance();
                self.save_prefs();
            }
            Message::ToggleHighContrast => {
                self.high_contrast = !self.high_contrast;
                self.refresh_appearance();
                self.save_prefs();
            }
            Message::ToggleReducedMotion => {
                self.reduced_motion = !self.reduced_motion;
                // Leave the animation phases where they are rather than
                // resetting: a pulse frozen mid-beat is still a static icon.
                self.save_prefs();
            }
            Message::SetFontScale(idx) => {
                self.font_scale = FONT_SCALES.get(idx).copied().unwrap_or(1.0);
                self.refresh_appearance();
                self.save_prefs();
            }
            Message::SetTextScale(idx) => {
                self.text_scale = TEXT_SCALES.get(idx).copied().unwrap_or(1.0);
                self.refresh_appearance();
                self.save_prefs();
            }
            Message::ExportCsv => {
                let range = HISTORY_RANGES[self.history_range_idx].0;
                let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
                let csv = self.history.export_csv(now - range, now);
                if let Some(dir) = dirs::download_dir().or_else(dirs::home_dir) {
                    let path = dir.join("digger_export.csv");
                    match std::fs::write(&path, &csv) {
                        Ok(_) => {
                            self.status_message = Some(format!("Exported to {}", path.display()))
                        }
                        Err(e) => self.status_message = Some(format!("Export failed: {e}")),
                    }
                }
            }
            Message::ExportJson => {
                let range = HISTORY_RANGES[self.history_range_idx].0;
                let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
                let json = self.history.export_json(now - range, now);
                if let Some(dir) = dirs::download_dir().or_else(dirs::home_dir) {
                    let path = dir.join("digger_export.json");
                    match std::fs::write(&path, &json) {
                        Ok(_) => {
                            self.status_message = Some(format!("Exported to {}", path.display()))
                        }
                        Err(e) => self.status_message = Some(format!("Export failed: {e}")),
                    }
                }
            }
            Message::KillProcess(pid) => {
                // SAFETY: Sending SIGTERM to a process is safe when the PID
                // is a valid process ID obtained from sysinfo. The libc::kill
                // function is a standard POSIX syscall that sends a signal to
                // a process. We use SIGTERM (graceful termination) rather than
                // SIGKILL to allow the process to clean up.
                #[cfg(unix)]
                {
                    let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
                    if result == 0 {
                        self.status_message = Some(format!("Sent SIGTERM to PID {pid}"));
                    } else {
                        self.status_message =
                            Some(format!("Failed to kill PID {pid} (permission denied?)"));
                    }
                }
                #[cfg(windows)]
                {
                    use std::ptr;
                    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LUID};
                    use windows_sys::Win32::Security::{
                        AdjustTokenPrivileges, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
                        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
                    };
                    use windows_sys::Win32::System::Threading::{
                        GetCurrentProcess, OpenProcess, OpenProcessToken, TerminateProcess,
                        PROCESS_TERMINATE,
                    };

                    // Try to enable SeDebugPrivilege so we can kill
                    // processes owned by other accounts (services, SYSTEM).
                    // This succeeds only when Digger is running as admin.
                    unsafe {
                        let mut token: HANDLE = ptr::null_mut();
                        if OpenProcessToken(
                            GetCurrentProcess(),
                            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                            &mut token,
                        ) != 0
                        {
                            let priv_name: Vec<u16> = "SeDebugPrivilege\0".encode_utf16().collect();
                            let mut luid = LUID {
                                LowPart: 0,
                                HighPart: 0,
                            };
                            if LookupPrivilegeValueW(ptr::null(), priv_name.as_ptr(), &mut luid)
                                != 0
                            {
                                let mut tp = TOKEN_PRIVILEGES {
                                    PrivilegeCount: 1,
                                    Privileges: [
                                        windows_sys::Win32::Security::LUID_AND_ATTRIBUTES {
                                            Luid: luid,
                                            Attributes: SE_PRIVILEGE_ENABLED,
                                        },
                                    ],
                                };
                                AdjustTokenPrivileges(
                                    token,
                                    0,
                                    &mut tp,
                                    0,
                                    ptr::null_mut(),
                                    ptr::null_mut(),
                                );
                            }
                            CloseHandle(token);
                        }

                        let handle: HANDLE = OpenProcess(PROCESS_TERMINATE, 0, pid);
                        if !handle.is_null() {
                            if TerminateProcess(handle, 1) != 0 {
                                self.status_message = Some(format!("Terminated PID {pid}"));
                            } else {
                                self.status_message = Some(format!(
                                    "Failed to kill PID {pid} (access denied — try running as administrator)"
                                ));
                            }
                            CloseHandle(handle);
                        } else {
                            self.status_message = Some(format!(
                                "Failed to open PID {pid} (access denied — try running as administrator)"
                            ));
                        }
                    }
                }
                #[cfg(not(any(unix, windows)))]
                {
                    self.status_message =
                        Some("Process kill not supported on this platform".into());
                }
            }
            Message::SetCpuAlertThreshold(v) => {
                self.cpu_alert_threshold = v;
                self.save_prefs();
            }
            Message::SetMemAlertThreshold(v) => {
                self.mem_alert_threshold = v;
                self.save_prefs();
            }
            Message::SetLanguage(lang) => {
                self.language = lang;
                self.refresh_appearance();
                self.rebuild_cached_strings();
                self.save_prefs();
            }
            Message::KeyPressed(key, modifiers) => {
                use keyboard::key::Named;
                match key {
                    // Tab navigation: 1-4 for tabs
                    keyboard::Key::Character(ref c) if !self.show_settings => {
                        match c.as_str() {
                            "1" => {
                                self.prev_tab = self.tab;
                                self.tab = Tab::Overview;
                            }
                            "2" => {
                                self.prev_tab = self.tab;
                                self.tab = Tab::Processes;
                            }
                            "3" => {
                                self.prev_tab = self.tab;
                                self.tab = Tab::History;
                            }
                            "4" => {
                                self.prev_tab = self.tab;
                                self.tab = Tab::EventLog;
                            }
                            "s" | "," => {
                                self.prev_show_settings = self.show_settings;
                                self.show_settings = !self.show_settings;
                            }
                            "g" if self.tab == Tab::Processes => {
                                self.process_grouped = !self.process_grouped;
                                self.save_prefs();
                            }
                            "/" if self.tab == Tab::Processes => {
                                // Focus on search (will be handled by the text input focus)
                            }
                            _ => {}
                        }
                    }
                    keyboard::Key::Named(Named::Escape) => {
                        if self.show_settings {
                            self.show_settings = false;
                        }
                    }
                    keyboard::Key::Named(Named::Tab)
                        if !modifiers.shift() && !self.show_settings =>
                    {
                        // Cycle tabs forward
                        self.prev_tab = self.tab;
                        self.tab = match self.tab {
                            Tab::Overview => Tab::Processes,
                            Tab::Processes => Tab::History,
                            Tab::History => Tab::EventLog,
                            Tab::EventLog => Tab::Overview,
                        };
                    }
                    keyboard::Key::Named(Named::Tab)
                        if modifiers.shift() && !self.show_settings =>
                    {
                        // Cycle tabs backward
                        self.prev_tab = self.tab;
                        self.tab = match self.tab {
                            Tab::Overview => Tab::EventLog,
                            Tab::Processes => Tab::Overview,
                            Tab::History => Tab::Processes,
                            Tab::EventLog => Tab::History,
                        };
                    }
                    _ => {}
                }
            }
        }
    }
}
