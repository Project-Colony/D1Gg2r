//! Preferences -> Language.

use iced::widget::{button, column, container, row, text, Column, Row, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::i18n::Language;
use crate::ui::settings::*;

impl Digger {
    pub(crate) fn view_settings_language(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let text_c = p.text;
        let label_c = p.label;
        let accent = p.accent;
        let panel_bg = p.panel_bg;
        let border_c = p.border;
        let t = self.t();

        let title = column![
            text(t.language)
                .size(self.typo.sz(16))
                .font(self.typo.regular)
                .color(text_c),
            text(t.language_desc)
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(label_c),
        ]
        .spacing(SPACE_XS);

        // Current language info
        let current_info = container(row![
            {
                let name = if has_native_font(self.language) {
                    self.language.native_name()
                } else {
                    self.language.english_name()
                };
                text(format!("{ICON_CHECK} {name}"))
                    .size(self.typo.sz(12))
                    .color(accent)
                    .font(font_for_lang(self.language))
            },
            Space::new().width(8),
            text(format!("({})", self.language.code()))
                .size(self.typo.sz(11))
                .color(label_c),
        ])
        .padding(PAD_CARD)
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(panel_bg)),
            border: Border {
                color: border_c,
                width: 1.0,
                radius: RADIUS_CONTROL.into(),
            },
            ..Default::default()
        });

        // Language grid: 2 columns
        let all = Language::ALL;
        let rows_count = all.len().div_ceil(2);
        let mut grid_rows: Vec<Element<Message>> = Vec::new();
        for r in 0..rows_count {
            let mut cols: Vec<Element<Message>> = Vec::new();
            for c in 0..2 {
                let idx = r + c * rows_count;
                if idx < all.len() {
                    let lang = all[idx];
                    let is_active = self.language == lang;
                    let lang_accent = accent;
                    let lang_label_c = label_c;
                    let lang_text_c = text_c;
                    let lang_panel_bg = panel_bg;
                    let lang_border_c = border_c;
                    let btn_border = if is_active {
                        lang_accent
                    } else {
                        lang_border_c
                    };
                    let btn_width = if is_active { 2.0 } else { 1.0 };
                    let active_bg = if is_active {
                        Color::from_rgba(lang_accent.r, lang_accent.g, lang_accent.b, 0.1)
                    } else {
                        lang_panel_bg
                    };
                    let hover_bg =
                        Color::from_rgba(lang_accent.r, lang_accent.g, lang_accent.b, 0.05);

                    let check: Element<Message> = if is_active {
                        text(ICON_CHECK)
                            .size(self.typo.sz(11))
                            .color(lang_accent)
                            .into()
                    } else {
                        Space::new().into()
                    };

                    let content = row![
                        check,
                        Space::new().width(4),
                        column![
                            {
                                let name_color = if is_active { lang_accent } else { lang_text_c };
                                let name = if has_native_font(lang) {
                                    lang.native_name()
                                } else {
                                    lang.english_name()
                                };
                                text(name)
                                    .size(self.typo.sz(11))
                                    .font(font_for_lang(lang))
                                    .color(name_color)
                            },
                            text(lang.code()).size(self.typo.sz(9)).color(lang_label_c),
                        ]
                        .spacing(SPACE_2XS),
                    ]
                    .align_y(Alignment::Center);

                    let btn = button(content)
                        .on_press(Message::SetLanguage(lang))
                        .width(Length::Fill)
                        .padding(PAD_ROW)
                        .style(move |_: &Theme, status| {
                            let bg = match status {
                                button::Status::Hovered => hover_bg,
                                button::Status::Pressed => active_bg,
                                _ => active_bg,
                            };
                            button::Style {
                                background: Some(Background::Color(bg)),
                                text_color: lang_text_c,
                                border: Border {
                                    color: btn_border,
                                    width: btn_width,
                                    radius: RADIUS_CARD.into(),
                                },
                                ..Default::default()
                            }
                        });
                    cols.push(container(btn).width(Length::FillPortion(1)).into());
                } else {
                    cols.push(Space::new().width(Length::FillPortion(1)).into());
                }
            }
            grid_rows.push(Row::with_children(cols).spacing(SPACE_SM).into());
        }
        let grid = Column::with_children(grid_rows).spacing(SPACE_2XS);

        column![
            title,
            Space::new().height(8),
            current_info,
            Space::new().height(12),
            grid,
        ]
        .spacing(SPACE_XS)
        .into()
    }
}
