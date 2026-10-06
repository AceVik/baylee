#!/usr/bin/env bash
# Opens the installers scripts/package-installers.sh wrote and checks what a
# player would get from them.
#
#   scripts/release/check-installers.sh [--install] <target-triple> <version>
#
# Without --install it only looks: mounts the dmg, extracts the AppImage,
# lists the .deb. That much runs on a developer's machine.
#
# --install is for a throwaway CI runner: it installs as a player would and
# removes it again. macOS copies Baylee.app to /Applications (what the drag
# does) and starts it from there until the client logs the assets it found;
# Windows runs the setup silently for this user, checks the files, shortcut
# and uninstall entry, then uninstalls; Linux installs the .deb with apt,
# which also proves its dependencies resolve, then removes it.
set -euo pipefail

install=false
if [ "${1:-}" = --install ]; then install=true; shift; fi
target=$1
version=$2
cd "$(dirname "$0")/../.."
out=target/package

fail() { echo "::error::$*" >&2; exit 1; }
need() { [ -e "$1" ] || [ -L "$1" ] || fail "missing: $1"; }

case "$target" in
x86_64-*) arch=x86_64 ;;
aarch64-*) arch=aarch64 ;;
*) fail "unsupported target $target" ;;
esac

# The launcher logs `assets from <dir>` once the client is up; that line is
# how a smoke test sees which folder the started client reads
# (docs/client.md §"Where a packaged desktop build finds its fonts").
started_from() {
    local program=$1 expect=$2 log
    log=$(mktemp)
    RUST_LOG=info "$program" >"$log" 2>&1 &
    local pid=$!
    local seen=false
    for _ in $(seq 1 120); do
        if grep -qF "assets from $expect" "$log"; then seen=true; break; fi
        kill -0 "$pid" 2>/dev/null || break
        sleep 0.5
    done
    pkill -P "$pid" 2>/dev/null || true
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    if ! $seen; then
        cat "$log" >&2
        fail "$program did not start a client reading $expect"
    fi
    echo "started from $expect"
}

mac() {
    local dmg="$out/Baylee-$version-$arch.dmg"
    need "$dmg"
    local mnt
    mnt=$(mktemp -d)
    hdiutil attach -nobrowse -readonly -noautoopen -mountpoint "$mnt" "$dmg" >/dev/null
    # shellcheck disable=SC2064 # expand now: $mnt is local
    trap "hdiutil detach '$mnt' -quiet || hdiutil detach '$mnt' -force -quiet" EXIT
    local listing
    listing=$(cd "$mnt" && find . -mindepth 1 -maxdepth 1 | sed 's|^\./||' | LC_ALL=C sort | tr '\n' ' ')
    [ "$listing" = ".DS_Store .VolumeIcon.icns .background.tiff Applications Baylee.app " ] ||
        fail "unexpected disk image contents: $listing"
    [ "$(readlink "$mnt/Applications")" = /Applications ] || fail "Applications is not a link to /Applications"
    [ "$(readlink "$mnt/Baylee.app/Contents/MacOS/assets")" = ../Resources/assets ] ||
        fail "the bundle lost its assets link"
    need "$mnt/Baylee.app/Contents/MacOS/baylee-client"
    need "$mnt/Baylee.app/Contents/MacOS/baylee-runtime"
    codesign --verify --deep --strict "$mnt/Baylee.app"
    echo "dmg: layout and seal ok"
    $install || return 0
    local app=/Applications/Baylee.app
    [ ! -e "$app" ] || fail "$app exists already; this check runs on a clean runner"
    ditto "$mnt/Baylee.app" "$app"
    codesign --verify --deep --strict "$app"
    started_from "$app/Contents/MacOS/baylee-client" "$app/Contents/MacOS/assets"
    rm -rf "$app"
}

windows() {
    local winarch
    case "$arch" in x86_64) winarch=x64 ;; aarch64) winarch=arm64 ;; esac
    local setup="$out/Baylee-Setup-$version-$winarch.exe"
    need "$setup"
    echo "setup: present"
    $install || return 0
    local dir
    dir="$(cygpath -u "$LOCALAPPDATA")/Programs/Baylee"
    [ ! -e "$dir" ] || fail "$dir exists already; this check runs on a clean runner"
    # Inno's own `/` switches; MSYS_NO_PATHCONV keeps the MSYS shell from
    # turning them into paths.
    MSYS_NO_PATHCONV=1 "$setup" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART "/LOG=$(cygpath -w "$out/setup.log")"
    for f in baylee-client.exe baylee-runtime.exe baylee.ico LICENSE NOTICE README.txt \
        assets/fonts/AlegreyaSans-Regular.ttf unins000.exe; do
        need "$dir/$f"
    done
    need "$(cygpath -u "$APPDATA")/Microsoft/Windows/Start Menu/Programs/Baylee.lnk"
    # The uninstall entry is per user: Settings > Apps lists it, and no
    # administrator was asked.
    MSYS_NO_PATHCONV=1 reg query 'HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{01A111A7-93E6-7434-8D14-F893B3405EC8}_is1' /v DisplayName >/dev/null ||
        fail "no per-user uninstall entry"
    MSYS_NO_PATHCONV=1 "$dir/unins000.exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
    # The uninstaller hands itself to a copy in %TEMP% and returns at once.
    for _ in $(seq 1 60); do [ -e "$dir/baylee-client.exe" ] || break; sleep 1; done
    [ ! -e "$dir/baylee-client.exe" ] || fail "uninstall left baylee-client.exe"
    echo "setup: installed per user and uninstalled"
}

linux() {
    local debarch
    case "$arch" in x86_64) debarch=amd64 ;; aarch64) debarch=arm64 ;; esac
    local image="$out/Baylee-$version-$arch.AppImage" deb="$out/baylee_${version}_$debarch.deb"
    need "$image"
    need "$deb"
    local tmp
    tmp=$(mktemp -d)
    # Extracting needs no FUSE; the runtime is what a player's system runs.
    (cd "$tmp" && "$OLDPWD/$image" --appimage-extract >/dev/null)
    local root="$tmp/squashfs-root"
    [ "$(readlink "$root/AppRun")" = opt/baylee/baylee-client ] || fail "AppRun is not the launcher"
    for f in baylee-client baylee-runtime LICENSE NOTICE README.txt assets/fonts/AlegreyaSans-Regular.ttf; do
        need "$root/opt/baylee/$f"
    done
    need "$root/baylee.desktop"
    need "$root/baylee.png"
    rm -rf "$tmp"
    echo "AppImage: layout ok"
    dpkg-deb --info "$deb" >/dev/null
    # Whole listing first: `grep -q` stops reading early, and under pipefail
    # the killed tar would fail the check.
    local contents
    contents=$(dpkg-deb --contents "$deb")
    grep -q '\./opt/baylee/baylee-runtime$' <<<"$contents" || fail ".deb has no runtime"
    echo ".deb: readable"
    $install || return 0
    sudo apt-get install -y --no-install-recommends "./$deb" desktop-file-utils
    [ "$(readlink -f /usr/bin/baylee)" = /opt/baylee/baylee-client ] || fail "/usr/bin/baylee does not lead to the launcher"
    need /opt/baylee/baylee-runtime
    need /usr/share/applications/baylee.desktop
    need /usr/share/icons/hicolor/256x256/apps/baylee.png
    desktop-file-validate /usr/share/applications/baylee.desktop
    # Root-owned, so the launcher will not install updates here; the client
    # only links to the release (docs/releasing.md §"Installers").
    [ ! -w /opt/baylee ] || fail "/opt/baylee is writable for $(id -un)"
    sudo apt-get remove -y baylee
    [ ! -e /opt/baylee/baylee-client ] || fail "removing the package left /opt/baylee"
    echo ".deb: installed and removed"
}

case "$target" in
*-apple-darwin) mac ;;
*-pc-windows-msvc) windows ;;
*-unknown-linux-gnu) linux ;;
*) fail "no installer for $target" ;;
esac
