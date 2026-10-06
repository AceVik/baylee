#!/usr/bin/env bash
# Wraps a packaged client into the installers a GitHub Release offers beside
# its archives.
#
#   scripts/package-installers.sh <target-triple> <version>
#
# Reads the tree scripts/package-client.sh left in
# target/package/baylee-client-<version>-<triple>/ and writes, beside it,
# one file per line on stdout:
#
#   aarch64-apple-darwin    Baylee-<version>-aarch64.dmg
#   *-pc-windows-msvc       Baylee-Setup-<version>-{x64,arm64}.exe
#   *-unknown-linux-gnu     Baylee-<version>-{x86_64,aarch64}.AppImage
#                           baylee_<version>_{amd64,arm64}.deb
#
# The installers hold exactly the archive's tree. The updater never
# downloads them: it fetches and verifies the signed archive (#326), and an
# installed client updates itself from there (docs/releasing.md
# §"Installers").
#
# Tools, each found on PATH or named by a variable:
#   macOS    dmgbuild (DMGBUILD; scripts/installers/macos/requirements.txt)
#   Windows  Inno Setup's ISCC.exe (ISCC)
#   Linux    appimagetool (APPIMAGETOOL) with the AppImage runtime it embeds
#            (APPIMAGE_RUNTIME, optional: without it appimagetool downloads
#            one), and dpkg-deb
set -euo pipefail

target=$1
version=$2
cd "$(dirname "$0")/.."
here=$PWD
art=scripts/installers

name="baylee-client-$version-$target"
out=target/package
stage="$out/$name"
[ -d "$stage" ] || { echo "no $stage; run scripts/package-client.sh first" >&2; exit 1; }

case "$target" in
x86_64-*) arch=x86_64 ;;
aarch64-*) arch=aarch64 ;;
*) echo "unsupported target $target" >&2; exit 1 ;;
esac

dmg() {
    local file="$out/Baylee-$version-$arch.dmg"
    local tool=${DMGBUILD:-dmgbuild}
    rm -f "$file"
    # Word splitting on purpose: DMGBUILD may be a command with arguments.
    # shellcheck disable=SC2086
    $tool -s "$art/macos/dmg-settings.py" \
        -D app="$stage/Baylee.app" \
        -D background="$art/macos/background.png" \
        -D icon="$stage/Baylee.app/Contents/Resources/Baylee.icns" \
        Baylee "$file" >&2
    echo "$file"
}

setup() {
    local winarch numeric file
    case "$arch" in x86_64) winarch=x64 ;; aarch64) winarch=arm64 ;; esac
    # The file version wants four numbers; a pre-release part has no place
    # in it (0.1.0-beta.5 -> 0.1.0.0). The product version keeps it.
    numeric="${version%%[-+]*}.0"
    file="Baylee-Setup-$version-$winarch"
    local iscc=${ISCC:-ISCC.exe}
    # ISCC is a Windows program: Windows paths, and its `/` options, which
    # MSYS_NO_PATHCONV keeps the MSYS shell from rewriting into paths.
    local win=(echo)
    if command -v cygpath >/dev/null; then win=(cygpath -w); fi
    rm -f "$out/$file.exe"
    MSYS_NO_PATHCONV=1 "$iscc" /Q \
        "/DSource=$("${win[@]}" "$here/$stage")" \
        "/DVersion=$version" \
        "/DNumeric=$numeric" \
        "/DArch=$winarch" \
        "/DIcon=$("${win[@]}" "$here/$art/windows/baylee.ico")" \
        "/O$("${win[@]}" "$here/$out")" \
        "/F$file" \
        "$("${win[@]}" "$here/$art/windows/baylee.iss")" >&2
    echo "$out/$file.exe"
}

# The Linux installers keep the archive's tree in one folder, because the
# launcher finds `baylee-runtime` and the client its `assets` beside
# themselves: /opt/baylee in the .deb, the same path inside the AppImage.
opt_tree() {
    local root=$1
    mkdir -p "$root/opt/baylee"
    cp -R "$stage/." "$root/opt/baylee/"
    # Crash symbols stay in the archive; an installed game does not need them.
    rm -f "$root/opt/baylee/"*.pdb
}

# Modes that do not depend on the builder's umask: 0755 directories and
# programs, 0644 everything else.
settle_modes() {
    chmod -R u=rwX,go=rX "$1"
}

appimage() {
    local file="$out/Baylee-$version-$arch.AppImage"
    local appdir="$out/appimage-$target"
    rm -rf "$appdir" "$file"
    opt_tree "$appdir"
    # AppRun is the launcher itself: a link, so `current_exe` is the real
    # file and the launcher's `baylee-runtime` and `assets` sit beside it.
    ln -s opt/baylee/baylee-client "$appdir/AppRun"
    sed 's/^Exec=baylee$/Exec=baylee-client/' "$art/linux/baylee.desktop" >"$appdir/baylee.desktop"
    cp "$art/linux/baylee.png" "$appdir/baylee.png"
    ln -s baylee.png "$appdir/.DirIcon"
    settle_modes "$appdir"
    local runtime=()
    if [ -n "${APPIMAGE_RUNTIME:-}" ]; then runtime=(--runtime-file "$APPIMAGE_RUNTIME"); fi
    # No update information is embedded (no zsync): the client's own updater
    # is the update path, and it never touches this file.
    ARCH=$arch "${APPIMAGETOOL:-appimagetool}" --no-appstream \
        ${runtime[@]+"${runtime[@]}"} "$appdir" "$file" >&2
    rm -rf "$appdir"
    echo "$file"
}

deb() {
    local debarch
    case "$arch" in x86_64) debarch=amd64 ;; aarch64) debarch=arm64 ;; esac
    local file="$out/baylee_${version}_$debarch.deb"
    local root="$out/deb-$target"
    rm -rf "$root" "$file"
    opt_tree "$root"
    mkdir -p "$root/usr/bin" "$root/usr/share/applications" \
        "$root/usr/share/icons/hicolor/256x256/apps" "$root/usr/share/doc/baylee" "$root/DEBIAN"
    # /proc/self/exe resolves the link, so the launcher still finds /opt/baylee.
    ln -s ../../opt/baylee/baylee-client "$root/usr/bin/baylee"
    cp "$art/linux/baylee.desktop" "$root/usr/share/applications/baylee.desktop"
    cp "$art/linux/baylee.png" "$root/usr/share/icons/hicolor/256x256/apps/baylee.png"
    cp LICENSE NOTICE "$root/usr/share/doc/baylee/"
    # Debian sorts `~` before anything, so 0.1.0~beta.5 comes before 0.1.0;
    # with the `-` of semver it would come after.
    # (sed, not ${version/-/~}: bash 3.2 and 5 disagree on the tilde.)
    local debversion
    debversion=$(printf '%s' "$version" | sed 's/-/~/')
    local size
    settle_modes "$root"
    size=$(du -sk "$root" | cut -f1)
    cat >"$root/DEBIAN/control" <<EOF
Package: baylee
Version: $debversion
Architecture: $debarch
Maintainer: Baylee (AceVik) <AceVik@users.noreply.github.com>
Installed-Size: $size
Depends: libc6 (>= 2.35), libasound2 | libasound2t64, libudev1, libxkbcommon-x11-0, libwayland-client0, libvulkan1
Section: games
Priority: optional
Homepage: https://github.com/AceVik/baylee
Description: Baylee card game client (unofficial fan content)
 Plays against the house AI, or at a table on a Baylee gateway.
 Unofficial fan content, not affiliated with Wizards of the Coast.
 .
 Installed system-wide in /opt/baylee, which a player cannot write, so the
 client does not replace itself: it says when a new release is out, and
 that release's .deb is installed over this one.
EOF
    # xz rather than dpkg's zstd default, which Debian before 12 cannot read.
    dpkg-deb -Zxz --root-owner-group --build "$root" "$file" >&2
    rm -rf "$root"
    echo "$file"
}

case "$target" in
*-apple-darwin) dmg ;;
*-pc-windows-msvc) setup ;;
*-unknown-linux-gnu) appimage; deb ;;
*) echo "no installer for $target" >&2; exit 1 ;;
esac
