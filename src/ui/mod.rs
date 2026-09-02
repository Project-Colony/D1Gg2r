//! The window: the menu bar, the tab shell, and the modules each tab lives in.

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Background, Element, Length, Theme};

use crate::icons::*;
use crate::message::*;
use crate::state::*;
use crate::ui::layout::*;
use crate::ui::widgets::*;

pub mod detail;
pub mod events;
pub mod history;
pub mod layout;
pub mod overview;
pub mod processes;
pub mod settings;
pub mod widgets;

impl Digger {
    pub fn view(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let tabs = row![
            menu_tab(
                &self.cached_tab_overview,
                Tab::Overview,
                self.tab,
                p,
                &self.typo
            ),
            menu_tab(
                &self.cached_tab_processes,
                Tab::Processes,
                self.tab,
                p,
                &self.typo
            ),
            menu_tab(
                &self.cached_tab_history,
                Tab::History,
                self.tab,
                p,
                &self.typo
            ),
            menu_tab(
                &self.cached_tab_events,
                Tab::EventLog,
                self.tab,
                p,
                &self.typo
            ),
        ]
        .spacing(SPACE_XS);

        let digger_label = if self.show_settings {
            &self.cached_digger_label_settings
        } else {
            &self.cached_digger_label
        };
        let accent = p.accent;
        let digger_btn = button(text(digger_label).size(self.typo.sz(15)).color(accent))
            .on_press(Message::ToggleSettings)
            .style(button::text)
            .padding(PAD_TIGHT);

        let border_c = p.border;
        let text_c = p.text;

        // Heartbeat BPM indicator with pulsing icon
        let bpm = self.health_score;
        let heart_color = if bpm < 100.0 {
            p.green
        } else if bpm <= 130.0 {
            p.yellow
        } else {
            p.red
        };
        // Sharp beat curve: sin clamped to positive half, squared for snappy
        // pulse. Flat under reduced motion — the colour still carries the
        // health reading, so the icon loses its beat and nothing else.
        let beat = if self.reduced_motion {
            0.0
        } else {
            self.heart_phase.sin().max(0.0).powi(2)
        };
        let heart_size = 10.0 + beat * 4.0; // 10px base, up to 14px on beat
        let health_el: Element<Message> = row![
            container(text(ICON_HEART).size(heart_size).color(heart_color))
                .width(16)
                .height(16)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
            text(format!(" {:.0}", bpm))
                .size(self.typo.sz(10))
                .font(self.typo.regular)
                .color(heart_color),
        ]
        .spacing(0)
        .align_y(Alignment::Center)
        .into();

        // Status bar with alerts/errors/messages
        let status_el: Element<Message> = if let Some(msg) = &self.status_message {
            let warning_color = p.yellow;
            text(msg).size(self.typo.sz(10)).color(warning_color).into()
        } else {
            Space::new().into()
        };

        // Event log badge
        let event_count = self.event_log.len();
        let event_badge: Element<Message> = if event_count > 0 {
            let badge_color =
                if self.event_log.back().map(|e| e.severity) == Some(EventSeverity::Critical) {
                    p.red
                } else {
                    p.yellow
                };
            row![
                text(ICON_LOG).size(self.typo.sz(10)).color(badge_color),
                text(format!(" {}", event_count))
                    .size(self.typo.sz(10))
                    .font(self.typo.regular)
                    .color(badge_color),
            ]
            .spacing(0)
            .align_y(Alignment::Center)
            .into()
        } else {
            Space::new().into()
        };

        let menu_bar = row![
            digger_btn,
            Space::new().width(8),
            health_el,
            Space::new().width(6),
            event_badge,
            Space::new().width(8),
            text(ICON_SEPARATOR).size(self.typo.sz(14)).color(border_c),
            Space::new().width(8),
            status_el,
            Space::new().width(Length::Fill),
            tabs,
            Space::new().width(Length::Fill),
            text(chrono::Local::now().format("%H:%M:%S").to_string())
                .size(self.typo.sz(13))
                .font(self.typo.regular)
                .color(text_c),
        ]
        .align_y(Alignment::Center)
        .padding(PAD_ROW);

        let content: Element<Message> = if self.show_settings {
            self.view_settings()
        } else {
            match self.tab {
                Tab::Overview => self.view_overview(),
                Tab::Processes => self.view_processes(),
                Tab::History => self.view_history(),
                Tab::EventLog => self.view_event_log(),
            }
        };

        let bg = p.bg;
        let sidebar_bg = p.sidebar_bg;
        let main = column![panel_bg(menu_bar.into(), sidebar_bg, border_c), content,].spacing(0);

        container(main)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(bg)),
                ..Default::default()
            })
            .into()
    }

    // ─── EVENT LOG TAB ─────────────────────────────────────────
}
