#!/usr/bin/env bash
# Build (and optionally install and run) the client as an APK.
#
#     scripts/mobile/android-build.sh                          # LAN address, dev-control on
#     scripts/mobile/android-build.sh --gateway http://10.0.2.2:28766 --run
#     scripts/mobile/android-build.sh --no-dev-control         # a plain build
#
# Two values are compiled **in**, because an Android app inherits no
# environment: the gateway it dials and the port dev-control listens on. That
# is why this is a script and not one command to remember — changing the
# gateway means a rebuild, and `crates/baylee-client/build.rs` is what makes
# cargo notice.
#
# Which gateway address is right depends on where the client runs:
#
#     Pixel on WiFi      http://<this machine>:28766   (the default below)
#     Pixel over USB     http://127.0.0.1:28766        (cargo apk sets up adb reverse)
#     Android emulator   http://10.0.2.2:28766
set -euo pipefail

cd "$(dirname "$0")/../.."
source scripts/mobile/android-env.sh

LAN_IP="$(ipconfig getifaddr en0 2>/dev/null || ipconfig getifaddr en1 2>/dev/null || echo 127.0.0.1)"
GATEWAY="http://${LAN_IP}:28766"
DEVCTL_PORT=28770
FEATURES="--features dev-control"
ACTION=build

while [ $# -gt 0 ]; do
    case "$1" in
        --gateway) GATEWAY="$2"; shift 2 ;;
        --devctl) DEVCTL_PORT="$2"; shift 2 ;;
        --no-dev-control) FEATURES=""; DEVCTL_PORT=""; shift ;;
        --run) ACTION=run; shift ;;
        --release) PROFILE="--release"; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

echo "gateway      ${GATEWAY}"
echo "dev-control  ${DEVCTL_PORT:-off}"
echo

# `cargo apk run` also installs and starts it, and applies the
# `reverse_port_forward` table from the android package's Cargo.toml.
BAYLEE_GATEWAY="$GATEWAY" \
BAYLEE_DEV_CONTROL="${DEVCTL_PORT}" \
    cargo apk "$ACTION" -p baylee-client-android ${FEATURES} ${PROFILE:-}

APK="$(ls -t target/debug/apk/*.apk target/release/apk/*.apk 2>/dev/null | head -1 || true)"
if [ -n "$APK" ]; then
    echo
    echo "APK: $APK"
    echo
    echo "  install by hand   adb install -r $APK"
    if [ -n "$DEVCTL_PORT" ]; then
        echo "  reach dev-control adb forward tcp:28773 tcp:${DEVCTL_PORT} && curl -s localhost:28773/health"
    fi
    echo "  let it dial here  adb reverse tcp:28766 tcp:28766"
fi
