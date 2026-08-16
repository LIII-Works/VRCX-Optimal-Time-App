# Release Checklist

- [x] `cargo fmt --check`
- [x] `cargo test`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `pwsh -File packaging\\build-windows.ps1`
- [x] `pwsh -File packaging\\smoke-test.ps1 -FixturePath tests\\fixtures\\vrcx-fixture.sqlite3`
- [x] Verify `dist\\VRCXOptimalTimeApp.exe` has the expected SHA-256 manifest entry.
- [x] Perform a Windows GUI pass covering persisted IDs, friend selection, loading state, graph zoom/reset, and hover coordinates.
- [x] Run a separately authorized live VRCX read-only check with the supplied IDs; fixture results do not prove live compatibility.
