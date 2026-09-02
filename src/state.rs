//! The application state, and everything that derives from it.

use iced::keyboard;
use iced::{Color, Subscription, Theme};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use crate::chart::ChartColors;
use crate::history::History;
use crate::i18n::{Language, Strings};
use crate::icons::*;
use crate::message::*;
use crate::metrics::{Collector, LivePoint, Snapshot};
use crate::preferences::Preferences;
use crate::ringbuf::RingBuffer;
use crate::theme::{build_palette, AccentChoice, Palette, ThemeChoice};
use crate::{DEJAVU_FONT, DYSLEXIC_FONT, NERD_FONT_MONO, NOTO_SANS_FONT, SARASA_FONT};

/// Returns the best available monospace font for a given language's script.
/// The application font, and the only place it is decided.
///
/// The dyslexia font is a whole-application swap, not a per-widget option, so
/// it wins over the script-specific choice below. It was previously stored and
/// toggled but never consulted, which made the accessibility setting a no-op.
pub(crate) fn app_font(lang: Language, dyslexic: bool) -> iced::Font {
    if dyslexic {
        return DYSLEXIC_FONT;
    }
    font_for_lang(lang)
}

pub(crate) fn font_for_lang(lang: Language) -> iced::Font {
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
pub(crate) fn has_native_font(lang: Language) -> bool {
    !matches!(
        lang,
        Language::He
            | Language::Bn
            | Language::Pa
            | Language::Ta
            | Language::Te
            | Language::Th
            | Language::Am
    )
}

/// Detect if the system prefers dark mode.
pub(crate) fn system_prefers_dark() -> bool {
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
/// What "follow the system" resolves to. Catppuccin's two ends, because they
/// are the default theme's own family — switching on the system preference
/// should not also change which theme the user is looking at.
pub(crate) fn auto_theme_choice() -> ThemeChoice {
    if system_prefers_dark() {
        ThemeChoice::new("catppuccin", "mocha")
    } else {
        ThemeChoice::new("catppuccin", "latte")
    }
}

/// The shared widgets read the active theme, accent and locale from process
/// globals rather than from arguments — a widget cannot reach the host's state.
/// Every place Digger changes one of the three calls this, so the pickers never
/// draw a theme the program is not actually using.
pub(crate) fn sync_shared_state(theme: &ThemeChoice, accent: &AccentChoice, language: Language) {
    colony_ui::set_active_theme(&theme.family, &theme.variant);
    colony_ui::set_active_accent(accent.key().and_then(colony_ui::accent_key_to_color));
    // colony-ui ships English and French; Digger ships twenty-five. Anything
    // else falls back to English, which is what the theme names were before
    // this — they were hardcoded English strings.
    colony_ui::i18n::set_locale(match language {
        Language::Fr => colony_ui::i18n::Locale::Fr,
        _ => colony_ui::i18n::Locale::En,
    });
}

/// Digger's fonts, in the shape the shared widgets expect.
pub(crate) fn typography_for(language: Language, dyslexic: bool) -> colony_ui::Typography {
    let base = app_font(language, dyslexic);
    colony_ui::Typography {
        scale: 1.0,
        regular: base,
        medium: base,
        bold: base,
    }
}

pub(crate) fn send_notification(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .appname("Digger")
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();
}

// ─── ANIMATION CONSTANTS ────────────────────────────────────────
/// The About page used to hardcode "0.1.0" and "Iced 0.13 + Rust", both of
/// which had gone stale. The version now comes from Cargo; this one still has
/// to be written down, but next to the dependency it describes.
pub(crate) const ICED_VERSION: &str = "Iced 0.14 + Rust";

pub(crate) const ANIM_TICK_MS: u64 = 33; // ~30fps for animations
pub(crate) const TWEEN_SPEED: f32 = 0.12; // lerp factor per animation tick
pub(crate) const PULSE_SPEED: f32 = 0.05; // pulse cycle speed

pub(crate) const EVENT_LOG_MAX: usize = 100;
pub(crate) const HISTORY_RELOAD_INTERVAL_SECS: f64 = 10.0;

pub(crate) const HISTORY_RANGES: &[(f64, &str)] = &[
    (60.0, "1m"),
    (300.0, "5m"),
    (900.0, "15m"),
    (3600.0, "1h"),
    (86400.0, "24h"),
];

pub(crate) const REFRESH_OPTIONS: &[u64] = &[1, 2, 5];

// ─── EVENT LOG ──────────────────────────────────────────────────

/// An event logged by the anomaly detection system.
#[derive(Clone, Debug)]
pub(crate) struct LogEvent {
    pub(crate) timestamp: Arc<str>,
    pub(crate) icon: &'static str,
    pub(crate) message: String,
    pub(crate) severity: EventSeverity,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum EventSeverity {
    Info,
    Warning,
    Critical,
}

/// Compute a heartbeat BPM (80–160) based on system load.
/// Resting heart rate is 80 BPM; CPU and memory usage increase it.
pub(crate) fn compute_heartbeat_bpm(cpu: f32, mem_pct: f32) -> f32 {
    (80.0 + cpu * 0.45 + mem_pct * 0.35).clamp(80.0, 160.0)
}

/// Dynamic saturation: low usage → desaturated, high usage → vivid color
pub(crate) fn dynamic_color(base: Color, intensity: f32) -> Color {
    // intensity: 0.0 to 1.0
    let t = intensity.clamp(0.0, 1.0);
    let gray = 0.5;
    Color::from_rgb(
        gray + (base.r - gray) * (0.3 + 0.7 * t),
        gray + (base.g - gray) * (0.3 + 0.7 * t),
        gray + (base.b - gray) * (0.3 + 0.7 * t),
    )
}

// ─── APP STATE ──────────────────────────────────────────────────

pub struct Digger {
    pub(crate) collector: Collector,
    pub(crate) history: History,
    pub(crate) current: Option<Arc<Snapshot>>,
    pub(crate) live_buffer: RingBuffer<LivePoint>,
    pub(crate) live_max: usize,
    pub(crate) tab: Tab,
    pub(crate) overview_panel: OverviewPanel,
    pub(crate) process_filter: String,
    pub(crate) process_sort: ProcessSort,
    pub(crate) process_sort_asc: bool,
    pub(crate) process_grouped: bool,
    pub(crate) history_range_idx: usize,
    pub(crate) history_points: Vec<crate::history::HistoryPoint>,
    // Settings
    pub(crate) show_settings: bool,
    pub(crate) settings_panel: SettingsPanel,
    pub(crate) refresh_interval_secs: u64,
    pub(crate) temp_celsius: bool,
    pub(crate) collapsed_sections: HashSet<SettingsSection>,
    // Theme
    pub(crate) theme_variant: ThemeChoice,
    pub(crate) accent_color: AccentChoice,
    pub(crate) pal: Palette,
    // Language
    pub(crate) language: Language,
    /// Monospace font for the current language's script.
    // New configurable fields
    pub(crate) process_limit: usize,
    pub(crate) use_dyslexic_font: bool,
    pub(crate) high_contrast: bool,
    pub(crate) reduced_motion: bool,
    pub(crate) font_scale: f32,
    pub(crate) text_scale: f32,
    pub(crate) retention_hours: u64,
    pub(crate) cpu_alert_threshold: f32,
    pub(crate) mem_alert_threshold: f32,
    // Status message for user feedback
    pub(crate) status_message: Option<String>,
    // ─── Health & Events ───
    /// Health score 0–100 (higher is better)
    pub(crate) health_score: f32,
    /// Recent event log entries (bounded VecDeque, opt #5)
    pub(crate) event_log: VecDeque<LogEvent>,
    /// Previous CPU reading for spike detection
    pub(crate) prev_cpu: f32,
    /// Previous memory % for leak detection
    pub(crate) prev_mem_pct: f32,
    // ─── Animation state ───
    /// Smoothly interpolated CPU usage for display
    pub(crate) anim_cpu: f32,
    /// Smoothly interpolated memory percentage for display
    pub(crate) anim_mem_pct: f32,
    /// Smoothly interpolated per-core CPU values
    pub(crate) anim_cores: Vec<f32>,
    /// Pulse phase for critical alerts (0.0 → 2*PI cycle)
    pub(crate) pulse_phase: f32,
    /// Heart beat phase (0.0 → 2*PI), advances based on BPM
    pub(crate) heart_phase: f32,
    /// Previous tab (to detect page transitions)
    pub(crate) prev_tab: Tab,
    /// Previous settings visibility
    pub(crate) prev_show_settings: bool,
    /// Opt #7: Timestamp of last history reload to throttle SQL queries.
    pub(crate) history_last_reload: f64,
    /// Opt #10: Pending snapshots for batched SQLite inserts.
    pub(crate) pending_snapshots: Vec<Arc<Snapshot>>,
    /// Opt #10: Timestamp of last DB flush.
    pub(crate) last_db_flush: f64,
    // ─── Cached UI strings (avoid format! every frame) ───
    pub(crate) cached_tab_overview: String,
    pub(crate) cached_tab_processes: String,
    pub(crate) cached_tab_history: String,
    pub(crate) cached_tab_events: String,
    pub(crate) cached_digger_label: String,
    pub(crate) cached_digger_label_settings: String,
    /// What the shared widgets need to know about Digger's text. They cannot
    /// reach this state, so it is passed in on every call.
    pub(crate) typo: colony_ui::Typography,
}

impl Default for Digger {
    fn default() -> Self {
        Self::new()
    }
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

        let mut app = Self {
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
                auto_theme_choice()
            } else {
                prefs.theme.clone()
            },
            accent_color: prefs.accent.clone(),
            language: prefs.language,
            pal: build_palette(
                &if prefs.auto_theme {
                    auto_theme_choice()
                } else {
                    prefs.theme.clone()
                },
                &prefs.accent,
                prefs.high_contrast,
            ),
            process_limit: prefs.process_limit,
            use_dyslexic_font: prefs.use_dyslexic_font,
            high_contrast: prefs.high_contrast,
            reduced_motion: prefs.reduced_motion,
            font_scale: prefs.font_scale,
            text_scale: prefs.text_scale,
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
            pulse_phase: 0.0,
            heart_phase: 0.0,
            prev_tab: Tab::Overview,
            prev_show_settings: false,
            history_last_reload: 0.0,
            pending_snapshots: Vec::new(),
            last_db_flush: 0.0,
            // Cached UI strings
            cached_tab_overview: format!(
                "{ICON_OVERVIEW}  {}",
                prefs.language.strings().tab_overview
            ),
            cached_tab_processes: format!(
                "{ICON_PROCESSES}  {}",
                prefs.language.strings().tab_processes
            ),
            cached_tab_history: format!("{ICON_HISTORY}  {}", prefs.language.strings().tab_history),
            cached_tab_events: format!("{ICON_LOG}  {}", prefs.language.strings().tab_events),
            cached_digger_label: format!("{ICON_DIGGER} Digger"),
            cached_digger_label_settings: format!("{ICON_DIGGER} Digger  {ICON_CLOSE}"),
            typo: typography_for(prefs.language, prefs.use_dyslexic_font),
        };
        app.refresh_appearance();
        app
    }

    /// Recompute everything that depends on the theme, the accent, the language
    /// or the dyslexia font, and push the same three into colony-ui's globals so
    /// the shared widgets draw what Digger is actually using.
    ///
    /// One method rather than four scattered assignments: the previous shape is
    /// how the dyslexia font came to be persisted but never applied.
    pub(crate) fn refresh_appearance(&mut self) {
        self.pal = build_palette(&self.theme_variant, &self.accent_color, self.high_contrast);
        self.typo = typography_for(self.language, self.use_dyslexic_font);
        // The two size preferences multiply — see design/typography.md.
        self.typo.scale = self.font_scale * self.text_scale;
        colony_ui::set_high_contrast(self.high_contrast);
        sync_shared_state(&self.theme_variant, &self.accent_color, self.language);
    }

    /// The opacity of a pulsing critical indicator.
    ///
    /// Flat when the user has asked for reduced motion — and flat at full
    /// opacity rather than at the bottom of the pulse, so silencing the
    /// animation does not also dim the thing it was drawing attention to.
    pub(crate) fn pulse_opacity(&self) -> f32 {
        if self.reduced_motion {
            1.0
        } else {
            self.pulse_opacity()
        }
    }

    /// Get the current translation strings.
    pub(crate) fn t(&self) -> &'static Strings {
        self.language.strings()
    }

    /// Rebuild cached tab strings when language changes.
    pub(crate) fn rebuild_cached_strings(&mut self) {
        let t = self.language.strings();
        self.cached_tab_overview = format!("{ICON_OVERVIEW}  {}", t.tab_overview);
        self.cached_tab_processes = format!("{ICON_PROCESSES}  {}", t.tab_processes);
        self.cached_tab_history = format!("{ICON_HISTORY}  {}", t.tab_history);
        self.cached_tab_events = format!("{ICON_LOG}  {}", t.tab_events);
    }

    pub fn title(&self) -> String {
        String::from("Digger")
    }

    pub fn theme(&self) -> Theme {
        if self.theme_variant.is_light() {
            Theme::Light
        } else {
            Theme::Dark
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let data_tick = iced::time::every(Duration::from_secs(self.refresh_interval_secs))
            .map(|_| Message::Tick);
        let anim_tick =
            iced::time::every(Duration::from_millis(ANIM_TICK_MS)).map(|_| Message::AnimTick);
        // 0.14 dropped the on_key_press helper: listen() yields every keyboard
        // event and the caller picks. Same behaviour, one more line.
        let keys = keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed { key, modifiers, .. } => {
                Some(Message::KeyPressed(key, modifiers))
            }
            _ => None,
        });
        Subscription::batch([data_tick, anim_tick, keys])
    }

    pub(crate) fn save_prefs(&self) {
        let prefs = Preferences {
            theme: self.theme_variant.clone(),
            accent: self.accent_color.clone(),
            refresh_interval_secs: self.refresh_interval_secs,
            temp_celsius: self.temp_celsius,
            process_limit: self.process_limit,
            live_buffer_size: self.live_max,
            retention_hours: self.retention_hours,
            cpu_alert_threshold: self.cpu_alert_threshold,
            mem_alert_threshold: self.mem_alert_threshold,
            use_dyslexic_font: self.use_dyslexic_font,
            high_contrast: self.high_contrast,
            reduced_motion: self.reduced_motion,
            font_scale: self.font_scale,
            text_scale: self.text_scale,
            process_grouped: self.process_grouped,
            process_sort: match self.process_sort {
                ProcessSort::Pid => "pid",
                ProcessSort::Name => "name",
                ProcessSort::Cpu => "cpu",
                ProcessSort::Memory => "memory",
            }
            .into(),
            process_sort_asc: self.process_sort_asc,
            auto_theme: false, // When saving manually, auto is off
            language: self.language,
        };
        prefs.save();
    }

    pub(crate) fn chart_colors(&self) -> ChartColors {
        ChartColors {
            bg: self.pal.panel_bg,
            border: self.pal.border,
            grid: self.pal.grid,
            label: self.pal.label,
            text: self.pal.text,
        }
    }

    // ─── MAIN VIEW ──────────────────────────────────────────────
}
