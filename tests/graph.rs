use std::time::Duration;

use vrcx_optimal_time_app::{
    graph::{WEEKDAY_NAMES, format_hover_coordinates, project, weekday_points, weekday_values},
    model::WeeklyGraph,
};

#[test]
fn ten_minute_graph_projects_week_labels_and_144_buckets() {
    let graph = WeeklyGraph {
        bucket_duration: Duration::from_secs(10 * 60),
        weekdays: std::array::from_fn(|weekday| {
            (0..144)
                .map(|bucket| {
                    if weekday == 0 && bucket == 3 {
                        None
                    } else {
                        Some(bucket as f64)
                    }
                })
                .collect()
        }),
    };

    let projection = project(&graph);

    assert_eq!(projection.labels.len(), 144);
    assert_eq!(projection.labels.first().unwrap(), "00:00");
    assert_eq!(projection.labels.last().unwrap(), "23:50");
    assert_eq!(projection.weekday_names, WEEKDAY_NAMES);
    assert_eq!(weekday_values(&graph, 0).len(), 144);
    assert_eq!(weekday_values(&graph, 0)[3], None);
    let points = weekday_points(&graph, 0);
    assert!(matches!(points, egui_plot::PlotPoints::Owned(_)));
}

#[test]
fn hover_coordinates_show_local_time_and_value() {
    assert_eq!(
        format_hover_coordinates(Duration::from_secs(10 * 60), 3, 2.5),
        "Time 00:30 | Value 2.50"
    );
}
