# Fixtures for the `seen` tests

## The golden tables: `table-40.json`, `table-end.json`

`seen`'s encoding is pinned on two public tables (`seen::PublicTable`, plain numbers): the table after the 40th input of the original acceptance game at seed 3, and the table at its end. Their digests are the original golden values, `30b17ab8dd2e7087` and `deb49d0b74b15d90`.

They were captured on 3.11.2026 by replaying the original `acceptance-seed3.jsonl` exactly on origin/main `5819631ae`, the last engine that played it. That record was captured from `record::tests::played(3)` on origin/main `49c9a4d8f941be56958c779e4d23c4daa95f9b78` (original SHA256 `27bebb34b190759683b4823f52db6ba16f3de018e2e2bb0c398f4def7142be70`). Its hashes were refreshed five times before; the last SHA256 is `5d812f46e01ebc6511e5e2fd1d7fd62d34d6db903dae37b62c986233c3f70b2b`. Each refresh changed only the 16-hex `hash` values, with every `seen` and the finish checked.

The capture was needed because that game became obsolete. Seat 1 started it, and the engine let seat 1 draw on turn 1. CR 103.8a skips the starting player's first draw whichever seat starts, and fixing that changed the game from its third input on (and the opening miracle check its header hashed). As the earlier version of this file asked, the public-table encoding fixture was isolated rather than a new golden made from the current house AI. The tables are what `PublicTable::of` read.

**Never edit these files or the golden literals.** A change to the encoding is a new field (`seen2`) beside `seen`.

## The acceptance record: `acceptance-seed3.jsonl`

This is a game played by the house from seed 3 on the current engine, for the tests that change a table and watch the digest move or stay (`what_the_table_shows_moves_seen`, the library swap, the battlefield order, the clone). It pins no value. The tests replay it with `replay_exact`, so every engine hash is checked; its header names the build `test`.

When the owner has decided that the state hash changes, `refresh_fixture_hashes` (an ignored test in `seen.rs`) rewrites the `hash` values and checks every `seen` and the finish on the way. It is never the way past a red test after adding a feature: a rule that is new writes nothing into a game that never meets it, and a red `replay_exact` here means that promise broke (`docs/engine-internals.md`, "The replay contract"). When a rule fix makes the game diverge, `capture_acceptance_record` (ignored too) writes it again from `played(3)`. Either way, update the SHA256 below.

Current SHA256: `6ebc204d3c827c575f26ca4630cce338222909e8c298d84c9f2012830609fe68`, refreshed 13.11.2026 on branch `agy/mirrodin-2`, when the hash began including each player's next-spell cost reduction; every public `seen` digest and the finish stayed unchanged. Captured 3.11.2026 on branch `c42/llm-report-fixes-20261103`.

The games and names are synthetic repository test data, not a player record.
