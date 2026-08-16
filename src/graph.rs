use std::time::Duration;

use eframe::egui::Vec2;
use egui_plot::{CoordinatesFormatter, Corner, Legend, Line, Plot, PlotPoints};

use crate::model::WeeklyGraph;

pub const WEEKDAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphProjection {
    pub labels: Vec<String>,
    pub weekday_names: [&'static str; 7],
}

pub fn project(graph: &WeeklyGraph) -> GraphProjection {
    let bucket_count = graph.weekdays.first().map_or(0, Vec::len);
    GraphProjection {
        labels: (0..bucket_count)
            .map(|index| format_bucket_label(graph.bucket_duration, index))
            .collect(),
        weekday_names: WEEKDAY_NAMES,
    }
}

pub fn weekday_points(graph: &WeeklyGraph, weekday: usize) -> PlotPoints<'static> {
    weekday_values(graph, weekday)
        .into_iter()
        .enumerate()
        .map(|(index, value)| [index as f64, value.unwrap_or(f64::NAN)])
        .collect()
}

pub fn weekday_values(graph: &WeeklyGraph, weekday: usize) -> Vec<Option<f64>> {
    graph.weekdays.get(weekday).cloned().unwrap_or_default()
}

pub fn render(ui: &mut eframe::egui::Ui, graph: &WeeklyGraph, visible: &[bool; 7]) {
    let bucket_duration = graph.bucket_duration;
    let reset_view = ui
        .button("Reset view")
        .on_hover_text("Restore the automatic graph bounds")
        .clicked();
    let mut plot = Plot::new("weekly-availability")
        .legend(Legend::default())
        .allow_scroll(false)
        .allow_zoom(true)
        .allow_boxed_zoom(false)
        .x_axis_label("Local time bucket")
        .y_axis_label("Availability")
        .coordinates_formatter(
            Corner::RightTop,
            CoordinatesFormatter::new(move |point, _| {
                format_hover_coordinates(
                    bucket_duration,
                    point.x.round().max(0.0) as usize,
                    point.y,
                )
            }),
        )
        .x_axis_formatter(move |mark, _range| {
            format_bucket_label(bucket_duration, mark.value.max(0.0).round() as usize)
        });
    if reset_view {
        plot = plot.reset();
    }
    plot.show(ui, |plot_ui| {
        if plot_ui.response().contains_pointer() {
            let scroll_y = plot_ui.ctx().input(|input| input.smooth_scroll_delta.y);
            if let Some(factor) = wheel_zoom_factor(scroll_y) {
                plot_ui.zoom_bounds_around_hovered(Vec2::splat(factor));
            }
        }
        for (weekday, name) in WEEKDAY_NAMES.iter().enumerate() {
            if !visible[weekday] {
                continue;
            }
            plot_ui.line(Line::new(*name, weekday_points(graph, weekday)));
        }
    });
}

const WHEEL_ZOOM_SENSITIVITY: f32 = 0.002;

fn wheel_zoom_factor(scroll_y: f32) -> Option<f32> {
    if scroll_y.abs() < f32::EPSILON {
        return None;
    }
    Some((scroll_y * WHEEL_ZOOM_SENSITIVITY).clamp(-0.25, 0.25).exp())
}

pub fn format_hover_coordinates(duration: Duration, index: usize, value: f64) -> String {
    format!(
        "Time {} | Value {value:.2}",
        format_bucket_label(duration, index)
    )
}

fn format_bucket_label(duration: Duration, index: usize) -> String {
    let seconds = duration.as_secs().saturating_mul(index as u64);
    let hour = (seconds / 3_600) % 24;
    let minute = (seconds % 3_600) / 60;
    let second = seconds % 60;
    if second == 0 {
        format!("{hour:02}:{minute:02}")
    } else {
        format!("{hour:02}:{minute:02}:{second:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::wheel_zoom_factor;

    #[test]
    fn wheel_zoom_factor_is_cursor_zoom_direction_and_clamped() {
        assert!(wheel_zoom_factor(100.0).unwrap() > 1.0);
        assert!(wheel_zoom_factor(-100.0).unwrap() < 1.0);
        assert_eq!(wheel_zoom_factor(0.0), None);
        assert_eq!(wheel_zoom_factor(f32::MAX).unwrap(), 0.25_f32.exp());
    }
}
