use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::i18n::Language;
use crate::theme::{AccentColor, ThemeVariant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub theme: ThemeVariant,
    pub accent: AccentColor,
    pub refresh_interval_secs: u64,
    pub temp_celsius: bool,
    /// Maximum number of processes displayed in the process list.
    #[serde(default = "default_process_limit")]
    pub process_limit: usize,
    /// Number of live data points kept in the rolling chart buffer.
    #[serde(default = "default_live_buffer_size")]
    pub live_buffer_size: usize,
    /// History retention in hours (pruned periodically).
    #[serde(default = "default_retention_hours")]
    pub retention_hours: u64,
    /// CPU usage threshold (%) for alert highlighting.
    #[serde(default = "default_cpu_alert_threshold")]
    pub cpu_alert_threshold: f32,
    /// Memory usage threshold (%) for alert highlighting.
    #[serde(default = "default_mem_alert_threshold")]
    pub mem_alert_threshold: f32,
    /// Whether to use the OpenDyslexic font.
    #[serde(default)]
    pub use_dyslexic_font: bool,
    /// Whether the process list is grouped (Apps/Background/System).
    #[serde(default)]
    pub process_grouped: bool,
    /// Process sort column: "pid", "name", "cpu", "memory".
    #[serde(default = "default_process_sort")]
    pub process_sort: String,
    /// Whether process sort is ascending.
    #[serde(default)]
    pub process_sort_asc: bool,
    /// Auto-detect system dark/light theme.
    #[serde(default)]
    pub auto_theme: bool,
    /// Interface language.
    #[serde(default)]
    pub language: Language,
}

fn default_process_limit() -> usize { 200 }
const MAX_PROCESS_LIMIT: usize = 5000;
const REFRESH_OPTIONS: &[u64] = &[1, 2, 5];
fn default_live_buffer_size() -> usize { 120 }
fn default_retention_hours() -> u64 { 24 }
fn default_cpu_alert_threshold() -> f32 { 90.0 }
fn default_mem_alert_threshold() -> f32 { 90.0 }
fn default_process_sort() -> String { "cpu".into() }

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemeVariant::CatppuccinMocha,
            accent: AccentColor::Blue,
            refresh_interval_secs: 1,
            temp_celsius: true,
            process_limit: default_process_limit(),
            live_buffer_size: default_live_buffer_size(),
            retention_hours: default_retention_hours(),
            cpu_alert_threshold: default_cpu_alert_threshold(),
            mem_alert_threshold: default_mem_alert_threshold(),
            use_dyslexic_font: false,
            process_grouped: false,
            process_sort: default_process_sort(),
            process_sort_asc: false,
            auto_theme: false,
            language: Language::default(),
        }
    }
}

impl Preferences {
    /// Config directory: Windows → AppData/Local/Colony/Digger/
    /// Linux → ~/.config/Colony/Digger/
    fn config_dir() -> PathBuf {
        dirs::config_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Colony")
            .join("Digger")
    }

    fn config_path() -> PathBuf {
        Self::config_dir().join("preferences.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match fs::read_to_string(&path) {
            Ok(contents) => {
                let mut prefs: Self = serde_json::from_str(&contents).unwrap_or_else(|e| {
                    eprintln!("[digger] Invalid preferences file, using defaults: {e}");
                    Self::default()
                });
                prefs.sanitize();
                prefs
            }
            Err(_) => Self::default(),
        }
    }

    /// Clamp all numeric fields to valid ranges.
    fn sanitize(&mut self) {
        self.process_limit = self.process_limit.clamp(10, MAX_PROCESS_LIMIT);
        self.live_buffer_size = self.live_buffer_size.clamp(30, 1000);
        self.retention_hours = self.retention_hours.clamp(1, 168); // 1h to 7 days
        self.cpu_alert_threshold = self.cpu_alert_threshold.clamp(10.0, 100.0);
        self.mem_alert_threshold = self.mem_alert_threshold.clamp(10.0, 100.0);
        if !REFRESH_OPTIONS.contains(&self.refresh_interval_secs) {
            self.refresh_interval_secs = 1;
        }
    }

    pub fn save(&self) {
        let dir = Self::config_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("[digger] Failed to create config directory: {e}");
            return;
        }

        // Set restrictive permissions on config directory (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
        }

        let path = Self::config_path();
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, &json) {
                    eprintln!("[digger] Failed to save preferences: {e}");
                    return;
                }
                // Set restrictive permissions on the file (Unix only)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
                }
            }
            Err(e) => {
                eprintln!("[digger] Failed to serialize preferences: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let prefs = Preferences::default();
        assert_eq!(prefs.process_limit, 200);
        assert_eq!(prefs.live_buffer_size, 120);
        assert_eq!(prefs.retention_hours, 24);
        assert!((prefs.cpu_alert_threshold - 90.0).abs() < 0.01);
        assert!(prefs.temp_celsius);
        assert!(!prefs.use_dyslexic_font);
    }

    #[test]
    fn test_serde_roundtrip() {
        let prefs = Preferences::default();
        let json = serde_json::to_string(&prefs).unwrap();
        let loaded: Preferences = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.process_limit, prefs.process_limit);
        assert_eq!(loaded.theme, prefs.theme);
    }

    #[test]
    fn test_backwards_compat_missing_fields() {
        // Simulate an old config without new fields
        let old_json = r#"{"theme":"CatppuccinMocha","accent":"Blue","refresh_interval_secs":2,"temp_celsius":false}"#;
        let prefs: Preferences = serde_json::from_str(old_json).unwrap();
        assert_eq!(prefs.refresh_interval_secs, 2);
        assert!(!prefs.temp_celsius);
        // New fields should use defaults
        assert_eq!(prefs.process_limit, 200);
        assert_eq!(prefs.live_buffer_size, 120);
        assert!(!prefs.use_dyslexic_font);
    }
}
