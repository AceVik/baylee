# BayleeAI v5: round 9 review, 29 October 2026

Round 9 is not promoted. Its final checkpoint won 15 of 64 Constructed games
against Sharp, versus 21 for round 6, 19 for round 8, and 15 for the deployed
checkpoint. The round-9 checkpoint after the LLM mixture stage won 13 games.

| Checkpoint | Wins / 64 | House fallback answers | Games with a fallback |
| --- | ---: | ---: | ---: |
| R6 final | 21 | 21 | 3 |
| R8 final | 19 | 9 | 3 |
| Deployed | 15 | 8 | 2 |
| R9 LLM stage | 13 | 9 | 2 |
| R9 final | 15 | 21 | 5 |

The comparison retained every case, with no caps, refusals or evaluator stalls.
It used the same 32 table clusters, both focal-seat rotations, deck manifest,
starting seeds, pinned engine, raw K=2 policy and Sharp opponents. These are
repeated development cases, not a new holdout or head-to-head games. Later
draws and states can diverge when actions differ. Fallbacks remain included;
their causes were not recorded by this pinned evaluator.

Against R6, R9 final improved three outcomes and worsened nine; 52 were equal.
The paired utility difference was -9.375 percentage points, with a descriptive
table-bootstrap 95% interval of [-20.3125, 0] percentage points. Repeated use
for model selection prevents treating this as an unbiased final strength test.
No causal conclusion about the quality or usefulness of LLM data follows.

The independent audit rehashed 737 distinct files, verified 725 run input pins,
32 new and 48 reused native jobs, reconstructed all five aggregates and
recomputed all seven paired reports. Evidence:
`/home/ace/baylee-data/v5/ops-codex-20261028/review-round9/completed-audit.json`.
The monitor event `cpu_review:de4f6f1c647ab4b0:terminal` was recorded as reviewed.

## Follow-up underway

A separate CPU-only diagnostic compares R6 final, R9 LLM stage and R9 final
on all 7,608 frames of the same five held-out LLM games. It preserves complete
seat histories and uses identical FP32, B=8, U=32 and K=2 settings. Reports
separate categorical pass/active targets, pending question types, policy
heads and games. Subset threshold agreement does not establish decoded action
legality. This small dataset measures imitation behavior, not playing strength.
Its physical `train` directory does not admit these records to training.

The bounded diagnostic has an identity-bound completion/failure monitor and
retains its inputs and logs under
`/home/ace/baylee-data/v5/ops-codex-20261028/diagnose-round9/`.
It was started alongside the existing round-10 GPU controller. The current
GPU run remains limited through round 12, and the user gateway is unchanged.

The mixture schedule needs a controlled follow-up: R9 used 6,202 LLM mixture
updates on 383 games, while R10 selected a fully occupied prefix of 1,124
updates on 546 games with a different teacher package. Half the trajectory
lanes does not imply half the labels, equal gradient mass or complete coverage.
Further learning comparisons must freeze data, initialization and update
budgets. A proposal to isolate value/auxiliary gradients is recorded alongside
the diagnostic; it has not been launched or selected as a proven remedy.

## Job cleanup correction

The job guard previously reaped its leader through `poll()` before signalling
the old process-group ID during cleanup. It now observes exit with `WNOWAIT`,
retains ownership until remaining workers stop, and then reaps the leader.
Lost ownership prevents group signals. Interruptions and cleanup failures
retain quarantined manifests. Eighteen focused process and digest-cache tests
passed, including deadline, memory limit, SIGTERM and a grandchild ignoring
SIGTERM. The new diagnostic uses a frozen copy of this fix; already-running
training code was not changed. Commit: `66294d993`. Full landing gates and a
push were not performed for this review.

## Completed policy diagnostic and mixture coverage review

The three CPU diagnostics completed with all 7,608 frames and ten complete
seat histories from the same five games. Independent verification checked
209 input pins, all three successful job/report seals, 129 group metrics and
450 additive partition checks.

| Checkpoint | Exact categorical agreement / 1,030 | Exact pass-target agreement / 314 | Exact active-target agreement / 716 | Active-target NLL |
| --- | ---: | ---: | ---: | ---: |
| R6 final | 538 | 201 | 337 | 1.5336 |
| R9 LLM stage | 594 | 246 | 348 | 1.3388 |
| R9 final | 602 | 265 | 337 | 1.4166 |

All 64 net additional correct categorical choices between R6 and R9 final
come from pass targets. Active agreement is unchanged, although active-target
NLL improves. This does not establish excessive passing: these original
metrics did not count pass predictions when the recorded target was active.

Commit `cbcd05bfa` adds that confusion matrix to diagnostics, including a
separate distinction between selecting any pass option and the exact recorded
pass option. Existing metrics and training behavior remain unchanged. Thirteen
CPU tests passed, covering masked candidates, forced/zero-weight/padded
targets, multiple pass options, aggregation and empty denominators. A fresh
CPU-only run under `ops-codex-20261028/diagnose-round9-pass-confusion/` repeats
the identical histories and checkpoints. It requires all original diagnostic
values to match the preceding reports within numerical tolerance.

An index-only audit also reconstructed the mixture schedule. R9's limiting
LLM lane had 6,202 batches; R10's limiting teacher lane had only 1,124. The
R10 teacher half contains eight games and 145,096 frames. Its LLM source has
546 games and could sustain 8,182 fully occupied batches, but the common
prefix touches only 147 of those games; 399 are untouched in that stage.
This is shuffled trajectory scheduling, not a semantic quality filter or
proof that those games were never used in earlier rounds. Planned exposure
is distinct from a log of successfully consumed optimizer batches.

The previously proposed 256-update gradient-detach pilot is deferred: its
schedule would touch only five teacher games and 33 LLM games, and the
diagnosis does not identify gradient interference as the cause. The next
comparison design instead fixes initialization, a broader teacher source,
LLM source and common full-coverage update budget before comparing teacher
only with the LLM mixture. Existing `--bc-epoch-batches` support can repeat
exhausted sources to cover both complete first passes; repetitions, padding
and effective label counts must be reported. No new GPU experiment has been
started and no checkpoint has been promoted.

Evidence and the revised design are recorded in
`ops-codex-20261028/diagnose-round9/completed-audit.json`,
`mixture-exposure-audit.json` and `next-learning-decision-20261029.json` under
`/home/ace/baylee-data/v5/`. Event
`cpu_review:aedc48a0864e382e:terminal` is reviewed and deduplicated.
