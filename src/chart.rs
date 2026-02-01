use iced::mouse;
use iced::widget::canvas::{self, Event, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

use crate::{NERD_FONT, NERD_FONT_MONO};

/// Hover state: stores the snapped data-point index (not raw pixel).
#[derive(Debug, Clone, Default)]
pub struct ChartState {
    /// Index of the hovered data point, or None if not hovering.
    pub hover_idx: Option<usize>,
}

/// Colors the chart needs from the active palette.
#[derive(Debug, Clone, Copy)]
pub struct ChartColors {
    pub bg: Color,
    pub border: Color,
    pub grid: Color,
    pub label: Color,
    pub text: Color,
}

/// A line chart drawn via iced Canvas with hover tooltip support.
#[derive(Debug, Clone)]
pub struct LineChart {
    pub series: Vec<(String, Color, Vec<f32>)>,
    pub y_min: f32,
    pub y_max: f32,
    pub title: String,
    pub filled: bool,
    /// Unit suffix for the tooltip (e.g. "%", " B/s", "°C").
    pub unit: String,
    pub colors: ChartColors,
    /// Whether to draw a horizontal average line for each series.
    pub show_avg: bool,
}

impl LineChart {
    /// Number of data points in the longest series.
    fn data_len(&self) -> usize {
        self.series.iter().map(|(_, _, d)| d.len()).max().unwrap_or(0)
    }
}

impl<Message: 'static> canvas::Program<Message> for LineChart {
    type State = ChartState;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        let pad_left = 44.0f32;
        let pad_right = 8.0f32;
        let chart_w = bounds.width - pad_left - pad_right;
        let n = self.data_len();

        let new_idx = match &event {
            Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    if n >= 2 && chart_w > 0.0 && pos.x >= pad_left && pos.x <= pad_left + chart_w {
                        let frac = (pos.x - pad_left) / chart_w;
                        let idx = (frac * (n - 1) as f32).round() as usize;
                        Some(idx.min(n - 1))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Event::Mouse(iced::mouse::Event::CursorLeft) => None,
            _ => return (canvas::event::Status::Ignored, None),
        };

        // Only update state (and thus invalidate cache) when the index actually changes.
        if new_idx != state.hover_idx {
            state.hover_idx = new_idx;
        }
        (canvas::event::Status::Ignored, None)
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let c = &self.colors;

        let pad_left = 44.0f32;
        let pad_right = 8.0f32;
        let pad_top = 22.0f32;
        let pad_bottom = 6.0f32;

        let chart_w = bounds.width - pad_left - pad_right;
        let chart_h = bounds.height - pad_top - pad_bottom;

        if chart_w <= 0.0 || chart_h <= 0.0 {
            return vec![frame.into_geometry()];
        }

        // Background with subtle rounded appearance
        let bg = Path::rectangle(Point::ORIGIN, bounds.size());
        frame.fill(&bg, c.bg);

        // Softer border
        let border = Path::rectangle(Point::new(0.5, 0.5), Size::new(bounds.width - 1.0, bounds.height - 1.0));
        frame.stroke(&border, Stroke::default().with_color(c.border).with_width(0.5));

        // Title
        let mut title_text = Text::from(self.title.clone());
        title_text.position = Point::new(pad_left, 3.0);
        title_text.color = c.text;
        title_text.size = 12.0.into();
        title_text.font = NERD_FONT;
        frame.fill_text(title_text);

        // Y-axis labels + grid — nice round tick values
        let y_range = self.y_max - self.y_min;
        if y_range > 0.0 {
            let step = nice_tick_step(y_range, 10);
            let first_tick = (self.y_min / step).ceil() * step;
            let mut val = first_tick;
            while val <= self.y_max + step * 0.001 {
                let frac = 1.0 - (val - self.y_min) / y_range;
                let y = pad_top + chart_h * frac;

                let grid = Path::line(
                    Point::new(pad_left, y),
                    Point::new(pad_left + chart_w, y),
                );
                frame.stroke(&grid, Stroke::default().with_color(c.grid).with_width(1.0));

                let label_str = if step >= 1.0 { format!("{val:.0}") } else { format!("{val:.1}") };
                let mut label = Text::from(label_str);
                label.position = Point::new(4.0, y - 5.0);
                label.color = c.label;
                label.size = 10.0.into();
                label.font = NERD_FONT_MONO;
                frame.fill_text(label);

                val += step;
            }
        }

        let n = self.data_len();

        // Draw series
        for (_label, color, data) in &self.series {
            if data.len() < 2 {
                continue;
            }
            let dn = data.len();

            // Filled area
            if self.filled {
                let mut builder = canvas::path::Builder::new();
                builder.move_to(Point::new(pad_left, pad_top + chart_h));
                for (i, &val) in data.iter().enumerate() {
                    let x = pad_left + (i as f32 / (dn - 1) as f32) * chart_w;
                    let normalized = if y_range > 0.0 { (val - self.y_min) / y_range } else { 0.5 };
                    let y = pad_top + chart_h * (1.0 - normalized);
                    builder.line_to(Point::new(x, y));
                }
                builder.line_to(Point::new(pad_left + chart_w, pad_top + chart_h));
                builder.close();
                let fill_path = builder.build();
                let fill_color = Color::from_rgba(color.r, color.g, color.b, 0.15);
                frame.fill(&fill_path, fill_color);
            }

            // Line with glow effect
            let mut builder = canvas::path::Builder::new();
            for (i, &val) in data.iter().enumerate() {
                let x = pad_left + (i as f32 / (dn - 1) as f32) * chart_w;
                let normalized = if y_range > 0.0 { (val - self.y_min) / y_range } else { 0.5 };
                let y = pad_top + chart_h * (1.0 - normalized);
                if i == 0 {
                    builder.move_to(Point::new(x, y));
                } else {
                    builder.line_to(Point::new(x, y));
                }
            }
            let path = builder.build();
            // Glow pass: thicker, semi-transparent
            let glow_color = Color::from_rgba(color.r, color.g, color.b, 0.2);
            frame.stroke(&path, Stroke::default().with_color(glow_color).with_width(4.0));
            // Main line
            frame.stroke(&path, Stroke::default().with_color(*color).with_width(1.8));
        }

        // Average line (dashed appearance via dotted segments)
        if self.show_avg {
            for (_label, color, data) in &self.series {
                if data.is_empty() {
                    continue;
                }
                let avg_val = data.iter().sum::<f32>() / data.len() as f32;
                let normalized = if y_range > 0.0 { (avg_val - self.y_min) / y_range } else { 0.5 };
                let y = pad_top + chart_h * (1.0 - normalized);
                // Draw dashed line (alternating segments)
                let dash_len = 6.0;
                let gap_len = 4.0;
                let mut x = pad_left;
                while x < pad_left + chart_w {
                    let end = (x + dash_len).min(pad_left + chart_w);
                    let seg = Path::line(Point::new(x, y), Point::new(end, y));
                    frame.stroke(
                        &seg,
                        Stroke::default()
                            .with_color(Color::from_rgba(color.r, color.g, color.b, 0.5))
                            .with_width(1.0),
                    );
                    x += dash_len + gap_len;
                }
                // Small avg label
                let avg_str = format!("avg {avg_val:.1}");
                let mut avg_text = Text::from(avg_str);
                avg_text.position = Point::new(pad_left + chart_w - 52.0, y - 12.0);
                avg_text.color = Color::from_rgba(color.r, color.g, color.b, 0.6);
                avg_text.size = 9.0.into();
                avg_text.font = NERD_FONT_MONO;
                frame.fill_text(avg_text);
            }
        }

        // Hover: snap to data-point index
        if let Some(idx) = state.hover_idx {
            if n >= 2 && idx < n {
                let snap_x = pad_left + (idx as f32 / (n - 1) as f32) * chart_w;

                // Vertical crosshair at snapped position
                let crosshair = Path::line(
                    Point::new(snap_x, pad_top),
                    Point::new(snap_x, pad_top + chart_h),
                );
                frame.stroke(
                    &crosshair,
                    Stroke::default()
                        .with_color(Color::from_rgba(c.text.r, c.text.g, c.text.b, 0.35))
                        .with_width(1.0),
                );

                // Dot + tooltip for each series
                let mut tooltip_y = pad_top + 4.0;
                for (label, color, data) in &self.series {
                    if idx >= data.len() {
                        continue;
                    }
                    let val = data[idx];

                    let normalized = if y_range > 0.0 { (val - self.y_min) / y_range } else { 0.5 };
                    let dot_y = pad_top + chart_h * (1.0 - normalized);

                    // Outer glow ring on dot
                    let glow = Path::circle(Point::new(snap_x, dot_y), 7.0);
                    frame.fill(&glow, Color::from_rgba(color.r, color.g, color.b, 0.25));
                    // Dot
                    let dot = Path::circle(Point::new(snap_x, dot_y), 4.0);
                    frame.fill(&dot, *color);
                    let ring = Path::circle(Point::new(snap_x, dot_y), 4.0);
                    frame.stroke(&ring, Stroke::default().with_color(c.text).with_width(1.2));

                    // Tooltip
                    let tooltip_str = if self.series.len() > 1 {
                        format!("{}: {:.1}{}", label, val, self.unit)
                    } else {
                        format!("{:.1}{}", val, self.unit)
                    };
                    let text_w = tooltip_str.len() as f32 * 6.6 + 20.0;
                    let tx = (snap_x + 14.0).min(pad_left + chart_w - text_w);

                    // Shadow box (offset slightly)
                    let shadow_path = Path::rectangle(
                        Point::new(tx - 3.0, tooltip_y - 0.0),
                        Size::new(text_w, 18.0),
                    );
                    frame.fill(&shadow_path, Color::from_rgba(0.0, 0.0, 0.0, 0.15));

                    // Background box with better styling
                    let box_path = Path::rectangle(
                        Point::new(tx - 4.0, tooltip_y - 2.0),
                        Size::new(text_w, 18.0),
                    );
                    frame.fill(&box_path, Color::from_rgba(c.bg.r, c.bg.g, c.bg.b, 0.95));
                    // Subtle border on tooltip
                    frame.stroke(&box_path, Stroke::default()
                        .with_color(Color::from_rgba(color.r, color.g, color.b, 0.4))
                        .with_width(0.8));

                    let mut tt = Text::from(tooltip_str);
                    tt.position = Point::new(tx, tooltip_y);
                    tt.color = *color;
                    tt.size = 11.0.into();
                    tt.font = NERD_FONT_MONO;
                    frame.fill_text(tt);
                    tooltip_y += 20.0;
                }
            }
        }

        // Legend (top-right)
        let mut lx = bounds.width - 10.0;
        let ly = 7.0;
        for (label, color, data) in self.series.iter().rev() {
            if let Some(&last) = data.last() {
                let legend_str = format!("{label}: {last:.1}");
                let text_w = legend_str.len() as f32 * 6.0 + 14.0;
                lx -= text_w;
                let dot = Path::circle(Point::new(lx, ly + 3.0), 3.0);
                frame.fill(&dot, *color);
                let mut lt = Text::from(legend_str);
                lt.position = Point::new(lx + 8.0, ly - 2.0);
                lt.color = c.label;
                lt.size = 10.0.into();
                lt.font = NERD_FONT_MONO;
                frame.fill_text(lt);
            }
        }

        vec![frame.into_geometry()]
    }
}

/// Pick a "nice" tick step (1, 2, 5, 10, 20, 50, …) so that the range
/// is divided into at most `max_ticks` intervals.
fn nice_tick_step(range: f32, max_ticks: usize) -> f32 {
    let rough = range / max_ticks as f32;
    let mag = 10f32.powf(rough.log10().floor());
    let norm = rough / mag;
    let nice = if norm <= 1.0 { 1.0 } else if norm <= 2.0 { 2.0 } else if norm <= 5.0 { 5.0 } else { 10.0 };
    (nice * mag).max(f32::EPSILON)
}
