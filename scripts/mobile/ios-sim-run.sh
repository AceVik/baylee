#!/usr/bin/env bash
# Build the client for the iOS **simulator**, wrap it in an .app bundle and
# launch it there.
#
#     scripts/mobile/ios-sim-run.sh                       # boot a simulator and run
#     scripts/mobile/ios-sim-run.sh --device "iPhone 15 Pro"
#     scripts/mobile/ios-sim-run.sh --gateway http://127.0.0.1:28766
#
# There is no Xcode project here, and that is deliberate: a simulator app is a
# directory with a Mach-O binary and an `Info.plist` in it, nothing is signed,
# and `simctl install` takes it as it is. A project would be a second place
# where the build settings live.
#
# The simulator shares this machine's network stack, so the gateway is plain
# `127.0.0.1` — and so is dev-control: the client binds its loopback socket on
# *this* machine's loopback, which is why nothing has to be forwarded.
set -euo pipefail

cd "$(dirname "$0")/../.."

DEVICE="iPhone 15 Pro"
GATEWAY="http://127.0.0.1:28766"
DEVCTL_PORT=28770
BUNDLE_ID="local.baylee.client"

while [ $# -gt 0 ]; do
    case "$1" in
        --device) DEVICE="$2"; shift 2 ;;
        --gateway) GATEWAY="$2"; shift 2 ;;
        --devctl) DEVCTL_PORT="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

TARGET=aarch64-apple-ios-sim
APP="target/ios/Baylee.app"

echo "building for ${TARGET}"
cargo build -p baylee-client --features dev-control --target "$TARGET"

rm -rf "$APP"
mkdir -p "$APP"
cp "target/${TARGET}/debug/baylee-client" "$APP/baylee-client"
# Beside the executable, which is where bevy looks when neither
# BEVY_ASSET_ROOT nor CARGO_MANIFEST_DIR is set — see `standalone::asset_root`.
cp -R crates/baylee-client/assets "$APP/assets"

cat > "$APP/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>baylee-client</string>
    <key>CFBundleIdentifier</key><string>local.baylee.client</string>
    <key>CFBundleName</key><string>Baylee</string>
    <key>CFBundleDisplayName</key><string>Baylee</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>0.1.0</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>CFBundleSupportedPlatforms</key><array><string>iPhoneSimulator</string></array>
    <key>MinimumOSVersion</key><string>16.0</string>
    <key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>
    <!-- An empty launch-screen dict is what stops iOS from running the app in
         a letterboxed compatibility window at the wrong resolution. -->
    <key>UILaunchScreen</key><dict/>
    <key>UIRequiredDeviceCapabilities</key><array><string>arm64</string></array>
    <!-- A table is played across, not down. Both landscape directions and
         neither portrait one, on phone and on tablet — the same thing the
         Android shim's manifest asks for with `sensorLandscape`. iOS takes
         the launch orientation from this list, so the app opens the right
         way up however the device is held and never starts portrait. -->
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UISupportedInterfaceOrientations~ipad</key>
    <array>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UIStatusBarHidden</key><true/>
    <key>UIApplicationSupportsIndirectInputEvents</key><true/>
</dict>
</plist>
PLIST

UDID="$(xcrun simctl list devices available | awk -v d="$DEVICE" '$0 ~ d {match($0, /\(([0-9A-F-]{36})\)/, m); print m[1]; exit}' 2>/dev/null || true)"
if [ -z "$UDID" ]; then
    UDID="$(xcrun simctl list devices available | grep -F "$DEVICE (" | head -1 | sed -E 's/.*\(([0-9A-F-]{36})\).*/\1/')"
fi
[ -n "$UDID" ] || { echo "no simulator called '$DEVICE'; xcrun simctl list devices available" >&2; exit 1; }

echo "simulator ${DEVICE} ${UDID}"
xcrun simctl boot "$UDID" 2>/dev/null || true
open -a Simulator
xcrun simctl install "$UDID" "$APP"

# `simctl` passes an environment through, one SIMCTL_CHILD_ prefix at a time —
# which is why the iOS build needs no compiled-in address the way Android does.
SIMCTL_CHILD_BAYLEE_GATEWAY="$GATEWAY" \
SIMCTL_CHILD_BAYLEE_DEV_CONTROL="$DEVCTL_PORT" \
    xcrun simctl launch --console-pty "$UDID" "$BUNDLE_ID"
