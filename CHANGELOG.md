# Changelog

## Unreleased

### Added

- Core damage systems and prototype user interactions

- Additional builds for varying CPU-supported features (sse2, sse4.2, avx2, avx512)

- Visualize cursor 3D position in world (spatial hash grid)

- Create interface panel to interact with internal profiler

### Changed

- Updated `build.sh` build script to support additional builds

- Profiler can now be enabled/disabled, rather than be running at all times

### Fixed

- Crash during debris simulation due to missing check(s)

- Restore debug lines rendering under `devmode` feature flag

- GUI: correctly selecting only the top-most floating panel when grabbing a panel that overlays another floating panel, avoiding conflict

## 0.1.0 - 2026/10/29
Base pre-changelog state
