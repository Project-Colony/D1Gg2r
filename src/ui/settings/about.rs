//! Preferences -> About.

use iced::widget::{column, text, Space};
use iced::Element;

use crate::ui::settings::*;

impl Digger {
    pub(crate) fn view_settings_about(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let text_c = p.text;
        let label_c = p.label;
        let t = self.t();

        let title = column![
            text(t.about_digger)
                .size(self.typo.sz(16))
                .font(self.typo.regular)
                .color(text_c),
            text(t.about_desc)
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(label_c),
        ]
        .spacing(SPACE_XS);

        let version_section = self.section(
            SettingsSection::Version,
            t.version,
            "",
            column![
                info_row(t.application, "Digger", p, &self.typo),
                info_row(t.version, env!("CARGO_PKG_VERSION"), p, &self.typo),
                info_row(t.framework, ICED_VERSION, p, &self.typo),
                info_row(t.license, "GPL-3.0-or-later", p, &self.typo),
            ]
            .spacing(SPACE_SM)
            .into(),
        );

        let font_section = self.section(
            SettingsSection::FontInfo,
            t.fonts,
            "",
            column![
                info_row(t.ui_font, "Iosevka Nerd Font Propo", p, &self.typo),
                info_row(t.mono_font, "Iosevka Nerd Font Mono", p, &self.typo),
                info_row(t.dyslexic_font_label, "OpenDyslexic", p, &self.typo),
                info_row(t.nerd_fonts, "v3.4.0", p, &self.typo),
            ]
            .spacing(SPACE_SM)
            .into(),
        );

        // System info section
        let sys_items = if let Some(snap) = &self.current {
            column![
                info_row(t.hostname, &snap.sys_info.hostname, p, &self.typo),
                info_row(t.os, &snap.sys_info.os_name, p, &self.typo),
                info_row(t.os_version, &snap.sys_info.os_version, p, &self.typo),
                info_row(t.kernel, &snap.sys_info.kernel_version, p, &self.typo),
                info_row(t.cpu, &snap.cpu_name, p, &self.typo),
                info_row(t.cores, snap.cpu_core_count.to_string(), p, &self.typo),
                info_row(t.total_ram, format_bytes(snap.memory_total), p, &self.typo),
            ]
            .spacing(SPACE_SM)
        } else {
            column![text(t.waiting_for_data)
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(label_c),]
        };

        let system_section = self.section(
            SettingsSection::SystemInfo,
            t.system_information,
            "",
            sys_items.into(),
        );

        column![
            title,
            Space::new().height(16),
            version_section,
            Space::new().height(8),
            font_section,
            Space::new().height(8),
            system_section,
        ]
        .spacing(SPACE_XS)
        .into()
    }

    // ─── OVERVIEW TAB ───────────────────────────────────────────
}
