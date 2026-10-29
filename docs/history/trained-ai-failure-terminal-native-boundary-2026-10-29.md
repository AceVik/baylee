# Native failure-terminal collection boundary · 29 October 2026

The opt-in `eval4 --collection-purpose failure_terminal_research_v1` is a
native collection foundation for the separately proposed research MDP. It is
not admission, a training converter, or authorization to run an optimizer.
Ordinary evaluation and `on-policy-training` retain their recovery behavior.

The initial implementation requires Encoder 7–9 (checked before collection),
raw FP32/K2/temperature 0.7, recording,
training deck provenance, `--no-baseline`, and a frozen neural opponent. This
keeps both seats under explicit control. Deliberate house-profile opponents
are not yet supported on this research path; their ordinary evaluator pump
would otherwise conceal opponent guard boundaries.

Before applying either an outer repeated-view fallback or an internal House
answer, research collection stops. Unsupported/inference failures and engine
refusals also stop without House recovery. An answer cap remains unknown.
The unexecuted fallback proposal, acting authorized view/pending, last included
input index, byte boundary, record SHA-256 and engine/view hashes are retained.
The engine receives no fabricated concession and no research terminal action.

For stopped prefixes, the collector separately replays every canonical input
and verifies every recorded engine snapshot hash. Each controlled input must
have one original Net/Forced sidecar; House/Search/external origins and invalid
behavior provenance are rejected. Deterministic scalar `Net` answers legitimately
keep `has_logprob=false` and zero LP, as the existing sampler specifies; they
remain recurrent history without becoming actor samples. Per-action bindings captured before execution
seal the original sidecar digest, acting seat, decision sequence, pending,
view hash and action, then the accepted engine hash. Replay checks these
bindings. Original masks, pick sequences and behavior log-probabilities are
preserved. Forced choices retain no actor log-probability. This is structural
sample-to-executed verification, **not** a numerical policy likelihood audit.

The versioned predicate requires an actual repeated-view guard on the learner
and at least two automatic announcement rollbacks for the same source card,
each from an announcement that includes the guard view or returns to it.
The original internal/outer guard threshold is independently checked against
the preceding view visits. Replay must show an announced source disappearing
after an accepted non-`CancelCast` action and either the exact pre-announcement
engine snapshot restored, or a newly journaled native `Unpaid` cancellation.
This accommodates `continue_cast_wizard` reversals, which deliberately journal
nothing. It does not infer rollback merely from a card returning to hand.
A deliberate `CancelCast` does not qualify even when it restores state. Paid
completion does not qualify. Unproved other reversals remain unknown.

A qualifying boundary has `termination_kind=learner_announcement_loop`,
`training_utility=0`, `mtg_outcome=null` and `bootstrap=false`. Opponent failures,
unverified loops, unsupported answers, refusals and caps have
`termination_kind=unknown` and null utility/outcome. Natural engine endings
have `termination_kind=mtg_terminal`, their actual MTG outcome, and the separate
natural utility. The declared zero is research shaping, not an observed MTG
loss. All finished attempts remain in `games.jsonl` and the research summary;
unknowns are not replaced or included in an MTG aggregate.

The collection contract and completion receipt use version 2 and bind the
predicate and utility mapping. `training_data` remains false. Research results
are marked `unadmitted_native_collection_only`. Existing `convert4` explicitly
rejects the new purpose or research-target marker instead of treating it as
ordinary untyped data; old complete-game admission remains unchanged.

Before any fresh research batch, complete the fixed four-case offline audit
from the external failed-announcement protocol and review the new predicate.
The read-only entry point is:

```text
target/debug/eval4 audit-failure-terminal --runs <exact-original-run-dir>[,<dir2>] --cases 64,65,96,97 --input-seals <existing-seals.json> --reference-report <existing-report.json> --out <new-audit.json>
```

It retains all four requested case keys, source digests, the actual first House
input and unchanged preceding sidecar digest. Known natural controls replay
through their real terminal. Action bindings reconstructed for archived inputs
are labeled diagnostic; they do not recertify old artifacts or substitute for
original collection provenance. Original source artifacts remain immutable.

Still required: independent research admission/receipt, exact role/source/shard
binding and tamper checks, complete recurrent numerical
LP audit, separate target converter/cache identity, research-aware Torch target
and checkpoint/resume/export contract, and bounded operational abort handling.
An external process/resource abort currently leaves the existing worker
attempt artifact and no certified episode; it is never converted to utility.
No full attempt-ledger completion is claimed for crashes or process kills.
No Gate B collection, optimizer update or live promotion follows automatically.

## Fixed-source audit result

The four originals in `e3-mixed-collection-preparation-01a18860-9237-7420-b8df-03b22d156d51/collection/`
were audited without new games. Run/index/shard SHA-256 seals, whole original
sidecars, role ownership, original recovery diagnostics and exact prefix bytes
were checked against the prior
`failed-announcement-gate-a-01a188ae-1c79-7780-a732-dd380715ffdf` input seals
and report. The new artifact is
`failed-announcement-next-protocol-01a188a9-e814-7508-b0fe-7a88e06cbedc/native-boundary-gate-a-v2.json`
under the external investigation directory. All four cases replayed with every
canonical snapshot hash matching.

| Case | Learner / failing seat | First House input / last included | Result |
| --- | --- | --- | --- |
| 64 | learner 0 / historical 1 | 303 / 302 | Opponent failure, unknown utility |
| 65 | learner 1 / learner 1 | 417 / 416 | Research learner loop, utility 0, MTG outcome null |
| 96 | learner 0 / no recovery | Natural engine terminal | Unmodified natural control |
| 97 | learner 1 / no recovery | Natural engine terminal | Unmodified natural control |

Both loop cases have three prior guard-view visits and three verified source
rollbacks. Case 64's prefix is exactly 57,742 bytes, SHA-256
`98002b3ff035389967d01ff4766f1a49ed0fdab75f3417002664798c87ee93a7`;
case 65's is 72,601 bytes, SHA-256
`1e11ea00d26326557e8933dd626e27a232bfec429b0038972901a5c65bb37a1c`.
Their original sidecar-prefix seals are respectively
`5a99c8c3b6187ce2cc3c2f33cb2d843594790268f08b55730351d02ecd4479c6`
and `e0dc4c922c73f77f565df160fce28f304fbc6e7dfce87b9c23b966831316f257`,
matching the prior report. No post-recovery natural result supplies a target.

The initial `native-boundary-gate-a-v1.json` is retained as a failed attempt:
an overly strict new guard rejected legitimate scalar Net/no-LP observations.
The correction follows the existing sampler contract and preserves their absent
actor probability. A separate temporary diagnostic artifact records the exact
offending inputs (116/329/163/232 for cases 64/65/96/97). The native v2 result
is still development evidence, not independent evaluation or admission.

The focused native tests cover real Fireball unpaid rollbacks, opponent ownership,
legal cancellation, paid completion, unknown cap/refusal semantics, hash/sidecar
tampering, and the collector stopping before applying a proposed House action.
