# Text and fonts

[Pointory](../../README.md) · [Guide index](README.md) · [KR](../KR/04-text-fonts.md)

![Pointory — Text and fonts](../assets/en-text-fonts.jpg)

*Actual v0.1 UI in browser preview with sample data; not native runtime verification.*

## Click to type
1. Set the pen width and choose a font under **Drawing tools → Text font**.
2. Click the toolbar **T** icon, then click the intended location on the screen.
3. Type. **Shift+Enter** inserts a newline; **Enter** commits; **Escape** cancels.
4. Use Select to move or resize committed text. To change characters, erase or undo and recreate it.

There is no default T keyboard shortcut. Input methods such as Korean IME are supported by the editor; finish composition before committing.

## Size follows pen width
At normal zoom, widths 2 / 4 / 8 / 16 / 24 produce text sizes 16 / 24 / 40 / 72 / 104 px. New text uses the current font and width. Existing text retains its stored font and size. Zoom uses source coordinates to maintain display sizing.

## Installed fonts and TTF
The native app scans standard system font directories. Restart Pointory after installing a new font. Fonts only activated inside another font manager may not appear. The screenshot's short preview list is a placeholder, not an inventory of installed fonts.

To add a file, use the TTF import button and select a valid `.ttf` up to 32 MB. Pointory copies it into its own configuration folder; it does not install it system-wide. Duplicate content reuses the same font ID. Imported fonts work in text, PNG, and GIF. Missing characters may use system fallback, so choose a font covering your language and check an export.

