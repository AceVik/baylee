# Corrective BC review and complete-source training, 27 October 2026

The completed 1500-update mixed-data pilot does not establish a benefit from
the additional Sharp labels. All three models were evaluated again with one
corrected engine, identical starts, focal-seat rotations and opponent profiles.
These are policy substitutions against Sharp, not direct model-versus-model
matches. Steady is a repeated baseline against the same opponents.

| Model | Duel wins / 64 | Commander wins / 32 | Explicit house fallbacks, duel / Commander |
| --- | ---: | ---: | ---: |
| Broad selected source | 12 | 1 | 21 / 8 |
| Old-data control, fixed final 1500 | 16 | 2 | 39 / 4 |
| Corrective mixture, fixed final 1500 | 15 | 2 | 44 / 10 |
| Steady baseline | 31 | 10 | — |

All six reports have zero draws, decision caps, refused actions and stalls.
The explicit house fallbacks are decisions on the neural seat; raw mode does
not mean every answer came from the neural sampler. Augmented versus control
has one better/two worse duel outcomes and two better/two worse Commander
outcomes. Its table-bootstrap 95% intervals are respectively
[-0.0625, 0.03125] and [-0.09375, 0.09375]. Commander has only eight independent
tables. All bounded independent-table intervals include zero. No model was
promoted, and the reused development decks do not establish general strength.

The comparison initially failed on Night Soil: a cost menu offered cards from
multiple payable graveyards without expressing the single-graveyard condition.
The engine now requests one card to select the graveyard, then the remaining
cards from that graveyard. A single payable graveyard keeps its counted batch.
Six targeted tests and the exact previously crashing deal (seed 500291005,
focal seat 0) pass. All six gameplay reports were rerun together. Older
multi-graveyard batch recordings still require their original frozen runtime.

A separate execution audit identified grouped-attack canonicalization in the
raw-action verification path. A legal choice of the second identical creature
could be re-encoded as the first and incorrectly sent to the house fallback.
An exact executed-action encoder now preserves subset members and defenders,
both in the raw guard and conversion of explicit neural sidecars. Real session
and replay regressions check masks, free masks, selection order and behavior
probabilities; genuine decoder repairs remain rejected. BC teacher targets
remain canonical. Their independent per-defender canonicalization has a
separate potential collision; an audit of the admitted training data found
zero selected entity/rank collisions in 6893 old and 1641 corrective active
attack frames. Cached ranks lack original object IDs, so this is not a
certificate of all engine legality. Newly corrected evaluators and converters
must be frozen together before new collection or evaluation.

The original bounded training consumed only three of the 45 new MulliganTake
labels. The next experiment therefore uses one complete corrective source
pass: 17261 updates, 2197633 new observation/decision frames and 99883 teacher
labels, including all 207 Keep and 45 Take labels once. Shorter old sources
cycle through chronological trajectories instead of leaving half the batch
empty after about 8089 batches. Whole trajectory resets and passive observations
are preserved. Both arms share the exact old-left schedule. Source repetition,
tail padding and the final partial repeat are explicitly recorded; physical
half-batches do not imply equal label or gradient mass.

Both arms start from the same broad-selected v5 checkpoint and fresh
optimizers. They use loss5 with outcome weighting disabled and per-head
normalization across the unroll. The outcome of a neural actor's original
action is not evidence for the counterfactual Sharp recommendation. This is a
combined recipe change relative to the short pilot, not an isolated test of
normalization. Architecture, learning rates, value/auxiliary coefficients,
B8/U32/K2 and the 769611-frame validation remain fixed. Full validation runs
initially, every 3000 updates and at the fixed final step. Selected and final
checkpoints are retained separately; playing strength needs subsequent matched
games. The controller runs the two arms sequentially, preserves failures and
does not promote a model automatically.

Local evidence roots under `/home/ace/baylee-data/v5/`:

- `experiments/shadow-mixed-bc-eval-recovery2-20261027/`: completed gameplay,
  six independently recomputed paired reports, corrected evaluator identity.
- `advice/shadow-mixed-bc-complete-review-20261027/`: independent completion audit.
- `advice/grouped-raw-action-fix-20261027/`: failing-before and passing-after
  regressions, targeted policy tests and Clippy checks.
- `advice/grouped-attack-label-audit-20261027/`: bounded interpretation of
  cached attack-label checks.
- `experiments/shadow-full-coverage-20261027/`: new frozen Python runtime,
  coverage proof, sealed inputs, sequential training controller and watcher.

Twenty-two CPU sampler/loss/driver tests pass. The existing BF16 loss4 versus
legacy-loss5 update comparison fails its parameter tolerance identically before
and after the sampler change (2/1024 values, maximum absolute difference
9.77e-5); no tolerance was relaxed and CUDA bit parity is not claimed. Both
new arms use the same loss5 execution path.
