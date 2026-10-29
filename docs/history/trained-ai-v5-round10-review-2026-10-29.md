# V5 round 10 review and bounded continuation comparison

Round 10 completed successfully. The existing controller proceeded to round
11 and remains bounded through round 12. No second GPU controller was started;
the user gateway and deployed model are unchanged. Monitor event
`training:49efb950e5b82472:round-10` is reviewed and deduplicated.

## Training result

The three stages completed 1,500 teacher, 1,124 LLM-mixture and 995 live-mixture
updates. Final weights match their last checkpoints and are finite. The audit
verified three isolated validation jobs, their input/result bindings and
worker exit. Export covered K=0/1/2/4/8 with 79 parity frames and maximum
absolute error 2.003e-5. The highest logged GPU peak was 9.21 decimal GB and
all three stages reported zero allocator retries.

The comparison below uses the unchanged GPU/BF16 validation protocol and
compares the final mixed checkpoints, not intermediate or selected models.

| Metric | R9 final | R10 final |
| --- | ---: | ---: |
| Teacher categorical NLL | 0.25448 | 0.24996 |
| Teacher active agreement | 58.52% | 58.11% |
| Teacher value MSE | 0.14857 | 0.15676 |
| Five-game LLM categorical NLL | 1.12004 | 1.09680 |
| Five-game LLM active agreement | 46.65% | 45.95% |
| Five-game LLM value MSE | 0.10427 | 0.12810 |

Lower categorical NLL is accompanied by worse active agreement and value
prediction. This is not evidence of stronger play. The five LLM games remain
a repeatedly consulted development diagnostic. Round 10 uses 546 LLM games
versus 383 in round 9, a different teacher package and different update
budgets; this is not a causal LLM-data comparison.

## Completed pass-prediction diagnostic

The CPU/FP32 follow-up verified 211 input pins and all three completed jobs.
All 4,890 numeric values from the original diagnostic remain unchanged within
1e-6; group sums and new pass/active partitions agree.

| Checkpoint | Correct pass / 314 | Correct active / 716 | Predicted pass on active target / 716 |
| --- | ---: | ---: | ---: |
| R6 final | 201 | 337 | 23 |
| R9 LLM stage | 246 | 348 | 15 |
| R9 final | 265 | 337 | 30 |

R9 final improves recognition of recorded pass actions, while exact active
agreement remains unchanged. Its additional pass predictions on active
targets are a small part of the active-action disagreement. The intermediate
LLM stage actually makes fewer such predictions than R6. These five games
do not prove indiscriminate passing or optimality of recorded actions.

## Explicit limitation: prior card and deck reservations

A reservation audit found that 445 of the 546 complete LLM tables conflict
with the original held-card or test-deck policy. Of those tables, 314 contain
a seat deck with stable held-card-rule hits and 415 contain an exact reserved
deck match; the categories overlap. Deck matching normalizes printing metadata
and commander placement while retaining section counts. Examples include
Padeem with Sky Diamond and Tolaria West, and Sultai Graveyard with Fiend Artisan.
All source/table/seed exclusions against the reserved game records still pass.

These are technically valid games. The user explicitly requested learning
from human and house decks. They are retained for a **known-deck development
continuation experiment**, with an explicit scope decision; they do not qualify
for the original holdout-preserving training contract. The negative reservation
audit and official deck manifests remain unchanged. This affected model lineage
is ineligible for unseen-card/deck claims using those reservations. Removing
445 tables from a later stage cannot erase inherited exposure, and R6 is not
certified as an unexposed initialization. A future zero-shot experiment needs
a separately audited initialization and training corpus.

## Prepared next experiment

Two sequential arms start from the same fixed R6 checkpoint. Both use
teacher package 0002, B=8, U=32, K=2, seed 33002 and exactly 8,443 optimizer
updates. Four persistent lanes share the same teacher half; the other four
use its complementary teacher half or the frozen 546-game LLM source.
Every source segment receives its first exposure. The shared half repeats
for 1,058 batches; the LLM source repeats for 260. Padding and repetitions
are recorded. Loss, architecture and optimizer settings are unchanged.

An existing-cache payload audit parsed 640,573 decision records using two CPU
workers in 36 seconds, with no CUDA or cache construction. Planned consumed
genuine labels are 357,781 for control and 320,773 for the mixture. Pass/active
categorical targets are 282,910/65,001 versus 193,651/109,589. Raw genuine-label
weights total 357,781 versus 306,838.75. These are not loss-normalized gradient
weights: equal updates and lane counts do not imply equal label doses.

The experiment compares two complete continuation recipes at fixed update
budget. Finalization requires the actual final step and full final validation;
a validation-selected earlier checkpoint cannot replace it. A separate CPU
worker strips training state and seals final model artifacts. Execution is
blocked while the existing controller is active or has not completed round 12.
Nine focused controller tests pass. The preparation check verified 2,567
sealed inputs (23.3 decimal GB); its only launch blockers are the active
continuous controller and its unfinished 12-round budget. No training was
started by the preparation check.

Before training, 256 gameplay cases were preregistered: 128 fresh table seeds,
both focal seats, 29 known development decks, raw K=2 policy versus Sharp and
a Steady baseline. The seed range is 813301729012345000 through
813301729012345127; no overlap was found with 2,242 checked source seeds.
The case list, engine, manifest and deck files are pinned. All caps and
fallbacks remain reported, with paired whole-table uncertainty. This is a
fresh-seed comparison on known deck families, not an unseen-deck holdout.
Neither gameplay execution nor promotion is automatic.

Evidence lives under `/home/ace/baylee-data/v5/ops-codex-20261028/`:
`training-round10-independent-audit.json`, `training-round10-reviewed-20261029.json`,
`diagnose-round9-pass-confusion/completed-audit.json`, and
`full-coverage-pair-20261029/` (schedule, label exposure, reservation, scope
decision, explicit gameplay plan and bounded runner). No push or landing gates
were performed for this documentation update.
