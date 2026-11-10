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

**Requested-card choices (2 November 2026):** The shared v4 candidate reader
also offers the foretell special action from the engine's `foretellable` list.
It appends these candidates after existing priority actions, using the existing
cast feature and scalar slot 18 (`0.5` for foretell, `1.0` for suspend).
Tensor widths and candidates outside these new offers stay unchanged.
For `ChoicePrompt::Vote`, candidate slots 16/17/18 hold the votes for that
permanent divided by eight, the vote marker, and ballots cast divided by
eight. They read only the public `PlayerView::ballots`; other card choices
keep their previous bytes. The observation hash includes ballots only while
nonempty. These additions make the actions and information representable;
they do not establish that an existing checkpoint has learned their value.

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

**Buyback and multikicker (4 November 2026):** no width, no feature index and
no existing byte moves. `NumberPrompt::Multikicker` takes prompt-kind code 11
in global 39 (Encoder 8 only; a new prompt, so no stored row changes), and
`YesNoPrompt::Buyback` is a yes-or-no like any other, with no feature of its
own: the net can answer it and is told nothing about it beyond the card. The
view's `times_kicked` and `bought_back` are not read. In the rule tree the two
`FaceDef` fields are written only where a card prints one (`build.rs`), so a
face without them keeps the tree it had: an older export needs
`--catalog-drift` for the cards of the buyback batch, whose trees changed, and
only those are withdrawn. The DSL summary and `cardwalk` read neither field; their widths are
contracts, and a checkpoint has learned nothing about either keyword.

**A resolved spell's ending (4 November 2026):** `CastModeKind::SpellEnding` is
a new question, so no stored row changes, and no width moves. Its candidates
share slot 40 with `MutatePosition`, the other "where does it go" of a resolving
spell, as Foretell shares the alternative cost's, and scalar 46 tells the four
endings apart. `mode_kind` gives it `Counter`'s 14. The net can answer the
question and has learned nothing about it.

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
A seated house answers as the evaluator's in-process chair does since
10.11.2026: from its socket's view, the note the table sends beside each
question (the decision context's public part) and the deck it brought
(`baylee_ai::noted::answer_as_seated`, `docs/house-ai.md` §"The seated
house"). Before that it answered from the view alone and was about eight
points weaker. `eval4 --house-as-seat` plays the net against the seated
kind, so a ladder's house rung and the evaluator can be compared on the
same opponent.
It sits in the chair of the slot's second account, `HOUSE-bench-<slot>`
(hosting where it plays seat 0, joining chair 1 otherwise): a chair ticket
seats only a language model, and a chair must be called what plays it.
The house reserves nothing and reads no key; its answers are counted as the
house's (`house_answers` in `stats.json`, never `llm`), and the house against
the house is refused. Such a game is a measurement: the LLM freeze and the
converter hold both seats to an agent-CLI model and refuse it as BC input.

`collect --starting-life N` starts every player at `N` life (1–999, the room
setup's bound) instead of the format's, for long games that put most of each
deck into play: a bug hunt for the models' rules reports (`reports.jsonl`),
not ordinary teacher data. The value is frozen in each attempt's request
(`starting_life_override`, `null` without it), the table builds the room from
it, and the audit holds every seat's starting life to the requested room. Such
games want `--game-timeout-seconds` and the profiles' `game_usd`/`game_calls`
caps raised to match.

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
separate from the teacher labels. `convert4 --runs DIR --train-manifest MANIFEST --out NEW_DIR --include-passive-obs --shadow-bc`
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

A language model can watch instead (`--shadow-teacher llm:<profile>
--llm-profiles <seat settings file>`, feature `llm`; owner, 01.11.2026: the
LLM trainer). The neural seat still answers every question. The model sits
in that seat as an LLM seat would, through the seat's own frames: the
evaluator hands the frames the table routes to the taken-over chair to a
`SeatCore` whose mind is the profile's model, and asks the question again
(`Session::reask`) where no frame carried it. The watcher's copy of the static
frame names the chair `LLM-watcher`, because a seat logic leaves a chair named
anything else; the game's roster and record keep the evaluator's names. Where
the seat logic hands a question to the mind, its answer to the question the
policy answers next is the label (`teacher_source: "llm"`, its seat
`teacher_question`). Standing orders, the house's answer for a failed mind
and refused answers label nothing (`teacher_refits`). The seat's notes go
beside the labels as `shadow/<game>.seat.jsonl`, hashed in the footer, and
the mind's own log goes to `shadow/transcripts/<game>/`. `convert4` refuses an
LLM label without that transcript, or with one whose `answered` row by the
mind for its question says another action. The house's labels are recomputed
as before. Converted LLM labels carry `SourceKind4::External`, as an LLM seat's
own answers do. A model that says a limit ends the run: no further deal
starts. The fixture test plays one game with and without a watching
`ScriptedMind`. It shows the same record, behavior probabilities and
observations, and labels on 73 of 245 neural decisions.

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
the fixed decision budget defaults to K=2 with greedy selection and the
tactical layer with the 18-group bundle (§"Rule groups over the tactical
layer"): what BayleeAI is measured at, read from `baylee_train_ops::measured`,
which `ladder4`'s `measured` mode passes too. Only the
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

### Encoder 10: the side of a target candidate (November 2026)

Encoder 10 is Encoder 7 with one signed value on target candidates: candidate slot
12, which no other question writes, holds `+1` on the side of the table the effect
wants, `-1` on the other, and `0` where the question has no side to read or the
candidate has none (a card in a graveyard or in exile, "no target"). The net sees who
controls a candidate and which spell is asking; on removal and helpful effects it had
not learnt to cross the two.

The reading is `baylee_engine::target_side`, the table the house takes its signs
from, carried to the chooser as `TargetingContext::side`, so conversion, `eval4` and a
live seat read the same value from the view. It is an input, never a label. No width
moves and no graph changes shape: `CAND_FEAT_WIDTH` stays 48, an older export reads
zero, and an Encoder 10 export with that column at zero plays as Encoder 7.

`--train-candidate-column 12` (with `--encoder-version 10 --encoder-warmstart-to10`)
starts that column from zero and trains nothing else. A candidate with the slot at
zero then gets no gradient and scores as the initial network does.

Unread, so `0`: effects outside the table (regenerate, life loss, mill, "can't block",
control and mana Auras, equip), a teammate reads as across the table.

### PR #410's questions in Encoders 4 and 7–9 (1 November 2026)

PR #410 added a card-type question, the foretell special action and cast,
and nonland naming. Each lands on a slot the encoders already had, so no
input, head, width or slot count moved and no checkpoint changes shape.
Encoder 6 (the v4 encoder) and Encoders 7–9 share one candidate builder
(`features4::encode_candidates4`, which `encode_candidates7_grouped` wraps),
so they answer each question the same way:

| Engine | Encoder 6 (v4) | Encoders 7, 8, 9 |
| --- | --- | --- |
| `LegalActions.foretellable`, answered `PlayerAction::Foretell { card }` | A Priority candidate per card, after the granted actions, marked as suspend is (`feat[3]`, cast family) with `feat[18] = 0.5` where suspend has 1.0, pointing at the card's hand row; the row's `entity_feat[27]` ("offered to cast") is set | The same candidates; the v7 state sets the same bit, `encode_candidates7` counts them, permanent grouping keys them (kind 9) |
| `CastModeKind::Foretell` (the later cast) | An alternative cost: cast-mode slot 28 (and 2 in the v3 `policy::mode_kind`); no extra rule reference (the structure catalog has no foretell entry) | The same |
| `Pending::ChooseCardType` | The subtype question: `PendingKind4::ChooseSubtype` (global 34), and kind 9 in the v3 `features::question`; one candidate per offered type in the engine's order, `feat[15]` and `feat[18] = index / 16` | The same |
| `Pending::ChooseCardName.excluded_types` | No candidate for a face with an excluded type: an excluded teacher name is unencodable (`unencoded_decisions`), and the policy cannot decode one | The same |

The v3 policy leaves the card-type question to its fallback, as it does a
card name, and never foretells. Foretell is marked as suspend because it is
the same act (pay from the pool, exile from hand, cast on a later turn); the
card and the scalar tell the two apart, and a slot of its own would be one
no trained net has read. A pass with a foretell on offer keeps its full imitation weight
(`convert4::idle_pass`), and the tactical guards count a foretell offer where
they count a suspend offer. Like suspend, foretell is offered only once {2}
floats, and the tactical guard taps mana only for a sink it can see, so in
tactical mode the net foretells only with mana already floating; raw mode
has no such guard.

Older encodings are byte for byte as they were:
`encoder_contract::pins` hashes everything the four encoders write on two
scripted games against digests taken before the change, and convert4 output
for 48 house-AI games with Encoders 7, 8 and 9 was identical before and after.
`encoder_contract::mechanics` plays each question on a real table through
each encoder (candidate, teacher target, decoding, the engine taking the
decoded answer), and `netplay4::policy::questions_tests` has both sampling
modes play the favoured answer with no house fallback.

**Votes.** The engine asks each ballot of Council's Judgment (CR 701.38a)
as `Pending::ChooseCards { prompt: ChoicePrompt::Vote, min: 1, max: 1 }`,
in turn order from the caster, and every ballot cast so far is public in
`PlayerView::ballots`. Every encoder reads the question as the ChooseCards
kind, one single-choice candidate per permanent (`feat[6]`), and a vote's
candidates also carry the tally: `feat[16]` the votes for that permanent,
`feat[17] = 1` (a vote), `feat[18]` the ballots cast (both `/ 8`).
Encoders 7–9 group identical permanents into one candidate; for a vote the
group key includes `feat[16]`, so a permanent that already has a vote
stays a candidate of its own. A teacher's ballot for it is then learned as
cast, and the net can vote with the caster instead of forcing a tie that
exiles both. Every other question writes 0 there, so no older encoding
moved. `encoder_contract::mechanics` plays a second ballot over two
identical Sol Rings through each encoder.

**Current objective (`LOSS_VERSION = 4`):** categorical cross-entropy and ordinary
Bernoulli BCE use symmetric class treatment and unit policy-head coefficients.
Earlier loss versions retained asymmetric behavior that their “calibrated” label
overstated. Explicit per-step/outcome weights still apply: this is weighted
imitation, not a guarantee of calibrated action probabilities or critic win
probabilities. Changing the loss version requires fresh `--init`, not resuming
an incompatible optimizer/objective. Re-evaluate source and candidate with the
same current validator before comparing their losses.

**Which number means what** (owner, 6.11.2026): a model's playing strength is
measured with the tactical layer, as `seat4` plays by default, and its learning
progress with `--raw-policy`. A report names which one it gives, and two numbers
compare only within one mode and one build. Tactical records stay out of
training: the layer is the house AI's reasoning, and the house AI is no teacher.

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

### Batched GPU inference and FP16 graphs

By default every `eval4` worker owns its sessions and runs one step per
call: eight workers held eight copies of each depth graph (9 GB on a 12 GB
card) and kept the GPU busy with small calls while the CPU idled.
`eval4 --inference-batch N` instead shares one copy of each graph
(`netplay4::BatchedNet4`): a thread per depth graph takes every step the
games queued while its previous call ran and pads them into one call of at
most N rows (and N×512 entity plus candidate tokens), grouping similar
sizes. Padding is the masking a single step already uses; rows never mix.
Batched runs are not bitwise reproducible (kernel choice depends on the
call's shapes); reports record the limits and call counts under
`inference_batch`, and batched record runs name `inference_batch_rows`.

`export_onnx4.py` writes the FP32 graphs and, in `MODEL/fp16`, an FP16 export
whose graphs compute in FP16 behind FP32 inputs and outputs, so the Rust ABI and
the recurrent memory handed between steps stay FP32 (`--precision float32` skips
it, `--precision float16 [--out DIR]` writes only it, in its own directory). Its
metadata says `"precision": "float16"`; the model identity carries it, and
collection contracts report it, so PPO admission keeps refusing anything but FP32
rollouts.

FP16 is the default for evaluation. `eval4 --precision auto`, the default, runs
`MODEL/fp16` for an evaluation on the GPU when that export exists, for the seat
under test and for a neural opponent alike; `--precision float32` keeps the model
as given, and `float16` fails when there is no export. A collection always keeps
FP32, and so does the CPU, where FP16 graphs gain nothing, so `ladder4` and the
seats are unchanged. The report keeps `--model` as asked for and says what ran under
`precision`; `phase_a4 table` refuses to pair two precisions unless told they are
exports of one net (`--exports`).

Measured on 31 October with arm a (paired `dev1024`, 16 workers for the
batched runs; `~/baylee-data/v6/eval/v6p-inference/`):

| Path | Wins | Wall time | VRAM |
| --- | ---: | ---: | ---: |
| FP32, one step per call, 8 workers | 354 | 991 s | 9.0 GB |
| FP32, `--inference-batch 32` | 354, every deal identical | 597 s | 1.4 GB |
| FP16, `--inference-batch 32` | 352 (2 better, 4 worse) | 392 s | 0.8 GB |

FP16 against FP32: −0.2 pp [−0.7, +0.3]. On real game states
(`netplay4::batch_tests`, ignored, run with `BAYLEE_TEST_MODEL=DIR` and
`BAYLEE_TEST_MODEL_FP16=DIR`) batched rows match single steps within
2.6e-5 and the FP16 export picks the FP32 top choice in all 267 menus. For
pure net games, FP16 throughput grows with workers until about 64
(`--threads 64 --inference-batch 64`: 3.7 games per second, about 18 rows
per call, 1.3 GB).

Measured on 7 November for B-refresh with plain tactics against `sharp`, 1024
deals each, one frozen runtime (`--threads 64 --inference-batch 96`):

| Deals | FP32 | FP16 | FP16 minus FP32 | Better/worse | Wall time | VRAM |
| --- | ---: | ---: | --- | --- | --- | --- |
| dev | 500 | 500 | 0.0 pp [−0.5, +0.5] | 3 / 3 | 595 → 268 s | 2.4 → 1.3 GB |
| fresh | 503 | 504 | +0.1 pp [−0.3, +0.5] | 3 / 2 | 543 → 234 s | 2.4 → 1.3 GB |
| both | 1003 | 1004 | +0.0 pp [−0.2, +0.4] | 6 / 5 | | |

What FP16 rollouts would do to PPO, measured the same night. `precision_gap4.py`
replays a recorded rollout through the frozen FP32 learner as
`prepare_policy_targets` does and reports the gap instead of refusing it; the ratio is
exp(learner − recorded), where PPO's first update starts. B-refresh, raw, T = 0.7,
against `sharp`, 512 games each:

| | FP32 rollouts | FP16 rollouts |
| --- | --- | --- |
| Choices | 104,951 | 104,262 |
| Mean log ratio | −6e-10 | −4e-6 |
| abs log ratio, mean / p99 / p99.9 / max | 3e-7 / 3e-6 / 1e-5 / 2e-4 | 9e-4 / .010 / .032 / .48 |
| Ratio range | .9999 to 1.0002 | .89 to 1.62 |
| Above the audit's .002 | 0 | 10.9 % |
| Outside the clip of ±.2 | 0 | 2 choices |

There is no bias, and subsets carry the widest gap (p99 .043) because their log
probability sums one term per card. The value output moves little: on 167,599 net
decisions, each run with its own memory, the K = 0 value differs by .0027 on average,
.011 at p99 and .145 at most (`phase_a4 value-gap`). So FP16 rollouts look safe for
PPO's ratio, but the parity audit would refuse every one of them, and widening its
maximum would stop it catching decoding bugs. Admission stays FP32 until the audit
becomes a bound on the distribution and one PPO update has run on FP16 rollouts.

### Card catalog compatibility (`--catalog-superset`, `--catalog-drift`)

A structured export (v4, v5, relational v1/v2; `PlayerNet5` extends
`PlayerNet4`, and encoders 7, 8 and 9 differ only in entity and global
features) reads a card through two channels:

- its **DSL summary** (`entity_dsl`, `cand_dsl`), computed at run time from
  the card's current definition by the versioned Rust encoder; and
- its **rule-tree references** (`entity_rules`, `cand_rules`: `c/`, `i/`,
  `a/`, `alt/` keys of a card, `t/`, `ta/` of a token, `q/` of a reserved
  ability), looked up in the model's own `roots` and read as rows of
  `FrozenRules.table`. `export_onnx4.py` freezes that table from the
  catalog the model was trained on (`card_structure.json`, digest in
  `structure_sha256`); it is a constant of the ONNX graph.

There is no per-card id embedding in a v4/v5 graph. Node 0 is absence: the
frozen module masks it out (`ids != 0`), so a left-out reference and node 0
are the same input, and a card the encoder cannot identify (`UNKNOWN_CARD`)
produces no references at all. Rule references only feed the encoder:
answers are decoded from the run-time candidate menu, never from the
catalog.

By default a model loads only when this build compiles the same catalog,
digest for digest. The digest covers the DSL source hash, every operator,
every node and every root, so an added card, a rules fix or an edited DSL
comment refuses every older export. `card_structure4 --check-model DIR`
compares a model's catalog with this build's tree by tree, by tag *name*,
bytes and ordered children (node and tag ids are traversal positions and
move whenever anything is added), and prints what changed, what was added,
the new operators, and which cards drift would withdraw.

**Superset** (`eval4 --catalog-superset`) accepts a newer catalog when every
root the model knows is in it with an identical tree. References the model
knows read the same row of its own table as in training, bit for bit.
References to roots only this build has (cards and tokens appended since)
are left out, so such a card reads as one whose rules the encoder cannot
see: no references, and still its public features and DSL summary. A key
neither catalog has is still an error. Where it holds, this is the rule to
use.

**Drift** (`eval4 --catalog-drift`: indices stable, structures may drift)
also accepts cards and tokens whose tree changed since the export, or which
lost or gained a root. The export cannot read a drifted card's new tree: its
table has no row for it, only the old tree's, which describes rules the card
no longer has. So drift withdraws the card whole. Every reference to it is
left out like an appended card's, and its DSL summary, computed from its
current rules, is all the model learns of its rules. A drifted entry cannot
break decoding; it can only make the model know less about that card. Drift
does not detect a reused index, which reads as a drifted card; the ledger is
append-only.

`run.json` and the summary name the binding (`catalog`): the policy, both
digests, `appended_cards`, `appended_tokens`, added roots, new operators,
`changed_roots`, and under drift every withdrawn card or token with its name.
The summary also counts `rule_refs_left_out`, how much the run read without
trees. Read such an evaluation with that caveat: it measures the export on
this build's rules, knowing the drifted and appended cards only by their
summaries.

Both flags are for evaluation only: `eval4` refuses them with any other
`--collection-purpose`. `convert4` re-encodes every record with this build's
whole catalog, so a probability recorded under another binding would belong
to another observation. A run that names `catalog` therefore never
certifies a policy-gradient dataset (`certified_run`), and an on-policy
source that names one is refused (`RunBinding::load`).

`v6s-selfimit/s1` against this branch (1 November 2026): of its 39,496 roots,
39,492 are unchanged, none are gone, 176 are new (51 cards and tokens
appended), and 4 changed: Mana Drain (card 623, `5c5bda190`) and Everybody
Lives! (card 26615, `522380714`), both rules fixes. The strict superset therefore refuses s1, and drift binds it with
those two withdrawn. The train-split decks never meet either card: the
measured runs left 0 references out. PR #410 drifts many more: about 67
former stubs implemented, hand-owned cards edited (Chrome Mox, Smokestack,
Oath of Druids, Fireblast, Dream Beavers, Cabal Therapy and others), and card
names changed by the new `excluded_types` field. After #410 only drift binds
an older export, and decks that play those cards are read without their
trees.

A model meant to *play* drifted or new cards needs a new export instead.
`refresh_structure4.py` rebinds the learned operator weights by name to a
refreshed catalog (`card_structure4 --out`, then `export_onnx4.py`, then the
regressions), so every card built from known operators gets its tree's
embedding from the same weights. It refuses operators the model never
learned unless `--allow-new-operators` initializes them from donors,
untrained. This build already uses 65 such operators (among them
`Effect::DelayedManaAtNextMain`, Mana Drain's fix), and #410's
`excluded_types` field adds more. So the refresh needs a fine-tuning phase
on the new catalog: replayed feature data converted against it, so the new
operators and the drifted cards' new trees are trained. Fresh memory and
optimizer, as `refresh_structure4.py` records.

Replaying LLM games recorded on an older build is where a rules change
shows: the replay audit refuses an input, or finds a different table, in
each game whose cards now play differently (47 of the 1,008 games of
`dataset-e7-v3a10` on the build after #410). An LLM-BC conversion publishes
only when every game converts, so a second conversion of the same runs
takes the first one's `rejected-games.json` as
`convert4 --llm-bc --llm-bc-skip-unreplayable <file>`: it leaves those
games out and records them, with their reasons and the list's digest, as
`llm_bc_skipped_unreplayable` in `meta.json`. Only replay-audit refusals
may be listed, and a list naming a game the runs lack is refused, so the
selection never depends on an outcome or goes stale silently.

#### Bringing a model onto a new catalog, step by step

New cards keep arriving; this is the repeatable procedure, as run on
7 November for B-refresh (`~/baylee-data/v6/train/v6r-refresh-notes.md`).
`C` is the commit the cards landed in, `R` its runtime, `M` the model.

1. **One frozen runtime from one commit.** `git worktree add --detach
   runtime/src-C C`, build there (`cargo build --locked --release -p
   baylee-train-ops --bins`; `cargo build --locked --profile selfplay -p
   baylee-train --features onnx-cuda --bin eval4`; the same without the feature
   for `convert4 card_structure4`), copy every binary, `trajectory_cache4`
   included, and the ONNX provider libraries into `runtime/C`, write
   `SHA256SUMS`. `eval4` reads decks and engine sources from the checkout it
   was built in (`repo_root()`), so it is built in the frozen worktree, never
   in a working one. A runtime missing `trajectory_cache4`, or carrying an older
   `train_ops`, makes the trainer fall back to a checkout's `target/` build:
   on 7 November that build predated Encoder 10, and the refresh failed with
   "Unsupported encoder version" (the refusal now names the binary and the
   version).
2. **Decks outside the checkout.** `import_decks --imported-dir
   <data>/decks-<date>/imported --manifest-out <data>/decks-<date>/manifest.v4.json
   --rejected-out ... --report-out ...` with absolute paths: the manifest names
   the decks absolutely and no checkout changes. Check that `eval4` loads the
   number of decks the manifest's filters give: `accepted_deck_count`, and
   `manifest_skipped_decks` empty (builds before a6e029cfa skipped an
   unreadable deck without a word).
3. **What changed.** `card_structure4 --out <data>/structure-C.json` and
   `card_structure4 --check-model M`. Read `changed`, `new_tags` and
   `withdrawn_under_drift`. A change to nearly every card is an encoding
   change, not cards: on 7 November `FaceDef.madness` was written on every
   face and all 12,681 face roots drifted; `build.rs` writes a face field only
   where a card prints it, and `a_plain_face_keeps_its_41_children` fails
   until a new field joins that list. Count how many test decks hold a changed
   card: if few do, the test split cannot show whether the cards were learnt.
4. **Baseline.** The old export on the new runtime with `--catalog-drift`,
   raw (`--raw-policy`) and tactics (`--raw-policy --choice-guard tactical`),
   fixed deals and seeds, through `train_ops pipeline` with the GPU lock.
5. **Refresh.** Through a pipeline whose `PATH` starts with `R`:
   `refresh_structure4.py --model M --catalog <data>/structure-C.json --out
   <new dir> --allow-new-operators`, then `export_onnx4.py --precision
   float16`. `card_structure4 --check-model <export>` must hold with nothing
   changed; a smoke game, then the baseline's evaluations without drift,
   paired with `phase_a4 table --exports --root <dir> refresh-raw~base-raw
   refresh-tac~base-tac`.
6. **Data on the new catalog.** `convert4` always encodes against its own
   build's catalog. Convert the frozen LLM runs with `R/convert4
   --encoder-version <the model's> --llm-bc --include-passive-obs`; when games
   fail the replay audit, move the output aside and convert again with
   `--llm-bc-skip-unreplayable <failed>/rejected-games.json`. Make a new list
   for every build: on 5aed5359c 367 of 1,128 games failed (47 had on the build
   after #410). Own eval and self-play records of an older build do not
   replay; house choices, tactical records and v3/v5 games are never labels.
7. **Card stage** (`card_structure4 --labels`, then `card_stage4.py`): the
   rule encoder learns what each face is, costs and does, through linear
   probes. `--train new-tags` moves only operators the old model never had,
   so every other card keeps its embedding bit for bit; `--train encoder`
   moves the whole encoder with every unchanged root anchored to the old
   model's value. A card stage leaves the trunk as it was, so it cannot move
   play by itself; it is judged on the held-out probes, with play not worse,
   and its worth in play shows only through a fine-tune that starts from it.
8. **Game fine-tune** with fresh memory and optimizer: `train5.py --init
   <refreshed or card-stage dir>` on the converted data, either
   `--train-rules-only` (only the encoder learns, through the frozen trunk) or
   the whole net with `--anchor-l2` (L2 to the `--init` weights).
9. **Gate.** A step that changes play counts only when the paired lower bound
   against the baseline is above zero in raw or tactics and neither arm's upper
   bound is below zero, pooled over three seeds. Imitation metrics have
   improved while play got worse (−5.1 pp), so they never decide play.

Measured on 7 November (runtime 5aed5359c, 512 test deals against `sharp`,
B-refresh with drift as the baseline: 155 of 512 raw, 241 with tactics). After
the madness fix 27 rules had changed, and one test deck of 83 held such a card.
The refresh played every raw deal as the baseline did (+0.0 pp, tactics
−0.2 pp [−2.0, +1.6]). The card stage, `--train encoder`, three seeds, raised
the held-out probes from B-refresh's encoder (mana value .30 → .82–.91, power
.41 → .98, toughness .36 → .97, types F1 .72 → .90, effect families .16 →
.21–.23, keywords .02 → .04–.06) and left play where it was (raw +0.1 pp
[−1.6, +1.8], tactics +0.3 pp [−1.6, +2.2]): the encoder learns printed numbers
and types far better than what an effect does, and the trunk does not use
either until a fine-tune teaches it to.

The game fine-tune did not teach it to. On the 761 language-model games that
replay on this build (367 of 1,128 did not), 4,000 updates, three seeds each,
against the same baseline: the whole net from the refresh, anchored to it
(`--anchor-l2 0.1`), lost raw play (−4.4 pp [−7.6, −1.2], tactics −0.2 pp)
while its imitation of held-out language-model games improved, the third time
that pattern showed; the rule encoder alone (`--train-rules-only`, lr 1e-4)
kept play where it was (raw −1.4 pp [−4.2, +1.5], tactics +0.3 pp), and so did
the same from the card stage (raw −0.4 pp, tactics +0.2 pp). None passed the
gate. A model meant to learn new cards from games needs a signal that rewards
play, not imitation alone.

Two instruments came out of the same night. `eval4 --random-policy` seats a
player that answers uniformly among the offered answers (`policy::random_answer`,
seeded by the deal): against `sharp` on the same 512 deals it won 21 (4.1 %),
B-refresh 153 raw and 239 with tactics. `gamelog4 audit` counts, per decision
window, what the evaluated seat answered and how often a fresh `sharp` agent on
the same view would have answered the same: B-refresh passes 99.8 % of its
priority decisions in the opponent's turn and 100 % in its own combat, where
`sharp` acts in about a third, and never takes a mulligan; with tactics it
takes them (17 %) but still does not act at instant speed. A random seat wins
69 of the same 512 against B-refresh raw and 26 against B-refresh with tactics.

#### Does the net read a card or name it?

Two of a card's references name it rather than describe it: `i/<card>/<face>`,
a `CardIdentity` node whose bytes are the card's index, and the printed name, a
string leaf under `FaceDef.name` inside the rules tree `c/<card>/<face>`. Two
switches take each away, for a measurement only:

- `eval4 --leave-out-identity` leaves every `i/` reference of the seat under
  test out at bind time, as drift leaves out a withdrawn card's
  (`CatalogRead4`; the binding records `identity_left_out`).
- `export_onnx4.py --blind-card-names --out DIR` encodes the frozen rule table
  from the catalog with every face-name string emptied (same node ids and
  digest; `rule_table_ablation` in `model.onnx.json`). The table is a constant
  of the graph, so this half is an export, not an `eval4` switch. `eval4` loads
  such an export only with `--ablated-rule-table`; every other loader refuses it.

Both are refused outside an evaluation against the house. Measured on 8
November (B-refresh FP16 with drift, the 512 test deals, each arm paired with
the unchanged export on the same build, 153 raw and 239 with tactics):

| Left out | Raw | Tactics |
| --- | --- | --- |
| identity references | +2.7 pp [−0.8, +6.3] (35 better / 20 worse) | +0.2 pp [−3.3, +3.7] |
| printed names | +0.0 pp [−1.6, +1.6] (no game changed) | +0.0 pp [−1.8, +1.8] (1/1) |
| both | +2.7 pp [−0.8, +6.3] | −0.2 pp [−3.7, +3.1] |

The net plays no worse without either. On B-refresh's encoder, emptying the
names moves a card's rule row by a median 1.0 % of its length while two
different cards' rows lie 125 % apart, and the `i/` rows of two different cards
differ by only 4.7 %: the index, read through the shared byte table, nearly
collapses to one vector. Cards are told apart by their compiled rules.

### The tactician, stage 1 (`eval4 --tactician-windows`)

A head that learns from game outcomes where the tactical layer earns most of its
+12.9 pp: declaring attackers, declaring blockers and choosing one target
(`netplay4::tactician`, owner's decision of 7 November; plan and every number
in `~/baylee-data/v6/train/v6r-refresh-notes.md`). It sits on the frozen trunk of
refreshed B-refresh (`r1`). The rules may propose answers to explore; outcomes
are its only target, and no tactical, house or teacher answer is a label.

**Answers.** At such a question the seat holds the net's raw greedy answer `N`,
the tactical layer's answer `R` on the same logits (as `--choice-guard tactical`
plays it, read on copies) and the *neighbors* of `N`: the legal answers one
change away (another target; one attacker in or out; one blocker unblocked or
onto another attacker). Every answer is a mask over the question's units, its
candidates or its blocker-attacker pairs.

**Collection** (`--tactician-explore ε --tactician-out DIR`): `N` with `1 − ε`,
`R` with `ε/3` when it differs, the rest spread over the neighbors; the played
answer's probability under the whole mixture is exact. Each decision is kept
(`tactician_records_v1`: `wNN.jsonl` with the deal's utility for the seat, and
`wNN.f32` with the embeddings, window codes and looks). These runs are
evaluations; their records train the tactician head and nothing else.

**Inputs.** `export_onnx4.py --embeddings` adds `state_emb` (the thought-through
global slot), `cand_emb` and `pair_emb` (each head's last hidden layer) to every
depth graph; the old outputs are the same ops. Per unit the head reads
`[state | unit | window code (16) | look (112)]`, and a null row with no unit.

**The look** is a one-step read of the other side's answers per unit. The engine
forked only at ordinary priority when this was built (combat declarations fork
since November 10; a target
mid-cast a `Continuation`), so it is computed: the expected castable
instant-speed cards in their hand by effect family, over a fixed prior of the
train-split decks conditioned only on what the seat sees (colours shown, open
mana, hand size), never on their real list; and view-only reads per unit (who
may block an attacker and what survives, who kills whom in a block, a target's
side, body, worth and protections, lives, open mana, visible lethal). It takes
the view and the prior, nothing else; a test deals the hidden cards again and
asserts the same belief.

**Training** (`tactician4.py`): one MLP, 2d + 128 → 1,408 → 1,408 → 1 (3.97 M
parameters at d = 640, four times the plain head); the played answer's advantage
is the null score plus the played units' scores, its target the utility minus
the net's value at the decision, weighted by min(1/μ, 10); every tenth table is
held out, and train and held-out loss are reported per epoch beside the zero
predictor's. **Play** (`--tactician DIR`): of `N`, `R` and the neighbors the head
plays the best, and leaves `N` only past the margin in `tactician.json`, chosen
on held-out decisions by a one-step importance estimate.

**Gates.** Against B-refresh (raw 153, tactics 239 of the 512 test deals), three
seeds pooled: the lower bound above zero in raw or tactics, no upper bound below
zero. A rule group goes only when "tactician, group off" against "tactics, group
on" has its lower bound above −1.5 pp and its point at or above −0.5 pp, pooled
over three seeds, on the test deals and on fresh ones.

Measured on 8 November (one frozen runtime, 5aed5359c's engine; references on
it: B-refresh 153 raw, 238 with tactics). Collection: 8 × 1,024 train deals in
29 minutes, half raw, half with tactics outside the windows, ε 0.25: 68,919
window decisions, 17,280 played away from the net's answer. The tactician costs
a median 0.12 ms per decision (at most 0.21), so no candidate cap is needed.
Three heads: held-out loss tracks training loss in every seed (no overfitting),
but the null row alone fits almost as well as the whole head (held-out MSE 0.164
against 0.162; 0.211 for predicting nothing): what is learnt is mostly the state
baseline, as the net's value is about 0.2 too optimistic. The heads leave the
net's answer in 3–8 % of the questions.

| Arm (three seeds pooled) | Test, 512 deals | Fresh, 1,536 deals |
| --- | --- | --- |
| tactician, raw elsewhere, against raw | +0.8 pp [−1.2, +2.9] | |
| tactician, tactics elsewhere, against tactics | −1.6 pp [−4.9, +1.7] | |
| at attacks only, against tactics | −1.8 pp [−4.8, +1.2] | |
| at blocks only | −1.2 pp [−3.8, +1.4] | |
| at single targets only | +1.2 pp [−1.0, +3.5] | +0.3 pp [−0.9, +1.5] |
| the raw net's own target there, against tactics | +1.4 pp [−1.0, +3.7] | +0.3 pp [−0.9, +1.4] |

The tactician does not pass its gate. At single targets the tactical layer's
part passes the test for dropping it, but the raw net's own targeting passes it
as well and the heads add nothing to it (+0.0 pp [−0.7, +0.7] on the fresh
deals): the layer earns nothing there, the head is not why.

### Deliberation over the answers (thinking revision 2)

Off by default; design, cost and the measurement plan are in
`docs/proposals/baylee-ai-thinking.md`. The legacy thinking steps move no
answer (K = 0, 1, 4, 8 against K = 2 change the top answer in 1-2 % of r1's
decisions): they never see a candidate and were never supervised per step.
`Config5.thinking_revision = 2` adds a block that runs after the heads' first
scores, over the 16 best answers: each step reads its previous proposal,
compares the answers, looks at the board again and corrects the logits, and
states a confidence in its top answer. Its correction starts at zero, so a net
with the block attached plays as its source until trained.

- Train: `train5.py --thinking-revision 2 --thinking-warmstart --init SOURCE
  --train-thinking-only --think-supervision --decision-chunk N` (N > 1; the
  per-step terms read the trace of deferred readouts). `--thinking-steps` is
  then the trunk's scratchpad depth and must be the source's; the block runs
  `--deliberation-steps`. `[think] right_by_step` in the log is the share of
  decisions each step answers as the target (step 0: the first proposal).
- Export: `export_onnx4.py` as before; a depth graph `model.kK.onnx` runs K
  deliberation steps and outputs `think_confidence` beside the five outputs.
- Play: `--thinking-steps K` picks the depth as before. `seat4` and `eval4`
  `--think-adaptive DEEP,THRESHOLD` think a decision through again at DEEP
  steps when the first pass states a confidence below THRESHOLD; a step costs
  under a millisecond on the CPU, so this is a measurement of the confidence,
  not a saving.
- Measure: `thinking_probe5.py` (what each step corrects and breaks,
  confidence calibration), `bench_thinking5.py` (latency per depth graph).

### Lookahead search (`eval4 --search K,P,H`)

The first thinking BayleeAI does beyond one forward pass, for **training and
evaluation only**: it needs the real engine, so it runs where the evaluator
hosts its games in-process and never for a seat over the gateway
(`docs/proposals/lookahead-design.md` stays the design for that one).
`netplay4::search` holds the algorithm; `Session::fork_for` (gamehost) and
`Engine::fork_for` (`docs/engine-internals.md` §"Forking a simulation")
make the worlds.

**Roots.** Only the net's `Pending::Priority` in a main phase or combat whose
answer is the policy's own (not forced, unsupported, guarded against a
repeated view or a fallback) is searched. Everything else is answered as
without a search and counted by reason (`not_priority`, `phase`,
`not_policy`); a position the engine cannot fork faithfully (a suspended
operation, face-down objects, a loop signature, a static that works in a
hidden zone (`MarkedHiddenCard`), …) is answered by the policy and counted
under the engine's `ForkRefusal`, each by its own name in
`search.counts.refused`. What the seat can see now (a hand a static reveals,
a hand it may inspect, a library top a static reveals) is kept in a fork,
not dealt again. The fork exists only under the engine's `simulation` feature, which
`baylee-train/onnx` turns on (`docs/engine-internals.md` §"Forking a
simulation"), so every `eval4` build can search.

**Candidates.** K answers: the policy's own answer first, then the other
legal answers of the same decision by the raw candidate logit of the same
inference, ties in menu order (`rank_single_choice`; a candidate decoding to
an illegal or already ranked action is dropped). In tactical mode the
policy's own answer is the tactical choice; the others are still ranked by
raw logits.

**Targets as candidates (v3, `--search-targets T`).** A target is chosen at
the question after the cast, mid-announcement, where the engine does not
fork (`ForkRefusal::Continuation`), so a v2 search never compares targets:
on s1r's dev1024 games the target head alone aimed Swords to Plowshares at
the net's own creatures 29 times though another target was legal, and the
search left that alone (`gamelog4 patterns`). With T > 0 each candidate
that casts a spell or activates a non-mana ability is applied in world 0;
when the seat's next question there is a single target (the policy's
`SingleChoice` target menu) with two or more answers, the candidate becomes
one per answer, up to T: the policy's own target first, then its next by
raw logit (the same rule as the root's candidates). Each is tried at its
target in every world: the cast is applied, the seat's target question
answered with that target as the net's own choice (it decides, then
finishes with the target), and the rollout goes on as before. Targets on
the battlefield, on the stack and players are public and keep their ids in
a fork, so every world and the table offer the same ones; a world that
asks something else after the cast, or refuses the target, refuses the
candidate like any other. A candidate's prior is the root's probability
times the target's (the softmax of the raw logits at the target question).
When a candidate with a target is chosen, the answer at the root is its
cast and `SearchedAnswer4::follow` carries the target. The harness answers
the seat's next question with it if that question is the seat's target
question and offers it as a ranked answer (`follow_decision`); the answer is
then recorded as `Search`, with the target's menu index as its teacher
target, so `convert4 --search-targets` distills target choices too.
Otherwise the policy answers and the target is dropped. The policy's own
target, chosen, is followed the same way, so every searched cast's target
is a searched answer. T = 0 (the default) is v2 exactly; T > 0 names the
contract `belief_lookahead_v3` and adds `targets`, `target_candidates`,
`target_prior` and `target_follow` to the metadata. A cast with a mode, an
X or a cost choice before its target, a triggered ability's target (an
enters trigger such as Flametongue Kavu's), and menus of more than one
target (`SubsetPick`) are not expanded yet; "up to one" is a single target
whose answers include none.

**Combat declarations as roots (v4, `--search-combat N`).** The engine forks
at a declaration of attackers or blockers as at priority (the offer is
public; `docs/engine-internals.md` §"Forking a simulation"). With N > 0 such
a declaration is a root when the policy's declaration is its own: it is
compared with up to N − 1 others built from it — no attack or no block, all
in, the policy's attackers all at each defender (focus in a team game or on a
planeswalker), and each creature's part changed alone (an attacker added or taken out
at its defender; a blocker dropped or moved to another attacker it may
block) — those world 0 refuses dropped before the rounds. The subset and
pair heads give no ranking over declarations, so the policy's declaration
has prior 1 and every other 0: under `prior` selection another declaration
must beat its mean by c / (1 + n). A changed declaration is recorded as the
search's answer; `convert4` encodes its mask from the recorded declaration.
The contract is `belief_lookahead_v4`. Its worlds are dealt from the real
decks like every search's, so it is an offline teacher only (owner, 6
November: hidden cards as an offline target, never an input or in the
shipped path).

**Worlds.** For each world p of P, the table is forked for the deciding seat
with the policy seed `derive(search_seed, "world", decision_seq, p)`, where
`search_seed = derive(deal policy seed, "search")` (SplitMix64, each purpose
its own tag; never the deal's shuffle seed). The fork deals every hidden card
again from that seed (`DeckKnowledge::Known`: the evaluator chose every
deck at the table, so it asserts the lists known), rolls its own dice, and
reseeds every house agent from it. Every candidate meets the same P worlds.
In a world every other chair, a teammate the net controls included, is
handed to the house it kept; the net plays the deciding seat from its own
memory at the root (`NetPlayer4::snapshot`/`restore`), its sampling stream
reseeded per world.

**Rollouts (v2, settled leaves).** Apply the candidate, then repeat as the
evaluator does: the net observes the seat's view, then one action is taken
(the house's, or the net's for the seat; a refused net answer falls back to
the house's). H = 0 scores the candidate's own position. Otherwise a
rollout ends:

- at the game's end;
- when the seat holds priority in a main phase *after the root's phase*,
  with an empty stack, no spell being cast, no payment owed and no mana
  floating, *and its own answer there is to pass* (`settled`; the position
  is scored before the pass). Every candidate is thus scored at the same
  point of the game, the seat's first settled pass after the root's phase,
  with what it did in between: a land drop with the spells it enables, a
  pass with the land played later, an attack with its damage. Scoring at the
  seat's first settled pass instead, in the root's own phase, compared a
  land drop before combat with a pass after it, and combat's damage chose
  the pass;
- when the seat's next own turn has ended (`turn_end`);
- when H of the seat's decisions are spent (`horizon`). A decision counts
  unless it is forced or a mana ability, whether a basic land type's
  (`ActivateManaAbility`) or one a card prints (`ActivateAbility` on a mana
  ability, CR 605.1);
- when nobody can answer, an answer is refused, or 4,096 actions have passed
  (`stopped`, a guard, not a budget).

v1 counted H in single actions, every chair's, and stopped when the seat was
asked in its next turn: mana activations used it up, and leaves fell on
transient positions.

**Scores and choice.** A finished game scores the side utility of the
deciding seat as the evaluator scores it (`terminal_utility`: 1 win, 0 loss,
a draw shared among the sides); any other leaf scores the value head's
output of the seat's latest K = 0 observation (`NetPlayer4::observed_value`,
the observation the rollout just ran, so no extra inference), which under
`mtg_side_outcome_utility_v1` is the probability that the seat's side wins.
Only complete rounds (every candidate in world p) count; a candidate refused
in its world before one round completed answers with the policy
(`candidate_refused`). Every candidate meets the same worlds (deal, house
seeds and the net's sampling stream), so its scores pair with the policy's
answer's world by world. The policy's answer is played unless the selection
(`Selection4`, `--search-select`) finds another better:

- `prior` (the default, `--search-prior-c 1.0`): the PUCT exploitation
  score with equal visits, meanᵢ + c · priorᵢ / (1 + n), where priorᵢ is the
  policy's probability of the answer (softmax of the raw logits over every
  legal answer) and n the complete rounds. The policy's preference fades as
  worlds accumulate; the best score wins, ties to the policy's answer.
- `paired` (`--search-margin 0.05 --search-z 1.0`): candidate i overrides
  only if its mean paired improvement d̄ᵢ = mean over worlds of
  (scoreᵢ − score₀) exceeds max(margin, z · se), se being the standard error
  of those differences. With one world there is no standard error and only
  z = 0 can override. Of the candidates that pass, the largest d̄ wins.
- `mean`: v1's best mean, ties to the policy's answer, then candidate order.

`paired` is not the default. It guards against noise between worlds, and on
s1 there was little: with greedy rollouts and leaves inside the turn, the
re-dealt hidden cards often change nothing, so the paired differences agree
in every world and their standard error is 0, and only the margin is left
to guard. Against what remained, the value head's consistent preference for
a pass in some positions, it replaced one land drop in seven with a pass
(14 of 93), at gains of 0.07 to 0.27; `prior` asks for more evidence the
surer the policy is and replaced 3 of 79 (measured below).

Research-utility critics are refused at start-up.

**Provenance.** A searched answer is recorded with `SourceKind4::Search`, the
net's temperature and depth, and no behavior probability; the player ends
exactly as if it had answered the chosen action itself (only its coverage
counts the decision as searched), and the table is never touched by a world
(`session/tests/simulation.rs`, `netplay4/search/tests.rs`). `run.json`
names the search (`"search"`: K, P, H and their units, the selection and its
parameters, the stop rules, the per-game cap, the critic and the contract
`belief_lookahead_v2`); unsearched runs do not name it. The summary adds
`search.counts`: decisions offered, searched (by the kind of the policy's
answer), changed (by `policy kind->played kind`), skipped and refused by
reason, rounds, rollouts, rollout steps, the seat's counted decisions,
leaves by kind, the targets followed (`followed`), not offered as a ranked
answer (`follow_declined`) or left without their target question
(`follow_lost`), a change that kept the cast at another target counted as
`<kind>->retarget`, and the wall time of searched decisions (`latency_ms`
leaves them out). Each game's line in `games.jsonl` adds `search_events`:
per searched decision its candidates in order (the policy's first) and
their targets (`null` where the policy chooses), their
means, paired improvements and standard errors, priors, selection scores,
the choice, rounds and leaves, bound to the recorded input like a fallback
event (`record_input_n`). `convert4` replays searched records: a `Search` step's target is
encoded as a teacher action, it is written with `src_kind = 3` and no
log-probability, so PPO actor masks (`source_kind == 0`) never take it; and
`certified_run` refuses any run that names a search, so a searched run never
certifies a policy-gradient dataset.

**Rollouts at once (v3).** Under `--inference-batch N` every worker's net
runs on one shared batched session (`BatchedNet4`), and a search runs up to
`--search-threads` (default 16) of its K × P rollouts at once, each on a
thread, a world and a twin of the net of its own
(`NetPlayer4::twin_on_shared_net`: the same batched session, temperature,
policy mode and depth, in the root's memory), whose steps join the same
batches as every other worker's. The deciding player is not touched until
the choice. A new rollout starts as soon as one ends, so a long rollout
does not hold the others back. The rollouts are tallied in the serial order
(world by world, candidate by candidate), and whatever ends the rounds (a
refused candidate, a failed rollout, a world the table will not fork) ends
them at the same rollout as it would one by one; rollouts past it that
already ran are not counted. With a stand-in net the report, the answer, the
player and the recorded game are the serial ones byte for byte
(`parallel_rounds_are_the_serial_rounds`,
`parallel_rollouts_record_the_serial_game`). A player on sessions of its own
lends no twin, so without `--inference-batch` the rollouts run one by one
and the run's metadata says `threads: 1`. A batch can then hold up to
`--threads × (--search-threads + 1)` rows, which is what to size
`--inference-batch` to; `search.counts.parallel` counts the decisions
searched at once. Running the rollouts at once changes how fast, not what is
searched: the contract stays `belief_lookahead_v2`, and its metadata adds
`threads` and `bitwise_reproducible` (false under `--inference-batch`, in
`run.json` and the summary alike).

**Options.** `--search K,P,H` (off by default; K 1..=32, P 1..=256,
H 0..=10000), `--search-select prior|paired|mean` (default `prior`) with
`--search-prior-c` (default 1), `--search-margin` and `--search-z` (finite,
non-negative; run metadata names the selection used and the default),
`--search-max-decisions-per-game N` (0 = no cap; later decisions answer
without searching, counted as `budget`), `--search-threads N` (1..=256,
default 16; only with `--inference-batch`), and `--search-targets T`
(0..=8, default 0; targets as candidates, above). Refused with
`--legacy-v3`, a frozen or self-play opponent or any collection purpose but
`evaluation`, and `--shadow-teacher`.

**Determinism.** A search is a function of the table, the player's state at
the root, K, P, H, the selection and the deal's policy seed: budgets are
counts, worlds and house seeds are derived from the policy seed, the engine
and the house are deterministic, and the net is on the owned CPU path; time
is measured only for the report. `--inference-batch` is not bitwise
reproducible, with or without search: a row's outputs in a batch differ from
its outputs alone by under 1e-3, and which rows share a batch depends on
timing, so a greedy rollout can turn another way.

**Limits.** The worlds are dealt from the real remaining decks of the
evaluator's known lists — an upper bound on what a seat without deck
knowledge could do. What the seat can see now is kept (a hand a static
reveals, a hand it may inspect, a library top a static reveals); what it
knew earlier about a hidden card (a top it saw, a card revealed from a hand
and kept there) is lost (`docs/engine-internals.md` §"Forking a
simulation"). Only priority roots; attack and
block declarations, targets and other questions are the policy's. The rest
of the net's side plays as the house in a world. Every observation in a
rollout costs a model step: one call at a time on a net with sessions of its
own, shared batches under `--inference-batch`.

**v1 measured (1 November 2026, CPU, a loaded 24-core workstation).** The
machinery alone (`lookahead::tests::measure_the_search_machinery_without_a_model`,
`--profile ci-release`, stand-in net, one Victory mirror, 464 decisions,
128 searched): K,P,H = 1,1,0 0.32 ms per searched decision; 4,2,0 1.6 ms;
4,2,8 9.4 ms (7.8 actions per rollout); 4,2,32 29 ms; 8,4,32 82 ms. With
`v6s-selfimit/s1` (d_model 640, encoder 7, `--raw-policy`, greedy, K = 2,
`BAYLEE_EP=cpu`, 4 deals against steady on 4 threads, train-split
manifest decks): without search a decision takes 57 ms (p50) and the 4
games 78 s. `--search 4,2,0`: 14 % of 1451 offered decisions searched (refused:
316 mid-activation, 8 trigger scans; skipped: 440 not the policy's, 338
phase, 139 not priority), 7.5 worlds and 511 ms (p50 518, p95 673) per
searched decision, 129 s in all. `--search 4,2,8`: 189 searched, 7.6 worlds
of 8.0 actions each, 6.5 s (p50 7.0, p95 10.0) per searched decision, 897 s;
every leaf was a value leaf. One game in four was won in each arm; nothing
about strength can be read from that. That model's card structure predates
this branch's catalog, so the runs used a local build with the catalog
check removed (never committed); indices are unchanged between the two
revisions and no answer fell back.

What the search changes is a warning. H = 0 changed 81 % of searched
answers (H = 8: 61 %), and a recorded 2-deal run (`search_events`) shows
what: a pass swapped for tapping a land for mana and back, and of 140
searched decisions 27 land drops and 6 casts replaced by a pass or a mana
ability, on value differences of a few hundredths (median 0.037). H = 8
does not cure it: in a capped 2-deal run (51 searched) 30 answers changed,
7 of 14 land drops and 1 of 2 casts among them, median difference 0.015.
The value head's opinion of a transient state (mana floating, priority
passed) is not a judgement of the move. v1 is plumbing to measure with, not
a player: a leaf should be a settled position (the seat's next own priority
with an empty stack, or its next turn), and a change should need a margin
over the noise, before any strength is claimed. The search is converted by `convert4` end to end (4
games, 189 `Search` frames, no policy-gradient certification).

**v2 measured (1 November 2026, CPU, load average 20 to 78 from other
sessions' test suites).** Committed path, no bypass: `c21cd263c`,
`v6s-selfimit/s1` bound with `--catalog-drift` (0 references left out on
these decks), `--raw-policy`, greedy, K = 2, `BAYLEE_EP=cpu`, 16 deals
against sharp on train-split manifest decks, 4 threads per arm, guarded
jobs (`train_ops run`) compared by `paired_eval4`. Search at K,P,H = 4,2,6:

| arm | won | searched | changed | land drop → pass / mana | cast → pass / mana | s per searched (mean, p50, p95) |
| --- | --- | --- | --- | --- | --- | --- |
| no search | 3/16 | — | — | — | — | — (117 ms p50 per decision) |
| `paired` 0.05, z 1 | 4/16 | 816 of 5,272 | 76 (9.3 %) | 14 / 7 of 93 | 0 / 2 of 37 | 19.0, 14.2, 58.0 |
| `prior` c 1 | 4/16 | 856 of 5,182 | 52 (6.1 %) | 3 / 0 of 79 | 0 / 3 of 47 | 17.8, 14.4, 48.9 |
| `mean` (4 deals) | 1/4 | 313 of 1,681 | 166 (53 %) | 22 / 12 of 52 | 3 / 7 of 18 | 17.4, 14.7, 40.0 |

Both search arms beat no search on one deal (deal 12, loss to win) and lose
none: paired utility +0.0625 over 8 independent tables, a small sample from
which no strength can be read. A game with search took 11 times as long
(4,533 s against 405 s for the 16 deals). Per searched decision: 7.5
rollouts of 15 actions, 3.1 of them the seat's counted decisions; 73 % of
leaves settled, 26 % reached H, 1 % ended the game.

What changed: the prior rule leaves land drops and casts alone in all but 3
of 79 and 3 of 47, and its other changes are one land for another (13) or a
mana tap skipped (16). The paired rule still replaced 14 of 93 land drops
with a pass, at large paired gains (0.07 to 0.27) against policy priors of
0.02 to 0.08 for the pass. The value head, given the policy's continuation,
consistently prefers the pass in those positions. 40 of the paired rule's 76
overrides had a standard error of 0: with greedy rollouts and leaves within
the turn, the re-dealt hidden cards often change nothing, so two worlds show
no noise, and only the margin guards. Selection by the mean alone changes
half of all searched answers, so v1's churn came from the rule as much as
from the leaves.

So the paired test guards against world-to-world noise, not against the
critic's systematic misjudgement; the prior rule does, by asking more
evidence the surer the policy is, and is the default since (1 November
2026). A margin calibrated on the value head's
validation error, more worlds, or sampled rollouts would give the paired
test something to measure; none of the three rules is evidence of a stronger
player yet. An earlier v2 that scored a settled pass in the root's own phase
compared a land drop before combat with a pass after it (`c21cd263c`).

**v3 measured (1 November 2026, CPU, load average about 4).** `7ee2fa72e`,
`v6s-selfimit/s1` as above, 2 deals against sharp, one worker,
`--inference-batch 16`, `--search 4,2,6` (prior), at most 12 searched
decisions per game. Both arms searched the same 24 decisions, ran the same
186 rollouts of 2,704 actions and changed the same 4 answers, so batching
turned no greedy rollout here:

| `--search-threads` | calls (rows 6,493) | rows per call | s per searched (mean, p50, max) | searched total | run |
| --- | --- | --- | --- | --- | --- |
| 1 | 6,493 | 1.00 | 5.87, 3.69, 17.0 | 141 s | 314 s |
| 4 | 5,250 | 1.24 | 4.06, 2.37, 11.7 | 97 s | 271 s |

On the CPU a call's cost grows with its rows (each lane runs on one
intra-op thread), so four rollouts at once gain 1.45 times, mostly from
encoding and the engine running beside inference. Batches stay small because
each twin encodes its next row while the lane runs the others'. The gain
this is built for is on the GPU, where a call's cost grows little with its
rows (§"Batched GPU inference and FP16 graphs"); not measured yet.

**v3 measured on the GPU (2 November 2026).** `f9f2dd4bd`, `v6s-selfimit/s1`
with `--catalog-drift`, `dev1024` against sharp, 8 workers,
`--inference-batch 136`, `--search-threads 16`, `--search 4,4,6` (prior):
430 of 1024 won (42.0 %) against 372 (36.3 %) without search, +5.6 points;
deal by deal 103 better, 45 worse, 876 equal (sign test z ≈ 4.8). 67,399 of
314,178 decisions were searched (21 %) and 4,277 answers changed (6.3 % of
them; most often a mana ability for a pass and back, and one land drop for
another). A searched decision took 0.80 s (p95 1.61 s); the 1024 games
9,280 s against 626 s.

**Search distillation (`convert4 --search-targets`).** A searched run's
games become a BC-only dataset in which the answers the search played are
the policy targets: a decision answered as `Search` keeps its weight, the
net's own unsearched decisions keep `--search-unsearched-weight` (default 0)
of theirs, and the house's, forced answers and passive frames keep their
frames for memory and value but no policy weight. Every run must name a
search in its `run.json` and carry its rollout sidecar, and must say it was
played by the raw policy alone: `policy_mode` raw, `choice_guard` off, no
`tactics_rules`, not `numbers_only`. A run in tactics mode records the
tactical layer's answer as the policy's own, so its unchanged answers would
enter as targets, and tactical records stay out of training; a run that
does not say is refused too. Separate `--val-runs` are refused. The dataset
header records the weighting and each run's search under
`search_distillation` and is not PPO-eligible. Its games are played by the
search, so the value targets are the searched player's outcomes.

**Teammates' hands in a replay (`convert4::TeamHands`).** Which hands a seat
saw is a seat setting the record does not hold. `eval4` shares every team's
hands on the net's tables and says so in `run.json`
(`"team_hand_sharing": true`); `convert4` shows teammate hands in a replay
only when the run says it shared them, none when it says `false`, and
refuses a team game whose run does not say, so a team game of people,
language models or a shadow seat never gives a seat's features a hand it did
not see.

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

`train_value_head6.py` is the same idea for v6, with what `calibrate_value4.py`
refuses let in on purpose: greedy own games, duels and language model games. Its
only target is who won, so no choice of the house or anyone else is a label.
`extract` runs the trunk once over a dataset's K=0 stream, a net seat's or every
seat's, and keeps the head's inputs (`[global, own seat, other seat]`, float16) at
each position a seat answers; decision rows never reach the trunk. Read back, a
position's value equals `eval4 --value-log` to four decimals. `fit` trains a copy of
`seat_value_head`, each turn of each game weighing the same, and merges it into a
new `net.pt` with every other tensor bit-identical; `log` writes a set as value-log
lines, so `phase_a4 values` scores games that were never played through `eval4`.
Sets are `DIR=WEIGHT:HOLD:DOMAIN`: every HOLD-th game stays out, and each domain
but the first has an intercept of its own, because base rates depend on who sits
opposite and the head stays calibrated for the first domain.
`--step-weight` reads the same label a second time on the change of the logit
between one turn's first settled position and the next, wins and losses balanced per
domain, because a search compares positions a move apart. On its own that stretches
the logit to twice its calibrated width; `--learn-step-gain` gives the step a scale
of its own.

Measured on 7 November on B-refresh's frozen trunk, K = 0 head, settled positions,
first of each turn (`phase_a4 values`, baselines fitted on dev). Data: 3,072 greedy
and 1,024 sampled games against `sharp`, 1,024 sampled duels, 760 language model
games, train decks only, weights 1, .5, .5, 1, step weight 3, every game's result the
only target. Exported and played through `eval4`, the policy unchanged on every deal:

| Deals | Head | Step AUC (board) | Brier (constant) | Mean (outcome) | AUC (board) |
| --- | --- | --- | --- | --- | --- |
| fresh | old | .671 (.715) | .2219 (.2323) | .531 (.366) | .734 (.718) |
| fresh | new, mean of 5 seeds | .733 (.715) | .2010 (.2323) | .373 (.366) | .741 (.718) |
| 1070301… | old | .670 (.718) | .2177 (.2269) | .526 (.344) | .755 (.734) |
| 1070301… | new, seed 1 | .731 (.718) | .1870 (.2269) | .367 (.344) | .762 (.734) |

Step minus board on all turns is +.018 [+.008, +.028] and +.013 [+.002, +.024]. On
fresh the head did not beat the board in every turn band: 13–18 is a tie (.771 against
.772), and which band falls short moves between sets of deals. Its slope is .50, too
confident at the ends. On language model games no head was fit on (381 games, both
seats) the step is .742 against .695, and with tactics on the fresh deals .723 against
.710. With `--learn-step-gain` the slope is 1.2 and the step .738 against .712 on new
deals, every band above the board, but the mean sits .056 high: on decks it has not seen
the head reads too much into them. No search has been run with either head yet.

`fit --weights bfloat16 [--update-rounding stochastic] [--state bfloat16]` keeps no
32-bit copy between steps. At lr 3e-4 plain BF16 costs .007 of step AUC; at lr 3e-5 it
stalls (validation Brier .1986 against .1885) because updates fall below BF16's step.
Stochastic rounding brings it back within .0001 of Brier, and BF16 Adam moments cost
nothing more. That is a head on fixed features; the trunk has not been tried.

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

**Reading the games (`gamelog4`, feature `llm`).** Win rates say how often,
not why. `gamelog4 deals --summary S` lists the deals of an eval4 summary
with a house baseline where the net lost and the house, playing the same
seat of the same deal, won (`--which won|all` for the others).
`gamelog4 show --run R --game N` replays one game of the `--record-out`
directory: both boards and hands at each turn's start, the game's log
lines as the net's seat was told them, and every answer of that seat
marked `Net`, `Search`, `House` or `Forced` from the rollout sidecar, with
names and controllers (`White Knight/P2`); a passed priority lists what
else was legal (`cast?` for a card castable only while paid for, whose
mana is not checked). `gamelog4 patterns --run R --summary S` replays the
selected games on several threads and counts what each spell or ability
was aimed at, the net's own objects or itself against anything else, and
the net's own second main phases passed with a castable card or a land in
hand. A record is omniscient, so this is for studying the net, never for a
seat. Measured on `s1r` (`dev1024`, 3 November 2026): 221 deals lost where
the house won, 76 the other way round; the net aims removal at its own
permanents about as often as at the opponent's (Swords to Plowshares 39
own / 20 other, Disenchant 14 / 1, Weight of the Underworld 13 / 4, Hero's
Downfall 8 / 5) while Lightning Bolt goes almost always elsewhere (7 / 111);
a Disenchant with no other target destroyed the net's own Fellwar Stone.
The search cannot correct this: it searches priority roots only, and
targets stay the policy's.
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

## A trampler's damage

`HouseRules::divide_trample` is set and stored as `order_triggers` below, so
training and evaluation play without it and nothing the net has seen moves. At a
table a share of a trampler's damage is a `NumberPrompt::CombatDamage` like any
other: its prompt-kind code stays 7 and `lethal` is in no encoder. `convert4`
treats it as it treats every share, left to the house and no training data.
`netplay4` answers it as the other shares, and takes `lethal` from the question,
which also covers the last blocker: there it leaves the rest to go past them.

## The order of triggers

The net is not asked what order its triggers go on the stack in, and its numbers
do not move:

- **Training and evaluation play without the question.** Self-play, `eval4` and
  every other preset built by `preset_for` or `preset_for_all` keep
  `HouseRules::default()`, where `order_triggers` is off, so their records and
  hashes are what they were. Only a table sets it (`HouseRules::for_a_table`).
- **A seat of the net at a table keeps the order offered.** `netplay4` answers
  before it encodes anything (`fallback::unsupported_choice_answer`, as for a
  band), with `SourceKind4::House` and no policy probability. It is the order
  every game it trained on played.
- **Collector games have it on**, being real rooms with language models in them.
  `convert4` reads such a record: a trigger-order step counts as forced, so it
  leaves no training step and no unencoded decision, and the model's order shows
  only as the stack it made. `a_game_that_asked_the_order_of_triggers_converts`
  holds it. Rows of the question are not in `ObservableView`, so they move no
  observation hash.

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

## A profile-guided build

Only for training and evaluation binaries that run on bigace. PGO took 27 to 32 % off
the engine's own time there, and the same on two shapes the profile never saw
(`docs/perf-baseline.md` §"What a build for one CPU gains"). Those numbers leave the
net's time out. A whole `eval4` run with the net against `sharp` gained about 2 % of
its wall time and 7 % of its CPU time, because such a run keeps only two cores busy:
it is not limited by the engine. Expect a run to gain by the share of it the rules
take, and measure the binary it is for.

Set `RUSTFLAGS` for the one build and give it its own `CARGO_TARGET_DIR`, as below.
Never put it in `.cargo/config.toml` or `~/.cargo/config.toml`: gates and anything
shipped stay generic. The client and the live servers stay as they are, and the
frozen runtime of a running job is not rebuilt.

```text
C=$(git rev-parse --short HEAD); P=~/baylee-data/pgo/$C
RUSTFLAGS="-C profile-generate=$P/raw" CARGO_TARGET_DIR=$P/target-gen \
    cargo build --locked --profile selfplay -p baylee-train --features onnx --bin eval4
$P/target-gen/selfplay/eval4 <the arguments of the workload this binary will play>
llvm-profdata merge -o $P/eval4.profdata $P/raw
RUSTFLAGS="-C profile-use=$P/eval4.profdata" CARGO_TARGET_DIR=$P/target-use \
    cargo build --locked --profile selfplay -p baylee-train --features onnx --bin eval4
```

- **Train on what the binary does.** The profile comes from the workload the binary
  will play, not from another binary's. An instrumented binary is slower, so a few
  hundred deals of the real run are enough.
- **`llvm-profdata` matches rustc's LLVM** (`rustc -vV`). The one in the toolchain
  component `llvm-tools` does; unpacked from
  `static.rust-lang.org/dist/llvm-tools-<version>-x86_64-unknown-linux-gnu.tar.xz`
  it needs no change to the toolchain. A distribution's `llvm-profdata` is usually
  older.
- **A profile belongs to one source commit.** Build it again when the engine changes.
  A stale profile is not wrong, it stops paying: code it has no counts for is
  optimised blind.
- **Prove it before a binary is used, every time.** Build `engine_perf`
  (`crates/baylee-perf`) with the same `profile-use` beside a generic one, and
  require the `--verify` files of its shapes to be byte-equal, hashes
  included. That holds the engine to the generic build, but it is another binary:
  play the same few deals with the new `eval4` and the generic one as well, and
  require every game's outcome and record to be equal, which `eval4` repeats exactly
  with the same arguments.

## Rule groups over the tactical layer

`netplay4::tactics::rules` holds groups of checks on top of the tactical layer, each with its own switch
(`seat4 --tactics-rules`, `eval4 --tactics-rules`, a comma list or `all`). With no group on, the layer plays as it did. Each group
takes one class of mistake a game against a language model showed, and reads it from the seat's view and the structure of the
cards' rules, never from a card's name and never from anything the view does not show:

| Group | What it does |
| --- | --- |
| `void` | strikes an option that changes nothing: a gate no target opens (Pyroblast with nothing blue), an amount of zero (Typhoon with no Island), a land animated while tapped or already a creature, a second copy of a legend, a kill on what the seat's own kill on the stack already names |
| `side` | a target on the wrong side, as `TargetingContext::side` reads the effect |
| `mass` | a sweep (damage and X included) that takes more of the seat's board than theirs, or nothing of theirs, unless the seat is dying to the swing |
| `gift` | a lord without "you control" that lifts their swing (Goblin King into a board of Goblins with a Mountain under them) |
| `race` | keeps blockers home when the visible swing back kills within two turns and the seat cannot get there first |
| `trip` | blocks an attacker that does something when it connects (Goblin Lackey) |
| `aim` | a kill goes to what is worth killing: not far below the best target, and not a land with a creature in reach |
| `spend`, `react` | a pass turned to the proactive spell or instant the mana could pay (`guard`'s arms, on the layer) |
| `trade`, `wall`, `chump` | an even trade block, a blocker that would live, a chump when the seat would be left within one swing |
| `press`, `walker` | a creature nothing can block left at home; attackers turned on a planeswalker |
| `keep` | a hand of one land or none goes back; spare lands go to the bottom before spells |
| `land` | the land drop that pays for most of the hand this turn; a tapped land when nothing can be cast |
| `ping` | "sacrifice this: N damage" waits for a kill or for its creature's death, and is not lost with it |
| `curve` | a cheap spell does not strand the mana a dearer set would spend |
| `burn` | burn waits for something to kill or a game to close |
| `trick` | a pump goes off in the blocks when it turns a one-on-one fight |

They answer through the tactical layer, so their records are tactical records and stay out of training. `eval4 --tactics-shadow` plays as the
layer plays and counts per deal what each group alone would have changed. A deal a group does not touch is the same game with it on, so
`phase_a4 group-plan` replays only the deals a group reaches (`eval4 --only-deals`) and `phase_a4 group-table` reads the whole paired
comparison from the shadow run and those replays.

A team run replays its games: deal `d` is table `d / seats` with focal seat `d % seats`, and the focal seats of one team play one game
(the same seed, decks and seats under the net). `eval4 --one-deal-per-game` plays only the deal whose focal seat is its team's first
seat: at `--teams 1,2,…` two games a table, so `--deals` = seats × tables plays 2 × tables, and the summary says `one_deal_per_game`.
Without teams it skips nothing.

Measured on 2026-11-06 for B-refresh against `sharp`, each group alone on 1024 dev and 1024 fresh deals against plain tactics (500 and 503 of 1024
won; raw plays 364 and 378). `mass` gives +3.5 pp [+2.7, +4.4] and `react` +2.4 [+1.4, +3.4], pooled over both splits; no other group's interval
lies above zero. A group with a negative estimate entered only if readers found its class deciding at least three ladder games: `keep` (-0.3 [-0.8, +0.2])
and `burn` (-0.4 [-0.8, 0.0]) passed that and cost a little here; `gift` (-0.4 [-0.8, -0.1]) and `press` (-0.1 [-0.3, +0.1]) did not and stayed out.
The bundle is the other 18: `race,trip,trade,wall,chump,keep,land,ping,curve,aim,void,side,mass,burn,trick,walker,spend,react`.
It is what every number since is measured with, at K=2, and what `seat4` plays by default (`baylee_train_ops::measured`; `seat4
--tactics-rules ''` plays the layer with no group). A change to that default is a change of the champion's play and comes with its
paired measurement.
It wins 571 and 579 of 1024, +7.2 pp [+5.4, +9.0], and on 1024 new deals 576 against 512, +6.2 pp [+3.9, +8.6]. Numbers are for that one runtime.

## The ladder against language models

`ladder4` plays `seat4` against `baylee-seat` on a private gateway (`llm_collector4 serve --guests`). A room left alone holds the gateway's default of
40 life and a free mulligan, so `ladder4` sets 20 and none through the host's configure call. It plays the main deck of a corpus list
(never `[sideboard]`). One gateway takes 10 guest sign-ins per address in five minutes and a game takes two, so a run of many games uses a gateway per
opponent. `--stop-spent` stops starting games at a sum of money; the book's cap stays the hard limit above it and has to hold the open games'
reservations, because a seat that sits down on less hands its game to the house, and such a game counts for nothing.
Every game keeps `seat4 --journal`: this seat's views, questions, answers and the net's ranking, local and never training data. `journal4 read`,
`ledger` and `audit` read them; `audit` runs the rule groups over them. `phase_a4 ladder-pairs` compares two arms paired by deck pair and seat;
`--later-reference` and `--later-arm` add a second round on the same deck pairs. A game is clean when the house gave at most 5 % of the answers it and
the model gave between them, and the seat closed with a summary: `agy` can die in mid-game, and the result the table then records says nothing.

Measured on 2026-11-06, B-refresh with plain tactics against the 18-group bundle, 20 life, train decks, net on both sides, bootstrap over deck pairs:

| Opponent | Round 1: slots, wins plain and bundle | Round 1 difference | Both rounds: slots | Both rounds difference |
| --- | --- | --- | --- | --- |
| `deepseek-low` | 95, 26 and 30.5 | +4.7 pp [-7.4, +17.4] | 167 | +5.1 pp [-2.7, +12.7] |
| `flash-low` | 94, 16 and 15 | -1.1 pp [-10.0, +7.7] | 187 | -2.9 pp [-9.8, +3.9] |
| `haiku-max` | 64, 10 and 16 | +9.4 pp [-1.6, +20.3] | 64 | +9.4 pp [-1.6, +20.3] |
| pooled | 253, 52 and 61.5 | +3.8 pp [-2.7, +10.1] | 418 | +2.2 pp [-2.6, +6.8] |

Round 1 is the comparison fixed before play. Round 2 was added after reading it, so the last two columns are extended after a look. The bundle's +7 pp
against `sharp` does not show here: no interval excludes zero, and against Flash it leans the other way.

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
