# Absolute Analysis Time Range Design

Status: Approved for implementation planning
Approved: 2026-08-17
Target: VRCX Optimal Time App

## Goal

Allow users to limit analysis to an absolute local date-time interval with
year, month, day, hour, and minute inputs. The existing weekly availability
graph remains the output format.

## Product Decisions

- The range is an absolute interval, not a recurring month/day/hour filter.
- The user enters numeric components only; no RFC3339 text and no date-picker
  dependency are exposed.
- Start and End are independently optional. No bounds preserves current
  behavior; one bound creates a one-sided interval.
- Values use the computer's local timezone, matching the analyzer's existing
  graph bucketing behavior.
- The interval is start-inclusive and end-exclusive internally.
- Both bounds are stored as typed local `DateTime` values in settings. The UI
  owns the component editing state.

## UI Design

The Options section replaces the current free-form RFC3339 start-time field
with two compact numeric groups:

- `Start date/time`: enable checkbox plus `YYYY`, `MM`, `DD`, `HH`, and `mm`.
- `End date/time`: enable checkbox plus `YYYY`, `MM`, `DD`, `HH`, and `mm`.

Each component uses a bounded numeric control:

- Year: 1 through 9,999, subject to the supported local `chrono` range.
- Month: 1 through 12.
- Day: 1 through 31 before full-date validation.
- Hour: 0 through 23.
- Minute: 0 through 59.

The complete date is validated after component edits. Impossible calendar
dates, including February 30 and April 31, are rejected. Invalid clock values
are rejected as well. The local timezone conversion must produce one
unambiguous instant; nonexistent or daylight-saving-overlap local minutes are
reported as invalid rather than guessed. A partially entered or invalid bound
keeps the last valid typed setting, reports an inline error, and does not
schedule a refresh.

When both bounds are enabled, the UI rejects `Start > End` and reports the
range error without replacing the last valid analysis request. A valid change
uses the existing debounce and automatic refresh behavior.

## Data Model And Persistence

Add `end_time: Option<DateTime<Local>>` to `AnalysisSettings` and
`AnalysisRequest`, and copy it through the existing request snapshot path.
Serde defaults the new field to `None`, so schema version 1 settings files
without an end bound remain readable without migration. Saving includes the
new field when present.

The app keeps separate component state for the two UI groups so users can edit
individual numbers without exposing a serialized text format. Loading settings
initializes those components from the stored typed values.

## Analyzer Behavior

Replace the start-only interval clipping helper with range clipping:

1. If an interval ends at or before the selected start, discard it.
2. If an interval starts at or after the selected exclusive end, discard it.
3. Clamp the remaining interval start to the selected start, when present.
4. Clamp the remaining interval end to the selected end, when present.
5. Discard any empty result where the clamped start is not before the clamped
   end.

The clipping occurs before aggregate and per-friend bucket registration, so
both graph views observe the same range. With no end bound, behavior remains
identical to the current start-only implementation. The graph continues to
show seven weekday series and local time-of-day buckets; the selected absolute
range changes which historical intervals contribute to those buckets.

## Verification

Add focused tests for:

- default settings with no bounds;
- round-trip persistence of both bounds;
- loading an older settings file without `end_time`;
- valid component construction;
- impossible dates, including leap-year and month-length cases;
- invalid hour and minute values;
- rejecting a start later than the end;
- end-only clipping;
- simultaneous start/end clipping;
- intervals wholly outside the range;
- preserving existing start-only behavior;
- aggregate and per-friend graphs receiving the same clipped intervals.

Run the existing formatting, test, lint, packaging, and smoke-test gates after
implementation. Live GUI acceptance should confirm that numeric fields are
usable, validation is visible, and changing a valid bound refreshes the graph.

## Scope Boundary

This change does not add recurring filters, timezone selection, calendar
pickers, free-form date parsing, new graph dimensions, or changes to VRCX
database access.
