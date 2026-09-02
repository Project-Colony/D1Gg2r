//! Every message the app can receive, and the enums they carry.

use crate::i18n::Language;
use iced::keyboard;

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
    /// Catalog keys rather than an enum: the picker renders every family
    /// colony-ui ships, so the set is not known at compile time.
    SetTheme {
        family: String,
        variant: String,
    },
    /// `None` is "follow the theme", never a colour value.
    SetAccent(Option<String>),
    ToggleDyslexicFont,
    ToggleHighContrast,
    ToggleReducedMotion,
    SetFontScale(usize),
    SetTextScale(usize),
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
    // Appearance
    Typography,
    // Accessibility
    Fonts,
    Vision,
    Motion,
    Reading,
    // About
    Version,
    FontInfo,
    SystemInfo,
}
