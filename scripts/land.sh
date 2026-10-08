#!/usr/bin/env bash
# Lands the checked-out commit on main: the three local gates, then a
# fast-forward push of exactly the commit they passed. One command for the
# rule in force since 08.10.2026 (docs/releasing.md §"Landing on main"): a
# change lands after the local gates and does not wait for CI; CI runs on the
# main push, and a release needs that run green.
#
#   DATABASE_URL=… scripts/land.sh           # gates, then push to origin/main
#   DATABASE_URL=… scripts/land.sh --check   # the same, without the push
#
# The gates: `gate.sh` (fmt, clippy, every test, validate), `gate-features.sh`
# (the non-default features) and `gate-wasm.sh` (the browser crates). All
# three run even when one fails, so one run reports every red. They run under
# the machine's cargo lock when there is one (BAYLEE_CARGO_LOCK, default
# ~/.baylee-locks/with-cargo-lock.sh): two sessions' gates side by side break
# timing-sensitive e2e tests by load alone.
#
# What it refuses, before spending a gate's minutes on it:
# - tracked changes not committed (the gates would measure something else);
# - a HEAD that is not a fast-forward of origin/main. It never rebases or
#   merges for you, and never forces: rebase, then run it again.
# And again after the gates, because main may have moved meanwhile: the push
# names the gated commit, and git refuses it if it is no fast-forward.
set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "land: $*" >&2; exit 1; }

if [ "${1:-}" = --gates ]; then
    # The part that runs under the lock.
    rc=0
    for gate in gate.sh gate-features.sh gate-wasm.sh; do
        t=$SECONDS
        if "scripts/$gate"; then r=0; else r=$?; fi
        echo "LAND $gate rc=$r $((SECONDS - t))s"
        [ "$r" -eq 0 ] || rc=1
    done
    exit "$rc"
fi

push=true
case ${1:-} in
    --check) push=false ;;
    "") ;;
    *) die "usage: scripts/land.sh [--check]" ;;
esac
: "${DATABASE_URL:?set DATABASE_URL - gate.sh and gate-features.sh need Postgres}"

clean() {
    [ -z "$(git status --porcelain --untracked-files=no)" ] \
        || die "uncommitted changes to tracked files; commit or set them aside first"
}
fast_forward() {
    git fetch --quiet origin main
    local base
    base=$(git rev-parse origin/main)
    [ "$1" != "$base" ] || die "nothing to land: $1 is origin/main"
    git merge-base --is-ancestor "$base" "$1" \
        || die "$1 is not a fast-forward of origin/main ($base): rebase onto origin/main and run again"
}

branch=$(git symbolic-ref --quiet --short HEAD || echo "(detached)")
head=$(git rev-parse HEAD)
clean
fast_forward "$head"
untracked=$(git ls-files --others --exclude-standard -- crates xtask Cargo.toml Cargo.lock | head -5)
[ -z "$untracked" ] || echo "land: note - untracked files beside the code (not part of the commit): $untracked" >&2
echo "land: gating $head ($branch) on top of origin/main"

t0=$SECONDS
lock=${BAYLEE_CARGO_LOCK:-$HOME/.baylee-locks/with-cargo-lock.sh}
if [ -x "$lock" ]; then
    BAYLEE_SESSION=${BAYLEE_SESSION:-land $branch} "$lock" "$0" --gates || die "a gate is red; nothing pushed"
else
    "$0" --gates || die "a gate is red; nothing pushed"
fi
echo "land: gates green in $((SECONDS - t0))s"

[ "$(git rev-parse HEAD)" = "$head" ] || die "HEAD moved during the gates; run again"
clean
fast_forward "$head"
if ! $push; then
    echo "land: --check: $head would land; not pushed"
    exit 0
fi
# Never --force: a push that is no fast-forward is refused by git itself.
git push origin "$head:refs/heads/main"
echo "land: main is $head; CI runs on it (a release needs that run green)"
