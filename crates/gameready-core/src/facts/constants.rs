//! Distribution ids that more than one probe has to agree on.

/// The three family names. Each is both an `ID` value the parser matches and
/// the name the family renders as, so a rename that fixed only one site would
/// make `doctor` report a family the parser can no longer produce.
pub const ARCH: &str = "arch";

/// See [`ARCH`].
pub const DEBIAN: &str = "debian";

/// See [`ARCH`].
pub const FEDORA: &str = "fedora";

/// Universal Blue's gaming image, Fedora-derived and image-based.
///
/// Named because two probes read it: the family table maps it to Fedora, and
/// the root-filesystem probe marks it read-only. A rename that fixed only one
/// would produce a system classified as Fedora but treated as mutable.
pub const BAZZITE: &str = "bazzite";

/// Fedora's GNOME image, image-based.
pub const SILVERBLUE: &str = "silverblue";

/// Fedora's KDE image, image-based.
pub const KINOITE: &str = "kinoite";
