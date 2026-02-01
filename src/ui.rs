use iced::widget::canvas::Canvas;
use iced::widget::{
    button, column, container, progress_bar, row, scrollable, text, text_input,
    tooltip, Column, Row, Space,
};
use iced::keyboard;
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Subscription, Theme, Vector};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use crate::chart::{ChartColors, LineChart};
use crate::gauge::{GaugeColors, RadialGauge, Sparkline};
use crate::history::History;
use crate::i18n::{Language, Strings};
use crate::icons::*;
use crate::metrics::{Collector, LivePoint, Snapshot};
use crate::preferences::Preferences;
use crate::ringbuf::RingBuffer;
use crate::theme::{AccentColor, Palette, ThemeVariant, build_palette};
use crate::{NERD_FONT_MONO, SARASA_FONT, DEJAVU_FONT, NOTO_SANS_FONT};

/// Returns the best available monospace font for a given language's script.
fn font_for_lang(lang: Language) -> iced::Font {
    match lang {
        // CJK → Sarasa
        Language::ZhCn | Language::ZhTw | Language::Ja | Language::Ko => SARASA_FONT,
        // Arabic / Persian → DejaVu
        Language::Ar | Language::Fa => DEJAVU_FONT,
        // Devanagari (Hindi, Marathi) → NotoSans NF
        Language::Hi | Language::Mr => NOTO_SANS_FONT,
        // Everything else (Latin, Cyrillic, and scripts without coverage) → Iosevka
        _ => NERD_FONT_MONO,
    }
}

/// Returns true if the language's native script can be rendered by an embedded font.
fn has_native_font(lang: Language) -> bool {
    !matches!(lang,
        Language::He | Language::Bn | Language::Pa | Language::Ta
        | Language::Te | Language::Th | Language::Am
    )
}

/// Detect if the system prefers dark mode.
fn system_prefers_dark() -> bool {
    // Check common environment variables on Linux/macOS
    if let Ok(gtk_theme) = std::env::var("GTK_THEME") {
        if gtk_theme.to_lowercase().contains("dark") {
            return true;
        }
    }
    if let Ok(color_scheme) = std::env::var("COLORFGBG") {
        // COLORFGBG format: "fg;bg" - if bg < 8, it's a dark terminal
        if let Some(bg) = color_scheme.split(';').next_back() {
            if let Ok(n) = bg.parse::<u32>() {
                return n < 8;
            }
        }
    }
    // Check freedesktop dark mode preference
    if let Ok(val) = std::env::var("XDG_CURRENT_DESKTOP") {
        // Default to dark for most modern desktops
        let _ = val;
    }
    // Default: assume dark mode
    true
}

/// Send a desktop notification (non-blocking, best-effort).
fn send_notification(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .appname("Digger")
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();
}

// ─── ANIMATION CONSTANTS ────────────────────────────────────────
const ANIM_TICK_MS: u64 = 33; // ~30fps for animations
const TWEEN_SPEED: f32 = 0.12; // lerp factor per animation tick
const FADE_SPEED: f32 = 0.08; // fade-in speed per tick
const PULSE_SPEED: f32 = 0.05; // pulse cycle speed

const EVENT_LOG_MAX: usize = 100;
const HISTORY_RELOAD_INTERVAL_SECS: f64 = 10.0;

const HISTORY_RANGES: &[(f64, &str)] = &[
    (60.0, "1m"),
    (300.0, "5m"),
    (900.0, "15m"),
    (3600.0, "1h"),
    (86400.0, "24h"),
];

const REFRESH_OPTIONS: &[u64] = &[1, 2, 5];

// ─── EVENT LOG ──────────────────────────────────────────────────

/// An event logged by the anomaly detection system.
#[derive(Clone, Debug)]
struct LogEvent {
    timestamp: Arc<str>,
    icon: &'static str,
    message: String,
    severity: EventSeverity,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum EventSeverity {
    Info,
    Warning,
    Critical,
}

/// Compute a heartbeat BPM (80–160) based on system load.
/// Resting heart rate is 80 BPM; CPU and memory usage increase it.
fn compute_heartbeat_bpm(cpu: f32, mem_pct: f32) -> f32 {
    (80.0 + cpu * 0.45 + mem_pct * 0.35).clamp(80.0, 160.0)
}

/// Dynamic saturation: low usage → desaturated, high usage → vivid color
fn dynamic_color(base: Color, intensity: f32) -> Color {
    // intensity: 0.0 to 1.0
    let t = intensity.clamp(0.0, 1.0);
    let gray = 0.5;
    Color::from_rgb(
        gray + (base.r - gray) * (0.3 + 0.7 * t),
        gray + (base.g - gray) * (0.3 + 0.7 * t),
        gray + (base.b - gray) * (0.3 + 0.7 * t),
    )
}

// ─── MESSAGE & ENUMS ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    AnimTick,
    TabSelected(Tab),
    OverviewSection(OverviewPanel),
    ProcessFilterChanged(String),
    SortBy(ProcessSort),
    ToggleGrouped,
    HistoryRangeSelected(usize),
    // Settings
    ToggleSettings,
    SettingsPanelSelected(SettingsPanel),
    SetRefreshInterval(u64),
    ToggleTempUnit,
    ToggleSection(SettingsSection),
    SetTheme(ThemeVariant),
    SetAccent(AccentColor),
    ToggleDyslexicFont,
    // Export
    ExportCsv,
    ExportJson,
    // Process management
    KillProcess(u32),
    // Alerts
    SetCpuAlertThreshold(f32),
    SetMemAlertThreshold(f32),
    // Language
    SetLanguage(Language),
    // Keyboard
    KeyPressed(keyboard::Key, keyboard::Modifiers),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Processes,
    History,
    EventLog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverviewPanel {
    Cpu,
    Memory,
    Network,
    Disk,
    Temperature,
    Gpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSort {
    Pid,
    Name,
    Cpu,
    Memory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPanel {
    General,
    Appearance,
    Accessibility,
    Language,
    About,
}

/// Identifiers for collapsible settings sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsSection {
    Monitoring,
    Display,
    Data,
    Alerts,
    // Appearance
    Theme,
    Accent,
    // Accessibility
    Fonts,
    // About
    Version,
    FontInfo,
    SystemInfo,
}

// ─── APP STATE ──────────────────────────────────────────────────

pub struct Digger {
    collector: Collector,
    history: History,
    current: Option<Arc<Snapshot>>,
    live_buffer: RingBuffer<LivePoint>,
    live_max: usize,
    tab: Tab,
    overview_panel: OverviewPanel,
    process_filter: String,
    process_sort: ProcessSort,
    process_sort_asc: bool,
    process_grouped: bool,
    history_range_idx: usize,
    history_points: Vec<crate::history::HistoryPoint>,
    // Settings
    show_settings: bool,
    settings_panel: SettingsPanel,
    refresh_interval_secs: u64,
    temp_celsius: bool,
    collapsed_sections: HashSet<SettingsSection>,
    // Theme
    theme_variant: ThemeVariant,
    accent_color: AccentColor,
    pal: Palette,
    // Language
    language: Language,
    /// Monospace font for the current language's script.
    ui_mono: iced::Font,
    // New configurable fields
    process_limit: usize,
    use_dyslexic_font: bool,
    retention_hours: u64,
    cpu_alert_threshold: f32,
    mem_alert_threshold: f32,
    // Status message for user feedback
    status_message: Option<String>,
    // ─── Health & Events ───
    /// Health score 0–100 (higher is better)
    health_score: f32,
    /// Recent event log entries (bounded VecDeque, opt #5)
    event_log: VecDeque<LogEvent>,
    /// Previous CPU reading for spike detection
    prev_cpu: f32,
    /// Previous memory % for leak detection
    prev_mem_pct: f32,
    // ─── Animation state ───
    /// Smoothly interpolated CPU usage for display
    anim_cpu: f32,
    /// Smoothly interpolated memory percentage for display
    anim_mem_pct: f32,
    /// Smoothly interpolated per-core CPU values
    anim_cores: Vec<f32>,
    /// Page fade-in opacity (0.0 → 1.0)
    page_opacity: f32,
    /// Pulse phase for critical alerts (0.0 → 2*PI cycle)
    pulse_phase: f32,
    /// Heart beat phase (0.0 → 2*PI), advances based on BPM
    heart_phase: f32,
    /// Previous tab (to detect page transitions)
    prev_tab: Tab,
    /// Previous settings visibility
    prev_show_settings: bool,
    /// Opt #7: Timestamp of last history reload to throttle SQL queries.
    history_last_reload: f64,
    /// Opt #10: Pending snapshots for batched SQLite inserts.
    pending_snapshots: Vec<Arc<Snapshot>>,
    /// Opt #10: Timestamp of last DB flush.
    last_db_flush: f64,
    // ─── Cached UI strings (avoid format! every frame) ───
    cached_tab_overview: String,
    cached_tab_processes: String,
    cached_tab_history: String,
    cached_tab_events: String,
    cached_digger_label: String,
    cached_digger_label_settings: String,
    /// Cached theme preview palettes (rebuilt only when accent color changes).
    cached_theme_previews: Vec<(ThemeVariant, Palette)>,
    cached_theme_accent: AccentColor,
}

impl Digger {
    pub fn new() -> Self {
        let prefs = Preferences::load();
        let live_max = prefs.live_buffer_size;
        let mut collector = Collector::with_process_limit(prefs.process_limit);
        let mut history = History::open();

        // Collect immediately so the UI never shows "Collecting data..."
        let snap = Arc::new(collector.collect());
        history.record(&snap);
        let mem_pct = if snap.memory_total > 0 {
            snap.memory_used as f32 / snap.memory_total as f32 * 100.0
        } else {
            0.0
        };
        let mut live_buffer = RingBuffer::new(live_max);
        live_buffer.push(LivePoint {
            cpu: snap.cpu_usage_global,
            mem_pct,
            net_rx: snap.net_rx_bytes,
            net_tx: snap.net_tx_bytes,
            disk_read: snap.disk_io.read_bytes,
            disk_write: snap.disk_io.write_bytes,
        });

        Self {
            collector,
            history,
            current: Some(Arc::clone(&snap)),
            live_buffer,
            live_max,
            tab: Tab::Overview,
            overview_panel: OverviewPanel::Cpu,
            process_filter: String::new(),
            process_sort: match prefs.process_sort.as_str() {
                "pid" => ProcessSort::Pid,
                "name" => ProcessSort::Name,
                "memory" => ProcessSort::Memory,
                _ => ProcessSort::Cpu,
            },
            process_sort_asc: prefs.process_sort_asc,
            process_grouped: prefs.process_grouped,
            history_range_idx: 0,
            history_points: Vec::new(),
            show_settings: false,
            settings_panel: SettingsPanel::General,
            refresh_interval_secs: prefs.refresh_interval_secs,
            temp_celsius: prefs.temp_celsius,
            collapsed_sections: HashSet::new(),
            theme_variant: if prefs.auto_theme {
                if system_prefers_dark() { ThemeVariant::CatppuccinMocha } else { ThemeVariant::CatppuccinLatte }
            } else {
                prefs.theme
            },
            accent_color: prefs.accent,
            language: prefs.language,
            ui_mono: font_for_lang(prefs.language),
            pal: build_palette(
                if prefs.auto_theme {
                    if system_prefers_dark() { ThemeVariant::CatppuccinMocha } else { ThemeVariant::CatppuccinLatte }
                } else {
                    prefs.theme
                },
                prefs.accent,
            ),
            process_limit: prefs.process_limit,
            use_dyslexic_font: prefs.use_dyslexic_font,
            retention_hours: prefs.retention_hours,
            cpu_alert_threshold: prefs.cpu_alert_threshold,
            mem_alert_threshold: prefs.mem_alert_threshold,
            status_message: None,
            // Health & events
            health_score: 100.0,
            event_log: VecDeque::with_capacity(EVENT_LOG_MAX),
            prev_cpu: snap.cpu_usage_global,
            prev_mem_pct: mem_pct,
            // Animation state
            anim_cpu: snap.cpu_usage_global,
            anim_mem_pct: mem_pct,
            anim_cores: snap.cpu_usage_per_core.clone(),
            page_opacity: 1.0,
            pulse_phase: 0.0,
            heart_phase: 0.0,
            prev_tab: Tab::Overview,
            prev_show_settings: false,
            history_last_reload: 0.0,
            pending_snapshots: Vec::new(),
            last_db_flush: 0.0,
            // Cached UI strings
            cached_tab_overview: format!("{ICON_OVERVIEW}  {}", prefs.language.strings().tab_overview),
            cached_tab_processes: format!("{ICON_PROCESSES}  {}", prefs.language.strings().tab_processes),
            cached_tab_history: format!("{ICON_HISTORY}  {}", prefs.language.strings().tab_history),
            cached_tab_events: format!("{ICON_LOG}  {}", prefs.language.strings().tab_events),
            cached_digger_label: format!("{ICON_DIGGER} Digger"),
            cached_digger_label_settings: format!("{ICON_DIGGER} Digger  {ICON_CLOSE}"),
            cached_theme_previews: Self::build_theme_previews(prefs.accent),
            cached_theme_accent: prefs.accent,
        }
    }

    /// Get the current translation strings.
    fn t(&self) -> &'static Strings {
        self.language.strings()
    }

    /// Rebuild cached tab strings when language changes.
    fn rebuild_cached_strings(&mut self) {
        let t = self.language.strings();
        self.cached_tab_overview = format!("{ICON_OVERVIEW}  {}", t.tab_overview);
        self.cached_tab_processes = format!("{ICON_PROCESSES}  {}", t.tab_processes);
        self.cached_tab_history = format!("{ICON_HISTORY}  {}", t.tab_history);
        self.cached_tab_events = format!("{ICON_LOG}  {}", t.tab_events);
    }

    fn build_theme_previews(accent: AccentColor) -> Vec<(ThemeVariant, Palette)> {
        use ThemeVariant::*;
        let variants = [
            CatppuccinLatte, CatppuccinFrappe, CatppuccinMacchiato, CatppuccinMocha,
            GruvboxLight, GruvboxDark,
            EverblushLight, EverblushDark,
            KanagawaLight, KanagawaDark, KanagawaDragon,
        ];
        variants.iter().map(|&v| (v, build_palette(v, accent))).collect()
    }

    pub fn title(&self) -> String {
        String::from("Digger")
    }

    pub fn theme(&self) -> Theme {
        if self.theme_variant.is_light() { Theme::Light } else { Theme::Dark }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let data_tick = iced::time::every(Duration::from_secs(self.refresh_interval_secs))
            .map(|_| Message::Tick);
        let anim_tick = iced::time::every(Duration::from_millis(ANIM_TICK_MS))
            .map(|_| Message::AnimTick);
        let keys = keyboard::on_key_press(|key, modifiers| {
            Some(Message::KeyPressed(key, modifiers))
        });
        Subscription::batch([data_tick, anim_tick, keys])
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tick => {
                let snap = Arc::new(self.collector.collect());
                let now_ts = snap.timestamp;

                // Opt #10 + #11: Batch SQLite inserts in a single transaction — flush every 5 seconds.
                self.pending_snapshots.push(Arc::clone(&snap));
                if now_ts - self.last_db_flush >= 5.0 || self.last_db_flush == 0.0 {
                    let batch: Vec<Arc<Snapshot>> = self.pending_snapshots.drain(..).collect();
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
                let now_str: Arc<str> = Arc::from(chrono::Local::now().format("%H:%M:%S").to_string());

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
                    let msg = format!("CPU spike: {:.0}% → {:.0}% (+{:.0}%)", self.prev_cpu, snap.cpu_usage_global, cpu_delta);
                    send_notification("Digger: CPU Spike", &msg);
                    push_event(&mut self.event_log, LogEvent {
                        timestamp: Arc::clone(&now_str),
                        icon: ICON_BOLT,
                        message: msg,
                        severity: EventSeverity::Warning,
                    });
                }

                // Memory monotonic rise detection
                if mem_pct > self.prev_mem_pct + 2.0 && mem_pct > 80.0 {
                    push_event(&mut self.event_log, LogEvent {
                        timestamp: Arc::clone(&now_str),
                        icon: ICON_WARNING,
                        message: format!("Memory rising: {:.1}% → {:.1}%", self.prev_mem_pct, mem_pct),
                        severity: EventSeverity::Warning,
                    });
                }

                // Critical thresholds
                if snap.cpu_usage_global >= self.cpu_alert_threshold && self.prev_cpu < self.cpu_alert_threshold {
                    let msg = format!("CPU exceeded threshold: {:.0}% >= {:.0}%", snap.cpu_usage_global, self.cpu_alert_threshold);
                    send_notification("Digger: CPU Alert", &msg);
                    push_event(&mut self.event_log, LogEvent {
                        timestamp: Arc::clone(&now_str),
                        icon: ICON_WARNING,
                        message: msg,
                        severity: EventSeverity::Critical,
                    });
                }
                if mem_pct >= self.mem_alert_threshold && self.prev_mem_pct < self.mem_alert_threshold {
                    let msg = format!("Memory exceeded threshold: {:.0}% >= {:.0}%", mem_pct, self.mem_alert_threshold);
                    send_notification("Digger: Memory Alert", &msg);
                    push_event(&mut self.event_log, LogEvent {
                        timestamp: Arc::clone(&now_str),
                        icon: ICON_WARNING,
                        message: msg,
                        severity: EventSeverity::Critical,
                    });
                }

                // Recovery events
                if snap.cpu_usage_global < self.cpu_alert_threshold && self.prev_cpu >= self.cpu_alert_threshold {
                    push_event(&mut self.event_log, LogEvent {
                        timestamp: Arc::clone(&now_str),
                        icon: ICON_CHECK,
                        message: format!("CPU recovered: {:.0}% < {:.0}% threshold", snap.cpu_usage_global, self.cpu_alert_threshold),
                        severity: EventSeverity::Info,
                    });
                }
                if mem_pct < self.mem_alert_threshold && self.prev_mem_pct >= self.mem_alert_threshold {
                    push_event(&mut self.event_log, LogEvent {
                        timestamp: Arc::clone(&now_str),
                        icon: ICON_CHECK,
                        message: format!("Memory recovered: {:.0}% < {:.0}% threshold", mem_pct, self.mem_alert_threshold),
                        severity: EventSeverity::Info,
                    });
                }

                // Temperature alerts
                let max_temp = snap.temperatures.iter().map(|t| t.temp_c).fold(0.0_f32, f32::max);
                if max_temp > 85.0 {
                    let temp_msg = format!("High temperature: {:.0}°C", max_temp);
                    send_notification("Digger: Temperature Alert", &temp_msg);
                    push_event(&mut self.event_log, LogEvent {
                        timestamp: now_str,
                        icon: ICON_TEMP,
                        message: format!("High temperature: {:.0}°C", max_temp),
                        severity: EventSeverity::Critical,
                    });
                }

                self.prev_cpu = snap.cpu_usage_global;
                self.prev_mem_pct = mem_pct;

                // ─── Heartbeat BPM ───
                self.health_score = compute_heartbeat_bpm(
                    snap.cpu_usage_global, mem_pct
                );

                self.current = Some(snap);

                // Opt #7: Throttle History tab SQL reload to every 10s.
                if self.tab == Tab::History && (now_ts - self.history_last_reload >= HISTORY_RELOAD_INTERVAL_SECS) {
                    self.history_last_reload = now_ts;
                    let range = HISTORY_RANGES[self.history_range_idx].0;
                    self.history_points = self.history.load_last_n_seconds_downsampled(range, 600);
                }
            }
            Message::AnimTick => {
                // Opt #4: Skip animation work when values have converged.
                let mut needs_anim = self.page_opacity < 1.0;

                if let Some(snap) = &self.current {
                    let target_cpu = snap.cpu_usage_global;
                    let target_mem = if snap.memory_total > 0 {
                        snap.memory_used as f32 / snap.memory_total as f32 * 100.0
                    } else { 0.0 };

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

                // Page fade-in
                if self.page_opacity < 1.0 {
                    self.page_opacity = (self.page_opacity + FADE_SPEED).min(1.0);
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
                // Trigger fade-in on page change
                if tab != self.prev_tab {
                    self.page_opacity = 0.0;
                }
                if tab == Tab::History {
                    // Force immediate reload on tab switch
                    self.history_last_reload = 0.0;
                    let range = HISTORY_RANGES[self.history_range_idx].0;
                    self.history_points = self.history.load_last_n_seconds_downsampled(range, 600);
                }
            }
            Message::OverviewSection(s) => {
                if s != self.overview_panel {
                    self.page_opacity = 0.0;
                }
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
                self.page_opacity = 0.0;
            }
            Message::SettingsPanelSelected(p) => {
                if p != self.settings_panel {
                    self.page_opacity = 0.0;
                }
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
            Message::SetTheme(variant) => {
                self.theme_variant = variant;
                self.pal = build_palette(variant, self.accent_color);
                self.save_prefs();
            }
            Message::SetAccent(accent) => {
                self.accent_color = accent;
                self.pal = build_palette(self.theme_variant, accent);
                self.cached_theme_previews = Self::build_theme_previews(accent);
                self.cached_theme_accent = accent;
                self.save_prefs();
            }
            Message::ToggleDyslexicFont => {
                self.use_dyslexic_font = !self.use_dyslexic_font;
                self.save_prefs();
            }
            Message::ExportCsv => {
                let range = HISTORY_RANGES[self.history_range_idx].0;
                let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
                let csv = self.history.export_csv(now - range, now);
                if let Some(dir) = dirs::download_dir().or_else(dirs::home_dir) {
                    let path = dir.join("digger_export.csv");
                    match std::fs::write(&path, &csv) {
                        Ok(_) => self.status_message = Some(format!("Exported to {}", path.display())),
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
                        Ok(_) => self.status_message = Some(format!("Exported to {}", path.display())),
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
                        self.status_message = Some(format!("Failed to kill PID {pid} (permission denied?)"));
                    }
                }
                #[cfg(windows)]
                {
                    use std::ptr;
                    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LUID};
                    use windows_sys::Win32::Security::{
                        AdjustTokenPrivileges, LookupPrivilegeValueW,
                        SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
                        TOKEN_QUERY,
                    };
                    use windows_sys::Win32::System::Threading::{
                        GetCurrentProcess, OpenProcess, OpenProcessToken,
                        TerminateProcess, PROCESS_TERMINATE,
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
                        ) != 0 {
                            let priv_name: Vec<u16> = "SeDebugPrivilege\0"
                                .encode_utf16().collect();
                            let mut luid = LUID { LowPart: 0, HighPart: 0 };
                            if LookupPrivilegeValueW(
                                ptr::null(), priv_name.as_ptr(), &mut luid,
                            ) != 0 {
                                let mut tp = TOKEN_PRIVILEGES {
                                    PrivilegeCount: 1,
                                    Privileges: [windows_sys::Win32::Security::LUID_AND_ATTRIBUTES {
                                        Luid: luid,
                                        Attributes: SE_PRIVILEGE_ENABLED,
                                    }],
                                };
                                AdjustTokenPrivileges(
                                    token, 0, &mut tp, 0, ptr::null_mut(), ptr::null_mut(),
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
                    self.status_message = Some("Process kill not supported on this platform".into());
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
                self.ui_mono = font_for_lang(lang);
                self.rebuild_cached_strings();
                self.save_prefs();
            }
            Message::KeyPressed(key, modifiers) => {
                use keyboard::key::Named;
                match key {
                    // Tab navigation: 1-4 for tabs
                    keyboard::Key::Character(ref c) if !self.show_settings => {
                        match c.as_str() {
                            "1" => { self.prev_tab = self.tab; self.tab = Tab::Overview; self.page_opacity = 0.0; }
                            "2" => { self.prev_tab = self.tab; self.tab = Tab::Processes; self.page_opacity = 0.0; }
                            "3" => { self.prev_tab = self.tab; self.tab = Tab::History; self.page_opacity = 0.0; }
                            "4" => { self.prev_tab = self.tab; self.tab = Tab::EventLog; self.page_opacity = 0.0; }
                            "s" | "," => {
                                self.prev_show_settings = self.show_settings;
                                self.show_settings = !self.show_settings;
                                self.page_opacity = 0.0;
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
                            self.page_opacity = 0.0;
                        }
                    }
                    keyboard::Key::Named(Named::Tab) if !modifiers.shift() && !self.show_settings => {
                        // Cycle tabs forward
                        self.prev_tab = self.tab;
                        self.tab = match self.tab {
                            Tab::Overview => Tab::Processes,
                            Tab::Processes => Tab::History,
                            Tab::History => Tab::EventLog,
                            Tab::EventLog => Tab::Overview,
                        };
                        self.page_opacity = 0.0;
                    }
                    keyboard::Key::Named(Named::Tab) if modifiers.shift() && !self.show_settings => {
                        // Cycle tabs backward
                        self.prev_tab = self.tab;
                        self.tab = match self.tab {
                            Tab::Overview => Tab::EventLog,
                            Tab::Processes => Tab::Overview,
                            Tab::History => Tab::Processes,
                            Tab::EventLog => Tab::History,
                        };
                        self.page_opacity = 0.0;
                    }
                    _ => {}
                }
            }
        }
    }

    fn save_prefs(&self) {
        let prefs = Preferences {
            theme: self.theme_variant,
            accent: self.accent_color,
            refresh_interval_secs: self.refresh_interval_secs,
            temp_celsius: self.temp_celsius,
            process_limit: self.process_limit,
            live_buffer_size: self.live_max,
            retention_hours: self.retention_hours,
            cpu_alert_threshold: self.cpu_alert_threshold,
            mem_alert_threshold: self.mem_alert_threshold,
            use_dyslexic_font: self.use_dyslexic_font,
            process_grouped: self.process_grouped,
            process_sort: match self.process_sort {
                ProcessSort::Pid => "pid",
                ProcessSort::Name => "name",
                ProcessSort::Cpu => "cpu",
                ProcessSort::Memory => "memory",
            }.into(),
            process_sort_asc: self.process_sort_asc,
            auto_theme: false, // When saving manually, auto is off
            language: self.language,
        };
        prefs.save();
    }

    fn chart_colors(&self) -> ChartColors {
        ChartColors {
            bg: self.pal.panel_bg,
            border: self.pal.border,
            grid: self.pal.grid,
            label: self.pal.label,
            text: self.pal.text,
        }
    }

    // ─── MAIN VIEW ──────────────────────────────────────────────

    pub fn view(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let tabs = row![
            menu_tab(&self.cached_tab_overview, Tab::Overview, self.tab, p, self.ui_mono),
            menu_tab(&self.cached_tab_processes, Tab::Processes, self.tab, p, self.ui_mono),
            menu_tab(&self.cached_tab_history, Tab::History, self.tab, p, self.ui_mono),
            menu_tab(&self.cached_tab_events, Tab::EventLog, self.tab, p, self.ui_mono),
        ]
        .spacing(4);

        let digger_label = if self.show_settings {
            &self.cached_digger_label_settings
        } else {
            &self.cached_digger_label
        };
        let accent = p.accent;
        let digger_btn = button(
            text(digger_label).size(15).color(accent)
        )
        .on_press(Message::ToggleSettings)
        .style(button::text)
        .padding([2, 4]);

        let border_c = p.border;
        let text_c = p.text;

        // Heartbeat BPM indicator with pulsing icon
        let bpm = self.health_score;
        let heart_color = if bpm < 100.0 { p.green }
            else if bpm <= 130.0 { p.yellow }
            else { p.red };
        // Sharp beat curve: sin clamped to positive half, squared for snappy pulse
        let beat = self.heart_phase.sin().max(0.0).powi(2);
        let heart_size = 10.0 + beat * 4.0; // 10px base, up to 14px on beat
        let health_el: Element<Message> = row![
            container(text(ICON_HEART).size(heart_size as u16).color(heart_color))
                .width(16)
                .height(16)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
            text(format!(" {:.0}", bpm)).size(10).font(self.ui_mono).color(heart_color),
        ].spacing(0).align_y(Alignment::Center).into();

        // Status bar with alerts/errors/messages
        let status_el: Element<Message> = if let Some(msg) = &self.status_message {
            let warning_color = p.yellow;
            text(msg).size(10).color(warning_color).into()
        } else {
            Space::new(0, 0).into()
        };

        // Event log badge
        let event_count = self.event_log.len();
        let event_badge: Element<Message> = if event_count > 0 {
            let badge_color = if self.event_log.back().map(|e| e.severity) == Some(EventSeverity::Critical) {
                p.red
            } else {
                p.yellow
            };
            row![
                text(ICON_LOG).size(10).color(badge_color),
                text(format!(" {}", event_count)).size(10).font(self.ui_mono).color(badge_color),
            ].spacing(0).align_y(Alignment::Center).into()
        } else {
            Space::new(0, 0).into()
        };

        let menu_bar = row![
            digger_btn,
            Space::with_width(8),
            health_el,
            Space::with_width(6),
            event_badge,
            Space::with_width(8),
            text(ICON_SEPARATOR).size(14).color(border_c),
            Space::with_width(8),
            status_el,
            Space::with_width(Length::Fill),
            tabs,
            Space::with_width(Length::Fill),
            text(chrono::Local::now().format("%H:%M:%S").to_string())
                .size(13)
                .font(self.ui_mono)
                .color(text_c),
        ]
        .align_y(Alignment::Center)
        .padding([6, 12]);

        let content: Element<Message> = if self.show_settings {
            self.view_settings()
        } else {
            match self.tab {
                Tab::Overview => self.view_overview(),
                Tab::Processes => self.view_processes(),
                Tab::History => self.view_history(),
                Tab::EventLog => self.view_event_log(),
            }
        };

        let bg = p.bg;
        let sidebar_bg = p.sidebar_bg;
        let main = column![
            panel_bg(menu_bar.into(), sidebar_bg, border_c),
            content,
        ]
        .spacing(0);

        container(main)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(bg)),
                ..Default::default()
            })
            .into()
    }

    // ─── EVENT LOG TAB ─────────────────────────────────────────

    fn view_event_log(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let t = self.t();
        let label_c = p.label;
        let panel_bg = p.panel_bg;
        let bg = p.bg;

        let title_row = row![
            text(format!("{ICON_LOG} {}", t.event_log)).size(13).font(self.ui_mono).color(p.accent),
            Space::with_width(Length::Fill),
            text(format!("{} {}", self.event_log.len(), t.events)).size(11).font(self.ui_mono).color(label_c),
        ]
        .padding([6, 10])
        .align_y(Alignment::Center);

        let mut rows: Vec<Element<Message>> = Vec::new();

        if self.event_log.is_empty() {
            rows.push(
                container(
                    text(t.no_events).size(12).font(self.ui_mono).color(label_c)
                )
                .padding([20, 10])
                .center_x(Length::Fill)
                .into()
            );
        } else {
            for (i, ev) in self.event_log.iter().rev().enumerate() {
                let sev_color = match ev.severity {
                    EventSeverity::Info => p.green,
                    EventSeverity::Warning => p.yellow,
                    EventSeverity::Critical => p.red,
                };
                let row_bg = if i % 2 == 0 { panel_bg } else { bg };
                let r = container(
                    row![
                        text(&*ev.timestamp).size(10).font(self.ui_mono).color(label_c).width(80),
                        text(ev.icon).size(11).color(sev_color).width(20),
                        text(&ev.message).size(11).color(p.text),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                )
                .padding([3, 10])
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(row_bg)),
                    ..Default::default()
                });
                rows.push(r.into());
            }
        }

        let table = Column::with_children(rows).spacing(0);
        let content = panel(
            column![title_row, table].spacing(0).into(),
            p,
        );

        scrollable(column![content].padding(4)).into()
    }

    // ─── SETTINGS VIEW ─────────────────────────────────────────

    fn view_settings(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let sidebar_bg = p.sidebar_bg;
        let border_c = p.border;

        let sidebar = container(
            column![
                settings_sidebar_item(
                    format!("{ICON_SETTINGS}  {}", self.t().general_settings),
                    SettingsPanel::General,
                    self.settings_panel,
                    p, self.ui_mono,
                ),
                settings_sidebar_item(
                    format!("{ICON_PAINT}  {}", self.t().appearance),
                    SettingsPanel::Appearance,
                    self.settings_panel,
                    p, self.ui_mono,
                ),
                settings_sidebar_item(
                    format!("{ICON_ACCESS}  {}", self.t().accessibility),
                    SettingsPanel::Accessibility,
                    self.settings_panel,
                    p, self.ui_mono,
                ),
                settings_sidebar_item(
                    format!("{ICON_NETWORK}  {}", self.t().language),
                    SettingsPanel::Language,
                    self.settings_panel,
                    p, self.ui_mono,
                ),
                settings_sidebar_item(
                    format!("{ICON_INFO}  {}", self.t().about_digger),
                    SettingsPanel::About,
                    self.settings_panel,
                    p, self.ui_mono,
                ),
            ]
            .spacing(2)
            .padding(8)
        )
        .width(170)
        .height(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(sidebar_bg)),
            border: Border { color: border_c, width: 1.0, radius: 0.0.into() },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                offset: Vector::new(2.0, 0.0),
                blur_radius: 8.0,
            },
            ..Default::default()
        });

        let detail = match self.settings_panel {
            SettingsPanel::General => self.view_settings_general(),
            SettingsPanel::Appearance => self.view_settings_appearance(),
            SettingsPanel::Accessibility => self.view_settings_accessibility(),
            SettingsPanel::Language => self.view_settings_language(),
            SettingsPanel::About => self.view_settings_about(),
        };

        row![
            sidebar,
            scrollable(
                container(detail).width(Length::Fill).padding(16)
            ),
        ]
        .spacing(0)
        .height(Length::Fill)
        .into()
    }

    fn view_settings_general(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let text_c = p.text;
        let label_c = p.label;
        let accent = p.accent;
        let green = p.green;
        let t = self.t();

        let title = column![
            text(t.general_settings).size(16).font(self.ui_mono).color(text_c),
            text(t.settings_saved_auto).size(11).font(self.ui_mono).color(label_c),
        ]
        .spacing(4);

        let mut rate_btns: Vec<Element<Message>> = Vec::new();
        for &secs in REFRESH_OPTIONS {
            let is_active = self.refresh_interval_secs == secs;
            let color = if is_active { accent } else { label_c };
            let btn = button(
                text(format!("{secs}s")).size(11).font(self.ui_mono).color(color)
            )
            .on_press(Message::SetRefreshInterval(secs))
            .style(if is_active { button::primary } else { button::secondary })
            .padding([4, 12]);
            rate_btns.push(btn.into());
        }

        let refresh_row = row![
            column![
                text(t.refresh_rate).size(12).font(self.ui_mono).color(text_c),
                text(t.refresh_rate_desc).size(10).font(self.ui_mono).color(label_c),
            ].spacing(2).width(Length::FillPortion(2)),
            Row::with_children(rate_btns).spacing(4),
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let temp_toggle = button(
            text(if self.temp_celsius { ICON_TOGGLE_ON } else { ICON_TOGGLE_OFF })
                .size(22)
                .color(if self.temp_celsius { accent } else { label_c })
        )
        .on_press(Message::ToggleTempUnit)
        .style(button::text)
        .padding(0);

        let temp_label = if self.temp_celsius { format!("{} (\u{00b0}C)", t.celsius) } else { format!("{} (\u{00b0}F)", t.fahrenheit) };

        let temp_row = row![
            column![
                text(t.temperature_unit).size(12).font(self.ui_mono).color(text_c),
                text(format!("{} {temp_label}", t.currently)).size(10).font(self.ui_mono).color(label_c),
            ].spacing(2).width(Length::FillPortion(2)),
            temp_toggle,
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let monitoring_section = collapsible_section(
            SettingsSection::Monitoring,
            t.monitoring,
            t.monitoring_desc,
            self.collapsed_sections.contains(&SettingsSection::Monitoring),
            column![
                refresh_row,
                Space::with_height(12),
                temp_row,
            ].into(),
            p,
            self.ui_mono,
        );

        let process_limit_row = row![
            column![
                text(t.process_limit).size(12).font(self.ui_mono).color(text_c),
                text(t.process_limit_desc).size(10).font(self.ui_mono).color(label_c),
            ].spacing(2).width(Length::FillPortion(2)),
            text(format!("{}", self.process_limit)).size(12).font(self.ui_mono).color(accent),
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let history_points_row = row![
            column![
                text(t.history_buffer).size(12).font(self.ui_mono).color(text_c),
                text(t.history_buffer_desc).size(10).font(self.ui_mono).color(label_c),
            ].spacing(2).width(Length::FillPortion(2)),
            text(format!("{}", self.live_max)).size(12).font(self.ui_mono).color(accent),
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let retention_row = row![
            column![
                text(t.history_retention).size(12).font(self.ui_mono).color(text_c),
                text(t.history_retention_desc).size(10).font(self.ui_mono).color(label_c),
            ].spacing(2).width(Length::FillPortion(2)),
            text(format!("{}h", self.retention_hours)).size(12).font(self.ui_mono).color(accent),
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let display_section = collapsible_section(
            SettingsSection::Display,
            t.display,
            t.display_desc,
            self.collapsed_sections.contains(&SettingsSection::Display),
            column![
                process_limit_row,
                Space::with_height(12),
                history_points_row,
                Space::with_height(12),
                retention_row,
            ].into(),
            p,
            self.ui_mono,
        );

        let db_status = if self.history.is_available() {
            format!("{ICON_CHECK} {}", t.active)
        } else {
            format!("{ICON_WARNING} {}", t.unavailable)
        };
        let db_color = if self.history.is_available() { green } else { p.red };

        let mut data_items: Vec<Element<Message>> = vec![
            row![
                column![
                    text(t.history_database).size(12).font(self.ui_mono).color(text_c),
                    text(t.history_database_desc).size(10).font(self.ui_mono).color(label_c),
                ].spacing(2).width(Length::FillPortion(2)),
                text(db_status).size(11).font(self.ui_mono).color(db_color),
            ]
            .align_y(Alignment::Center)
            .spacing(12)
            .into(),
        ];

        // Show DB error if any
        if let Some(err) = &self.history.last_error {
            data_items.push(Space::with_height(6).into());
            data_items.push(
                text(format!("{ICON_WARNING} {err}")).size(10).color(p.red).into()
            );
        }

        let data_section = collapsible_section(
            SettingsSection::Data,
            t.data,
            "",
            self.collapsed_sections.contains(&SettingsSection::Data),
            Column::with_children(data_items).spacing(0).into(),
            p,
            self.ui_mono,
        );

        // Alert thresholds section
        let cpu_alert_btns = make_threshold_buttons(
            self.cpu_alert_threshold,
            &[70.0, 80.0, 90.0, 95.0],
            Message::SetCpuAlertThreshold,
            accent, label_c, self.ui_mono,
        );
        let mem_alert_btns = make_threshold_buttons(
            self.mem_alert_threshold,
            &[70.0, 80.0, 90.0, 95.0],
            Message::SetMemAlertThreshold,
            accent, label_c, self.ui_mono,
        );

        let alerts_section = collapsible_section(
            SettingsSection::Alerts,
            t.alerts,
            t.alerts_desc,
            self.collapsed_sections.contains(&SettingsSection::Alerts),
            column![
                row![
                    column![
                        text(t.cpu_threshold).size(12).font(self.ui_mono).color(text_c),
                        text(t.cpu_threshold_desc).size(10).font(self.ui_mono).color(label_c),
                    ].spacing(2).width(Length::FillPortion(2)),
                    cpu_alert_btns,
                ].align_y(Alignment::Center).spacing(12),
                Space::with_height(12),
                row![
                    column![
                        text(t.memory_threshold).size(12).font(self.ui_mono).color(text_c),
                        text(t.memory_threshold_desc).size(10).font(self.ui_mono).color(label_c),
                    ].spacing(2).width(Length::FillPortion(2)),
                    mem_alert_btns,
                ].align_y(Alignment::Center).spacing(12),
            ].into(),
            p,
            self.ui_mono,
        );

        column![
            title,
            Space::with_height(16),
            monitoring_section,
            Space::with_height(6),
            display_section,
            Space::with_height(6),
            data_section,
            Space::with_height(6),
            alerts_section,
        ]
        .spacing(4)
        .into()
    }

    fn view_settings_appearance(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let text_c = p.text;
        let label_c = p.label;
        let accent = p.accent;
        let panel_bg = p.panel_bg;
        let border_c = p.border;
        let t = self.t();

        let title = column![
            text(t.appearance).size(16).font(self.ui_mono).color(text_c),
            text(t.appearance_desc).size(11).font(self.ui_mono).color(label_c),
        ]
        .spacing(4);

        // Build theme grid grouped by family (using cached palettes)
        let families: &[(&str, &[ThemeVariant])] = &[
            ("Catppuccin", &[
                ThemeVariant::CatppuccinLatte,
                ThemeVariant::CatppuccinFrappe,
                ThemeVariant::CatppuccinMacchiato,
                ThemeVariant::CatppuccinMocha,
            ]),
            ("Gruvbox", &[
                ThemeVariant::GruvboxLight,
                ThemeVariant::GruvboxDark,
            ]),
            ("Everblush", &[
                ThemeVariant::EverblushLight,
                ThemeVariant::EverblushDark,
            ]),
            ("Kanagawa", &[
                ThemeVariant::KanagawaLight,
                ThemeVariant::KanagawaDark,
                ThemeVariant::KanagawaDragon,
            ]),
        ];

        let mut theme_items: Vec<Element<Message>> = Vec::new();
        for (family_name, variants) in families {
            theme_items.push(
                text(*family_name).size(13).color(text_c).into()
            );
            theme_items.push(Space::with_height(2).into());
            let mut variant_btns: Vec<Element<Message>> = Vec::new();
            for &variant in *variants {
                let is_active = self.theme_variant == variant;
                // Use cached palette instead of rebuilding every frame
                let pv = self.cached_theme_previews.iter()
                    .find(|(v, _)| *v == variant)
                    .map(|(_, p)| p.clone())
                    .unwrap_or_else(|| build_palette(variant, self.accent_color));
                let pv_bg = pv.bg;
                let pv_panel = pv.panel_bg;
                let pv_text = pv.text;
                let pv_label = pv.label;
                let pv_accent = pv.accent;
                let pv_green = pv.green;
                let pv_red = pv.red;
                let pv_yellow = pv.yellow;
                let btn_border = if is_active { accent } else { border_c };
                let btn_width = if is_active { 2.5 } else { 1.0 };

                // Color swatch dots showing the palette
                let swatches = row![
                    text(ICON_BULLET).size(10).color(pv_accent),
                    text(ICON_BULLET).size(10).color(pv_green),
                    text(ICON_BULLET).size(10).color(pv_red),
                    text(ICON_BULLET).size(10).color(pv_yellow),
                ]
                .spacing(1);

                let card = container(
                    column![
                        // Top: panel bg strip
                        container(Space::new(Length::Fill, 6))
                            .width(Length::Fill)
                            .style(move |_: &Theme| container::Style {
                                background: Some(Background::Color(pv_panel)),
                                ..Default::default()
                            }),
                        // Body
                        container(
                            column![
                                text(variant.name()).size(11).color(pv_text),
                                swatches,
                            ]
                            .spacing(4)
                            .align_x(Alignment::Center)
                        )
                        .center_x(Length::Fill)
                        .padding([6, 8]),
                    ]
                    .spacing(0)
                )
                .width(100)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(pv_bg)),
                    border: Border { color: pv_label, width: 0.0, radius: 6.0.into() },
                    ..Default::default()
                });

                let btn = button(card)
                    .on_press(Message::SetTheme(variant))
                    .padding(0)
                    .style(move |_: &Theme, _status| button::Style {
                        background: Some(Background::Color(pv_bg)),
                        text_color: pv_text,
                        border: Border {
                            color: btn_border,
                            width: btn_width,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    });
                variant_btns.push(btn.into());
            }
            theme_items.push(
                Row::with_children(variant_btns).spacing(8).into()
            );
            theme_items.push(Space::with_height(10).into());
        }

        let theme_section = collapsible_section(
            SettingsSection::Theme,
            t.theme,
            t.theme_desc,
            self.collapsed_sections.contains(&SettingsSection::Theme),
            Column::with_children(theme_items).spacing(4).into(),
            p,
            self.ui_mono,
        );

        // Accent color selector
        let mut accent_btns: Vec<Element<Message>> = Vec::new();
        for &ac in AccentColor::ALL {
            let is_active = self.accent_color == ac;
            let ac_color = ac.color();
            let check_color = text_c;
            let btn_border = if is_active { text_c } else { Color::TRANSPARENT };

            let label_el: Element<Message> = if is_active {
                text(ICON_CHECK).size(12).color(check_color).into()
            } else {
                Space::new(0, 0).into()
            };

            let btn = button(
                container(label_el)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .width(32)
                    .height(32)
                    .style(move |_: &Theme| container::Style {
                        background: Some(Background::Color(ac_color)),
                        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 16.0.into() },
                        ..Default::default()
                    })
            )
            .on_press(Message::SetAccent(ac))
            .padding(0)
            .style(move |_: &Theme, _status| button::Style {
                background: Some(Background::Color(ac_color)),
                text_color: check_color,
                border: Border {
                    color: btn_border,
                    width: if is_active { 2.0 } else { 0.0 },
                    radius: 16.0.into(),
                },
                ..Default::default()
            });
            accent_btns.push(
                column![
                    btn,
                    text(ac.name()).size(9).color(label_c),
                ]
                .align_x(Alignment::Center)
                .spacing(2)
                .into()
            );
        }

        let accent_section = collapsible_section(
            SettingsSection::Accent,
            t.accent_color,
            t.accent_color_desc,
            self.collapsed_sections.contains(&SettingsSection::Accent),
            Row::with_children(accent_btns).spacing(8).into(),
            p,
            self.ui_mono,
        );

        // Current theme info
        let current_info = container(
            row![
                text(format!("{ICON_CHECK} {}  {}", self.theme_variant.family(), self.theme_variant.name()))
                    .size(11).font(self.ui_mono).color(accent),
                Space::with_width(12),
                text(format!("{ICON_BULLET} Accent: {}", self.accent_color.name()))
                    .size(11).font(self.ui_mono).color(self.accent_color.color()),
            ]
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(panel_bg)),
            border: Border { color: border_c, width: 1.0, radius: 8.0.into() },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 4.0,
            },
            ..Default::default()
        });

        column![
            title,
            Space::with_height(8),
            current_info,
            Space::with_height(12),
            theme_section,
            Space::with_height(6),
            accent_section,
        ]
        .spacing(4)
        .into()
    }

    fn view_settings_accessibility(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let text_c = p.text;
        let label_c = p.label;
        let accent = p.accent;
        let t = self.t();

        let title = column![
            text(t.accessibility).size(16).font(self.ui_mono).color(text_c),
            text(t.accessibility_desc).size(11).font(self.ui_mono).color(label_c),
        ]
        .spacing(4);

        let dyslexic_toggle = button(
            text(if self.use_dyslexic_font { ICON_TOGGLE_ON } else { ICON_TOGGLE_OFF })
                .size(22)
                .color(if self.use_dyslexic_font { accent } else { label_c })
        )
        .on_press(Message::ToggleDyslexicFont)
        .style(button::text)
        .padding(0);

        let font_status = if self.use_dyslexic_font { t.enabled } else { t.disabled };

        let font_section = collapsible_section(
            SettingsSection::Fonts,
            t.fonts,
            t.fonts_desc,
            self.collapsed_sections.contains(&SettingsSection::Fonts),
            column![
                row![
                    column![
                        text(t.dyslexic_font).size(12).font(self.ui_mono).color(text_c),
                        text(format!("{} {} {font_status}", t.dyslexic_font_desc, t.currently)).size(10).font(self.ui_mono).color(label_c),
                    ].spacing(2).width(Length::FillPortion(2)),
                    dyslexic_toggle,
                ]
                .align_y(Alignment::Center)
                .spacing(12),
            ].into(),
            p,
            self.ui_mono,
        );

        column![
            title,
            Space::with_height(16),
            font_section,
        ]
        .spacing(4)
        .into()
    }

    fn view_settings_language(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let text_c = p.text;
        let label_c = p.label;
        let accent = p.accent;
        let panel_bg = p.panel_bg;
        let border_c = p.border;
        let t = self.t();

        let title = column![
            text(t.language).size(16).font(self.ui_mono).color(text_c),
            text(t.language_desc).size(11).font(self.ui_mono).color(label_c),
        ]
        .spacing(4);

        // Current language info
        let current_info = container(
            row![
                {
                    let name = if has_native_font(self.language) { self.language.native_name() } else { self.language.english_name() };
                    text(format!("{ICON_CHECK} {name}")).size(12).color(accent).font(font_for_lang(self.language))
                },
                Space::with_width(8),
                text(format!("({})", self.language.code()))
                    .size(11).color(label_c),
            ]
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(panel_bg)),
            border: Border { color: border_c, width: 1.0, radius: 8.0.into() },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 4.0,
            },
            ..Default::default()
        });

        // Language grid: 2 columns
        let all = Language::ALL;
        let rows_count = all.len().div_ceil(2);
        let mut grid_rows: Vec<Element<Message>> = Vec::new();
        for r in 0..rows_count {
            let mut cols: Vec<Element<Message>> = Vec::new();
            for c in 0..2 {
                let idx = r + c * rows_count;
                if idx < all.len() {
                    let lang = all[idx];
                    let is_active = self.language == lang;
                    let lang_accent = accent;
                    let lang_label_c = label_c;
                    let lang_text_c = text_c;
                    let lang_panel_bg = panel_bg;
                    let lang_border_c = border_c;
                    let btn_border = if is_active { lang_accent } else { lang_border_c };
                    let btn_width = if is_active { 2.0 } else { 1.0 };
                    let active_bg = if is_active {
                        Color::from_rgba(lang_accent.r, lang_accent.g, lang_accent.b, 0.1)
                    } else {
                        lang_panel_bg
                    };
                    let hover_bg = Color::from_rgba(lang_accent.r, lang_accent.g, lang_accent.b, 0.05);

                    let check: Element<Message> = if is_active {
                        text(ICON_CHECK).size(11).color(lang_accent).into()
                    } else {
                        Space::new(11, 0).into()
                    };

                    let content = row![
                        check,
                        Space::with_width(4),
                        column![
                            {
                                let name_color = if is_active { lang_accent } else { lang_text_c };
                                let name = if has_native_font(lang) { lang.native_name() } else { lang.english_name() };
                                text(name).size(11).font(font_for_lang(lang)).color(name_color)
                            },
                            text(lang.code()).size(9).color(lang_label_c),
                        ].spacing(1),
                    ]
                    .align_y(Alignment::Center);

                    let btn = button(content)
                        .on_press(Message::SetLanguage(lang))
                        .width(Length::Fill)
                        .padding([6, 10])
                        .style(move |_: &Theme, status| {
                            let bg = match status {
                                button::Status::Hovered => hover_bg,
                                button::Status::Pressed => active_bg,
                                _ => active_bg,
                            };
                            button::Style {
                                background: Some(Background::Color(bg)),
                                text_color: lang_text_c,
                                border: Border {
                                    color: btn_border,
                                    width: btn_width,
                                    radius: 6.0.into(),
                                },
                                ..Default::default()
                            }
                        });
                    cols.push(container(btn).width(Length::FillPortion(1)).into());
                } else {
                    cols.push(Space::with_width(Length::FillPortion(1)).into());
                }
            }
            grid_rows.push(Row::with_children(cols).spacing(6).into());
        }
        let grid = Column::with_children(grid_rows).spacing(3);

        column![
            title,
            Space::with_height(8),
            current_info,
            Space::with_height(12),
            grid,
        ]
        .spacing(4)
        .into()
    }

    fn view_settings_about(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let text_c = p.text;
        let label_c = p.label;
        let t = self.t();

        let title = column![
            text(t.about_digger).size(16).font(self.ui_mono).color(text_c),
            text(t.about_desc).size(11).font(self.ui_mono).color(label_c),
        ]
        .spacing(4);

        let version_section = collapsible_section(
            SettingsSection::Version,
            t.version,
            "",
            self.collapsed_sections.contains(&SettingsSection::Version),
            column![
                info_row(t.application, "Digger", p, self.ui_mono),
                info_row(t.version, "0.1.0", p, self.ui_mono),
                info_row(t.framework, "Iced 0.13 + Rust", p, self.ui_mono),
                info_row(t.license, "MIT", p, self.ui_mono),
            ].spacing(6).into(),
            p,
            self.ui_mono,
        );

        let font_section = collapsible_section(
            SettingsSection::FontInfo,
            t.fonts,
            "",
            self.collapsed_sections.contains(&SettingsSection::FontInfo),
            column![
                info_row(t.ui_font, "Iosevka Nerd Font Propo", p, self.ui_mono),
                info_row(t.mono_font, "Iosevka Nerd Font Mono", p, self.ui_mono),
                info_row(t.dyslexic_font_label, "OpenDyslexic", p, self.ui_mono),
                info_row(t.nerd_fonts, "v3.4.0", p, self.ui_mono),
            ].spacing(6).into(),
            p,
            self.ui_mono,
        );

        // System info section
        let sys_items = if let Some(snap) = &self.current {
            column![
                info_row(t.hostname, &snap.sys_info.hostname, p, self.ui_mono),
                info_row(t.os, &snap.sys_info.os_name, p, self.ui_mono),
                info_row(t.os_version, &snap.sys_info.os_version, p, self.ui_mono),
                info_row(t.kernel, &snap.sys_info.kernel_version, p, self.ui_mono),
                info_row(t.cpu, &snap.cpu_name, p, self.ui_mono),
                info_row(t.cores, snap.cpu_core_count.to_string(), p, self.ui_mono),
                info_row(t.total_ram, format_bytes(snap.memory_total), p, self.ui_mono),
            ].spacing(6)
        } else {
            column![
                text(t.waiting_for_data).size(11).font(self.ui_mono).color(label_c),
            ]
        };

        let system_section = collapsible_section(
            SettingsSection::SystemInfo,
            t.system_information,
            "",
            self.collapsed_sections.contains(&SettingsSection::SystemInfo),
            sys_items.into(),
            p,
            self.ui_mono,
        );

        column![
            title,
            Space::with_height(16),
            version_section,
            Space::with_height(8),
            font_section,
            Space::with_height(8),
            system_section,
        ]
        .spacing(4)
        .into()
    }

    // ─── OVERVIEW TAB ───────────────────────────────────────────

    fn view_overview(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let t = self.t();
        let Some(snap) = &self.current else {
            return container(
                text(format!("{ICON_LOADING} {}", t.collecting_data)).size(14).font(self.ui_mono).color(p.label)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        };

        // Use animated values for smooth display
        let display_cpu = self.anim_cpu;
        let display_mem = self.anim_mem_pct;

        let sidebar_bg = p.sidebar_bg;
        let border_c = p.border;

        // Mini sparkline data
        let cpu_spark_data: Vec<f32> = self.live_buffer.iter().map(|lp| lp.cpu).collect();
        let mem_spark_data: Vec<f32> = self.live_buffer.iter().map(|lp| lp.mem_pct).collect();
        let disk_io_spark: Vec<f32> = self.live_buffer.iter()
            .map(|lp| (lp.disk_read + lp.disk_write) as f32 / 1024.0)
            .collect();

        let make_spark = |data: Vec<f32>, color: Color| -> Element<'_, Message> {
            Canvas::new(Sparkline {
                data,
                color,
            })
            .width(Length::Fill)
            .height(Length::Fixed(20.0))
            .into()
        };

        let sidebar = container(
            column![
                sidebar_item(
                    format!("{ICON_CPU} {}", t.cpu),
                    format!("{:.0}%", display_cpu),
                    dynamic_color(p.accent, display_cpu / 100.0),
                    OverviewPanel::Cpu, self.overview_panel, p, self.ui_mono,
                ),
                make_spark(cpu_spark_data, p.accent),
                sidebar_item(
                    format!("{ICON_MEMORY} {}", t.memory),
                    format!("{:.0}%", display_mem),
                    dynamic_color(p.green, display_mem / 100.0),
                    OverviewPanel::Memory, self.overview_panel, p, self.ui_mono,
                ),
                make_spark(mem_spark_data, p.green),
                sidebar_item(
                    format!("{ICON_DISK} {}", t.disk),
                    format!("{}/s I/O", format_bytes(snap.disk_io.read_bytes + snap.disk_io.write_bytes)),
                    p.cyan, OverviewPanel::Disk, self.overview_panel, p, self.ui_mono,
                ),
                make_spark(disk_io_spark, p.cyan),
                sidebar_item(
                    format!("{ICON_NETWORK} {}", t.network),
                    format!("{}/s", format_bytes(snap.net_rx_bytes + snap.net_tx_bytes)),
                    p.yellow, OverviewPanel::Network, self.overview_panel, p, self.ui_mono,
                ),
                sidebar_item(
                    format!("{ICON_TEMP} {}", t.temp),
                    format!("{} {}", snap.temperatures.len(), t.sensors),
                    p.red, OverviewPanel::Temperature, self.overview_panel, p, self.ui_mono,
                ),
                sidebar_item(
                    format!("{ICON_GPU} {}", t.gpu),
                    if snap.gpu.gpus.is_empty() { t.n_a.into() } else { format!("{} GPU(s)", snap.gpu.gpus.len()) },
                    p.magenta, OverviewPanel::Gpu, self.overview_panel, p, self.ui_mono,
                ),
                // Load Average (small display at bottom of sidebar)
                Space::with_height(Length::Fill),
                text(format!("{ICON_LOAD} {}", t.load)).size(10).font(self.ui_mono).color(p.label),
                text(format!("{:.2}  {:.2}  {:.2}", snap.load_avg[0], snap.load_avg[1], snap.load_avg[2]))
                    .size(10).font(self.ui_mono).color(p.text),
            ]
            .spacing(2)
            .padding(4)
        )
        .width(160)
        .height(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(sidebar_bg)),
            border: Border { color: border_c, width: 1.0, radius: 0.0.into() },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                offset: Vector::new(2.0, 0.0),
                blur_radius: 8.0,
            },
            ..Default::default()
        });

        let detail = match self.overview_panel {
            OverviewPanel::Cpu => self.view_detail_cpu(snap),
            OverviewPanel::Memory => self.view_detail_memory(snap),
            OverviewPanel::Network => self.view_detail_network(snap),
            OverviewPanel::Disk => self.view_detail_disk(snap),
            OverviewPanel::Temperature => self.view_detail_temp(snap),
            OverviewPanel::Gpu => self.view_detail_gpu(snap),
        };

        row![
            sidebar,
            scrollable(
                container(detail).width(Length::Fill).padding(6)
            ),
        ]
        .spacing(0)
        .height(Length::Fill)
        .into()
    }

    // ─── CPU Detail ──
    fn view_detail_cpu<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let cc = self.chart_colors();
        let cpu_data: Vec<f32> = self.live_buffer.iter().map(|p| p.cpu).collect();
        // Pulse effect: if CPU exceeds threshold, pulse the chart title
        let is_critical = self.anim_cpu >= self.cpu_alert_threshold;
        let pulse_alpha = if is_critical {
            0.7 + 0.3 * self.pulse_phase.sin().abs()
        } else {
            1.0
        };
        let title_color = if is_critical {
            Color::from_rgba(p.red.r, p.red.g, p.red.b, pulse_alpha)
        } else {
            p.accent
        };
        // Radial gauge for CPU
        let gc = GaugeColors {
            bg: p.panel_bg,
            label: p.label,
            text: p.text,
            bar_bg: p.bar_bg,
        };
        let cpu_gauge: Element<Message> = Canvas::new(RadialGauge {
            value: self.anim_cpu,
            label: "CPU".into(),
            color: title_color,
            colors: gc,
        })
        .width(Length::Fixed(120.0))
        .height(Length::Fixed(100.0))
        .into();

        let cpu_chart = make_chart(ChartCfg {
            title: format!("CPU {ICON_DASH} {:.1}%", self.anim_cpu),
            series: vec![("CPU".into(), title_color, cpu_data)],
            y_min: 0.0, y_max: 100.0, filled: true, height: 180.0, unit: "%".into(), colors: cc,
        });

        // Load average info
        let load_info: Row<Message> = row![
            text(format!("{ICON_LOAD} {}", t.load_avg)).size(10).font(self.ui_mono).color(p.label),
            text(format!(" 1m {:.2}", snap.load_avg[0])).size(10).font(self.ui_mono).color(p.text),
            text(format!("  5m {:.2}", snap.load_avg[1])).size(10).font(self.ui_mono).color(p.text),
            text(format!("  15m {:.2}", snap.load_avg[2])).size(10).font(self.ui_mono).color(p.text),
        ].spacing(2).align_y(Alignment::Center);

        // Use animated per-core values
        let cores = &self.anim_cores;
        let num_cols = if cores.len() > 16 { 4 } else if cores.len() > 8 { 3 } else { 2 };
        let rows_count = cores.len().div_ceil(num_cols);
        let mut grid_rows: Vec<Element<Message>> = Vec::new();
        for r in 0..rows_count {
            let mut cols: Vec<Element<Message>> = Vec::new();
            for c in 0..num_cols {
                let idx = r + c * rows_count;
                if idx < cores.len() {
                    let usage = cores[idx];
                    let color = gradient_color(usage / 100.0, p);
                    let core = row![
                        text(format!("C{idx:<2}")).size(10).font(self.ui_mono).color(p.label).width(26),
                        themed_bar(usage, color, p.bar_bg),
                        text(format!("{usage:>3.0}%")).size(10).font(self.ui_mono).color(color).width(36),
                    ]
                    .spacing(2)
                    .align_y(Alignment::Center);
                    cols.push(container(core).width(Length::FillPortion(1)).into());
                } else {
                    cols.push(Space::with_width(Length::FillPortion(1)).into());
                }
            }
            grid_rows.push(Row::with_children(cols).spacing(8).into());
        }
        let cores_grid = Column::with_children(grid_rows).spacing(1);

        let uptime = format_duration(snap.uptime_secs);
        let info = column![
            info_row(t.model, &snap.cpu_name, p, self.ui_mono),
            info_row(t.logical_cores, snap.cpu_core_count.to_string(), p, self.ui_mono),
            info_row(t.base_speed, format!("{} MHz", snap.cpu_frequency_mhz), p, self.ui_mono),
            info_row(t.utilization, format!("{:.1}%", self.anim_cpu), p, self.ui_mono),
            info_row(t.processes, snap.process_count.to_string(), p, self.ui_mono),
            info_row(t.uptime, &uptime, p, self.ui_mono),
        ]
        .spacing(4);

        panel(
            column![
                row![
                    cpu_gauge,
                    column![cpu_chart].width(Length::Fill),
                ].spacing(6).align_y(Alignment::Center),
                Space::with_height(4),
                Element::from(load_info),
                Space::with_height(6),
                section_title(t.per_core_usage, p, self.ui_mono),
                cores_grid,
                Space::with_height(6),
                section_title(t.system_info, p, self.ui_mono),
                info,
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── Memory Detail ──
    fn view_detail_memory<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let cc = self.chart_colors();
        let mem_data: Vec<f32> = self.live_buffer.iter().map(|p| p.mem_pct).collect();
        let display_mem = self.anim_mem_pct;
        // Pulse effect for memory threshold
        let is_critical = display_mem >= self.mem_alert_threshold;
        let pulse_alpha = if is_critical {
            0.7 + 0.3 * self.pulse_phase.sin().abs()
        } else {
            1.0
        };
        let chart_color = if is_critical {
            Color::from_rgba(p.red.r, p.red.g, p.red.b, pulse_alpha)
        } else {
            p.green
        };
        let mem_chart = make_chart(ChartCfg {
            title: format!("Memory {ICON_DASH} {:.1}%", display_mem),
            series: vec![("RAM".into(), chart_color, mem_data)],
            y_min: 0.0, y_max: 100.0, filled: true, height: 200.0, unit: "%".into(), colors: cc,
        });

        let swap_pct = if snap.swap_total > 0 {
            snap.swap_used as f32 / snap.swap_total as f32 * 100.0
        } else { 0.0 };

        let available = snap.memory_total.saturating_sub(snap.memory_used);

        let info = column![
            info_row(t.in_use, format!("{} / {}", format_bytes(snap.memory_used), format_bytes(snap.memory_total)), p, self.ui_mono),
            info_row(t.available, format_bytes(available), p, self.ui_mono),
            info_row(t.usage, format!("{:.1}%", display_mem), p, self.ui_mono),
        ]
        .spacing(4);

        let bars = column![
            labeled_bar("RAM", snap.memory_used, snap.memory_total, p.green, p, self.ui_mono),
            labeled_bar("Swap", snap.swap_used, snap.swap_total, p.yellow, p, self.ui_mono),
        ]
        .spacing(6);

        // Process virtual memory total
        let total_virt: u64 = if let Some(snap) = &self.current {
            snap.processes.iter().map(|p| p.virtual_memory_bytes).sum()
        } else { 0 };

        let swap_info = column![
            info_row(t.swap_used, format!("{} / {}", format_bytes(snap.swap_used), format_bytes(snap.swap_total)), p, self.ui_mono),
            info_row(t.swap_usage, format!("{:.1}%", swap_pct), p, self.ui_mono),
            info_row(t.virtual_memory_total, format_bytes(total_virt), p, self.ui_mono),
        ]
        .spacing(4);

        let gc = GaugeColors {
            bg: p.panel_bg, label: p.label, text: p.text, bar_bg: p.bar_bg,
        };
        let mem_gauge: Element<Message> = Canvas::new(RadialGauge {
            value: self.anim_mem_pct,
            label: "RAM".into(),
            color: chart_color,
            colors: gc,
        })
        .width(Length::Fixed(120.0))
        .height(Length::Fixed(100.0))
        .into();

        panel(
            column![
                row![
                    mem_gauge,
                    column![mem_chart].width(Length::Fill),
                ].spacing(6).align_y(Alignment::Center),
                Space::with_height(8),
                bars,
                Space::with_height(8),
                section_title("RAM", p, self.ui_mono),
                info,
                Space::with_height(8),
                section_title(t.swap, p, self.ui_mono),
                swap_info,
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── Network Detail ──
    fn view_detail_network<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let cc = self.chart_colors();
        let rx_kb: Vec<f32> = self.live_buffer.iter().map(|p| p.net_rx as f32 / 1024.0).collect();
        let tx_kb: Vec<f32> = self.live_buffer.iter().map(|p| p.net_tx as f32 / 1024.0).collect();
        let max_kb = rx_kb.iter().chain(tx_kb.iter()).cloned().fold(0.001f32, f32::max);
        let (rx_data, tx_data, unit, y_max) = if max_kb >= 1024.0 {
            let rx_mb: Vec<f32> = rx_kb.iter().map(|v| v / 1024.0).collect();
            let tx_mb: Vec<f32> = tx_kb.iter().map(|v| v / 1024.0).collect();
            let max_mb = max_kb / 1024.0;
            (rx_mb, tx_mb, " MB/s", max_mb)
        } else {
            (rx_kb, tx_kb, " KB/s", max_kb)
        };
        let net_chart = make_chart(ChartCfg {
            title: t.network.into(),
            series: vec![
                (format!("{ICON_ARROW_DOWN} rx"), p.green, rx_data),
                (format!("{ICON_ARROW_UP} tx"), p.red, tx_data),
            ],
            y_min: 0.0, y_max, filled: true, height: 200.0, unit: unit.into(), colors: cc,
        });

        let totals = column![
            info_row(format!("{ICON_ARROW_DOWN} {}", t.receive), format!("{}/s", format_bytes(snap.net_rx_bytes)), p, self.ui_mono),
            info_row(format!("{ICON_ARROW_UP} {}", t.send), format!("{}/s", format_bytes(snap.net_tx_bytes)), p, self.ui_mono),
        ]
        .spacing(4);

        let text_c = p.text;
        let green = p.green;
        let red = p.red;
        let mut iface_items: Vec<Element<Message>> = Vec::new();
        for iface in &snap.net_interfaces {
            let item = row![
                text(&iface.name).size(11).color(text_c).width(140),
                text(format!("{ICON_ARROW_DOWN} {}", format_bytes(iface.rx_bytes))).size(11).font(self.ui_mono).color(green).width(110),
                text(format!("{ICON_ARROW_UP} {}", format_bytes(iface.tx_bytes))).size(11).font(self.ui_mono).color(red).width(110),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            iface_items.push(item.into());
        }

        panel(
            column![
                net_chart,
                Space::with_height(8),
                section_title(t.throughput, p, self.ui_mono),
                totals,
                Space::with_height(8),
                section_title(t.interfaces, p, self.ui_mono),
                Column::with_children(iface_items).spacing(3),
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── Disk Detail ──
    fn view_detail_disk<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let text_c = p.text;
        let label_c = p.label;
        let green = p.green;
        let border_c = p.border;
        let panel_bg = p.panel_bg;
        let sidebar_bg = p.sidebar_bg;

        let total_space: u64 = snap.disks.iter().map(|d| d.total).sum();
        let total_avail: u64 = snap.disks.iter().map(|d| d.available).sum();
        let total_used = total_space.saturating_sub(total_avail);
        let total_pct = if total_space > 0 { total_used as f64 / total_space as f64 * 100.0 } else { 0.0 };

        let summary = container(
            row![
                column![
                    text(format!("{} {}", snap.disks.len(), t.drives)).size(20).font(self.ui_mono).color(text_c),
                    text(format!("{:.1}% {}", total_pct, t.overall_usage)).size(11).font(self.ui_mono).color(label_c),
                ].spacing(4).width(Length::FillPortion(1)),
                column![
                    info_row(t.total_capacity, format_bytes(total_space), p, self.ui_mono),
                    info_row(t.total_used, format_bytes(total_used), p, self.ui_mono),
                    info_row(t.total_free, format_bytes(total_avail), p, self.ui_mono),
                ].spacing(4).width(Length::FillPortion(1)),
            ].spacing(20)
        )
        .padding(12)
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(sidebar_bg)),
            border: Border { color: border_c, width: 1.0, radius: 8.0.into() },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 4.0,
            },
            ..Default::default()
        });

        let mut disk_items: Vec<Element<Message>> = Vec::new();
        for d in &snap.disks {
            let used = d.total.saturating_sub(d.available);
            let pct = if d.total > 0 { used as f32 / d.total as f32 * 100.0 } else { 0.0 };
            let color = gradient_color(pct / 100.0, p);
            let bar_bg = p.bar_bg;

            let icon = if d.is_removable { ICON_USB } else { ICON_DISK };
            let disk_type = if d.name.contains("nvme") { "NVMe SSD" }
                else if d.name.contains("sd") { "SATA" }
                else { "Drive" };

            let disk_card = container(
                column![
                    row![
                        text(format!("{icon} {}", &d.mount)).size(14).color(text_c),
                        Space::with_width(Length::Fill),
                        text(format!("{} {ICON_BULLET} {}", &d.name, disk_type)).size(10).color(label_c),
                    ],
                    Space::with_height(6),
                    themed_bar(pct, color, bar_bg),
                    Space::with_height(6),
                    row![
                        text(format!("{:.1}%", pct)).size(14).font(self.ui_mono).color(color),
                        Space::with_width(Length::Fill),
                        text(format!("{} {}", format_bytes(used), t.used)).size(11).font(self.ui_mono).color(text_c),
                        Space::with_width(12),
                        text(format!("{} {}", format_bytes(d.available), t.free)).size(11).font(self.ui_mono).color(green),
                        Space::with_width(12),
                        text(format!("{} {}", format_bytes(d.total), t.total)).size(11).font(self.ui_mono).color(label_c),
                    ],
                    Space::with_height(8),
                    row![
                        column![
                            info_row(t.file_system, &d.fs_type, p, self.ui_mono),
                            info_row(t.mount_point, &d.mount, p, self.ui_mono),
                        ].spacing(3).width(Length::FillPortion(1)),
                        column![
                            info_row(t.device, &d.name, p, self.ui_mono),
                            info_row(t.type_label, if d.is_removable { t.removable } else { t.fixed }, p, self.ui_mono),
                        ].spacing(3).width(Length::FillPortion(1)),
                    ].spacing(20),
                ]
                .spacing(0)
            )
            .padding(12)
            .width(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(panel_bg)),
                border: Border { color: border_c, width: 1.0, radius: 8.0.into() },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 6.0,
                },
                ..Default::default()
            });
            disk_items.push(disk_card.into());
        }

        // Disk I/O
        let disk_io_info = column![
            info_row(format!("{ICON_ARROW_DOWN} {}", t.read), format!("{}/s", format_bytes(snap.disk_io.read_bytes)), p, self.ui_mono),
            info_row(format!("{ICON_ARROW_UP} {}", t.write), format!("{}/s", format_bytes(snap.disk_io.write_bytes)), p, self.ui_mono),
        ].spacing(4);

        let disk_title = format!("{ICON_DISK} {}", t.disk_drives);
        panel(
            column![
                section_title(&disk_title, p, self.ui_mono),
                summary,
                Space::with_height(8),
                section_title(t.io_throughput, p, self.ui_mono),
                disk_io_info,
                Space::with_height(8),
                Column::with_children(disk_items).spacing(8),
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── Temperature Detail ──
    fn view_detail_temp<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let text_c = p.text;
        let label_c = p.label;
        let green = p.green;
        let red = p.red;
        let yellow = p.yellow;
        let panel_bg = p.panel_bg;
        let bg = p.bg;

        let temp_title = format!("{ICON_TEMP} {}", t.temperatures);
        if snap.temperatures.is_empty() {
            return panel(
                column![
                    section_title(&temp_title, p, self.ui_mono),
                    text(t.no_sensors).size(12).font(self.ui_mono).color(label_c),
                ]
                .spacing(6)
                .into(),
                p,
            );
        }

        let mut temp_items: Vec<Element<Message>> = Vec::new();
        for (i, t) in snap.temperatures.iter().enumerate() {
            let color = if t.temp_c > 80.0 { red } else if t.temp_c > 60.0 { yellow } else { green };
            let row_bg = if i % 2 == 0 { panel_bg } else { bg };
            let temp_str = format_temp(t.temp_c, self.temp_celsius);
            let item = container(
                row![
                    text(&t.label).size(11).color(text_c).width(Length::Fill),
                    text(temp_str).size(11).font(self.ui_mono).color(color),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
            )
            .padding([4, 8])
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(row_bg)),
                ..Default::default()
            });
            temp_items.push(item.into());
        }

        let valid_temps: Vec<f32> = snap.temperatures.iter()
            .map(|t| t.temp_c)
            .filter(|&t| t > -30.0)
            .collect();
        let (min_t, max_t, avg_t) = if valid_temps.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let min = valid_temps.iter().cloned().fold(f32::MAX, f32::min);
            let max = valid_temps.iter().cloned().fold(f32::MIN, f32::max);
            let avg = valid_temps.iter().sum::<f32>() / valid_temps.len() as f32;
            (min, max, avg)
        };

        let summary = column![
            info_row(t.sensors, snap.temperatures.len().to_string(), p, self.ui_mono),
            info_row(t.minimum, format_temp(min_t, self.temp_celsius), p, self.ui_mono),
            info_row(t.maximum, format_temp(max_t, self.temp_celsius), p, self.ui_mono),
            info_row(t.average, format_temp(avg_t, self.temp_celsius), p, self.ui_mono),
        ]
        .spacing(4);

        let temp_overview_title = format!("{ICON_TEMP} {}", t.temperature_overview);
        panel(
            column![
                section_title(&temp_overview_title, p, self.ui_mono),
                summary,
                Space::with_height(8),
                section_title(t.all_sensors, p, self.ui_mono),
                Column::with_children(temp_items).spacing(0),
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── GPU Detail ──
    fn view_detail_gpu<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let text_c = p.text;
        let label_c = p.label;

        if snap.gpu.gpus.is_empty() {
            return panel(
                column![
                    section_title(format!("{ICON_GPU} {}", t.gpu), p, self.ui_mono),
                    text(t.no_gpu)
                        .size(12).font(self.ui_mono).color(label_c),
                ]
                .spacing(6)
                .into(),
                p,
            );
        }

        let mut gpu_items: Vec<Element<Message>> = Vec::new();
        for gpu in &snap.gpu.gpus {
            let mem_pct = if gpu.memory_total > 0 {
                gpu.memory_used as f32 / gpu.memory_total as f32 * 100.0
            } else { 0.0 };
            let util_color = gradient_color(gpu.utilization as f32 / 100.0, p);
            let _temp_color = if gpu.temperature > 80.0 { p.red }
                else if gpu.temperature > 60.0 { p.yellow }
                else { p.green };

            gpu_items.push(
                column![
                    text(&gpu.name).size(14).color(text_c),
                    Space::with_height(4),
                    info_row(t.utilization, format!("{}%", gpu.utilization), p, self.ui_mono),
                    info_row(t.temperature, format!("{:.0}°C", gpu.temperature), p, self.ui_mono),
                    info_row(t.vram, format!("{} / {}", format_bytes(gpu.memory_used), format_bytes(gpu.memory_total)), p, self.ui_mono),
                    info_row(t.vram_usage, format!("{:.1}%", mem_pct), p, self.ui_mono),
                    info_row(t.power, format!("{:.1}W", gpu.power_watts), p, self.ui_mono),
                    Space::with_height(4),
                    labeled_bar("Util", gpu.utilization as u64, 100, util_color, p, self.ui_mono),
                    labeled_bar("VRAM", gpu.memory_used, gpu.memory_total, p.magenta, p, self.ui_mono),
                ]
                .spacing(4)
                .into()
            );
        }

        panel(
            column![
                section_title(format!("{ICON_GPU} {}", t.gpu), p, self.ui_mono),
                Column::with_children(gpu_items).spacing(12),
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── PROCESSES TAB ──────────────────────────────────────────

    fn view_processes(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let t = self.t();
        let label_c = p.label;
        let accent = p.accent;
        let green = p.green;
        let yellow = p.yellow;
        let red = p.red;
        let panel_bg = p.panel_bg;
        let bg = p.bg;
        let border_c = p.border;
        let sidebar_bg = p.sidebar_bg;

        let Some(snap) = &self.current else {
            return container(
                text(format!("{ICON_LOADING} {}", t.collecting_data)).size(14).font(self.ui_mono).color(label_c)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        };

        let group_label = if self.process_grouped { t.grouped } else { t.all };
        let group_color = if self.process_grouped { accent } else { label_c };

        let filter_row = row![
            text(format!("{ICON_SEARCH} {}", t.filter)).size(11).font(self.ui_mono).color(label_c),
            Space::with_width(4),
            text_input(t.search, &self.process_filter)
                .on_input(Message::ProcessFilterChanged)
                .width(220),
            Space::with_width(12),
            button(text(format!("{ICON_BARS} {group_label}")).size(11).font(self.ui_mono).color(group_color))
                .on_press(Message::ToggleGrouped)
                .style(button::secondary)
                .padding([3, 10]),
            Space::with_width(Length::Fill),
            text(format!("{ICON_LIST} {} {}", snap.processes.len(), t.processes)).size(11).font(self.ui_mono).color(label_c),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .padding([6, 10]);

        let filter_lower = self.process_filter.to_lowercase();
        let filtered: Vec<_> = snap
            .processes
            .iter()
            .filter(|p| {
                filter_lower.is_empty()
                    || p.name.to_lowercase().contains(&filter_lower)
                    || p.cmd.iter().any(|c| c.to_lowercase().contains(&filter_lower))
            })
            .collect();

        let si = |col: ProcessSort| -> &str {
            if self.process_sort == col {
                if self.process_sort_asc { ICON_SORT_UP } else { ICON_SORT_DOWN }
            } else { "" }
        };

        let header = container(
            row![
                sort_btn(format!("PID {}", si(ProcessSort::Pid)), ProcessSort::Pid, 60, accent),
                text("PPID").size(11).color(accent).width(50),
                sort_btn(format!("{} {}", t.command, si(ProcessSort::Name)), ProcessSort::Name, 180, accent),
                sort_btn(format!("CPU% {}", si(ProcessSort::Cpu)), ProcessSort::Cpu, 70, accent),
                sort_btn(format!("{} {}", t.memory, si(ProcessSort::Memory)), ProcessSort::Memory, 90, accent),
                text("St").size(11).color(accent).width(25),
                text(format!("{ICON_THREAD} Thr")).size(11).color(accent).width(40),
                text(t.action).size(11).font(self.ui_mono).color(accent).width(60),
            ]
            .spacing(2)
        )
        .padding([4, 10])
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(sidebar_bg)),
            border: Border { color: border_c, width: 0.0, radius: 0.0.into() },
            ..Default::default()
        });

        let mut rows: Vec<Element<Message>> = Vec::new();

        if self.process_grouped {
            // SAFETY: libc::getuid() is a simple POSIX syscall that returns the real
            // user ID of the calling process. It is always safe to call, has no side
            // effects, cannot fail, and requires no special resources or permissions.
            // It is used here to separate user-owned processes from system processes.
            #[cfg(unix)]
            let current_uid = unsafe { libc::getuid() };
            // On Windows, metrics.rs sets uid=0 for user processes and uid=1
            // for system processes (SYSTEM/LOCAL SERVICE/NETWORK SERVICE).
            // current_uid=0 makes the grouping logic work correctly:
            // uid != 0 → System, is_desktop_app → Apps, else → Background.
            #[cfg(not(unix))]
            let current_uid = 0u32;

            let mut apps: Vec<_> = Vec::new();
            let mut background: Vec<_> = Vec::new();
            let mut system: Vec<_> = Vec::new();

            for proc in &filtered {
                if proc.uid != current_uid {
                    system.push(*proc);
                } else if proc.is_desktop_app {
                    apps.push(*proc);
                } else {
                    background.push(*proc);
                }
            }

            let sort_fn = |list: &mut Vec<&crate::metrics::ProcessInfo>| {
                match self.process_sort {
                    ProcessSort::Pid => list.sort_by_key(|p| p.pid),
                    ProcessSort::Name => list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                    ProcessSort::Cpu => list.sort_by(|a, b| a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap_or(std::cmp::Ordering::Equal)),
                    ProcessSort::Memory => list.sort_by_key(|p| p.memory_bytes),
                }
                if !self.process_sort_asc {
                    list.reverse();
                }
            };
            sort_fn(&mut apps);
            sort_fn(&mut background);
            sort_fn(&mut system);

            let mut row_idx = 0usize;
            let sections: Vec<(&str, &str, Color, &Vec<&crate::metrics::ProcessInfo>)> = vec![
                (ICON_APPS, t.applications, green, &apps),
                (ICON_BACKGROUND, t.background_processes, yellow, &background),
                (ICON_SYSTEM, t.system, red, &system),
            ];

            for (icon, label, color, list) in sections {
                if list.is_empty() { continue; }
                let hdr_bg = sidebar_bg;
                let section_hdr = container(
                    text(format!("{icon} {label} ({})", list.len())).size(11).font(self.ui_mono).color(color),
                )
                .padding([4, 10])
                .width(Length::Fill)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(hdr_bg)),
                    ..Default::default()
                });
                rows.push(section_hdr.into());

                for proc in list.iter() {
                    let row_bg = if row_idx.is_multiple_of(2) { panel_bg } else { bg };
                    rows.push(process_row(proc, row_bg, p, self.cpu_alert_threshold, self.ui_mono));
                    row_idx += 1;
                }
            }
        } else {
            let mut procs = filtered;
            match self.process_sort {
                ProcessSort::Pid => procs.sort_by_key(|p| p.pid),
                ProcessSort::Name => procs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                ProcessSort::Cpu => procs.sort_by(|a, b| a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap_or(std::cmp::Ordering::Equal)),
                ProcessSort::Memory => procs.sort_by_key(|p| p.memory_bytes),
            }
            if !self.process_sort_asc {
                procs.reverse();
            }
            for (i, proc) in procs.iter().take(self.process_limit).enumerate() {
                let row_bg = if i % 2 == 0 { panel_bg } else { bg };
                rows.push(process_row(proc, row_bg, p, self.cpu_alert_threshold, self.ui_mono));
            }
        }

        let table = Column::with_children(rows).spacing(0);
        let content = panel(
            column![filter_row, header, table].spacing(0).into(),
            p,
        );

        scrollable(column![content].padding(4)).into()
    }

    // ─── HISTORY TAB ────────────────────────────────────────────

    fn view_history(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let t = self.t();
        let cc = self.chart_colors();
        let label_c = p.label;
        let accent = p.accent;

        let mut range_btns: Vec<Element<Message>> = Vec::new();
        range_btns.push(text(format!("{ICON_CLOCK} {}", t.range)).size(11).font(self.ui_mono).color(label_c).into());
        range_btns.push(Space::with_width(4).into());
        for (i, (_, label)) in HISTORY_RANGES.iter().enumerate() {
            let is_active = self.history_range_idx == i;
            let color = if is_active { accent } else { label_c };
            let btn = button(text(*label).size(11).color(color))
                .on_press(Message::HistoryRangeSelected(i))
                .style(if is_active { button::primary } else { button::secondary })
                .padding([3, 10]);
            range_btns.push(btn.into());
        }

        // Export buttons
        range_btns.push(Space::with_width(Length::Fill).into());
        range_btns.push(
            button(text(format!("{ICON_EXPORT} CSV")).size(11).color(label_c))
                .on_press(Message::ExportCsv)
                .style(button::secondary)
                .padding([3, 10])
                .into()
        );
        range_btns.push(
            button(text(format!("{ICON_EXPORT} JSON")).size(11).color(label_c))
                .on_press(Message::ExportJson)
                .style(button::secondary)
                .padding([3, 10])
                .into()
        );

        let range_row = Row::with_children(range_btns).spacing(4).padding([6, 10]);

        if self.history_points.is_empty() {
            return panel(
                column![
                    range_row,
                    Space::with_height(20),
                    container(
                        text(format!("{ICON_HISTORY} {}", t.no_history_data)).size(13).font(self.ui_mono).color(label_c)
                    ).center_x(Length::Fill),
                    Space::with_height(20),
                ]
                .spacing(4).into(),
                p,
            );
        }

        const MAX_PTS: usize = 600;

        let cpu_data = downsample(
            &self.history_points.iter().map(|h| h.cpu).collect::<Vec<_>>(), MAX_PTS,
        );
        let cpu_chart = make_chart(ChartCfg {
            title: format!("{ICON_CPU} {}", t.cpu_history),
            series: vec![("CPU".into(), p.accent, cpu_data)],
            y_min: 0.0, y_max: 100.0, filled: true, height: 140.0, unit: "%".into(), colors: cc,
        });

        let mem_data = downsample(
            &self.history_points.iter().map(|h| {
                if h.mem_total > 0 { h.mem_used as f32 / h.mem_total as f32 * 100.0 } else { 0.0 }
            }).collect::<Vec<_>>(), MAX_PTS,
        );
        let mem_chart = make_chart(ChartCfg {
            title: format!("{ICON_MEMORY} {}", t.memory_history),
            series: vec![("RAM".into(), p.green, mem_data)],
            y_min: 0.0, y_max: 100.0, filled: true, height: 140.0, unit: "%".into(), colors: cc,
        });

        let rx_kb = downsample(
            &self.history_points.iter().map(|h| h.net_rx as f32 / 1024.0).collect::<Vec<_>>(), MAX_PTS,
        );
        let tx_kb = downsample(
            &self.history_points.iter().map(|h| h.net_tx as f32 / 1024.0).collect::<Vec<_>>(), MAX_PTS,
        );
        let hist_max_kb = rx_kb.iter().chain(tx_kb.iter()).cloned().fold(0.001f32, f32::max);
        let (h_rx, h_tx, h_unit, h_ymax) = if hist_max_kb >= 1024.0 {
            let rx_mb: Vec<f32> = rx_kb.iter().map(|v| v / 1024.0).collect();
            let tx_mb: Vec<f32> = tx_kb.iter().map(|v| v / 1024.0).collect();
            (rx_mb, tx_mb, " MB/s", hist_max_kb / 1024.0)
        } else {
            (rx_kb, tx_kb, " KB/s", hist_max_kb)
        };
        let net_chart = make_chart(ChartCfg {
            title: format!("{ICON_NETWORK} {}", t.network_history),
            series: vec![
                (format!("{ICON_ARROW_DOWN} rx"), p.green, h_rx),
                (format!("{ICON_ARROW_UP} tx"), p.red, h_tx),
            ],
            y_min: 0.0, y_max: h_ymax, filled: true, height: 140.0, unit: h_unit.into(), colors: cc,
        });

        let content = column![
            panel(column![range_row, cpu_chart].spacing(6).into(), p),
            panel(mem_chart, p),
            panel(net_chart, p),
        ]
        .spacing(4)
        .padding(4);

        scrollable(content).into()
    }
}

// ─── HELPER FUNCTIONS ────────────────────────────────────────────

fn gradient_color(t: f32, p: &Palette) -> Color {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        let f = t * 2.0;
        Color::from_rgb(
            p.green.r + (p.yellow.r - p.green.r) * f,
            p.green.g + (p.yellow.g - p.green.g) * f,
            p.green.b + (p.yellow.b - p.green.b) * f,
        )
    } else {
        let f = (t - 0.5) * 2.0;
        Color::from_rgb(
            p.yellow.r + (p.red.r - p.yellow.r) * f,
            p.yellow.g + (p.red.g - p.yellow.g) * f,
            p.yellow.b + (p.red.b - p.yellow.b) * f,
        )
    }
}

fn format_temp(temp_c: f32, celsius: bool) -> String {
    if temp_c < -30.0 {
        "N/A".to_string()
    } else if celsius {
        format!("{:.0}\u{00b0}C", temp_c)
    } else {
        format!("{:.0}\u{00b0}F", temp_c * 9.0 / 5.0 + 32.0)
    }
}

fn themed_bar(value: f32, color: Color, bar_bg: Color) -> Element<'static, Message> {
    // Enhanced bar with more rounded corners and subtle lighter tint
    let bar_color = Color::from_rgba(
        (color.r * 0.9 + 0.1).min(1.0),
        (color.g * 0.9 + 0.1).min(1.0),
        (color.b * 0.9 + 0.1).min(1.0),
        color.a,
    );
    progress_bar(0.0..=100.0, value)
        .width(Length::Fill)
        .style(move |_: &Theme| progress_bar::Style {
            background: Background::Color(bar_bg),
            bar: Background::Color(bar_color),
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 5.0.into() },
        })
        .into()
}

struct ChartCfg {
    title: String,
    series: Vec<(String, Color, Vec<f32>)>,
    y_min: f32,
    y_max: f32,
    filled: bool,
    height: f32,
    unit: String,
    colors: ChartColors,
}

fn make_chart(cfg: ChartCfg) -> Element<'static, Message> {
    let chart = LineChart {
        series: cfg.series,
        y_min: cfg.y_min,
        y_max: cfg.y_max,
        title: cfg.title,
        filled: cfg.filled,
        unit: cfg.unit,
        colors: cfg.colors,
        show_avg: true,
    };
    Canvas::new(chart)
        .width(Length::Fill)
        .height(Length::Fixed(cfg.height))
        .into()
}

fn sidebar_item<'a>(
    label: impl ToString,
    value: impl ToString,
    color: Color,
    target: OverviewPanel,
    current: OverviewPanel,
    p: &Palette,
    mono_font: iced::Font,
) -> Element<'a, Message> {
    let is_active = target == current;
    let sidebar_bg = p.sidebar_bg;
    // Slightly lighten sidebar_bg for active state
    let active_bg = Color::from_rgb(
        (sidebar_bg.r + 0.06).min(1.0),
        (sidebar_bg.g + 0.06).min(1.0),
        (sidebar_bg.b + 0.06).min(1.0),
    );
    let hover_bg = Color::from_rgb(
        (sidebar_bg.r + 0.03).min(1.0),
        (sidebar_bg.g + 0.03).min(1.0),
        (sidebar_bg.b + 0.03).min(1.0),
    );
    let bg = if is_active { active_bg } else { sidebar_bg };
    let border_color = if is_active { color } else { Color::TRANSPARENT };
    let label_c = p.label;
    let text_c = p.text;
    let label = label.to_string();
    let value = value.to_string();

    let content = column![
        text(label).size(12).color(if is_active { color } else { label_c }),
        text(value).size(13).font(mono_font).color(if is_active { text_c } else { label_c }),
    ]
    .spacing(2);

    button(content)
        .on_press(Message::OverviewSection(target))
        .width(Length::Fill)
        .padding([8, 10])
        .style(move |_: &Theme, status| {
            let bg_final = match status {
                button::Status::Hovered => if is_active { active_bg } else { hover_bg },
                button::Status::Pressed => active_bg,
                _ => bg,
            };
            button::Style {
                background: Some(Background::Color(bg_final)),
                text_color: text_c,
                border: Border {
                    color: border_color,
                    width: if is_active { 2.5 } else { 0.0 },
                    radius: 6.0.into(),
                },
                shadow: if is_active {
                    Shadow {
                        color: Color::from_rgba(color.r, color.g, color.b, 0.2),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 4.0,
                    }
                } else {
                    Shadow::default()
                },
            }
        })
        .into()
}

fn settings_sidebar_item(
    label: impl ToString,
    target: SettingsPanel,
    current: SettingsPanel,
    p: &Palette,
    mono_font: iced::Font,
) -> Element<'static, Message> {
    let is_active = target == current;
    let accent = p.accent;
    let bg = if is_active { accent } else { Color::TRANSPARENT };
    let hover_bg = Color::from_rgba(accent.r, accent.g, accent.b, 0.3);
    let text_color = if is_active { p.text } else { p.label };
    let text_c = p.text;

    button(text(label.to_string()).size(12).font(mono_font).color(text_color))
        .on_press(Message::SettingsPanelSelected(target))
        .width(Length::Fill)
        .padding([8, 12])
        .style(move |_: &Theme, status| {
            let bg_final = match status {
                button::Status::Hovered => if is_active { accent } else { hover_bg },
                button::Status::Pressed => accent,
                _ => bg,
            };
            button::Style {
                background: Some(Background::Color(bg_final)),
                text_color: text_c,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 6.0.into(),
                },
                shadow: if is_active {
                    Shadow {
                        color: Color::from_rgba(accent.r, accent.g, accent.b, 0.25),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 4.0,
                    }
                } else {
                    Shadow::default()
                },
            }
        })
        .into()
}

fn collapsible_section<'a>(
    section: SettingsSection,
    title: impl ToString,
    description: impl ToString,
    collapsed: bool,
    content: Element<'a, Message>,
    p: &Palette,
    mono_font: iced::Font,
) -> Element<'a, Message> {
    let title_str = title.to_string();
    let desc_str = description.to_string();
    let chevron = if collapsed { ICON_CHEVRON_RIGHT } else { ICON_CHEVRON_DOWN };
    let text_c = p.text;
    let label_c = p.label;
    let panel_bg = p.panel_bg;
    let border_c = p.border;

    let hover_bg = Color::from_rgb(
        (panel_bg.r + 0.02).min(1.0),
        (panel_bg.g + 0.02).min(1.0),
        (panel_bg.b + 0.02).min(1.0),
    );
    let header = button(
        row![
            text(title_str).size(13).font(mono_font).color(text_c),
            Space::with_width(Length::Fill),
            text(chevron).size(12).color(label_c),
        ]
        .align_y(Alignment::Center)
    )
    .on_press(Message::ToggleSection(section))
    .width(Length::Fill)
    .padding([10, 14])
    .style(move |_: &Theme, status| {
        let bg_final = match status {
            button::Status::Hovered => hover_bg,
            _ => panel_bg,
        };
        button::Style {
            background: Some(Background::Color(bg_final)),
            text_color: text_c,
            border: Border {
                color: border_c,
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 3.0,
            },
        }
    });

    if collapsed {
        return header.into();
    }

    let mut body_items: Vec<Element<Message>> = Vec::new();
    if !desc_str.is_empty() {
        body_items.push(
            text(desc_str).size(10).color(label_c).into()
        );
        body_items.push(Space::with_height(10).into());
    }
    body_items.push(content);

    let body = container(
        Column::with_children(body_items).spacing(0)
    )
    .padding([10, 14])
    .width(Length::Fill)
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(panel_bg)),
        border: Border {
            color: border_c,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    });

    column![header, body].spacing(0).into()
}

fn info_row<'a>(label: impl ToString, value: impl ToString, p: &Palette, mono_font: iced::Font) -> Element<'a, Message> {
    let l = format!("{}:", label.to_string());
    let v = value.to_string();
    let label_c = p.label;
    let text_c = p.text;
    row![
        text(l).size(11).color(label_c).width(120),
        text(v).size(11).font(mono_font).color(text_c),
    ]
    .spacing(8)
    .into()
}

fn process_row<'a>(proc: &crate::metrics::ProcessInfo, bg: Color, p: &'a Palette, cpu_threshold: f32, mono_font: iced::Font) -> Element<'a, Message> {
    let cpu_color = gradient_color(proc.cpu_usage / 100.0, p);
    let pid = proc.pid;
    let pid_str = pid.to_string();
    let name = proc.name.clone();
    let cpu = format!("{:.1}%", proc.cpu_usage);
    let mem = format_bytes(proc.memory_bytes);
    let label_c = p.label;
    let text_c = p.text;
    let accent = p.accent;

    // Command-line tooltip (truncated) — avoid allocation if no args
    let cmd_str: String = if proc.cmd.len() > 1 {
        let mut args = String::new();
        for (i, arg) in proc.cmd[1..].iter().enumerate() {
            if i > 0 { args.push(' '); }
            if args.len() + arg.len() > 60 {
                args.push_str(&arg[..60_usize.saturating_sub(args.len()).min(arg.len())]);
                args.push('\u{2026}');
                break;
            }
            args.push_str(arg);
        }
        args
    } else {
        String::new()
    };

    // Parent PID display
    let ppid_str = proc.parent_pid.map(|p| p.to_string()).unwrap_or_default();

    // Highlight row if CPU exceeds threshold
    let row_bg = if proc.cpu_usage >= cpu_threshold {
        Color::from_rgba(p.red.r, p.red.g, p.red.b, 0.1)
    } else {
        bg
    };

    let kill_btn = button(
        text(ICON_KILL).size(10).color(label_c)
    )
    .on_press(Message::KillProcess(pid))
    .style(button::text)
    .padding([1, 4]);

    let name_col: Element<Message> = if cmd_str.is_empty() {
        text(name.clone()).size(11).color(text_c).width(180).into()
    } else {
        tooltip(
            text(name.clone()).size(11).color(text_c).width(180),
            text(cmd_str).size(9).color(text_c),
            tooltip::Position::Top,
        )
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(p.panel_bg)),
            border: Border {
                color: accent,
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: Some(text_c),
            shadow: Shadow::default(),
        })
        .padding(6)
        .into()
    };

    container(
        row![
            text(pid_str).size(11).font(mono_font).color(label_c).width(60),
            text(ppid_str).size(10).font(mono_font).color(label_c).width(50),
            name_col,
            text(cpu).size(11).font(mono_font).color(cpu_color).width(70),
            text(mem).size(11).font(mono_font).color(accent).width(90),
            text(String::from(proc.status)).size(11).font(mono_font).color(match proc.status {
                'R' => p.green,
                'Z' => p.red,
                'D' => p.yellow,
                _ => label_c,
            }).width(25),
            text(proc.thread_count.to_string()).size(11).font(mono_font).color(label_c).width(40),
            kill_btn,
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .padding([2, 10])
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(row_bg)),
        ..Default::default()
    })
    .into()
}

fn panel<'a>(content: Element<'a, Message>, p: &Palette) -> Element<'a, Message> {
    let panel_bg = p.panel_bg;
    let border_c = p.border;
    container(content)
        .width(Length::Fill)
        .padding(10)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(panel_bg)),
            border: Border {
                color: border_c,
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            ..Default::default()
        })
        .into()
}

fn panel_bg<'a>(content: Element<'a, Message>, bg: Color, border_c: Color) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: border_c,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn menu_tab(label: impl ToString, tab: Tab, current: Tab, p: &Palette, mono_font: iced::Font) -> Element<'static, Message> {
    let is_active = tab == current;
    let accent = p.accent;
    let label_c = p.label;
    let text_c = p.text;
    let color = if is_active { accent } else { label_c };
    let hover_color = Color::from_rgba(accent.r, accent.g, accent.b, 0.15);
    button(text(label.to_string()).size(12).font(mono_font).color(color))
        .on_press(Message::TabSelected(tab))
        .padding([4, 14])
        .style(move |_: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => hover_color,
                button::Status::Pressed => Color::from_rgba(accent.r, accent.g, accent.b, 0.25),
                _ => if is_active { Color::from_rgba(accent.r, accent.g, accent.b, 0.1) } else { Color::TRANSPARENT },
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: text_c,
                border: Border {
                    color: if is_active { accent } else { Color::TRANSPARENT },
                    width: 0.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn section_title(label: impl ToString, p: &Palette, mono_font: iced::Font) -> Element<'static, Message> {
    let accent = p.accent;
    text(label.to_string()).size(11).font(mono_font).color(accent).into()
}

fn labeled_bar(
    label: &str,
    used: u64,
    total: u64,
    color: Color,
    p: &Palette,
    mono_font: iced::Font,
) -> Element<'static, Message> {
    if total == 0 {
        return row![].into();
    }
    let pct = used as f32 / total as f32 * 100.0;
    let label_c = p.label;
    let bar_bg = p.bar_bg;
    row![
        text(format!("{label}:")).size(11).color(label_c).width(60),
        themed_bar(pct, color, bar_bg),
        text(format!("{}/{}", format_bytes(used), format_bytes(total)))
            .size(11)
            .font(mono_font)
            .color(color)
            .width(150),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

fn sort_btn(label: String, col: ProcessSort, width: u16, accent: Color) -> Element<'static, Message> {
    button(text(label).size(11).color(accent))
        .on_press(Message::SortBy(col))
        .style(button::text)
        .padding([2, 4])
        .width(width)
        .into()
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TiB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn format_duration(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn downsample(data: &[f32], max_points: usize) -> Vec<f32> {
    let n = data.len();
    if n <= max_points {
        return data.to_vec();
    }
    let bucket_size = n as f64 / max_points as f64;
    let mut out = Vec::with_capacity(max_points);
    for i in 0..max_points {
        let start = (i as f64 * bucket_size) as usize;
        let end = (((i + 1) as f64 * bucket_size) as usize).min(n);
        let peak = data[start..end]
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        out.push(peak);
    }
    out
}

fn make_threshold_buttons<'a>(
    current: f32,
    options: &[f32],
    on_press: impl Fn(f32) -> Message + 'a,
    accent: Color,
    label_c: Color,
    mono_font: iced::Font,
) -> Element<'a, Message> {
    let mut btns: Vec<Element<Message>> = Vec::new();
    for &val in options {
        let is_active = (current - val).abs() < 0.5;
        let color = if is_active { accent } else { label_c };
        let btn = button(
            text(format!("{:.0}%", val)).size(11).font(mono_font).color(color)
        )
        .on_press(on_press(val))
        .style(if is_active { button::primary } else { button::secondary })
        .padding([4, 10]);
        btns.push(btn.into());
    }
    Row::with_children(btns).spacing(4).into()
}
