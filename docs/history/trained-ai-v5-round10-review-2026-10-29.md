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

## Architecture and training consultation, 29 October

The owner requested Opus 5.5 and Fable 5.1, explicitly including architecture.
Both native Claude requests failed before analysis because the OAuth session
had expired. Opus was then reached through agy as `claude-opus-5-5-high` and
completed an initial review and a challenge/correction round. Fable was not
available through agy's model catalog and has provided **no review**. This is
not a three-model consensus. A separate Codex helper checked encoder limits;
the root agent reproduced the architectural findings below on CPU.

Requests, source hashes, responses, exit receipts and reproducible probes live
in `/home/ace/baylee-data/v5/advice/opus-fable-training-diagnosis-20261029/`.
The source of truth for model operations is the frozen continuous controller's
`recovery-round9-llm-oom-20261028/trainer-offload/`, not the modified checkout.
Rust encoder findings below refer to `baylee-ai-payment`; they must be bound
to a production binary before attributing an observed game's failure to them.

### What the training evidence establishes

R6 final won 21/64 development cases against Sharp; R9 final won 15/64.
There were three improved and nine worsened cases, with a paired table-cluster
95% interval of [-0.203125, 0]. There are only 32 independent table clusters,
and this suite has been repeatedly consulted. The point estimate is concerning;
it does not establish a general regression or identify its cause. R10 has not
yet received a playing-strength comparison. Technical completion, finite
weights and export parity are established separately from strength.

The continuous recipe is behavioral cloning plus value/auxiliary regression.
It does not use the critic to rank alternative actions or optimize policy
advantages. A lower prediction loss therefore does not establish better play.
All 64 extra exact categorical matches in the five-game R9 diagnostic are
passes, but **active-target NLL also improves** (1.533592 to 1.416559). Saying
that all NLL improvement comes from passing would be wrong.

The minimum-prefix schedule is a concrete coverage problem. R10's LLM stage
touched 147 of 546 games, 141,566 frames and 41,687 decision frames before its
1,124-update limit; 399 games were untouched in that stage. This does not mean
they were never used in earlier rounds, nor that all 147 touched games were
fully consumed. The prepared complete-coverage comparison addresses this
defect without changing the current run.

The following remain hypotheses: competing teacher styles, forgetting between
stages, effects of resetting AdamW moments at each stage, interference from
value/auxiliary gradients, and unequal loss contributions from rare heads.
`loss_v5.py` does sum a separately normalized mean for each present head, with
value/auxiliary weights 0.5/0.25 on all valid frames. Its behavioral consequence
requires measurement; the coefficients alone do not establish gradient conflict.
The config deliberately rejects non-unit policy-head weights, so the lack of a
head-weight multiplication in this version is not an ignored supported option.

The live-BC converter gives positive policy weight only to eligible actions
declared as Human/LlmApi/LlmCli. Correctly declared neural seats receive zero
policy weight; their observations can still contribute value/auxiliary loss.
Thus the presence of neural-versus-LLM games does not establish blind
self-imitation. A provenance audit would need to find mislabeled minds before
making that claim.

### Reproduced architecture defect: categorical menu context cancels

In frozen `model4/__init__.py` and `training4/deferred.py`, candidate hidden
vectors receive the same pooled-context vector immediately before a linear
scalar projection:

```text
logit_i = w * (h_i + context(menu)) + b
```

The context adds the same scalar to every valid candidate. It cancels in
categorical softmax, log-odds and cross-entropy. It cannot provide the intended
menu-dependent relative preferences. The existing
`test_p0_7_encoder7_warm_start_and_candidate_context_refinement` checks a change
in a raw logit, which does not test this property. Independent subset sigmoid
outputs can use the common shift, so the path is not dead for every head.

`architecture_probes.py` ran a tiny v5 revision-2 instance through the actual
frozen forward and deferred modules. Enabling the context changed raw logits
by 0.237, but categorical probabilities by only 5.96e-8; its categorical weight
gradient was 2.56e-8, numerical residue. The deferred path reproduced the
cancellation. This is a demonstrated mismatch with the feature's intent, not
evidence that it caused the R6-to-R9 difference: both models inherited it.

`candidate_repair_probe.py` tests an isolated alternative:

```text
new_h_i = old_h_i + h_i * tanh(new_projection(mean(h), max(h)))
```

A zero-initialized new projection preserves initial logits bit-for-bit and
receives a nonzero categorical gradient. With nonzero weights, changing a
third candidate changes the relative preference between two fixed candidates.
At width 640 it adds 819,840 parameters and avoids quadratic attention over
large naming menus. This is an algebra/gradient feasibility test, not an
implemented production revision, ONNX benchmark or playing-strength result.
Any integration needs an explicit new architecture contract, separate forward
and deferred parity checks, archived warmstart checks and an isolated ablation.
It must preserve the legacy subset behavior and archived model identities.

### Memory, reasoning and representation

- **Nine recurrent slots retain state across the match**, but the recurrent
  cell updates slots independently. Thinking queries the static board with
  each slot separately; there is no slot-to-slot attention and the board
  encoder does not receive H. Perturbing an opponent slot in the CPU probe
  changed value by 0.0178 but left categorical/scalar output unchanged when
  candidates had no seat link. Global memory still reaches these heads, and
  a valid `cand_seat_idx` exposes that seat's history. A small slot mixer is a
  plausible separate experiment; these results do not prove memory is unused.
- **The pair head does read global memory/thinking** through `g_pair = g_ref`.
  Entity vectors themselves remain board representations. Opus initially
  claimed the pair head saw no memory; it withdrew that claim after the root
  pointed to the actual scorer inputs.
- **K is repeated latent refinement, not simulated future play.** The current
  K2 training gives no task gradient to thinking-step embedding rows 2 onward;
  the CPU gradient probe confirms this. Warmstarted rows may have earlier
  history and weight decay, so we cannot claim they remain randomly
  initialized throughout the lineage. K1 also lacks a separately supervised
  endpoint in this recipe. Export parity at K4/K8 establishes numerical
  correspondence, not trained difficulty levels or stronger thinking.
- **TBPTT length 32 limits direct temporal credit, not memory lifetime.** H is
  carried between segments with detached gradients. Memory-reset probes on
  complete histories can measure reliance; a small measured effect would not
  by itself prove a particular mixer or longer unroll will improve play.
- **V5 has real graph processing and card structure inputs.** Two 160-wide
  relational rounds use typed directed edges and explicit multiplicity.
  Typed AST tags, bytes and ordered children enter through the 128-wide rule
  encoder plus numeric DSL features. The relationship module receives these
  representations; claiming it receives no card semantics would be false.
  It has no explicit learned transition model or counterfactual simulator.
  Neither complete syntax ingestion nor the name “geometric reasoning” proves
  rules understanding, zero-shot transfer or strategic planning.
- **E7 has a known payment-context gap.** Owed mana symbols are already
  present, but announced hand-source identity is not persistent through all
  nested questions. The isolated E8 implementation addresses that observation
  contract; it has not been trained or demonstrated stronger play. Grouped
  candidate encoding intentionally retains the tap-first menu and omits
  `legal.payable`. E8 alone does not turn it into a trained announce-and-pay
  planner. Replay and native paths use the grouped encoder, so the existence
  of a separate announce-before-payment encoder is not proof of a mismatch.
- **Encoder ceilings require explicit diagnostics.** The reviewed E7 code
  silently stops at 1,024 entities and 8,192 edges. Its production grouped
  candidate path calls `encode_candidates4` directly, bypassing the other
  encoder's explicit `CandidateLimit7` check. Some choices truncate at 65,535
  candidates/pairs before grouping; card naming enumerates all matching faces
  without that cap. No inspected evidence shows these ceilings were reached
  in the reported duels. Treat them as contract risks, not the established
  cause of weak results.

### Decision and next checks

Do not enlarge the trunk merely because recent rounds failed to improve.
Prioritize complete source exposure and a versioned categorical-context repair.
Test a slot mixer, better payment context and a trained K schedule separately;
changing all of them together would prevent attribution. Existing training,
the user gateway and sealed comparison inputs were not changed in this review.

The 8,443-update two-arm plan remains a known-deck recipe comparison with
unequal label doses, one training seed and no automatic promotion. Its fixed
256 cases are 128 fresh tables, not 256 independent table seeds. The R6
reference is running on CPU under the existing review lock. Do not expand the
primary comparison to every historical checkpoint or choose cases after
inspecting partial outcomes. Additional exposure summaries can record turn,
decision type and evaluation-deck overlap without editing the sealed recipe.

After the CPU reference releases its lock, a bounded diagnostic can compare
active-choice loss on training versus development histories and test memory
resets. Use whole histories and explicitly distinguish diagnostic slices from
unbiased game samples. Subsequent architecture checks need relative-logit and
gradient tests, not just changed output tensors. Promotion requires an actual
paired playing-strength review, with caps/fallbacks retained; a promising
screening result needs separately planned confirmation.

A focused primary-source research check supports testing these mechanisms,
not assuming they transfer to Baylee: [DAgger](https://proceedings.mlr.press/v15/ross11a.html)
addresses the mismatch between expert and learner-visited states;
[recurrent-depth training](https://arxiv.org/abs/2502.05171) studies learned
latent computation at variable depth; and
[STARS](https://arxiv.org/abs/2605.26733) reports that naive extra recurrence can
degrade performance and studies stabilization. Their tasks and scales differ
from this game. None establishes that increasing Baylee's K, parameter count
or rollout time will improve its play without suitable training and evaluation.
