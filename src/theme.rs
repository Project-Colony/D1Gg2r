//! Digger's colours, derived from the shared Colony catalog.
//!
//! Digger used to carry eleven themes of its own. It now names a theme the way
//! every Colony program does — a family key and a variant key resolved through
//! `colony-ui` — which is what lets one catalog serve the whole ecosystem and
//! what gave Digger fifty-nine themes instead of eleven.

use iced::Color;
use serde::{Deserialize, Deserializer, Serialize};

// ─── THE USER'S CHOICE ──────────────────────────────────────────

/// A theme, named by the catalog's own keys.
///
/// Stored rather than an enum because the catalog grows: a family added to
/// Project-Colony-Resources shows up here on the next `cargo update`, with no
/// enum to extend and no match arm to forget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeChoice {
    pub family: String,
    pub variant: String,
}

impl Default for ThemeChoice {
    fn default() -> Self {
        Self {
            family: "catppuccin".into(),
            variant: "mocha".into(),
        }
    }
}

impl ThemeChoice {
    pub fn new(family: &str, variant: &str) -> Self {
        Self {
            family: family.to_string(),
            variant: variant.to_string(),
        }
    }

    pub fn is(&self, family: &str, variant: &str) -> bool {
        self.family == family && self.variant == variant
    }

    /// The catalog entry, or `None` for a theme this build has never heard of.
    ///
    /// Distinct from `resolve`, which silently falls back: a config naming a
    /// removed theme should still *render*, but a caller asking "is this light?"
    /// deserves to know the answer is a guess.
    pub fn entry(&self) -> Option<&'static colony_ui::ThemeVariantMeta> {
        colony_ui::THEME_FAMILIES
            .iter()
            .find(|f| f.key == self.family)?
            .variants
            .iter()
            .find(|v| v.key == self.variant)
    }

    pub fn is_light(&self) -> bool {
        self.entry().map(|v| v.mode == "light").unwrap_or(false)
    }

    /// The eleven names Digger persisted before it used the shared catalog.
    ///
    /// `KanagawaDragon` is the one that is not a rename: Colony's
    /// `kanagawa/journal` holds the const name `KANAGAWA_DRAGON` while being a
    /// light parchment theme, so the upstream Dragon is `kanagawa/dragon`.
    fn from_legacy(name: &str) -> Self {
        let (family, variant) = match name {
            "CatppuccinLatte" => ("catppuccin", "latte"),
            "CatppuccinFrappe" => ("catppuccin", "frappe"),
            "CatppuccinMacchiato" => ("catppuccin", "macchiato"),
            "CatppuccinMocha" => ("catppuccin", "mocha"),
            "GruvboxLight" => ("gruvbox", "light"),
            "GruvboxDark" => ("gruvbox", "dark"),
            "EverblushLight" => ("everblush", "light"),
            "EverblushDark" => ("everblush", "dark"),
            "KanagawaLight" => ("kanagawa", "light"),
            "KanagawaDark" => ("kanagawa", "dark"),
            "KanagawaDragon" => ("kanagawa", "dragon"),
            // Not a theme this build knows. Falling back to the default is the
            // only option that still starts, and the picker will show what the
            // user actually has selected rather than a phantom.
            _ => return Self::default(),
        };
        Self::new(family, variant)
    }
}

/// Accepts both shapes: `{"family":"gruvbox","variant":"dark"}` as written
/// today, and the bare `"GruvboxDark"` that older preferences files hold.
impl<'de> Deserialize<'de> for ThemeChoice {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Stored {
            Keys { family: String, variant: String },
            Legacy(String),
        }
        Ok(match Stored::deserialize(d)? {
            Stored::Keys { family, variant } => Self { family, variant },
            Stored::Legacy(name) => Self::from_legacy(&name),
        })
    }
}

/// The accent override. `None` means "follow the theme".
///
/// Never a colour value: an unset override resolves to the active palette's own
/// `accent_blue`, which is a different thing from the user having picked blue.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AccentChoice(pub Option<String>);

impl AccentChoice {
    pub fn key(&self) -> Option<&str> {
        self.0.as_deref()
    }

    pub fn is(&self, key: &str) -> bool {
        self.0.as_deref() == Some(key)
    }
}

/// Older preferences stored the capitalised enum name, `"Blue"`. The catalog
/// keys are the same eight words in lowercase, so one `to_lowercase` reads both
/// — and anything that is not one of the eight becomes "follow the theme"
/// rather than a hard error on a file the user cannot edit.
impl<'de> Deserialize<'de> for AccentChoice {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let stored = Option::<String>::deserialize(d)?;
        Ok(Self(stored.map(|s| s.to_lowercase()).filter(|key| {
            colony_ui::ACCENT_OVERRIDES.iter().any(|a| a.key == *key)
        })))
    }
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

pub fn build_palette(theme: &ThemeChoice, accent: &AccentChoice, high_contrast: bool) -> Palette {
    let base = base_palette_with(theme, high_contrast);
    let chosen = accent
        .key()
        .and_then(colony_ui::accent_key_to_color)
        .unwrap_or(base.accent);
    Palette {
        // The accent is a chart line like any other, and the user can pick a
        // pale one on a pale theme.
        accent: readable_on(chosen, base.bg, base.panel_bg),
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
pub fn base_palette(theme: &ThemeChoice) -> Palette {
    base_palette_with(theme, false)
}

/// High contrast is a parameter rather than a read of colony-ui's global,
/// because `resolve` returns the raw palette: a program that derives its
/// colours from `resolve` and sets the global would show the setting doing
/// nothing at all.
pub fn base_palette_with(theme: &ThemeChoice, high_contrast: bool) -> Palette {
    let p = colony_ui::resolve(&theme.family, &theme.variant);
    let p = if high_contrast {
        p.with_high_contrast()
    } else {
        p
    };
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

// ─── LEGIBILITY ─────────────────────────────────────────────────

/// WCAG relative luminance.
///
/// Not `colony_ui::ColorExt::luma`, which is the cheaper YIQ approximation and
/// the right tool for "is this surface light?". Deciding whether a thin line is
/// *visible* needs the real curve.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every theme the picker can offer has to produce a usable palette — the
    /// picker renders the whole catalog now, not a list Digger curated.
    fn all_themes() -> impl Iterator<Item = ThemeChoice> {
        colony_ui::THEME_FAMILIES
            .iter()
            .flat_map(|f| f.variants.iter().map(|v| ThemeChoice::new(f.key, v.key)))
    }

    #[test]
    fn the_catalog_is_not_empty_and_holds_what_digger_used_to_ship() {
        let all: Vec<_> = all_themes().collect();
        assert!(
            all.len() >= 59,
            "the catalog shrank: {} variants",
            all.len()
        );
        for (family, variant) in [
            ("catppuccin", "mocha"),
            ("gruvbox", "dark"),
            ("everblush", "dark"),
            ("kanagawa", "dragon"),
        ] {
            assert!(
                all.iter().any(|t| t.is(family, variant)),
                "{family}/{variant} is gone"
            );
        }
    }

    /// The regression the legibility helper exists for. Before it, `warning` on
    /// Kanagawa journal was 1.65:1 — a yellow sparkline on parchment. Every
    /// series has to clear the floor on both surfaces it is drawn on, for every
    /// theme and every accent the user can pick.
    #[test]
    fn every_series_colour_is_visible_on_every_theme_and_accent() {
        let accents = std::iter::once(AccentChoice(None)).chain(
            colony_ui::ACCENT_OVERRIDES
                .iter()
                .map(|a| AccentChoice(Some(a.key.to_string()))),
        );
        let accents: Vec<_> = accents.collect();

        let mut failures = Vec::new();
        for high_contrast in [false, true] {
            for theme in all_themes() {
                for accent in &accents {
                    let p = build_palette(&theme, accent, high_contrast);
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
                                "{}/{} {:?} hc={high_contrast} {name} on {surface}: {ratio:.2}:1",
                                theme.family, theme.variant, accent.0
                            ));
                            }
                        }
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} series colours below {MIN_SERIES_CONTRAST}:1:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }

    /// Adjusting for legibility must not collapse two chart lines into one.
    #[test]
    fn the_three_series_stay_distinct_on_every_theme() {
        for theme in all_themes() {
            let p = base_palette(&theme);
            let series = [p.cyan, p.magenta, p.blue];
            for (i, a) in series.iter().enumerate() {
                for b in series.iter().skip(i + 1) {
                    assert_ne!(
                        a, b,
                        "{}/{}: two chart series share a colour",
                        theme.family, theme.variant
                    );
                }
            }
        }
    }

    /// A colour that already clears the floor comes back untouched — the dark
    /// themes were fine before and must not be repainted.
    #[test]
    fn a_colour_that_already_passes_is_left_alone() {
        let p = colony_ui::resolve("kanagawa", "dragon");
        assert_eq!(
            readable_on(p.success, p.bg_primary, p.bg_card),
            p.success,
            "Dragon's green passed at 6.9:1 and should not have moved"
        );
    }

    #[test]
    fn light_and_dark_come_from_the_catalog() {
        assert!(ThemeChoice::new("catppuccin", "latte").is_light());
        assert!(!ThemeChoice::new("catppuccin", "mocha").is_light());
        assert!(!ThemeChoice::new("kanagawa", "dragon").is_light());
        assert!(
            ThemeChoice::new("kanagawa", "journal").is_light(),
            "journal is Colony's parchment theme, not upstream Dragon"
        );
    }

    #[test]
    fn a_theme_this_build_does_not_know_is_not_claimed_as_light() {
        let unknown = ThemeChoice::new("a_theme_from_the_future", "shiny");
        assert!(unknown.entry().is_none());
        assert!(!unknown.is_light());
        // It still renders: resolve falls back rather than panicking.
        let _ = base_palette(&unknown);
    }

    // ─── persisted preferences ──────────────────────────────────

    /// The eleven names older preferences files hold. Getting this wrong resets
    /// a user's theme on upgrade, silently.
    #[test]
    fn legacy_theme_names_still_load() {
        let cases = [
            ("CatppuccinLatte", "catppuccin", "latte"),
            ("CatppuccinMocha", "catppuccin", "mocha"),
            ("GruvboxLight", "gruvbox", "light"),
            ("EverblushDark", "everblush", "dark"),
            ("KanagawaLight", "kanagawa", "light"),
            ("KanagawaDark", "kanagawa", "dark"),
            ("KanagawaDragon", "kanagawa", "dragon"),
        ];
        for (stored, family, variant) in cases {
            let got: ThemeChoice = serde_json::from_str(&format!("\"{stored}\"")).unwrap();
            assert!(
                got.is(family, variant),
                "{stored} loaded as {}/{}",
                got.family,
                got.variant
            );
        }
    }

    #[test]
    fn the_new_theme_shape_round_trips() {
        let choice = ThemeChoice::new("rosepine", "moon");
        let json = serde_json::to_string(&choice).unwrap();
        assert_eq!(serde_json::from_str::<ThemeChoice>(&json).unwrap(), choice);
    }

    #[test]
    fn an_unreadable_theme_falls_back_rather_than_failing_to_load() {
        let got: ThemeChoice = serde_json::from_str("\"SomeThemeWeDropped\"").unwrap();
        assert_eq!(got, ThemeChoice::default());
    }

    #[test]
    fn legacy_accent_names_still_load() {
        for (stored, key) in [
            ("\"Blue\"", "blue"),
            ("\"Amber\"", "amber"),
            ("\"Violet\"", "violet"),
        ] {
            let got: AccentChoice = serde_json::from_str(stored).unwrap();
            assert!(got.is(key), "{stored} loaded as {:?}", got.0);
        }
    }

    #[test]
    fn an_unset_accent_means_follow_the_theme() {
        let got: AccentChoice = serde_json::from_str("null").unwrap();
        assert_eq!(got.key(), None);
        // And it resolves to the palette's own accent, not to a stored colour.
        let theme = ThemeChoice::new("gruvbox", "dark");
        assert_eq!(
            build_palette(&theme, &got, false).accent,
            base_palette(&theme).accent
        );
    }

    #[test]
    fn an_accent_this_build_does_not_know_means_follow_the_theme() {
        let got: AccentChoice = serde_json::from_str("\"chartreuse\"").unwrap();
        assert_eq!(got.key(), None);
    }
}
