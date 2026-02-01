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
        accent: accent.color(),
        ..base
    }
}

fn base_palette(theme: ThemeVariant) -> Palette {
    match theme {
        // ── Catppuccin Latte ──
        ThemeVariant::CatppuccinLatte => Palette {
            bg:         hex(0xef, 0xf1, 0xf5),
            panel_bg:   hex(0xe6, 0xe9, 0xef),
            sidebar_bg: hex(0xdc, 0xe0, 0xe8),
            border:     hex(0xcc, 0xd0, 0xda),
            grid:       Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            label:      hex(0x6c, 0x6f, 0x85),
            text:       hex(0x4c, 0x4f, 0x69),
            bar_bg:     hex(0xcc, 0xd0, 0xda),
            accent:     hex(0x89, 0xb4, 0xfa), // placeholder, overridden
            green:      hex(0x40, 0xa0, 0x2b),
            red:        hex(0xd2, 0x0f, 0x39),
            yellow:     hex(0xdf, 0x8e, 0x1d),
            cyan:       hex(0x04, 0xa5, 0xe5),
            magenta:    hex(0x88, 0x39, 0xef),
            blue:       hex(0x1e, 0x66, 0xf5),
        },
        // ── Catppuccin Frappé ──
        ThemeVariant::CatppuccinFrappe => Palette {
            bg:         hex(0x30, 0x34, 0x46),
            panel_bg:   hex(0x29, 0x2c, 0x3c),
            sidebar_bg: hex(0x23, 0x26, 0x34),
            border:     hex(0x41, 0x45, 0x59),
            grid:       Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            label:      hex(0xa5, 0xad, 0xce),
            text:       hex(0xc6, 0xd0, 0xf5),
            bar_bg:     hex(0x41, 0x45, 0x59),
            accent:     hex(0x8c, 0xaa, 0xee),
            green:      hex(0xa6, 0xd1, 0x89),
            red:        hex(0xe7, 0x82, 0x84),
            yellow:     hex(0xe5, 0xc8, 0x90),
            cyan:       hex(0x81, 0xc8, 0xbe),
            magenta:    hex(0xca, 0x9e, 0xe6),
            blue:       hex(0x8c, 0xaa, 0xee),
        },
        // ── Catppuccin Macchiato ──
        ThemeVariant::CatppuccinMacchiato => Palette {
            bg:         hex(0x24, 0x27, 0x3a),
            panel_bg:   hex(0x1e, 0x20, 0x30),
            sidebar_bg: hex(0x18, 0x1a, 0x26),
            border:     hex(0x36, 0x3a, 0x4f),
            grid:       Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            label:      hex(0xa5, 0xad, 0xcb),
            text:       hex(0xca, 0xd3, 0xf5),
            bar_bg:     hex(0x36, 0x3a, 0x4f),
            accent:     hex(0x8a, 0xad, 0xf4),
            green:      hex(0xa6, 0xda, 0x95),
            red:        hex(0xed, 0x87, 0x96),
            yellow:     hex(0xee, 0xd4, 0x9f),
            cyan:       hex(0x8b, 0xd5, 0xca),
            magenta:    hex(0xc6, 0xa0, 0xf6),
            blue:       hex(0x8a, 0xad, 0xf4),
        },
        // ── Catppuccin Mocha ──
        ThemeVariant::CatppuccinMocha => Palette {
            bg:         hex(0x1e, 0x1e, 0x2e),
            panel_bg:   hex(0x18, 0x18, 0x25),
            sidebar_bg: hex(0x11, 0x11, 0x1b),
            border:     hex(0x31, 0x32, 0x44),
            grid:       Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            label:      hex(0xa6, 0xad, 0xc8),
            text:       hex(0xcd, 0xd6, 0xf4),
            bar_bg:     hex(0x31, 0x32, 0x44),
            accent:     hex(0x89, 0xb4, 0xfa),
            green:      hex(0xa6, 0xe3, 0xa1),
            red:        hex(0xf3, 0x8b, 0xa8),
            yellow:     hex(0xf9, 0xe2, 0xaf),
            cyan:       hex(0x94, 0xe2, 0xd5),
            magenta:    hex(0xcb, 0xa6, 0xf7),
            blue:       hex(0x89, 0xb4, 0xfa),
        },
        // ── Gruvbox Dark ──
        ThemeVariant::GruvboxDark => Palette {
            bg:         hex(0x28, 0x28, 0x28),
            panel_bg:   hex(0x1d, 0x20, 0x21),
            sidebar_bg: hex(0x17, 0x19, 0x1a),
            border:     hex(0x3c, 0x38, 0x36),
            grid:       Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            label:      hex(0xa8, 0x99, 0x84),
            text:       hex(0xeb, 0xdb, 0xb2),
            bar_bg:     hex(0x3c, 0x38, 0x36),
            accent:     hex(0x83, 0xa5, 0x98),
            green:      hex(0xb8, 0xbb, 0x26),
            red:        hex(0xfb, 0x49, 0x34),
            yellow:     hex(0xfa, 0xbd, 0x2f),
            cyan:       hex(0x8e, 0xc0, 0x7c),
            magenta:    hex(0xd3, 0x86, 0x9b),
            blue:       hex(0x83, 0xa5, 0x98),
        },
        // ── Gruvbox Light ──
        ThemeVariant::GruvboxLight => Palette {
            bg:         hex(0xfb, 0xf1, 0xc7),
            panel_bg:   hex(0xf2, 0xe5, 0xbc),
            sidebar_bg: hex(0xeb, 0xdb, 0xb2),
            border:     hex(0xd5, 0xc4, 0xa1),
            grid:       Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            label:      hex(0x66, 0x5c, 0x54),
            text:       hex(0x3c, 0x38, 0x36),
            bar_bg:     hex(0xd5, 0xc4, 0xa1),
            accent:     hex(0x42, 0x7b, 0x58),
            green:      hex(0x79, 0x74, 0x0e),
            red:        hex(0x9d, 0x00, 0x06),
            yellow:     hex(0xb5, 0x76, 0x14),
            cyan:       hex(0x42, 0x7b, 0x58),
            magenta:    hex(0x8f, 0x3f, 0x71),
            blue:       hex(0x07, 0x66, 0x78),
        },
        // ── Everblush Dark ──
        ThemeVariant::EverblushDark => Palette {
            bg:         hex(0x14, 0x17, 0x1f),
            panel_bg:   hex(0x1a, 0x1e, 0x28),
            sidebar_bg: hex(0x10, 0x13, 0x1a),
            border:     hex(0x2c, 0x31, 0x3d),
            grid:       Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            label:      hex(0x6e, 0x78, 0x8a),
            text:       hex(0xda, 0xdf, 0xe7),
            bar_bg:     hex(0x2c, 0x31, 0x3d),
            accent:     hex(0x67, 0xb0, 0xe8),
            green:      hex(0x8c, 0xd0, 0x81),
            red:        hex(0xe0, 0x6e, 0x6e),
            yellow:     hex(0xf1, 0xc1, 0x7b),
            cyan:       hex(0x67, 0xb0, 0xe8),
            magenta:    hex(0xc2, 0x8f, 0xd0),
            blue:       hex(0x67, 0xb0, 0xe8),
        },
        // ── Everblush Light ──
        ThemeVariant::EverblushLight => Palette {
            bg:         hex(0xf0, 0xf0, 0xf0),
            panel_bg:   hex(0xe5, 0xe5, 0xe8),
            sidebar_bg: hex(0xdb, 0xdb, 0xe0),
            border:     hex(0xc8, 0xc8, 0xd0),
            grid:       Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            label:      hex(0x6e, 0x72, 0x80),
            text:       hex(0x2e, 0x32, 0x3a),
            bar_bg:     hex(0xc8, 0xc8, 0xd0),
            accent:     hex(0x3a, 0x8a, 0xc0),
            green:      hex(0x50, 0x99, 0x46),
            red:        hex(0xc0, 0x3e, 0x3e),
            yellow:     hex(0xb8, 0x8b, 0x30),
            cyan:       hex(0x3a, 0x8a, 0xc0),
            magenta:    hex(0x90, 0x55, 0xa2),
            blue:       hex(0x3a, 0x6a, 0xb5),
        },
        // ── Kanagawa Wave (dark) ──
        ThemeVariant::KanagawaDark => Palette {
            bg:         hex(0x1f, 0x1f, 0x28),
            panel_bg:   hex(0x16, 0x16, 0x1d),
            sidebar_bg: hex(0x10, 0x10, 0x16),
            border:     hex(0x36, 0x36, 0x46),
            grid:       Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            label:      hex(0x72, 0x73, 0x8c),
            text:       hex(0xdc, 0xd7, 0xba),
            bar_bg:     hex(0x36, 0x36, 0x46),
            accent:     hex(0x7e, 0x79, 0x6b),
            green:      hex(0x98, 0xbb, 0x6c),
            red:        hex(0xc3, 0x46, 0x43),
            yellow:     hex(0xdb, 0xa4, 0x5a),
            cyan:       hex(0x7f, 0xb4, 0xca),
            magenta:    hex(0x95, 0x7f, 0xb8),
            blue:       hex(0x7e, 0x9c, 0xd8),
        },
        // ── Kanagawa Dragon ──
        ThemeVariant::KanagawaDragon => Palette {
            bg:         hex(0x18, 0x16, 0x16),
            panel_bg:   hex(0x12, 0x10, 0x10),
            sidebar_bg: hex(0x0d, 0x0c, 0x0c),
            border:     hex(0x2d, 0x2a, 0x2a),
            grid:       Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            label:      hex(0x73, 0x71, 0x67),
            text:       hex(0xc5, 0xc9, 0xc5),
            bar_bg:     hex(0x2d, 0x2a, 0x2a),
            accent:     hex(0x8b, 0xa4, 0xb0),
            green:      hex(0x87, 0xa9, 0x87),
            red:        hex(0xc4, 0x74, 0x6e),
            yellow:     hex(0xc4, 0xb2, 0x8a),
            cyan:       hex(0x8b, 0xa4, 0xb0),
            magenta:    hex(0xa2, 0x92, 0xa3),
            blue:       hex(0x8b, 0xa4, 0xb0),
        },
        // ── Kanagawa Lotus (light) ──
        ThemeVariant::KanagawaLight => Palette {
            bg:         hex(0xf2, 0xec, 0xbc),
            panel_bg:   hex(0xe7, 0xe1, 0xb0),
            sidebar_bg: hex(0xdc, 0xd5, 0xa5),
            border:     hex(0xc7, 0xc0, 0x92),
            grid:       Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            label:      hex(0x71, 0x6e, 0x61),
            text:       hex(0x54, 0x54, 0x64),
            bar_bg:     hex(0xc7, 0xc0, 0x92),
            accent:     hex(0x62, 0x4c, 0x83),
            green:      hex(0x6f, 0x89, 0x4e),
            red:        hex(0xc8, 0x46, 0x48),
            yellow:     hex(0x77, 0x71, 0x3a),
            cyan:       hex(0x6c, 0x78, 0x2e),
            magenta:    hex(0x62, 0x4c, 0x83),
            blue:       hex(0x4d, 0x69, 0x9b),
        },
    }
}

const fn hex(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}
