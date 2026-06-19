#!/usr/bin/env bash
# Build + install xPackageManager.
#
# On systems whose enabled repos already provide `downgrade` / `appimageupdate`
# (e.g. XeroLinux + Chaotic-AUR), those are pulled as normal dependencies and
# nothing extra is built. On a fresh Arch (core+extra only), this first builds
# `downgrade` and `appimageupdate-git` from the standalone PKGBUILDs under deps/
# - real, separately-removable Arch packages, each with its OWN upstream version,
# never the AUR - then builds and installs xpm-gui.
#
# Build artifacts (packages, downloads, work dirs) are cleaned automatically
# after a successful install.
#
#   ./build.sh [makepkg-args...]   build + install everything, then clean up
#   ./build.sh --no-clean          build + install, KEEP the built packages
#   ./build.sh --clean             only clean up (no build)
#
# Uninstall (not done here):  sudo pacman -R xpm-gui [appimageupdate-git downgrade]

set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"   # packaging/
proj="$(cd "$here/.." && pwd)"                          # project root

_clean() {
    shopt -s nullglob
    local PKGDEST="" SRCDEST="" _c
    for _c in /etc/makepkg.conf "$HOME/.makepkg.conf" \
              "${XDG_CONFIG_HOME:-$HOME/.config}/pacman/makepkg.conf"; do
        [[ -r "$_c" ]] && source "$_c"
    done
    echo "==> Cleaning build artifacts"
    rm -rf "$here"/*.pkg.tar.* "$here"/*.tar.* "$here"/src "$here"/pkg \
           "$here"/deps/*/src "$here"/deps/*/pkg "$here"/deps/*/*.pkg.tar.* \
           "$here"/deps/*/*.tar.* \
           "$here"/deps/appimageupdate-git/AppImageUpdate \
           "$here"/deps/appimageupdate-git/argagg \
           "$here"/deps/appimageupdate-git/build \
           "$here"/deps/downgrade/downgrade-* \
           "$proj"/target/pkg
    if [[ -n "$PKGDEST" ]]; then
        rm -f "$PKGDEST"/xpm-gui-*.pkg.tar.* \
              "$PKGDEST"/downgrade-*.pkg.tar.* \
              "$PKGDEST"/appimageupdate-git-*.pkg.tar.*
    fi
}

# --- arg parsing ---
_do_clean=1
_clean_only=0
_mkargs=()
for _a in "$@"; do
    case "$_a" in
        --clean)            _clean_only=1 ;;
        --no-clean|--keep)  _do_clean=0 ;;
        *)                  _mkargs+=("$_a") ;;
    esac
done

if (( _clean_only )); then
    _clean
    echo "==> Done. Installed packages left untouched."
    exit 0
fi

if [[ $EUID -eq 0 ]]; then
    echo "Run as your normal user, NOT root - makepkg refuses to run as root" >&2
    echo "(it will sudo for the install steps itself)." >&2
    exit 1
fi

_have()      { pacman -Si "$1" &>/dev/null; }   # exact pkg name in an enabled repo?
_installed() { pacman -Qq "$1" &>/dev/null; }   # installed under that exact name?

# Will makepkg be able to resolve dependency $1? Satisfied if it's already
# installed/provided (pacman -T honors provides), or if any of the candidate
# package names ($@) exists in an enabled repo (pacman -Si is name-only, so we
# pass the real repo names - e.g. appimageupdate is provided by appimageupdate-git).
_dep_ok() {
    pacman -T "$1" &>/dev/null && return 0
    local _n
    for _n in "$@"; do _have "$_n" && return 0; done
    return 1
}

# downgrade: build our standalone package only if nothing provides/has it.
if ! _have downgrade && ! _installed downgrade; then
    echo "==> downgrade not available - building it from deps/downgrade"
    ( cd "$here/deps/downgrade" && makepkg -sfi --noconfirm )
fi

# appimageupdate: same. The name in repos is appimageupdate-git (provides appimageupdate).
if ! _have appimageupdate && ! _have appimageupdate-git \
   && ! _installed appimageupdate-git && ! _installed appimageupdate; then
    echo "==> appimageupdate not available - building it from deps/appimageupdate-git"
    ( cd "$here/deps/appimageupdate-git" && makepkg -sfi --noconfirm )
fi

# Sanity check before building xpm-gui: both deps must now be resolvable
# (installed, provided, or in a repo - including the -git name for appimageupdate).
if ! _dep_ok downgrade; then
    echo "ERROR: 'downgrade' not installed/available; cannot build xpm-gui." >&2
    echo "       The deps/downgrade build above must have failed - see its output." >&2
    exit 1
fi
if ! _dep_ok appimageupdate appimageupdate-git; then
    echo "ERROR: 'appimageupdate' not installed/available; cannot build xpm-gui." >&2
    echo "       The deps/appimageupdate-git build above must have failed - see its output." >&2
    exit 1
fi

echo "==> building xpm-gui"
( cd "$here" && makepkg -sfci "${_mkargs[@]}" )

if (( _do_clean )); then
    _clean
fi
echo "==> All done."
