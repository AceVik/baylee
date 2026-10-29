# Fixed synthetic acceptance record for the seen golden tests

`acceptance-seed3.jsonl` was captured from `record::tests::played(3)` on origin/main commit `49c9a4d8f941be56958c779e4d23c4daa95f9b78`. The original finished-table golden assertion passed before the public-table export. Its 40th input also carries the original `30b17ab8dd2e7087` seen value; the final table carries `deb49d0b74b15d90`.

Original capture SHA256: `27bebb34b190759683b4823f52db6ba16f3de018e2e2bb0c398f4def7142be70`.

Current SHA256: `4205ef611c9b4308ca66e063581922f95fe95c6d88bec499c43e7a916615e9f5`. During the PR #399 review, the internal `hash` fields were refreshed after the engine state encoding changed. All 913 lines retain identical actions, metadata and `seen` values; replay checked every public digest and both golden tables before the fixture was replaced.

The game and names are synthetic repository test data, not a player record. The fixture holds the original preset and executed inputs so future House policy decisions cannot redefine either golden table. Encoding and both golden literals remain unchanged. Gameplay replay still checks every exact engine hash and real completion. If an intentional engine change makes those historical inputs obsolete, isolate the captured public-table encoding fixture; do not regenerate a new golden from the current House AI.
