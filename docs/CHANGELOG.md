# Pointory v0.1.0 brand update — 2026-09-09

- Rename the product, repository links, package metadata, installers, and portable assets to Pointory (포인토리).
- Preserve the internal application identifier and migrate previous settings, imported fonts, and default capture preferences.
- Prefer POINTORY_* environment variables while accepting legacy aliases.
- Reuse complete legacy OpenVINO model caches and existing Windows caption environments in place.
- Refresh the English/Korean documentation screenshots with Pointory branding; retain the old overview image for existing links.
- Keep `pointory.app` documented as the preferred, unpurchased domain.

Validation: 34 Rust, 29 JavaScript UI, and 17 Python STT tests pass (80 total); the manual browser server fixture is excluded from the default Rust run. Model-cache migration tests use mocked inference/download APIs. The Windows release and NSIS installer build, portable ZIP integrity, native toolbar/settings/GIF checks, and actual loopback student-page fixture passed. Installer upgrade/install/uninstall execution, cross-device LAN sharing, and Mac/Linux runtime validation remain pending. See the [validation record](validation/0.1.0.ko.md) for scope and updates.

Earlier changes below retain the product names used at the time.

---

# OnPen v0.1.0 refresh — 2026-09-09

- Rebrand app and release assets to OnPen, keeping storage compatibility.
- Add five persistent app themes and click-to-type text.
- List installed fonts and import persistent TTF assets for text and exports.
- Simplify the toolbar to horizontal/vertical rails, grouped tools and attached detail panels. Preserve old direction settings while retiring line counts.
- Use OnPen capture paths, native executable name, icons and video; migrate legacy capture defaults.
- Improve toolbar, add CC shortcut, dock settings beside toolbar and follow movement.
- Experimental captions: microphone/device selection, local CPU and compatible GPU/NPU probing, optional cloud API keys.
- Ship STT worker and setup instructions; no developer-machine path is required for installed resources.
- macOS runtime/Apple Silicon acceleration remain v0.2 work; API-key providers remain unverified live.

---

# Changelog

## 0.1.0 — First public release

- Selected-monitor annotation with pen, highlighter, shapes, whole-stroke eraser, undo/redo, and stroke movement/resizing.
- Horizontal/vertical toolbar, customizable color palette and shortcuts.
- Frozen-region zoom with coordinate and stroke-width correction.
- White/black boards, fading ink, PNG capture, and GIF replay.
- English, Korean, Japanese, and Simplified Chinese UI with saved language selection.
- Windows x64 installer and portable ZIP; source and dependency notices included.

Public version numbering starts at 0.1.0. Previous local prototypes used an internal version sequence. Legacy settings paths remain compatible. macOS/Linux binaries and meeting-service compatibility have not been validated.

## v0.1.0 documentation and classroom update — 2026-09-09

- English/Korean open-source README and 11 illustrated guides per language.
- Binary-focused release assets; previous MP4 withdrawn.
- Experimental local classroom server: selected-file downloads, random URL/QR, separate opt-in live monitor view.
- Cursor spotlight, configurable radius/dimming, Windows circular live magnification.
- Actual classroom network, native spotlight, macOS/Linux runtime validation remains pending.
