# Localization

OnPen supports Korean (`ko`), English (`en`), Japanese (`ja`), and Simplified Chinese (`zh-CN`). Settings → Language updates all open application windows and persists the choice. System language is the default; unsupported languages fall back to English. Chinese regional variants currently use Simplified Chinese.

`ui/locales/*.json` contains matching message keys. Existing Korean source messages serve as IDs for both UI strings and application-owned native errors. Placeholders such as `{value}` and `{e}` must remain intact. External diagnostics and user paths are not translated. `ui/i18n.js` translates text nodes and accessibility attributes without changing HTML or event handlers, and retains each node's source text for reversible language changes.

Do not translate monitor names, filenames, user content, shortcut keys, or saved preference values. Add new messages to all four catalogs and run `npm test`. Check translated settings at 520px and 620px widths and both toolbar orientations. Review a real app's settings, toolbar, save dialog, and restart persistence before releasing. Contributions from native speakers are welcome.
