# Native research tensors and input admission, 2026-10-30

The separate `research_tensor4` executable and `research_tensor` library now
turn externally pinned research-frame storage into strict model inputs. They
never construct ordinary `Step4`, BST4, MTG outcome, auxiliary, or engine-terminal
targets. Existing conversion/cache manifests stay unchanged and unadmitted.
This milestone verifies **native research inputs**, not numerical policy replay
or permission to update a model.

## Frozen inputs and admission

The caller supplies an independently frozen SHA256 for the pins file:

```json
{
  "schema": "failure_terminal_research_input_pins_v1",
  "cache_dir": "/absolute/research-cache",
  "conversion_manifest_sha256": "<frozen conversion SHA256>",
  "cache_manifest_sha256": "<frozen cache SHA256>",
  "source_checkpoint": {"path": "/absolute/source.pt", "sha256": "<SHA256>"},
  "learner_model_identity": "<exact conversion learner_model_identity object>",
  "catalog": "<exact conversion catalog object>",
  "producers": [{
    "source_run_sha256": "<original run SHA256>",
    "guard_job": "/absolute/original-guard/job.json",
    "guard_job_sha256": "<original guard SHA256>",
    "executable": "/absolute/retained/eval4",
    "executable_sha256": "<executed binary SHA256>",
    "collection_contract": "<exact original collection_contract object>"
  }]
}
```

The angle-bracket strings above describe substitutions; identity, catalog and
collection-contract values must be their original JSON objects. Paths to pinned
files must be canonical absolute regular files. No expected hash is discovered
from the artifact being admitted. Every original source run must have exactly
one producer pin. The native gate revalidates the finalized attempt ledger,
guard cleanup, exact conversion/cache seals and original frame bytes; checks
source checkpoint, model, catalog, roles, Raw/FP32 K2/T0.7; and requires original
schema-3 guard executable/path/hash/argv correspondence. It rehashes the retained
binary and original guard inputs. Both learner and opponent graph/metadata/depth
identities must occur in those original input seals, with K0 and K2 present.
Export metadata must bind the expected source checkpoint and semantic catalog.
Revision/dirty fields alone never certify a binary. Missing original evidence
cannot be added by relabelling an archived fixture.

```text
research_tensor4 admit --pins /absolute/pins.json --expected-pins-sha256 FROZEN_SHA --out /absolute/NEW/admission.json
research_tensor4 verify --pins /absolute/pins.json --expected-pins-sha256 FROZEN_SHA --admission /absolute/admission.json
```

Admission refuses an existing output. Verification recomputes the exact receipt
without repairing any artifact. The separate receipt schema is
`failure_terminal_research_input_admission_v1`; it declares
`research_input_admitted=true`, `ordinary_ppo_admitted=false`, and
`numerical_replay_required=true`. It binds pins, cache/conversion/source identities,
all histories, frame/candidate/genuine-choice counts and every attempt denominator.
No ordinary cache/admission contract is widened.

## Persistent reader and wire format

```text
research_tensor4 serve --cache-dir /absolute/research-cache --expected-conversion-manifest-sha256 CONVERSION_SHA --expected-cache-manifest-sha256 CACHE_SHA --pins /absolute/pins.json --expected-pins-sha256 PINS_SHA --admission /absolute/admission.json
```

Unadmitted numerical diagnostics instead require the explicit
`--diagnostic-unadmitted` flag and omit all three admission arguments. The modes
cannot be combined. A session verifies the cache and original evidence once,
validates tensor packing for every frame before its handshake, retains the
verified native random-access reader, and checks captured frame digests on reads.
Reuse the process across histories; there is no per-frame verifier subprocess.

The first stdout message is one newline-terminated JSON handshake with
`schema=failure_terminal_research_tensors_v1`, `content_fingerprint`, cache and
conversion SHA256s, `source_checkpoint_sha256`, `learner_model_identity`,
`encoder_version`, `structure_sha256`, the exact `structure` object, `catalog`,
`histories`, `attempt_counts`, `frames`, `actor_candidates`, `genuine_choices`,
the admission receipt or null, and explicit admission flags. Each history retains
`attempt_id`, `episode_id`, physical `seat`, `first_frame`, `frames`, source receipt
and separate research returns. Unknown and unstarted attempts remain counted and
have no histories or tensor rows. An all-unknown collection can be inspected as
a zero-row session but cannot manufacture an update batch.

Each stdin line is exactly `{"history":0,"start":0,"length":16}`. No optional
filtering fields are accepted. Length must be 1–256 and the entire range must
fit within one history. It retains every passive/forced/scalar frame in cursor
order. Starting later in a history does not claim a reset recurrent state; the
caller must replay the preceding history before using such a range for learning.

Each reply is `u64LE descriptor_length`, UTF-8 descriptor JSON,
`u64LE payload_length`, and raw contiguous little-endian array bytes. The descriptor
has `schema`, `identity`, `payload_bytes`, and `arrays`. Each array entry has
`name`, `dtype` (`f32`, `i64`, or `u8` for booleans), `shape`, `offset`, and `nbytes`.
Identity includes history/attempt/episode/seat, start/length, `new_history`,
`complete_history`, and research returns. Shapes start with the request's time
length; scalar-per-frame fields have shape `[U]`. Rust pads spatial dimensions
to the request maximum, at least one, without truncating or filtering. It does
not pad time. Requests are capped at 4 KiB and packed input/output buffers at
512 MiB each; arithmetic and spatial references are checked.

Arrays include all seventeen model inputs before recurrent state/thinking steps:
`entity_feat`, `entity_dsl`, `entity_mask`, `seat_feat`, `global_feat`, `edge_index`,
`edge_type`, `edge_mask`, `cand_feat`, `cand_entity_idx`, `cand_seat_idx`,
`cand_mask`, `pair_index`, `pair_mask`, `cand_dsl`, `entity_rules`, and `cand_rules`.
Additional arrays are `thinking_steps`, `head`, `single_idx`, `scalar_target`,
`cand_target_mask`, `cand_free_mask`, `pair_target_mask`, `pair_free_mask`,
`behavior_logprob`, `has_logprob`, `source_kind`, `temperature`, `is_decision`,
`training_utility`, `training_terminal`, `actor_candidate`, `actor_mask`,
`pick_sequence`, `pick_sequence_mask`, `pick_sequence_length`, `step_valid`, and
real entity/candidate/pair counts. Pick order and original conditional masks are
preserved. Actor candidates retain original Net+LP provenance; `actor_mask`
selects genuine stochastic choices separately. Scalars and forced/passive frames
acquire no fabricated categorical probability. Every emitted frame has a known
research utility; MTG outcome remains nullable history metadata.

Real empty candidate/pair menus retain the native evaluator's **false** masks;
the ordinary Python dataset's dummy-true masks are not copied. Known active-card
DSL takes precedence over a retained copied-token slot, matching native inference.
Token tables contain the native zero row plus all token rows. The sealed catalog
currently represents front/back only, so faces 2–254 are refused instead of being
silently mapped to front. Face 255 retains its native sentinel behavior. Invalid
edges, pair endpoints, candidate references, missing masks, nonfinite/overflowing
FP32 values and catalog references are errors. No free-mask repair or entity cap
truncation occurs.

The objective marker is
`failure_terminal_utility_minus_frozen_source_value_uncentered_v1`: the intended
actor advantage is utility minus frozen source value, without centering. This
native layer does not compute that baseline or certify original probabilities.
Whole-history numerical replay, frozen source-value evidence, minimum genuine
choice count, research-only loss/checkpoint/resume handling, rollback and a bounded
actual update are separate requirements owned by the Torch consumer.

## Verification and limits

Focused commands:

```text
cargo test -p baylee-train-ops --lib research_ -j 2 -- --test-threads=1
cargo test -p baylee-train-ops --test research_tensor_cli --test research_cache_cli -j 2 -- --test-threads=1
cargo clippy -p baylee-train-ops --lib --bin research_tensor4 --tests -j 2 -- -D warnings
```

The final combined suite passed 23 tests in 24.13 seconds (11 operational ledger,
six lossless cache, six tensor/input-admission). Actual native-process CLI tests
passed for cache (one test, 1.35 seconds) and tensor/admission (one test, 1.59
seconds). The final strict Clippy command passed in 3.43 seconds; `git diff --check`
was clean. Tests cover passive/scalar history, genuine/candidate
distinction, original pick order and masks, separate null MTG loop outcomes,
5 planned / 4 assigned / 3 recorded / 1 operational unknown / 1 not started,
zero-target sessions, bounded multi-request wire decoding, malformed references,
FP32 overflow, externally pinned hashes, receipt/executable/source tampering,
and ordinary admission rejection. The test graphs, checkpoint, executable
evidence and LPs are explicitly synthetic fixtures: they exercise the contract,
not numerical model equivalence or fresh-data learning admission.

No collection, GPU workload, live game, model update, archived-artifact rewriting
or push was launched by this native implementation work. The parent experiment
must produce separate real producer/replay/update receipts before claiming an
end-to-end learning result.
