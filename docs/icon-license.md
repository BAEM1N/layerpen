# Functional icon licenses

Pointory's functional UI icons use one family: **Lucide**, pinned to
`lucide-static` **1.44.0**. The renderer is local (`ui/icons.js`); no icon CDN,
external request, webfont, or runtime package is required. Icons use the upstream
24 × 24 coordinate grid, round caps and joins, and a shared 1.75-unit stroke.
SVG geometry is copied unchanged from the package. The `ellipse` action uses
Lucide's `circle`; the `fade` action uses `timer` to distinguish disappearing ink.

Lucide is distributed under the **ISC License**. The package also contains the
**MIT License** for icons derived from Feather. Both permit commercial use,
modification, and distribution when their copyright and permission notices are
preserved. Pointory includes the complete, unmodified upstream license in
[`ui/assets/lucide/LICENSE.txt`](../ui/assets/lucide/LICENSE.txt), including both
copyright notices and both licenses. This asset is embedded with the UI in the
desktop application and copied to `LICENSE.lucide` beside the installed resources.
The distribution's `THIRD-PARTY-NOTICES.txt` identifies both locations.

- Official license: <https://lucide.dev/license>
- Pinned source package: <https://registry.npmjs.org/lucide-static/-/lucide-static-1.44.0.tgz>
- Exact source SVG paths and SHA-256 hashes: [`icon-sources.json`](icon-sources.json)

The package archive's SHA-512 integrity was verified against its npm metadata
before extracting the selected SVGs and license. The manifest records that
integrity value so a future update can be audited.

The approved Pointory B symbol is the product's own branding. It is separate from
this functional icon family and is not described as a Lucide or Feather icon.
Keyboard shortcut characters, dimensions (for example `1280 × 720`), and text
labels are text rather than downloaded icon artwork.

## Adding or updating an icon

Use a named SVG from the pinned package, preserve its geometry, and add the alias
and source hash to the manifest. Keep the full upstream notices with any binary
or source distribution. When updating the package version, verify its archive
integrity and recheck its complete license instead of assuming the notices have
remained unchanged. Controls must retain visible labels or accessible names;
the SVGs themselves are decorative and excluded from keyboard focus.
