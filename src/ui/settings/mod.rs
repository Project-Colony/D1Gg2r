//! The preferences page: the category sidebar, and the shape every section
//! inside a category shares.

use iced::widget::{button, column, container, row, scrollable, text, Row, Space};
use iced::{Background, Border, Color, Element, Length, Theme};

use crate::icons::*;
use crate::message::*;
use crate::state::*;
use crate::ui::layout::*;
use crate::ui::widgets::*;

pub mod about;
pub mod accessibility;
pub mod appearance;
pub mod general;
pub mod language;

impl Digger {
    pub(crate) fn view_settings(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let sidebar_bg = p.sidebar_bg;
        let border_c = p.border;

        let sidebar = container(
            column![
                settings_sidebar_item(
                    format!("{ICON_SETTINGS}  {}", self.t().general_settings),
                    SettingsPanel::General,
                    self.settings_panel,
                    p,
                    &self.typo,
                ),
                settings_sidebar_item(
                    format!("{ICON_PAINT}  {}", self.t().appearance),
                    SettingsPanel::Appearance,
                    self.settings_panel,
                    p,
                    &self.typo,
                ),
                settings_sidebar_item(
                    format!("{ICON_ACCESS}  {}", self.t().accessibility),
                    SettingsPanel::Accessibility,
                    self.settings_panel,
                    p,
                    &self.typo,
                ),
                settings_sidebar_item(
                    format!("{ICON_NETWORK}  {}", self.t().language),
                    SettingsPanel::Language,
                    self.settings_panel,
                    p,
                    &self.typo,
                ),
                settings_sidebar_item(
                    format!("{ICON_INFO}  {}", self.t().about_digger),
                    SettingsPanel::About,
                    self.settings_panel,
                    p,
                    &self.typo,
                ),
            ]
            .spacing(SPACE_2XS)
            .padding(8),
        )
        .width(170)
        .height(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(sidebar_bg)),
            border: Border {
                color: border_c,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        let detail = match self.settings_panel {
            SettingsPanel::General => self.view_settings_general(),
            SettingsPanel::Appearance => self.view_settings_appearance(),
            SettingsPanel::Accessibility => self.view_settings_accessibility(),
            SettingsPanel::Language => self.view_settings_language(),
            SettingsPanel::About => self.view_settings_about(),
        };

        row![
            sidebar,
            scrollable(container(detail).width(Length::Fill).padding(16)),
        ]
        .spacing(0)
        .height(Length::Fill)
        .into()
    }

    /// One collapsible settings section, in the shared shape: a flat header row
    /// carrying the title and a chevron, and the description as the first line
    /// of the body rather than a second line in the header — a closed category
    /// should read as a short list of titles.
    pub(crate) fn section<'a>(
        &'a self,
        id: SettingsSection,
        title: &'a str,
        description: &'a str,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let body = column![
            text(description)
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(self.pal.label),
            Space::new().height(8),
            content,
        ];
        colony_ui::widgets::collapsible_section(
            &self.typo,
            title,
            !self.collapsed_sections.contains(&id),
            Message::ToggleSection(id),
            body.into(),
        )
    }

    /// A row of size steps, the selected one filled with the accent.
    ///
    /// Deliberately not a slider: there are three or four steps, each with a
    /// name, and a slider would invite the values in between — which the
    /// preferences sanitiser snaps away anyway.
    pub(crate) fn scale_picker<'a>(
        &'a self,
        title: &'a str,
        description: &'a str,
        steps: &'static [f32],
        current: f32,
        on_select: fn(usize) -> Message,
    ) -> Element<'a, Message> {
        let t = self.t();
        let label_for = |scale: f32| match scale {
            s if s < 0.9 => t.size_small,
            s if s < 1.1 => t.size_default,
            s if s < 1.3 => t.size_large,
            _ => t.size_xlarge,
        };

        let mut buttons: Vec<Element<Message>> = Vec::new();
        for (idx, &step) in steps.iter().enumerate() {
            // Compare against the step rather than an index: the stored value is
            // a multiplier, and the two pickers do not offer the same steps.
            let selected = (current - step).abs() < f32::EPSILON;
            let (bg, fg) = if selected {
                (self.pal.accent, colony_ui::contrast_on(self.pal.accent))
            } else {
                (self.pal.panel_bg, self.pal.label)
            };
            buttons.push(
                button(
                    text(label_for(step))
                        // The label previews the size it selects, which is the
                        // whole point of a size setting.
                        .size(self.typo.sz(12) * step)
                        .font(self.typo.regular)
                        .color(fg),
                )
                .on_press(on_select(idx))
                .padding(PAD_ROW)
                .style(move |_: &Theme, _status| button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: fg,
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: RADIUS_CONTROL.into(),
                    },
                    ..Default::default()
                })
                .into(),
            );
        }

        column![
            text(title)
                .size(self.typo.sz(12))
                .font(self.typo.regular)
                .color(self.pal.text),
            text(description)
                .size(self.typo.sz(10))
                .font(self.typo.regular)
                .color(self.pal.label),
            Space::new().height(8),
            Row::with_children(buttons).spacing(SPACE_MD),
        ]
        .spacing(SPACE_2XS)
        .into()
    }
}
