#!/bin/zsh
# The full gate before a push, as CI runs it: fmt, clippy, every test through
# nextest, and `validate`. Every phase runs even when an earlier one failed, so
# one run reports every red, and each prints its seconds.
#
# Needs Postgres for the catalog and gateway tests: `docker compose up -d` and
# DATABASE_URL (CLAUDE.md). `gate-features.sh` covers the non-default features.
set -u
cd "$(dirname "$0")/.."
: ${DATABASE_URL:?set DATABASE_URL - the catalog and gateway tests need Postgres}

# Every rustc lists target/debug/deps when it starts. On macOS that directory
# once held 1.3 M stale objects and made a gate take 13 minutes; the rustflags
# in .cargo/config.toml stop that, and this says so if it comes back.
if [[ $OSTYPE == darwin* && -d target/debug/deps ]]; then
    size=$(stat -f %z target/debug/deps)
    if (( size > 4000000 )); then
        echo "WARN target/debug/deps is ${size} bytes of directory entries - every rustc lists it."
        echo "     Sweep it: mv target/debug target/sweep-\$(date +%s) && (taskpolicy -b rm -rf target/sweep-* &)"
    fi
fi

rc=0
step() {
    local name=$1; shift
    local t=$SECONDS
    "$@"
    local r=$?
    echo "STEP $name rc=$r $((SECONDS - t))s"
    (( r == 0 )) || rc=1
}

step fmt cargo fmt --all --check
step clippy cargo clippy --workspace --all-targets -- -D warnings
step test cargo nextest run --workspace --all-targets --no-fail-fast
if [ -d data/scryfall-cache ]; then
    step validate cargo run -q -p xtask -- validate
else
    echo "STEP validate SKIPPED - no data/scryfall-cache"
fi
echo "done rc=$rc"
exit $rc
