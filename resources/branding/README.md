# cosmic-locale branding assets

Source SVGs for the project's icon and logo. The build / install
flow only ships [`cosmic-locale-app-icon.svg`](cosmic-locale-app-icon.svg)
(copied to `resources/icons/hicolor/scalable/apps/icon.svg` and
installed under `/usr/share/icons/hicolor/scalable/apps/`); the
others live here for documentation, marketing material, and any
future symbolic-icon work.

| File | Purpose |
|---|---|
| `cosmic-locale-app-icon.svg` | Full app icon, 512×512, with rounded-rect dark gradient background. **Shipped to users** as the launcher icon. |
| `cosmic-locale-symbol.svg` | Just the symbol — same orbits + globe + terminal prompt — without a background. Useful for compositing on top of arbitrary surfaces. |
| `cosmic-locale-symbol-white.svg` | Monochrome white, no background. Use when overlaying on dark surfaces (e.g. system tray on a dark theme). |
| `cosmic-locale-symbol-black.svg` | Monochrome black, no background. Use when overlaying on light surfaces. |
| `cosmic-locale-horizontal-logo.svg` | Horizontal lockup combining the symbol and the wordmark. README header / website / press. |
| `icon-color.svg`, `icon-mono.svg` | Earlier simpler iterations of the mark. Kept for reference. |

If you change the master design, the **shipped** copy at
`resources/icons/hicolor/scalable/apps/icon.svg` needs the same
update — they're independent files (no symlink) so they don't get
out of sync silently from a careless edit on one side. Updating
both is one edit each.
