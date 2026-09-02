use iced::Color;
use serde::{Deserialize, Serialize};

// ─── ACCENT COLORS ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccentColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Indigo,
    Violet,
    Amber,
}

impl AccentColor {
    pub const ALL: &[AccentColor] = &[
        AccentColor::Red,
        AccentColor::Orange,
        AccentColor::Yellow,
        AccentColor::Green,
        AccentColor::Blue,
        AccentColor::Indigo,
        AccentColor::Violet,
        AccentColor::Amber,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            AccentColor::Red => "Red",
            AccentColor::Orange => "Orange",
            AccentColor::Yellow => "Yellow",
            AccentColor::Green => "Green",
            AccentColor::Blue => "Blue",
            AccentColor::Indigo => "Indigo",
            AccentColor::Violet => "Violet",
            AccentColor::Amber => "Amber",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            AccentColor::Red => Color::from_rgb(0.93, 0.30, 0.35),
            AccentColor::Orange => Color::from_rgb(0.96, 0.52, 0.20),
            AccentColor::Yellow => Color::from_rgb(0.95, 0.80, 0.25),
            AccentColor::Green => Color::from_rgb(0.35, 0.87, 0.40),
            AccentColor::Blue => Color::from_rgb(0.33, 0.63, 0.95),
            AccentColor::Indigo => Color::from_rgb(0.40, 0.35, 0.90),
            AccentColor::Violet => Color::from_rgb(0.65, 0.45, 0.85),
            AccentColor::Amber => Color::from_rgb(1.0, 0.75, 0.03),
        }
    }
}

// ─── THEME VARIANTS ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeVariant {
    // Catppuccin
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    CatppuccinMocha,
    // Gruvbox
    GruvboxLight,
    GruvboxDark,
    // Everblush
    EverblushLight,
    EverblushDark,
    // Kanagawa
    KanagawaLight,
    KanagawaDark,
    KanagawaDragon,
}

impl ThemeVariant {
    pub const ALL: &[ThemeVariant] = &[
        ThemeVariant::CatppuccinLatte,
        ThemeVariant::CatppuccinFrappe,
        ThemeVariant::CatppuccinMacchiato,
        ThemeVariant::CatppuccinMocha,
        ThemeVariant::GruvboxLight,
        ThemeVariant::GruvboxDark,
        ThemeVariant::EverblushLight,
        ThemeVariant::EverblushDark,
        ThemeVariant::KanagawaLight,
        ThemeVariant::KanagawaDark,
        ThemeVariant::KanagawaDragon,
    ];

    /// The catalog keys this variant names in colony-ui.
    ///
    /// Digger's eleven themes are four of the shared families, and the palettes
    /// were copies of the shared ones. This is the whole of the mapping.
    pub fn keys(&self) -> (&'static str, &'static str) {
        match self {
            ThemeVariant::CatppuccinLatte => ("catppuccin", "latte"),
            ThemeVariant::CatppuccinFrappe => ("catppuccin", "frappe"),
            ThemeVariant::CatppuccinMacchiato => ("catppuccin", "macchiato"),
            ThemeVariant::CatppuccinMocha => ("catppuccin", "mocha"),
            ThemeVariant::GruvboxLight => ("gruvbox", "light"),
            ThemeVariant::GruvboxDark => ("gruvbox", "dark"),
            ThemeVariant::EverblushLight => ("everblush", "light"),
            ThemeVariant::EverblushDark => ("everblush", "dark"),
            ThemeVariant::KanagawaLight => ("kanagawa", "light"),
            ThemeVariant::KanagawaDark => ("kanagawa", "dark"),
            ThemeVariant::KanagawaDragon => ("kanagawa", "dragon"),
        }
    }

    pub fn family_key(&self) -> &'static str {
        self.keys().0
    }

    pub fn variant_key(&self) -> &'static str {
        self.keys().1
    }

    pub fn name(&self) -> &'static str {
        match self {
            ThemeVariant::CatppuccinLatte => "Latte",
            ThemeVariant::CatppuccinFrappe => "Frappé",
            ThemeVariant::CatppuccinMacchiato => "Macchiato",
            ThemeVariant::CatppuccinMocha => "Mocha",
            ThemeVariant::GruvboxLight => "Light",
            ThemeVariant::GruvboxDark => "Dark",
            ThemeVariant::EverblushLight => "Light",
            ThemeVariant::EverblushDark => "Dark",
            ThemeVariant::KanagawaLight => "Lotus",
            ThemeVariant::KanagawaDark => "Wave",
            ThemeVariant::KanagawaDragon => "Dragon",
        }
    }

    pub fn family(&self) -> &'static str {
        match self {
            ThemeVariant::CatppuccinLatte
            | ThemeVariant::CatppuccinFrappe
            | ThemeVariant::CatppuccinMacchiato
            | ThemeVariant::CatppuccinMocha => "Catppuccin",
            ThemeVariant::GruvboxLight | ThemeVariant::GruvboxDark => "Gruvbox",
            ThemeVariant::EverblushLight | ThemeVariant::EverblushDark => "Everblush",
            ThemeVariant::KanagawaLight
            | ThemeVariant::KanagawaDark
            | ThemeVariant::KanagawaDragon => "Kanagawa",
        }
    }

    pub fn is_light(&self) -> bool {
        matches!(
            self,
            ThemeVariant::CatppuccinLatte
                | ThemeVariant::GruvboxLight
                | ThemeVariant::EverblushLight
                | ThemeVariant::KanagawaLight
        )
    }
}

// ─── LEGIBILITY ─────────────────────────────────────────────────

/// WCAG relative luminance.
fn luminance(c: Color) -> f32 {
    fn channel(v: f32) -> f32 {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
}

/// WCAG contrast ratio, 1.0 (identical) to 21.0 (black on white).
fn contrast(a: Color, b: Color) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// The floor for a chart line or a coloured status label: WCAG's non-text
/// contrast minimum.
const MIN_SERIES_CONTRAST: f32 = 3.0;

/// Keep a series colour's hue, move its lightness until it is actually visible.
///
/// The shared palette picks hues that tell series apart from *each other*.
/// Reading against the paper is a different job, and on the light themes the
/// shared palette loses it badly — `warning` on Kanagawa journal is 1.65:1, a
/// yellow line on parchment. Digger draws these as one-pixel sparklines and as
/// small status text, so both surfaces have to clear the floor: a colour tuned
/// only against the window background still disappears on a card.
///
/// Blending toward black preserves hue exactly; toward white it desaturates,
/// which is the lesser evil on a dark theme where there is no room downward.
fn readable_on(color: Color, bg: Color, panel: Color) -> Color {
    let clears = |c: Color| {
        contrast(c, bg) >= MIN_SERIES_CONTRAST && contrast(c, panel) >= MIN_SERIES_CONTRAST
    };
    if clears(color) {
        return color;
    }
    let target = if luminance(bg) > 0.5 { 0.0 } else { 1.0 };
    let mut best = color;
    for step in 1..=64 {
        let t = step as f32 / 64.0;
        best = Color {
            r: color.r + (target - color.r) * t,
            g: color.g + (target - color.g) * t,
            b: color.b + (target - color.b) * t,
            a: color.a,
        };
        if clears(best) {
            break;
        }
    }
    best
}

// ─── PALETTE ────────────────────────────────────────────────────

/// All semantic colors the app uses, derived from theme + accent.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub panel_bg: Color,
    pub sidebar_bg: Color,
    pub border: Color,
    pub grid: Color,
    pub label: Color,
    pub text: Color,
    pub bar_bg: Color,
    // Semantic
    pub accent: Color,
    pub green: Color,
    pub red: Color,
    pub yellow: Color,
    pub cyan: Color,
    pub magenta: Color,
    pub blue: Color,
}

pub fn build_palette(theme: ThemeVariant, accent: AccentColor) -> Palette {
    let base = base_palette(theme);
    Palette {
        // The accent is a chart line like any other, and the user can pick a
        // pale one on a pale theme.
        accent: readable_on(accent.color(), base.bg, base.panel_bg),
        ..base
    }
}

/// Derive Digger's fifteen roles from the shared thirty-eight.
///
/// Twelve map onto a shared role that means the same thing. Three do not:
/// `cyan`, `magenta` and `blue` are *series* colours — they identify one line on
/// a chart against another — and the shared palette has no such vocabulary. They
/// come from the shared accent overrides instead, which are fixed hues chosen to
/// be distinguishable from each other. That makes a series keep its identity
/// across themes, and it is the reason they are not simply mapped onto
/// accent_icon and accent_progress: on most themes those two are the same
/// colour, which would have quietly turned two chart lines into one.
fn base_palette(theme: ThemeVariant) -> Palette {
    let p = colony_ui::resolve(theme.family_key(), theme.variant_key());
    let series = |key: &str| colony_ui::accent_key_to_color(key).unwrap_or(p.accent_blue);
    let legible = |c: Color| readable_on(c, p.bg_primary, p.bg_card);
    Palette {
        bg: p.bg_primary,
        panel_bg: p.bg_card,
        sidebar_bg: p.bg_sidebar,
        border: p.border_subtle,
        // A hairline over the chart area rather than a palette entry: it has to
        // read against whatever the theme's background is, so it is the ink at
        // low alpha rather than a colour of its own.
        grid: Color {
            a: 0.06,
            ..p.text_primary
        },
        label: p.text_muted,
        text: p.text_primary,
        bar_bg: p.bg_progress,
        // Replaced by build_palette with the user's chosen accent.
        accent: p.accent_blue,
        green: legible(p.success),
        red: legible(p.error),
        yellow: legible(p.warning),
        cyan: legible(series("blue")),
        magenta: legible(series("violet")),
        blue: legible(series("indigo")),
    }
}

#[cfg(test)]
mod shared_palette_tests {
    use super::*;

    /// colony_ui::resolve falls back to Gruvbox Dark for a pair it does not
    /// recognise, so a wrong mapping would not fail — every theme would quietly
    /// become Gruvbox Dark. Check each against the catalog instead.
    #[test]
    fn every_variant_names_a_real_catalog_entry() {
        for variant in ThemeVariant::ALL {
            let (family, name) = variant.keys();
            let found = colony_ui::THEME_FAMILIES
                .iter()
                .find(|f| f.key == family)
                .and_then(|f| f.variant(name));
            assert!(
                found.is_some(),
                "{variant:?} maps to ({family}, {name}), absent from the catalog"
            );
        }
    }

    #[test]
    fn distinct_variants_stay_distinct() {
        let mut seen = std::collections::BTreeSet::new();
        for variant in ThemeVariant::ALL {
            assert!(
                seen.insert(variant.keys()),
                "{variant:?} duplicates another"
            );
        }
    }

    /// is_light() is Digger's own list; the catalog records the same fact.
    /// They must agree, or a light theme gets dark-theme treatment somewhere.
    #[test]
    fn the_light_dark_split_agrees_with_the_catalog() {
        for variant in ThemeVariant::ALL {
            let (family, name) = variant.keys();
            let mode = colony_ui::THEME_FAMILIES
                .iter()
                .find(|f| f.key == family)
                .and_then(|f| f.variant(name))
                .map(|v| v.mode)
                .expect("catalog entry");
            assert_eq!(
                variant.is_light(),
                mode == "light",
                "{variant:?}: Digger says light={}, the catalog says {mode}",
                variant.is_light()
            );
        }
    }

    /// Two chart lines sharing a colour is the bug this mapping exists to
    /// avoid — accent_icon and accent_progress are the same colour on most
    /// themes, which is why the series colours do not come from them.
    #[test]
    fn the_three_series_colours_are_distinct_on_every_theme() {
        for variant in ThemeVariant::ALL {
            let p = base_palette(*variant);
            let series = [p.cyan, p.magenta, p.blue];
            for (i, a) in series.iter().enumerate() {
                for b in series.iter().skip(i + 1) {
                    assert_ne!(a, b, "{variant:?}: two chart series share a colour");
                }
            }
        }
    }

    #[test]
    fn catppuccin_latte_still_has_its_own_colours() {
        let p = base_palette(ThemeVariant::CatppuccinLatte);
        assert_eq!(p.bg, colony_ui::hex(0xeff1f5));
        assert_eq!(p.text, colony_ui::hex(0x4c4f69));
        assert_eq!(p.label, colony_ui::hex(0x6c6f85));
    }

    /// The regression this whole helper exists for. Before it, `warning` on
    /// Kanagawa journal was 1.65:1 — a yellow sparkline on parchment. Every
    /// series has to clear the floor on both surfaces it is drawn on, for every
    /// theme and every accent the user can pick.
    #[test]
    fn every_series_colour_is_visible_on_every_theme_and_accent() {
        let mut failures = Vec::new();
        for variant in ThemeVariant::ALL {
            for accent in AccentColor::ALL {
                let p = build_palette(*variant, *accent);
                let roles = [
                    ("accent", p.accent),
                    ("green", p.green),
                    ("red", p.red),
                    ("yellow", p.yellow),
                    ("cyan", p.cyan),
                    ("magenta", p.magenta),
                    ("blue", p.blue),
                ];
                for (name, color) in roles {
                    for (surface, bg) in [("bg", p.bg), ("panel", p.panel_bg)] {
                        let ratio = contrast(color, bg);
                        if ratio < MIN_SERIES_CONTRAST {
                            failures.push(format!(
                                "{variant:?}/{accent:?} {name} on {surface}: {ratio:.2}:1"
                            ));
                        }
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "series colours below {MIN_SERIES_CONTRAST}:1:\n  {}",
            failures.join("\n  ")
        );
    }

    /// A colour that already clears the floor must come back untouched — the
    /// dark themes were fine before and must not be repainted.
    #[test]
    fn a_colour_that_already_passes_is_left_alone() {
        let p = colony_ui::resolve("kanagawa", "dragon");
        assert_eq!(
            readable_on(p.success, p.bg_primary, p.bg_card),
            p.success,
            "Dragon's green passed at 6.9:1 and should not have moved"
        );
    }
}
