#!/bin/bash
# Every feature this workspace declares and `--workspace` never builds.
#
# `cargo clippy --workspace --all-targets`, `cargo test --workspace
# --all-targets` and CI's jobs for both compile each crate with its
# **default** features. Six non-default ones are declared here:
#
#   baylee-client          dev-control, dev-reload, dev-dylink
#   baylee-client-android  dev-control
#   baylee-gateway         dev-table
#   baylee-client-core     test-support
#
# so six pieces of this workspace were compiled by nobody. It fails in
# **both** directions, which is the half that is easy to miss. A feature adds
# code the default build never sees — `devctl.rs` broke on `Option<Refusal>`
# with the whole gate green, and the commit had to be pulled back out of a
# push (#121). And a feature makes *other* code live that the default build
# reads as dead, so `-D warnings` fires on an unused import only once the
# feature is on.
#
# `--all-features` is deliberately not the shortcut. `dev-dylink` changes how
# the binary links and `dev-reload` changes how assets are loaded; all-on
# measures a configuration nobody runs while hiding the two that matter. What
# this runs instead is, per feature, the *shape* of check that feature earns,
# and each one says below why it is that shape.
#
# bash rather than zsh, unlike `gate-rules.sh`: this file is also CI's
# `features` job, and the runner image is not this laptop.
#
# Usage:  DATABASE_URL=… scripts/gate-features.sh
set -u
cd "$(dirname "$0")/.."

# The number that lets the test step *refuse* the thing this whole file exists
# for. `cargo test` passing says nothing on its own — it passes just as
# happily on a build where the feature quietly stopped applying and its tests
# stopped existing, which is a green run reporting nothing.
#
# It counts `devctl::` tests and not the suite, and that is the difference
# between a guard and a number that rots. Measured 20.09.2026 through
# `cargo test -p baylee-client --features dev-control,dev-reload --lib --
# --list`: **17** of them with the feature and **0** without it. A floor over
# the *suite* total cannot say that — it has to be re-anchored against
# whatever the default build currently runs, every time either number grows,
# and a floor that has drifted under the default is a check that passes on
# the failure it names. This one is anchored to the feature by construction,
# and zero is the exact signature of the failure. Proven rather than
# reasoned: run against a default build's output the predicate reports
# `0 devctl` and fails.
#
# Raise it when `devctl.rs` grows; never lower it to whatever came out.
DEVCTL_TEST_FLOOR=15

fail=0
log=$(mktemp)
trap 'rm -f "$log"' EXIT

# The status is read off the command and never off a pipeline. `cmd | tail`
# reports **tail's** exit code, so the obvious spelling of this helper prints
# ok for every failure it was built to catch; `${PIPESTATUS[0]}` would answer
# that in bash and not in the zsh the sibling scripts use. No pipe, no
# question.
step() {
    local label="$1"; shift
    local t0=$SECONDS
    if "$@" >"$log" 2>&1; then
        echo "STEP $label ok ($((SECONDS - t0))s)"
    else
        echo "STEP $label FAILED ($((SECONDS - t0))s)"
        tail -40 "$log"
        fail=1
    fi
}

# --- the client -------------------------------------------------------------
#
# `dev-control` and `dev-reload` in **one** invocation, which is not
# `--all-features` wearing a hat: it is the configuration `CLAUDE.md`
# documents for editing a shader in a running client, so it is a build
# somebody actually makes. It also buys the thing that dominates the cost
# here — `dev-reload = ["bevy/embedded_watcher"]` forwards a *bevy* feature,
# so every distinct client feature set is a separate bevy compile, and
# checking the two apart would pay for that twice to cover the same `cfg`
# sites. The union compiles a superset of either, so nothing is lost but the
# duplicate build.
CLIENT_FEATURES=dev-control,dev-reload

step client-features-clippy \
    cargo clippy -p baylee-client --features "$CLIENT_FEATURES" --all-targets -- -D warnings

# The one step that counts rather than merely passing. See the floor above.
t0=$SECONDS
if cargo test -p baylee-client --features "$CLIENT_FEATURES" --all-targets >"$log" 2>&1; then
    passed=$(grep -Eo '^test result: ok\. [0-9]+ passed' "$log" | awk '{s += $4} END {print s + 0}')
    devctl=$(grep -c '^test devctl::' "$log")
    if [ "$devctl" -ge "$DEVCTL_TEST_FLOOR" ]; then
        echo "STEP client-features-test ok ($passed passed, $devctl of them devctl::, floor $DEVCTL_TEST_FLOOR, $((SECONDS - t0))s)"
    else
        echo "STEP client-features-test FAILED - $passed tests passed and only $devctl of them were"
        echo "  devctl:: tests, under the floor of $DEVCTL_TEST_FLOOR. Zero means the feature did not apply and"
        echo "  the whole harness compiled out, which is a green suite measuring the default"
        echo "  build under another name. A number short of the floor but not zero means tests"
        echo "  were removed: re-measure and move the floor in a commit that says the number."
        fail=1
    fi
else
    echo "STEP client-features-test FAILED ($((SECONDS - t0))s)"
    tail -40 "$log"
    fail=1
fi

# `dev-dylink` gates **no source code at all** — there is not one
# `cfg(feature = "dev-dylink")` in the tree. It is `["bevy/dynamic_linking"]`
# and nothing else, so the only thing it can break is the feature graph, and
# `cargo tree` answers that in under half a second against the two-minute
# bevy build a `clippy` would pay for. Proven able to fail rather than
# assumed: forwarding to a `dynamic_linking_NOPE` that does not exist makes
# this exit 101.
#
# What it therefore does *not* check is the link itself, which is the thing
# the feature is for. That is the trade, said out loud: a developer's
# convenience feature does not earn a bevy rebuild and a dylib link on every
# push, and `cargo clippy -p baylee-client --features dev-dylink` is one word
# away for anyone who wants it.
#
# A failure here is worth one moment's suspicion before it is believed. Both
# of these crates were once written up as impossible at bevy 0.19.1 with a
# resolver error to prove it, and the cause was a year-stale local registry
# index cache; `crates/baylee-client/Cargo.toml` carries that story beside the
# feature.
step dev-dylink-resolves \
    cargo tree -p baylee-client --features dev-dylink -e features

# --- the other three --------------------------------------------------------
#
# Off Android this package has no dependencies at all — everything sits under
# `[target.'cfg(target_os = "android")'.dependencies]`, so the cdylib is an
# empty shared object and this costs no measurable time. What it earns is
# that the feature *forward* still resolves: `dev-control` here is
# `["baylee-client/dev-control"]`, naming a dependency that exists on one
# target only, and a rename on either side of that arrow is otherwise found
# on a phone.
step android-dev-control \
    cargo clippy -p baylee-client-android --features dev-control --all-targets -- -D warnings

# The gateway's seated dev table. Both arms matter and only one is ever
# built: `main.rs` carries `cfg(feature = "dev-table")` *and*
# `cfg(not(feature = "dev-table"))`, so the default gate compiles the refusal
# and never the thing it refuses.
if [ -n "${DATABASE_URL:-}" ]; then
    step dev-table-clippy \
        cargo clippy -p baylee-gateway --features dev-table --all-targets -- -D warnings
    step dev-table-test \
        cargo test -p baylee-gateway --features dev-table --all-targets
else
    echo "STEP dev-table SKIPPED - no DATABASE_URL; the gateway suite needs PostgreSQL"
    echo "  (a gateway without one refuses to start, so this is not a check that can"
    echo "   degrade gracefully -- it is one that did not run)"
fi

# `--lib`, and that is the whole point of the line. `test-support` is
# `cfg(any(test, feature = "test-support"))`, so `--all-targets` compiles that
# code anyway through `test` and proves nothing about the feature. The shape
# nobody builds is the **library** with the feature on and `cfg(test)` off —
# which is how every downstream consumer of it is built, and where an item
# reaching for something only `cfg(test)` provides would break.
step test-support \
    cargo clippy -p baylee-client-core --features test-support --lib -- -D warnings

if [ "$fail" -ne 0 ]; then
    echo "done rc=1"
    exit 1
fi
echo "done rc=0"
