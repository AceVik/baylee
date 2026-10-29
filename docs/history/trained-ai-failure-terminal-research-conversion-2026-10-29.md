# Separate failure-terminal research conversion, 29 October 2026

This is a conversion-only foundation for `failure_terminal_research_v1`.
It produces a distinct research observation/return format. It does not admit
data for PPO, create an ordinary dataset/cache, or implement a PyTorch loss.
The shared native boundary was extracted mechanically in `9f4fadc46` before
this behavior change. The collection boundary is documented in
[the native boundary report](trained-ai-failure-terminal-native-boundary-2026-10-29.md).

## Explicit entry point and accepted inputs

```text
cargo run -p baylee-train --bin convert4 -- research --runs RUN_DIRECTORY --out NEW_DIRECTORY
```

The ordinary `convert4` entry point continues to reject research collection
contracts. The explicit research path requires the fresh producer's version 2
research contract and exact target descriptor, a supported encoder 7–9 identity,
raw sampler version 2, FP32, K2, temperature 0.7, frozen learner/opponent
identities and physical-seat role assignments. Self-play requires identical
learner/opponent identities. It rejects BC/shadow, held-out, ordinary evaluation
and historical diagnostic artifacts. Gate A records remain diagnostics and
cannot become fresh research input through this path.

This first input adapter requires `run.complete.json` version 2, matching exact
`run.json` and `games.jsonl` digests, and every planned indexed attempt exactly
once. Its denominator contract is explicitly `complete_index_v2`. The separate
`failure_terminal_attempt_ledger_v1` operational-abort ledger is **not ingested**.
Missing games, worker crashes and partially recorded collections therefore fail
this adapter; no crash denominator or synthetic game is claimed.

## Canonical proof and provenance

Every accepted episode replays original engine inputs and snapshot hashes using
the shared native boundary implementation. Original sidecars are bound to seat,
decision sequence, pre/post snapshot hashes, authorized view, pending question,
executed action and sidecar digest. Roles, raw record bytes, source row, episode,
source index, source run, completion receipt and compressed shards receive content
seals; sources are rechecked before publication. Redirected source files and
duplicate source identities are rejected.

The replay reconstructs candidate forcedness from the supported authorized view
and chronological per-seat knowledge. A multi-choice Net action cannot be
relabeled Forced, even if an altered sidecar's digest is updated. Forced answers
require their absent-LP/zero sentinel. Scalar Net answers retain absent categorical
LP and zero sentinel; categorical Net answers require their original finite LP,
K2 and temperature 0.7. This checks structure, not numerical likelihood under
the original policy. Research feature conversion preserves original action masks
and sequences, and rejects incompatible masks rather than silently reconstructing
them. It rejects any unencoded action instead of publishing a shortened actor
history.

A natural return requires the actual final engine End, GameOver, recorded winners
and every canonical hash. A stopped prefix must reconstruct the exact supplied
boundary proof and target descriptor. Learner failed-announcement utility 0 has
null MTG outcome; unknowns have null utility and null MTG outcome. Legal voluntary
cancellation never supplies failed-announcement evidence. All returns declare
`bootstrap=false`.

## Separate output contract

The output directory must be new. Failed conversion can leave an incomplete
directory; successful publication requires `research.complete.json`, which binds
`research.manifest.json`. The manifest schema is
`failure_terminal_research_conversion_v1`; its content fingerprint binds target,
roles, model/encoder, source seals, converter identity, catalog and output seals.
It declares `ppo_eligible=false`, `ordinary_target_cache_compatible=false` and
`conversion_only_unadmitted`.

`attempts.jsonl` retains every completed-index attempt. Unknown attempts explicitly
declare `excluded_unknown_no_target_rows`, null utility and null episode file.
Known attempts reference `episodes/*.research.jsonl.gz` histories with the
`failure_terminal_research_observations_v1` schema. Each history retains learner
observations and decisions in chronological order, including passive recurrent
updates from opponent actions. Its separately named `returns` and
`training_terminal` fields do not replace engine outcomes. Legacy `outcome`,
`aux` and `is_terminal` fields are omitted. `actor_eligible` identifies original
Net decisions with an actor LP as potential samples; it does not certify
numerical policy admission.

The output includes the complete compiled card structure and DSL tables with
digests. Runtime/model catalog equivalence still needs a separate numerical
audit. No ordinary `meta.json`, `collection.json`, BST4 or training shard/cache
is published; the existing ordinary cache builder rejects this output.

## Verification and remaining integration

Real-engine synthetic fixtures cover a learner announcement loop, opponent
unknown, and natural win/loss. Their logged probabilities are synthetic test
values: these are structural converter tests, not admitted policy trajectories.
Tests exercise fixed attempt accounting, unknown exclusion, single final history
marker, unchanged scalar/actor provenance, tampered targets/roles/actions/source
files, forged Forced labels with rebound digests, incompatible masks, publication
seals, overwrite refusal, ordinary-converter rejection and ordinary-cache rejection.
Existing ordinary conversion tests cover streaming parity and encoder contracts.

Validation commands (all successful, `-j 2`):

```text
cargo test -p baylee-train --lib convert4:: -j 2 -- --test-threads=1
cargo test -p baylee-train --bin convert4 -j 2 -- --test-threads=1
cargo test -p baylee-train --features onnx --bin eval4 -j 2 -- --test-threads=1
cargo clippy -p baylee-train --features onnx --lib --bin convert4 --bin eval4 --tests -j 2 -- -D warnings
```

Results: 22 converter-library tests, 10 converter-binary tests and 17 evaluator
tests passed. The existing full two-/four-seat streaming parity regression is
included in the library suite (246.85 seconds total); it was not repeated after
the final small publication-helper extraction and test-fixture type rename.
The focused five research tests were repeated on the final code.

The debug evaluator was rebuilt incrementally and its read-only
`audit-failure-terminal` command rechecked original Gate A cases 64/65/96/97
against the original `input-seals.json` and `report.json`. New artifact:
`failed-announcement-next-protocol-01a188a9-e814-7508-b0fe-7a88e06cbedc/native-boundary-gate-a-v3.json`
under the evidence root, SHA-256
`e2f5403b339ebba87fda21c186a11344bb8d52ed4899a287a0ef26ce6b780dbf`.
All four passed with the new shared forcedness/scalar checks: 64 remains
opponent unknown with null utility before House input 303; 65 remains learner
announcement-loop utility 0 with null MTG outcome before House input 417;
96/97 remain natural controls. Prior audit artifacts and all source files were
preserved. This diagnostic does not run the new research converter on the old
records and is no numerical LP audit or fresh-producer admission.

Still required before any training experiment: the separate cache/tensor contract,
PyTorch research-return handling and loss, whole-history original-policy numerical
LP verification, actual runtime catalog equivalence, admission/reporting gates,
and an explicit adapter for the finalized operational attempt ledger. No new
collection, training, GPU operation or live game was started for this slice.

## Finalized operational-ledger adapter addendum

The next conversion-only slice adds an explicit Linux input adapter:

```text
cargo run -p baylee-train --bin convert4 -- research --attempt-ledger-runs RUN_DIRECTORY --out NEW_DIRECTORY
```

`--attempt-ledger-runs` and the original `--runs` are mutually exclusive. The
original complete-index path and its v1 output remain intact. The new path emits
manifest `failure_terminal_research_conversion_v2` with denominator contract
`failure_terminal_attempt_ledger_v1`; observation/return framing is unchanged.
Both paths remain unadmitted and incompatible with ordinary target caches.

The shared Rust `research_attempts::verify_finalized` API requires the existing
`attempts.final.json` and `attempts.complete.json`. It opens and locks only
existing ownership files, creates nothing, and never repairs a missing seal.
It reuses the finalizer's exact reconciliation logic, requires a dead matching
producer and the sealed guard's verified cleanup, recomputes the complete ledger
and requires byte-identical ledger content plus the exact completion receipt.
Canonical regular-file paths, claims, game/abort receipts, index and all shards
are checked, including orphan/partial shards. Absolute original source/guard
paths remain part of this contract; relocated copies are not silently rebound.
The converter rechecks the immutable proof and source seals before publication.

Every planned deal appears once in output `attempts.jsonl`, with its original
planning metadata, assigned flag, operational status and exact source ledger
entry. Only `recorded` game receipts enter the existing strict native episode
and feature replay. An indexed row without a completed receipt stays an
operational unknown even if its JSON line is complete. Torn index tails remain
sealed evidence and are never promoted to recorded games. Original deck schedule
and physical learner-seat rotation must match the sealed plan.

Assigned `operational_unknown` attempts retain their original failure reason,
null utility, null MTG outcome and `bootstrap=false`; they receive no record,
sidecar, episode file or target frames. `not_started` retains `assigned=false`
and null returns, and is not counted as an attempted game or unknown outcome.
The manifest preserves planned, assigned, recorded, operational-unknown and
not-started counts, plus recorded-unknown and emitted target-episode counts.
Its aggregate unknown count includes operational and recorded unknowns, excluding
not-started entries. These statuses never supply a fabricated MTG result.

The bounded native fixture uses a real Rust child under the existing process
guard. It publishes three recorded receipts, durably assigns and fails worker
3, leaves deal 4 unstarted, and exits with code 17. It leaves an orphan complete
index row, a torn tail and a partial shard. Conversion preserves counts
5 planned / 4 assigned / 3 recorded / 1 operational unknown / 1 not started,
retains the recorded opponent unknown separately and emits only two known-return
histories. No model inference or external collection occurs. Additional fixtures
reject changed source evidence, missing/corrupt completion, resealed false
ledger utility/counts, unverified cleanup and mixed adapter selection. The
existing ordinary conversion and cache-rejection fixtures remain in place.

Final checks passed on this slice:

```text
cargo test -p baylee-train --bin convert4 -j 2 -- --test-threads=1
cargo test -p baylee-train-ops --lib research_attempts -j 2 -- --test-threads=1
cargo clippy -p baylee-train-ops --lib --bin train_ops --tests -j 2 -- -D warnings
cargo clippy -p baylee-train --features onnx --lib --bin convert4 --bin eval4 --tests -j 2 -- -D warnings
git diff --check
```

Results: all 14 converter tests passed (65.74 seconds), and all 11 operational
ledger tests passed (4.04 seconds), including the existing native deadline-kill
fixture and the new read-only verification cases. Both Clippy checks passed
with warnings denied. The native failed-worker fixture confirmed that its exact
child identity exited; temporary files were removed by fixture cleanup. No
expensive full-game parity suite or release build was repeated for this adapter.

This completes only the finalized-ledger input adapter. Cache/tensor, PyTorch,
numerical likelihood/catalog audits and research admission remain outstanding;
no collection, GPU workload, training or live-game change was launched.
