#!/usr/bin/env bash
# Signs every release archive in a directory, then checks that the client
# would accept each signature (#326, docs/releasing.md §"Signing").
#
#   scripts/release/sign-archives.sh <dir>
#
# Writes `<archive>.sig` beside every `.zip` and `.tar.gz` in <dir>, with
# the seed in BAYLEE_UPDATE_SIGNING_KEY (the Actions secret: base64 of 32
# bytes). Then verifies each against the keys compiled into the client, so
# a secret that is not the client's key fails here, in the release, and not
# in every player's updater.
#
# Without the secret: on a tag (GITHUB_REF_TYPE=tag) this fails, because a
# release is never published unsigned again; on a dry run it says so and
# signs nothing.
#
# BAYLEE_SIGNER is the signing program (default: `cargo run` of
# baylee-update-sign). BAYLEE_UPDATE_VERIFY_KEY replaces the compiled keys
# for the verification; only `crates/baylee-update/tests/sign_script.rs`
# sets it, and the release workflow never does.
set -euo pipefail

dir=${1:?usage: sign-archives.sh <dir>}
signer=${BAYLEE_SIGNER:-cargo run --locked -q -p baylee-update --bin baylee-update-sign --}

archives=()
for f in "$dir"/*.zip "$dir"/*.tar.gz; do
    [ -f "$f" ] && archives+=("$f")
done
if [ ${#archives[@]} -eq 0 ]; then
    echo "::error::no release archive in $dir"
    exit 1
fi

if [ -z "${BAYLEE_UPDATE_SIGNING_KEY:-}" ]; then
    if [ "${GITHUB_REF_TYPE:-}" = tag ]; then
        echo "::error::BAYLEE_UPDATE_SIGNING_KEY is not set; a tag never publishes unsigned archives"
        exit 1
    fi
    echo "::warning::BAYLEE_UPDATE_SIGNING_KEY is not set; this dry run signs nothing"
    exit 0
fi

# Word splitting on purpose: the default is a command with arguments.
# shellcheck disable=SC2086
$signer sign "${archives[@]}"

verify=()
if [ -n "${BAYLEE_UPDATE_VERIFY_KEY:-}" ]; then
    verify=(--key "$BAYLEE_UPDATE_VERIFY_KEY")
fi
# `${verify[@]+…}`: an empty array is "unbound" to bash 3.2 under `set -u`.
# shellcheck disable=SC2086
$signer verify ${verify[@]+"${verify[@]}"} "${archives[@]}"

for f in "${archives[@]}"; do
    if [ ! -s "$f.sig" ]; then
        echo "::error::$f has no signature"
        exit 1
    fi
done
echo "signed and verified ${#archives[@]} archives"
