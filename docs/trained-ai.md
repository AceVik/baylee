# The trained AI (`BayleeKI`)

**Current work (27 October 2026):** BayleeAI v5 adds an explicit relational
architecture (`model_version = 5`); archived v4 remains a separate loadable
model family. The matched BC pilot below is research, with no playing-strength
claim or automatic promotion. Historical v4 implementation through
`88bc2b07f` is on
`main`; the v5 pipeline and subsequent training corrections were integrated
into `main` on 27 October. The
[latest review and run results](history/trained-ai-v4-review-2026-10-24.md) and
[takeover and training log](history/trained-ai-v4-codex-run-2026-10-22.md)
record the repaired observation clock, contract versions, measured model size,
checks and remaining limitations. v4 uses structural card features and
persistent seat memory; the older pipeline below describes v1–v3.
The [structured encoder implementation](history/trained-ai-v4-structured-encoder-2026-10-23.md)
adds encoder 6 / model 3: complete ordered typed rule trees, neural hierarchical
card/ability representations, public identity and selected-rule references.
Its dataset and checkpoint contract is separate from encoder 5 / model 2.
The runtime can still evaluate archived encoder-5 models with their original
17-input interface, allowing comparisons in the same engine executable.
CUDA inference is measured and rollout-probability audited on the 4070 Ti;
see the log for whole-game CPU/GPU comparisons. v4 fallbacks use legal offered
answers, valid clock defaults, or the ordinary view-only house AI. A question
without a clock default must never be answered with an invented priority pass.
Band formation currently uses an explicit house fallback that declines the
optional band. The neural subset sampler represents cardinality, but not the
banding/bands-with-other membership restriction; such decisions carry house
provenance, no behavior probability and no neural policy target. This avoids
invalid multi-member bands without changing sampling version 2. It sacrifices
band tactics until those constraints are represented for neural sampling.
Experienced-human strength and Phoenix NPU deployment are not established.

**Opt-in payment context (28 October 2026):** Encoder 8 is a new observation
contract for **BayleeAI v5**, requiring relational revision 2; it is not a new
model generation or a demonstrated strength improvement. Encoder 7 remains
the default and byte-compatible for existing data, checkpoints and ongoing
training. Tensor widths and candidate sampling are unchanged, but the feature
semantics differ: entity feature 33 marks the announced spell in its visible
zone, including the owner's hand, and persists through nested mana-color
questions. Global 29 still means permission to cancel; global 36 means a spell
is being announced. Globals 37/38 carry the engine's exact numeric prompt
minimum/maximum, and 39 carries an explicit stable prompt-kind code (zero when
there is no numeric prompt). These bounds describe legal answers, not a proof
that a chosen X is payable with the available mana sources.

The separate owner-only `PlayerView.announced_cast` observation is supplied by
`Session::view_for_with_payment_context` and
`observe_view_with_payment_context`. It survives X and nested mana questions
without granting `CancelCast`. Existing default, socket and `with_context`
views remain unchanged. Encoder 8 is currently an **offline experiment**:
conversion selects `--encoder-version 8`, and native `eval4` uses the matching
payment-context views and versioned observation hashes. Live `seat4` rejects
Encoder 8 until the wire has the required payment-context capability; the
legacy `arena` path also rejects it. Existing wire `casting` must not be used
as a substitute for the persistent source.

The separate experimental native methods
`Session::view_for_with_announcement_context` and
`observe_view_with_announcement_context` additionally expose optional
`announced_x`. They distinguish an unsettled X from settled zero, retain the
fact across nested mana/miracle payment, and enforce resource-owner/authorized
controller visibility. **Encoder 9 (29 October 2026)** consumes this field with
an explicit 82-wide global tensor: the unchanged 80 Encoder-8 globals followed
by X-presence and selected X (zero when absent). Entity width remains 128.
Selected X uses the versioned raw `u32` to `f32` scalar representation, so very
large integers lose numeric precision; observation hashes include the exact
integer and presence separately. Native collection, passive/decision clocks,
conversion, packed caches, checkpoints and all loaded depth graphs validate
this contract. Encoder 9 requires model 5, relational revision 2 and candidate
context revision 1 or 2. Encoder 7, Encoder 8, default/socket views and their
hashes retain their previous semantics; sampling remains version 2.

Encoder 9 is offline-only: use explicit `--encoder-version 9` conversion and
the matching `eval4` model. Live `seat4`, legacy `arena`, shadow-BC and LLM-BC
conversion reject it until their context/provenance paths support it. A fresh
supervised experiment can use `--encoder-warmstart-to9` to copy Encoder-7/8
weights, retain the first 80 global-projection columns and zero the two new
columns. Optimizer, scheduler, sampler and recurrent state start fresh. Frozen
trunk warmstarts are rejected because the new input columns must learn.
An Encoder-7 transition also includes Encoder-8 semantics; only an Encoder-8
control isolates the added X observation contract. X changes also participate
in the observation clock, so even zero-padding an Encoder-8 projection does
not guarantee identical recurrent behavior on a full history. Existing
artifacts and behavior probabilities are never relabelled.

Deterministic Fireball fixtures establish selected-X aliasing in the old
projections and matching native runtime/converter histories after the fix.
Tiny-model learning, all-five-depth CPU ONNX parity and native loading with
the full compiled catalog passed. These are integration checks: neither a
playing-strength gain nor a fix for the observed X=0, unaffordable multi-target
announcements is established. No guard or admission rule is relaxed. See the
[dated research report](history/trained-ai-v5-rust-refactor-2026-10-29.md).

Datasets, caches, checkpoints and exports bind the explicit encoder identity.
A new v5 Encoder-8 training experiment may import Encoder-7 weights only with
`--encoder-warmstart-from7`; this is a weights-only transition, not a compatible
resume. Existing Encoder-7 artifacts must not be silently relabelled or reused
as Encoder-8 caches. Focused checks passed two gamehost tests (including actual
X/dual-land nested color, owner privacy and unchanged legacy serialization)
and one engine miracle-payment regression. The Python CPU checks passed 42
tests: 31 covering encoder contracts, export and legacy BC paths, plus 11
policy-diagnostic tests. A tiny v5 relational-revision-2 Encoder-8 checkpoint
also passed actual CPU ONNX Runtime export parity for K=0/1/2/4/8 over 15
frames (maximum absolute error 9.54e-7), with Encoder-8 identity verified in
the exported metadata. This numerical fixture does not certify gameplay or
all model shapes. Subsequent state/clock, replay, live-contract, native model
identity and converter/seat/evaluator binary tests also passed. A two-update
CPU smoke on real records passed 25-frame ONNX parity and bounded native
inference, with no refusal or fallback. Its tiny model and intentionally capped
game establish integration only; no gameplay benefit is established. See the
[dated verification report](history/llm-collector-recovery-2026-10-28.md).

Executed LLM BC records may also come from `codex:gpt-6.1-terra` at low effort.
These require a pinned `input_receipt` binding the requested models and exact
declared minds to the technical admission receipt. The converter checks this
binding independently; changing a provider/model label is insufficient. Legacy
agy records remain supported. Model answers become BC targets only after
technical replay/provenance validation; standing and fallback answers are not
external-model policy targets, and none of these records are PPO behavior data.

A collector pairing (`llm_collector4 collect --pairings-file`) may name
`house:<level>` for a seat instead of a profile: the house plays it at that
level (`baylee-seat join --mind house`), so each teacher can be measured
against the opponent the trained net is benchmarked against (`house:sharp`).
It sits in the chair of the slot's second account, `HOUSE-bench-<slot>`
(hosting where it plays seat 0, joining chair 1 otherwise): a chair ticket
seats only a language model, and a chair must be called what plays it.
The house reserves nothing and reads no key; its answers are counted as the
house's (`house_answers` in `stats.json`, never `llm`), and the house against
the house is refused. Such a game is a measurement: the LLM freeze and the
converter hold both seats to an agent-CLI model and refuse it as BC input.

New `convert4` datasets carry `idle_pass_weighting_revision = 2`. For live and
LLM behavior-cloning labels, a priority pass during visible casting, announced
casting or an outstanding payment keeps its normal policy weight. It can finish
payment and is not an ordinary idle pass, including on an opponent's turn with
an empty stack. Ordinary idle-pass discounts and source eligibility are unchanged.
Missing metadata means legacy revision 1; existing datasets are not relabelled.
Reconversion changes the sealed metadata/shard content and therefore the native
cache fingerprint. This changes supervision only, not legal cancellation or
sampling behavior; validation replays used to diagnose it remain out of training.

Raw action verification and replay of explicit `SourceKind4::Net` sidecars
preserve the executed subset members and defenders. They use
`encode_executed_action4`; canonical BC teacher targets remain separate.
Canonicalizing an executed grouped attack could previously reject a legal
choice or change its replay target while retaining its behavior probability.
Evaluator and converter builds containing this correction must be pinned
together; archived datasets retain their original producer/converter identity.

**Learning diagnostics and isolated ablations (27 October):**
`tools/trainer/diagnose_policy4.py` reads strict v4/v5 checkpoints and complete
seat histories. Its independent reports group policy errors by head, pending
question, verified source format and game namespace. Genuine categorical
choices exclude forced/padded/zero-weight targets. Subset/pair scores describe
free-entry BCE and threshold agreement; they do not certify the legality of a
decoded engine action. On CUDA, use `--bf16 --num-workers 2` when comparing
against the current optimized validation protocol; the diagnostic CLI otherwise
defaults to FP32. Existing checkpoint-selection metrics remain separate.

Supervised loss4 remains the default. `train4.py`/`train5.py --loss-version 5`
permit independent `--outcome-weighting-off`,
`--loss-normalization unroll_per_head`, `--detach-value-input` and
`--detach-aux-input` ablations. Full-unroll normalization changes relative
head/value gradient scaling; it is a learning experiment. Checkpoints retain
the complete objective contract and reject incompatible resume requests.
Loss5 records `categorical_weighting: "symmetric_v1"`: categorical targets,
including priority passes and payment settlement, receive only their explicit
label and optional outcome weights. Candidate features and padding never add
an implicit pass discount. Historical loss5 contracts without this marker
are ambiguous and cannot resume exactly; use a declared weights-only `--init`
continuation. The loss4 objective and canonical contract remain unchanged.
Loss5 requires an optimized execution path; reference execution and package
`--init-state` are explicitly unsupported. Compare variants with fixed data,
initial weights, seeds, update budgets and unchanged validation definitions.

`eval4 --raw-policy --record-out DIR --shadow-teacher sharp` (or `expert`)
can collect separate, view-only teacher labels on states visited by the neural
actor. The original executed-action record and rollout probabilities remain
separate from the teacher labels. `convert4 --runs DIR --out NEW_DIR --include-passive-obs --shadow-bc`
replays complete records, verifies source/view/menu identities and reconstructs
each teacher target. Unlabelled decisions retain their chronological position
with zero policy weight; actual outcomes remain available for value/auxiliary
targets. Completed metadata and hashed `shadow-bc/` companion files are required
before Python cache admission. These datasets are explicitly BC-only and both
PPO admission paths reject them. Streaming partial packages are unsupported.
The real-network greedy pilot passed full replay and a plain-versus-shadow
comparison of all 150,393 frames and 5,964 teacher targets. Observations, menus,
outcomes, auxiliary targets and memory clocks were unchanged. The stochastic
sidecar test failed exact floating-point equality; repeated shadow-off runs also
varied slightly. Those stochastic records remain excluded and are not certified
for PPO. The 256-game greedy corrective collection completed; 252 games passed
complete replay/admission after excluding the whole four-seat rotation of a
table with capped games. This selected collection is training data, not an
unbiased strength estimate. Its first 1500-update mixed-data pilot did not
establish a playing-strength improvement over matched old-data training.

`train5.py --bc-replay-data DIR` opts into two separate supervised data stores
with fixed half-batch trajectory lanes. The primary `--data` contributes the
same stable table half in both comparisons. Passing that same directory as
replay uses its complementary table half for the old-data control; a different
directory contributes all of its training games for the corrective arm.
Source game indices must remain available to prove table identity, seat coverage
and separation from `--validation-data`. Local game IDs stay within their own
stores, and checkpoints bind the mixture and its source identities. This is
half the physical lanes, not necessarily half the labels or loss. A bounded
comparison must fit inside both arms' fully occupied lane prefixes; exhausted
lanes otherwise produce padding. Existing single-source training is unchanged.

For full-coverage mixtures, opt-in `--bc-epoch-batches N` repeats exhausted
child schedules with deterministic cycle seeds until exactly N batches. N must
cover both sources' complete first passes. Complete trajectories retain their
chronological frames and genuine H resets; per-pass lane tail padding remains.
The final repeated pass may stop mid-trajectory at the explicit epoch boundary.
Checkpoints bind the budget, seed rule and epoch-zero source repetition/padding
report; the sampler can report later epochs separately. Both matched arms must
use the same anchor source, seed and budget. This does not equalize labels or
loss mass, and stopping early with `--max-steps` still forfeits full coverage.
Zero (the default) preserves the original exhausted-source padding behavior.

Legacy nondecision frames may store default provenance `thinking_steps=1`.
Execution derives their effective K=0 from `is_decision`; the stored value does
not advance scratchpad thinking. A final genuine decision may also carry
`is_terminal=true` and still have a valid policy target. Audits must preserve
these flags and compare effective observation semantics rather than reject
valid legacy metadata or discard terminal decisions.

For execution experiments, `--compact-thinking-rows` restricts scratchpad
thinking to CPU-planned decision rows while preserving every observation and
persistent-memory update. It is off by default and requires deferred readouts
and zero dropout. CPU forward/memory/gradient parity passes. A bounded CUDA
screen passed its one-batch numerical tolerances, but compact thinking provided
no throughput gain over 20 timed early-game updates. Broader shape/history and
repeatability checks remain required before adoption. `bench_training5.py`
restores the declared architecture, objective and optimizer groups into disposable probes;
`bench_training4.py` is not a valid benchmark for current v5 optimizer/K settings.
Use immutable checkpoint copies and an idle GPU. A short fresh-trajectory probe
does not establish performance on late-game Commander boards.
Future probes include `baseline-repeat` by default to measure repeat-computation
variation. They record effective precision flags and mark numerical coverage as
`FIRST_UNROLL_ONLY`; a failing repeat control makes comparisons inconclusive.
`--float32-precision inherit` preserves existing flags, while `highest` explicitly
disables TF32 for the whole probe. Results under a different precision policy
need separate validation before applying them to training.

Rust controllers may use `baylee_train_ops::digests::InputDigestCache`. Repeated unchanged inputs then
reuse a SHA-256 receipt after checking resolved path, device/inode, size and
nanosecond modification/change times. A changed identity forces a fresh read;
changes during that read fail. Controllers must keep their expected sealed
digests separately and call `cache.verify(expected)` before and after consumers.
This avoids repeatedly streaming the same large immutable dataset for each
job. Receipts are never loaded as trusted persistent caches, and default
`guard::run` behavior remains full hashing. It is not a filesystem lock.

### Native experiment control and PPO admission

`baylee-train-ops` owns operational tooling. Python entry points are limited to
PyTorch model, optimizer, sampler and tensor work; their `torch_io4` and
`trajectory_store4` adapters call native binaries without a Python fallback.
Build the binaries with `cargo build --release -p baylee-train-ops --bins`.
`BAYLEE_TRAIN_OPS` can pin the exact `train_ops` executable for a tensor worker.
Rebuild `trajectory_cache4` alongside it when changing the dataset contract.

`train_ops pipeline --manifest PATH` runs an explicit version-1 JSON plan:
absolute `root` and `cwd`, finite `total_timeout_seconds`, and ordered `stages`.
Each stage declares a unique `name`, literal `command` array, `timeout_seconds`,
`memory_gib`, immutable `inputs`, distinct absolute `required_outputs`, and an
optional absolute `gpu_lock`. Use a fresh UUIDv7 run directory and the shared
GPU lock for every GPU stage. There is no implicit training recipe or promotion.
Every child has an isolated process group; deadlines, interruption and memory
limits clean up owned descendants before reaping the leader. Limits use summed
proportional memory, avoiding repeated counting of shared pages. Failed attempts
remain quarantined; starting a new attempt is an explicit research decision.
Include executed scripts, configuration, provider libraries and dataset seals
in `inputs`; untracked scripts are not covered by the tracked Git patch alone.
The working directory needs a Git identity. For a detached experiment copy,
create an isolated snapshot commit and retain its original checkout receipt.
Dataset files must resolve inside their declared dataset root. An isolated
dataset made of symlinks back to another root is refused; copy and verify the
files instead. Before reserving the GPU, exercise the actual native-backed
dataset constructor in the final directory on CPU and load a complete history.
A generic successful command under the process guard does not validate the
data loader or the model worker's inputs.
The guard executes the resolved invocation path, preserving symlinks and
`argv[0]` semantics such as a virtual environment's Python executable. Its
schema-3 receipt records both `executable_invocation` and the canonical
`executable` target plus its hash. Reuse and postflight require the same
invocation, target and bytes; older receipts without that binding are retained
but cannot certify reuse. A worker preflight must import Torch through the
same guarded invocation and environment that the update will use.
The pipeline seals all external inputs before its first stage and rechecks
them and completed producer outputs before each consumer. Generated inputs
must name an earlier stage's declared outputs; forward dependencies fail.

The initial native PPO collection contract is deliberately narrower than general
gameplay. `eval4 --collection-purpose on-policy-training` requires 1v1, no teams,
Raw/FP32, K2 and temperature 0.7. `--against` chooses a house opponent;
`--opponent-model PATH --opponent-role historical` or `--opponent-role self-play`
uses independent neural actors. Each seat has its own memory and RNG. Collection
manifests bind source records, completion indices, source checkpoints and seat
roles. Complete losing games are retained; cap, fallback, refusal and unencoded
frames reject the initial collection. Ordinary evaluation cannot certify itself
as PPO data. Encoder 8 or 9 conversion requires the matching explicit
`--encoder-version`, `--include-passive-obs` and `--val-every 0` for this
training-only collection.

`train_ops admit-policy --data DIR --source-checkpoint FILE
--collection-manifest FILE --out FILE` audits full histories and creates the
learner-only receipt. Both `train_policy4.py` and `train_rl4.py` require it as
`--admission`. `verify-admission` rechecks actual content, then the PyTorch worker
replays behavior probabilities before updating. Opponent/teacher actions never
become learner PPO targets merely because they were marked neural. Packed cache
reuse independently verifies source, data and index digests, including before
each worker's first lazy mmap; older unsealed caches are rebuilt.

Full-model PPO recomputes the current policy's entire preceding observation
history at each segment boundary, detaches there and differentiates through H
within the unroll. Its initial episode-return contract is gamma=lambda=1;
segment boundaries are not terminal events. Only referenced public rule subtrees
are encoded. KL rejection restores model, optimizer and RNG together. A small
synthetic capacity probe does not certify real Commander-board memory use.
Full-PPO resume contract version 2 explicitly binds
`behavior_probability_reference='recorded_executed_action'`: replay checks the
recorded likelihoods without replacing the PPO denominator. Older or differently
marked optimizer continuations fail before restoration; their weights remain
usable through a fresh `--init` experiment. The complete-collection KL audit
uses `collection_audit=current_policy_streaming_v1`: under fixed current
weights it streams each full history once and resets H on lane reuse. It
does not reuse behavior-policy H or change the differentiable training
segments' separate current-policy burn-in. A bounded 256-frame R6 inference
probe measured 15.10 s for repeated prefixes versus 3.59 s for streaming,
with identical action logprobs. This is a probe of the audit implementation,
not a complete training-speed or full-model memory benchmark.

`monitor_ops4 --root PATH --check` observes configured round, review, quota-probe
and maintenance state. It preserves the existing event IDs and delivery ledger,
and never starts a controller. Probe completion requests technical review;
it does not itself authorize resuming collectors. The per-experiment watcher is
described below. Existing historical Python scripts retained with old artifacts
are evidence, not supported operational entry points.

Freeze the complete evaluator runtime, including its provider libraries. The
current Linux CUDA evaluator loads `libonnxruntime_providers_shared.so` and
`libonnxruntime_providers_cuda.so` beside the executable; copying only `eval4`
failed even with the original library directory in `LD_LIBRARY_PATH`. Pin the
executable and library hashes together, and run a short actual CUDA match
before a long training/evaluation chain. `eval4 --help` and Python ONNX export
parity do not exercise that native runtime. Preserve failed jobs and recover
evaluation in a new directory using the verified completed checkpoints.

The local `seat4` preview accepts frozen v4 exports and explicit v5 relational
v1/v2 exports. V5 selection binds the architecture, relational revision and
model dimensions; it cannot silently switch between architecture revisions.
Inference stays on CPU while training may use the GPU. `--raw-policy` opts
into the unmodified neural policy; the default retains tactical overrides.
Waiting, pinned-model and aggregate reports record this policy mode. Selecting
a newer checkpoint makes no claim that it plays more strongly.

`cargo build --release -p baylee-train --features live-seat --bin seat4`
builds an optional local v4/v5 opponent for the ordinary gateway protocol.
`seat4 --model /path/to/exported/model --name v4-R02 --acceptance Victory`
opens a two-player room at `http://127.0.0.1:28766`, signs in as `NET-v4-R02`,
and starts when the other socket player joins with a deck and says ready.
`--deck` accepts an exported deck instead, `--join <room>` joins an existing
two-socket room, and `--games 0` hosts another room after each completed game.
An engine agent and a gateway accepting guests must already be running.
Use a frozen exported checkpoint; this is a playtest, not model promotion.
The guest keeps a ticketed lobby socket open across games, answering heartbeats
so the gateway retains its next waiting room after the previous game socket
closes, and `--games 0` opens a fresh waiting room if an idle room expires
before play starts. Losing either live connection stops the adapter; it does not reconnect
over missing game history. Local aggregate reports are retained on failure too.

This adapter uses `baylee-seat`'s lobby, ticketed socket and `TableMemory`
transport helpers, with its own `live4` core instead of the LLM `SeatCore` or
`WakeFilter`. Every distinct authorized view advances recurrent state at K=0;
the fixed decision budget defaults to K=1 with greedy selection. Only the
submitted own deck and public teams initialize knowledge. Repeated statics do
not reset memory. An observation failure or an opened socket's interruption
ends the session; reconnecting with a latest snapshot cannot reconstruct missed
observations. Aggregate reports separate neural, forced and fallback answers.
Archived Encoder-5/6 policies clear socket `casting` and `targeting` before
observation hashing, memory updates and answering; the authorized wire view
remains intact. Encoder 7 retains this permitted context, matching its offline
contract. Encoders 8 and 9 remain rejected before initialization or observation:
the socket does not declare or supply their persistent announcement contract.
The current wire has no neural `SeatMind` kind, so the visible `NET-` name and
local report identify it without claiming to be an LLM.

`seat4 --raw-policy --temperature 0.7 --policy-seed 77 --decision-audits DIR`
opts into stochastic version-2 neural sampling. Temperature must be zero or
finite and above 0.35; stochastic live play requires all three explicit options.
The independent seed is mixed with the completed-game index rotated left by
17 and the physical seat number, never with the shuffle seed. Each game's
local audit pins its effective seed, policy contract, running executable SHA256,
build revision/dirty stamp, and exact model/metadata/depth-graph hashes.
The normal preview stays greedy.
Its local audit UUID also appears in the pinned-model and aggregate reports;
the hash/probability file itself names no gateway game or account.

The optional `live_decision_audit_v1` JSONL files contain hashes of the exact
normalized policy view, its separate semantic observation-clock projection,
semantic pending question and outgoing action, the distinct-observation clock,
and a SHA256 history chain over own-deck/public-team
initialization, consumed observations and prior decisions. They retain runtime
`StepProvenance4` unchanged, including conditional log-probability only where the
runtime supplies one. Forced, scalar, tactical, fallback and live draw-rule
answers do not acquire a neural behavior probability. A draw override clears
the replaced neural answer's probability. Duplicate frames/questions create no
decision entry. Raw views, decks, questions, actions, account IDs, names and
credentials are absent; on Unix the newly created files have mode 0600.
The semantic observation/history digests exclude transport sequence, running
clocks and presentation fields using the same projection as the observation
clock. The separate exact policy-view digest intentionally includes those
bytes for wire diagnosis and cannot serve as offline semantic replay identity.

The writer durably records `generated` before attempting the socket send and
`socket_sent` only after that send completes. A send completion does not prove
engine acceptance; a cancellation/failure may leave only `generated`. These are
local diagnostics, explicitly neither anonymous exports nor admitted BC/PPO
data. Complete official anonymous export, replay matching of executed actions
and observation histories, probability verification and technical admission
remain required before training. This preparation does not enable Encoder 9
on the wire or change any converter/admission guard.

`seat4 --model-selection /path/to/selected-model.json` optionally reads a local
supervisor manifest at the start of each game, before the first view. A model
stays pinned for that entire game. The manifest carries the absolute frozen
directory, label, benchmark ID, score and SHA-256 of every model file. A later
selection must improve the same benchmark, preserve encoder/catalog contracts
and load successfully; otherwise the previous valid selection remains. Local
reports include the exact model, hash, game and neural seat for diagnosis.
They are operational metadata, not anonymous training exports.

The native `publish_live_model4` binary in `baylee-train-ops` verifies completed paired evaluation jobs,
identical deals and house baselines, export parity and evaluated model hashes
before publishing that manifest atomically. A job records its inputs as it
declared them (relative to its `cwd`, or through a symlink such as macOS's
`/var`); publication finds them by canonical path, and refuses one file
recorded twice with two digests. Receipts the corpus tools write are keyed by
canonical path too. Ranking is development mean side
utility across duel and Commander suites, not a statistically established
strength promotion. Native `live_play_maintenance4` checks the registered experiment
every 30 seconds and imports completed official gateway exports every five
minutes. A changed benchmark requires new comparable evidence. It never
restarts a live game, changes a checkpoint or trains on unfinished records.

The local adapter answers actual draw offers using `critic_draw_rule_v1`:
compare the neural continuation side utility with the agreed-draw utility
(0.5 in duels; divided among surviving sides otherwise). Accept only if it is
more than 0.10 lower. This is a heuristic decision margin, not a confidence
interval or a calibrated pure win probability. Unavailable/nonfinite inference
declines. The ordinary policy logits do not determine this reply: it is marked
as a rule/House override with separate draw counters and values in the report.
Other yes/no questions remain unchanged. Frozen evaluation and training actors
do not acquire this live-only rule. A learned draw policy needs explicit
prompt/proposer features in a new encoder contract.

New `export_onnx4.py` metadata also binds `critic_semantics`: ordinary side
outcomes use `mtg_side_outcome_utility_v1`; the separate research worker uses
`failure_terminal_research_utility_v1` with its exact objective and target
contract. Malformed or conflicting research checkpoints are refused before
export. The native identity parser rejects unknown or inconsistent markers;
offline policy evaluation can load either supported meaning. The live adapter
rejects research-utility critics before initialization, observation or answering
because their targets are incompatible with `critic_draw_rule_v1`.
The preview publisher rejects research, unknown or conflicting critic metadata
before replacing an existing live selection.

Unmarked archival exports retain their legacy identity and live behavior. This
compatibility is not an inference about arbitrary weights: research exports
made before the marker existed must remain offline and be re-exported from
their original research checkpoint for future use. Do not add metadata to a
frozen experiment or erase training provenance during a later warm start.

The owner authorized human playtest records for training on 25 October 2026.
Native `import_live_records4` reads only numbered files from the official anonymized
gateway export, deduplicates uncompressed content and creates an immutable,
train-only staging corpus. Confirmed human, neural, clock and house origins
stay distinct; house smoke opponents are excluded, unknown/mixed opponent
declarations quarantined. These are not automatically good imitation labels.
Greedy neural moves have no behavior probabilities and are not PPO data.
Staging does not mean replay validation, conversion or optimization has run.

The [live information audit](history/trained-ai-v4-live-information-audit-2026-10-25.md)
records the encoder-6 omissions and successive Encoder 7 implementation claims.
Those dated milestones are history, not evidence that every proposed field is
connected end to end. The current Encoder 7 contract has 128 entity, 64 seat,
80 global and 48 candidate features. It includes public commander designations,
ordered-zone features, projected characteristics, mana/payment features,
registered-deck inventory bookkeeping and permanent grouping with
explicit multiplicity (`entity_feat[122]`). The v4 runtime dispatches archived
encoder-6 and Encoder 7 models separately; `upgrade_state_dict_to_v7` zero-pads
older input projections and initializes the candidate-context residual.

**Known Encoder 7 gaps remain:** inventory bookkeeping and aggregate counts do
not emit all unseen own-library/sideboard cards as graph entities; payable-cast
candidate coverage is incomplete. Inventory uncertainty and some payment and
public rule relationships still need audited end-to-end contracts. Existing
features do not establish complete rules understanding. The v5 pilot reuses this
same Encoder 7 ABI and therefore inherits these gaps. New semantic edge kinds
such as legal-target, legal-block or tutor-match relationships require a
separately versioned, view-only encoder change; they are not silently supplied
by the new model.

**Current objective (`LOSS_VERSION = 4`):** categorical cross-entropy and ordinary
Bernoulli BCE use symmetric class treatment and unit policy-head coefficients.
Earlier loss versions retained asymmetric behavior that their “calibrated” label
overstated. Explicit per-step/outcome weights still apply: this is weighted
imitation, not a guarantee of calibrated action probabilities or critic win
probabilities. Changing the loss version requires fresh `--init`, not resuming
an incompatible optimizer/objective. Re-evaluate source and candidate with the
same current validator before comparing their losses.

**Raw versus tactical provenance:** `PolicyMode4::Raw` uses unmodified neural
outputs and the sampling-version-2 constraints (`raw_neural_sampler2_v1`).
`Tactical`, the default, includes deterministic playtest heuristics and records
successful non-forced answers as House demonstrations
(`tactical_house_demonstrations_v1`), without neural behavior probabilities.
Raw mode records stochastic neural probabilities only at finite temperature
above 0.35 and for supported nonscalar heads; temperature zero is greedy and
has no PPO behavior probability. Forced and fallback answers remain distinct.
Evaluation/collection records must preserve the mode and executable/model
identity. Historical tactical improvements and mislabeled tactical PPO samples
do not establish raw-policy learning or valid on-policy PPO data.

### BayleeAI v5 relational pilot

`model5.Config5` / `PlayerNet5` identify the new major family with
`model_version = 5`, `model_family = "v5"` and architecture
`baylee_ai_v5_relational_v1` or `baylee_ai_v5_relational_v2`. A small identity
hook leaves archived `PlayerNet4` behavior unchanged. V5 adds two shared-weight
message-passing rounds with a
160-wide bottleneck after the existing typed-edge block and before the
transformer. Messages depend nonlinearly on source, destination, relation and
direction; gated updates receive directed log-degree counts and public grouped
multiplicity. A zero-initialized output projection gives an exact v4 warmstart.
`relational_steps=0` bypasses the new processing. This is graph relational
processing over existing public entities, not new physical coordinates or
additional engine knowledge.

New `train5.py` runs default to relational revision 2. Revision 1 remains
loadable: a missing `Config5.relational_revision` means revision 1, never an
implicit upgrade. Revision 2 adds input normalization, source-multiplicity
weighted message means accumulated in FP32, separate group and source-card
counts for each directed relation, and accumulated residual deltas. Synthetic
fanout tests exposed saturation in revision 1's unnormalized sums; they do not
establish a playing-strength gain for revision 2. The zero-output warmstart,
public-information boundary and observation clock remain unchanged.

Optional `--relational-lr`, `--relational-output-lr` and
`--relational-warmup-steps` configure only the new parameters. Shared backbone
and critic rates retain their existing semantics. Checkpoints record group
membership, rates and warmup; resume rejects a changed schedule. Leaving these
options disabled preserves the old optimizer layout.

The module preserves initialization RNG for matched experiments, supports
activation checkpointing, and leaves the once-per-view recurrent observation
contract unchanged. CPU tests cover exact transfer, permutation/masking/count
behavior, two-hop dependence, learning after the first output update and
checkpointed gradients. ONNX parity covers actual nonzero relational weights,
all exported K budgets and variable/empty/duplicate-edge cases. Additional GPU
memory and throughput costs must be measured, not inferred from the design.
`train5.py` selects v5 explicitly; checkpoint/export loaders require its
architecture identity. Explicit v4-to-v5 initialization transfers the backbone
with a fresh optimizer. Isolated value-head calibration supports explicit v5
revisions 1 and 2 as described below. The original October-27 pilot did not
support v5 PPO or optimizer/package continuation. Current strict v5 loaders
support the explicitly versioned training, export and continuation contracts;
the PPO admission and head/full-model paths are documented above. Both PPO
paths require native whole-collection admission and recorded-probability replay.
The bounded real-history head-PPO experiment establishes one technical update,
not playing strength; full-model gameplay training remains unproven.

The matched pilot lives at
`/home/ace/baylee-data/v4/experiments/v5-relational-20261027`. Its admission audit
contains 185 complete training games (639,954 frames) and 19 complete held-out
validation games (164,055 frames), with no overlapping source records. Both
arms start from the same 75.37M-parameter clean 12-layer, width-640 stage-a BC
checkpoint (`net.pt` SHA-256
`9ec9035f9689683cdb16099c3c430a4b68934bd28b2fbe469f0e3a2ab88240e0`).
They use 1,000 full-model updates, loss 4, seed 31437, K2, batch 8, unroll 32,
learning rate 1e-5 and combined held-out checkpoint selection. The control's
frozen Python revision is `df838e9952f36a4e56e4a6be9a99c281ce0a5b11`;
V5 implementation is `753535908` with separately versioned export/loader
support. Both arms completed. Active-choice validation accuracy was 59.36%
for control and 59.25% for v5 revision 1, versus 57.96% initially; this is a
small supervised feasibility result, not a playing-strength verdict. Game
evaluation stopped before any deals: the current compiled catalog has 47 new
rule tags, so these old-catalog exports cannot be evaluated by that engine.
Failed startup artifacts and original checkpoints are retained.

The [v5 takeover and training plan](history/trained-ai-v5-takeover-2026-10-27.md)
records the three-model review, revised architecture and fresh-data follow-up.
New rule tags require explicit initialization followed by training; a catalog
refresh with donor-initialized symbols is not a lossless model migration.
Raw-policy and tactical match comparisons remain separate, with pinned engine,
catalog, model, deck files, seeds and execution provider. Validation selection
is not an independent test. Engine-derived
public combat/target/stack auxiliary supervision is a future separately audited
objective. No result here establishes stronger play or human-level ability.

The eight-round Sharp experiment is complete with **no confirmed playing
improvement** on its new-seed confirmation. A completed nonpositive mean
confirmation now vetoes publication of that candidate. The publisher writes
`<selection>.confirmation-status.json`, and maintenance exposes this as
`selection_review`; an already active preview is retained and explicitly
marked for review. This is a conservative operational filter, not a test of
statistical significance. Pending confirmation still means an experimental
development preview, never a confirmed strength promotion.

The preview requires two socket chairs, no house-AI chairs, and CPU inference
(`BAYLEE_EP=cpu` or unset); it refuses CUDA to keep training independent.
`BAYLEE_ENGINE_PUMP_STEPS=1` on the engine agent limits attached host pumping;
it is not a guarantee of observation parity for arbitrary mixed house tables.
No gateway/engine hidden state is passed to the neural adapter. The native
client accepts cleartext only on numeric loopback, so another local device can
use an SSH port forward and add `http://127.0.0.1:28766` to its gateway list.
For example: `ssh -N -L 28766:127.0.0.1:28766 user@linux-host`. Gateway and client
must speak the same protocol; this does not require running the model on the
client's device.

The native `watch_run4` binary provides an optional local completion wake-up for
long experiments. It reads `status.json` and checks the controller's Linux boot
ID and process start ticks; PID reuse cannot make a dead run look live. A
30-second user timer checks the entire experiment, including comparisons.
Unchanged running state makes no model call. A declared failure wakes immediately
on the next check; unexpected controller exit needs two checks, unreadable status
three. Successful terminal phases require the explicitly listed result artifacts
(JSON files must parse). Existing per-job timeouts remain responsible for live
but stuck jobs. The watcher never stops or starts training.

Build with `cargo build --release -p baylee-train-ops --bin watch_run4`, install
the binary in `~/.local/bin/`, and install both
`scripts/systemd/baylee-training-watch@.*` units in `~/.config/systemd/user/`.
Arm a live run before enabling its timer, using the **exact** destination thread
UUID and the controller's actual terminal phase:

```sh
watch_run4 arm \
  --root "$RUN_ROOT" --thread "$THREAD_UUID" \
  --success-phase depth-pilot-and-comparisons-complete \
  --require-artifact evaluation/paired-depth12-k4-vs-k1-commander.json
systemctl --user daemon-reload
systemctl --user enable --now baylee-training-watch@depth12-20261024.timer
```

The unit instance is the directory name below `~/baylee-data/v4/experiments/`;
adjust paths for another layout. The installed binary uses `/usr/local/bin/codex
queue --thread UUID --message TEXT`, the supported
[existing-session queue](https://learn.chatgpt.com/docs/developer-commands#codex-queue).
This is a local user timer, not a cloud scheduled task. It needs the computer and
user manager running and a usable local Codex session. No linger setting changes;
after a reboot checks resume at the next user-manager start. Queue failures retry
after 60 seconds. `watchdog/state.json` records intent and the CLI queue receipt;
`watchdog/delivered.json` suppresses further checks after accepted delivery.
Acceptance by the queue is not proof that the model has processed the message.
An ambiguous crash/timeout around enqueue can repeat a wake: the prompt carries
a stable event ID for deduplication. Stop an old timer with
`systemctl --user disable --now baylee-training-watch@depth12-20261024.timer`.
Use a fresh experiment directory for a fresh run; arming is idempotent and refuses
to overwrite a different controller/thread configuration.

`deepen4.py --model SOURCE --out NEW --layers 12` grows the eight-layer v4
trunk without discarding its learned behavior. Existing layers retain their
order; inserted pre-norm residual blocks start with zero attention/FFN output
projections and initially compute the identity. Their output projections get
gradients immediately and their internal projections learn after the first
update. CPU float32 transfer requires dropout=0 and preserves the catalog,
action heads and `h_out -> h_in` memory contract. It records source hashes and
the layer mapping; source checkpoints are never overwritten. Start the new
experiment with `--init NEW --layers 12`, fresh optimizer and trajectory lanes.
This is inspired by function-preserving network growth
([Net2Net](https://arxiv.org/abs/1511.05641)); greater playing strength must be
measured, not inferred from depth. The
[depth-pilot plan](history/trained-ai-v4-depth-pilot-2026-10-24.md) also records
the subsequent learner-game/critic work and the house-AI improvement loop.

`calibrate_value4.py` strictly loads v4 or explicitly identified v5 relational
revision 1/2 checkpoints. V5 architecture/config disagreement, missing v2
revision, incompatible family/version, corrupt state or declared catalog
mismatch is refused. Old v1 configs may omit their revision only with an explicit
v1 architecture. The calibrated checkpoint preserves source model identity,
config and structural catalog, with source file hash and architecture in
provenance. Feature-cache compatibility includes both model families' source
code. CPU tests exercise nonzero learned relational weights and verify that all
policy/backbone/memory tensors and outputs remain unchanged after fitting and
reloading the value head.

This bounded extension retains genuine stochastic-neural-trajectory eligibility.
Current house-teacher packages and greedy evaluation reports are refused as
calibration inputs; they must not be relabeled to pass that gate. Greedy own
states could support future critic fitting under a separate explicit provenance
and eligibility contract. Teacher-state critic metrics do not establish
calibration on the deployed actor's visited-state distribution.

Trajectory-version-2 conversion emits separate deduplicated K0 observation rows,
including the acting seat, before its positive-K decision readout. Calibration
consumes those observation rows and does not observe the decision view again;
adding a K0 update to every decision would double-advance memory. This contract
is covered by real v5 extraction tests, rather than inferred from the model's
positive-K forward alone.

`calibrate_value4.py` fits only `seat_value_head` from complete games visited by
the frozen source actor. Its `value_calibration4/` package advances recurrent
memory through every chronological observation in float32, then caches sampled
readout features outside the repository. First/terminal states and every 64th
cursor are retained by default. Only trajectories containing genuine stochastic
neural decisions enter fitting; earlier passive observations of those trajectories
remain eligible. Each trajectory has equal loss weight, with equal weights among
its sampled states, so long losing games cannot dominate merely by length.
Training and validation game IDs must be disjoint. Epoch zero is a selection
candidate, and reports compare held-out MSE/calibration with the original head,
uniform side prior and training-derived seat-count prior. Selection does not
establish improvement on an untouched test set or in playing strength.

The new checkpoint verifies every non-value tensor bit-for-bit and carries no
stale optimizer, recurrent lanes or validation metrics. Cache identity includes
source/data/implementation hashes and device type; record content is hash-checked.
`--collection-plan plan.json` additionally requires the collecting model's hash
to match `--model`. The experiment controller must still verify the actual
collection job manifests: a plan alone is not proof of who produced the games.
Output is published atomically to a new directory. This is an isolated critic
experiment, not a policy update or automatic promotion. See the
[own-game critic pilot](history/trained-ai-v4-own-critic-2026-10-24.md).

`train_policy4.py` provides a separate, bounded action-head PPO path in
`policy_training4/`. It audits complete learner histories against recorded
stochastic probabilities in FP32 before fitting. House-only histories are
excluded from this audit; every observation of a learner history remains.
Raw conditional masks and provenance must be valid, not loader-invented defaults.
PPO admission requires `policy_provenance_version = 2`, certifying the exact
sampled-choice-to-action decoding gate. Version 1 predates that gate and is
rejected for PPO even if numerical probability parity passes; it remains
available for supervised imitation.
Terminal Monte Carlo outcomes (`gamma=lambda=1`) supply returns. The fixed critic
is a state baseline, not a judge assigning labels to individual moves.

Only candidate/pair action heads learn. Shared encoding, thinking, recurrent
memory, scalar and value heads remain bit-identical. At least 128 genuine choices
are accumulated per update by default, with one decision-count denominator across
all microbatches. Forced/no-choice answers cannot meet that minimum; zero-policy
and zero-gradient groups never advance Adam. Updates that exceed sampled behavior
KL restore both parameters and optimizer state. A final audit over all eligible
rollout decisions must pass before the changed policy is published; otherwise the
unchanged source is published with zero accepted updates. This empirical guard
does not certify behavior outside the recorded states. See the
[Sharp-intensive experiment](history/trained-ai-v4-sharp-intensive-2026-10-25.md).

The v4 Python CLI remains `tools/trainer/train4.py`; its implementation is split
into `training4/` (driver, losses, validation and execution planning), and the
network into the `model4/` package. `--board-chunk 8` batches independent spatial
encodings across eight observations while retaining sequential per-seat memory
updates and the same batch lanes/unroll. Loss statistics remain on the device;
`--reference-execution` provides the previous synchronized loss path for checks.
Ordinary AdamW step counters stay on the CPU on resume; only capturable/fused
optimizers put them on the device. Optional `--fused-optimizer` is a measured
execution choice, not a change to the model/encoder contract. Freeze these package
directories as well as the top-level scripts when snapshotting a trainer.
`--compile-regions` additionally compiles the repeated transformer/thinking blocks
without changing checkpoint keys; it is optional and currently CUDA-verified.
`--compile-readouts` extends that choice to the recurrent update and the
candidate/pair/seat-value readouts; benchmark both cold compilation and sustained
throughput before enabling it for a run. The 54M comparison rejected this extra
mode: it was slower, hit recompilation limits and amplified BF16 differences.
`--cache-thinking-kv` optionally reuses the unchanged board's cross-attention
key/value projections within one multi-step decision. The projections retain
their autograd history and are discarded after that forward call; they never
carry across observations or optimizer updates. This adds no persistent model
state and leaves the portable ONNX inference path unchanged.
`--prune-unused-policy-heads` omits only policy branches absent from the current
frame's supervised loss. It keeps value/auxiliary supervision and every memory
update, including passive observations. Suppressed logits are placeholders for
training only; ordinary inference/export computes every policy output.
For frozen-trunk head training, a batch with no differentiable supervised loss
still advances the chronological memory, but does not call AdamW or advance its
warmup. Checkpoints distinguish consumed `step_num` batches from actual
`optimizer_updates`; metadata also records batches without an update. This
prevents passive observations from causing weight decay or fictitious learning
steps, and keeps resumed optimizer schedules aligned with real updates.
`--decision-chunk N` is an opt-in execution path: it first advances memory in
strict chronological order, then groups independent readouts using each frame's
own already-computed state. It requires grouped boards and dropout=0. Grouping
respects thinking depth and candidate/pair size; large candidate sets reduce
the group size instead of truncating choices. No future observation enters an
earlier decision, and gradients still traverse the full recurrent chain.
`--gpu-prefetch 1` adds one future batch on a dedicated CUDA copy stream, using
the loader's pinned RAM. Values 2/3 provide triple/quadruple buffering. Each queued
GPU input batch is limited to 128 MiB; oversized/unpinned inputs retain the
ordinary transfer/streaming path. Events establish copy-before-use ordering,
and allocator stream tracking prevents premature reuse of retired GPU buffers.
This is separate from the existing DataLoader CPU prefetch and filesystem cache;
benchmark the complete update before enabling it.
`--saved-activation-layers N` retains the last N trunk layers' intermediate
activations instead of checkpoint-recomputing them. It requires activation
checkpointing, defaults to zero and spends VRAM to avoid backward recomputation;
benchmark worst-case entity/candidate shapes before selecting it. It does not
change model weights, the loss or the recurrent memory contract.
`--offload-checkpoint-inputs` instead stores CUDA checkpoint inputs in pinned
CPU RAM and restores them for backward recomputation. It requires
`--activation-checkpointing`, applies to trunk and v5 relational checkpoints,
and leaves the default and CPU paths unchanged. It trades host RAM and transfer
time for VRAM without changing batch/unroll lengths, RNG preservation, the
full recurrent graph, model configuration or state-dict keys; existing
checkpoints remain resumable. Compiled CUDA/BF16 numerical parity is not
guaranteed: compare gradients and optimizer updates against a repeat control
before adopting it for a run.
`--async-checkpoint` stages one immutable CPU snapshot of `last.pt`, then writes
and atomically publishes it in a background thread. CUDA staging uses pinned RAM
and a copy stream. The next optimizer mutation waits for staging, while forward/
backward may overlap it. This relies on v4's forward not mutating model buffers;
models with running-statistic updates would need an earlier barrier. At most one
snapshot is pending, write failures propagate and normal/exceptional context
exit drains the writer. The default remains synchronous until measured.
Changing optimizer batch lanes is a learning experiment, not an equivalent
execution change: saved recurrent state and sampler cursors cannot be reassigned
to a new batch size. The benchmark's `--fresh-trajectories --batch-size N` resets
both at trajectory boundaries and reports valid frames/decisions per second;
its results alone do not justify a production batch-size change.
Benchmark continuation restores the checkpoint's sampler version, seed, epoch
and cursor before reusing its saved H. The old helper hardcoded seed 42, which
assigned that H to different trajectories for checkpoints trained with another
seed. Such a benchmark was not a valid continuation. `--sampler-seed` overrides
require `--fresh-trajectories`; incompatible or missing resume metadata is
rejected. Frozen trainers already checked their actual saved seed.
For learning pilots, `--validation-batch-size 8` keeps validation lanes and
per-head normalization fixed even when the training batch size changes. The
default follows the training batch size for compatibility. Resume checks this
scoring setting; old checkpoints implicitly used their training batch size.
Full optimized validation logs progress every 500 batches without per-frame
device synchronization.
`last.pt` also records `last_validation_step` and `last_validation_metrics`;
metrics apply only to that step, which may precede its latest optimizer step.
`net.pt` is now selected by validation policy loss by default. The previous
`policy + 2*value` criterion remains explicit `--checkpoint-selection combined`
for controlled comparisons; it is not a playing-strength criterion.
`--validate-initial` includes the unchanged initial weights in snapshot selection.
Loss version 2 removes forced single-option rows from the policy denominator,
while retaining their observations and value/auxiliary targets. Exact resume
rejects a different loss or selection protocol. `--init` starts a new experiment
with transferred weights and fresh optimizer/memory. Validation additionally
reports `val_choice_nll`, `val_choice_top1_acc` and `val_choices` over supervised
single-choice questions with at least two legal candidates. Optional passes count;
forced non-pass answers do not. The historical `val_active_top1_acc` means
non-pass labels and is retained only as a separately named comparison metric.
The current converter already omits forced policy rows; the full 24 October
bootstrap audit found none. This is defensive loss validation, not an explanation
of that run's weak match results. Older/other producers can differ; re-evaluate
both checkpoints with the same validator for a versioned comparison.
Evaluate unique parameter sets: a resumed run may keep the same earlier best
checkpoint while `last.pt` contains newer weights worth a separate diagnostic.

The v4 converter streams complete games instead of retaining all encoded frames
of a game in RAM. An initial replay verifies every recorded state hash and
derives terminal training labels. Feature encoding then retains one pending
frame per seat for next-observation targets; actor inputs still come only from
that seat's current view. Each game is compressed privately, closed and published
under `ready/` only after successful replay, semantic-reference and coverage
checks. A failed game never becomes a ready training shard. `--schedule-seed`
mixes the processing order reproducibly without changing game IDs or splits;
`--package-steps` is a consumer size hint, not a per-game truncation limit.

`package_ops4 prepare` groups only complete training games and builds their
random-access caches on the CPU. `package_ops4 consume --manifest PATH` runs
only the explicitly declared, bounded training command under the shared GPU lock.
`ready.json` follows gzip integrity, cursor continuity and terminal checks;
version 2 also seals every shard and catalog asset by content hash.
The default queue holds two unconsumed packages (current and next); whole games
may exceed the requested frame count. Raw shards are hard-linked, and no running
dataset is modified. Package mixing uses available frame shares per source run,
seeded game order (`--seed`) and whole-game virtual finish times. It must not
re-sort a ready backlog by game ID: IDs follow source runs, producing blocks of
duels followed by large tables. Source balance is approximate because games are
indivisible and readiness can vary; manifests record the exact selected games.
`--validation-data` reuses a separate fixed validation set
with the same rule catalog. The consumer manifest declares its literal training
arguments; it never infers the retired curriculum recipe or chooses a bootstrap
checkpoint from validation scores. V4 `--init-state last.pt` transfers weights
and Adam state to a new package, resetting sampler lanes and recurrent memory;
v5 package optimizer continuation remains unsupported. Exact mid-package continuation remains `--resume`, which
requires matching training/validation fingerprints. This pipeline currently
distills recorded house games; fresh on-policy RL additionally needs policy
generation barriers and verified behavior probabilities, not arbitrary stale
packages. See the structured-encoder log for active jobs and measurements.

Training sampler version 2 balances whole trajectories by segment count across
persistent lanes, then shuffles their order independently within each lane.
It never reads outcome labels, drops tails or moves a live recurrent state to
another lane. Validation retains its existing fixed grouping. A measured
1,025,727-frame package previously ended with 637 single-lane updates and 500
consecutive winner-only updates; balancing reduced its 4,683 batches to 4,031,
with 99.93% lane occupancy. This is a batching correction, not a measured gain
in playing strength. Exact resume requires the same sampler version and seed;
older checkpoints can transfer weights/Adam via `--init-state` with fresh lanes.
This sampler transfer does not imply that an old loss version can resume exactly.
`deck_catalog4` also exports held-out-card presence across all deck sections,
including the sideboard and commanders, so new imports cannot silently move
held-out cards into training.

Composed copy abilities now use `PublicObject.ability_sources`: runtime ability
indices are mapped to their original printed/token source and exact quoted copy
modifier. A copied Effigy mana ability must not select a nonexistent ability of
the copied Goblin Striker or Thrun. The card catalog exports `/copy/N` roots for
quoted `CopyOnEnter` abilities. Its changed digest requires catalog refresh,
fresh export and replayed feature data; old frozen experiments stay unchanged.
Unresolved future provenance still rejects a game instead of silently discarding
the ability. Both the whole copied object and its selected action carry these
public references; unknown/face-down objects do not expose them.

The native `paired_eval4` binary in `baylee-train-ops` compares completed model evaluations case by
case against the same opponents. It verifies the frozen engine/deck hashes,
protocol, complete seed/deck/focal-seat set and repeated house-baseline outcomes.
It reports improvements, regressions, ties and unknown capped games, with
uncertainty clustered by table. `summarize_eval4` and paired reports declare the
fixed bootstrap stream as `splitmix64-rejection-v1/20261023`. Analytic estimates,
censoring bounds and independent-table bounds preserve the earlier definitions;
finite bootstrap draws differ from the archived NumPy stream. Losing deals remain included; two policies
losing a game does not establish that it was unwinnable. Fixed-start comparisons
control luck better than unrelated win rates, but actions can change later
draws. Keep final seed/deck holdouts separate from repeatedly consulted tuning
suites and complement match results with view-only tactical regression cases.
The optimized validator uses the same metric definitions with device-side
reductions and float64 accumulators. Grouped BF16 execution has normal rounding
differences; verify gradients, full validation and actual game results.

Baylee's own Magic AI: a net that knows every card by its `CardIndex` alone,
reads only what its seat may see, scores only the answers the engine offers,
and runs on a player's CPU, GPU or NPU. This is how it is built, what its
interfaces promise, and where it stands. Code: `crates/baylee-train` (Rust) and
`tools/trainer/` (Python). Data and checkpoints live outside git
(`~/baylee-data/` on the training PC).

## The pipeline

```
selfplay ──records──▶ convert ──columns──▶ train_value / train_policy ──▶ export_onnx ──▶ arena / client
(Session, house AI)   (replay + encode)     (PyTorch)                      (score tables)   (ort, Rust)
```

1. **Which cards.** Only cards that work 100 % are dealt: `Coverage::Implemented`
   *and* named in the engine's test code (`card_index("…")`, an oracle-id literal
   or an `index::` constant; `baylee_train::working`). `xtask deck-check`
   holds a deck to the same rule. The set's SHA-256 stamps every run; a deck with
   a card outside it plays only with `--working-only`, which puts the deck's own
   basics in those cards' places.
2. **Games** (`selfplay`): every chair the house's, hosted by a `Session`, so a
   record is what a gateway keeps. Stopped by an answer cap (100k) and a wall
   cap (15 s, checked after every answer past half of it); stopped and panicked
   games are findings with a replayable record (`--name --only` replays one game
   byte for byte; `inspect` names where it stopped), never data.
3. **Decisions** (`convert`): each record is replayed with the hash checked after
   every input. A record that no longer replays was played by other rules (a card
   changed) and is refused whole — that is how a dataset drops the games a change
   touched. Each answer becomes one or more steps (see Encoding), written as flat
   column files a trainer memory-maps.
4. **Training** in PyTorch on the 4070 Ti (WSL2; `torch.compile`; CUDA graphs for
   inference).
5. **Export** of the fixed-shape part of the net to ONNX, checked against torch on
   held-out decisions before it is kept.
6. **Play** through ONNX Runtime from Rust (`netplay`, feature `onnx`): the net
   takes a chair (`Session::take_over` → `view_for` → `act`), exactly as a socket
   player would.

## Encoding (versioned: `features::ENCODER_VERSION`)

A decision is what the house AI answers from (`Session::agent_view`): no clock,
no policy acts, no teammate's hand. Hidden information is never an input to the
actor; the omniscient half (other hands, library tops) is written apart, for the
training-only critic and auxiliary heads.

- **Entities**: one row per object the seat sees (hand, stack, battlefield,
  command, looked-at, revealed tops, graveyards, exile), at most 256, the objects
  the question offers kept first. 38 raw columns: zone, controller and owner
  relative to the seat, status bits, types, colours, mana value, P/T, loyalty,
  damage, counters, relations as row numbers (attached to, attacks, blocks,
  targets), what the question offers to do with it, 128 keyword bits.
- **Card ids**: `CardIndex + 1` (0 is padding), one id for an object the seat may
  not identify, then the registry tokens. The embedding is sized for all 33,694
  ledger rows from the start; a new set adds rows, never layers.
- **Globals**: turn, phase, step, who is active and asked, question kind and
  answer count, per seat life, poison, energy, zone sizes, mana pool.
- **Options** (v2): every answer a `Pending` offers is a triple `(head, a, b)` —
  a fixed answer, something done with one object, one of its abilities, a pair of
  objects, an attacker and a player, a player, a colour, a creature type, a
  number, a way to cast (`policy.rs`). No option cap. A multi-pick answer is a
  sequence of single picks plus "done", the picks so far marked on their rows;
  `policy::assemble` puts picks back into one `PlayerAction` (tested as the exact
  inverse of `policy::steps` over whole games). `Arrange` and numbers above 63
  are not answered by the net yet and are counted per kind.
- **v3** (`features3`, `convert3`, `dataset3.py`, `model3.py`, `train3.py`,
  `export_onnx3.py`, `netplay3`). v2 stays as it is for the nets trained on it,
  and the arena plays either.
  - **Piles and runs.** Objects that differ in nothing a player reads are one
    row with a count and a picked count. The grouping key is the client's own
    (`PublicObject::summary_key`) together with the zone and what the question
    offers for the object. Objects keep a row of their own when others point at
    them or they point at others (combat, attachments, targets), and when they
    are face down. Consecutive identical stack items are one row. Grouping comes
    before the entity cap, and offered rows before others. A tested case: 300
    Forests offered for mana are one row where v2 dropped some. A pile's option
    stands for its lowest unpicked member, so a label never depends on which of
    two identical objects the house took.
  - **Seats** are rows of their own, up to 8, in turn order from the deciding
    seat. Each has its side, whether it is still in, whether it is active or
    asked, life, zones, commander damage and mana pool.
  - **The deck list.** The deciding seat's own list, and per card how many it
    has not seen yet (hand and owned public cards count as seen), read from the
    view alone. No other seat's list is an input. The critic's half has every
    seat's list and what is left of it in each library.
  - **Cards by structure.** The dataset carries the card table
    (`cardwalk::features`, one row per pool card and registry token). The net
    adds the projected structure to the id embedding. It replaces 40 % of ids
    in training by one "unseen card" vector, so a card it never saw is read
    from its structure. The export folds both into one table and gives every id
    that training never met the unseen vector.
  - **The value is a distribution over the seats.** Labels: the winners share
    it (a team alike); in a draw, the seats still in do. The deciding seat's
    win chance is the sum over its side, at any table size.

## The net

An entity transformer: every object a token (card embedding + its state), the
globals one more token (plus, for the policy, the house profile asked for), a
transformer encoder over all of them (pre-norm, no positions: order means only
what the cap keeps). Heads:

- **Value**: the win chance of the deciding seat, from the globals token. It is
  the win-chance bar and the critic's baseline.
- **Policy**: scores each option triple — fixed answers and parameters off the
  globals token, verbs and abilities off the object's token, pairs by a bilinear
  form of two tokens (`PolicyNet.tables` + `PolicyNet.score`).
- Later: the privileged critic (training only), auxiliary heads (opponent's hand,
  turns to the end, their next action), a GRU memory once the log rule is decided.

The profile input is the level knob at play time (novice … expert). Imitation
tops out at the house's own level; any level above expert comes from RL and is
labelled only once measured.

## Interfaces that stay fixed while the net grows

The owner asked that size never become a problem. Size is configuration; the
interfaces are what must not move silently.

1. **Size is configuration, not code.** Width, depth, heads and feed-forward
   width are a `Config` saved with every checkpoint and read back on load; the
   ONNX export records them in `policy.onnx.json`. The Rust player reads every
   shape from the model file (the pair width from the `pair_a` output, the
   entity count from the export's metadata) and assumes no size.
2. **The export interface is versioned**: the encoder version, the table
   layout (`PolicyNet.TABLES`, in order) and the entity count travel in
   `policy.onnx.json`; a loader refuses a mismatch. A bigger net drops in with
   no Rust change; a changed encoding is a version bump.
3. **Retraining stays cheap.** Records are kept and the converter is
   deterministic, so any net can be trained again from scratch. To grow in place:
   function-preserving widening and deepening (Net2Net: new layers start as the
   identity, widened units split), or distilling the old net into the bigger one
   before RL.
4. **Size is chosen by measurement**: trunks of about 5M, 20M and 60M on the same
   data, compared by held-out loss, calibration, agreement and win rate against
   compute; the report says where the curve flattens. The teacher trained here can
   be as large as that says; what ships is a distilled student.
5. **Headroom (the owner's 33 %)**: the deployed student is chosen so that a net
   one third larger would still meet every level's decision-latency budget on the
   reference laptop's CPU path (`acenb`: Ryzen 7 PRO 7840U, 8 Zen 4 cores). The
   margin sits in the budget, so the net can grow by a third later without
   missing the target.

## Which cards it trains on: the verification ladder

The net can only learn a card the engine plays right, so the training pool is a
level of `cargo run -p xtask -- verify` (`xtask/src/verify.rs`):

| Level | Evidence |
|---|---|
| L1 | `Coverage::Implemented`: every clause of the card was read |
| L2 | and `validate` reports nothing for its file |
| L3 | and the engine's test code names it |
| L4 | and every ability of it fired in a test, no mechanic it uses is untested, and it leaves the battlefield clean |
| L5 | and removing any one of its abilities makes one of its tests fail |

L4's mechanics part is read now (`--coverage`, `xtask/src/mechanics.rs`). The
engine's rule tests run under `cargo llvm-cov` without the per-card tests.
`syn` reads the engine's `match` arms, `if let`s and checks on each DSL variant a
card can hold. A variant is tested where all of its sites ran, partly where some
did, untested where none did. Code that faces the AI rather than the rules
(`engine/decision.rs`), hashing and formatting are no evidence. The report
ranks what is missing by the house-deck and L3 cards that use it, and names the
functions whose arms never ran. A static's layer counts as structure: its
modifier decides it (`Modifier::layer`), and the modifier is the mechanic.

L4's firing part and L5 read the engine's test hooks (`docs/verification-hooks.md`,
`xtask/src/hooks.rs`). `--ability-log <dir>` is a run of the engine's tests with
`BAYLEE_ABILITY_LOG=<dir>`. A card's abilities have all fired when every entry of
the pool inventory that a door logs appears in some test's file. An entry no door
logs (ward, toxic, …) passes when the rule tests run its variant. `--mutate`
replaces each ability of every L4 card by nothing (`BAYLEE_MUTATE`), one process
per ability, and runs the tests that fired the card. The card's own tests run
first; the five pool-wide sweeps run only if those let the mutant survive. The
card is L5 when every mutant is killed. L4's leave part has no hook yet. The
report says so, and grants L4 without it.

```text
cargo run -p xtask -- verify --coverage <llvm-cov export> --ability-log <dir> --mutate
```

Numbers on main (2026-09-30), without the hooks:

- The pool: 2716 cards, 2242 at L1, 2242 at L2, 2241 at L3.
- Of 387 variants the pool uses, 304 are tested, 49 partly, 11 untested and 23
  unsited (no engine code names them, for instance `ActivationTiming::InstantSpeed`,
  which is the absence of a restriction).
- Of the L3 cards, 2227 use no untested mechanic, and 762 use only fully run ones.

With the hooks (`c42/engine-verify-hooks`, not yet on main):

- The pool: 2749 cards, 2293 at L1, 2293 at L2, 2292 at L3, 2210 at L4 and 1855 at
  L5. L5 took 16.5 minutes for 2819 mutants.
- Of the 383 mutants that survive, 328 remove a mana ability. The only test that
  fires those abilities takes its expectation from the card itself, so a mutant
  that removes the ability also removes what the test expects.

The fuzzer (`bin/fuzz`) plays decks generated from the L3 pool. Half of each
chair's answers are picked at random among what the question offers, and the
house gives the rest. It reports:

- engine panics;
- offered answers the engine refuses;
- refusals that change the game;
- records that do not replay;
- games that do not end.

`--only N` plays a finding's game again, move for move:

```text
cargo run --profile selfplay -p baylee-train --bin fuzz -- --games 10000 --out ~/baylee-data/fuzz/f002
```

Against the engine fixes of `c42/engine-karn-targets`, 10,000 games left two
kinds: mana abilities of two-basic-type duals, and blocks of a menace attacker by
one creature. The second is a constraint `ChooseBlockers` does not state yet.

## Where it stands (2026-09-30)

- **Value net v2** (8.2M parameters, d002→d003: 200k games, every 32nd decision):
  held-out Brier 0.160, log loss 0.477, accuracy 75.3 %, calibration error 0.006;
  rated under 10 % → won 4.1 %, over 90 % → won 96.2 %; calibrated in every turn
  bucket (ECE ≤ 0.011). A logistic model on life, hand, library and board scores
  Brier 0.209. It measures house AIs playing on, not people, on two decks.
- **Value net v1** overfit (20k games are 20k labels however many positions they
  are cut into); the fix was more distinct games, sparser positions and keeping
  the best checkpoint on held-out games.
- **Policy v1** (imitation of the house, d003): 90.5 % top-1 agreement on
  held-out games (a uniform pick among the offered options agrees 26 % of the
  time). In the arena (`bin/arena`, ONNX through `ort`) it wins 37.0 %
  [32.6, 41.6] of 463 games against the house: an imitation below its teacher, as
  expected before RL.
- **v3-a** (encoder v3, one 8.7M net for policy and value, d3-r003: r003's
  decisions under v3) against v2 on the same held-out games:
  - Policy agreement 92.9 % (v2 90.5 %): attackers 87.2 % (77.6 %), priority
    94.5 % (92.6 %), blockers 59.9 % (55.3 %).
  - Value Brier 0.1574 (0.1601), calibration error 0.004 (0.006). It is better
    in every turn band.
  - The v3 conversion of r003 dropped 35,707 offered objects, against v2's
    10,326,197.
  - In the arena, the same 1,000 games against the five house profiles (the
    same seeds, decks and engine): v3-a wins 40.8 % [37.7, 44.1] of decided
    games and v2's policy 36.0 % [33.0, 39.2]. v3-a is ahead against every
    profile, most against expert (39.3 % against 29.7 %).
  - About a tenth of the games (v3 104, v2 89) were stopped for both nets:
    refusal loops of the engine bugs the Karn fixes remove.
  - Per answer v3-a takes 56 ms and v2 50 ms, with 20 games at once on the
    CPU.
- **The value net's early edge** is partly the decks (a baseline on deck, seat and
  turn scores Brier 0.245 at turns 1–3, v2 0.238), but it survives mirror
  matches, where the deck says nothing (`tools/trainer/eval_value.py --no-deck`).
- **Historical v3 reinforcement learning** (`bin/league`, `train_rl.py`): the
  learner samples at temperature 0.7 against a league (the house profiles,
  policy-v1 as the frozen yardstick, itself and its two latest former
  selves). Each iteration is 2000 league games and one step on the learner's
  decisions, with a GAE advantage from its own value head.
  - Judge checkpoints paired, never by two intervals. Every arena game has a
    fixed seed, so native `paired_arena` compares two nets on the games both
    finished, with a sign test on the games only one won. At 300 games the
    intervals had shown a gain and a loss that 1000 paired games erased.
  - The old automatic loop and machine-switch shell scripts are retired.
    New operations use `train_ops pipeline --manifest PATH`; `stop_job4 --job PATH`
    signals only the identity-bound native supervisor and verifies its owned
    children exited. Copying artifacts and publishing code are explicit stages.
  - Advantage-weighted regression (AWR) sharpened the policy without a gain:
    over three steps the mean top probability rose 0.921 → 0.950 and the
    entropy fell 0.20 → 0.13 (`eval_shift.py`). Its weights are all positive,
    and 96 % of the samples are the net's own top answer. The data rated the
    off-top samples slightly better than expected (+0.09 sd).
  - A clipped policy-gradient step (PPO) moves probability between answers
    by a zero-mean advantage, against the frozen net that played, corrected
    for its sampling temperature, with an entropy bonus. Seven PPO steps later,
    rl-15 won 46.5 % of 849 paired games against the house's expert profile,
    where v3-a won 37.2 % (p < 0.001), and the climb had not flattened.
- **Engine findings** from self-play, handed to the engine agents with replayable
  records: Karn, the Great Creator's loyalty targets (a panic), a refused `apply`
  that changed the engine (records that do not replay), a question with no legal
  answer (`ChooseCards` min 2 of one), and quadratic legality checks on token
  swarms (fixed on `c42/engine-pay-scaling`).

## Open

- Multi-seat and broad-pool v3 data, once the engine fixes reach main.
  Self-play already seats 2 to 8 decks (`--seats`, `--teams`). A four-seat
  game of house decks takes about 3000 answers and 5 s, against 40 ms for a
  duel.
- RL: head-to-head against v3-a and against every house profile, not only
  expert; the value head as the critic is the noise floor of the advantage.
- The scaling measurement and the student's latency on `acenb`.
- Regenerating data once the house AI's lethal-first fix lands: the policy
  imitates the house, so it learns whatever the house does.
