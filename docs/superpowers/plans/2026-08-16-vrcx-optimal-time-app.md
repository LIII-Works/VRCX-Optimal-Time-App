# VRCX Optimal Time App Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Build a portable Windows desktop application named `VRCXOptimalTimeApp.exe` that reads VRCX SQLite data read-only, calculates friend availability, and displays the weekly graph in one native window.

**Architecture:** A single native Rust executable uses `eframe`/`egui` for the UI and `egui_plot` for the graph. The MIT-licensed `vrcx-optimal-time` calculation code is refactored into an internal analyzer library that returns typed data directly; no CSV file, browser runtime, sidecar analyzer, or installer is required.

**Tech Stack:** Rust 2024 edition, `eframe`, `egui_plot`, `rusqlite` with bundled SQLite, `chrono`, `serde`, `toml`, `thiserror`, `rfd`, Windows MSVC target, Cargo tests, and PowerShell packaging scripts.

---

## Scope and Repository Rules

- Work in `C:\Users\LIII Works\Personal Files\Project\VRCX Optimal Time App`.
- Preserve the existing untracked `Example\` directory. Do not move, overwrite, or stage it unless a later task explicitly adds a fixture beside it.
- The approved design is `docs/superpowers/specs/2026-08-16-vrcx-optimal-time-app-design.md`.
- Configure a repository-local Git author using the project's chosen name and email before the first commit. The current repository is initialized but has no author identity yet.

- Pin the reused analyzer source to a reviewed upstream commit and retain its MIT header and license text in `THIRD_PARTY_NOTICES.txt`.
- Do not copy source from `px-byte/vrcx-optimal-time-viewer`; its repository currently has no declared license. Recreate the behavior with the native graph.

## File Map

```text
Cargo.toml, Cargo.lock, build.rs, LICENSE, THIRD_PARTY_NOTICES.txt, README.md
src/main.rs             native entry point
src/app.rs              egui app state and event routing
src/model.rs            typed settings, requests, results, and status
src/errors.rs           internal errors and user-facing text
src/settings.rs         persistence, migration, and atomic writes
src/validation.rs       ID and option validation
src/refresh.rs          debounce and generation coordination
src/database.rs         path discovery and connection setup
src/analyzer/mod.rs     public typed analyzer API
src/analyzer/sqlite.rs  read-only VRCX queries
src/analyzer/uptime.rs  VRCX running interval reconstruction
src/analyzer/buckets.rs bucket accumulation and normalization
src/ui/mod.rs           main two-pane layout
src/ui/controls.rs      Your ID, Friend IDs, Options, and status controls
src/ui/graph.rs         seven weekday line series
tests/support/mod.rs    temporary SQLite fixture builder
tests/validation.rs, tests/settings.rs, tests/analyzer.rs
tests/refresh.rs, tests/graph.rs, tests/fixtures/expected.json
packaging/build-windows.ps1, packaging/smoke-test.ps1
docs/README.md, docs/RELEASE-CHECKLIST.md
```

## Task 1: Establish the Rust Application Shell

**Files:** Create `Cargo.toml`, `build.rs`, `src/main.rs`, `src/app.rs`, `src/model.rs`, `src/errors.rs`.

- [ ] Create one binary crate named `vrcx-optimal-time-app`, version `0.1.0`, edition `2024`. Use `chrono`, `eframe`, `egui_plot`, `num-traits`, `rfd`, `rusqlite` with `bundled`, `serde`, `thiserror`, and `toml`; use `winresource` as a build dependency. Set release `lto = "thin"`, `codegen-units = 1`, and `strip = true`.
- [ ] Make `main.rs` call `eframe::run_native`, set title `VRCX Optimal Time`, initial size `1280x720`, and construct `VrcxOptimalTimeApp::default()`.
- [ ] Define stable model boundaries: `AppSettings`, `AnalysisSettings`, `WindowSettings`, `AnalysisRequest`, `WeeklyGraph`, and `AppStatus`. Keep persisted state separate from transient UI state.
- [ ] Add a compile-only test and run `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets --all-features -- -D warnings`; all must pass.
- [ ] Commit `build: scaffold native VRCX optimal time app`.

## Task 2: Implement Validation and Typed Settings

**Files:** Create `src/validation.rs`, `src/settings.rs`; modify `src/model.rs`, `src/errors.rs`; test `tests/validation.rs`, `tests/settings.rs`.

- [ ] Write failing tests for valid UUID-shaped `usr_` IDs, empty IDs, malformed IDs, multiline friend-ID paste, duplicate removal, invalid-line reporting, and bucket durations that do or do not evenly divide 24 hours.
- [ ] Implement strict ID parsing: accept `usr_` plus UUID, normalize to lowercase, preserve first-seen order, remove duplicates, and report invalid input line numbers.
- [ ] Implement `schema_version: u32` TOML persistence at `%LOCALAPPDATA%\\VRCX Optimal Time App\\settings.toml`; write to a temporary file and replace atomically. Missing files load defaults; malformed files produce readable errors; unsupported future versions are rejected; version 1 migrates by filling new fields with defaults.
- [ ] Test defaults, round trips, atomic replacement, malformed TOML, future-version rejection, and migration.
- [ ] Run `cargo test --test validation --test settings` and clippy, then commit `feat: add validated persisted settings`.

## Task 3: Port the Analyzer as a Typed Library

**Files:** Create `src/analyzer/mod.rs`, `src/analyzer/sqlite.rs`, `src/analyzer/uptime.rs`, `src/analyzer/buckets.rs`, `src/analyzer/upstream_license.txt`, `tests/support/mod.rs`, `tests/analyzer.rs`, `tests/fixtures/expected.json`, `LICENSE`, `THIRD_PARTY_NOTICES.txt`; modify `src/model.rs`, `src/errors.rs`.

- [ ] Record the reviewed upstream `vrcx-optimal-time` commit SHA and MIT text. Keep upstream copyright headers on reused functions. Do not copy viewer source.
- [ ] Define `analyze(request: AnalysisRequest) -> Result<AnalysisResult, AnalyzerError>`; the request contains `your_user_id`, optional friend IDs, database path, uptime threshold, bucket duration, normalization, start time, minimum activations, and missing-data behavior. The result contains seven typed bucket series and diagnostic metadata.
- [ ] Write failing SQLite fixture tests for normal online/offline pairing, VRCX uptime gaps, friend filtering, unmatched offline events, normalization, minimum activations, and missing-data output.
- [ ] Refactor the upstream event queries, five-second busy timeout, uptime reconstruction, online/offline pairing, clamping, bucket registration, and local-time conversion without changing semantics. Replace external-boundary `unwrap`/`panic` paths with typed errors and return data instead of TSV.
- [ ] Hash each fixture before and after analysis and assert that the read-only connection leaves the file unchanged. Compare deterministic results to `tests/fixtures/expected.json`.
- [ ] Run `cargo test --test analyzer`, fmt, and clippy, then commit `feat: integrate read-only VRCX analyzer`.

## Task 4: Add Database Discovery and Error Translation

**Files:** Create `src/database.rs`; modify `src/settings.rs`, `src/errors.rs`, `src/model.rs`, `src/analyzer/sqlite.rs`; test `tests/settings.rs`, `tests/analyzer.rs`.

- [ ] Test default `%APPDATA%\\VRCX\\VRCX.sqlite3` discovery, custom paths, missing files, unreadable files, a busy database that succeeds within five seconds, and a busy database that reaches the timeout.
- [ ] Resolve `APPDATA`, append `VRCX\\VRCX.sqlite3`, expose the checked path in errors, and make a successful custom path persistent.
- [ ] Use `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI | SQLITE_OPEN_NO_MUTEX`, set a five-second busy timeout, and map failures to `DatabaseNotFound`, `DatabaseLocked`, `SchemaMismatch`, or `DatabaseReadFailed`.
- [ ] Hash the database before and after analysis and assert equal hashes. No application connection may be writable.
- [ ] Run `cargo test --test analyzer --test settings` and clippy, then commit `feat: discover VRCX database safely`.

## Task 5: Build the Single-Window Control Layout

**Files:** Create `src/ui/mod.rs`, `src/ui/controls.rs`; modify `src/app.rs`, `src/model.rs`; test `tests/validation.rs`.

- [ ] Keep persisted settings separate from transient status, invalid-line messages, weekday visibility, refresh generation, and last graph.
- [ ] Render an `egui::SidePanel` with fixed minimum width and `ScrollArea::vertical`; render the graph in the remaining central panel so scrolling the rail never scrolls the graph.
- [ ] Render `Your ID` as an always-visible field with inline validation. Valid edits update settings and schedule refresh.
- [ ] Render `Friend IDs` as the only `egui::CollapsingHeader`; support multiline paste, valid selected IDs, invalid-line messages, removal, and persisted collapsed state.
- [ ] Render `Options` always visible with uptime threshold, bucket duration, normalize, optional start time, minimum activations, and no-data behavior. Reject invalid values.
- [ ] Add a header `Refresh database` command that rereads the database without acting as an Apply step.
- [ ] Run fmt, tests, and clippy, then commit `feat: add live two-pane control layout`.

## Task 6: Implement Graph Projection and Rendering

**Files:** Create `src/ui/graph.rs`; modify `src/model.rs`, `src/ui/mod.rs`, `src/app.rs`; test `tests/graph.rs`.

- [ ] Write tests asserting that a 10-minute result projects to 144 time labels, seven Monday-through-Sunday series, and preserved missing-data markers.
- [ ] Convert analyzer buckets to `egui_plot::PlotPoints`; keep missing values as gaps unless `no_data_returns_zero` is enabled.
- [ ] Render seven named weekday lines, local-time labels, numeric values, hover coordinates, and weekday visibility toggles. Keep the previous graph visible during loading.
- [ ] Run `cargo test --test graph` and clippy, then commit `feat: render weekly availability graph`.

## Task 7: Add Debounced Background Refresh

**Files:** Create `src/refresh.rs`; modify `src/app.rs`, `src/model.rs`, `src/errors.rs`; test `tests/refresh.rs`.

- [ ] Write tests for one launch request, three edits inside 300 ms coalescing to one request, a later edit creating a second request, generation-1 results being discarded after generation 2 starts, and failures preserving the last graph.
- [ ] Implement a 300 ms debounce, monotonically increasing `u64` generation, background worker channel, and `RefreshReason::{Launch, ControlChanged, DatabaseRefresh}`. Snapshot settings before dispatch.
- [ ] Set status to `Calculating`, `Updated { at }`, or a typed error without clearing the last successful graph.
- [ ] Close the worker channel on shutdown and prevent late results from touching destroyed UI state.
- [ ] Run `cargo test --test refresh`, full tests, and clippy, then commit `feat: refresh analysis automatically`.

## Task 8: Complete Persistence, Window State, and Diagnostics

**Files:** Create `src/diagnostics.rs`; modify `src/settings.rs`, `src/app.rs`, `src/errors.rs`; test `tests/settings.rs`.

- [ ] Persist `Your ID`, friend IDs, database path, all analysis options, friend-list collapsed state, window size, and window position. Never persist invalid IDs as active settings.
- [ ] Write technical errors and calculation metadata to `%LOCALAPPDATA%\\VRCX Optimal Time App\\logs\\app.log`; do not log the VRCX event stream or friend activity records.
- [ ] Test restart persistence, recovery from partially invalid settings, log-directory creation, and unchanged graph state after a failed refresh.
- [ ] Run focused tests and clippy, then commit `feat: persist app state and diagnostics`.

## Task 9: Add Version Metadata and Windows Packaging

**Files:** Modify `build.rs`, `Cargo.toml`, `README.md`; create `packaging/build-windows.ps1`, `packaging/smoke-test.ps1`, `THIRD_PARTY_NOTICES.txt`.

- [ ] Use `winresource` to set Windows file description and product name to `VRCX Optimal Time App`, original filename to `VRCXOptimalTimeApp.exe`, and file version from `CARGO_PKG_VERSION`.
- [ ] Make `build-windows.ps1` require the `x86_64-pc-windows-msvc` target, run fmt/tests/clippy, build release, copy the binary to `dist\\VRCXOptimalTimeApp.exe`, copy notices and README, write `dist\\SHA256SUMS.txt`, and print version/output paths.
- [ ] Add hidden `--self-test <sqlite-path>` mode. It runs deterministic analysis against the fixture and exits `0` only when the result matches `tests/fixtures/expected.json`; normal launch remains GUI mode.
- [ ] Make `smoke-test.ps1` launch the packaged self-test, fail on nonzero exit, assert the exact executable name, and assert a SHA-256 entry.
- [ ] Document purpose, one-EXE launch, database discovery, read-only behavior, settings location, error statuses, build/test commands, SemVer policy, and license notices in `README.md`.
- [ ] Run `pwsh -File packaging\\build-windows.ps1` and `pwsh -File packaging\\smoke-test.ps1 -FixturePath tests\\fixtures\\vrcx-fixture.sqlite3`; expected output is `dist\\VRCXOptimalTimeApp.exe` with a passing self-test and no adjacent analyzer/config file.
- [ ] Commit `build: package portable Windows executable`.

## Task 10: Release Gate and Handoff

**Files:** Create `docs/README.md`, `docs/RELEASE-CHECKLIST.md`; modify `README.md`.

- [ ] Run the complete gate:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
pwsh -File packaging\\build-windows.ps1
pwsh -File packaging\\smoke-test.ps1 -FixturePath tests\\fixtures\\vrcx-fixture.sqlite3
```

- [ ] Perform a clean-profile Windows acceptance pass: start without config; discover or override the VRCX path; block missing `Your ID`; paste/deduplicate/remove friend IDs; confirm only Friend IDs collapses; scroll the rail without moving the graph; confirm automatic refresh with no Apply; preserve the old graph during slow/error refresh; verify readable locked/schema errors; verify database hash unchanged; verify restart persistence.
- [ ] Record live-database checks separately from fixture evidence and do not claim live compatibility without running them against the user's actual VRCX copy.
- [ ] After the Git author identity is configured and all checks pass, review staged paths, commit `release: VRCX Optimal Time App 0.1.0`, and tag `v0.1.0`.

## Completion Criteria

The roadmap is complete when one `VRCXOptimalTimeApp.exe` builds for Windows; analyzer behavior is covered by deterministic SQLite fixtures and parity assertions; the UI matches the approved two-pane design; automatic refresh rejects stale results; VRCX remains read-only with unchanged hash; settings, diagnostics, version metadata, notices, packaging, and smoke tests exist; and `0.1.0` verification passes with live-environment limitations recorded.
