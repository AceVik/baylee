#!/bin/zsh
# The rules half of the gate: everything except the client and wasm.
#
# `baylee-client` is a Bevy crate and dominates a full `--workspace` run. An
# engine or card change that rebuilt it was paying for a renderer it had not
# touched, which is most of a twenty-minute wait for a two-minute answer.
#
# Run this while working; run the full gate (fmt, clippy, test, wasm, all
# workspace) before pushing. This is a filter, not a replacement — it says
# nothing about the client, and CI runs everything.
set -e
cd "$(dirname "$0")/.."
cargo fmt --all
echo "STEP fmt ok"
cargo lint-rules
echo "STEP lint ok"
cargo test-rules
echo "STEP test ok"
echo "done rc=0"
