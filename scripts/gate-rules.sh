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

# `validate` is part of this gate because CI runs it as its own job and the
# rules gate did not: the card files compile, every test passes, and the one
# check that reads a card against its *printing* was the one nothing local
# ran. Invasion of Ikoria carried a battle subtype no `SubtypeKind` had for
# six commits that way.
#
# It needs the Scryfall payload cache, which is nobody's to commit
# (`docs/legal.md` §3), and a checkout that has never run `codegen` has none.
# That is said out loud rather than skipped in silence: a missing cache makes
# this step report what it is missing, and `PRINTING_FLOOR` is what fails the
# run if the cache is there but has quietly shrunk.
#
# And `validate` itself prints how old that cache is, because this step does
# not close the gap it looks like it closes. `fetch_named` answers from disk
# and never refetches, so a developer validates against a snapshot while CI
# starts cold and validates against live Scryfall. That is not a detail: CI's
# `validate` has been red since 0f450764 over 32 headers Scryfall moved when
# it shipped a set, and this gate is green on the same tree (#50). The tool
# reports the age rather than the script, so a bare
# `cargo run -p xtask -- validate` says it too.
if [ -d data/scryfall-cache ]; then
    cargo run -q -p xtask -- validate
    echo "STEP validate ok"
else
    echo "STEP validate SKIPPED - no data/scryfall-cache; run 'cargo xtask codegen' or 'scryfall-cache'"
fi
echo "done rc=0"
