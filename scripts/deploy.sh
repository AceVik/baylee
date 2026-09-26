#!/usr/bin/env bash
# Deploys the committed HEAD to the production server by hand.
#
#   ./scripts/deploy.sh             # waits for open games to end
#   ./scripts/deploy.sh --force     # ends them
#   BAYLEE_DEPLOY_HOST=user@host ./scripts/deploy.sh
#
# The same thing happens by itself for a release: a timer on the server runs
# `baylee-deploy watch` every minute and stages the newest `v*` tag the first
# time it sees one. What a deploy does while games are open is described in
# `scripts/server/baylee-deploy`; in short, the agent and engine are swapped
# at once (no new games start), the gateway once the last game is over.
#
# The server builds for itself (x86_64, twelve cores, about two minutes), so
# nothing here cross-compiles. The source travels as a `git bundle`, not a
# tarball: `baylee-build` stamps the commit by asking git, and a tree with no
# `.git` says "unknown" in `/info`. After the first run a bundle carries only
# the commits the server does not have yet.
#
# Only commits are deployed. Uncommitted edits stay here, and the script says
# so rather than refusing, because the server builds exactly what `/info`
# will name.
set -euo pipefail

host=${BAYLEE_DEPLOY_HOST:-viktor@89.58.3.105}
src=/opt/baylee/src
force=
case ${1:-} in
    --force) force=--force ;;
    "") ;;
    *) echo "usage: $0 [--force]" >&2; exit 2 ;;
esac

cd "$(git rev-parse --show-toplevel)"
rev=$(git rev-parse HEAD)
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    echo "note: uncommitted changes are not deployed; building $(git rev-parse --short HEAD)" >&2
fi

if ! ssh "$host" "git -C $src cat-file -e $rev^{commit} 2>/dev/null"; then
    have=$(ssh "$host" "git -C $src rev-parse -q --verify HEAD 2>/dev/null || true")
    range=HEAD
    if [ -n "$have" ] && git merge-base --is-ancestor "$have" HEAD 2>/dev/null; then
        range="$have..HEAD"
    fi
    bundle=$(mktemp -t baylee-deploy)
    trap 'rm -f "$bundle"' EXIT
    git bundle create "$bundle" "$range" 2>/dev/null
    scp -q "$bundle" "$host:/tmp/baylee-deploy.bundle"
    ssh "$host" "git -C $src fetch -q /tmp/baylee-deploy.bundle HEAD && rm /tmp/baylee-deploy.bundle"
fi

# The server script is taken from the commit being deployed, so a change to
# how deploys work ships with the first deploy that needs it.
ssh "$host" "bash <(git -C $src show $rev:scripts/server/baylee-deploy) stage $rev $force"
