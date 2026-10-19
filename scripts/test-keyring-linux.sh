#!/usr/bin/env bash
# An actual Secret Service round trip with no access to the user's keyring.
set -euo pipefail

if [[ ${1:-} == --inside ]]; then
    shift
    : "${BAYLEE_KEYRING_FIXTURE:?isolated fixture required}"
    [[ $HOME == "$BAYLEE_KEYRING_FIXTURE" ]]
    gnome-keyring-daemon --foreground --components=secrets --unlock \
        --control-directory="$HOME/control" < <(printf '\n') &
    keyring_pid=$!
    cleanup_provider() {
        [[ -n ${keyring_pid:-} ]] || return 0
        kill "$keyring_pid" 2>/dev/null || true
        for ((i = 0; i < 100; i++)); do
            kill -0 "$keyring_pid" 2>/dev/null || break
            sleep 0.1
        done
        if kill -0 "$keyring_pid" 2>/dev/null; then
            kill -KILL "$keyring_pid" 2>/dev/null || true
        fi
        wait "$keyring_pid" 2>/dev/null || true
        keyring_pid=
    }
    trap cleanup_provider EXIT INT TERM
    # Wait for the service name, without reading a credential or opening a UI.
    ready=false
    for ((i = 0; i < 100; i++)); do
        if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
            --method org.freedesktop.DBus.NameHasOwner org.freedesktop.secrets | grep -q true; then
            ready=true
            break
        fi
        kill -0 "$keyring_pid"
        sleep 0.1
    done
    [[ $ready == true ]]
    cargo test --locked -p baylee-keyring --lib "$@" \
        platform::tests::isolated_secret_service -- --ignored --exact
    exit
fi

command -v dbus-run-session >/dev/null
command -v gnome-keyring-daemon >/dev/null
command -v gdbus >/dev/null
fixture=$(mktemp -d -t baylee-keyring.XXXXXXXX)
cleanup_fixture() { rm -rf -- "$fixture"; }
trap cleanup_fixture EXIT INT TERM
# Keep toolchains/caches at their original locations while the daemon sees only
# the temporary home. dbus-run-session gives it a separate session bus as well.
export CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}
export RUSTUP_HOME=${RUSTUP_HOME:-$HOME/.rustup}
export HOME=$fixture BAYLEE_KEYRING_FIXTURE=$fixture
export XDG_DATA_HOME=$fixture/data XDG_CONFIG_HOME=$fixture/config
export XDG_CACHE_HOME=$fixture/cache XDG_RUNTIME_DIR=$fixture/run
mkdir -m 700 -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_RUNTIME_DIR" "$fixture/control"
timeout --kill-after=15s 120 dbus-run-session -- bash "$(realpath "$0")" --inside "$@"
