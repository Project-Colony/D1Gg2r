//! The events tab.

use iced::widget::{column, container, row, scrollable, text, Column, Space};
use iced::{Alignment, Background, Element, Length, Theme};

use crate::icons::*;
use crate::message::*;
use crate::state::*;
use crate::ui::widgets::*;

impl Digger {
    pub(crate) fn view_event_log(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let t = self.t();
        let label_c = p.label;
        let panel_bg = p.panel_bg;
        let bg = p.bg;

        let title_row = row![
            text(format!("{ICON_LOG} {}", t.event_log))
                .size(self.typo.sz(13))
                .font(self.typo.regular)
                .color(p.accent),
            Space::new().width(Length::Fill),
            text(format!("{} {}", self.event_log.len(), t.events))
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(label_c),
        ]
        .padding([6, 10])
        .align_y(Alignment::Center);

        let mut rows: Vec<Element<Message>> = Vec::new();

        if self.event_log.is_empty() {
            rows.push(
                container(
                    text(t.no_events)
                        .size(self.typo.sz(12))
                        .font(self.typo.regular)
                        .color(label_c),
                )
                .padding([20, 10])
                .center_x(Length::Fill)
                .into(),
            );
        } else {
            for (i, ev) in self.event_log.iter().rev().enumerate() {
                let sev_color = match ev.severity {
                    EventSeverity::Info => p.green,
                    EventSeverity::Warning => p.yellow,
                    EventSeverity::Critical => p.red,
                };
                let row_bg = if i % 2 == 0 { panel_bg } else { bg };
                let r = container(
                    row![
                        text(&*ev.timestamp)
                            .size(self.typo.sz(10))
                            .font(self.typo.regular)
                            .color(label_c)
                            .width(80),
                        text(ev.icon)
                            .size(self.typo.sz(11))
                            .color(sev_color)
                            .width(20),
                        text(&ev.message).size(self.typo.sz(11)).color(p.text),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([3, 10])
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(row_bg)),
                    ..Default::default()
                });
                rows.push(r.into());
            }
        }

        let table = Column::with_children(rows).spacing(0);
        let content = panel(column![title_row, table].spacing(0).into(), p);

        scrollable(column![content].padding(4)).into()
    }

    // ─── SETTINGS VIEW ─────────────────────────────────────────
}
