# LLM collector recovery, 2026-10-28

The native collector is running again at `/home/ace/baylee-data/v5/llm-teacher-20261028`, starting with attempt 409. It uses the existing private gateway on loopback port 28768 and 50 concurrent Argon/Flash games. Continuous v5 training, both evaluation arenas and the user gateway were left running.

The old collector had restarted during the recovery. That restart overlapped cleanup of its generated credential files and accounts: 50 games lost lobby presence, followed by 303 failed websocket-ticket requests with HTTP 401. The old implementation incorrectly described those rooms as closed while the gateway still reported 50 games. The collector stopped at its assignment boundary; its dedicated gateway service was then stopped. Only after verifying that its cgroup was gone and its port closed were historical room lifecycles reconciled. Original receipts remain unchanged, and failed games remain excluded.

The source changes in `crates/baylee-train/src/bin/llm_collector4/` provide:

- Separate lifetime process locks for collector and gateway supervisor; verified gateway/database/agent readiness, child liveness and private child environments.
- Memory-only account credentials, gateway-valid usernames, websocket tickets and retained seat stdin for tethered processes. Account creation retries require verified cleanup of the preceding account.
- Owned seat process guards and explicit cleanup verification. Uncertain ownership blocks new assignments. `DRAIN` finishes current games without opening more; `STOP` and signals cancel active work. A failed worker now produces a failed controller result rather than silent success.
- Full deterministic engine replay through every recorded input, including inputs without a public view change, in addition to the gateway replay tape. Hash checks must reach the final input number; input numbers need not be contiguous.

Verification: 18 focused Rust tests, binary Clippy with warnings denied, and release build passed. Three new games (attempts 413, 425 and 436) subsequently completed and were admitted after technical verification; their pinned artifact hashes were checked. None used house-AI fallback. This verifies the collection setup, not model playing strength, and does not upgrade historical records to the new replay guarantee.

The monitor was rearmed for the new Linux process identity without clearing delivered event IDs. Its attempt floor is 409, so already-reviewed historical failures do not become current failure alerts. Twelve monitor tests passed. Detailed deployment hashes, original room states, lifecycle evidence and activation checks are retained in `history/native-recovery-20261028/` below the collection root.

At handoff the collector PID was 190701 and gateway supervisor PID 190594. These are observations, not durable identifiers: recheck PID, boot ID and start ticks before acting. For planned maintenance, create `DRAIN`, wait for collector exit and verify room closure before restarting. Do not delete active accounts or restart the gateway during games. These changes are uncommitted in the integration checkout; repository landing still requires the normal local gates.

## Training ownership, Flash transition and Terra pilot

Codex took training coordination from Argon on 28 October. Argon explicitly
acknowledged the handoff and went idle. Its existing training child was kept
alive: controller PID 113844, root
`/home/ace/baylee-data/v5/experiments/continuous-v5-training-20261028`.
The controller already schedules rounds through 12. Do not start another GPU
trainer while it owns the GPU lock. Both old Argon evaluation arenas were
drained at game boundaries. The user's live gateway was left running.

The owner permits all remaining 17% agy quota to be used, with the overall
reset expected in four days; new games should use Gemini 3.8 Flash low and
existing Argon games may finish. The primary mixed collector therefore drains
and a separate Flash-only lane was started. The planned replacement is 34
primary Flash games plus 16 lane9 Flash games. This is an operational setting,
not a measured provider concurrency limit.

At 15:57 Berlin time, receipts contained **351 distinct admitted game hashes**
across 36 decks, with 96,136 model answer sends. Sixteen games contained some
house fallback; strict action provenance still excludes fallback and ambiguous
policy labels. Counts of transcript answers are not counts of accepted labels.
Round 7's frozen LLM dataset contains 137 games and 299,580 observation frames.
Round 6's summary incorrectly says 123 LLM games: its reused frozen dataset
actually contains 84. Round 7 was independently checked and does contain 137.

The round-6 LLM stage reduced held-out LLM choice NLL from 1.595082 to 1.359976
and increased active-choice agreement from 45.1117% to 47.2067%. This is a small,
reused five-game development set. The earlier matched 14-game training pilot
did not establish a playing-strength improvement: duel score difference
-1.171875 percentage points (95% CI -5.078 to +2.734), Commander +1.5625 points
(-2.344 to +5.469). Subsequent mixed-stage gains cannot all be attributed to
LLM data. A fresh paired gameplay comparison is still required.

Three training-controller bookkeeping issues remain for a controlled update:
existing dataset reuse does not bind the newly selected receipts; arena indices
use synthetic rather than recorded seeds/deck identities; and the summary
reports the base live dataset instead of the actual mixed live/arena dataset.
The audit found no actual overlap with the five held-out record hashes. It also
confirmed that v5's own arena actions have zero imitation weight; the declared
Argon seat supplies the arena policy targets. Preserve the current frozen
artifacts and running trainer when fixing these issues.

The collector now accepts a pinned `--pairings-file` and optional
`--max-attempts`. Profile identity is resolved through the seat's actual
configuration/declaration code, rather than a hardcoded model map. Config and
pairing hashes are checked before room creation and after play. The bound
counts assignments, including failed attempts; a bounded run need not contain
that many valid games. `CODEX_HOME` is forwarded without copying credentials.

The initial Terra-low versus Flash-low pilot used
`codex:gpt-6.1-terra`, effort `low`, through the existing tool-restricted Codex
seat adapter. Its first completed Constructed game ended on turn 16 with Flash
conceding to Terra. Terra made 37 model calls and supplied 66 mind answers;
both seats had no fallback, retries or refused answers. Terra exercised
mulligans, lands, spells, activated abilities, targets and attacks. This one
unequal-deck game establishes working gameplay, not superiority over Flash.

That game exposed an admission bug: the record audit hardcoded provider `agy`,
so it rejected a correct Codex declaration before recording room closure. The
fix compares the full expected declaration, including provider, model and
effort, and separates completed-game lifecycle evidence from training-data
admission. Cleanup failure evidence also retains the original error. Twenty-four
collector tests, targeted Clippy with warnings denied, and the release build
passed. The immutable corrected binary is
`/home/ace/baylee-data/v5/ops-codex-20261028/bin/llm_collector4-eb0a2989527d`.
Original pilot receipts remain failed; no failed record was relabelled as
admitted. A separate bounded two-game Terra self-play smoke test verifies the
corrected path without consuming agy quota.

Two further setup/lifecycle issues were found during the live check. When a
finished room disappeared and one seat had exited without writing GameOver,
the absent-room branch bypassed the check for exited children. The worker
could wait until its one-hour timeout. The corrected worker checks child exits
first and can prove lifecycle closure from a fresh official completed export
for the exact owned game, while continuing to reject incomplete transcripts.
The old Terra controller was stopped only after both games had official End
records and no owned seat/engine processes. Their lifecycle recovery evidence
is archived under that pilot's `history/closure-recovery-*`; failed admission
receipts are unchanged. A distinct old primary orphan, attempt 460, remains
for maintenance once the other games finish; the private gateway has no admin
token configured, and no engine was killed by hand.

Room creation also omitted `setup`, inheriting the lobby default of 40 life
and one free mulligan even for Constructed decks. New requests explicitly use
20 life/no free mulligan for Constructed and Highlander, and the existing
friendly Commander convention of 40 life/one free mulligan. The request pins
the intended setup, and admission checks the actual recorded seat life and
mulligan rules. Historical 40-life games are valid house-rule data, but their
deck win rates are not standard 20-life duel results. The first Terra win and
the two self-play smoke games used that historical setup. The corrected
collector revision for subsequent starts is pinned in
`ops-codex-20261028/closure-fix-deployment.json`.
Its SHA-256 is
`1150e9f66aca6e6d7916843b0ed45bc03af5f80773edcd08af8a0d041eb1c5a9`.
Thirty offline collector tests and targeted Clippy passed. A live setup-only
test created one empty waiting room, configured and confirmed 20/0 through the
existing gateway route, removed the room, verified its absence and deleted
the temporary account. It started no engine or model. The 34 mocked tests for
monitoring, probe admission and deferred collector resumption also passed.

The first Terra self-play game subsequently passed full collection admission
and exact replay, with 400 model answer sends and no house fallback. Visible
decisions included choosing Absorb over available Counterspell at five life
to gain three life, and building Jace to its ultimate before emptying the
opponent's library and waiting for the draw-loss condition. These examples
show tactical awareness and a coherent plan, but are selected observations
from one game rather than a strength benchmark.

At 16:27 the delayed lane9 success event
`7a969153e1471db61c41b6ab` was reviewed: attempt 3 contains 267 Flash answer
sends, zero fallback, and unchanged artifact hashes. Its existing admission
was retained without counting or starting it again. The old primary collector
had meanwhile failed on unresolved room closure. Its unit's automatic restart
was stopped; both primary and lane9 now use `Restart=no`, so a failed drain
cannot bypass the reviewed quota/probe transition. Lane9's active games and
the GPU trainer were left running. The primary's unresolved old rooms still
require maintenance after genuine games finish, before resumption is allowed.

The subsequent primary failure event `a5a67a708b2aea237b042987` was checked
against PostgreSQL and process ownership. Attempts 460, 547, 558 and 574 have
no official End and each retain an orphan engine, with no seat processes.
They remain excluded. At the check, other Flash games and Terra's second
smoke game were still active on the dedicated collection gateway, so that
gateway was not restarted. A read-only readiness watcher is armed to notify
this session after all collection controllers and their clients have exited;
maintenance must recheck ownership before touching that gateway. It checks
the three specific collector units, their process identities, terminal root
states, empty cgroups and the unchanged dedicated gateway identity. All 36
mocked operational tests pass. The user gateway and GPU trainer are separate
and remain running.

Both Terra smoke games have now completed and passed artifact-hash review:
700 model answer sends in total, no house fallback. The bounded controller
exited after two assignments, and its service is inactive. These remain
40-life integration games, not a relative-strength benchmark. Their review
is retained in `ops-codex-20261028/terra-smoke-reviewed.json`.

During the initial Commander game, Flash reported **Individual quota reached**
with a reset approximately three hours away. This is separate from the owner's
four-day overall quota reset. New Flash assignments were drained. A single
Flash-versus-Flash availability game is scheduled for **19:05 Berlin time**;
the larger collectors may resume only after that game completes with verified
artifacts, the expected Flash identities and no fallback. They also require
old controllers to have exited, verified room closure and unchanged pinned
inputs. A Terra/Flash comparison needs all eight deck/model/seat rotation cells
before even its small descriptive score is balanced; the server-generated
deals are not matched seeds.

Operational scripts, pinned configurations, process identities and deduplicated
watchers live under `/home/ace/baylee-data/v5/ops-codex-20261028`. They monitor
training round completion, the bounded Terra test, quota-probe failure and
collector transitions. Always inspect their current state before acting on a
wake event. Runtime files and services are local operational artifacts, not
repository release assets.

### Round 7 review and round 8 recovery

The `training:05941e0066b4077b:round-7` wake exposed a stale status: the
controller had exited during round 8 preparation, although `status.json` still
said `round-0008-convert-llm`. Different collector roots both owned an
`attempt-000000003`, and freezing their evidence into one directory failed
with `FileExistsError`. Rounds 1–7 and their checkpoints were intact. The
failed partial snapshot and old status were retained under the continuous
run's `recovery-round8-20261028T1650` directory.

Frozen storage keys now include the canonical source and decompressed record
identity. The original attempt, request, receipt and technical-result bytes
remain unchanged. An optional `frozen_game` alias lets the native converter
find namespaced storage while verifying the original `source_game` against
admitted evidence. Duplicate record content still fails admission. Four
Python preparation tests and six native LLM provenance tests pass.

The local controller now reuses datasets by exact frozen source identities,
keeps an interrupted round's snapshot authoritative, records actual dataset
counts and paths, and writes failed status under its ownership lock. Holdouts
are identified by exact primary source and record content across lanes,
instead of excluding unrelated attempts with the same number. Arena indices
use actual record seeds and deck-content identities. Six controller tests
pass. Old summaries remain historical evidence: round 6 reported 123 LLM
games but actually used 84; round 7 used 137.

Round 7 final versus round 6 final on the five-game LLM development set:
choice NLL 1.26660 → 1.20708, active-choice agreement 47.35% → 47.77%, and
critic MSE 0.10267 → 0.11373 (worse). The house-teacher validation critic MSE
improved 0.15779 → 0.14964. Within round 7, active house-teacher agreement
fell from 64.05% after the LLM stage to 59.76% after the live/arena stage.
These reused validation sets do not establish playing strength. A separate
CPU comparison of round 6 final, round 7 LLM-stage and round 7 final is
prepared with matched seeds, deck/seat rotations and a sharp opponent.
No model was promoted.

Recovery continues from round 7's exact final checkpoint under
`baylee-continuous-v5-training.service`, with the original twelve-round
budget and controller/GPU ownership locks. Round 8 freezes 346 unique
non-heldout LLM games from nine collector roots. The converter is pinned as
`ops-codex-20261028/bin/convert4-a49a379aef58`. The monitor is rebound to the
new process identity and starts notifications at round 8, retaining its
previous delivery history.

The recovered converter successfully replayed all 346 games in 39.7 seconds,
producing 699,803 training frames with zero unknown vocabulary cards. Their
346 record identities have no overlap with the five held-out records. The
GPU trainer was observed making optimizer progress from the round 7 source
checkpoint. The CPU review was started independently as
`baylee-round7-cpu-review.service`; its optional watcher uses exact process
identity and deduplicated completion/failure notifications (39 operational
watcher/transition tests pass). No comparison result or model promotion is
claimed yet.

Only 27 GB of disk space remained. With no Cargo/rustc processes active,
45.28 GiB of regenerable debug incremental build cache was removed; the
selected paths are recorded in `ops-codex-20261028/build-cache-cleanup.json`.
Training data, failure evidence, checkpoints and runtime binaries were
preserved. The filesystem then had approximately 71 GB available.

### Final Flash drain and isolated gateway maintenance

Event `322d504bfcf5e2a4c692ff83` concerned lane9 attempts 25 and 26. Both
reported Flash's individual quota limit; they contain 18 and 30 house
fallback answer sends respectively. The older collector also failed to
confirm lifecycle closure after one transcript received GameOver and the
other closed. Both original error receipts remain excluded from training.

After all collectors and model-seat clients had exited, a process, cgroup,
socket and database audit found no external player on private port 28768.
Nine unresolved rooms had complete official End records; eleven other rooms
contained only orphan engines (four primary, seven lane9). All engines were
bound to these known rooms. The separate user gateway on port 28766 and its
neural seat remained active.

The private gateway service was stopped, its old cgroup and listening port
were verified empty, and twenty room lifecycle states were reconciled. Failed
receipts, requests and technical results were hash-pinned and unchanged;
incomplete games were explicitly marked incomplete, never converted to wins,
draws or admitted samples. Original room/status files and gateway logs were
archived in `ops-codex-20261028/maintenance-20261028T1700/`. Historical ephemeral
accounts were retained because their process-local credentials were gone;
no direct database account deletion was performed.

The restarted private gateway passed health checks with zero running/waiting
games. Both collectors remain inactive with DRAIN present. The 19:05 Flash
probe and the clean-probe/configuration gates still control resumption.
Training and the paired CPU evaluation retained their original live process
identities throughout maintenance. The old maintenance watcher was retired
after recording completion.

### CPU review timeout and checkpointed recovery

Event `cpu_review:4b5b2a209e4095c6:terminal` reported the first full CPU
comparison failing. The guardian terminated `eval-r6-final` at its 1,200-second
deadline; peak proportional memory was 2.16 GB. Captured logs did not report
a model or engine exception. The three preceding two-case smoke runs had
completed in approximately 80 seconds each without fallback, refusal or caps.
The deadline was too short for the planned 64-case CPU run. Its artifacts
remain unchanged, and no incomplete score is treated as a comparison result.

Recovery uses a fresh UUID namespace under
`ops-codex-20261028/review-round7-recovery1/` and the same frozen models,
runtime, 29 decks, seeds, seat rotations and sharp opponent. Each model's
64 cases are evaluated in four-case chunks, alternating the three models.
Each completed chunk retains its actual job receipt and output before the
next starts. Final aggregation requires the exact complete case set, matching
protocols and unchanged repeated baseline outcomes; capped games remain
unresolved. Five tests cover composition and rejection of mismatched inputs.

`baylee-round7-cpu-review-recovery1.service` runs on the CPU at low priority,
with a 20-minute limit per chunk and a four-hour overall job budget. A failed
chunk stops the controller and preserves earlier results; it does not silently
drop cases or retry. The first two chunks completed and passed verification.
The watcher is bound to the new controller's PID, start ticks and boot ID,
with prior notification history preserved. No model was promoted. The GPU
controller continued uninterrupted into round 8's LLM training stage with
346 frozen games; the user gateway stayed running.

### Completed paired comparison

The recovery completed all 48 chunks in 81.7 minutes. Independent review
verified the actual job/output hashes, all 64 scheduled cases for each model,
the 32 table/seed clusters, repeated baseline results and all three paired
reports. No cases were dropped. There were no capped games, refused engine
actions or evaluator-level stalls.

| Policy against sharp | Wins / 64 | Recorded fallback actions | Games with fallback |
| --- | ---: | ---: | ---: |
| Round 6 final | 21 | 21 | 3 |
| Round 7 after LLM stage | 18 | 8 | 2 |
| Round 7 final | 19 | 11 | 3 |
| Steady house baseline | 29 | — | — |

Round 7 final improved two matched outcomes, worsened four and matched 58
relative to round 6 final. Its observed difference is −3.125 percentage
points; the table-cluster bootstrap 95% interval is −10.94 to +4.69 points.
Every paired interval includes zero. This development screen establishes
neither a playing-strength gain nor a reliable regression, and no candidate
was promoted. Better imitation loss on the small reused LLM validation set
did not translate into a demonstrated advantage in these games.

All 40 recorded fallback actions were `PassPriority`, each immediately after
the neural actor cancelled a cast. Saved records show repeated sequences of
announcing a spell, choosing X, selecting mana sources/colors and cancelling
before the fallback pass. The runtime has a repeated-view loop guard, but the
sidecar records only the `House` source tag, not the exact fallback branch;
the records alone cannot conclusively distinguish that guard from an inference
failure. These results therefore describe the executed policy with its guards,
not entirely unassisted neural play. The zero evaluator-stall counter does not
certify that the separate runtime guard never intervened.

The integrity audit and per-fallback action histories are retained alongside
the recovery controller in `completed-audit.json` and `fallback-reviewed.json`.
The next diagnostic focuses on payment/cancellation behavior and its encoded
context and teacher targets. Evaluation seeds remain excluded from corrective
training. Round 8's existing GPU training continues; later candidates require
another gameplay comparison before replacing the live model.

The focused source review found a concrete observation gap:
`features7/state.rs` marks a visible public object with feature 33 when it
matches `view.casting`, but `push_hand_object7` receives no casting context
and leaves that feature unset. A spell being announced from hand therefore
lacks that explicit per-card marker in its hand entity. This does not prove
the network cannot infer the source from recurrent history, nor that it caused
the observed loops. Exact owed mana symbols are already encoded in global
features 64–75; describing mana costs as entirely absent would be incorrect.

There is an additional distinction at nested mana-color questions:
`Engine::cancellable_cast` deliberately returns `None` while a mana ability
is resolving. `PlayerView.casting` is cancellation permission, not a persistent
identifier for the announced spell. Marking the hand entity alone therefore
does not solve color-prompt context. Any added announced-source context must
remain separate from cancellation permission, respect the authorized view,
and survive the nested question without enabling an illegal cancellation.

The next implementation must test the announced hand card through X and mana
color prompts, cancellation and completion, while retaining the legacy
encoder contract for existing data and checkpoints. A corrected, versioned
encoding needs matching conversion, training and inference paths and rebuilt
data caches. Do not silently change the interpretation of the currently
running round's tensors. Separate fallback reason/visit diagnostics should
also distinguish repeated-view guards from inference/decode failures.


## Isolated Encoder-8 payment-context implementation

The isolated `baylee-ai-payment` checkout now implements an opt-in observation
contract for BayleeAI v5 relational revision 2. Encoder 7 and the current
training/checkpoint contract remain unchanged. An owner-only optional
`announced_cast` persists through X and nested mana-color questions; the new
Session payment-context methods expose it without changing default/socket or
existing `with_context` views. Cancellation permission stays separate.

Encoder 8 marks the announced hand/public entity in feature 33, retains global
29 as cancellation permission, adds global 36 as announcement presence, and
uses globals 37–39 for exact numeric bounds and a stable prompt-kind code.
Widths and candidate sampling are unchanged. Legal X bounds do not certify
mana affordability. Conversion, training identity and offline native evaluation
select Encoder 8 explicitly; live seats and legacy arena reject it pending
suitable context support. A 7-to-8 weights-only warmstart requires an explicit
opt-in through the training flag, not artifact relabelling or resume.

Confirmed focused Rust checks: two gamehost choice-context tests and one engine
miracle-payment test passed. The real X/dual-land test verifies that the hand
card remains visible during nested color choice, opponents receive no source,
legacy serialized views are unchanged, and `CancelCast` remains rejected there.
The four Encoder-8 state/clock tests, 29 conversion/live/model-identity tests,
two Codex-provenance tests, five converter-binary tests, twelve seat-binary
tests and seven evaluator-binary tests also passed. The conversion tests include
two- and four-seat replay and retain zero unencoded decisions. This implementation
is not evidence of improved playing strength and does not promote or replace a
running model.

Python CPU validation passed 42 tests: 31 encoder-contract/export/legacy-BC
checks and 11 policy-diagnostic checks. The first invocation selected
`test_encoder8_contract`, `test_shadow_dataset4`, `test_llm_dataset4`,
`test_bc_mixture`, and the two `test_export_models.ExportModels` cases
`test_loader_rejects_wrong_contracts_and_state_shapes` and
`test_v5_architecture_is_mandatory`; the second selected
`test_policy_diagnostics4`. Both used `python -m unittest -q` from
`tools/trainer` with the existing trainer virtual environment,
`CUDA_VISIBLE_DEVICES=''` and `PYTHONDONTWRITEBYTECODE=1`.
`Encoder8Contract.test_full_cpu_export_emits_checkpoint_encoder8` created an
actual tiny v5 relational-revision-2 Encoder-8 checkpoint, ran the exporter,
and verified the exported model5/encoder8 metadata. CPU ONNX Runtime parity
passed K=0/1/2/4/8 across 15 frames with maximum absolute error 9.54e-7.
An end-to-end CPU smoke then converted two actual games from the existing
training corpus: 732 training frames and 703 validation frames, zero unencoded
decisions. A tiny 0.77M-parameter model completed two updates; its five ONNX
graphs passed 25-frame parity (maximum absolute error 9.54e-7). Native `eval4`
loaded Encoder 8 and produced nine neural answers without refusal, fallback or
stall. That smoke deliberately stopped at its 64-decision limit; it is no
playing-strength or latency benchmark. Verification and artifact hashes are in
`/home/ace/baylee-data/v5/ops-codex-20261028/payment-context-implementation/verification.json`.

## Balanced Terra/Flash pilot and training admission

A fresh eight-cell pilot runs under `baylee-terra-flash-balanced.service`, with
one game at a time, two deck orientations, two model-seat orientations and two
replicates. It uses Constructed 20-life/zero-free-mulligan settings. Server seeds
are retained but cannot be prescribed here, so these are not identical-deal
comparisons. Root:
`/home/ace/baylee-data/v5/experiments/terra-flash-balanced-20261028/01a183b8-b5c3-7c58-acb7-c878ed6243db`.
The first game was admitted without fallback (Terra 41 model answers, Flash 57;
Flash won). The next cell was running when this note was written. The OPS
monitor is bound to the wrapper's PID/start-ticks/boot identity.

The replay converter formerly required every CLI declaration to name `agy`.
Terra admission now requires an additional pinned `input_receipt`, independently
bound to the technical receipt, requested models and exact declared minds.
Only `codex:gpt-6.1-terra` at low effort is added; provider/model/effort/kind
spoofs are rejected. Legacy agy manifests remain supported. Eight preparation
tests and nine LLM/encoder companion checks passed. The actual first Terra/Flash
game was frozen and converted into 708 Encoder-8 frames, with zero unencoded
decisions, and passed Python dataset admission. It remains BC-only, never PPO
behavior evidence. The running GPU experiment and its frozen inputs were not
modified; no model was promoted.

## Shared gateway pacing and waiting-room cleanup

The resumed primary and lane9 collectors independently paced their launches
at 2,200 ms. Together they could exceed the private gateway's shared
30-per-60-second chair-redemption limit for loopback. Fourteen attempts failed
while seating the second model, before any game-start request or transcript;
each second-seat diagnostic reported a rate-limit mention. This supports a
gateway-startup diagnosis, not a new provider-quota conclusion.

Those failures exposed a separate cleanup bug. The host seat had already left
and removed its waiting room. The collector's second leave could then fail,
and an `&&` short circuit skipped the independent lobby query. The collector
consequently blocked itself on an already-closed room. Cleanup now always
checks the authenticated exact-ID query after its best-effort leave. It only
accepts a complete empty first page (`offset=0`, `limit=100`, `total=0`,
`games=[]`), before any start request and after successful child cleanup.
Started games still require the official completed-record proof. Regression
tests cover 404/409 duplicate leaves, refused queries and incomplete pages.

Both failed controllers exited with empty cgroups. Two authenticated exact-ID
queries independently confirmed each of the fourteen unstarted rooms absent.
The temporary audit account was deleted. Original failure receipts and
technical results remain unchanged and excluded; only room-lifecycle evidence
was reconciled. No gateway restart or active-game interruption was necessary.
Evidence is retained at
`/home/ace/baylee-data/v5/ops-codex-20261028/collector-pacing-recovery-20261028/`.

The replacement release binary has SHA-256
`0e59baa62b485af7648c0179e9efbd4139e74c85fe472cb85289e39b64f0fa4d`.
All 32 offline collector tests passed (one explicit live integration test was
not run), and the release build succeeded. One primary controller now owns
the 50 Flash slots with a 3,000-ms launch interval; lane9 remains drained.
This coordinates startup traffic while retaining game concurrency. Its
launch spacing is not a hard limit on subsequent redemption timestamps:
gateway or process delays can still bunch them. Further 429s require reviewing
actual redemption timing rather than increasing the number of controllers.
Its
monitor was rebound to the new PID/start-ticks/boot identity with attempt
floor 636, preserving delivery history. The GPU trainer, bounded Terra pilot,
private gateway and user gateway retained their process identities.

The first new admission, attempt 644, subsequently passed connection,
completed-record, provenance, replay and view-boundary checks. Both Flash
seats supplied 66 model answers each, with zero house fallback. All eleven
receipt-bound artifact hashes were independently rechecked. At that observation,
50 Flash games and the separate Terra pilot were playing, with no new failed
receipt or transport blocker.

During recovery, continuous training finished round 8 (346 LLM games,
8,046 updates) and entered round 9 with 383 LLM games. Relative to round 6,
round 8's final LLM validation choice NLL improved from 1.2666 to 1.1446,
but active-choice agreement fell from 47.35% to 45.11%; teacher agreement
also fell, from 59.61% to 54.49%. These mixed metrics do not establish better
play. No checkpoint was promoted. The next gameplay review should compare
round 6, the actual deployed model and round 8 on the same frozen cases;
round 8's pre-live-mix checkpoint can diagnose the final training stage.

## Terra/Flash pilot completion and explicit provider quota

The eight-cell pilot ended with seven admitted games and one failed attempt.
The seventh admitted game completed normally; the wrapper then asserted on
the eighth receipt's `error` status. Flash's native error explicitly reported
`Individual quota reached` with about 91 hours until reset. Its final seat had
zero model answers, 15 house fallbacks and no GameOver transcript. Terra's seat
and the official engine record reported a win, but that eighth game remains
excluded from clean model comparison and from the prepared BC dataset.

An independent audit passed 328 checks, including the 77 admitted artifact
hashes, actual model/provider/effort and seat identities, decks, 20 starting
life and zero free mulligans. The seven clean games were fallback-free:
Flash won five and Terra two. The final missing cell and independent server
seeds prevent describing this as a complete balanced or seed-paired strength
test. No semantic move-quality filter was applied.

All seven valid games were independently frozen and replayed into Encoder8
BC: 11,616 frames, zero rejected games and zero unencoded decisions, with
strict Python dataset validation passing. The earlier cell00 conversion smoke
is overlapping evidence, not an eighth unique training game. This dataset is
ready for the versioned Encoder8 training branch; the active Encoder7 run was
not changed. Evidence is under the pilot's `review-20261028/` directory.

Matching quota errors also appeared in the main Flash collector. Its DRAIN
marker now prevents new assignments while active games close normally. A fresh
one-game Flash probe is scheduled for 1 November 2026 at 15:34 Europe/Berlin,
conservatively after the reported reset. It has pinned inputs, one attempt and
no automatic full-collector restart. The existing watcher will report its
outcome. GPU training and the CPU gameplay comparison continue independently.

The isolated collector source now checks native individual-quota evidence
after every attempt, including failures lacking GameOver, rather than only
after admission. Detection sets controller-wide assignment drain and stores
`quota-hold.json`; active attempts are not cancelled and data admission is
unchanged. Startup also checks retained history before connecting to the DB.
Raw error strings are not copied into the hold. Valid positive reset durations
up to seven days use transcript mtime as a conservative fixed upper-bound
anchor; missing or ambiguous reset times require review. Transient generic
rate limits and cooldown messages do not trigger this persistent hold.

An existing hold stays blocking until reviewed and explicitly cleared; a
still-unexpired or unparseable historical signal can recreate it. In that
case, use a separately reviewed fresh namespace after provider recovery,
without deleting or altering the original evidence. Four focused quota tests
and the full collector test target passed (36 offline tests; one explicit live
test skipped). This source change has not been deployed to the draining live
process. Local landing gates remain required before pushing.

## 28 October 2026: owned-engine cleanup and private-gateway recovery

The isolated `baylee-ai-payment` checkout was rebased onto main
`94316b78a`. An independent read-only audit found 31 old unresolved primary
rooms: all official records were incomplete, with no terminal End. Audited
maintenance stopped only the dedicated private gateway and closed those old
lifecycles; evidence is in
`/home/ace/baylee-data/v5/ops-codex-20261028/maintenance-20261028T2145/recovered.json`.
All 31 attempts remain excluded, and their original admission receipts and
technical results were preserved. Lifecycle closure does not invent a draw,
a winner or a complete replay.

The agent now cancels its control writer and heartbeat on disconnect, retains
each child assignment until confirmed reaping, and handles SIGTERM/Ctrl-C by
stopping and reaping only its own engines. There is no engine reattachment
protocol: losing an agent control owner ends its owned engine assignments,
rather than leaving them attached to an obsolete connection identity. The
gateway handles exits before EngineHello, a 30-second engine-start deadline,
and GameSetup send failures; agent-control silence is bounded at 90 seconds.
An attached engine's exit status does not overtake its ordered final record
chunks. These transport changes leave the genuine 24-hour player reconnect
pause unchanged.

Collector cleanup can request an administrative close only for the exact
owned game granted to that attempt. A close acknowledgement alone is not
proof: cleanup also requires a complete authenticated exact-ID lobby absence
witness. Waiting-room cancellation and started-game aborts remain separate
from official terminal outcomes. An aborted or technically failed attempt is
never promoted to training admission by successful cleanup.

Verification passed 21 agent unit tests and one actual agent/SIGTERM subprocess
regression, four gateway lifecycle E2E tests plus the record-chunk byte/DB-drain
enhancement, 11 moderation E2E tests and one player-resume E2E test. The collector
passed 43 offline tests, including seven admin-cleanup tests; one explicit live
test remained ignored. Agent and gateway all-target Clippy passed, as did
`cargo fmt --all --check`. Collector Clippy also passed with warnings denied.
These are focused checks, not a claim that the full landing gates passed.

Release binaries were frozen under
`/home/ace/baylee-data/v5/ops-codex-20261028/lifecycle-fix-20261028/bin`.
The private gateway, primary collector and scheduled November 1 one-game probe
now reference them. A local smoke test started one actual engine, closed the
exact game through the admin route, verified process exit and authenticated
lobby absence, and deleted its test account. No model was called and no test
record was admitted. The first smoke attempt used an invalid display name;
the corrected fixture passed. Both attempts stopped their private gateway.
`private-smoke.json` and `deployment.json` in the same directory retain proof.
The collector remains drained during the Flash quota hold; the next probe
remains bounded to one game. Training and the user gateway kept their original
process identities. The user gateway's running binary was not replaced.

## 29 October 2026: native source integration and cleanup review

The collector source and pinned Terra provider/model evidence were integrated
in the isolated checkout. Cleanup now observes child exit without reaping the
group leader, preserving ownership until every live group member has stopped.
Signaling refuses a reaped child. A subprocess regression covers a leader that
exits before its remaining descendant, and verifies the descendant is dead
before cleanup succeeds.

Authenticated gateway requests disable proxies and redirects. Collection
requires a literal loopback HTTP endpoint. An explicitly supplied
`BAYLEE_COLLECTOR_ADMIN_TOKEN` can bind standalone serve and collect commands
through their environments. Serve preserves its generated memory-only token
fallback for the existing supervised identity-checked child-environment bridge.
Neither path adds a secret to arguments, receipts or logs.

The combined `llm-collector,onnx` collector target passed 47 offline tests.
One live private-gateway test stayed ignored; a second ignored fixture is
invoked explicitly by the passing owned-descendant subprocess test. Eight LLM
converter provenance tests passed, including requested Terra identity spoofing,
record/evidence/action tampering and frozen storage aliases. Targeted Clippy
for the collector, converter, legacy arena and tests passed with warnings
denied. These source checks started no gateway, collector, model call or
training service, and establish no playing-strength result or full gate pass.
