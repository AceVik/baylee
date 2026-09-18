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

# `validate` is part of this gate because leaving it out cost six commits.
# CI runs it as its own job, the rules gate did not, and `main` sat red from
# 3a851e8d — the commit that added the one card with a battle subtype, the
# header check (f774bd28) being a week older than it — with nobody the
# wiser: the card files compiled, every test passed, and the one check that
# reads a card against the *printing* was the one nothing local ran. A gate
# that is missing the check a job fails on is a gate that reports on
# something else.
#
# It needs the Scryfall payload cache, which is nobody's to commit
# (`docs/legal.md` §3), and a checkout that has never run `codegen` has none.
# That is said out loud rather than skipped in silence: a missing cache makes
# this step report what it is missing, and `PRINTING_FLOOR` is what fails the
# run if the cache is there but has quietly shrunk.
if [ -d data/scryfall-cache ]; then
    cargo run -q -p xtask -- validate
    echo "STEP validate ok"
else
    echo "STEP validate SKIPPED - no data/scryfall-cache; run 'cargo xtask codegen' or 'scryfall-cache'"
fi
echo "done rc=0"
