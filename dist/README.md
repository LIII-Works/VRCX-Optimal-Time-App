# VRCX Optimal Time App

This project uses and references code and behavior from these upstream projects:

- [zkxs/vrcx-optimal-time](https://github.com/zkxs/vrcx-optimal-time) for the VRCX activity analysis behavior.
- [px-byte/vrcx-optimal-time-viewer](https://px-byte.github.io/vrcx-optimal-time-viewer/) for the graph and viewer behavior.

See [`THIRD_PARTY_NOTICES.txt`](THIRD_PARTY_NOTICES.txt) and [`src/analyzer/upstream_license.txt`](src/analyzer/upstream_license.txt) for attribution and license details.

Portable Windows desktop application for calculating and viewing weekly VRChat friend availability from a VRCX SQLite database.

> **Important prerequisite:** VRCX must stay installed and running long enough to capture a useful amount of online, offline, and activity history. This app can only graph the history that VRCX has already collected; a new or rarely used VRCX database will not have enough data for a meaningful graph.

The application reads VRCX data through a read-only SQLite connection. It does not write to VRCX, use the VRChat API, or require a browser, analyzer sidecar, or separate configuration program.

## Current build

The native executable window is `VRCX Optimal Time App`. It provides the two-pane controls and graph surface, typed validation and settings persistence libraries, read-only analyzer, graph projection, and debounced background refresh coordinator.

Fixture evidence covers the analyzer and packaging contract. The packaged executable has also been checked against the local VRCX database and GUI controls; results still depend on the history captured in each user's VRCX database.

The executable also supports `--self-test <sqlite-path>` for deterministic fixture validation.

## How to use

1. Keep VRCX installed and running for long enough to capture enough online, offline, and activity history for a useful graph. The app cannot create history that VRCX has not collected yet.
2. Enter your VRChat user ID in the `Your ID` box.
3. Enter friend IDs in `Friend IDs`, one ID per line.
4. The analysis starts automatically after valid input changes. If the default VRCX database is not detected, use `Choose database...` in the warning or the `Database` control to select the SQLite file manually.

## Data and settings

The default database path is `%APPDATA%\\VRCX\\VRCX.sqlite3`. A custom path can be supplied through the persisted settings model. Settings are stored at `%LOCALAPPDATA%\\VRCX Optimal Time App\\settings.toml` using a versioned TOML schema.

Invalid user IDs and invalid friend-ID lines block a refresh. A missing, locked, or schema-incompatible database is reported as a typed status while the last successful graph remains available.

## Build and test

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
pwsh -File packaging\\build-windows.ps1
pwsh -File packaging\\smoke-test.ps1 -FixturePath tests\\fixtures\\vrcx-fixture.sqlite3
```

The packaging script copies the release binary to `dist\\VRCXOptimalTimeApp.exe` and writes `SHA256SUMS.txt` plus license and notice files. Technical errors are appended to `%LOCALAPPDATA%\\VRCX Optimal Time App\\logs\\app.log`; event records and friend activity data are never logged.

## License

This application is MIT licensed. Analyzer behavior derived from `zkxs/vrcx-optimal-time` is attributed in `THIRD_PARTY_NOTICES.txt` and `src/analyzer/upstream_license.txt`.
