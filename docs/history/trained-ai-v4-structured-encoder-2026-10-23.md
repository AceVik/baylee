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
   the same broad corpus used by the previous policy (`dataset-broad-v1`,
   expected 847 games, 6,620,782 training and 3,072,672 validation observations).
   The converter checks replay hashes and every semantic reference. Existing
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
5. **Broad continuation:** train on the complete replayed broad corpus with the
   existing B=8, U=32, observation chunk=8, decision chunk=4 and thinking-depth
   curriculum. Keep validation lanes fixed at 8 and evaluate full trajectories.
   Start with the existing cautious fine-tuning rate (5e-5); retain atomic
   last/best checkpoints and distinguish their parameter sets.
6. **Strength comparison:** export and verify nonzero references, then compare
   to the archived encoder-5 policy in the **same new eval executable**, with
   the frozen deck manifest, starting seeds and seat rotations. Keep all bad
   deals and unknown capped outcomes. Repeat K=1/4 only after a useful base
   policy is established; this change does not create future-state search.
   Final untouched tests and broader tactical/card holdouts remain required.

## Verification and operational notes

Rust tests cover Negate/Mana Leak/Remove Soul, negation vs positive filters,
And vs Or, sequence order, large numbers, twelve abilities, public card zero,
and identical actor inputs after changing an opponent's hidden deal. Python
tests check semantic dependence, gradient equivalence with table reuse, frozen
lookup equivalence, malformed DAG rejection and ONNX with nonzero references
and changing reference-list lengths. Existing recurrent/training tests and the
six original Python contract tests are included in the checks.
The complete library suite passes **62 Rust tests**; Python discovery passes
**40 tests**, including refresh after operator/DAG renumbering.

The initial migrated 54.8M model exported K=0/1/2/4/8 with maximum Torch/ORT
error **1.38e-5** on 15 synthetic frames. Real-trajectory parity and subsequent
playing-strength results are separate checks. After 100 training updates,
export passed **143 frames**, including a recurrent held-out trajectory with
nonzero rule references, at maximum absolute error **3.41e-5**. The migrated
initial policy also matched the legacy policy exactly for all outputs at
K=0/1/4 and for two actual games (answers, outcomes and fallback counts).

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
from ongoing source edits. The machine monitor remains running.
