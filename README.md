# OnPen

**Draw over your screen. Keep the conversation moving.**

OnPen is a free, open-source desktop annotation overlay for screen sharing, live demos, and teaching. Pick one monitor, draw over the apps you already use, then switch back to clicking without losing your annotations.

Built with **Tauri 2, Rust, and plain JavaScript**. Previously named LayerPen / Monitor Ink. The repository URL and internal app identifier stay unchanged for compatibility.

[日本語](docs/README.ja.md) · [简体中文](docs/README.zh-CN.md) · [한국어 사용 안내](docs/USER-GUIDE.ko.md) · [Getting started](docs/GETTING-STARTED.md) · [Changelog](docs/CHANGELOG.md) · [Contributing](CONTRIBUTING.md) · [MIT license](LICENSE)

## v0.1 refresh

OnPen adds selectable app themes, click-to-type text with configurable fonts, six toolbar layouts (horizontal/vertical × 1/2/3 lines), installed font selection and TTF import, and a settings panel that docks beside it. Experimental live captions support microphone/device selection and optional local acceleration. See [STT setup](docs/STT-SETUP.md); Python dependencies and model weights are downloaded separately. Cloud provider integration has not been verified with real API credentials. macOS validation and Apple Silicon acceleration are planned for v0.2.

## What you can do

- Annotate **one selected monitor** with a pen, highlighter, lines, rectangles, and ellipses.
- Erase entire strokes, move and resize annotations, and undo or redo edits.
- Hide the ink layer or switch to mouse mode to interact with the underlying app.
- Use a compact horizontal or vertical toolbar, five configurable colors plus a custom picker, and five thickness presets.
- Customize tool shortcuts and enable or disable them individually.
- Magnify a frozen region and draw at the correct scale. Annotations stay in their original screen coordinates when you return.
- Use a whiteboard, blackboard, or fading ink for temporary emphasis.
- Export a PNG of the screen with ink, a transparent ink layer, or a GIF replay of your annotation sequence.

Drawing and local captions need no service account. Optional cloud captions require the chosen provider’s API key and may incur charges. The current application has no analytics integration. Windows WebView2 and installer downloads are separate runtime components.

## Status and supported platforms

| Platform | Status |
| --- | --- |
| Windows x64 | Beta; locally built and exercised with the native app and automated tests |
| macOS | Development target; runtime and packaging have not been verified |
| Linux X11 | Development target; runtime and packaging have not been verified |
| Linux Wayland | Not currently supported as a release target |

The UI supports **English, 한국어, 日本語, and 简体中文**. Choose a language in Settings or follow the system language. Unsupported system languages fall back to English. See [localization](docs/LOCALIZATION.md). macOS and Linux CI checks are experimental and do not establish runtime support.

## Get started

Watch the updated OnPen [36-second product introduction](https://github.com/BAEM1N/layerpen/releases/download/v0.1.0/OnPen-intro-v0.1.0.mp4). Its editable [Remotion source](video/README.md) and [MacBook handoff guide](docs/HANDOFF-MACBOOK.ko.md) are included.

Download the [Windows installer](https://github.com/BAEM1N/layerpen/releases/download/v0.1.0/OnPen_0.1.0_x64-setup.exe) or [portable ZIP](https://github.com/BAEM1N/layerpen/releases/download/v0.1.0/OnPen-0.1.0-windows-x64-portable.zip) from [GitHub Releases](https://github.com/BAEM1N/layerpen/releases). The portable edition contains `OnPen.exe`; no build tools are needed. Close older copies before launching.

This is **v0.1.0, the first public release**. Earlier local development builds used a separate version sequence.

1. Open Settings (the gear icon) and select the monitor you want to annotate.
2. Choose the pen and draw over that screen.
3. Select the cursor icon to interact with the underlying app, or press **Ctrl+Shift+D** to toggle drawing/mouse mode.
4. Export anything you want to keep before closing the app.

For an online meeting, start with **sharing the entire selected monitor**, then confirm that another participant can see the ink. Sharing only an application window or browser tab may omit the overlay. Zoom, Teams, and Meet compatibility has **not yet been verified end to end**. OnPen does not control screen sharing and is not a remote collaborative whiteboard.

## Important behavior

- Ink is attached to screen coordinates. It does not follow document text when you scroll or switch slides.
- Zoom uses a frozen screenshot, not a live video magnifier.
- GIF export replays annotations over a static background; it does not record video, audio, or zoom-camera movement.
- Strokes and replay history live in memory and are lost on exit. Settings are saved locally.
- Clear All resets the selected monitor's ink and replay history. Press it twice within three seconds; this cannot be undone.
- Tool shortcuts work while the application has focus. The drawing/mouse toggle is the configurable global shortcut.
- Current Windows installers are unsigned. Automatic updates are not implemented.

## Build from source

Install Node.js 22 or later, npm, Rust stable, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS. Windows needs the MSVC C++ build tools, Windows SDK, and WebView2.

```sh
npm ci
npm test
cargo test --locked --manifest-path src-tauri/Cargo.toml --features custom-protocol
npm run dev
```

Create a Windows NSIS installer on Windows:

```sh
npm run installer:windows
```

The default output is `src-tauri/target/release/bundle/nsis/`. `CARGO_TARGET_DIR` overrides the target directory. The native binary is `onpen`. New settings use the OnPen config folder, with migration from the legacy identifier. `ONPEN_DATA_DIR` overrides the profile directory. Legacy environment variable names remain readable for upgrades.

## Repository layout

```text
layerpen/
├── src-tauri/       Rust application, native windows, capture, GIF export
├── ui/              HTML, CSS, and JavaScript overlay and settings
├── tests/           JavaScript geometry, drawing, and replay tests
├── docs/            Guides, localization, and validation records
├── scripts/         Portable source packaging utility
├── .github/         CI and issue templates
├── LICENSE
└── THIRD-PARTY-NOTICES.txt
```

Installers, old binaries, build caches, and private test captures belong outside the repository. See [the development guide](docs/DEVELOPMENT.md) for module boundaries and release checks.

## Contribute

Useful contributions include screen-sharing verification, mixed-DPI monitor testing, keyboard usability, translation improvements, and macOS/Linux validation. Please include reproducible steps and avoid posting confidential meeting content in screenshots. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Project-authored code is available under the [MIT license](LICENSE), including use at work. Dependencies retain their own licenses; see [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt). License texts missing from cached crates are supplied with [source provenance](docs/dependency-licenses/sources.json). Unmodified MPL-2.0 dependency sources are included in [third-party-sources.zip](third-party-sources.zip), also bundled with the installer and portable edition.

