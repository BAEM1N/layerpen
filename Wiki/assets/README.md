# Screenshot provenance

The main screenshots were refreshed from Pointory v0.1 source on 2026-09-09, including the student page served by the actual Rust HTTP fixture. Settings font-size PNGs were added on 2026-09-10. JPEG and PNG files retain the capture tool's returned format and are stored without visual alteration. The old `onpen-overview.jpg` is retained separately for link compatibility.

- `kr-*` / `en-*`: actual settings UI in a 520×700 iframe. Language controls were used to select Korean or English. Example monitors and font lists are browser-preview placeholders, not detected native hardware.
- `settings-font-ko-20px.png` / `settings-font-en-20px.png`: actual Korean/English settings UI in a 520×700 browser preview, showing the selected font and 20px settings text size. The short font inventory is preview data; these images do not establish native Mac font or input behavior.
- `toolbar-layouts.jpg`: actual horizontal and vertical toolbar UI with the attached tools panel.
- `pointory-overview.jpg`: actual toolbar and drawing renderer over an authored example lesson. This is a documentation composition, not a native desktop screenshot.
- `onpen-overview.jpg`: the previous OnPen overview, retained only so existing image links continue to work. Current README and drawing guides use `pointory-overview.jpg`.
- `captions-*`: actual bilingual caption settings, no API key, microphone recording, or cloud session.
- `sharing-settings.jpg`: actual bilingual sharing settings, server disabled in browser preview.
- `sharing-student.jpg`: actual Pointory Rust HTTP sharing server on loopback, using its ignored `browser_fixture` test with a sample README download and a static example JPEG. The browser displayed the frame using the production viewer script, and the selected README download returned HTTP 200. No personal screen or audio was captured; this is not a cross-device LAN or native live-capture test.

Reproduce UI previews by serving the repository root with `python -m http.server 8767 --bind 127.0.0.1`, then opening `scripts/docs-preview.html?page=settings`, `captions`, `sharing`, `hero`, or `layouts`. Native overlay behavior, microphone access, cross-device LAN access, and native screen streaming require separate runtime checks. Never substitute these preview images for evidence of those checks.

- `spotlight-settings.jpg`: actual bilingual settings in preview.
- `spotlight-preview.jpg`: real spotlight shader over the example lesson, highlighting only. Native Windows magnification is not shown or verified by this image.
