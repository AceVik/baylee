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
# - a HEAD that cannot be merged with origin/main without a conflict
#   (checked with `git merge-tree`, nothing written): resolve, then run again.
#
# Landing is serial (09.10.2026). Everything from the merge to the push runs
# under the cargo lock: once the lock is ours, origin/main is fetched again,
# and if it moved while we waited, it is merged into HEAD (`git merge
# --no-edit`, a merge commit; a conflict aborts the merge and stops). Then
# the gates run on that commit and it is pushed, still under the lock. So
# the next lander, waiting on the same lock, gates on top of this push
# instead of racing it: before this, two landers gated side by side, the
# second went green and was then refused as "not a fast-forward", 25 minutes
# lost per collision. A push that did not come through land.sh can still
# move main during the gates; git then refuses the push (never --force), and
# the run says so. `--check` never merges: it gates HEAD as it is.
set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "land: $*" >&2; exit 1; }

gates() {
    rc=0
    for gate in gate.sh gate-features.sh gate-wasm.sh; do
        t=$SECONDS
        if "scripts/$gate"; then r=0; else r=$?; fi
        echo "LAND $gate rc=$r $((SECONDS - t))s"
        [ "$r" -eq 0 ] || rc=1
    done
    return "$rc"
}

push=true
locked=false
for arg in "$@"; do
    case $arg in
        --check) push=false ;;
        --locked) locked=true ;; # internal: the part that runs under the lock
        *) die "usage: scripts/land.sh [--check]" ;;
    esac
done
: "${DATABASE_URL:?set DATABASE_URL - gate.sh and gate-features.sh need Postgres}"

clean() {
    [ -z "$(git status --porcelain --untracked-files=no)" ] \
        || die "uncommitted changes to tracked files; commit or set them aside first"
}
behind() {
    # Fetches origin/main; true when HEAD does not already contain it.
    git fetch --quiet origin main
    ! git merge-base --is-ancestor origin/main HEAD
}
mergeable() {
    git merge-tree --write-tree origin/main HEAD >/dev/null 2>&1 \
        || die "HEAD conflicts with origin/main ($(git rev-parse --short origin/main)): merge or rebase, resolve, and run again"
}

branch=$(git symbolic-ref --quiet --short HEAD || echo "(detached)")

if $locked; then
    # Under the lock: catch up with main, gate, push. Nobody else landing
    # through this script moves main in between.
    clean
    if $push && behind; then
        mergeable
        git merge --quiet --no-edit origin/main \
            || { git merge --abort 2>/dev/null; die "merging origin/main failed; nothing pushed"; }
        echo "land: merged origin/main ($(git rev-parse --short origin/main)) into $branch"
    fi
    head=$(git rev-parse HEAD)
    [ "$head" != "$(git rev-parse origin/main)" ] || die "nothing to land: $head is origin/main"
    echo "land: gating $head ($branch)"
    t0=$SECONDS
    gates || die "a gate is red; nothing pushed"
    echo "land: gates green in $((SECONDS - t0))s"
    [ "$(git rev-parse HEAD)" = "$head" ] || die "HEAD moved during the gates; run again"
    clean
    if ! $push; then
        echo "land: --check: $head passed the gates; not pushed"
        exit 0
    fi
    # Never --force: a push that is no fast-forward is refused by git itself.
    git push origin "$head:refs/heads/main" \
        || die "main moved during the gates by a push outside land.sh; run again"
    echo "land: main is $head; CI runs on it (a release needs that run green)"
    exit 0
fi

clean
behind || [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ] \
    || die "nothing to land: HEAD is origin/main"
if $push; then mergeable; fi
untracked=$(git ls-files --others --exclude-standard -- crates xtask Cargo.toml Cargo.lock | head -5)
[ -z "$untracked" ] || echo "land: note - untracked files beside the code (not part of the commit): $untracked" >&2

args=(--locked)
$push || args+=(--check)
lock=${BAYLEE_CARGO_LOCK:-$HOME/.baylee-locks/with-cargo-lock.sh}
if [ -x "$lock" ]; then
    BAYLEE_SESSION=${BAYLEE_SESSION:-land $branch} exec "$lock" "$0" "${args[@]}"
else
    exec "$0" "${args[@]}"
fi
