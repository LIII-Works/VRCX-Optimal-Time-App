# Graph Interactions And Friend Views

Status: approved in chat on 2026-08-16.

## Goal

Make numeric options self-explanatory, expose analysis progress, make the graph navigable, and let users inspect one selected friend at a time without losing the aggregate view.

## Behavior

- Running threshold, bucket duration, and minimum activations show units and concise explanations.
- `Calculating` shows a visible spinner while a refresh is pending or running; the previous graph remains visible.
- Mouse-wheel zoom and primary-button pan are enabled. Secondary-button boxed zoom is disabled. A `Reset view` command restores automatic bounds.
- Hover coordinates show local time and value, not only a bucket index.
- The graph view offers `All friends` plus each selected friend. `All friends` preserves current aggregate behavior; a friend view contains only that friend's online/offline intervals.

## Architecture

The analyzer keeps the existing aggregate graph and additionally returns typed per-friend graphs keyed by normalized friend ID. The app owns a selected graph key and projects that graph through the existing renderer. Friend selection is transient UI state; selected IDs and analysis settings remain persisted as before.

## Verification

Add analyzer fixture coverage for separate friend counts and graph selection/projection coverage for aggregate versus individual data. Add structural graph tests for time hover formatting and preserve existing read-only, refresh, packaging, and full-suite gates. Live GUI acceptance is tracked separately in the README and release checklist.
