# Development and release notes

## Modules

- `src-tauri/src/main.rs`: native windows, events, commands, and monitor placement.
- `model.rs`: settings, session state, undo/redo, and replay history.
- `geometry.rs`: hit testing; `zoom.rs`: frozen screen capture for zoom.
- `capture.rs` and `animation.rs`: PNG and GIF export.
- `ui/app.js`: toolbar/settings orchestration. Smaller modules handle drawing, selection, replay, shortcuts, boards, and zoom.
- `tests/`: Node built-in tests. Rust unit tests are colocated with modules.

Coordinates and stroke widths are stored in original-screen space. Convert pointer input and drawing output at the zoom boundary; avoid applying zoom or DPI twice. Overlay click-through is enabled in mouse mode. Toolbar/settings windows are owned by the overlay.

## Rebrand compatibility

LayerPen 0.1.0 changes the displayed product name and organizes documentation. The Rust package/binary `monitor-ink`, identifier `dev.personal.monitorink`, capture folder `MonitorInk`, and `MONITOR_INK_DATA_DIR` remain for compatibility. Do not rename data paths without a migration plan. An old Monitor Ink installer entry may coexist with a newly named LayerPen entry.

## Release checks

Run the documented unit tests, build the Windows installer, launch an isolated profile, and verify installation/removal. Inspect screen-sharing behavior from a second participant before claiming compatibility. Test new OS support before marking it supported. Dependency notices include exact-revision supplements; package third-party-sources.zip alongside every binary distribution.

Create a source archive with `python scripts/package-source.py /path/to/output.zip`. It excludes generated files, installed dependencies, binary outputs, and Git internals. Keep release archives outside this repository. Internal pre-release research and historical captures are excluded from the public repository.

See [localization](LOCALIZATION.md) for the four UI languages. Public versions start at 0.1.0 independently of the earlier local prototype sequence.
