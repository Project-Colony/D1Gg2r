//! Preferences -> Accessibility.

use iced::widget::{column, text, Space};
use iced::Element;

use crate::preferences::TEXT_SCALES;
use crate::ui::settings::*;

impl Digger {
    pub(crate) fn view_settings_accessibility(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let t = self.t();

        let title = column![
            text(t.accessibility)
                .size(self.typo.sz(16))
                .font(self.typo.regular)
                .color(p.text),
            text(t.accessibility_desc)
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(p.label),
        ]
        .spacing(4);

        let vision = self.section(
            SettingsSection::Vision,
            t.vision,
            t.vision_desc,
            colony_ui::widgets::functional_toggle(
                &self.typo,
                t.high_contrast,
                t.high_contrast_desc,
                self.high_contrast,
                Message::ToggleHighContrast,
            ),
        );

        let motion = self.section(
            SettingsSection::Motion,
            t.motion,
            t.motion_desc,
            colony_ui::widgets::functional_toggle(
                &self.typo,
                t.reduced_motion,
                t.reduced_motion_desc,
                self.reduced_motion,
                Message::ToggleReducedMotion,
            ),
        );

        let reading = self.section(
            SettingsSection::Reading,
            t.reading,
            t.reading_desc,
            self.scale_picker(
                t.text_size,
                t.text_size_desc,
                TEXT_SCALES,
                self.text_scale,
                Message::SetTextScale,
            ),
        );

        let fonts = self.section(
            SettingsSection::Fonts,
            t.fonts,
            t.fonts_desc,
            colony_ui::widgets::functional_toggle(
                &self.typo,
                t.dyslexic_font,
                t.dyslexic_font_desc,
                self.use_dyslexic_font,
                Message::ToggleDyslexicFont,
            ),
        );

        column![
            title,
            Space::new().height(16),
            vision,
            Space::new().height(6),
            motion,
            Space::new().height(6),
            reading,
            Space::new().height(6),
            fonts,
        ]
        .spacing(4)
        .into()
    }
}
