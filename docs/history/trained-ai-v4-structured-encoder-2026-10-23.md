# Structured card inputs — 23 October 2026

The owner authorized implementing the card-input repair before increasing the
policy's width. No engine rule, card implementation, or PlayerView protocol is
changed. Work is on `c42/trained-ai`; artifacts are in
`~/baylee-data/v4/experiments/codex-r4-structured/`.

## Contract and representation

- State encoder **6**, model **3**, structure **1**. Trajectory/sampling remain
  **2**. The existing 160-float summary remains DSL version **2**, an additional
  useful input; its old eight-row ability table is no longer the sole ability
  representation. Old checkpoints cannot resume silently under this contract.
- `baylee-train/build.rs` reads the local Rust DSL declarations with `syn` and
  emits exhaustive typed visitors into Cargo's build directory. It never reads
  the external card-script corpus, hashes Debug strings, or modifies generated
  card files. New fields are included; an unsupported reachable type fails to
  compile. Variant/field names, ordered children, exact integer bytes, strings,
  options, costs, filters, conditions, targets, replacement rules, all faces,
  every ability and every modal branch survive. Printing artwork identifiers
  and coverage metadata are excluded. Names are retained because rules can
  depend on names. Recursion/node limits fail explicitly, never truncate.
- `cardstructure` interns equal subtrees by exact equality in an ordered DAG.
  Its canonical JSON digest binds datasets, checkpoints and ONNX exports to
  the compiled card definitions. The first complete catalog contains **9,343
  compiled definitions**, **127,214 nodes** and **31,050 named roots**. This is
  a public knowledge catalog, not a claim that every definition is playable;
  the existing working-card restriction still governs games.
- `entity_rules` and `cand_rules` are variable-length lists of `(node, role)`.
  Role 1 is card/token semantics, 2 public card/face identity, 3/4 chosen-name
  semantics/identity, 5 stack ability, 6 visible grant provenance, 7 the selected
  ability, face, alternative cost or mode. Zero is unknown/padding, never card
  zero. Grant handles select references; no hidden identity enters the actor.
- References come exclusively from the existing PlayerView and legal offers.
  Rules overrides on copies, known names and printed/token stack abilities are
  linked. All visible grants are retained. When the view exposes a grantor but
  no exact inherited definition, its source card's rules are supplied. Hidden
  provenance remains unknown. For modal questions without an exact ability
  handle, matching modes are supplied with the observed stack context; this
  does not invent a privileged resolution context.
- A shared **128-dimensional** neural encoder composes the DAG bottom-up using
  node labels, byte positions and ordered child messages. During policy
  training its table is recomputed once per optimizer update with gradients
  intact, then reused across that unroll only. Validation computes one table
  for fixed weights. ONNX bakes that exact table and uses gathers/role pooling.
  Raw syntax is preserved; learned finite-dimensional vectors are not claimed
  to be mathematically collision-free. NPU execution has not been certified.
- The complete policy now has **54,825,608 parameters**. The nine persistent
  memory slots and K=0 observation / K=1,2,4,8 decision clock are unchanged.
- `refresh_structure4.py` explicitly binds weights to a later catalog. It
  rebuilds DAG buffers and maps learned operator embeddings by exact names,
  so changing traversal IDs cannot silently change meanings. New combinations
  of known operators work; unknown operators fail and require training. Export
  again, replay training data and start fresh memory/optimizer lanes. This is
  a compatibility mechanism, not a claim that unseen cards are played well.

## Training sequence

1. **Structure pretraining:** `pretrain_structure4.py` asks for the node kind
   and a numeric/string byte at an ordered path in a card's rule tree. Labels
   come from the DSL, not a learned move evaluator. Card identities, including
   every face, stay together in a deterministic 90/10 split. Shared subrules
   are expected. This auxiliary holdout is not the final game-strength holdout.
   The initial 1,200-step GPU run (batch 512, width 128, seed 31415, AdamW 3e-4)
   completed in about 110 seconds. Fixed held-out node-kind accuracy reached
   **95.36%**, combined query loss **0.3149**; these are not win rates.
2. **Replay data:** first a 704-game integration corpus (`dataset-v1`), then
   the same broad corpus used by the previous policy (`dataset-stream-v1`,
   expected 847 games, 6,620,782 training and 3,072,672 validation observations).
   The new streaming converter checks replay hashes and every semantic reference.
   The previous v4 collecting path did not audit recorded hashes; that gap was
   found and repaired together with its memory problem. Existing
   lossy vectors are not patched to pretend discarded information survived.
   Failed records/jobs are preserved; no rejected dataset is automatically used.
3. **Explicit warm start:** `migrate_structure4.py` transfers the selected
   evening policy at phase step 3116 and the pretrained encoder. Its new
   semantic projection starts at zero, allowing an initial equivalence check.
   Optimizer, sampler lanes and recurrent memory start fresh. The encoder and
   adapter subsequently learn jointly with the complete policy. Earlier
   policies and datasets remain immutable comparison artifacts.
4. **Integration pilot:** a short real-trajectory optimizer run, full validation
   of its bounded integration split, gradient/cache checks and actual Rust
   ONNX games. Check memory on large candidate sets; no choice/ability may be
   dropped to fit VRAM. The first 100 updates completed without allocation
   retries at **5.04 GiB peak allocated VRAM**, using two training shards and
   one validation shard. On its 27,288 validation observations, active-choice
   accuracy was **45.84%** (2,101 choices); this small split cannot be compared
   to earlier full-corpus scores. Both the semantic projection and encoder
   weights changed; gradients reach the structural encoder.
5. **Broad continuation:** CPU preparation and GPU training now overlap, using
   the existing B=8, U=32, observation chunk=8, decision chunk=4 and thinking-depth
   curriculum. Every complete-game package receives one full epoch at **2e-5**;
   Adam state transfers across packages, recurrent lanes reset. Two packages of
   approximately 500,000 frames can be queued. Each phase validates on the same
   463,322 integration holdout frames, with lanes fixed at 8. The earlier
   `joint-adapt-v1` completed 1,000 updates at 5e-5 but regressed in paired games
   and is not the starting policy. The failed replay prevented `joint-broad-v1`
   from starting. The replacement bootstrap and its evidence are detailed below.
   This is staged training, not an equal-budget architecture ablation.
6. **Strength comparison:** export and verify nonzero references, then compare
   to the archived encoder-5 policy in the **same new eval executable**, with
   the frozen deck manifest, starting seeds and seat rotations. Keep all bad
   deals and unknown capped outcomes. Repeat K=1/4 only after a useful base
   policy is established; this change does not create future-state search.
   The queued K=1 comparison uses 128 duels and 64 Commander games, seed
   980001, Sharp opponents, Steady house duplicates and the previous selected
   phase-3116 policy. Final untouched tests and broader tactical/card holdouts
   remain required; no model is promoted automatically.

## Verification and operational notes

Rust tests cover Negate/Mana Leak/Remove Soul, negation vs positive filters,
And vs Or, sequence order, large numbers, twelve abilities, public card zero,
and identical actor inputs after changing an opponent's hidden deal. Python
tests check semantic dependence, gradient equivalence with table reuse, frozen
lookup equivalence, malformed DAG rejection and ONNX with nonzero references
and changing reference-list lengths. Existing recurrent/training tests and the
six original Python contract tests are included in the checks.
The initial encoder milestone passed **63 Rust tests** and **40 Python tests**,
including refresh after operator/DAG renumbering. The streaming follow-up adds
exact frame/target equivalence and package-integrity tests, described below.

The initial migrated 54.8M model exported K=0/1/2/4/8 with maximum Torch/ORT
error **1.38e-5** on 15 synthetic frames. Real-trajectory parity and subsequent
playing-strength results are separate checks. After 100 training updates,
export passed **143 frames**, including a recurrent held-out trajectory with
nonzero rule references, at maximum absolute error **3.41e-5**. The migrated
initial policy also matched the legacy policy exactly for all outputs at
K=0/1/4 and for two actual games (answers, outcomes and fallback counts).
The trained pilot also completed eight GPU-inferred Rust games with 1,824
neural answers, two house fallbacks, no refused actions and no stalls. It won
0/8 against Sharp (Steady duplicates won 4/8); this bounded integration pilot
is not a strength improvement. Its inference p95 was about 4.1 ms. The first
attempt from a copied executable lacked adjacent ORT provider libraries and
failed at startup; the corrected runtime includes them and preserves the
failed attempt's logs separately.

A final room-specific check covers the difference between an offered active
door-list index and a per-face printed index. Selected references now map
through the publicly encoded unlocked-door bits. A blank left door does not
inherit the right door's rules when the catalog is built. Stack references
already carry the engine's captured per-face provenance. The current compiled
pool's only Room is Walk-In Closet (static/triggered abilities); this follow-up
does not alter its catalog digest or the active frozen experiment's inputs.

The old `pretrain_dsl4.py` summary experiment is explicitly rejected for the
new contract; it cannot silently emit an incomplete encoder-6 artifact.

The narrow Clippy check covers the library and changed converter/catalog/eval
binaries. An all-binaries attempt also reported pre-existing warnings in
`src/bin/import_decks.rs`; that unrelated importer was not rewritten here.
No changes have been pushed or landed; workspace gates remain required before
that step. Missing Linux build dependencies were installed via apt after the
owner enabled passwordless sudo.

Persistent experiment manifests record commands, source snapshots, catalog
checksums, stdout/stderr and failures. Initial pretraining uses a frozen
`pretrain-runtime`; replays use a frozen `convert-runtime`. These remain separate
from ongoing source edits. The subsequent `runtime-v1` freezes source at
`db27937b9` and includes the eval executable and CUDA provider libraries with
file hashes. `continue-structured.py`, `continuation-status.json`, and the
per-stage `jobs/` directories record the automatic sequence and its failures.
The first broad replay attempt hit its **28 GiB** process-group limit with four
workers during large 8-seat games (peak PSS 28.17 GiB). The two-worker retry
`reconvert-broad-v2` also failed at **48 GiB**, after about 4,302 seconds with
836/847 games converted. Both attempts are quarantined. The old controllers
exited at their failed prerequisite; all attempts and logs remain available.
The machine monitor remains running.

## CPU preparation and GPU training overlap

The user requested preparing the next training set while the GPU consumes the
current one. The actual memory cause was a whole game's encoded `Step4` vectors
being accumulated before writing. Raising the memory allowance did not solve it.
`convert4.rs` and its binary were first moved to directory modules in a separate
move-only commit. The new production path then changed to two bounded passes:

- Audit the record with `baylee_gamehost::record::replay`, checking the initial
  and every subsequent recorded state hash; derive final labels from the verified
  terminal state. These future labels never enter the actor's feature encoder.
- Replay features while keeping one pending frame per seat. Its successor gives
  the next-life target; the final frame receives the terminal marker. Every
  passive observation and decision, per-seat order, supervised and auxiliary
  target remains the same. Cross-seat file interleaving can differ.
- Each worker serializes and compresses its own game. Only completed games with
  represented semantic references and zero unencoded decisions receive an atomic
  ready marker. Failures retain private partial files and a record locator.

`stream-smoke-v1` replayed three complete records (duel, team, and the troublesome
eight-seat validation seed **898002**): **199,019 frames in 461.9 seconds**, peak
process-group PSS **257,002,496 bytes (245.1 MiB)**. The large eight-seat record now
completes. This measures conversion memory, not the GPU trainer's memory budget.

The replacement `stream-broad-v1` job runs six CPU workers against the original
847-game corpus, with schedule seed 2718 and a 16 GiB guard. It writes
`dataset-stream-v1`; the expected full counts remain 6,620,782 training and
3,072,672 validation frames. `stream-convert-runtime-v2/` freezes its binary,
sources and hashes. Completion and zero rejections still have to be checked;
partial progress is not the full dataset.

`prepare-stream-packages-v1` verifies completed training shards, builds their
compressed random-access caches and publishes immutable `packages-stream-v1/`
packages. It holds at most two unconsumed packages; validation runs cannot enter
the training queue. It hard-links closed files instead of copying them. The
GPU consumer uses `package-runtime-v1/`, retaining weights and optimizer moments
across packages through `--init-state`. No hidden state or sampler cursor from
the preceding package is transferred. Final source-count and duplicate checks
must show that every training frame was consumed once. This is supervised
training on existing recorded games, not collection by a changing on-policy agent.

To restart useful GPU work before the full replay finished, `recover_packages4.py`
verified a **65-game / 234,925-frame** subset from closed shards of the failed
attempt, using the same converter executable as a completed template. It excluded
the last possibly unclosed shard, checked gzip integrity, per-seat continuity,
terminal markers, source split and absence of conversion warnings. Original failed
attempts remain quarantined. This subset is explicitly not the full corpus and
does not replace the new hash-audited replay. Its separate integrity manifest is
`package-ready-v1.integrity.json`.

The subset's `joint-package-v1` bootstrap completed **1,005 updates**, one full
epoch, at 2e-5 from `initial`; peak allocated VRAM was **5.59 GiB**. Its matched
fixed validation is slightly better than the migrated initial policy:

| Same 463,322 validation frames, 17,793 active choices | Initial | Bootstrap |
| --- | ---: | ---: |
| Active-choice accuracy | 43.882% | 44.326% |
| Policy loss | 0.462767 | 0.460424 |
| Value MSE | 0.270361 | 0.256137 |

The uniform-seat value baseline is **0.199713 MSE**, so this value head remains
poorly calibrated and these numbers do not prove useful lookahead or stronger
play. `bootstrap-selection.json` records the same policy-loss-plus-twice-MSE
criterion used by training; the consumer chose the bootstrap's optimizer state.
The new first package has **500,058 frames / 96 games**, and the second has
**506,786 frames / 29 games**. GPU training and CPU replay are running concurrently.

For comparison, the earlier 1,000-update `joint-adapt-v1` won **24/128 duels** and
**5/64 Commander games**, against the archived reference's **31/128** and **10/64**.
The paired duel changes were 3 improvements, 10 regressions, 115 ties; Commander
had 4 improvements, 9 regressions, 48 ties and 3 unresolved capped pairs. It was
not promoted. The present pipeline does not continue from that policy.

`finish-package-comparison-v1.py` is queued behind the package consumer. It checks
full corpus coverage, selects the best checkpoint on the fixed tuning validation
including the initial baseline, exports into a separate directory, checks ONNX
parity on real trajectories, and runs 128 duel / 64 Commander paired games. It
uses the frozen `runtime-v1/eval4` executable and its already completed reference
results, with identical seeds, decks, opponents and focal seats. Selection is
recorded separately from the latest optimizer state. No automatic promotion is
performed. Full broader 3/6/8-seat validation and untouched final tests remain
necessary before a general-strength claim.

Durable status: `package-training-v1/status.json`, `progress.json`, per-package
`jobs/`, `package-comparison-status.json`, and the shared `codex-r4-active.json`.
All runtimes are frozen before execution; edits to repository sources do not
change a running job. The full library suite with ONNX passes **64 Rust tests**;
the Python suite passes **44 tests**, including optimizer
continuation, immutable cache reuse, rejection of validation/unverified games,
truncated gzip members and broken trajectory cursors/terminal markers. The Rust
streaming equivalence test compares every frame field against the collecting
implementation on duel and team records and checks corrupted-hash/output errors.

## Completed overnight run and follow-up — 24 October 2026

The entire streaming conversion completed: **847 games, 6,620,782 training and
3,072,672 validation frames, zero rejected games and zero unencoded decisions**.
Six workers took **1,850.2 seconds** at **318,213,120 bytes peak PSS (303.5 MiB)**.
The package consumer trained all **746 training games**, once across **13
packages**, with **27,483 optimizer updates** in addition to the earlier bootstrap.
Training ran from 22:34:37 to 04:31:09 local time; export and comparison finished
at **04:38 on 24 October**. Every guarded package job completed successfully.
The producer's five-hour wall time includes waiting behind its two-package
queue; conversion itself was finished at 22:56.

The combined validation score selected the first package's checkpoint, not the
last. Its lower value MSE (**0.196719**, initial **0.270361**) outweighed worse
policy loss (**0.466489**, initial **0.462767**) and lower active-choice accuracy
(**40.80%**, initial **43.88%**). This is a useful calibration diagnostic, but
an unsuitable sole proxy for playing strength. ONNX parity passed 143 frames
at maximum absolute error **2.57e-5**.

| Same-start K=1 games against Sharp | Previous v4 reference | Combined-score selection |
| --- | ---: | ---: |
| Duel wins / 128 | 31 | 19 |
| Commander wins / 64 | 10 | 5 |

Paired duels have **6 improvements, 18 regressions and 104 ties**. Commander has
**4 improvements, 9 regressions, 49 ties and 2 unresolved capped pairs**. There
were no refused actions, but fallback/stall events remain; the selected candidate
has one capped Commander game, as does the reference. These are substitutions
against fixed opponents, not direct games between the two networks. Neither
policy is promoted, and this run establishes no playing-strength improvement.

The review also found an unintended source-order curriculum in the package
producer. After the converter got ahead, sorting ready game IDs overrode the
shuffled replay schedule. Package 0002 was exclusively duels, 0004 exclusively
2v2, 0006 exclusively 4-player FFA, and 0010/0011 exclusively eight-seat games.
This can shift the training distribution substantially between packages; it is
a plausible contributor, not yet an isolated causal explanation of the regression.

Commit `134a2f565` fixes that ordering. Ready games are seeded within source runs;
whole-game virtual finish times mix sources according to available frame shares.
Including the upcoming game's length avoids front-loading long games at every
package boundary. Unequal game sizes and readiness prevent exact per-package
proportions. A complete planning pass preserves **all 746 games and 6,620,782
frames exactly once**; `mixed-package-plan-v2.json` records its composition.
The full Python suite passes **45 tests**, including source mixing, deterministic
selection independent of input enumeration, and no dropped/duplicated games.

`selection-diagnostic-v1/` now compares two prespecified additional checkpoints:
the last package and package 0005, which has the lowest validation policy loss.
Both use the same frozen engine, seeds, decks and K=1 protocol, with exports in
separate directories. The last package's results are **24/128 duels** and **9/64
Commander wins**. Its paired duel changes are 5 improvements, 12 regressions and
111 ties; Commander has 5 improvements, 7 regressions, 50 ties and 2 unresolved
capped pairs. Package 0005 has **30/128 duel wins**, with 8 improvements,
9 regressions and 111 ties; Commander has **6/64 wins**, one draw, zero capped
games, and 2 paired improvements, 6 regressions, 55 ties and 1 unresolved pair.
Both diagnostic comparisons completed without refused actions. Thus
neither alternative qualifies to replace the original pilot initialization under
the stated per-mode threshold. Complete results are in
`selection-diagnostic-v1/results.json`. These repeated comparisons remain tuning
diagnostics.

While those GPU comparisons run, `mixed-pilot-v2/prepare.py` prepares a single
mixed package: **125 games / 1,025,727 frames**, including duels, teams, and
3/4/6/8-seat FFA. Its continuation waits for both
preparation and diagnostics, then trains one full epoch at **1e-5**, half the
preceding rate. It starts from the migrated original reference unless a diagnosed
checkpoint wins at least as many games in both modes, has no additional caps or
refusals, and improves the combined win fraction. This chooses an experimental
initialization only. Optimizer and trajectory state start fresh. The final pilot
checkpoint is exported and tested directly in 128 duel / 64 Commander games;
there is no selection among intermediate value scores and no automatic promotion.
Changing mixture and learning rate together makes this a bounded corrective
pilot, not a causal ablation. Runtime `package-runtime-v2/` freezes sources at
`134a2f565`; preparation, training and comparisons have separate guarded jobs.
`mixed-pilot-v2/status.json` and the shared active pointer track its continuation.
Preparation completed in **155.1 seconds** at **88.3 MiB peak PSS**. The diagnostic
finished, the controller selected `initial`, and the new GPU training job has
started; its training and playing-strength results are still pending.
