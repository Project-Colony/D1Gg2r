#![windows_subsystem = "windows"]

mod chart;
mod gauge;
mod gpu;
mod history;
pub mod i18n;
pub mod icons;
mod metrics;
mod preferences;
mod ringbuf;
pub mod theme;
mod ui;

use ui::Digger;

// ─── Iosevka Nerd Font (default Latin/symbols) ─────────────────────
pub const NERD_FONT_BYTES: &[u8] =
    include_bytes!("ui/assets/fonts/IosevkaNerdFont/IosevkaNerdFontPropo-Regular.ttf");

pub const NERD_FONT_MONO_BYTES: &[u8] =
    include_bytes!("ui/assets/fonts/IosevkaNerdFont/IosevkaNerdFontMono-Regular.ttf");

pub const NERD_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Iosevka Nerd Font Propo"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const NERD_FONT_MONO: iced::Font = iced::Font {
    family: iced::font::Family::Name("Iosevka Nerd Font Mono"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

// ─── Sarasa Mono Nerd Font (CJK: Chinese/Japanese/Korean) ──────────
pub const SARASA_FONT_BYTES: &[u8] =
    include_bytes!("ui/assets/fonts/SarasaMonoNerdFont/Sarasa-Mono-SC-Nerd.ttf");

pub const SARASA_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Sarasa Nerd"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

// ─── DejaVu Sans Mono Nerd Font (Arabic script) ────────────────────
pub const DEJAVU_FONT_BYTES: &[u8] =
    include_bytes!("ui/assets/fonts/DejaVuSansMonoNerdFont/DejaVuSansMNerdFont-Regular.ttf");

pub const DEJAVU_MONO_FONT_BYTES: &[u8] =
    include_bytes!("ui/assets/fonts/DejaVuSansMonoNerdFont/DejaVuSansMNerdFontMono-Regular.ttf");

pub const DEJAVU_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("DejaVuSansM Nerd Font"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const DEJAVU_MONO_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("DejaVuSansM Nerd Font Mono"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

// ─── Noto Nerd Fonts (Indic scripts / broader Unicode) ──────────────
pub const NOTO_FONT_BYTES: &[u8] =
    include_bytes!("ui/assets/fonts/NotoMonoNerdFont/NotoMonoNerdFont-Regular.ttf");

pub const NOTO_MONO_FONT_BYTES: &[u8] =
    include_bytes!("ui/assets/fonts/NotoMonoNerdFont/NotoMonoNerdFontMono-Regular.ttf");

pub const NOTO_SANS_FONT_BYTES: &[u8] =
    include_bytes!("ui/assets/fonts/NotoMonoNerdFont/NotoSansNerdFont-Regular.ttf");

pub const NOTO_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("NotoMono NF"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const NOTO_MONO_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("NotoMono NFM"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const NOTO_SANS_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("NotoSans NF"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

// ─── OpenDyslexic (accessibility) ───────────────────────────────────
pub const DYSLEXIC_FONT_BYTES: &[u8] =
    include_bytes!("ui/assets/fonts/OpenDyslexicFont/OpenDyslexic-Regular.otf");

pub const DYSLEXIC_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("OpenDyslexic"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

fn main() -> iced::Result {
    let icon = iced::window::icon::from_file_data(
        include_bytes!("ui/assets/icons/digger.png"),
        None,
    )
    .ok();

    iced::application(Digger::title, Digger::update, Digger::view)
        .subscription(Digger::subscription)
        .theme(Digger::theme)
        .font(NERD_FONT_BYTES)
        .font(NERD_FONT_MONO_BYTES)
        .font(SARASA_FONT_BYTES)
        .font(DEJAVU_FONT_BYTES)
        .font(DEJAVU_MONO_FONT_BYTES)
        .font(NOTO_FONT_BYTES)
        .font(NOTO_MONO_FONT_BYTES)
        .font(NOTO_SANS_FONT_BYTES)
        .font(DYSLEXIC_FONT_BYTES)
        .default_font(NERD_FONT)
        .window(iced::window::Settings {
            icon,
            size: (950.0, 680.0).into(),
            #[cfg(target_os = "linux")]
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: String::from("digger"),
                ..Default::default()
            },
            ..Default::default()
        })
        .run_with(|| (Digger::new(), iced::Task::none()))
}
