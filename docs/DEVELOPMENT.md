# Development and release notes

## Modules

- `src-tauri/src/main.rs`: native windows, events, commands, and monitor placement.
- `model.rs`: settings, session state, undo/redo, and replay history.
- `profile.rs`: profile overrides, legacy settings and imported-font migration.
- `geometry.rs`: hit testing; `zoom.rs`: frozen screen capture for zoom.
- `capture.rs` and `animation.rs`: PNG and GIF export.
- `ui/app.js`: toolbar/settings orchestration. Smaller modules handle drawing, selection, replay, shortcuts, boards, and zoom.
- `tests/`: Node built-in tests. Rust unit tests are colocated with modules.

Coordinates and stroke widths are stored in original-screen space. Convert pointer input and drawing output at the zoom boundary; avoid applying zoom or DPI twice. Overlay click-through is enabled in mouse mode. Toolbar/settings windows are owned by the overlay.

## Rebrand compatibility

Pointory 0.1.0 uses the Rust package/binary `pointory` and the internal identifier `dev.personal.monitorink`. Keeping that identifier preserves upgrade compatibility. Captures default to `Pictures/Pointory`; legacy default folders are migrated in preferences without moving existing files. Custom capture folders remain unchanged. The Windows NSIS template retains the existing OnPen registry identity and uses Pointory for visible names; the full upgrade/install/uninstall path still requires native validation. Older installer entries from other product names may remain.

## Release checks

Run the documented unit tests, build the Windows installer, launch an isolated profile, and verify installation/removal. Inspect screen-sharing behavior from a second participant before claiming compatibility. Test new OS support before marking it supported. Dependency notices include exact-revision supplements; package third-party-sources.zip alongside every binary distribution.

Create a source archive with `python scripts/package-source.py /path/to/output.zip`. It excludes generated files, installed dependencies, binary outputs, and Git internals. Keep release archives outside this repository. Internal pre-release research and historical captures are excluded from the public repository.

See [localization](LOCALIZATION.md) for the four UI languages. Public versions start at 0.1.0 independently of the earlier local prototype sequence.

Settings use the OS configuration directory / Pointory / settings.json. On first launch, settings and imported fonts are migrated from OnPen or earlier identifier-based locations. Imported fonts live in fonts/ beside settings. Capture defaults recognize OnPen, LayerPen, InkLatch, Inklach, MonitorInk, and Monitor Ink under Pictures.

Use `POINTORY_DATA_DIR` for an isolated profile. `POINTORY_STT_PYTHON` selects a caption interpreter and `POINTORY_STT_WORKER` overrides the bundled worker. `ONPEN_DATA_DIR` and `MONITOR_INK_DATA_DIR` remain supported for profiles; the corresponding `ONPEN_STT_` and `LAYERPEN_STT_` names remain supported for captions. The Windows caption setup creates `%LOCALAPPDATA%/Pointory/stt-venv`; existing supported runtimes remain available through the native discovery fallback.

OpenVINO model downloads default to `~/.cache/pointory`; complete legacy downloads under `~/.cache/onpen` or `~/.cache/layerpen` are reused in place. `POINTORY_MODEL_DIR` overrides that root, with `ONPEN_MODEL_DIR` and `LAYERPEN_MODEL_DIR` aliases. Compiled OpenVINO caches use `ov-compiled-cache` under the selected root. Set `POINTORY_STT_DIAGNOSTICS=1` only when investigating a worker hang; previous `ONPEN_`/`LAYERPEN_` diagnostic flags remain accepted.
