# One SVG serves two consumers, and only one of them reads its colour

Case:
Case: `crates/gameready-tray/assets/gameready.svg` is read two ways, and the
fill attribute matters to exactly one of them.

- The tray does NOT read it. `icon.rs` rasterizes the file and `tint()` builds
  ARGB from `pixel.alpha()` plus the `Ink` colour; the source RGB is never
  touched. Changing the fill cannot change the tray at any size: measured 0
  differing pixels of 484, 1024 and 2304 at the three sizes in `SIZES`.
- The desktop shell DOES read it. `install.sh` copies the file to
  `~/.local/share/icons/hicolor/scalable/apps/gameready.svg` and GNOME draws it
  raw, with no tint step. With no fill attribute it fell back to the SVG default
  of black and was invisible on the dark app grid, fixed 2026-08-14 by setting
  `fill: #ffffff` in the file's style block.

Consequence for a future change: editing `tint()` will never fix the app grid,
and editing the fill will never change the tray. Also, the tray embeds the file
with `include_str!` at build time, so a change to the asset does nothing on a
machine until the tray binary is rebuilt and reinstalled.

Second, separate limit: the artwork is 122.88x79.92, a 1.54:1 ratio. Any square
icon slot letterboxes it, so it reads smaller than the square icons beside it in
the app grid. A square canvas does not help, the art itself is wide. Only a
background plate or a redrawn shape would, and both were declined.
