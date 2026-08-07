#!/bin/sh
# Installs the gameready binary from a GitHub release.
#
#   curl -fsSL https://andyngdz.github.io/gameready/install.sh | sh
#
# Environment:
#   GAMEREADY_VERSION      release tag to install, default: latest
#   GAMEREADY_INSTALL_DIR  where to put the binary, default: ~/.local/bin

set -eu

REPO="andyngdz/gameready"
VERSION="${GAMEREADY_VERSION:-latest}"
INSTALL_DIR="${GAMEREADY_INSTALL_DIR:-$HOME/.local/bin}"

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
x86_64)
	asset="gameready-linux-x86_64"
	;;
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

log "Downloading $asset ($VERSION)"
curl -fsSL -o "$workdir/$asset" "$download_base/$asset" ||
	die "download failed: $download_base/$asset. Check the tag exists at https://github.com/$REPO/releases"
curl -fsSL -o "$workdir/$asset.sha256" "$download_base/$asset.sha256" ||
	die "checksum file missing: $download_base/$asset.sha256"

log "Checking sha256"
# sha256sum -c matches on the bare filename recorded in the release file, so the
# check has to run from the directory both files were downloaded into.
(cd "$workdir" && $verify_checksum "$asset.sha256" >/dev/null 2>&1) ||
	die "sha256 mismatch, the download is damaged or tampered with. Nothing was installed."

mkdir -p "$INSTALL_DIR" 2>/dev/null || true
[ -d "$INSTALL_DIR" ] || die "$INSTALL_DIR does not exist and could not be created."

if [ -w "$INSTALL_DIR" ]; then
	install -m 755 "$workdir/$asset" "$INSTALL_DIR/gameready"
else
	require sudo
	log "$INSTALL_DIR needs root, asking sudo"
	sudo install -m 755 "$workdir/$asset" "$INSTALL_DIR/gameready"
fi

installed="$("$INSTALL_DIR/gameready" --version 2>/dev/null || echo gameready)"
log "Installed $installed to $INSTALL_DIR/gameready"

case ":$PATH:" in
*":$INSTALL_DIR:"*) ;;
*)
	log ""
	log "$INSTALL_DIR is not on your PATH. Add it with:"
	log "  echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.profile"
	;;
esac

log ""
log "Next: gameready doctor"

printf '%s\n' "$INSTALL_DIR/gameready"
