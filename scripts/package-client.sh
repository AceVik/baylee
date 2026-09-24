#!/usr/bin/env bash
# Packs a built game client into the archive a GitHub Release publishes.
#
#   scripts/package-client.sh <bin-dir> <target-triple> <version>
#
# <bin-dir> is where cargo left the binary (`target/<triple>/dist` in the
# release workflow). Writes target/package/baylee-client-<version>-<triple>
# as .zip (Windows, macOS) or .tar.gz (Linux) and prints its path.
#
# The binary alone does not run: the fonts load from `assets/` beside the
# executable at run time (only the shaders are embedded), and the OFL wants
# each font's licence to travel with it, as the AGPL wants LICENSE.
set -euo pipefail

bin_dir=$1
target=$2
version=$3
cd "$(dirname "$0")/.."

name="baylee-client-$version-$target"
out=target/package
stage="$out/$name"
rm -rf "$stage"
mkdir -p "$stage"

case "$target" in
*-windows-*) exe=baylee-client.exe ;;
*) exe=baylee-client ;;
esac
[ -f "$bin_dir/$exe" ] || { echo "no $bin_dir/$exe" >&2; exit 1; }

readme() {
    cat <<EOF
Baylee $version ($target)

Unofficial fan content, not affiliated with Wizards of the Coast.
Source: https://github.com/AceVik/baylee (AGPL-3.0-only, see LICENSE).

Start the client and play against the house AI, or enter a gateway in the
lobby to play at a table. The assets folder must stay next to the program.
EOF
    case "$target" in
    *-apple-*) cat <<'EOF'

macOS: the app is not notarised. On first start macOS refuses it; open
System Settings > Privacy & Security and choose "Open Anyway", or run
    xattr -dr com.apple.quarantine Baylee.app
EOF
        ;;
    *-windows-*) cat <<'EOF'

Windows: the program is not signed. If SmartScreen stops it, choose
"More info" and then "Run anyway".
EOF
        ;;
    *-linux-*) cat <<'EOF'

Linux: needs a Vulkan driver plus the ALSA, udev, X11/Wayland and xkbcommon
runtime libraries (on Debian/Ubuntu: libasound2 libudev1 libxkbcommon-x11-0
libwayland-client0).
EOF
        ;;
    esac
}

case "$target" in
*-apple-*)
    # A bundle, so Finder starts a game rather than a Terminal. Bevy looks
    # for `assets/` next to the executable; the files live in Resources and
    # MacOS holds a link, because codesign treats everything in MacOS as code.
    app="$stage/Baylee.app/Contents"
    mkdir -p "$app/MacOS" "$app/Resources"
    cp "$bin_dir/$exe" "$app/MacOS/"
    cp -R crates/baylee-client/assets "$app/Resources/assets"
    ln -s ../Resources/assets "$app/MacOS/assets"
    cat >"$app/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Baylee</string>
    <key>CFBundleDisplayName</key><string>Baylee</string>
    <key>CFBundleIdentifier</key><string>local.baylee.client</string>
    <key>CFBundleExecutable</key><string>$exe</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
    # Ad hoc, which costs nothing and is not an identity: without a seal
    # over the bundle a downloaded app reads as "damaged" rather than as
    # "unidentified developer", and only the second has a way past it.
    codesign --force --sign - "$stage/Baylee.app"
    # Symbols for a crash report, beside the app rather than in it.
    if [ -d "$bin_dir/$exe.dSYM" ]; then
        cp -RL "$bin_dir/$exe.dSYM" "$stage/"
    fi
    ;;
*)
    cp "$bin_dir/$exe" "$stage/"
    cp -R crates/baylee-client/assets "$stage/assets"
    # MSVC keeps the line tables in a .pdb beside the .exe.
    if [ -f "$bin_dir/baylee_client.pdb" ]; then
        cp "$bin_dir/baylee_client.pdb" "$stage/"
    fi
    ;;
esac

cp LICENSE NOTICE "$stage/"
readme >"$stage/README.txt"

cd "$out"
case "$target" in
*-apple-*)
    rm -f "$name.zip"
    ditto -c -k --keepParent "$name" "$name.zip"
    echo "$out/$name.zip"
    ;;
*-windows-*)
    rm -f "$name.zip"
    if command -v 7z >/dev/null; then
        7z a -tzip -bso0 "$name.zip" "$name"
    else
        powershell -NoProfile -Command "Compress-Archive -Path '$name' -DestinationPath '$name.zip'"
    fi
    echo "$out/$name.zip"
    ;;
*)
    tar -czf "$name.tar.gz" "$name"
    echo "$out/$name.tar.gz"
    ;;
esac
