# VRCX Optimal Time App Executable Specification

Status: autonomous implementation evidence plan
Source design: `2026-08-16-vrcx-optimal-time-app-design.md`
Source plan: `../plans/2026-08-16-vrcx-optimal-time-app.md`
Spec approval: not separately obtained; the user authorized autonomous implementation of the approved design and roadmap.

The approved design and roadmap remain the governing product requirements. This
document makes their behaviors executable and records extra verification
evidence; it does not redefine product scope.

## Scope And Setup

This specification covers version `0.1.0` of the single Windows executable
`VRCXOptimalTimeApp.exe`. The application reads VRCX SQLite data only through
read-only connections. It must never change the database, produce a CSV as
part of its application flow, start a browser, or require an analyzer sidecar
or installer.

The crate uses Rust 2024 and the dependencies named in the implementation
plan: `chrono`, `eframe`, `egui_plot`, `num-traits`, `rfd`, `rusqlite` with
bundled SQLite, `serde`, `thiserror`, `toml`, and build dependency
`winresource`. Each directly supports the approved native UI, typed analysis,
validation, persistence, read-only SQLite access, errors, or Windows metadata.
No new dependencies may be introduced without documenting their purpose here.

The implementation is developed through test-first Red-Green-Refactor cycles.
Each added behavior has a persisted Cargo test, and every test must first fail
because the behavior is absent. Final evidence runs formatting, all tests,
Clippy with warnings denied, package build, and package smoke test. The
implementation records any unavailable live-VRCX checks rather than claiming
them as complete.

## Executable Acceptance Criteria

### Application Shell

- Given a Windows desktop, when `VRCXOptimalTimeApp.exe` starts normally, then
  one native `eframe` window titled `VRCX Optimal Time` opens at an initial
  size of 1280 by 720 logical points.
- Given a caller constructs the app, when no settings have loaded, then it can
  construct `VrcxOptimalTimeApp::default()` without starting analysis on the
  UI thread.
- Given persisted and transient state, when models cross layer boundaries,
  then `AppSettings`, `AnalysisSettings`, `WindowSettings`,
  `AnalysisRequest`, `WeeklyGraph`, and `AppStatus` retain typed boundaries;
  transient UI state is not persisted as settings.

### Validation And Persistence

- Given a VRChat ID shaped `usr_` plus a UUID, when parsed, then its UUID
  letters normalize to lowercase.
- Given empty, malformed, or multiline IDs, when parsed, then invalid lines
  report their one-based line numbers; valid first-seen IDs remain ordered and
  duplicates are removed.
- Given a bucket duration, when it does not divide 24 hours evenly, then it is
  rejected before calculation.
- Given no settings file, when settings load, then defaults return. Given a
  valid version-1 file, then it migrates by filling current defaults. Given
  malformed TOML or a later schema version, then loading returns a readable
  error rather than overwriting the file.
- Given valid settings, when saved, then a temporary replacement write leaves
  one complete `settings.toml` under `%LOCALAPPDATA%\\VRCX Optimal Time App`.

### Analyzer And Read-Only Database Boundary

- Given deterministic SQLite fixtures, when `analyze(AnalysisRequest)` runs,
  then it returns seven typed weekday bucket series and diagnostics matching
  `tests/fixtures/expected.json`.
- Given online/offline events, VRCX uptime gaps, filtering, unmatched events,
  normalization, minimum activations, or missing data, then fixture tests
  preserve approved analyzer semantics for each case.
- Given any analyzed SQLite file, when analysis finishes or fails, then the
  full database file set (main file and any `-wal`, `-shm`, or `-journal`
  sidecars) has the same names and SHA-256 hashes. Connections use SQLite
  read-only flags and a five-second bounded busy timeout.
- Given a missing, locked, unreadable, or incompatible database, when opening
  or querying fails, then the user-facing result identifies the checked path
  and maps to a typed database error.

### Native UI, Graph, And Refresh

- Given a valid ID and database, when settings change, then analysis begins
  automatically after a 300 ms debounce. There is no Apply button.
- Given one launch request, three valid edits within 300 ms, and one later
  edit, then the refresh coordinator emits one launch request, one coalesced
  request, and one later request.
- Given an in-flight generation and a newer generation, when the older result
  arrives, then it is discarded. Given a refresh failure, the last graph stays
  visible and status becomes a typed error.
- Given the main window, when the control rail scrolls or Friend IDs expands,
  then only the fixed-width left rail moves; the graph pane stays fixed. Your
  ID and Options remain visible and Friend IDs is the only collapsible section.
- Given a ten-minute analysis result, when projected, then the graph has 144
  local-time labels and seven Monday-through-Sunday series. Missing values stay
  gaps unless zero-on-no-data is selected.

### Release Boundary

- Given the release scripts, when packaging succeeds, then `dist` contains
  exactly `VRCXOptimalTimeApp.exe`, README, notices, and a SHA-256 manifest;
  no analyzer executable or configuration file sits beside the executable.
- Given `--self-test <fixture>`, when the packaged executable analyzes the
  deterministic fixture, then it exits zero only when its output matches the
  expected fixture result.
- Given a clean profile, when the app starts without configuration, then it
  preserves readable validation and database-error states, persists valid
  settings and window state, and never writes VRCX data.

## Evidence Matrix

| Area | Primary evidence |
| --- | --- |
| Shell | compile-only test, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings` |
| Validation and persistence | `tests/validation.rs`, `tests/settings.rs` |
| Analyzer and read-only access | `tests/analyzer.rs`, main-and-sidecar fixture hashes, `expected.json` parity |
| UI projection and refresh | `tests/graph.rs`, `tests/refresh.rs`, manual Windows acceptance pass |
| Packaging | `packaging/build-windows.ps1`, `packaging/smoke-test.ps1`, packaged self-test |
| Release | complete gate plus separately recorded live-VRCX limitations |

## Invariants

- Do not modify, repair, or write to a VRCX SQLite file.
- Do not copy code from the unlicensed viewer repository.
- Retain MIT attribution for reused analyzer code and list redistributable
  third-party notices.
- Keep `Example/` untracked and untouched; it contains local reference inputs
  rather than release assets or test fixtures.
- Do not report a live VRCX compatibility result without a fresh test against
  an authorized live copy.
