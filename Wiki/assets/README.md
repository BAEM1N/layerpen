# Screenshot provenance

Captured from OnPen v0.1 source on 2026-09-09. JPEG is the browser capture tool's returned format; files are stored without visual alteration.

- `kr-*` / `en-*`: actual settings UI in a 520×700 iframe. Language controls were used to select Korean or English. Example monitors and font lists are browser-preview placeholders, not detected native hardware.
- `toolbar-layouts.jpg`: actual horizontal and vertical toolbar UI with the attached tools panel.
- `onpen-overview.jpg`: actual toolbar and drawing renderer over an authored example lesson. This is a documentation composition, not a native desktop screenshot.
- `captions-*`: actual bilingual caption settings, no API key, microphone recording, or cloud session.
- `sharing-settings.jpg`: actual bilingual sharing settings, server disabled in browser preview.
- `sharing-student.jpg`: actual Rust HTTP sharing server on loopback, using its ignored `browser_fixture` test with a sample README download and a static example JPEG. The browser fetches the frame using the production viewer script. No personal screen or audio was captured.

Reproduce UI previews by serving the repository root with `python -m http.server 8767 --bind 127.0.0.1`, then opening `scripts/docs-preview.html?page=settings`, `captions`, `sharing`, `hero`, or `layouts`. Native overlay behavior, microphone access, cross-device LAN access, and native screen streaming require separate runtime checks. Never substitute these preview images for evidence of those checks.

- `spotlight-settings.jpg`: actual bilingual settings in preview.
- `spotlight-preview.jpg`: real spotlight shader over the example lesson, highlighting only. Native Windows magnification is not shown or verified by this image.
