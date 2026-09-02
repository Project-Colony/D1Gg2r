//! Presentation helpers with no state of their own.
//!
//! Anything here takes what it needs as arguments, which is what makes it
//! reusable across the tabs rather than a method that happens to be free.

use iced::widget::canvas::Canvas;
use iced::widget::{button, column, container, progress_bar, row, text, tooltip, Row};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme};

use crate::chart::{ChartColors, LineChart};
use crate::icons::*;
use crate::message::*;
use crate::theme::Palette;
use crate::ui::layout::*;

// ─── HELPER FUNCTIONS ────────────────────────────────────────────

pub(crate) fn gradient_color(t: f32, p: &Palette) -> Color {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        let f = t * 2.0;
        Color::from_rgb(
            p.green.r + (p.yellow.r - p.green.r) * f,
            p.green.g + (p.yellow.g - p.green.g) * f,
            p.green.b + (p.yellow.b - p.green.b) * f,
        )
    } else {
        let f = (t - 0.5) * 2.0;
        Color::from_rgb(
            p.yellow.r + (p.red.r - p.yellow.r) * f,
            p.yellow.g + (p.red.g - p.yellow.g) * f,
            p.yellow.b + (p.red.b - p.yellow.b) * f,
        )
    }
}

pub(crate) fn format_temp(temp_c: f32, celsius: bool) -> String {
    if temp_c < -30.0 {
        "N/A".to_string()
    } else if celsius {
        format!("{:.0}\u{00b0}C", temp_c)
    } else {
        format!("{:.0}\u{00b0}F", temp_c * 9.0 / 5.0 + 32.0)
    }
}

pub(crate) fn themed_bar(value: f32, color: Color, bar_bg: Color) -> Element<'static, Message> {
    // The colour arrives already adjusted for legibility against this surface;
    // the old `* 0.9 + 0.1` lightening pushed it back toward white and undid
    // that on exactly the light themes where it mattered.
    let bar_color = color;
    progress_bar(0.0..=100.0, value)
        .length(Length::Fill)
        .style(move |_: &Theme| progress_bar::Style {
            background: Background::Color(bar_bg),
            bar: Background::Color(bar_color),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: RADIUS_CARD.into(),
            },
        })
        .into()
}

pub(crate) struct ChartCfg {
    pub(crate) title: String,
    pub(crate) series: Vec<(String, Color, Vec<f32>)>,
    pub(crate) y_min: f32,
    pub(crate) y_max: f32,
    pub(crate) filled: bool,
    pub(crate) height: f32,
    pub(crate) unit: String,
    pub(crate) colors: ChartColors,
}

pub(crate) fn make_chart(cfg: ChartCfg) -> Element<'static, Message> {
    let chart = LineChart {
        series: cfg.series,
        y_min: cfg.y_min,
        y_max: cfg.y_max,
        title: cfg.title,
        filled: cfg.filled,
        unit: cfg.unit,
        colors: cfg.colors,
        show_avg: true,
    };
    Canvas::new(chart)
        .width(Length::Fill)
        .height(Length::Fixed(cfg.height))
        .into()
}

pub(crate) fn sidebar_item<'a>(
    label: impl ToString,
    value: impl ToString,
    color: Color,
    target: OverviewPanel,
    current: OverviewPanel,
    p: &Palette,
    typo: &colony_ui::Typography,
) -> Element<'a, Message> {
    let is_active = target == current;
    let sidebar_bg = p.sidebar_bg;
    // From the palette, not from arithmetic on the background. Adding 0.06 to
    // each channel clips to white on the pale themes, which is how a selected
    // item came to be indistinguishable from a card on Modus Operandi and
    // One Dark Light.
    let active_bg = p.selected;
    let hover_bg = p.hover;
    let bg = if is_active { active_bg } else { sidebar_bg };
    let border_color = if is_active { color } else { Color::TRANSPARENT };
    let label_c = p.label;
    let text_c = p.text;
    let label = label.to_string();
    let value = value.to_string();

    let content = column![
        text(label)
            .size(typo.sz(12))
            .color(if is_active { color } else { label_c }),
        text(value)
            .size(typo.sz(13))
            .font(typo.regular)
            .color(if is_active { text_c } else { label_c }),
    ]
    .spacing(SPACE_2XS);

    button(content)
        .on_press(Message::OverviewSection(target))
        .width(Length::Fill)
        .padding(PAD_CARD)
        .style(move |_: &Theme, status| {
            let bg_final = match status {
                button::Status::Hovered => {
                    if is_active {
                        active_bg
                    } else {
                        hover_bg
                    }
                }
                button::Status::Pressed => active_bg,
                _ => bg,
            };
            button::Style {
                background: Some(Background::Color(bg_final)),
                text_color: text_c,
                border: Border {
                    color: border_color,
                    width: if is_active { 1.0 } else { 0.0 },
                    radius: RADIUS_CARD.into(),
                },

                ..Default::default()
            }
        })
        .into()
}

pub(crate) fn settings_sidebar_item(
    label: impl ToString,
    target: SettingsPanel,
    current: SettingsPanel,
    p: &Palette,
    typo: &colony_ui::Typography,
) -> Element<'static, Message> {
    let is_active = target == current;
    let accent = p.accent;
    let bg = if is_active {
        accent
    } else {
        Color::TRANSPARENT
    };
    let hover_bg = p.hover;
    // Colony fills the selected category with the accent and writes
    // text_primary on it. That pairing is below 4.5:1 on fifty-eight of the
    // fifty-nine themes and as low as 1.01:1 on Ayu Dark, so the fill stays and
    // the foreground comes from the helper colony-ui provides for exactly this.
    let on_accent = colony_ui::contrast_on(accent);
    let text_color = if is_active { on_accent } else { p.label };

    button(
        text(label.to_string())
            .size(typo.sz(12))
            .font(typo.regular)
            .color(text_color),
    )
    .on_press(Message::SettingsPanelSelected(target))
    .width(Length::Fill)
    .padding(PAD_CARD)
    .style(move |_: &Theme, status| {
        let bg_final = match status {
            _ if is_active => accent,
            button::Status::Hovered => hover_bg,
            _ => bg,
        };
        button::Style {
            background: Some(Background::Color(bg_final)),
            text_color,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: RADIUS_CONTROL.into(),
            },

            ..Default::default()
        }
    })
    .into()
}

pub(crate) fn info_row<'a>(
    label: impl ToString,
    value: impl ToString,
    p: &Palette,
    typo: &colony_ui::Typography,
) -> Element<'a, Message> {
    let l = format!("{}:", label.to_string());
    let v = value.to_string();
    let label_c = p.label;
    let text_c = p.text;
    row![
        text(l).size(typo.sz(11)).color(label_c).width(120),
        text(v).size(typo.sz(11)).font(typo.regular).color(text_c),
    ]
    .spacing(SPACE_MD)
    .into()
}

pub(crate) fn process_row<'a>(
    proc: &crate::metrics::ProcessInfo,
    bg: Color,
    p: &'a Palette,
    cpu_threshold: f32,
    typo: &colony_ui::Typography,
) -> Element<'a, Message> {
    let cpu_color = gradient_color(proc.cpu_usage / 100.0, p);
    let pid = proc.pid;
    let pid_str = pid.to_string();
    let name = proc.name.clone();
    let cpu = format!("{:.1}%", proc.cpu_usage);
    let mem = format_bytes(proc.memory_bytes);
    let label_c = p.label;
    let text_c = p.text;
    let accent = p.accent;

    // Command-line tooltip (truncated) — avoid allocation if no args
    let cmd_str: String = if proc.cmd.len() > 1 {
        let mut args = String::new();
        for (i, arg) in proc.cmd[1..].iter().enumerate() {
            if i > 0 {
                args.push(' ');
            }
            if args.len() + arg.len() > 60 {
                args.push_str(&arg[..60_usize.saturating_sub(args.len()).min(arg.len())]);
                args.push('\u{2026}');
                break;
            }
            args.push_str(arg);
        }
        args
    } else {
        String::new()
    };

    // Parent PID display
    let ppid_str = proc.parent_pid.map(|p| p.to_string()).unwrap_or_default();

    // Highlight row if CPU exceeds threshold
    let row_bg = if proc.cpu_usage >= cpu_threshold {
        Color::from_rgba(p.red.r, p.red.g, p.red.b, 0.1)
    } else {
        bg
    };

    let kill_btn = button(text(ICON_KILL).size(typo.sz(10)).color(label_c))
        .on_press(Message::KillProcess(pid))
        .style(button::text)
        .padding([1, 4]);

    let name_col: Element<Message> = if cmd_str.is_empty() {
        text(name.clone())
            .size(typo.sz(11))
            .color(text_c)
            .width(180)
            .into()
    } else {
        tooltip(
            text(name.clone())
                .size(typo.sz(11))
                .color(text_c)
                .width(180),
            text(cmd_str).size(typo.sz(9)).color(text_c),
            tooltip::Position::Top,
        )
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(p.panel_bg)),
            border: Border {
                color: accent,
                width: 1.0,
                radius: RADIUS_CARD.into(),
            },
            text_color: Some(text_c),
            shadow: Shadow::default(),

            ..Default::default()
        })
        .padding(6)
        .into()
    };

    container(
        row![
            text(pid_str)
                .size(typo.sz(11))
                .font(typo.regular)
                .color(label_c)
                .width(60),
            text(ppid_str)
                .size(typo.sz(10))
                .font(typo.regular)
                .color(label_c)
                .width(50),
            name_col,
            text(cpu)
                .size(typo.sz(11))
                .font(typo.regular)
                .color(cpu_color)
                .width(70),
            text(mem)
                .size(typo.sz(11))
                .font(typo.regular)
                .color(accent)
                .width(90),
            text(String::from(proc.status))
                .size(typo.sz(11))
                .font(typo.regular)
                .color(match proc.status {
                    'R' => p.green,
                    'Z' => p.red,
                    'D' => p.yellow,
                    _ => label_c,
                })
                .width(25),
            text(proc.thread_count.to_string())
                .size(typo.sz(11))
                .font(typo.regular)
                .color(label_c)
                .width(40),
            kill_btn,
        ]
        .spacing(SPACE_2XS)
        .align_y(Alignment::Center),
    )
    .padding([2, 10])
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(row_bg)),
        ..Default::default()
    })
    .into()
}

pub(crate) fn panel<'a>(content: Element<'a, Message>, p: &Palette) -> Element<'a, Message> {
    let panel_bg = p.panel_bg;
    let border_c = p.border;
    container(content)
        .width(Length::Fill)
        .padding(10)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(panel_bg)),
            border: Border {
                color: border_c,
                width: 1.0,
                radius: RADIUS_CONTROL.into(),
            },
            ..Default::default()
        })
        .into()
}

pub(crate) fn panel_bg<'a>(
    content: Element<'a, Message>,
    bg: Color,
    border_c: Color,
) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: border_c,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

pub(crate) fn menu_tab(
    label: impl ToString,
    tab: Tab,
    current: Tab,
    p: &Palette,
    typo: &colony_ui::Typography,
) -> Element<'static, Message> {
    let is_active = tab == current;
    let accent = p.accent;
    let label_c = p.label;
    let text_c = p.text;
    let color = if is_active { accent } else { label_c };
    let hover_color = Color::from_rgba(accent.r, accent.g, accent.b, 0.15);
    button(
        text(label.to_string())
            .size(typo.sz(12))
            .font(typo.regular)
            .color(color),
    )
    .on_press(Message::TabSelected(tab))
    .padding(PAD_CARD)
    .style(move |_: &Theme, status| {
        let bg = match status {
            button::Status::Hovered => hover_color,
            button::Status::Pressed => Color::from_rgba(accent.r, accent.g, accent.b, 0.25),
            _ => {
                if is_active {
                    Color::from_rgba(accent.r, accent.g, accent.b, 0.1)
                } else {
                    Color::TRANSPARENT
                }
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text_c,
            border: Border {
                color: if is_active {
                    accent
                } else {
                    Color::TRANSPARENT
                },
                width: 0.0,
                radius: RADIUS_CARD.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

pub(crate) fn section_title(
    label: impl ToString,
    p: &Palette,
    typo: &colony_ui::Typography,
) -> Element<'static, Message> {
    let accent = p.accent;
    text(label.to_string())
        .size(typo.sz(11))
        .font(typo.regular)
        .color(accent)
        .into()
}

/// How the two numbers under a bar should be written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BarUnit {
    /// `2.1 GiB/15.9 GiB`.
    Bytes,
    /// `2%` — for a value that is already a proportion, where the byte
    /// formatter used to print the memorable nonsense "2 B/100 B".
    Percent,
}

pub(crate) fn labeled_bar(
    label: &str,
    used: u64,
    total: u64,
    unit: BarUnit,
    color: Color,
    p: &Palette,
    typo: &colony_ui::Typography,
) -> Element<'static, Message> {
    if total == 0 {
        return row![].into();
    }
    let pct = used as f32 / total as f32 * 100.0;
    let label_c = p.label;
    let bar_bg = p.bar_bg;
    row![
        text(format!("{label}:"))
            .size(typo.sz(11))
            .color(label_c)
            .width(60),
        themed_bar(pct, color, bar_bg),
        text(match unit {
            BarUnit::Bytes => format!("{}/{}", format_bytes(used), format_bytes(total)),
            BarUnit::Percent => format!("{pct:.0}%"),
        })
        .size(typo.sz(11))
        .font(typo.regular)
        .color(color)
        .width(150),
    ]
    .spacing(SPACE_SM)
    .align_y(Alignment::Center)
    .into()
}

pub(crate) fn sort_btn(
    label: String,
    col: ProcessSort,
    width: u16,
    accent: Color,
    typo: &colony_ui::Typography,
) -> Element<'static, Message> {
    button(text(label).size(typo.sz(11)).color(accent))
        .on_press(Message::SortBy(col))
        .style(button::text)
        .padding(PAD_TIGHT)
        .width(Length::Fixed(f32::from(width)))
        .into()
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TiB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

pub(crate) fn format_duration(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

pub(crate) fn downsample(data: &[f32], max_points: usize) -> Vec<f32> {
    let n = data.len();
    if n <= max_points {
        return data.to_vec();
    }
    let bucket_size = n as f64 / max_points as f64;
    let mut out = Vec::with_capacity(max_points);
    for i in 0..max_points {
        let start = (i as f64 * bucket_size) as usize;
        let end = (((i + 1) as f64 * bucket_size) as usize).min(n);
        let peak = data[start..end]
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        out.push(peak);
    }
    out
}

pub(crate) fn make_threshold_buttons<'a>(
    current: f32,
    options: &[f32],
    on_press: impl Fn(f32) -> Message + 'a,
    accent: Color,
    label_c: Color,
    typo: &colony_ui::Typography,
) -> Element<'a, Message> {
    let mut btns: Vec<Element<Message>> = Vec::new();
    for &val in options {
        let is_active = (current - val).abs() < 0.5;
        let color = if is_active { accent } else { label_c };
        let btn = button(
            text(format!("{:.0}%", val))
                .size(typo.sz(11))
                .font(typo.regular)
                .color(color),
        )
        .on_press(on_press(val))
        .style(if is_active {
            button::primary
        } else {
            button::secondary
        })
        .padding(PAD_ROW);
        btns.push(btn.into());
    }
    Row::with_children(btns).spacing(SPACE_XS).into()
}
