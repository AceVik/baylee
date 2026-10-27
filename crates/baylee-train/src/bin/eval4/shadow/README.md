# Shadow teacher sidecar, schema 1

This opt-in collector foundation adds `--shadow-teacher sharp|expert` to eval4.
It requires Raw policy, encoder7, current provenance2, a source checkpoint hash,
and a fresh `--record-out` directory. It does not alter executed actions, RNG,
observation calls, behavior log probabilities, or engine records.

`run.json` declares the shadow contract and catalog hash. Each game writes a
bounded streaming `shadow/eval4-NNNNN.draft`. Only a completed original engine
record allows publication as `.jsonl`: header, zero or more labels, completed
footer. Draft/publishing files are NOT usable training labels. Answer-capped
files remain drafts with an explicit incomplete footer. Atomic hard-link
publication refuses overwrites.

Every label binds the authorized view hash (existing FNV observable-view
contract), SHA256 of pending/encoded state/exact runtime candidate menu,
chronological decision sequence, and the original record input's byte offset,
line number, seat, executed action, and resulting engine hash. The completion
footer binds SHA256 of the entire uncompressed record. The header binds SHA256
of run.json and its behavior model identity, hence encoder/model/graphs/source
checkpoint and evaluator identity. No account identities, credentials, full
views, or card text are copied into labels.

The teacher is fresh per query: public teams and an independent deterministic
seed, ordinary `HeuristicAgent::act(&PlayerView, &Pending)`, no scouting or
engine context. Expert here is explicitly view-only and does not peek at the
library. There is no speculative teacher history to resynchronize after the
learner executes a different action. This differs from a full teacher rollout
with remembered planning/scouting. Refitted, illegal, unmappable, or
non-roundtripping answers are excluded and counted. Only actual Net decisions
are eligible; forced/House/Search decisions are excluded. A refused NN action
cannot acquire a label for its subsequent fallback.

Teacher targets are BC-only House advice, with no teacher behavior probability
and no PPO certification. Executed behavior provenance is retained separately.
`convert4 --shadow-bc --include-passive-obs` explicitly consumes these labels
into a NEW BC-only dataset. It verifies all bindings, replays original inputs
and hashes, and re-runs the teacher on the reconstructed view before producing
separate BC targets. See `convert4/shadow_bc/README.md` for the admission and
binary companion provenance contract. Ordinary conversion and PPO data stay
unchanged. CPU fixture coverage does not replace a production ONNX rollout
audit, downstream packaging/admission checks or a controlled DAgger experiment.

CPU fixture tests use a deterministic mock policy, not an ONNX model: shadow
off/on produces byte-identical completed engine records and rollout provenance,
including when teacher advice differs. They establish collector separation,
not learned playing strength or real-policy sampling correctness.

For a persistent CPU producer/consumer fixture, the engine integration test has
an explicit `cfg(test)`-only export switch. Choose an absolute, nonexistent
output directory (its parent must exist):

```
CARGO_BUILD_JOBS=4 BAYLEE_SHADOW_TEST_EXPORT=/absolute/NEW-fixture \
  cargo test -p baylee-train --features onnx --bin eval4 \
  shadow::tests::shadow_preserves_executed_record_and_binds_only_bc_labels \
  -- --exact --nocapture
convert4 --runs /absolute/NEW-fixture/raw-run \
  --out /absolute/NEW-fixture/dataset --include-passive-obs --shadow-bc \
  --threads 1 --val-every 0
```

The test exports the actual run/record/collector-sidecar bytes and gzip JSONL
original/BC steps. The converter then writes its actual BST4 dataset and companion
provenance. This tests Rust→Python contracts without hand-written copies of the
companion format. The actor and LP remain explicit fixture stubs; no ONNX session
or GPU is used, and this is not a production rollout or sampling audit.
