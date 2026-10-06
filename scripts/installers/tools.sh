#!/usr/bin/env bash
# Fetches the installer tools for this system, pinned by version and
# SHA-256, into <dir>, and prints the variables scripts/package-installers.sh
# reads, one `NAME=value` per line (the workflow appends them to GITHUB_ENV).
#
#   scripts/installers/tools.sh <dir> <target-triple>
#
#   macOS    dmgbuild in a virtual environment (macos/requirements.txt, hashes)
#   Windows  Inno Setup 7.1.0, installed for this user only
#   Linux    appimagetool 1.9.1 (extracted, so it needs no FUSE) and the
#            type2 AppImage runtime it embeds; dpkg-deb is the system's
#
# A new version: change it and its checksum here together, from the
# release's own asset digests (`gh release view <tag> -R <repo> --json assets`).
set -euo pipefail

dir=$1
target=$2
cd "$(dirname "$0")/../.."
mkdir -p "$dir"
dir=$(cd "$dir" && pwd)

fetch() {
    local url=$1 sum=$2 file=$3
    curl --fail --location --silent --show-error --retry 3 --output "$file" "$url"
    local got
    if command -v sha256sum >/dev/null; then
        got=$(sha256sum "$file" | cut -d' ' -f1)
    else
        got=$(shasum -a 256 "$file" | cut -d' ' -f1)
    fi
    [ "$got" = "$sum" ] || { echo "::error::$url: sha256 $got, expected $sum" >&2; exit 1; }
}

case "$target" in
*-apple-darwin)
    python3 -m venv "$dir/dmgbuild"
    "$dir/dmgbuild/bin/python" -m pip install --quiet --require-hashes \
        -r scripts/installers/macos/requirements.txt >&2
    echo "DMGBUILD=$dir/dmgbuild/bin/dmgbuild"
    ;;
*-pc-windows-msvc)
    # One compiler for both targets: the x64 edition runs emulated on
    # Windows on ARM, and either edition builds an installer for arm64.
    fetch https://github.com/jrsoftware/issrc/releases/download/is-7_1_0/innosetup-7.1.0-x64.exe \
        0362a383ed217d4c4239b5933866dd96d3eb2102737da92f80f6057a4b40df2f "$dir/innosetup.exe"
    MSYS_NO_PATHCONV=1 "$dir/innosetup.exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- \
        /CURRENTUSER "/DIR=$(cygpath -w "$dir/inno")"
    [ -f "$dir/inno/ISCC.exe" ] || { echo "::error::Inno Setup did not install ISCC.exe" >&2; exit 1; }
    echo "ISCC=$dir/inno/ISCC.exe"
    ;;
x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu)
    arch=${target%%-*}
    case "$arch" in
    x86_64)
        tool=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
        runtime=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
        ;;
    aarch64)
        tool=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
        runtime=00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444
        ;;
    esac
    fetch "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-$arch.AppImage" \
        "$tool" "$dir/appimagetool.AppImage"
    fetch "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-$arch" \
        "$runtime" "$dir/runtime"
    chmod +x "$dir/appimagetool.AppImage"
    rm -rf "$dir/appimagetool"
    (cd "$dir" && ./appimagetool.AppImage --appimage-extract >/dev/null && mv squashfs-root appimagetool)
    echo "APPIMAGETOOL=$dir/appimagetool/AppRun"
    echo "APPIMAGE_RUNTIME=$dir/runtime"
    ;;
*)
    echo "::error::no installer tools for $target" >&2
    exit 1
    ;;
esac
