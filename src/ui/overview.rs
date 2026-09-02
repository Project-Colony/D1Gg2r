//! The overview tab.

use iced::widget::canvas::Canvas;
use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Background, Border, Color, Element, Length, Theme};

use crate::gauge::Sparkline;
use crate::icons::*;
use crate::message::*;
use crate::state::*;
use crate::ui::layout::*;
use crate::ui::widgets::*;

impl Digger {
    pub(crate) fn view_overview(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let t = self.t();
        let Some(snap) = &self.current else {
            return container(
                text(format!("{ICON_LOADING} {}", t.collecting_data))
                    .size(self.typo.sz(14))
                    .font(self.typo.regular)
                    .color(p.label),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        };

        // Use animated values for smooth display
        let display_cpu = self.anim_cpu;
        let display_mem = self.anim_mem_pct;

        let sidebar_bg = p.sidebar_bg;
        let border_c = p.border;

        // Mini sparkline data
        let cpu_spark_data: Vec<f32> = self.live_buffer.iter().map(|lp| lp.cpu).collect();
        let mem_spark_data: Vec<f32> = self.live_buffer.iter().map(|lp| lp.mem_pct).collect();
        let disk_io_spark: Vec<f32> = self
            .live_buffer
            .iter()
            .map(|lp| (lp.disk_read + lp.disk_write) as f32 / 1024.0)
            .collect();

        let make_spark = |data: Vec<f32>, color: Color| -> Element<'_, Message> {
            Canvas::new(Sparkline { data, color })
                .width(Length::Fill)
                .height(Length::Fixed(20.0))
                .into()
        };

        let sidebar = container(
            column![
                sidebar_item(
                    format!("{ICON_CPU} {}", t.cpu),
                    format!("{:.0}%", display_cpu),
                    dynamic_color(p.accent, display_cpu / 100.0),
                    OverviewPanel::Cpu,
                    self.overview_panel,
                    p,
                    &self.typo,
                ),
                make_spark(cpu_spark_data, p.accent),
                sidebar_item(
                    format!("{ICON_MEMORY} {}", t.memory),
                    format!("{:.0}%", display_mem),
                    dynamic_color(p.green, display_mem / 100.0),
                    OverviewPanel::Memory,
                    self.overview_panel,
                    p,
                    &self.typo,
                ),
                make_spark(mem_spark_data, p.green),
                sidebar_item(
                    format!("{ICON_DISK} {}", t.disk),
                    format!(
                        "{}/s I/O",
                        format_bytes(snap.disk_io.read_bytes + snap.disk_io.write_bytes)
                    ),
                    p.cyan,
                    OverviewPanel::Disk,
                    self.overview_panel,
                    p,
                    &self.typo,
                ),
                make_spark(disk_io_spark, p.cyan),
                sidebar_item(
                    format!("{ICON_NETWORK} {}", t.network),
                    format!("{}/s", format_bytes(snap.net_rx_bytes + snap.net_tx_bytes)),
                    p.yellow,
                    OverviewPanel::Network,
                    self.overview_panel,
                    p,
                    &self.typo,
                ),
                sidebar_item(
                    format!("{ICON_TEMP} {}", t.temp),
                    format!("{} {}", snap.temperatures.len(), t.sensors),
                    p.red,
                    OverviewPanel::Temperature,
                    self.overview_panel,
                    p,
                    &self.typo,
                ),
                sidebar_item(
                    format!("{ICON_GPU} {}", t.gpu),
                    if snap.gpu.gpus.is_empty() {
                        t.n_a.into()
                    } else {
                        format!("{} GPU(s)", snap.gpu.gpus.len())
                    },
                    p.magenta,
                    OverviewPanel::Gpu,
                    self.overview_panel,
                    p,
                    &self.typo,
                ),
                // Load Average (small display at bottom of sidebar)
                Space::new().height(Length::Fill),
                text(format!("{ICON_LOAD} {}", t.load))
                    .size(self.typo.sz(10))
                    .font(self.typo.regular)
                    .color(p.label),
                text(format!(
                    "{:.2}  {:.2}  {:.2}",
                    snap.load_avg[0], snap.load_avg[1], snap.load_avg[2]
                ))
                .size(self.typo.sz(10))
                .font(self.typo.regular)
                .color(p.text),
            ]
            .spacing(SPACE_2XS)
            .padding(4),
        )
        .width(160)
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

        let detail = match self.overview_panel {
            OverviewPanel::Cpu => self.view_detail_cpu(snap),
            OverviewPanel::Memory => self.view_detail_memory(snap),
            OverviewPanel::Network => self.view_detail_network(snap),
            OverviewPanel::Disk => self.view_detail_disk(snap),
            OverviewPanel::Temperature => self.view_detail_temp(snap),
            OverviewPanel::Gpu => self.view_detail_gpu(snap),
        };

        row![
            sidebar,
            scrollable(container(detail).width(Length::Fill).padding(6)),
        ]
        .spacing(0)
        .height(Length::Fill)
        .into()
    }
}
