#!/usr/bin/env bash
# The browser build's crates, checked for wasm32: CI's `wasm` job and the third
# local gate before landing (`scripts/land.sh`), one command for both.
#
# The browser client compiles nothing on a native build: its localStorage
# settings and Web Crypto entropy live behind `cfg(target_arch = "wasm32")` and
# would rot unnoticed without a target-specific check.
#
# All seven crates are named rather than the client alone, and that is not
# redundancy: cargo unifies features across the members a command selects, so
# `-p baylee-client` resolves the other six with whatever the client happens
# to ask for. Naming them is what checks that each one still compiles for this
# target on its *own* default features, which is the promise CLAUDE.md's
# architecture section makes about them.
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo check --locked \
    -p baylee-cardtext \
    -p baylee-core \
    -p baylee-deckio \
    -p baylee-protocol \
    -p baylee-view \
    -p baylee-client-core \
    -p baylee-client \
    --target wasm32-unknown-unknown
