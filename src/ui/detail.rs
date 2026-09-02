//! The detail tab.

use iced::widget::canvas::Canvas;
use iced::widget::{column, container, row, text, Column, Row, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme, Vector};

use crate::gauge::{GaugeColors, RadialGauge};
use crate::icons::*;
use crate::message::*;
use crate::metrics::Snapshot;
use crate::state::*;
use crate::ui::widgets::*;

impl Digger {
    // ─── CPU Detail ──
    pub(crate) fn view_detail_cpu<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let cc = self.chart_colors();
        let cpu_data: Vec<f32> = self.live_buffer.iter().map(|p| p.cpu).collect();
        // Pulse effect: if CPU exceeds threshold, pulse the chart title
        let is_critical = self.anim_cpu >= self.cpu_alert_threshold;
        let pulse_alpha = if is_critical {
            self.pulse_opacity()
        } else {
            1.0
        };
        let title_color = if is_critical {
            Color::from_rgba(p.red.r, p.red.g, p.red.b, pulse_alpha)
        } else {
            p.accent
        };
        // Radial gauge for CPU
        let gc = GaugeColors {
            bg: p.panel_bg,
            label: p.label,
            text: p.text,
            bar_bg: p.bar_bg,
        };
        let cpu_gauge: Element<Message> = Canvas::new(RadialGauge {
            value: self.anim_cpu,
            label: "CPU".into(),
            color: title_color,
            colors: gc,
        })
        .width(Length::Fixed(120.0))
        .height(Length::Fixed(100.0))
        .into();

        let cpu_chart = make_chart(ChartCfg {
            title: format!("CPU {ICON_DASH} {:.1}%", self.anim_cpu),
            series: vec![("CPU".into(), title_color, cpu_data)],
            y_min: 0.0,
            y_max: 100.0,
            filled: true,
            height: 180.0,
            unit: "%".into(),
            colors: cc,
        });

        // Load average info
        let load_info: Row<Message> = row![
            text(format!("{ICON_LOAD} {}", t.load_avg))
                .size(self.typo.sz(10))
                .font(self.typo.regular)
                .color(p.label),
            text(format!(" 1m {:.2}", snap.load_avg[0]))
                .size(self.typo.sz(10))
                .font(self.typo.regular)
                .color(p.text),
            text(format!("  5m {:.2}", snap.load_avg[1]))
                .size(self.typo.sz(10))
                .font(self.typo.regular)
                .color(p.text),
            text(format!("  15m {:.2}", snap.load_avg[2]))
                .size(self.typo.sz(10))
                .font(self.typo.regular)
                .color(p.text),
        ]
        .spacing(2)
        .align_y(Alignment::Center);

        // Use animated per-core values
        let cores = &self.anim_cores;
        let num_cols = if cores.len() > 16 {
            4
        } else if cores.len() > 8 {
            3
        } else {
            2
        };
        let rows_count = cores.len().div_ceil(num_cols);
        let mut grid_rows: Vec<Element<Message>> = Vec::new();
        for r in 0..rows_count {
            let mut cols: Vec<Element<Message>> = Vec::new();
            for c in 0..num_cols {
                let idx = r + c * rows_count;
                if idx < cores.len() {
                    let usage = cores[idx];
                    let color = gradient_color(usage / 100.0, p);
                    let core = row![
                        text(format!("C{idx:<2}"))
                            .size(self.typo.sz(10))
                            .font(self.typo.regular)
                            .color(p.label)
                            .width(26),
                        themed_bar(usage, color, p.bar_bg),
                        text(format!("{usage:>3.0}%"))
                            .size(self.typo.sz(10))
                            .font(self.typo.regular)
                            .color(color)
                            .width(36),
                    ]
                    .spacing(2)
                    .align_y(Alignment::Center);
                    cols.push(container(core).width(Length::FillPortion(1)).into());
                } else {
                    cols.push(Space::new().width(Length::FillPortion(1)).into());
                }
            }
            grid_rows.push(Row::with_children(cols).spacing(8).into());
        }
        let cores_grid = Column::with_children(grid_rows).spacing(1);

        let uptime = format_duration(snap.uptime_secs);
        let info = column![
            info_row(t.model, &snap.cpu_name, p, &self.typo),
            info_row(
                t.logical_cores,
                snap.cpu_core_count.to_string(),
                p,
                &self.typo
            ),
            info_row(
                t.base_speed,
                format!("{} MHz", snap.cpu_frequency_mhz),
                p,
                &self.typo
            ),
            info_row(
                t.utilization,
                format!("{:.1}%", self.anim_cpu),
                p,
                &self.typo
            ),
            info_row(t.processes, snap.process_count.to_string(), p, &self.typo),
            info_row(t.uptime, &uptime, p, &self.typo),
        ]
        .spacing(4);

        panel(
            column![
                row![cpu_gauge, column![cpu_chart].width(Length::Fill),]
                    .spacing(6)
                    .align_y(Alignment::Center),
                Space::new().height(4),
                Element::from(load_info),
                Space::new().height(6),
                section_title(t.per_core_usage, p, &self.typo),
                cores_grid,
                Space::new().height(6),
                section_title(t.system_info, p, &self.typo),
                info,
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── Memory Detail ──
    pub(crate) fn view_detail_memory<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let cc = self.chart_colors();
        let mem_data: Vec<f32> = self.live_buffer.iter().map(|p| p.mem_pct).collect();
        let display_mem = self.anim_mem_pct;
        // Pulse effect for memory threshold
        let is_critical = display_mem >= self.mem_alert_threshold;
        let pulse_alpha = if is_critical {
            self.pulse_opacity()
        } else {
            1.0
        };
        let chart_color = if is_critical {
            Color::from_rgba(p.red.r, p.red.g, p.red.b, pulse_alpha)
        } else {
            p.green
        };
        let mem_chart = make_chart(ChartCfg {
            title: format!("Memory {ICON_DASH} {:.1}%", display_mem),
            series: vec![("RAM".into(), chart_color, mem_data)],
            y_min: 0.0,
            y_max: 100.0,
            filled: true,
            height: 200.0,
            unit: "%".into(),
            colors: cc,
        });

        let swap_pct = if snap.swap_total > 0 {
            snap.swap_used as f32 / snap.swap_total as f32 * 100.0
        } else {
            0.0
        };

        let available = snap.memory_total.saturating_sub(snap.memory_used);

        let info = column![
            info_row(
                t.in_use,
                format!(
                    "{} / {}",
                    format_bytes(snap.memory_used),
                    format_bytes(snap.memory_total)
                ),
                p,
                &self.typo
            ),
            info_row(t.available, format_bytes(available), p, &self.typo),
            info_row(t.usage, format!("{:.1}%", display_mem), p, &self.typo),
        ]
        .spacing(4);

        let bars = column![
            labeled_bar(
                "RAM",
                snap.memory_used,
                snap.memory_total,
                p.green,
                p,
                &self.typo
            ),
            labeled_bar(
                "Swap",
                snap.swap_used,
                snap.swap_total,
                p.yellow,
                p,
                &self.typo
            ),
        ]
        .spacing(6);

        // Process virtual memory total
        let total_virt: u64 = if let Some(snap) = &self.current {
            snap.processes.iter().map(|p| p.virtual_memory_bytes).sum()
        } else {
            0
        };

        let swap_info = column![
            info_row(
                t.swap_used,
                format!(
                    "{} / {}",
                    format_bytes(snap.swap_used),
                    format_bytes(snap.swap_total)
                ),
                p,
                &self.typo
            ),
            info_row(t.swap_usage, format!("{:.1}%", swap_pct), p, &self.typo),
            info_row(
                t.virtual_memory_total,
                format_bytes(total_virt),
                p,
                &self.typo
            ),
        ]
        .spacing(4);

        let gc = GaugeColors {
            bg: p.panel_bg,
            label: p.label,
            text: p.text,
            bar_bg: p.bar_bg,
        };
        let mem_gauge: Element<Message> = Canvas::new(RadialGauge {
            value: self.anim_mem_pct,
            label: "RAM".into(),
            color: chart_color,
            colors: gc,
        })
        .width(Length::Fixed(120.0))
        .height(Length::Fixed(100.0))
        .into();

        panel(
            column![
                row![mem_gauge, column![mem_chart].width(Length::Fill),]
                    .spacing(6)
                    .align_y(Alignment::Center),
                Space::new().height(8),
                bars,
                Space::new().height(8),
                section_title("RAM", p, &self.typo),
                info,
                Space::new().height(8),
                section_title(t.swap, p, &self.typo),
                swap_info,
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── Network Detail ──
    pub(crate) fn view_detail_network<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let cc = self.chart_colors();
        let rx_kb: Vec<f32> = self
            .live_buffer
            .iter()
            .map(|p| p.net_rx as f32 / 1024.0)
            .collect();
        let tx_kb: Vec<f32> = self
            .live_buffer
            .iter()
            .map(|p| p.net_tx as f32 / 1024.0)
            .collect();
        let max_kb = rx_kb
            .iter()
            .chain(tx_kb.iter())
            .cloned()
            .fold(0.001f32, f32::max);
        let (rx_data, tx_data, unit, y_max) = if max_kb >= 1024.0 {
            let rx_mb: Vec<f32> = rx_kb.iter().map(|v| v / 1024.0).collect();
            let tx_mb: Vec<f32> = tx_kb.iter().map(|v| v / 1024.0).collect();
            let max_mb = max_kb / 1024.0;
            (rx_mb, tx_mb, " MB/s", max_mb)
        } else {
            (rx_kb, tx_kb, " KB/s", max_kb)
        };
        let net_chart = make_chart(ChartCfg {
            title: t.network.into(),
            series: vec![
                (format!("{ICON_ARROW_DOWN} rx"), p.green, rx_data),
                (format!("{ICON_ARROW_UP} tx"), p.red, tx_data),
            ],
            y_min: 0.0,
            y_max,
            filled: true,
            height: 200.0,
            unit: unit.into(),
            colors: cc,
        });

        let totals = column![
            info_row(
                format!("{ICON_ARROW_DOWN} {}", t.receive),
                format!("{}/s", format_bytes(snap.net_rx_bytes)),
                p,
                &self.typo
            ),
            info_row(
                format!("{ICON_ARROW_UP} {}", t.send),
                format!("{}/s", format_bytes(snap.net_tx_bytes)),
                p,
                &self.typo
            ),
        ]
        .spacing(4);

        let text_c = p.text;
        let green = p.green;
        let red = p.red;
        let mut iface_items: Vec<Element<Message>> = Vec::new();
        for iface in &snap.net_interfaces {
            let item = row![
                text(&iface.name)
                    .size(self.typo.sz(11))
                    .color(text_c)
                    .width(140),
                text(format!(
                    "{ICON_ARROW_DOWN} {}",
                    format_bytes(iface.rx_bytes)
                ))
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(green)
                .width(110),
                text(format!("{ICON_ARROW_UP} {}", format_bytes(iface.tx_bytes)))
                    .size(self.typo.sz(11))
                    .font(self.typo.regular)
                    .color(red)
                    .width(110),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            iface_items.push(item.into());
        }

        panel(
            column![
                net_chart,
                Space::new().height(8),
                section_title(t.throughput, p, &self.typo),
                totals,
                Space::new().height(8),
                section_title(t.interfaces, p, &self.typo),
                Column::with_children(iface_items).spacing(3),
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── Disk Detail ──
    pub(crate) fn view_detail_disk<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let text_c = p.text;
        let label_c = p.label;
        let green = p.green;
        let border_c = p.border;
        let panel_bg = p.panel_bg;
        let sidebar_bg = p.sidebar_bg;

        let total_space: u64 = snap.disks.iter().map(|d| d.total).sum();
        let total_avail: u64 = snap.disks.iter().map(|d| d.available).sum();
        let total_used = total_space.saturating_sub(total_avail);
        let total_pct = if total_space > 0 {
            total_used as f64 / total_space as f64 * 100.0
        } else {
            0.0
        };

        let summary = container(
            row![
                column![
                    text(format!("{} {}", snap.disks.len(), t.drives))
                        .size(self.typo.sz(20))
                        .font(self.typo.regular)
                        .color(text_c),
                    text(format!("{:.1}% {}", total_pct, t.overall_usage))
                        .size(self.typo.sz(11))
                        .font(self.typo.regular)
                        .color(label_c),
                ]
                .spacing(4)
                .width(Length::FillPortion(1)),
                column![
                    info_row(t.total_capacity, format_bytes(total_space), p, &self.typo),
                    info_row(t.total_used, format_bytes(total_used), p, &self.typo),
                    info_row(t.total_free, format_bytes(total_avail), p, &self.typo),
                ]
                .spacing(4)
                .width(Length::FillPortion(1)),
            ]
            .spacing(20),
        )
        .padding(12)
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(sidebar_bg)),
            border: Border {
                color: border_c,
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 4.0,
            },
            ..Default::default()
        });

        let mut disk_items: Vec<Element<Message>> = Vec::new();
        for d in &snap.disks {
            let used = d.total.saturating_sub(d.available);
            let pct = if d.total > 0 {
                used as f32 / d.total as f32 * 100.0
            } else {
                0.0
            };
            let color = gradient_color(pct / 100.0, p);
            let bar_bg = p.bar_bg;

            let icon = if d.is_removable { ICON_USB } else { ICON_DISK };
            let disk_type = if d.name.contains("nvme") {
                "NVMe SSD"
            } else if d.name.contains("sd") {
                "SATA"
            } else {
                "Drive"
            };

            let disk_card = container(
                column![
                    row![
                        text(format!("{icon} {}", d.mount))
                            .size(self.typo.sz(14))
                            .color(text_c),
                        Space::new().width(Length::Fill),
                        text(format!("{} {ICON_BULLET} {}", d.name, disk_type))
                            .size(self.typo.sz(10))
                            .color(label_c),
                    ],
                    Space::new().height(6),
                    themed_bar(pct, color, bar_bg),
                    Space::new().height(6),
                    row![
                        text(format!("{:.1}%", pct))
                            .size(self.typo.sz(14))
                            .font(self.typo.regular)
                            .color(color),
                        Space::new().width(Length::Fill),
                        text(format!("{} {}", format_bytes(used), t.used))
                            .size(self.typo.sz(11))
                            .font(self.typo.regular)
                            .color(text_c),
                        Space::new().width(12),
                        text(format!("{} {}", format_bytes(d.available), t.free))
                            .size(self.typo.sz(11))
                            .font(self.typo.regular)
                            .color(green),
                        Space::new().width(12),
                        text(format!("{} {}", format_bytes(d.total), t.total))
                            .size(self.typo.sz(11))
                            .font(self.typo.regular)
                            .color(label_c),
                    ],
                    Space::new().height(8),
                    row![
                        column![
                            info_row(t.file_system, &d.fs_type, p, &self.typo),
                            info_row(t.mount_point, &d.mount, p, &self.typo),
                        ]
                        .spacing(3)
                        .width(Length::FillPortion(1)),
                        column![
                            info_row(t.device, &d.name, p, &self.typo),
                            info_row(
                                t.type_label,
                                if d.is_removable { t.removable } else { t.fixed },
                                p,
                                &self.typo
                            ),
                        ]
                        .spacing(3)
                        .width(Length::FillPortion(1)),
                    ]
                    .spacing(20),
                ]
                .spacing(0),
            )
            .padding(12)
            .width(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(panel_bg)),
                border: Border {
                    color: border_c,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 6.0,
                },
                ..Default::default()
            });
            disk_items.push(disk_card.into());
        }

        // Disk I/O
        let disk_io_info = column![
            info_row(
                format!("{ICON_ARROW_DOWN} {}", t.read),
                format!("{}/s", format_bytes(snap.disk_io.read_bytes)),
                p,
                &self.typo
            ),
            info_row(
                format!("{ICON_ARROW_UP} {}", t.write),
                format!("{}/s", format_bytes(snap.disk_io.write_bytes)),
                p,
                &self.typo
            ),
        ]
        .spacing(4);

        let disk_title = format!("{ICON_DISK} {}", t.disk_drives);
        panel(
            column![
                section_title(&disk_title, p, &self.typo),
                summary,
                Space::new().height(8),
                section_title(t.io_throughput, p, &self.typo),
                disk_io_info,
                Space::new().height(8),
                Column::with_children(disk_items).spacing(8),
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── Temperature Detail ──
    pub(crate) fn view_detail_temp<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let text_c = p.text;
        let label_c = p.label;
        let green = p.green;
        let red = p.red;
        let yellow = p.yellow;
        let panel_bg = p.panel_bg;
        let bg = p.bg;

        let temp_title = format!("{ICON_TEMP} {}", t.temperatures);
        if snap.temperatures.is_empty() {
            return panel(
                column![
                    section_title(&temp_title, p, &self.typo),
                    text(t.no_sensors)
                        .size(self.typo.sz(12))
                        .font(self.typo.regular)
                        .color(label_c),
                ]
                .spacing(6)
                .into(),
                p,
            );
        }

        let mut temp_items: Vec<Element<Message>> = Vec::new();
        for (i, t) in snap.temperatures.iter().enumerate() {
            let color = if t.temp_c > 80.0 {
                red
            } else if t.temp_c > 60.0 {
                yellow
            } else {
                green
            };
            let row_bg = if i % 2 == 0 { panel_bg } else { bg };
            let temp_str = format_temp(t.temp_c, self.temp_celsius);
            let item = container(
                row![
                    text(&t.label)
                        .size(self.typo.sz(11))
                        .color(text_c)
                        .width(Length::Fill),
                    text(temp_str)
                        .size(self.typo.sz(11))
                        .font(self.typo.regular)
                        .color(color),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([4, 8])
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(row_bg)),
                ..Default::default()
            });
            temp_items.push(item.into());
        }

        let valid_temps: Vec<f32> = snap
            .temperatures
            .iter()
            .map(|t| t.temp_c)
            .filter(|&t| t > -30.0)
            .collect();
        let (min_t, max_t, avg_t) = if valid_temps.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let min = valid_temps.iter().cloned().fold(f32::MAX, f32::min);
            let max = valid_temps.iter().cloned().fold(f32::MIN, f32::max);
            let avg = valid_temps.iter().sum::<f32>() / valid_temps.len() as f32;
            (min, max, avg)
        };

        let summary = column![
            info_row(
                t.sensors,
                snap.temperatures.len().to_string(),
                p,
                &self.typo
            ),
            info_row(
                t.minimum,
                format_temp(min_t, self.temp_celsius),
                p,
                &self.typo
            ),
            info_row(
                t.maximum,
                format_temp(max_t, self.temp_celsius),
                p,
                &self.typo
            ),
            info_row(
                t.average,
                format_temp(avg_t, self.temp_celsius),
                p,
                &self.typo
            ),
        ]
        .spacing(4);

        let temp_overview_title = format!("{ICON_TEMP} {}", t.temperature_overview);
        panel(
            column![
                section_title(&temp_overview_title, p, &self.typo),
                summary,
                Space::new().height(8),
                section_title(t.all_sensors, p, &self.typo),
                Column::with_children(temp_items).spacing(0),
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── GPU Detail ──
    pub(crate) fn view_detail_gpu<'a>(&'a self, snap: &'a Snapshot) -> Element<'a, Message> {
        let p = &self.pal;
        let t = self.t();
        let text_c = p.text;
        let label_c = p.label;

        if snap.gpu.gpus.is_empty() {
            return panel(
                column![
                    section_title(format!("{ICON_GPU} {}", t.gpu), p, &self.typo),
                    text(t.no_gpu)
                        .size(self.typo.sz(12))
                        .font(self.typo.regular)
                        .color(label_c),
                ]
                .spacing(6)
                .into(),
                p,
            );
        }

        let mut gpu_items: Vec<Element<Message>> = Vec::new();
        for gpu in &snap.gpu.gpus {
            let mem_pct = if gpu.memory_total > 0 {
                gpu.memory_used as f32 / gpu.memory_total as f32 * 100.0
            } else {
                0.0
            };
            let util_color = gradient_color(gpu.utilization as f32 / 100.0, p);
            let _temp_color = if gpu.temperature > 80.0 {
                p.red
            } else if gpu.temperature > 60.0 {
                p.yellow
            } else {
                p.green
            };

            gpu_items.push(
                column![
                    text(&gpu.name).size(self.typo.sz(14)).color(text_c),
                    Space::new().height(4),
                    info_row(
                        t.utilization,
                        format!("{}%", gpu.utilization),
                        p,
                        &self.typo
                    ),
                    info_row(
                        t.temperature,
                        format!("{:.0}°C", gpu.temperature),
                        p,
                        &self.typo
                    ),
                    info_row(
                        t.vram,
                        format!(
                            "{} / {}",
                            format_bytes(gpu.memory_used),
                            format_bytes(gpu.memory_total)
                        ),
                        p,
                        &self.typo
                    ),
                    info_row(t.vram_usage, format!("{:.1}%", mem_pct), p, &self.typo),
                    info_row(t.power, format!("{:.1}W", gpu.power_watts), p, &self.typo),
                    Space::new().height(4),
                    labeled_bar(
                        "Util",
                        gpu.utilization as u64,
                        100,
                        util_color,
                        p,
                        &self.typo
                    ),
                    labeled_bar(
                        "VRAM",
                        gpu.memory_used,
                        gpu.memory_total,
                        p.magenta,
                        p,
                        &self.typo
                    ),
                ]
                .spacing(4)
                .into(),
            );
        }

        panel(
            column![
                section_title(format!("{ICON_GPU} {}", t.gpu), p, &self.typo),
                Column::with_children(gpu_items).spacing(12),
            ]
            .spacing(4)
            .into(),
            p,
        )
    }

    // ─── PROCESSES TAB ──────────────────────────────────────────
}
