//! Preferences -> General.

use iced::widget::{button, column, row, text, Column, Row, Space};
use iced::{Alignment, Element, Length};

use crate::ui::settings::*;

impl Digger {
    pub(crate) fn view_settings_general(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let text_c = p.text;
        let label_c = p.label;
        let accent = p.accent;
        let green = p.green;
        let t = self.t();

        let title = column![
            text(t.general_settings)
                .size(self.typo.sz(16))
                .font(self.typo.regular)
                .color(text_c),
            text(t.settings_saved_auto)
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(label_c),
        ]
        .spacing(4);

        let mut rate_btns: Vec<Element<Message>> = Vec::new();
        for &secs in REFRESH_OPTIONS {
            let is_active = self.refresh_interval_secs == secs;
            let color = if is_active { accent } else { label_c };
            let btn = button(
                text(format!("{secs}s"))
                    .size(self.typo.sz(11))
                    .font(self.typo.regular)
                    .color(color),
            )
            .on_press(Message::SetRefreshInterval(secs))
            .style(if is_active {
                button::primary
            } else {
                button::secondary
            })
            .padding([4, 12]);
            rate_btns.push(btn.into());
        }

        let refresh_row = row![
            column![
                text(t.refresh_rate)
                    .size(self.typo.sz(12))
                    .font(self.typo.regular)
                    .color(text_c),
                text(t.refresh_rate_desc)
                    .size(self.typo.sz(10))
                    .font(self.typo.regular)
                    .color(label_c),
            ]
            .spacing(2)
            .width(Length::FillPortion(2)),
            Row::with_children(rate_btns).spacing(4),
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let temp_toggle = button(
            text(if self.temp_celsius {
                ICON_TOGGLE_ON
            } else {
                ICON_TOGGLE_OFF
            })
            .size(self.typo.sz(22))
            .color(if self.temp_celsius { accent } else { label_c }),
        )
        .on_press(Message::ToggleTempUnit)
        .style(button::text)
        .padding(0);

        let temp_label = if self.temp_celsius {
            format!("{} (\u{00b0}C)", t.celsius)
        } else {
            format!("{} (\u{00b0}F)", t.fahrenheit)
        };

        let temp_row = row![
            column![
                text(t.temperature_unit)
                    .size(self.typo.sz(12))
                    .font(self.typo.regular)
                    .color(text_c),
                text(format!("{} {temp_label}", t.currently))
                    .size(self.typo.sz(10))
                    .font(self.typo.regular)
                    .color(label_c),
            ]
            .spacing(2)
            .width(Length::FillPortion(2)),
            temp_toggle,
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let monitoring_section = self.section(
            SettingsSection::Monitoring,
            t.monitoring,
            t.monitoring_desc,
            column![refresh_row, Space::new().height(12), temp_row,].into(),
        );

        let process_limit_row = row![
            column![
                text(t.process_limit)
                    .size(self.typo.sz(12))
                    .font(self.typo.regular)
                    .color(text_c),
                text(t.process_limit_desc)
                    .size(self.typo.sz(10))
                    .font(self.typo.regular)
                    .color(label_c),
            ]
            .spacing(2)
            .width(Length::FillPortion(2)),
            text(format!("{}", self.process_limit))
                .size(self.typo.sz(12))
                .font(self.typo.regular)
                .color(accent),
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let history_points_row = row![
            column![
                text(t.history_buffer)
                    .size(self.typo.sz(12))
                    .font(self.typo.regular)
                    .color(text_c),
                text(t.history_buffer_desc)
                    .size(self.typo.sz(10))
                    .font(self.typo.regular)
                    .color(label_c),
            ]
            .spacing(2)
            .width(Length::FillPortion(2)),
            text(format!("{}", self.live_max))
                .size(self.typo.sz(12))
                .font(self.typo.regular)
                .color(accent),
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let retention_row = row![
            column![
                text(t.history_retention)
                    .size(self.typo.sz(12))
                    .font(self.typo.regular)
                    .color(text_c),
                text(t.history_retention_desc)
                    .size(self.typo.sz(10))
                    .font(self.typo.regular)
                    .color(label_c),
            ]
            .spacing(2)
            .width(Length::FillPortion(2)),
            text(format!("{}h", self.retention_hours))
                .size(self.typo.sz(12))
                .font(self.typo.regular)
                .color(accent),
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let display_section = self.section(
            SettingsSection::Display,
            t.display,
            t.display_desc,
            column![
                process_limit_row,
                Space::new().height(12),
                history_points_row,
                Space::new().height(12),
                retention_row,
            ]
            .into(),
        );

        let db_status = if self.history.is_available() {
            format!("{ICON_CHECK} {}", t.active)
        } else {
            format!("{ICON_WARNING} {}", t.unavailable)
        };
        let db_color = if self.history.is_available() {
            green
        } else {
            p.red
        };

        let mut data_items: Vec<Element<Message>> = vec![row![
            column![
                text(t.history_database)
                    .size(self.typo.sz(12))
                    .font(self.typo.regular)
                    .color(text_c),
                text(t.history_database_desc)
                    .size(self.typo.sz(10))
                    .font(self.typo.regular)
                    .color(label_c),
            ]
            .spacing(2)
            .width(Length::FillPortion(2)),
            text(db_status)
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(db_color),
        ]
        .align_y(Alignment::Center)
        .spacing(12)
        .into()];

        // Show DB error if any
        if let Some(err) = &self.history.last_error {
            data_items.push(Space::new().height(6).into());
            data_items.push(
                text(format!("{ICON_WARNING} {err}"))
                    .size(self.typo.sz(10))
                    .color(p.red)
                    .into(),
            );
        }

        let data_section = self.section(
            SettingsSection::Data,
            t.data,
            "",
            Column::with_children(data_items).spacing(0).into(),
        );

        // Alert thresholds section
        let cpu_alert_btns = make_threshold_buttons(
            self.cpu_alert_threshold,
            &[70.0, 80.0, 90.0, 95.0],
            Message::SetCpuAlertThreshold,
            accent,
            label_c,
            &self.typo,
        );
        let mem_alert_btns = make_threshold_buttons(
            self.mem_alert_threshold,
            &[70.0, 80.0, 90.0, 95.0],
            Message::SetMemAlertThreshold,
            accent,
            label_c,
            &self.typo,
        );

        let alerts_section = self.section(
            SettingsSection::Alerts,
            t.alerts,
            t.alerts_desc,
            column![
                row![
                    column![
                        text(t.cpu_threshold)
                            .size(self.typo.sz(12))
                            .font(self.typo.regular)
                            .color(text_c),
                        text(t.cpu_threshold_desc)
                            .size(self.typo.sz(10))
                            .font(self.typo.regular)
                            .color(label_c),
                    ]
                    .spacing(2)
                    .width(Length::FillPortion(2)),
                    cpu_alert_btns,
                ]
                .align_y(Alignment::Center)
                .spacing(12),
                Space::new().height(12),
                row![
                    column![
                        text(t.memory_threshold)
                            .size(self.typo.sz(12))
                            .font(self.typo.regular)
                            .color(text_c),
                        text(t.memory_threshold_desc)
                            .size(self.typo.sz(10))
                            .font(self.typo.regular)
                            .color(label_c),
                    ]
                    .spacing(2)
                    .width(Length::FillPortion(2)),
                    mem_alert_btns,
                ]
                .align_y(Alignment::Center)
                .spacing(12),
            ]
            .into(),
        );

        column![
            title,
            Space::new().height(16),
            monitoring_section,
            Space::new().height(6),
            display_section,
            Space::new().height(6),
            data_section,
            Space::new().height(6),
            alerts_section,
        ]
        .spacing(4)
        .into()
    }
}
