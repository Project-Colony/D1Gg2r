use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Point, Rectangle, Renderer, Theme};
use std::f32::consts::PI;

use crate::{NERD_FONT, NERD_FONT_MONO};

/// Colors needed by the gauge from the active palette.
#[derive(Debug, Clone, Copy)]
pub struct GaugeColors {
    pub bg: Color,
    pub label: Color,
    pub text: Color,
    pub bar_bg: Color,
}

/// A radial arc gauge drawn via iced Canvas.
#[derive(Debug, Clone)]
pub struct RadialGauge {
    /// Current value (0.0 – 100.0)
    pub value: f32,
    /// Label shown below the value (e.g. "CPU", "RAM")
    pub label: String,
    /// The arc color
    pub color: Color,
    /// Colors from palette
    pub colors: GaugeColors,
}

impl<Message: 'static> canvas::Program<Message> for RadialGauge {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let c = &self.colors;

        // Background
        let bg = Path::rectangle(Point::ORIGIN, bounds.size());
        frame.fill(&bg, c.bg);

        let cx = bounds.width / 2.0;
        let cy = bounds.height * 0.52;
        let radius = (bounds.width.min(bounds.height) * 0.38).max(20.0);
        let thickness = (radius * 0.18).max(4.0);

        // Arc spans from 225° to -45° (270° total sweep)
        let start_angle = 225.0_f32.to_radians();
        let total_sweep = 270.0_f32.to_radians();

        // Background arc (track)
        draw_arc(&mut frame, cx, cy, radius, thickness, start_angle, total_sweep, c.bar_bg);

        // Value arc
        let pct = (self.value / 100.0).clamp(0.0, 1.0);
        if pct > 0.001 {
            let value_sweep = total_sweep * pct;
            // Glow pass
            let glow_color = Color::from_rgba(self.color.r, self.color.g, self.color.b, 0.2);
            draw_arc(&mut frame, cx, cy, radius, thickness + 4.0, start_angle, value_sweep, glow_color);
            // Main arc
            draw_arc(&mut frame, cx, cy, radius, thickness, start_angle, value_sweep, self.color);
        }

        // Center value text
        let val_str = format!("{:.0}%", self.value);
        let mut val_text = Text::from(val_str);
        val_text.position = Point::new(cx, cy - 6.0);
        val_text.color = c.text;
        val_text.size = (radius * 0.45).max(12.0).into();
        val_text.font = NERD_FONT_MONO;
        val_text.horizontal_alignment = iced::alignment::Horizontal::Center;
        val_text.vertical_alignment = iced::alignment::Vertical::Center;
        frame.fill_text(val_text);

        // Label below
        let mut label_text = Text::from(self.label.clone());
        label_text.position = Point::new(cx, cy + radius * 0.45);
        label_text.color = c.label;
        label_text.size = (radius * 0.2).max(9.0).into();
        label_text.font = NERD_FONT;
        label_text.horizontal_alignment = iced::alignment::Horizontal::Center;
        label_text.vertical_alignment = iced::alignment::Vertical::Center;
        frame.fill_text(label_text);

        vec![frame.into_geometry()]
    }
}

/// Draw a thick arc by approximating it with many small line segments.
#[allow(clippy::too_many_arguments)]
fn draw_arc(
    frame: &mut Frame,
    cx: f32, cy: f32,
    radius: f32, thickness: f32,
    start: f32, sweep: f32,
    color: Color,
) {
    let segments = ((sweep.abs() / PI * 60.0) as usize).max(8);
    let step = sweep / segments as f32;
    let mut builder = canvas::path::Builder::new();
    // Arcs go clockwise in screen coordinates (y-down),
    // but our start_angle is in standard math coordinates.
    // Convert: screen_angle = -math_angle
    for i in 0..=segments {
        let angle = -(start - step * i as f32);
        let x = cx + radius * angle.cos();
        let y = cy + radius * angle.sin();
        if i == 0 {
            builder.move_to(Point::new(x, y));
        } else {
            builder.line_to(Point::new(x, y));
        }
    }
    let path = builder.build();
    frame.stroke(
        &path,
        Stroke::default()
            .with_color(color)
            .with_width(thickness)
            .with_line_cap(canvas::LineCap::Round),
    );
}


/// A tiny sparkline drawn via iced Canvas (for sidebar).
#[derive(Debug, Clone)]
pub struct Sparkline {
    pub data: Vec<f32>,
    pub color: Color,
}

impl<Message: 'static> canvas::Program<Message> for Sparkline {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if self.data.len() < 2 {
            return vec![frame.into_geometry()];
        }

        let n = self.data.len();
        let max_val = self.data.iter().cloned().fold(1.0_f32, f32::max);
        let min_val = self.data.iter().cloned().fold(0.0_f32, f32::min);
        let range = (max_val - min_val).max(0.01);

        let w = bounds.width;
        let h = bounds.height;
        let pad = 1.0;

        // Filled area
        let mut fill_builder = canvas::path::Builder::new();
        fill_builder.move_to(Point::new(0.0, h));
        for (i, &val) in self.data.iter().enumerate() {
            let x = (i as f32 / (n - 1) as f32) * w;
            let y = pad + (h - 2.0 * pad) * (1.0 - (val - min_val) / range);
            fill_builder.line_to(Point::new(x, y));
        }
        fill_builder.line_to(Point::new(w, h));
        fill_builder.close();
        let fill_path = fill_builder.build();
        let fill_color = Color::from_rgba(self.color.r, self.color.g, self.color.b, 0.15);
        frame.fill(&fill_path, fill_color);

        // Line
        let mut builder = canvas::path::Builder::new();
        for (i, &val) in self.data.iter().enumerate() {
            let x = (i as f32 / (n - 1) as f32) * w;
            let y = pad + (h - 2.0 * pad) * (1.0 - (val - min_val) / range);
            if i == 0 {
                builder.move_to(Point::new(x, y));
            } else {
                builder.line_to(Point::new(x, y));
            }
        }
        let path = builder.build();
        frame.stroke(&path, Stroke::default().with_color(self.color).with_width(1.2));

        vec![frame.into_geometry()]
    }
}
