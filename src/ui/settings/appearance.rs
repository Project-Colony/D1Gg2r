//! Preferences -> Appearance.

use iced::widget::{column, text, Space};
use iced::Element;

use crate::preferences::FONT_SCALES;
use crate::ui::settings::*;

impl Digger {
    pub(crate) fn view_settings_appearance(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let t = self.t();

        let title = column![
            text(t.appearance)
                .size(self.typo.sz(16))
                .font(self.typo.regular)
                .color(p.text),
            text(t.appearance_desc)
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(p.label),
        ]
        .spacing(4);

        // The picker renders every family colony-ui ships — fifty-nine variants
        // where Digger used to hardcode eleven — and draws each card from the
        // catalog's own swatch. Adding a family upstream shows up here on the
        // next `cargo update`, with nothing to change in Digger.
        let theme_section = self.section(
            SettingsSection::Theme,
            t.theme,
            t.theme_desc,
            colony_ui::widgets::theme_picker(
                &self.typo,
                &self.theme_variant.family,
                &self.theme_variant.variant,
                |family, variant| Message::SetTheme {
                    family: family.to_string(),
                    variant: variant.to_string(),
                },
            ),
        );

        // Clicking the selected accent clears it. Without that there is no way
        // back to "follow the theme" once an override has been set, and the
        // override is not a colour the user can otherwise reproduce.
        let selected_accent = self.accent_color.key().map(str::to_string);
        let accent_section = self.section(
            SettingsSection::Accent,
            t.accent_color,
            t.accent_color_desc,
            colony_ui::widgets::accent_picker(&self.typo, self.accent_color.key(), move |key| {
                Message::SetAccent(if selected_accent.as_deref() == Some(key) {
                    None
                } else {
                    Some(key.to_string())
                })
            }),
        );

        let typography_section = self.section(
            SettingsSection::Typography,
            t.typography,
            t.typography_desc,
            self.scale_picker(
                t.font_size,
                t.font_size_desc,
                FONT_SCALES,
                self.font_scale,
                Message::SetFontScale,
            ),
        );

        column![
            title,
            Space::new().height(12),
            theme_section,
            Space::new().height(6),
            accent_section,
            Space::new().height(6),
            typography_section,
        ]
        .spacing(4)
        .into()
    }
}
