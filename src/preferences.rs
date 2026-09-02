use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::i18n::Language;
use crate::theme::{AccentChoice, ThemeChoice};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    /// The theme, as catalog keys. Reads the eleven enum names older files
    /// hold — see `ThemeChoice`'s Deserialize.
    #[serde(default)]
    pub theme: ThemeChoice,
    /// The accent override, or "follow the theme" when unset.
    #[serde(default)]
    pub accent: AccentChoice,
    /// Defaulted, like every other field: serde fails the whole struct on one
    /// missing field, so a file that has lost a single key would otherwise
    /// reset every setting the user ever changed.
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    #[serde(default = "default_temp_celsius")]
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
    /// Appearance -> Typography. Multiplies with `text_scale`.
    #[serde(default = "default_scale")]
    pub font_scale: f32,
    /// Accessibility -> Reading. Multiplies with `font_scale`; a user on large
    /// typography and xlarge reading text is at 1.68x and the layout has to
    /// survive it.
    #[serde(default = "default_scale")]
    pub text_scale: f32,
    /// Accessibility -> Vision. Derives a boosted palette from the active one
    /// rather than selecting a separate high-contrast theme.
    #[serde(default)]
    pub high_contrast: bool,
    /// Accessibility -> Motion. Silences every animation when set.
    #[serde(default)]
    pub reduced_motion: bool,
}

fn default_scale() -> f32 {
    1.0
}

/// The steps the two size settings offer, as (multiplier, label key index).
/// Reading gets a fourth step; typography stops at large.
pub const FONT_SCALES: &[f32] = &[0.85, 1.0, 1.2];
pub const TEXT_SCALES: &[f32] = &[0.85, 1.0, 1.2, 1.4];

fn default_refresh_interval() -> u64 {
    1
}
fn default_temp_celsius() -> bool {
    true
}
fn default_process_limit() -> usize {
    200
}
const MAX_PROCESS_LIMIT: usize = 5000;
const REFRESH_OPTIONS: &[u64] = &[1, 2, 5];
fn default_live_buffer_size() -> usize {
    120
}
fn default_retention_hours() -> u64 {
    24
}
fn default_cpu_alert_threshold() -> f32 {
    90.0
}
fn default_mem_alert_threshold() -> f32 {
    90.0
}
/// The offered step closest to `value`, so a hand-edited or future-version
/// preferences file still lands on something the UI can show as selected.
fn nearest(value: f32, steps: &[f32]) -> f32 {
    *steps
        .iter()
        .min_by(|a, b| {
            (*a - value)
                .abs()
                .partial_cmp(&(*b - value).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(&1.0)
}

fn default_process_sort() -> String {
    "cpu".into()
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::default(),
            accent: AccentChoice::default(),
            refresh_interval_secs: default_refresh_interval(),
            temp_celsius: default_temp_celsius(),
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
            font_scale: default_scale(),
            text_scale: default_scale(),
            high_contrast: false,
            reduced_motion: false,
        }
    }
}

impl Preferences {
    /// `<config>/Colony/Digger/` — Windows AppData\\Local, not Roaming, and
    /// `~/.config` on Linux. The shared helper is the definition of that
    /// layout; Digger used to spell it out and had no way to notice if it
    /// drifted.
    ///
    /// `locate::` rather than `config_dir` because reading preferences should
    /// not bring the directory into existence — only `save` creates it.
    fn config_dir() -> PathBuf {
        colony_ui::paths::locate::config_dir(crate::PROGRAM).unwrap_or_else(|_| PathBuf::from("."))
    }

    #[cfg(test)]
    pub fn config_dir_for_test() -> PathBuf {
        Self::config_dir()
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
        // A scale outside the offered steps reaches the layout as a size no
        // widget was designed for, so snap to the nearest one rather than
        // clamping to a range that still admits 1.07.
        self.font_scale = nearest(self.font_scale, FONT_SCALES);
        self.text_scale = nearest(self.text_scale, TEXT_SCALES);
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
    #[test]
    fn a_realistic_old_preferences_file_keeps_its_theme() {
        let old = r#"{"theme":"KanagawaDragon","accent":"Amber","refresh_interval_secs":2,
            "temp_celsius":true,"process_limit":200,"auto_theme":false}"#;
        let p: Preferences = serde_json::from_str(old).unwrap();
        assert!(
            p.theme.is("kanagawa", "dragon"),
            "theme became {:?}",
            p.theme
        );
        assert!(p.accent.is("amber"), "accent became {:?}", p.accent);
        assert_eq!(p.refresh_interval_secs, 2);
    }

    #[test]
    fn a_scale_outside_the_offered_steps_snaps_to_one_of_them() {
        // A hand-edited file, or one written by a version with different steps.
        // Clamping to a range would leave 1.07 selected and no button lit.
        let mut p = Preferences {
            font_scale: 1.07,
            text_scale: 3.0,
            ..Preferences::default()
        };
        p.sanitize();
        assert_eq!(p.font_scale, 1.0);
        assert_eq!(p.text_scale, 1.4);
        assert!(FONT_SCALES.contains(&p.font_scale));
        assert!(TEXT_SCALES.contains(&p.text_scale));
    }

    #[test]
    fn the_two_scales_multiply_to_the_documented_extremes() {
        // design/typography.md: a layout has to survive 0.7225x and 1.68x.
        let smallest = FONT_SCALES[0] * TEXT_SCALES[0];
        let largest = FONT_SCALES[FONT_SCALES.len() - 1] * TEXT_SCALES[TEXT_SCALES.len() - 1];
        assert!((smallest - 0.7225).abs() < 1e-4, "smallest is {smallest}");
        assert!((largest - 1.68).abs() < 1e-4, "largest is {largest}");
    }

    #[test]
    fn a_file_written_before_these_settings_existed_gets_the_defaults() {
        let old = r#"{"theme":"GruvboxDark","accent":"Green","refresh_interval_secs":1}"#;
        let p: Preferences = serde_json::from_str(old).unwrap();
        assert!(
            p.theme.is("gruvbox", "dark"),
            "the theme it did have was lost"
        );
        assert!(p.temp_celsius, "a missing field must not reset the others");
        assert_eq!(p.font_scale, 1.0);
        assert_eq!(p.text_scale, 1.0);
        assert!(!p.high_contrast);
        assert!(!p.reduced_motion);
    }
}
