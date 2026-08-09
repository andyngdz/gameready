#!/bin/sh
# Installs the gameready binaries from a GitHub release: the CLI, and the tray
# indicator with its desktop entry so it starts at the next login.
#
#   curl -fsSL https://andyngdz.github.io/gameready/install.sh | sh
#
# Environment:
#   GAMEREADY_VERSION      release tag to install, default: latest
#   GAMEREADY_INSTALL_DIR  where to put the binaries, default: ~/.local/bin
#   GAMEREADY_NO_TRAY      set to any value to install the CLI only

set -eu

REPO="andyngdz/gameready"
VERSION="${GAMEREADY_VERSION:-latest}"
INSTALL_DIR="${GAMEREADY_INSTALL_DIR:-$HOME/.local/bin}"

# Where a desktop reads per-user autostart entries and icons, per the XDG spec.
AUTOSTART_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"
ICON_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"

log() {
	printf '%s\n' "$*" >&2
}

die() {
	printf 'install: %s\n' "$*" >&2
	exit 1
}

require() {
	command -v "$1" >/dev/null 2>&1 ||
		die "$1 is required. Install it, then run this script again."
}

require curl
require install
require mktemp

system="$(uname -s)"
[ "$system" = Linux ] ||
	die "gameready runs on Linux only, this machine reports $system."

machine="$(uname -m)"
case "$machine" in
x86_64) ;;
*)
	die "no prebuilt binary for $machine. Build from source: https://github.com/$REPO#install"
	;;
esac

if command -v sha256sum >/dev/null 2>&1; then
	verify_checksum="sha256sum -c"
elif command -v shasum >/dev/null 2>&1; then
	verify_checksum="shasum -a 256 -c"
else
	die "sha256sum or shasum is required to check the download."
fi

if [ "$VERSION" = latest ]; then
	download_base="https://github.com/$REPO/releases/latest/download"
else
	download_base="https://github.com/$REPO/releases/download/$VERSION"
fi

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT INT TERM

# Downloads one release asset, and checks it against the sha256 the release
# published beside it.
fetch_verified() {
	asset="$1"
	log "Downloading $asset ($VERSION)"
	curl -fsSL -o "$workdir/$asset" "$download_base/$asset" ||
		die "download failed: $download_base/$asset. Check the tag exists at https://github.com/$REPO/releases"
	curl -fsSL -o "$workdir/$asset.sha256" "$download_base/$asset.sha256" ||
		die "checksum file missing: $download_base/$asset.sha256"

	# sha256sum -c matches on the bare filename recorded in the release file, so
	# the check has to run from the directory both files were downloaded into.
	# Not "nothing was installed": the CLI lands before the tray is fetched, so
	# a mismatch here stops the install without undoing what already worked.
	(cd "$workdir" && $verify_checksum "$asset.sha256" >/dev/null 2>&1) ||
		die "sha256 mismatch on $asset, the download is damaged or tampered with. It was not installed."
}

# Puts one file where it belongs, asking sudo only when the directory needs it.
install_into() {
	mode="$1"
	source_file="$2"
	target="$3"
	target_dir="$(dirname "$target")"

	mkdir -p "$target_dir" 2>/dev/null || true
	[ -d "$target_dir" ] || die "$target_dir does not exist and could not be created."

	if [ -w "$target_dir" ]; then
		install -m "$mode" "$source_file" "$target"
	else
		require sudo
		log "$target_dir needs root, asking sudo"
		sudo install -m "$mode" "$source_file" "$target"
	fi
}

fetch_verified gameready-linux-x86_64
install_into 755 "$workdir/gameready-linux-x86_64" "$INSTALL_DIR/gameready"

installed="$("$INSTALL_DIR/gameready" --version 2>/dev/null || echo gameready)"
log "Installed $installed to $INSTALL_DIR/gameready"

if [ -z "${GAMEREADY_NO_TRAY:-}" ]; then
	fetch_verified gameready-tray-linux-x86_64
	install_into 755 "$workdir/gameready-tray-linux-x86_64" "$INSTALL_DIR/gameready-tray"

	# The entry and its icon are not checksummed: they are text and an SVG, and
	# the binary they point at is the one that had to be verified.
	curl -fsSL -o "$workdir/gameready-tray.desktop" "$download_base/gameready-tray.desktop" ||
		die "desktop entry missing: $download_base/gameready-tray.desktop"
	curl -fsSL -o "$workdir/gameready.svg" "$download_base/gameready.svg" ||
		die "icon missing: $download_base/gameready.svg"

	# Absolute path rather than a bare name: a login session starts autostart
	# entries with its own PATH, which usually does not carry ~/.local/bin.
	sed "s|^Exec=gameready-tray$|Exec=$INSTALL_DIR/gameready-tray|" \
		"$workdir/gameready-tray.desktop" > "$workdir/autostart.desktop"

	install_into 644 "$workdir/autostart.desktop" "$AUTOSTART_DIR/gameready-tray.desktop"
	install_into 644 "$workdir/gameready.svg" "$ICON_DIR/gameready.svg"
	log "Installed the tray indicator, starting at your next login"
fi

case ":$PATH:" in
*":$INSTALL_DIR:"*) ;;
*)
	log ""
	log "$INSTALL_DIR is not on your PATH. Add it with:"
	log "  echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.profile"
	;;
esac

log ""
if [ -z "${GAMEREADY_NO_TRAY:-}" ]; then
	log "Next: gameready doctor, or start the tray now with gameready-tray"
else
	log "Next: gameready doctor"
fi

printf '%s\n' "$INSTALL_DIR/gameready"
