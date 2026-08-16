use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use chrono::{DateTime, Datelike, Local, Timelike};

use crate::{
    analyzer::{AnalysisResult, AnalyzerError, FriendGraph, analyze},
    database::resolve_database_path,
    diagnostics::{append_log, default_log_path},
    graph,
    model::{AnalysisRequest, AppSettings, AppStatus, MissingDataBehavior, WeeklyGraph},
    refresh::{RefreshCoordinator, RefreshReason},
    settings::{default_settings_path, load_settings, save_settings},
    validation::{
        DateTimeParts, local_datetime_from_parts, parse_friend_ids, parse_user_id,
        validate_bucket_duration, validate_time_range,
    },
};

pub const APP_TITLE: &str = "VRCX Optimal Time";
pub const INITIAL_WINDOW_SIZE: [f32; 2] = [1280.0, 720.0];

pub struct VrcxOptimalTimeApp {
    pub settings: AppSettings,
    pub status: AppStatus,
    pub weekly_graph: Option<WeeklyGraph>,
    friend_graphs: Vec<FriendGraph>,
    selected_friend_id: Option<String>,
    your_id_edit: String,
    friend_ids_edit: String,
    friend_ids_invalid_lines: Vec<usize>,
    start_time_draft: TimeBoundDraft,
    end_time_draft: TimeBoundDraft,
    weekday_visible: [bool; 7],
    refresh: RefreshCoordinator,
    completion_sender: Sender<RefreshCompletion>,
    completion_receiver: Receiver<RefreshCompletion>,
    settings_path: Option<std::path::PathBuf>,
    launch_refresh_scheduled: bool,
}

struct RefreshCompletion {
    generation: u64,
    result: Result<AnalysisResult, AppStatus>,
}

impl Default for VrcxOptimalTimeApp {
    fn default() -> Self {
        let (completion_sender, completion_receiver) = mpsc::channel();
        let (settings_path, settings, startup_status) = match default_settings_path() {
            Ok(path) => match load_settings(&path) {
                Ok(settings) => (Some(path), settings, AppStatus::Idle),
                Err(error) => (
                    Some(path),
                    AppSettings::default(),
                    AppStatus::Error(error.to_string()),
                ),
            },
            Err(error) => (
                None,
                AppSettings::default(),
                AppStatus::Error(error.to_string()),
            ),
        };
        let your_id_edit = settings.analysis.your_user_id.clone();
        let friend_ids_edit = settings.analysis.friend_ids.join("\n");
        let start_time_draft = TimeBoundDraft::from_value(settings.analysis.start_time);
        let end_time_draft = TimeBoundDraft::from_value(settings.analysis.end_time);
        Self {
            settings,
            status: startup_status,
            weekly_graph: None,
            friend_graphs: Vec::new(),
            selected_friend_id: None,
            your_id_edit,
            friend_ids_edit,
            friend_ids_invalid_lines: Vec::new(),
            start_time_draft,
            end_time_draft,
            weekday_visible: [true; 7],
            refresh: RefreshCoordinator::default(),
            completion_sender,
            completion_receiver,
            settings_path,
            launch_refresh_scheduled: false,
        }
    }
}

impl eframe::App for VrcxOptimalTimeApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        if !self.launch_refresh_scheduled {
            self.launch_refresh_scheduled = true;
            self.schedule_refresh(RefreshReason::Launch);
        }
        self.collect_completions();
        self.dispatch_due_refresh();
        ui.ctx().request_repaint_after(Duration::from_millis(50));
        let mut analysis_changed = false;
        let mut settings_changed = self.capture_window_state(ui);
        let mut database_refresh = false;
        let left_panel = eframe::egui::Panel::left("controls")
            .default_size(320.0)
            .min_size(280.0)
            .max_size(420.0)
            .resizable(false);
        left_panel.show(ui, |ui| {
            eframe::egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(APP_TITLE);
                ui.separator();
                ui.label("Your ID");
                let id_changed = ui.text_edit_singleline(&mut self.your_id_edit).changed();
                if id_changed {
                    match parse_user_id(&self.your_id_edit, 1) {
                        Ok(value) => {
                            self.settings.analysis.your_user_id = value;
                            analysis_changed = true;
                            settings_changed = true;
                        }
                        Err(error) => self.status = AppStatus::Error(error.to_string()),
                    }
                }
                if self.your_id_edit.is_empty() {
                    ui.colored_label(eframe::egui::Color32::LIGHT_RED, "A valid ID is required.");
                }

                let collapse_id = ui.make_persistent_id("friend-ids");
                let state =
                    eframe::egui::collapsing_header::CollapsingState::load_with_default_open(
                        ui.ctx(),
                        collapse_id,
                        !self.settings.window.friend_ids_collapsed,
                    );
                let header = state.show_header(ui, |ui| ui.label("Friend IDs"));
                let is_open = header.is_open();
                let (_, _, _) = header.body(|ui| {
                    if ui.text_edit_multiline(&mut self.friend_ids_edit).changed() {
                        self.friend_ids_invalid_lines =
                            apply_friend_ids_edit(&mut self.settings, &self.friend_ids_edit);
                        settings_changed = true;
                        if self.friend_ids_invalid_lines.is_empty() {
                            analysis_changed = true;
                        }
                    }
                    if !self.friend_ids_invalid_lines.is_empty() {
                        ui.colored_label(
                            eframe::egui::Color32::LIGHT_RED,
                            format!("Invalid lines: {:?}", self.friend_ids_invalid_lines),
                        );
                    }
                    let mut remove_index = None;
                    for (index, friend_id) in self.settings.analysis.friend_ids.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(friend_id);
                            if self.friend_ids_invalid_lines.is_empty()
                                && ui.small_button("Remove").clicked()
                            {
                                remove_index = Some(index);
                            }
                        });
                    }
                    if let Some(index) = remove_index {
                        self.settings.analysis.friend_ids.remove(index);
                        self.friend_ids_edit = self.settings.analysis.friend_ids.join("\n");
                        analysis_changed = true;
                        settings_changed = true;
                    }
                });
                let collapsed = !is_open;
                if collapsed != self.settings.window.friend_ids_collapsed {
                    self.settings.window.friend_ids_collapsed = collapsed;
                    settings_changed = true;
                }

                ui.separator();
                ui.label("Options");
                ui.label("VRCX running threshold (minutes)");
                let mut uptime_minutes = self.settings.analysis.uptime_threshold.as_secs() / 60;
                let uptime_response =
                    ui.add(eframe::egui::DragValue::new(&mut uptime_minutes).range(1..=120));
                if uptime_response.changed() {
                    self.settings.analysis.uptime_threshold =
                        Duration::from_secs(uptime_minutes * 60);
                    analysis_changed = true;
                    settings_changed = true;
                }
                uptime_response.on_hover_text(
                    "Maximum gap between VRCX events before your running session is split.",
                );

                ui.label("Bucket duration (minutes)");
                let mut bucket_minutes = self.settings.analysis.bucket_duration.as_secs() / 60;
                let bucket_response =
                    ui.add(eframe::egui::DragValue::new(&mut bucket_minutes).range(1..=1440));
                if bucket_response.changed() {
                    let candidate = Duration::from_secs(bucket_minutes * 60);
                    if validate_bucket_duration(candidate).is_ok() {
                        self.settings.analysis.bucket_duration = candidate;
                        analysis_changed = true;
                        settings_changed = true;
                    }
                }
                bucket_response.on_hover_text(
                    "Width of each graph time slot. Smaller values show more detail.",
                );

                if ui
                    .checkbox(&mut self.settings.analysis.normalize, "Normalize")
                    .changed()
                {
                    analysis_changed = true;
                    settings_changed = true;
                }
                ui.label("Analysis time range (local time)");
                if render_time_bound(ui, "Start", &mut self.start_time_draft) {
                    match commit_time_bound(
                        &self.start_time_draft,
                        self.settings.analysis.end_time,
                        true,
                    ) {
                        Ok(value) => {
                            self.settings.analysis.start_time = value;
                            self.start_time_draft.error = None;
                            analysis_changed = true;
                            settings_changed = true;
                        }
                        Err(error) => self.start_time_draft.error = Some(error),
                    }
                }
                if render_time_bound(ui, "End", &mut self.end_time_draft) {
                    match commit_time_bound(
                        &self.end_time_draft,
                        self.settings.analysis.start_time,
                        false,
                    ) {
                        Ok(value) => {
                            self.settings.analysis.end_time = value;
                            self.end_time_draft.error = None;
                            analysis_changed = true;
                            settings_changed = true;
                        }
                        Err(error) => self.end_time_draft.error = Some(error),
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Database");
                    let database_label =
                        self.settings.analysis.database_path.as_deref().map_or_else(
                            || "Default VRCX path".to_owned(),
                            |path| path.display().to_string(),
                        );
                    ui.label(database_label);
                    if ui.button("Choose...").clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter("SQLite database", &["sqlite3", "sqlite", "db"])
                            .pick_file()
                    {
                        self.settings.analysis.database_path = Some(path);
                        analysis_changed = true;
                        settings_changed = true;
                    }
                    if self.settings.analysis.database_path.is_some()
                        && ui.button("Use default").clicked()
                    {
                        self.settings.analysis.database_path = None;
                        analysis_changed = true;
                        settings_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Minimum activations per bucket");
                    let response = ui.add(
                        eframe::egui::DragValue::new(
                            &mut self.settings.analysis.minimum_activations,
                        )
                        .range(1..=100),
                    );
                    if response.changed() {
                        analysis_changed = true;
                        settings_changed = true;
                    }
                    response.on_hover_text(
                        "Hide time slots with fewer matching friend online intervals.",
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Missing data");
                    let missing_data_changed = ui
                        .selectable_value(
                            &mut self.settings.analysis.missing_data,
                            MissingDataBehavior::Gap,
                            "Gap",
                        )
                        .changed();
                    let missing_data_changed = missing_data_changed
                        || ui
                            .selectable_value(
                                &mut self.settings.analysis.missing_data,
                                MissingDataBehavior::Zero,
                                "Zero",
                            )
                            .changed();
                    if missing_data_changed {
                        analysis_changed = true;
                        settings_changed = true;
                    }
                });
                if ui.button("Refresh database").clicked() {
                    database_refresh = true;
                }
            });
        });

        eframe::egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Weekly availability");
            ui.horizontal(|ui| {
                if matches!(self.status, AppStatus::Calculating) {
                    ui.spinner();
                }
                let color = match self.status {
                    AppStatus::Warning(_) => eframe::egui::Color32::YELLOW,
                    AppStatus::Error(_) => eframe::egui::Color32::LIGHT_RED,
                    _ => ui.visuals().text_color(),
                };
                ui.colored_label(color, self.status.label());
                if matches!(&self.status, AppStatus::Warning(message) if message.contains("database was not found"))
                    && ui.button("Choose database...").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("SQLite database", &["sqlite3", "sqlite", "db"])
                        .pick_file()
                {
                    self.settings.analysis.database_path = Some(path);
                    settings_changed = true;
                    database_refresh = true;
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (visible, name) in self.weekday_visible.iter_mut().zip(graph::WEEKDAY_NAMES) {
                    ui.checkbox(visible, name);
                }
            });
            if !self.friend_graphs.is_empty() {
                let selected_label = self.selected_friend_id.as_deref().unwrap_or("All friends");
                eframe::egui::ComboBox::from_label("Graph")
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.selected_friend_id, None, "All friends");
                        for friend in &self.friend_graphs {
                            ui.selectable_value(
                                &mut self.selected_friend_id,
                                Some(friend.friend_id.clone()),
                                &friend.friend_id,
                            );
                        }
                    });
            }
            let selected_graph = self
                .selected_friend_id
                .as_ref()
                .and_then(|selected| {
                    self.friend_graphs
                        .iter()
                        .find(|friend| &friend.friend_id == selected)
                        .map(|friend| &friend.graph)
                })
                .or(self.weekly_graph.as_ref());
            if let Some(graph_data) = selected_graph {
                graph::render(ui, graph_data, &self.weekday_visible);
            } else {
                ui.centered_and_justified(|ui| ui.label("No analysis result yet."));
            }
        });

        if settings_changed {
            self.persist_settings();
        }
        if database_refresh {
            self.schedule_refresh(RefreshReason::DatabaseRefresh);
        } else if analysis_changed {
            self.schedule_refresh(RefreshReason::ControlChanged);
        }
    }
}

impl VrcxOptimalTimeApp {
    fn capture_window_state(&mut self, ui: &eframe::egui::Ui) -> bool {
        let (size, position) = ui.ctx().input(|input| {
            let viewport = input.viewport();
            (
                viewport
                    .inner_rect
                    .map(|rect| [rect.width(), rect.height()]),
                viewport
                    .outer_rect
                    .or(viewport.inner_rect)
                    .map(|rect| [rect.min.x, rect.min.y]),
            )
        });
        let mut changed = false;
        if let Some(size) = size
            && size != self.settings.window.size
        {
            self.settings.window.size = size;
            changed = true;
        }
        if let Some(position) = position
            && self.settings.window.position != Some(position)
        {
            self.settings.window.position = Some(position);
            changed = true;
        }
        changed
    }

    fn schedule_refresh(&mut self, reason: RefreshReason) {
        let user_id = match parse_user_id(&self.your_id_edit, 1) {
            Ok(user_id) => user_id,
            Err(error) => {
                self.status = AppStatus::Error(error.to_string());
                return;
            }
        };
        if !self.friend_ids_invalid_lines.is_empty() {
            self.status = AppStatus::Error(format!(
                "invalid friend ID lines: {:?}",
                self.friend_ids_invalid_lines
            ));
            return;
        }
        let database_path =
            match resolve_database_path(self.settings.analysis.database_path.as_deref()) {
                Ok(path) => path,
                Err(error) => {
                    self.status = AppStatus::Error(error.to_string());
                    return;
                }
            };
        let analysis = &self.settings.analysis;
        let request = AnalysisRequest {
            your_user_id: user_id,
            friend_ids: analysis.friend_ids.clone(),
            database_path,
            uptime_threshold: analysis.uptime_threshold,
            bucket_duration: analysis.bucket_duration,
            normalize: analysis.normalize,
            start_time: analysis.start_time,
            end_time: analysis.end_time,
            minimum_activations: analysis.minimum_activations,
            missing_data: analysis.missing_data,
        };
        self.refresh.request(request, reason, Instant::now());
        self.status = AppStatus::Calculating;
    }

    fn dispatch_due_refresh(&mut self) {
        let Some(job) = self.refresh.poll(Instant::now()) else {
            return;
        };
        let sender = self.completion_sender.clone();
        thread::spawn(move || {
            let result = analyze(&job.request).map_err(|error| status_for_analysis_error(&error));
            let _ = sender.send(RefreshCompletion {
                generation: job.generation,
                result,
            });
        });
    }

    fn collect_completions(&mut self) {
        while let Ok(completion) = self.completion_receiver.try_recv() {
            if !self.refresh.is_current(completion.generation) {
                continue;
            }
            match completion.result {
                Ok(result) => {
                    let has_data = result.graph.weekdays.iter().flatten().any(Option::is_some);
                    self.weekly_graph = Some(result.graph);
                    self.friend_graphs = result.friend_graphs;
                    if self.selected_friend_id.as_ref().is_some_and(|selected| {
                        !self
                            .friend_graphs
                            .iter()
                            .any(|friend| &friend.friend_id == selected)
                    }) {
                        self.selected_friend_id = None;
                    }
                    self.status = if has_data {
                        AppStatus::Updated
                    } else {
                        AppStatus::Empty
                    };
                }
                Err(status) => {
                    self.log_message(status.label());
                    self.status = status;
                }
            }
        }
    }

    fn persist_settings(&mut self) {
        let Some(path) = self.settings_path.as_deref() else {
            return;
        };
        if let Err(error) = save_settings(path, &self.settings) {
            let message = error.to_string();
            self.log_message(&message);
            self.status = AppStatus::Error(message);
        }
    }

    fn log_message(&self, message: &str) {
        if let Some(path) = default_log_path() {
            let _ = append_log(&path, message);
        }
    }
}

fn status_for_analysis_error(error: &AnalyzerError) -> AppStatus {
    match error {
        AnalyzerError::DatabaseNotFound { path } => AppStatus::Warning(format!(
            "Warning: VRCX database was not found at {}. Choose the database file below.",
            path.display()
        )),
        AnalyzerError::NoUsableActivity { path } => AppStatus::Warning(format!(
            "Warning: VRCX database at {} has too little usable activity history. Capture more VRCX data and try again.",
            path.display()
        )),
        _ => AppStatus::Error(error.to_string()),
    }
}

fn apply_friend_ids_edit(settings: &mut AppSettings, input: &str) -> Vec<usize> {
    let report = parse_friend_ids(input);
    settings.analysis.friend_ids = report.ids;
    report.invalid_lines
}

#[derive(Debug, Clone)]
struct TimeBoundDraft {
    enabled: bool,
    parts: DateTimeParts,
    error: Option<String>,
}

impl TimeBoundDraft {
    fn from_value(value: Option<DateTime<Local>>) -> Self {
        let parts = value.map_or(
            DateTimeParts {
                year: 2000,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
            },
            |value| DateTimeParts {
                year: value.year(),
                month: value.month(),
                day: value.day(),
                hour: value.hour(),
                minute: value.minute(),
            },
        );
        Self {
            enabled: value.is_some(),
            parts,
            error: None,
        }
    }
}

fn render_time_bound(ui: &mut eframe::egui::Ui, label: &str, draft: &mut TimeBoundDraft) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        if ui.checkbox(&mut draft.enabled, label).changed() {
            changed = true;
        }
        ui.label("date/time");
    });
    ui.add_enabled_ui(draft.enabled, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("YYYY");
            changed |= ui
                .add(eframe::egui::DragValue::new(&mut draft.parts.year).range(1..=9999))
                .changed();
            ui.label("MM");
            changed |= ui
                .add(eframe::egui::DragValue::new(&mut draft.parts.month).range(1..=12))
                .changed();
            ui.label("DD");
            changed |= ui
                .add(eframe::egui::DragValue::new(&mut draft.parts.day).range(1..=31))
                .changed();
            ui.label("HH");
            changed |= ui
                .add(eframe::egui::DragValue::new(&mut draft.parts.hour).range(0..=23))
                .changed();
            ui.label("mm");
            changed |= ui
                .add(eframe::egui::DragValue::new(&mut draft.parts.minute).range(0..=59))
                .changed();
        });
    });
    if let Some(error) = &draft.error {
        ui.colored_label(eframe::egui::Color32::LIGHT_RED, error);
    }
    changed
}

fn commit_time_bound(
    draft: &TimeBoundDraft,
    other: Option<DateTime<Local>>,
    is_start: bool,
) -> Result<Option<DateTime<Local>>, String> {
    if !draft.enabled {
        return Ok(None);
    }
    let value = local_datetime_from_parts(draft.parts).map_err(|error| error.to_string())?;
    let range = if is_start {
        validate_time_range(Some(value), other)
    } else {
        validate_time_range(other, Some(value))
    };
    range.map_err(|error| error.to_string())?;
    Ok(Some(value))
}

pub fn native_window_options() -> eframe::NativeOptions {
    let persisted_window = default_settings_path()
        .ok()
        .and_then(|path| load_settings(&path).ok())
        .map(|settings| settings.window);
    let mut viewport = eframe::egui::ViewportBuilder::default().with_inner_size(
        persisted_window
            .as_ref()
            .map_or(INITIAL_WINDOW_SIZE, |window| window.size),
    );
    if let Some(position) = persisted_window.and_then(|window| window.position) {
        viewport = viewport.with_position(position);
    }
    eframe::NativeOptions {
        viewport,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TimeBoundDraft, apply_friend_ids_edit, commit_time_bound, status_for_analysis_error,
    };
    use crate::{
        analyzer::AnalyzerError,
        model::{AppSettings, AppStatus},
        validation::DateTimeParts,
    };
    use chrono::{Local, TimeZone};
    use std::path::PathBuf;

    #[test]
    fn missing_database_is_classified_as_warning() {
        let status = status_for_analysis_error(&AnalyzerError::DatabaseNotFound {
            path: PathBuf::from("C:/missing/VRCX.sqlite3"),
        });

        assert!(matches!(
            status,
            AppStatus::Warning(message) if message.contains("VRCX database was not found")
        ));
    }

    #[test]
    fn insufficient_activity_is_classified_as_warning() {
        let status = status_for_analysis_error(&AnalyzerError::NoUsableActivity {
            path: PathBuf::from("C:/VRCX.sqlite3"),
        });

        assert!(matches!(
            status,
            AppStatus::Warning(message) if message.contains("too little usable activity history")
        ));
    }

    #[test]
    fn mixed_friend_paste_keeps_valid_ids_and_reports_invalid_lines() {
        let mut settings = AppSettings::default();
        let invalid_lines = apply_friend_ids_edit(
            &mut settings,
            "usr_550e8400-e29b-41d4-a716-446655440000\nbad\nusr_550E8400-E29B-41D4-A716-446655440000",
        );

        assert_eq!(
            settings.analysis.friend_ids,
            vec!["usr_550e8400-e29b-41d4-a716-446655440000"]
        );
        assert_eq!(invalid_lines, vec![2]);
    }

    #[test]
    fn numeric_bound_commit_rejects_impossible_date_and_reverse_range() {
        let invalid = TimeBoundDraft {
            enabled: true,
            parts: DateTimeParts {
                year: 2024,
                month: 2,
                day: 30,
                hour: 0,
                minute: 0,
            },
            error: None,
        };
        assert!(commit_time_bound(&invalid, None, true).is_err());

        let later_start = TimeBoundDraft {
            enabled: true,
            parts: DateTimeParts {
                year: 2024,
                month: 2,
                day: 2,
                hour: 0,
                minute: 0,
            },
            error: None,
        };
        let earlier_end = Local.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).single();
        assert!(commit_time_bound(&later_start, earlier_end, true).is_err());
    }

    #[test]
    fn numeric_bound_draft_round_trips_local_components() {
        let value = Local.with_ymd_and_hms(2024, 2, 29, 23, 58, 0).single();
        let draft = TimeBoundDraft::from_value(value);

        assert!(draft.enabled);
        assert_eq!(draft.parts.year, 2024);
        assert_eq!(draft.parts.month, 2);
        assert_eq!(draft.parts.day, 29);
        assert_eq!(draft.parts.hour, 23);
        assert_eq!(draft.parts.minute, 58);
    }

    #[test]
    fn disabled_numeric_bound_clears_its_saved_value() {
        let draft = TimeBoundDraft {
            enabled: false,
            parts: DateTimeParts {
                year: 2024,
                month: 2,
                day: 29,
                hour: 23,
                minute: 58,
            },
            error: None,
        };
        let other = Local.with_ymd_and_hms(2024, 2, 29, 23, 59, 0).single();

        assert_eq!(commit_time_bound(&draft, other, true).unwrap(), None);
    }
}
