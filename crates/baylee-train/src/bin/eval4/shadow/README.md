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
Current convert4 does NOT consume these labels. A future converter must verify
all run/record/input/view/menu/encoder/catalog bindings, replay original inputs
and hashes, and reject unknown schemas or mismatches before producing separate
BC targets. This milestone is NOT end-to-end DAgger training support.

CPU fixture tests use a deterministic mock policy, not an ONNX model: shadow
off/on produces byte-identical completed engine records and rollout provenance,
including when teacher advice differs. They establish collector separation,
not learned playing strength or real-policy sampling correctness.
