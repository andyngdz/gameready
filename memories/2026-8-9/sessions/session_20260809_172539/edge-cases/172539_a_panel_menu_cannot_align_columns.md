# A panel menu cannot align columns

Case:
GNOME draws dbusmenu labels in the desktop's proportional font
(`gsettings get org.gnome.desktop.interface font-name` reports `Ubuntu Sans 11`
here), and dbusmenu carries one flat `label` string per item with no columns,
no markup, and no tab stops. Space padding lines two rows up at two different
places and reads as a mistake.

Use a separator instead. `crates/gameready-tray/src/tray/domain.rs` puts
`  ·  ` between a row's name and its note, matching what the doctor screen
already does between fields.
