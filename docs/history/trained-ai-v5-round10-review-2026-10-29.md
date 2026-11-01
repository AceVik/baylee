# V5 round 10 review and bounded continuation comparison

**Later owner-directed change, 29 October:** The continuous run was stopped
during round 11; step 1,800 of its LLM stage is preserved and audited. The
[refactoring and training plan](../proposals/trained-ai-v5-refactor-training.md)
supersedes the unstarted follow-up actions below, including launching the
unchanged sealed coverage pair. Three CPU investigations and a no-update GPU
K1/2/4/8 diagnostic have completed. The CPU reference and user gateway continue.
The paragraphs below retain the earlier state and research provenance.

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

**Later update:** Native Claude authentication was restored. Both Opus 5.5 and
Fable 5.1 have now completed an independent review and a correction round of
the subsequent refactoring plan. See the
[follow-up consultation](trained-ai-v5-opus-fable-review-2026-10-29.md).
The account below describes the earlier consultation and its limitations.

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

## Expanded research: training the relational and recurrent components

Research checked on 29 October 2026 against the frozen trainer and the working
checkout. These are proposed experiments, not changes to the running controller
or the sealed full-coverage pair. The literature supports plausible mechanisms;
none of the cited experiments establishes why Baylee's recent rounds regressed.

### Keep the three kinds of recurrence separate

| Mechanism | Current recipe | What changing it would do |
| --- | --- | --- |
| Typed graph message passing | Two shared-weight rounds, width 160 | Changes local propagation between related entities during board encoding |
| Decision scratchpad | K=2 refinements against an encoded board | Changes computation for one decision; does not rerun the graph processor or simulate future actions |
| Chronological memory | Nine persistent slots, TBPTT U=32 | U changes direct gradient credit across events; detached memory already survives segment boundaries |

The transformer following the relational processor can communicate globally;
two message-passing rounds are not a two-hop limit on the whole network.
Increasing K does not increase the graph processor's round count. Neither
parameter automatically turns the model into an action-conditioned world model.

Relational inductive biases can help game agents, as demonstrated by
[Relational Deep Reinforcement Learning](https://arxiv.org/abs/1806.01830).
They do not imply that ordinary gradient-based imitation is invalid. Conversely,
[Google Research's graph-algorithm study](https://research.google/blog/understanding-transformer-reasoning-capabilities-via-graph-algorithms/)
finds task-dependent advantages for transformers and message-passing networks.
This supports retaining our global transformer and testing what the additional
relation path contributes. It is not evidence that a GNN must beat v4.

### Train the intended serving depths

[Adaptive-Depth Is a Finishing Stage](https://arxiv.org/abs/2610.14252),
submitted 9 October 2026, studies fixed-depth language-model training followed
by a shorter phase sampling multiple depths. Its controlled experiments retain
performance across serving depths at lower training cost than varying depth
throughout. This motivates finishing an existing K2 model at several K values;
it does not prescribe Baylee's optimum schedule or establish a game-strength gain.

[Recurrent-Depth VLA](https://arxiv.org/abs/2602.07845) supplies an action-policy
analogy: a shared recurrent scratchpad attends to a static context, with randomized
training depth and gradients through only the final part of deep unrolls. Its
robotics setup and depth distribution differ substantially from ours. Do not
copy its mean depth of 32 onto this 12 GB card or treat its convergence heuristic
as proof that an action is correct.

The code already has `--thinking-curriculum`, including in the frozen trainer.
`training4/execution.py::plan_batch` and the reference driver cycle
`(1, 2, 4, 8)[(step_num + t) % 4]`. Passive observations retain K=0. The current
continuous controller explicitly uses K=2 and does not enable the curriculum.
Therefore the next step is to make an existing facility reproducible and test
it, rather than claim that variable-depth training must be implemented from
scratch.

Two limitations need attention before relying on that flag in a new experiment:

- The curriculum boolean/schedule is absent from the trainer's checkpoint
  contract and resume validation; the default K is recorded but does not
  distinguish fixed K2 from cycling K with default K2. Bind schedule/version,
  allowed depths and any sampler state, and reject incompatible resumes.
- Current validation uses only `args.thinking_steps`. Report separate K1/2/4/8
  results. Measure actual supervised depth exposure: cycling over all frames
  need not give equal depth exposure among genuine decisions.

A bounded finishing pilot should compare fixed K2 with a declared mixture of
K1/2/4/8, from identical weights and data with matched source exposure. Supervise
the selected endpoint rather than evaluating every depth for every training
example. Record both updates and measured compute/time: equal updates are not
equal computation. A schedule sampled per update with an independent, saved RNG
is an alternative to frame cycling, but is a separate implementation change.
Keep observation updates and persistent H independent of the selected K.

More recurrence can also hurt. [STARS](https://arxiv.org/abs/2605.26733) studies
this failure and combines random loop sampling with a Jacobian-based stability
regularizer. First measure our latent norms, policy changes and accuracy versus
K; add such a regularizer only in a later measured ablation if instability is
present. A stable wrong answer is still wrong. Confidence or convergence alone
cannot certify stronger play or a safe early-exit rule.

### Give relations consequences to learn

[CLRS](https://proceedings.mlr.press/v162/velickovic22a/velickovic22a.pdf) trains
algorithmic processors using structured inputs, outputs and optional intermediate
states. [Latent Reasoning with Supervised Thinking States](https://research.google/pubs/latent-reasoning-with-supervised-thinking-states/)
also explores explicit intermediate supervision, in a language-model setting.
The proposed transfer is small, verifiable game subproblems; neither paper
demonstrates this transfer to Magic or licenses an assumption of automatic
strategic understanding.

Our four existing auxiliary targets are remaining turns, own final life,
minimum final life among other seats, and next-decision own-life change.
The third includes teammates under its historical semantics. These targets
do not directly teach the consequences of particular edges or proposed actions.

Start with a separately versioned, small auxiliary dataset for restricted
public-information tasks: whether a proposed mana-payment sequence covers a
specified cost; casualties/damage for a fully specified combat; or the effect
of removing an attachment under explicit conditions. Use a conditional prediction
head reading entity/pair representations and the proposed action. Merely copying
the already encoded power, toughness or attachment ID is not a reasoning test.
Generate paired positions differing in one relevant relationship, and include
larger combinations and different card identities in separately reserved tests.

The auxiliary must use exactly the information available to the focal player.
Future outcomes may remain training targets without becoming inputs. For
counterfactual exact labels, use fully specified synthetic scenarios or a
verified reconstruction from the view and authorized history; never clone the
real hidden engine state. If opponents can respond or hidden information affects
the answer, condition the task explicitly, mask the exact target, or predict a
distribution. The proposed general view reconstructor is not already implemented.

For restricted tasks with several demonstrably equivalent answers, score the
valid answer set rather than force one arbitrary tap order. Do not extend such
equivalence claims to general strategic actions without evidence. Auxiliary
readouts can be training-only, leaving the deployed inference graph unchanged;
their representation changes still require export/parity and strength checks.

### Improve decisions where the learner actually goes

[DAgger](https://proceedings.mlr.press/v15/ross11a.html) motivates teacher advice
on learner-visited states because imitation changes the distribution of future
observations. Baylee already implements a view-only Sharp/Expert shadow teacher
and verified corrective replay. Its earlier mixed-data pilot did not establish
a strength gain. This is a mechanism to diagnose and improve, not a newly
discovered missing feature or a guaranteed cure. Expert-versus-expert LLM games
alone do not provide the same coverage of the learner's mistakes.

Memory training needs chronological observations, not shuffled independent
decision frames with fresh H. Our stateful lanes already retain history. Test
recall of previously visible information and decision sensitivity to that history
before spending substantially more VRAM on U. Burn-in/reconstruction is relevant
if a future replay sampler starts mid-game; increasing U and mixing the slots
are separate hypotheses, not prerequisites established by these papers.

### Ordered follow-up and resource limits

1. Finish the unchanged full-coverage comparison and integrate/test a repair for
   the demonstrated categorical-context cancellation in its own versioned change. Keep Encoder8 and
   slot mixing separate so their effects remain distinguishable.
2. Run the depth-finishing pair after the schedule/resume contract and K-specific
   validation are ready. Profile a representative high-candidate batch first;
   retain BF16, activation checkpointing, cached static-board projections and
   compact decision rows. The existing GPU job owns the GPU until it exits.
3. Compare baseline BC against BC plus the restricted relational auxiliaries,
   holding depth fixed. Track auxiliary accuracy, active-action performance,
   relation-path gradients and output magnitude. Zeroing a trained branch is
   a reliance diagnostic, not a substitute for a trained v4 control.
4. Confirm any promising recipe in a v4/v5 comparison with an audited common
   initialization, matched data and explicit compute accounting. Test actual
   games on fresh table seeds, keeping caps/fallbacks and table-cluster uncertainty.
   Include K-dependent latency and win rates before naming difficulty levels.

No larger trunk, new optimizer, full world model or full restart is required
for these first tests. A better local consequence predictor could later support
action comparison and search, but variable-depth BC alone does not implement
the requested 15–45-second System-2 search. The review makes no promotion and
does not alter the preregistered 8,443-update coverage pair.

## Self-improvement, self-play and Terra: research follow-up

The owner proposed self-play and games against Terra. Research checked on
29 October 2026 supports a new learning loop around the existing network;
there is no demonstrated need for another architecture revision merely to
learn from game outcomes. Producing rollouts, updating weights and validating
the candidate are separate operations. Ordinary inference does not perform
the weight update, and copying all of a model's own actions is not an
outcome-driven improvement objective.

### Relevant evidence and its limits

- [DeepSeek-R1](https://arxiv.org/abs/2501.12948) demonstrates reasoning
  post-training with reinforcement learning and verifiable feedback. For Baylee,
  the engine can establish terminal results; it cannot identify every optimal
  move from one sampled game. A win is a learning signal, not a certificate
  that every preceding action was good.
- [SPIRAL, ICLR 2026](https://arxiv.org/abs/2506.24119) trains existing language
  models through multi-turn, two-player zero-sum games. Its role-conditioned
  advantage estimator and evolving opponents are relevant to our training
  design. The experiments concern much smaller games, including Kuhn poker;
  their results do not establish feasibility or strength at Magic's scale,
  or convergence in multiplayer Commander.
- [AlphaStar's league](https://deepmind.google/blog/alphastar-grandmaster-level-in-starcraft-ii-using-multi-agent-reinforcement-learning/)
  combines imitation, diverse opponents, frozen historical policies and agents
  exposing weaknesses. This motivates a small local opponent pool and retaining
  supervised anchors; it does not require reproducing AlphaStar's compute scale.
- [On-Policy Self-Training](https://arxiv.org/abs/2610.32770), submitted
  23 October 2026, studies self-generated tasks and demonstrations for
  self-distillation of pretrained foundation models. Baylee has neither their
  language-task interface nor their broad pretrained knowledge. The useful
  analogy is curriculum generation, with our engine checking restricted tasks.
- Recent self-distillation studies report [loss of strategy diversity](https://arxiv.org/abs/2606.26091)
  and [overconfidence from privileged teacher information in multi-turn tasks](https://arxiv.org/abs/2609.29051).
  These are reasons to test external performance and keep teacher advice within
  the student's authorized information. They do not prove the same failure has
  occurred in Baylee.

### Existing capability and specific implementation work

V5 already provides a stochastic-policy-capable action interface, a recurrent
state and a value head. An initial outcome-driven policy-gradient experiment
does not require a language decoder, a world model, an extra independent rater,
or a larger trunk. The value head estimates expected outcome as a baseline;
the verified terminal result supplies the reward. These roles must stay distinct.

The current continuous v5 job is BC plus value/auxiliary regression, not PPO.
`train_rl4.py` explicitly instantiates `Config4`/`PlayerNet4` and rejects v5
identity. The isolated action-head trainer shares a loader with value calibration
which can load v5, but its publication payload omits the v5 architecture identity;
this is not a supported v5 PPO round trip. Extend and test identity, loading,
sampling/likelihood replay, update, export and rejection paths together.

The head-only optimizer freezes representation, graph, thinking and memory
parameters. It is a useful first test of rollout integrity and conservative
policy updates, but cannot demonstrate that RL teaches the relational or thinking
blocks. A later full-model recurrent update needs correct chronological memory,
credit assignment, a small update budget and measured GPU memory consumption.
Do not merely remove the freeze from a path that reuses cached behavior states
and frozen spatial representations as if those assumptions still held.

Terra integration needs another explicit distinction: `seat4/main.rs` currently
sets temperature to zero. Those live neural actions have no PPO behavior
probability. Existing live-BC conversion also intentionally gives neural seats
zero policy imitation weight. Running more such games alone will not activate
neural policy learning. A dedicated collection mode must retain the exact
sampled learner action, temperature, K, model identity, conditional masks and
behavior log-probability, then verify the executed engine action and replay.
Do not retrofit invented probabilities onto archived games. Scalar or other
unsupported sampling heads need an explicit eligibility boundary or a tested
likelihood implementation.

Terra's own probabilities are not required to update Baylee from Baylee's moves
against it. Terra actions can separately supply supervised examples, under the
existing teacher-data contract. Asking Terra for a correction requires a view
of the learner's seat and its authorized history; an opponent's private view
cannot be reused as that teacher context. Neither source belongs in the other's
PPO denominator.

### Proposed bounded learning loop

1. Freeze one learner checkpoint for collection and mix opponents: the latest
   frozen learner, older retained versions, house profiles and a smaller stream
   of Terra games. Begin with 1v1 and balanced seats/deck pairings, then separately
   validate teams and multiplayer objectives. Keep game-level memories separate
   even when both seats share weights. Adapt matchmaking after measuring results;
   avoid making a nearly unbeatable or trivially beaten opponent the whole pool.
2. Retain full learner histories, including passive observations and losses.
   Use genuine terminal outcome with a declared draw utility. Do not reward mana
   tapping, raw life gain or apparent board size as substitutes for winning.
   Model errors, crashes and caps remain technical/unresolved outcomes, never
   invented victories; track exclusions to expose collection bias.
3. Audit action probabilities before any PPO update. Make a small conservative
   update with the existing KL rollback principle and a separate supervised
   anchor against forgetting. Calibrate/check the value baseline by seat and
   format. Estimate expected returns over varied games; one loss due to a bad
   draw does not label each action as a mistake. GRPO is an optional later
   comparison, not a required modernization: large groups of long game rollouts
   can be expensive, and their rewards may mostly reflect draw/opponent variance.
4. Evaluate against fixed opponents and fresh paired initial conditions, retaining
   all caps and fallbacks and uncertainty at the table level. Latest-versus-latest
   self-play win rate is not evidence of improvement. Only reviewed improvements
   enter the deployed slot; retained checkpoints guard against cyclic forgetting.

Self-play and house opponents should provide most local throughput; Terra can
add different strategies, selected corrective examples and a challenging external
opponent. This allocation is a proposal to measure, not a claim that Terra is
optimal or that its past small pilot establishes comparative strength. Keep
collection on CPU or batch inference according to measured throughput and the
single-GPU budget; never start a competing GPU trainer beside the active job.

First extend the supported v5 RL/data path, then run a small isolated pilot from
an audited checkpoint. Keep K fixed in that pilot to avoid confusing PPO effects
with depth training. No new collector, RL job or architecture change was started
by this research, and the existing full-coverage experiment remains sealed.
