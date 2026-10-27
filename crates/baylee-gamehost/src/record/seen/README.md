# Fixed synthetic acceptance record for the seen golden tests

`acceptance-seed3.jsonl` was captured from `record::tests::played(3)` on origin/main commit `49c9a4d8f941be56958c779e4d23c4daa95f9b78`. The original finished-table golden assertion passed before the public-table export. Its 40th input also carries the original `30b17ab8dd2e7087` seen value; the final table carries `deb49d0b74b15d90`.

SHA256: `27bebb34b190759683b4823f52db6ba16f3de018e2e2bb0c398f4def7142be70`.

The game and names are synthetic repository test data, not a player record. The fixture holds the original preset and executed inputs so future House policy decisions cannot redefine either golden table. Encoding and both golden literals remain unchanged. Gameplay replay still checks every exact engine hash and real completion. If an intentional engine change makes those historical inputs obsolete, isolate the captured public-table encoding fixture; do not regenerate a new golden from the current House AI.
