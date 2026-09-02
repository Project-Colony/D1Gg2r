//! The history tab.

use iced::widget::{button, column, container, scrollable, text, Row, Space};
use iced::{Element, Length};

use crate::icons::*;
use crate::message::*;
use crate::state::*;
use crate::ui::widgets::*;

impl Digger {
    pub(crate) fn view_history(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let t = self.t();
        let cc = self.chart_colors();
        let label_c = p.label;
        let accent = p.accent;

        let mut range_btns: Vec<Element<Message>> = Vec::new();
        range_btns.push(
            text(format!("{ICON_CLOCK} {}", t.range))
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(label_c)
                .into(),
        );
        range_btns.push(Space::new().width(4).into());
        for (i, (_, label)) in HISTORY_RANGES.iter().enumerate() {
            let is_active = self.history_range_idx == i;
            let color = if is_active { accent } else { label_c };
            let btn = button(text(*label).size(self.typo.sz(11)).color(color))
                .on_press(Message::HistoryRangeSelected(i))
                .style(if is_active {
                    button::primary
                } else {
                    button::secondary
                })
                .padding([3, 10]);
            range_btns.push(btn.into());
        }

        // Export buttons
        range_btns.push(Space::new().width(Length::Fill).into());
        range_btns.push(
            button(
                text(format!("{ICON_EXPORT} CSV"))
                    .size(self.typo.sz(11))
                    .color(label_c),
            )
            .on_press(Message::ExportCsv)
            .style(button::secondary)
            .padding([3, 10])
            .into(),
        );
        range_btns.push(
            button(
                text(format!("{ICON_EXPORT} JSON"))
                    .size(self.typo.sz(11))
                    .color(label_c),
            )
            .on_press(Message::ExportJson)
            .style(button::secondary)
            .padding([3, 10])
            .into(),
        );

        let range_row = Row::with_children(range_btns).spacing(4).padding([6, 10]);

        if self.history_points.is_empty() {
            return panel(
                column![
                    range_row,
                    Space::new().height(20),
                    container(
                        text(format!("{ICON_HISTORY} {}", t.no_history_data))
                            .size(self.typo.sz(13))
                            .font(self.typo.regular)
                            .color(label_c)
                    )
                    .center_x(Length::Fill),
                    Space::new().height(20),
                ]
                .spacing(4)
                .into(),
                p,
            );
        }

        const MAX_PTS: usize = 600;

        let cpu_data = downsample(
            &self
                .history_points
                .iter()
                .map(|h| h.cpu)
                .collect::<Vec<_>>(),
            MAX_PTS,
        );
        let cpu_chart = make_chart(ChartCfg {
            title: format!("{ICON_CPU} {}", t.cpu_history),
            series: vec![("CPU".into(), p.accent, cpu_data)],
            y_min: 0.0,
            y_max: 100.0,
            filled: true,
            height: 140.0,
            unit: "%".into(),
            colors: cc,
        });

        let mem_data = downsample(
            &self
                .history_points
                .iter()
                .map(|h| {
                    if h.mem_total > 0 {
                        h.mem_used as f32 / h.mem_total as f32 * 100.0
                    } else {
                        0.0
                    }
                })
                .collect::<Vec<_>>(),
            MAX_PTS,
        );
        let mem_chart = make_chart(ChartCfg {
            title: format!("{ICON_MEMORY} {}", t.memory_history),
            series: vec![("RAM".into(), p.green, mem_data)],
            y_min: 0.0,
            y_max: 100.0,
            filled: true,
            height: 140.0,
            unit: "%".into(),
            colors: cc,
        });

        let rx_kb = downsample(
            &self
                .history_points
                .iter()
                .map(|h| h.net_rx as f32 / 1024.0)
                .collect::<Vec<_>>(),
            MAX_PTS,
        );
        let tx_kb = downsample(
            &self
                .history_points
                .iter()
                .map(|h| h.net_tx as f32 / 1024.0)
                .collect::<Vec<_>>(),
            MAX_PTS,
        );
        let hist_max_kb = rx_kb
            .iter()
            .chain(tx_kb.iter())
            .cloned()
            .fold(0.001f32, f32::max);
        let (h_rx, h_tx, h_unit, h_ymax) = if hist_max_kb >= 1024.0 {
            let rx_mb: Vec<f32> = rx_kb.iter().map(|v| v / 1024.0).collect();
            let tx_mb: Vec<f32> = tx_kb.iter().map(|v| v / 1024.0).collect();
            (rx_mb, tx_mb, " MB/s", hist_max_kb / 1024.0)
        } else {
            (rx_kb, tx_kb, " KB/s", hist_max_kb)
        };
        let net_chart = make_chart(ChartCfg {
            title: format!("{ICON_NETWORK} {}", t.network_history),
            series: vec![
                (format!("{ICON_ARROW_DOWN} rx"), p.green, h_rx),
                (format!("{ICON_ARROW_UP} tx"), p.red, h_tx),
            ],
            y_min: 0.0,
            y_max: h_ymax,
            filled: true,
            height: 140.0,
            unit: h_unit.into(),
            colors: cc,
        });

        let content = column![
            panel(column![range_row, cpu_chart].spacing(6).into(), p),
            panel(mem_chart, p),
            panel(net_chart, p),
        ]
        .spacing(4)
        .padding(4);

        scrollable(content).into()
    }
}
