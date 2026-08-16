# Absolute Analysis Time Range Evidence

Date: 2026-08-17
Release target: VRCX Optimal Time App 0.2.0
Approved specification: `docs/superpowers/specs/2026-08-17-vrcx-optimal-time-range-design.md`

## Acceptance Mapping

- Numeric local `YYYY`, `MM`, `DD`, `HH`, and `mm` construction: `numeric_parts_build_a_local_datetime_without_text_parsing`, `numeric_bound_draft_round_trips_local_components`.
- Impossible leap-year and month-length dates, plus invalid hour and minute values: `impossible_calendar_dates_are_rejected`.
- Optional Start and End bounds, including disabling a bound: `disabled_numeric_bound_clears_its_saved_value` and settings defaults.
- Start later than End rejection: `time_range_requires_start_not_later_than_end`, `numeric_bound_commit_rejects_impossible_date_and_reverse_range`.
- Start-inclusive and End-exclusive clipping: `analyzer_clips_running_activity_at_end_time`, `analyzer_clips_both_bounds_and_friend_graphs_to_same_interval`.
- Combined clipping, wholly outside intervals, and zero-length intervals: `analyzer_clips_both_bounds_and_friend_graphs_to_same_interval`, `analyzer_rejects_range_with_no_activity_after_clipping`, `analyzer_rejects_empty_start_and_end_interval`.
- Aggregate and per-friend clipping parity: `analyzer_clips_both_bounds_and_friend_graphs_to_same_interval`.
- Persistence and legacy settings compatibility: `settings_round_trip_through_versioned_toml_and_atomic_replacement`, `version_one_migrates_missing_current_fields_to_defaults`.
- Existing behavior with no bound and existing start-only behavior: `analyzer_returns_weekly_buckets_without_mutating_its_database`, `analyzer_preserves_online_state_across_start_time_boundary`.

## Final Gates

Commands run after the final implementation and version edits:

```text
cargo fmt --check                 exit 0
cargo test                        39 passed, 0 failed; doc-tests 0 failed
cargo clippy --all-targets --all-features -- -D warnings
                                    exit 0
pwsh -File packaging\\build-windows.ps1
                                    exit 0; package version 0.2.0
pwsh -File packaging\\smoke-test.ps1 -FixturePath tests\\fixtures\\vrcx-fixture.sqlite3
                                    self-test passed; exit 0
```

Packaged executable hash:

```text
5204F65B394623DB6EB53B2F5F5CF068A3D0FFAA6CBA6BC0C92BE321B8F53FE7  VRCXOptimalTimeApp.exe
```

Installed executable verification:

```text
Path: C:\\Program Files\\VRCX Optimal Time App\\VRCXOptimalTimeApp.exe
Version: 0.2.0
Hash: 5204F65B394623DB6EB53B2F5F5CF068A3D0FFAA6CBA6BC0C92BE321B8F53FE7
Self-test: passed; all_event_count=6, online_offline_event_count=2,
            weekday_count=7, bucket_count=144, populated_buckets=2,
            value_sum=2.000000
```

## Adversarial Checks

Four temporary manual mutants were applied one at a time and restored after
each run. All four were killed by focused tests:

1. Reversed the Start/End comparison. `cargo test --lib --test validation` failed in `numeric_bound_commit_rejects_impossible_date_and_reverse_range`.
2. Changed End clipping from `min` to `max`. `cargo test --test analyzer` failed four range-clipping tests.
3. Changed Start clipping from `max` to `min`. `cargo test --test analyzer` failed three range-clipping tests.
4. Changed disabled-bound handling to return a value. The disabled-bound unit test failed with the unexpected saved value.

Unavailable optional layers:

- Coverage: `cargo-llvm-cov` is not installed.
- Mutation tool: `cargo-mutants` is not installed; manual mutation was used instead.
- Property-based testing: no property-test runner is present in this repository; deterministic boundary and round-trip tests cover the approved numeric invariants.
- Randomized test ordering: no `cargo-nextest` or randomized test runner is installed.

No new dependency was added. The executable was run with realistic SQLite input
through both the packaged and installed self-test paths. GUI interaction was
verified by launching the packaged window earlier; automated desktop interaction
was not used for this release gate.

## Cleanup

The generated `dist/settings.toml` test artifact and temporary rollback snapshot
were removed after installed verification. No rollback copy remains.
