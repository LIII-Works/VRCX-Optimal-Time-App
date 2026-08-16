# Graph Interactions And Friend Views Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add clear option semantics, visible processing state, navigable graph controls, local-time hover text, and selectable per-friend graphs.

**Architecture:** Extend typed analyzer results with per-friend weekly graphs while retaining the current aggregate graph. Keep selection transient in the app, and pass the selected graph to the existing graph renderer. Use egui_plot's built-in zoom, pan, reset, and coordinate overlay APIs.

**Tech Stack:** Rust 2024, eframe/egui, egui_plot, rusqlite, existing Cargo test and Clippy gates.

---

### Task 1: Analyzer friend projections

**Files:** Modify `src/analyzer/mod.rs`, `src/model.rs`; test `tests/analyzer.rs`.

- [x] Write fixture test with two friend IDs and paired events; assert `AnalysisResult.friend_graphs` has both normalized IDs and different populated buckets.
- [x] Run the focused analyzer test and observe failure because result has no friend projections.
- [x] Add `FriendGraph { friend_id, graph }`, collect online intervals by friend, and build one weekly graph per selected friend while preserving aggregate output.
- [x] Run focused analyzer tests and Clippy.

### Task 2: UI selection and progress

**Files:** Modify `src/app.rs`; test `src/app.rs` unit tests if pure helpers are added.

- [x] Add transient selected-friend state defaulting to `All friends`.
- [x] Render an `All friends` plus friend-ID selector when friend graphs exist; pass selected graph to `graph::render`.
- [x] Show a spinner and explicit `Loading database...`/`Calculating...` status while refresh is pending or running.
- [x] Apply friend paste through one helper so valid IDs remain active even when invalid lines are reported.

### Task 3: Graph controls and hover text

**Files:** Modify `src/graph.rs`; test `tests/graph.rs`.

- [x] Add a pure hover formatter test mapping x bucket index `3` at ten minutes to `00:30`.
- [x] Enable wheel zoom and primary-button pan, disable secondary boxed zoom, and wire a `Reset view` button to `Plot::reset`.
- [x] Use `coordinates_formatter` to display local time and y value on hover.

### Task 4: Numeric option explanations

**Files:** Modify `src/app.rs`.

- [x] Label all numeric controls with units and add concise hover explanations for threshold, bucket width, and minimum activations.
- [x] Run formatting, full tests, and Clippy.

### Task 5: Release verification

**Files:** None beyond generated `dist` artifacts.

- [x] Run `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] Run `pwsh -File packaging/build-windows.ps1` and `pwsh -File packaging/smoke-test.ps1 -FixturePath tests/fixtures/vrcx-fixture.sqlite3`.
- [x] Record the completed GUI and live-VRCX checks, including the supplied IDs, in the README and release checklist.
