# Troubleshooting and limitations

[OnPen](../../README.md) · [Guide index](README.md) · [KR](../KR/09-troubleshooting.md)

![OnPen — Troubleshooting and limitations](../assets/en-overview.jpg)

*Actual v0.1 UI in browser preview with sample data; not native runtime verification.*

## Nothing draws or clicks reach the wrong app
Check the selected monitor and drawing/mouse mode. Use the toolbar if the global shortcut conflicts. Ink stays in screen coordinates and does not track a scrolling document. In a meeting, confirm the participant view: sharing an app window can omit the overlay. Compatibility with every meeting application is not verified.

## Toolbar or exit button seems missing
Change direction with the toolbar's Horizontal/Vertical switch or in settings, then move the grip away from a screen edge. Shapes, Color, and More tools open in panels beside the toolbar. The final toolbar X quits the app, while the settings X only closes its panel. Save ink first.

## Font is missing or characters differ
Restart after installing system fonts. Import a valid TTF if the font manager does not expose it to system scanning. Choose a font covering your language; missing glyphs use fallback. The browser preview font list is not the native font inventory.

## Export or captions fail
For export, choose a writable folder and check disk space. For captions, verify the optional Python environment, audio device, permissions, model download, and selected backend. API keys and compatible models are your provider account's configuration. See [captions](07-live-captions.md).

## Old product names
OnPen is the product name. The repository URL and application identifier retain historical names for link/upgrade compatibility. Old default capture paths are migrated in preferences without moving your files. Custom paths remain. Ink and replay history are not saved across restarts.

## Report a useful bug
Use [Issues](https://github.com/BAEM1N/layerpen/issues/new?template=bug_report.md). Include OnPen version, OS, monitor layout/scaling, exact steps, expected vs actual result, and a redacted screenshot. Never include API keys or private documents. Windows is the shipped binary target; macOS/Linux native testing and cloud-key validation remain limited.

