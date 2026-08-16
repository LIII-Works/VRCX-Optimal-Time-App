# VRCX Optimal Time App Design

Status: Approved for implementation planning
Approved: 2026-08-16
Target version: 0.1.0
User-facing executable: `VRCXOptimalTimeApp.exe`

## 1. Product Goal

VRCX Optimal Time App combines the calculation behavior of `vrcx-optimal-time` with the charting behavior of `vrcx-optimal-time-viewer` in one Windows desktop application.

The user starts one portable executable, enters their VRChat user ID and optional friend IDs, chooses analysis options, and sees the weekly availability graph without exporting CSV data or changing pages.

The application reads the VRCX SQLite database in read-only mode. It must never modify the VRCX database.

## 2. Confirmed Product Decisions

- V1 targets Windows only.
- The release is a portable executable. The user starts `VRCXOptimalTimeApp.exe` directly and does not need to run a separate analyzer or viewer.
- The interface is one persistent window. Controls remain on the left and the graph remains on the right.
- The left control rail scrolls independently from the graph.
- `Your ID` is always visible.
- `Friend IDs` is the only collapsible section.
- `Options` is always visible.
- There is no `Apply` button.
- Valid changes trigger automatic recalculation after a short debounce.
- The application starts with semantic version `0.1.0` and follows Semantic Versioning for all future releases.
- User settings are persisted automatically. The user does not edit a configuration file manually.

## 3. Scope

### In scope for V1

- Automatic discovery of `%APPDATA%\\VRCX\\VRCX.sqlite3`.
- A manually selectable database path when automatic discovery fails.
- Required `Your ID` validation.
- Optional friend ID filtering.
- Multi-line friend ID paste and removal of individual selected IDs.
- The existing analyzer options represented in the settings UI:
  - VRCX running detection threshold.
  - Bucket duration.
  - Normalization.
  - Optional start time.
  - Minimum bucket activations.
  - No-data display behavior.
- Read-only SQLite analysis on a background thread.
- Seven weekday data series with time buckets on the horizontal axis.
- Graph legend controls for showing or hiding weekdays.
- Automatic recalculation when valid settings change.
- Clear loading, success, empty-data, and error states.
- Remembering settings, window size and position, and the friend-list collapsed state.
- Portable Windows release packaging as `VRCXOptimalTimeApp.exe`.
- Third-party license and attribution notices.

### Out of scope for V1

- Editing or repairing VRCX data.
- Writing data back into VRCX.
- VRChat API login, friend synchronization, or network requests.
- Importing the viewer's existing CSV workflow as a required step.
- A browser version or macOS/Linux release.
- User accounts, cloud synchronization, or shared configuration.
- Automatic background polling while the graph is idle. V1 refreshes in response to launch, explicit database refresh, or valid control changes.
- Copying source code from the viewer repository, which currently has no declared license.

## 4. Primary User Workflow

1. The user starts `VRCXOptimalTimeApp.exe`.
2. The application loads persisted settings and attempts to locate the VRCX database.
3. The user enters or confirms `Your ID`.
4. The user optionally pastes friend IDs, one per line.
5. The user changes options as needed.
6. Each valid change schedules a recalculation after a short debounce. The graph remains visible while the new calculation runs.
7. The user reads the weekly graph and can toggle weekday series through the legend.
8. The user may use the database refresh command to reread the current VRCX file without changing settings.

## 5. Interface Design

### Window structure

The main window is a two-pane layout:

- Left: fixed-width control rail with independent vertical scrolling.
- Right: flexible graph and status area.

The graph pane must not move when the left rail is scrolled or when the friend list expands and collapses.

### Your ID section

`Your ID` is always visible at the top of the left rail. It accepts one VRChat user ID and validates the expected `usr_` identifier shape before scheduling analysis. Empty or invalid input prevents analysis and displays an inline validation message.

### Friend IDs section

`Friend IDs` is the only collapsible section. It accepts multiple IDs pasted one per line, validates each entry, removes duplicates, and shows the selected IDs as removable list items. An empty friend filter means all friends are included, matching the analyzer's current behavior.

The expanded or collapsed state is persisted.

### Options section

`Options` remains visible while the left rail scrolls. Controls use appropriate input types for booleans, numeric ranges, dates, and enumerated choices. Every control has a visible current value and validation feedback when necessary.

### Graph section

The graph displays the seven weekday series produced by the analyzer. The horizontal axis is local time and the vertical values are the analyzer's normalized or unnormalized bucket values. The legend can enable or disable individual weekdays. Tooltips identify the weekday, time bucket, and value.

### Status states

The graph area provides an unobtrusive status line for:

- Loading database.
- Calculating.
- Updated time.
- Database not found.
- Database path invalid or unreadable.
- Invalid `Your ID`.
- Database locked.
- VRCX schema incompatibility.
- No matching friend activity.
- No usable activity history.
- Calculation failure.

When a recalculation fails, the previous successful graph remains visible until a newer successful result replaces it.

## 6. Architecture

The application is a native Rust desktop executable.

### UI layer

Use `eframe`/`egui` for the native Windows window and `egui_plot` for the graph. The UI owns editing state, validation messages, layout state, and display-only graph state.

### Analyzer library

Refactor the existing MIT-licensed `vrcx-optimal-time` implementation into an internal library module. Preserve the calculation semantics and isolate filesystem, SQLite, and console-output concerns behind library interfaces.

The library accepts a typed analysis request and returns typed weekly bucket data plus useful metadata. It must not print tab-delimited output as part of the application path.

### Persistence layer

Persist application settings under the user's local application-data directory. The exact filename is an implementation detail, but the format must be versioned so future changes can migrate or reject old settings safely.

### Threading and refresh coordinator

Database work and calculation run away from the UI thread. The refresh coordinator debounces edits, assigns each calculation a generation number, and applies only the newest completed generation. A stale calculation must not overwrite a newer graph.

### Database access

Open the VRCX database read-only using bundled SQLite. Preserve the upstream busy-timeout behavior and convert SQLite errors into user-readable status messages.

## 7. Data Flow

```text
UI edit or launch
  -> validate settings
  -> debounce refresh request
  -> snapshot typed analysis request
  -> background analyzer task
  -> read-only VRCX SQLite connection
  -> calculate VRCX uptime and friend online buckets
  -> return weekly bucket model
  -> discard stale generation if needed
  -> update graph and status
```

The analyzer continues to use the user's local timezone for bucket labeling, matching the upstream behavior.

## 8. Calculation Compatibility

The implementation must preserve the upstream analyzer's behavior for:

- Ten-minute default buckets.
- Seven-day bucket layout.
- VRCX running detection based on event spacing.
- Online/offline event pairing.
- VRCX uptime clamping.
- Optional friend filtering.
- Optional start-time filtering.
- Minimum bucket activation handling.
- Optional normalization.
- Optional zero for missing data.

Any intentional behavior change must be documented and covered by a regression test.

## 9. Licensing and Attribution

- `vrcx-optimal-time` is MIT-licensed. Reused source must retain its copyright and license notice.
- `VRCX` is MIT-licensed. The application reads its database format but does not need to copy VRCX source code. Any copied schema-related code would require the same attribution treatment.
- `vrcx-optimal-time-viewer` currently declares no repository license. Its source will not be copied. Its public behavior and file format may be used as interoperability reference.
- All additional dependencies must be checked for redistribution compatibility and listed in a generated or maintained third-party notices file.

## 10. Error and Recovery Rules

- Missing default database: show the path that was checked and offer a file picker.
- Invalid custom database path: retain the last valid path and show the error without crashing.
- Missing or invalid `Your ID`: do not start analysis.
- Invalid friend ID lines: keep valid entries, report invalid lines, and do not silently reinterpret them.
- SQLite busy or locked: retry within a bounded timeout, then show a retryable status.
- Unexpected schema or SQL error: preserve the old graph, show a diagnostic summary, and log technical details locally.
- Empty result: show an explicit no-data state rather than a blank graph with no explanation.
- Calculation panic or task failure: convert it to an application error boundary and keep the window usable.

## 11. Verification Strategy

### Analyzer verification

- Unit-test time bucketing, uptime detection, online/offline pairing, filtering, normalization, and missing-data rules.
- Use small fixture SQLite databases covering normal, empty, malformed, locked, and partial-history cases.
- Compare representative results against the existing `vrcx-optimal-time` executable before changing behavior.

### UI and state verification

- Test ID validation and multiline paste parsing.
- Test duplicate removal and friend-list collapse persistence.
- Test option validation and settings migration.
- Test debounce coalescing and stale-generation rejection.
- Test graph projection and weekday visibility state.

### Packaged application verification

- Build the release executable with the exact name `VRCXOptimalTimeApp.exe`.
- Start it from a directory containing no source files or configuration file.
- Confirm it can discover the example or test database through the configured path.
- Confirm the VRCX database remains byte-for-byte unchanged after analysis.
- Run a Windows smoke test against the packaged executable.

## 12. Release and Versioning

Use Semantic Versioning from the first release:

- `0.1.0`: first usable development release.
- Increment the minor component for backward-compatible pre-1.0 features.
- Increment the patch component for compatible fixes.
- Increment the major component for breaking changes after the project reaches stable release policy.

The executable filename remains `VRCXOptimalTimeApp.exe`; the version is carried in Windows file metadata and release notes.

## 13. Source References

- Analyzer: https://github.com/zkxs/vrcx-optimal-time
- Viewer behavior reference: https://github.com/px-byte/vrcx-optimal-time-viewer/
- VRCX database producer: https://github.com/vrcx-team/VRCX
