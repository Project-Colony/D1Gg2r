//! The processes tab.

use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::icons::*;
use crate::message::*;
use crate::state::*;
use crate::ui::widgets::*;

impl Digger {
    pub(crate) fn view_processes(&self) -> Element<'_, Message> {
        let p = &self.pal;
        let t = self.t();
        let label_c = p.label;
        let accent = p.accent;
        let green = p.green;
        let yellow = p.yellow;
        let red = p.red;
        let panel_bg = p.panel_bg;
        let bg = p.bg;
        let border_c = p.border;
        let sidebar_bg = p.sidebar_bg;

        let Some(snap) = &self.current else {
            return container(
                text(format!("{ICON_LOADING} {}", t.collecting_data))
                    .size(self.typo.sz(14))
                    .font(self.typo.regular)
                    .color(label_c),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        };

        let group_label = if self.process_grouped {
            t.grouped
        } else {
            t.all
        };
        let group_color = if self.process_grouped {
            accent
        } else {
            label_c
        };

        let filter_row = row![
            text(format!("{ICON_SEARCH} {}", t.filter))
                .size(self.typo.sz(11))
                .font(self.typo.regular)
                .color(label_c),
            Space::new().width(4),
            text_input(t.search, &self.process_filter)
                .on_input(Message::ProcessFilterChanged)
                .width(220),
            Space::new().width(12),
            button(
                text(format!("{ICON_BARS} {group_label}"))
                    .size(self.typo.sz(11))
                    .font(self.typo.regular)
                    .color(group_color)
            )
            .on_press(Message::ToggleGrouped)
            .style(button::secondary)
            .padding([3, 10]),
            Space::new().width(Length::Fill),
            text(format!(
                "{ICON_LIST} {} {}",
                snap.processes.len(),
                t.processes
            ))
            .size(self.typo.sz(11))
            .font(self.typo.regular)
            .color(label_c),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .padding([6, 10]);

        let filter_lower = self.process_filter.to_lowercase();
        let filtered: Vec<_> = snap
            .processes
            .iter()
            .filter(|p| {
                filter_lower.is_empty()
                    || p.name.to_lowercase().contains(&filter_lower)
                    || p.cmd
                        .iter()
                        .any(|c| c.to_lowercase().contains(&filter_lower))
            })
            .collect();

        let si = |col: ProcessSort| -> &str {
            if self.process_sort == col {
                if self.process_sort_asc {
                    ICON_SORT_UP
                } else {
                    ICON_SORT_DOWN
                }
            } else {
                ""
            }
        };

        let header = container(
            row![
                sort_btn(
                    format!("PID {}", si(ProcessSort::Pid)),
                    ProcessSort::Pid,
                    60,
                    accent,
                    &self.typo
                ),
                text("PPID").size(self.typo.sz(11)).color(accent).width(50),
                sort_btn(
                    format!("{} {}", t.command, si(ProcessSort::Name)),
                    ProcessSort::Name,
                    180,
                    accent,
                    &self.typo
                ),
                sort_btn(
                    format!("CPU% {}", si(ProcessSort::Cpu)),
                    ProcessSort::Cpu,
                    70,
                    accent,
                    &self.typo
                ),
                sort_btn(
                    format!("{} {}", t.memory, si(ProcessSort::Memory)),
                    ProcessSort::Memory,
                    90,
                    accent,
                    &self.typo
                ),
                text("St").size(self.typo.sz(11)).color(accent).width(25),
                text(format!("{ICON_THREAD} Thr"))
                    .size(self.typo.sz(11))
                    .color(accent)
                    .width(40),
                text(t.action)
                    .size(self.typo.sz(11))
                    .font(self.typo.regular)
                    .color(accent)
                    .width(60),
            ]
            .spacing(2),
        )
        .padding([4, 10])
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(sidebar_bg)),
            border: Border {
                color: border_c,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        let mut rows: Vec<Element<Message>> = Vec::new();

        if self.process_grouped {
            // SAFETY: libc::getuid() is a simple POSIX syscall that returns the real
            // user ID of the calling process. It is always safe to call, has no side
            // effects, cannot fail, and requires no special resources or permissions.
            // It is used here to separate user-owned processes from system processes.
            #[cfg(unix)]
            let current_uid = unsafe { libc::getuid() };
            // On Windows, metrics.rs sets uid=0 for user processes and uid=1
            // for system processes (SYSTEM/LOCAL SERVICE/NETWORK SERVICE).
            // current_uid=0 makes the grouping logic work correctly:
            // uid != 0 → System, is_desktop_app → Apps, else → Background.
            #[cfg(not(unix))]
            let current_uid = 0u32;

            let mut apps: Vec<_> = Vec::new();
            let mut background: Vec<_> = Vec::new();
            let mut system: Vec<_> = Vec::new();

            for proc in &filtered {
                if proc.uid != current_uid {
                    system.push(*proc);
                } else if proc.is_desktop_app {
                    apps.push(*proc);
                } else {
                    background.push(*proc);
                }
            }

            let sort_fn = |list: &mut Vec<&crate::metrics::ProcessInfo>| {
                match self.process_sort {
                    ProcessSort::Pid => list.sort_by_key(|p| p.pid),
                    ProcessSort::Name => list.sort_by_key(|p| p.name.to_lowercase()),
                    ProcessSort::Cpu => list.sort_by(|a, b| {
                        a.cpu_usage
                            .partial_cmp(&b.cpu_usage)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }),
                    ProcessSort::Memory => list.sort_by_key(|p| p.memory_bytes),
                }
                if !self.process_sort_asc {
                    list.reverse();
                }
            };
            sort_fn(&mut apps);
            sort_fn(&mut background);
            sort_fn(&mut system);

            let mut row_idx = 0usize;
            let sections: Vec<(&str, &str, Color, &Vec<&crate::metrics::ProcessInfo>)> = vec![
                (ICON_APPS, t.applications, green, &apps),
                (ICON_BACKGROUND, t.background_processes, yellow, &background),
                (ICON_SYSTEM, t.system, red, &system),
            ];

            for (icon, label, color, list) in sections {
                if list.is_empty() {
                    continue;
                }
                let hdr_bg = sidebar_bg;
                let section_hdr = container(
                    text(format!("{icon} {label} ({})", list.len()))
                        .size(self.typo.sz(11))
                        .font(self.typo.regular)
                        .color(color),
                )
                .padding([4, 10])
                .width(Length::Fill)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(hdr_bg)),
                    ..Default::default()
                });
                rows.push(section_hdr.into());

                for proc in list.iter() {
                    let row_bg = if row_idx.is_multiple_of(2) {
                        panel_bg
                    } else {
                        bg
                    };
                    rows.push(process_row(
                        proc,
                        row_bg,
                        p,
                        self.cpu_alert_threshold,
                        &self.typo,
                    ));
                    row_idx += 1;
                }
            }
        } else {
            let mut procs = filtered;
            match self.process_sort {
                ProcessSort::Pid => procs.sort_by_key(|p| p.pid),
                ProcessSort::Name => procs.sort_by_key(|p| p.name.to_lowercase()),
                ProcessSort::Cpu => procs.sort_by(|a, b| {
                    a.cpu_usage
                        .partial_cmp(&b.cpu_usage)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                ProcessSort::Memory => procs.sort_by_key(|p| p.memory_bytes),
            }
            if !self.process_sort_asc {
                procs.reverse();
            }
            for (i, proc) in procs.iter().take(self.process_limit).enumerate() {
                let row_bg = if i % 2 == 0 { panel_bg } else { bg };
                rows.push(process_row(
                    proc,
                    row_bg,
                    p,
                    self.cpu_alert_threshold,
                    &self.typo,
                ));
            }
        }

        let table = Column::with_children(rows).spacing(0);
        let content = panel(column![filter_row, header, table].spacing(0).into(), p);

        scrollable(column![content].padding(4)).into()
    }

    // ─── HISTORY TAB ────────────────────────────────────────────
}
