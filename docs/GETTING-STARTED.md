# Getting started with OnPen

Choose Settings → Language for English, 한국어, 日本語, or 简体中文. System language is selected automatically by default.

OnPen adds a drawing layer over one monitor. Use it while sharing your own screen or demonstrating a desktop application.

## Windows installation

Run the Windows x64 setup executable and choose an installation directory. The installer supports English, Korean, Japanese, and Simplified Chinese and installs for the current user. WebView2 is required; installation may download it if it is missing. A portable ZIP skips installation, but still uses the normal user settings folder.

Close any older Monitor Ink instance before starting OnPen. The applications share their legacy settings location. A previous Monitor Ink installation may remain as a separate Windows installed-app entry; remove the old version through Windows Settings if you no longer need it.

## Language and controls

| Control | Meaning |
| --- | --- |
| 설정 / gear | Settings; choose your annotation monitor here |
| 펜 / 형광펜 | Pen / highlighter |
| 지우개 | Whole-stroke eraser |
| 선택 | Select, move, and resize an annotation |
| 커서 / mouse icon | Interact with the desktop |
| eye icon | Hide or show the ink layer |
| 부분 확대 / magnifier | Magnify a frozen screen region |
| 카메라 / capture | Save a PNG |
| 전체 지우기 / trash | Clear ink and GIF history; press twice within three seconds |

The default global draw/mouse toggle is Ctrl+Shift+D on Windows and Cmd+Shift+D on macOS (unverified). Individual tool shortcuts are configurable in Settings. Most tool shortcuts require application focus.

## Sharing in a meeting

1. Select your monitor in OnPen.
2. Share that entire monitor in your meeting application.
3. Draw a short test stroke and ask a participant to confirm it is visible.
4. Switch to mouse mode to change slides, scroll, or click. Hide or clear annotations when changing content.

An application-window or browser-tab share may not contain the overlay. No meeting-service compatibility is claimed until tested from a participant's view. OnPen does not let other participants draw on your screen.

## Save your work

PNG defaults to screen plus annotations. Enable ink-only capture for a transparent background. GIF export replays the drawing sequence over a static background, not a screen recording. Export before exiting: sessions are not automatically restored.

The default capture folder is Pictures/OnPen. Existing settings using Pictures/MonitorInk switch to the new folder; existing files stay in the old folder. Custom folders are preserved. Change the folder in Settings. The portable and installed editions share settings unless `ONPEN_DATA_DIR` points to a separate profile.

## Uninstall

Use Windows Settings → Apps → OnPen → Uninstall. Leave the installer’s app-data deletion option unchecked to preserve settings. Keep captures outside the installation and configuration directories. There is no automatic updater.
