# Changelog

All notable changes to Walls are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2026-08-24

### Changed

- Removed the redundant Search tab: the query field is now global. Type and
  press Enter from any tab to search (query and/or theme color), and use the
  × button to clear and return to the tab's listing.

## [0.3.0] - 2026-08-24

### Added

- **Saved tab**: browse every wallpaper of the active theme (user-downloaded
  plus stock), with the current background marked. Hover a tile to apply it
  or remove it; removal is only offered for user-downloaded files.
- `walls list`, `walls set <path>` and `walls remove <path>` CLI commands.
  Removal is hard-restricted to `~/.config/omarchy/backgrounds/<theme>/` and
  refuses anything else (stock theme files, arbitrary paths). Removing the
  current background automatically cycles to the next one.
- Unit tests for the local library module (listing, current marking, removal
  safety).

## [0.2.0] - 2026-08-23

### Added

- Toplist, Latest and Random browsing modes with keyboard-free tab switching.
- Toplist range selector (1d / 3d / 1w / 1M / 3M / 6M / 1y).
- Theme color search: reads the active Omarchy theme's `colors.toml`, maps its
  palette onto the wallhaven.cc color filter (nearest perceptual match with
  grayscale snapping) and offers one-click color chips.
- Random mode keeps the API seed across pages so results never repeat.
- `walls theme-colors` CLI command.
- Rust codebase split into modules (`api`, `model`, `palette`, `theme`) with
  unit tests for URL building, id extraction, TOML scanning and color mapping.

### Changed

- Plugin renamed from `jlt.wallhaven` to `jltrench.walls`; binary renamed from
  `whd` to `walls`. The product name stays neutral of the wallhaven.cc
  trademark while the description credits the service.
- Search output is now an object (`{results, page, seed}`) instead of a bare
  array, so pagination state survives round-trips through stdout.
- Bar widget icon replaced with a themed SVG recolored via MultiEffect.
- Repository restructured with `manifest.json` at the root per the marketplace
  publishing guide.

## [0.1.0] - 2026-08-23

### Added

- Initial release: bar widget with search panel, SFW search by query,
  download into `~/.config/omarchy/backgrounds/<theme>/` and immediate
  application via `omarchy theme bg set`.
