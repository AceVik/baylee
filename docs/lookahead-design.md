# Lookahead from a seat's observations

Design for #77, read and measured on `main` at
`14d3c9fa34a27506a4a9d256e255b72f7178554a`, 19 September 2026.
This document proposes interfaces and tests; none is implemented here.

The house AI constructs a possible world from its observations, advances it
with the existing rules engine, and returns an action from the real pending
offer. Gamehost supplies no real-game `Engine`, `GameState`, checkpoint,
journal or RNG to that interface. Owning a fabricated engine is permitted;
receiving the real world's hidden state is not. An incomplete reconstruction
returns the current heuristic's answer.

## What the current code establishes

`baylee-ai/Cargo.toml` already links the engine, view, cards, cards DSL and
client core. The production policy reads the engine's choices and
`DecisionContext`; it constructs no engine. `act` and `act_with_context`
borrow observations. `act_with_scouting`, implemented in `intelligence.rs`,
clones the controller for a temporary strategy and discards that clone after
the answer. The anti-cheat boundary is the input, not the dependency edge.

`Engine::new` is the only engine constructor. There is a capability-checked
`Engine::dev_state_mut(seat)`, contrary to the brief's broader claim about
mutable access, but it cannot install a complete continuation and is not a
simulation interface. `GameState` currently has **33 public fields and two
private scratch fields**. `PlayerView` has **20 public fields** and
`VIEW_VERSION` is **21**. The field inventory below covers the current code,
rather than treating the brief's thirteen fields as a complete state.

The timing measurement used the shared cargo lock, with
`BAYLEE_SESSION=astra`, and this command:

```sh
/Users/viktor/.baylee-locks/with-cargo-lock.sh cargo bench -p baylee-engine --bench basics -- '^(setup/from_preset|state/clone|state/snapshot_hash|engine/priority_pass_x4)$' --quick
```

| Path | Estimate | Criterion interval |
| --- | ---: | ---: |
| `setup/from_preset` | 9.3862 µs | 9.3660–9.4666 µs |
| `state/clone` | 5.4615 µs | 5.4493–5.5100 µs |
| `state/snapshot_hash` | 6.3129 µs | 6.3031–6.3518 µs |
| `engine/priority_pass_x4` | 2.1564 µs | 2.1540–2.1570 µs |

These are optimized, quick measurements on the shared M1 Max, not a latency
guarantee. Clone, hash and setup use the existing 120-card setup fixture;
the pass fixture starts after mulligans. None measures a suspended engine
clone, belief construction, a long journal, spell resolution or a token swarm.
`docs/perf-baseline.md` already supersedes its historical 7.80 µs clone
figure in later sections. It should not be quoted as today's cost.

The engine is synchronous. `HouseRules` defaults to a 600-second decision
timeout and a 60-second reconnect window, but `EngineRunner::clock` returns
neither clock for an unattached AI or StandIn chair. The second clock applies
to an absent seat that answers over a socket. `Session::pump` can answer 4096
AI questions before returning. Thus a slow search can stall the deciding
thread and delay other seats' clocks; 600 seconds is not an AI watchdog.

## Q1 — Put the constructor on the engine, with a resume contract

Choose the constructor approach, refined to
`Engine::from_simulation_position(position, house_rules, lookup, simulation_seed)`.
`SimulationPosition` owns a fabricated `GameState` and an explicit
`ResumePoint`. Construction is fallible. It replaces the position's RNG with
the supplied simulation stream and creates no opening hands, shuffles or
mulligan questions. The AI builds this argument; the host never supplies it.
The type name documents provenance but cannot prove it. The enforceable
boundary remains the host-to-policy signature and its noninterference tests.

A bare `Engine::from_state(GameState, HouseRules, lookup)` is insufficient.
The engine owns consecutive passes, priority holder, combat declaration
progress, used loyalty abilities, trigger and entry scan cursors, queued
triggers and delayed actions, suspended `Resolution`, cast and activation
wizards, a mana-payment window, automation, and loop detection state. Two
engines with the same `GameState` can have different next legal transitions.
Copying `Engine::new`'s default latches around a mid-game state invents one of
those transitions.

The first `ResumePoint` is deliberately narrow: ordinary priority during a
main phase, with an empty stack and no active combat or suspended operation.
It supplies the priority holder, consecutive passes and loyalty uses from
observed history. Trigger and entry scans have consumed the reconstructed
past; queues, entry questions, miracle questions, commander redirects and
copy fixups are empty. The constructor validates structural consistency,
rebuilds derived caches and runs ordinary settlement before publishing the
offer, without replaying already-consumed events. If this changes an observed
root fact or produces an incompatible question, the adapter refuses the
sample. The adapter must establish
the history facts; an empty stack alone does not establish them. In
particular, a mana-payment window also uses `Pending::Priority` today.

Capabilities are disabled. Simulated opponents use an explicit response
policy, with automatic priority holds disabled; their real private standing
orders are neither copied nor inferred. An active loop break or an unknown
continuation refuses this first resume form. The initial form also admits
only histories whose answer count is known and still below the loop watch's
first sample. Restore that count and refuse a rollout before it would need
an unreconstructed historical loop sample. An ordinary empty stack is not
permission to reset a previously sampled loop watch. A future general resume form
must carry the complete driver continuation, including loop-watch progress;
it cannot silently extend the meaning of the narrow form.

The first search branches only at these roots. Each rollout reconstructs a
root and then advances its own engine normally, keeping every continuation
the engine creates. It does not repeatedly rebuild `GameState` in the middle
of a cast. Branching at arbitrary simulated questions would need a complete
engine checkpoint/fork; that is subsequent work, not an ability of
`GameState::clone`.

Reject a separate `Simulator` facade for this first seam. It still needs the
same resume contract and independent RNG underneath, and `apply`, `pending`
and `legal_actions` alone cannot supply observations for evaluation or a
complete fork. A wrapper does not make a real-state clone legitimate. Keeping
one driver also preserves the existing layer refresh, event processing,
state-based actions and loop policy. Reject a second rules interpreter for
the same reason: it would turn the omissions of today's combat estimate into
omissions of a second game engine.

The documentation cost is explicit:

- In `CLAUDE.md`, replace “The AI still receives no `Engine` or `GameState`
  reference” with “The house AI receives no real-game `Engine` or
  `GameState`; any simulation engine it owns is constructed from
  seat-authorized observations.” This is a clarification of provenance,
  not permission to pass a covered-up host state.
- Extend the architecture diagram's “choice taxonomy” edge to include
  construction and advancement of fabricated positions. Extend “`Engine`
  exposes essentially …” with the checked simulation constructor.
- Retain the ordinary `act(&PlayerView, &Pending)` entry point and the
  scouting paragraph's access and lifetime restrictions. Add a separate
  stateful lookahead entry point; do not give `act` a host callback.
- `docs/house-ai.md`'s “public tactical search, not whole-game
  determinization” remains the description of the existing combat search.
  Add a separately gated determinization policy. Its current requirement
  for a view-derived belief state is satisfied, not removed.

The proposed constructor is not a saved-game recovery API. Recovery of a
real game has a different provenance and requires a complete continuation.

## Q2 — Randomness belongs to the world being simulated

There are three separate inputs: the real game's seed, a policy seed, and
streams derived from that policy seed for belief sampling and simulated
future randomness. The real seed, ChaCha8 key, position and call count never
cross the policy boundary. `Session::new` and `harness::play_report` currently call
`with_seed(preset.seed)`; both must change before lookahead is connected.
XORing a public constant into that same seed is not separation.

Use a recorded policy seed independent of the game RNG, for example a
versioned derivation from the public game identifier and seat. Offline
fixtures supply an independent policy seed explicitly. Stable derivations
also include the decision number, particle number and a purpose tag
(`belief` or `future`). Sampling more particles cannot consume future-rule
randomness. Root alternatives within a particle start with the same future
stream to reduce comparison noise; advancing one alternative never advances
another. Every stream is an owned `GameRng`, and no mutable RNG is shared.
Neither system time nor scheduler timing participates.

The invariant is stronger than “the clone has a different address”:

> For fixed authorized observations, policy seed and work budget, search
> returns the same action and fabricated worlds regardless of the real
> hidden cards, RNG key or RNG position; running it leaves the real engine's
> snapshot hash, RNG seed, position, call count and next random outputs
> unchanged.

`search_does_not_advance_live_rng` should retain an untouched control game,
perform simulated shuffles and draws, compare the real game's
`snapshot_hash` before and after, then compare subsequent real RNG outputs
with the control. `equal_observations_ignore_live_rng` should change only
unobserved host data and its RNG, hold the independently supplied policy
seed fixed, and compare canonical sampled worlds, work counts and actions.
The first test catches stream consumption; the second catches inherited
seeds and hidden-state inputs. Merely comparing call counts catches neither
seed disclosure nor a cloned real stream.

Reject borrowing the live RNG, copying its stream, using the raw preset seed,
and reseeding from the wall clock. The first changes the game, the next two
make unobserved random outcomes predictable, and the last loses replay.

## Q3 — Build the world field by field

Supply a separate seat-authorized setup value for public format, house rules,
teams and starting player, plus that seat's own submitted cards and the
independent policy seed. Never pass a `GamePreset`: it also carries other
seats' private lists and the real seed. The immutable card registry is
ordinary rules knowledge and says nothing about which cards a game contains.

The view projection is an observation of characteristics, not their inverse.
A 5/5 may owe its size to its printed face, a copy, a counter or a temporary
effect. Installing projected characteristics as a printed base would apply
some effects twice and make others permanent. Keep printed/copiable data,
effect causes and durations separately, then run the engine's layers.
CR 613.1 and CR 707.2 distinguish those inputs.

**Known** below means the current view completely supplies the rules value.
**Partial** means some of it is visible, historical, or derived and must be
reconstructed. It never means “default the missing part to zero”.
**Sampled** names an unobserved identity, ordering or simulation random stream.
A partial field with no faithful reconstruction makes the position
unsupported. Administrative counters may be renumbered consistently inside
the simulation; rules facts may not.

| # | Public `GameState` field | Classification | Source and reconstruction |
| ---: | --- | --- | --- |
| 1 | `arena` | Partial; hidden identities sampled | Own hand, authorized `looking_at`, and visible public objects constrain identities. Sample opposing hands, unseen library cards and unknown face-down identities jointly. Rebuild objects from definitions plus observed copies, choices, riders, targets and effects. The view lacks several of those facts, including object version, exact timestamp, chosen subtype/color, modes, X and copy ability provenance. Never treat `card: None` as uniformly “unknown card”: tokens and emblems need their creation definition. |
| 2 | `zones` | Partial; hidden membership/order sampled | Public zone lists and their order are known; own hand is known. Library and opponent-hand sizes constrain sampled contents. Library order is sampled conditional on known positions. Face-down exile and outside-game contents require their own entitlement and count information. A sideboard count is not in today's view. Preserve one location per object and all cross-references. |
| 3 | `players` | Partial | Copy life, poison, energy, loss flags and commander damage; unrestricted mana is known. Restricted mana is only totaled by color in the view: source, flags, restriction and spend rider are missing. Recover these and hand-size modifiers, lands played and control-since-turn timestamps from observations and rules. Teams come from `SeatIdentity`/public setup. Empty-draw status must be established by progression, not guessed. |
| 4 | `turn` | Known | `turn`, `active`, `phase`, `step` map directly. Priority is outside this field and belongs to the resume description. |
| 5 | `turn_start_seq` | Partial | Rebase the observed start of turn onto the synthetic journal. `PlayerView.seq` is a host snapshot sequence, not a journal cursor. |
| 6 | `combat` | Known | Copy ordered attackers, defenders, blockers and the persistent `blocked` flag. Engine declaration progress is separate. The first constructor admits only empty combat. |
| 7 | `per_turn` | Partial | Count spells, noncreature spells, draws, life loss and creatures dying from a complete observation suffix for this turn. A current life total does not say whether life was lost and then regained. |
| 8 | `delayed` | Partial | Reconstruct registered obligations, timing, controller and referenced objects from observed effects. Unknown obligations refuse the position; empty is allowed only when established. |
| 9 | `pending_miracle` | Partial | Requires draw timing and the pending continuation; private drawn identities remain sampled unless disclosed. Must be empty at the first accepted root. Do not create a miracle offer merely because a sampled hand contains a miracle card. |
| 10 | `extra_turns` | Partial | Recover the ordered queue from observed grants and consumed turns. Current active player is insufficient. |
| 11 | `restriction_info` | Partial | Rebuild source/filter/spend-rider records with the corresponding restricted mana. Unknown restriction provenance refuses the position. |
| 12 | `next_restriction_id` | Partial; derived | Allocate simulation-local ids after the rebuilt restriction records, reserving zero. No real allocator state is needed. |
| 13 | `commander_casts` | Known; derived | Sum the view's per-commander `casts` by owning seat. Keep this distinct from each commander's own tax. |
| 14 | `commander_redirect` | Partial | Answers owed by an in-progress move require its continuation. Must be empty for the initial settled root. |
| 15 | `pending_copied_faces` | Partial | A pending copy fixup needs the creation operation and copied face. Must be empty at the initial root; ordinary simulation progression creates later entries. |
| 16 | `ltb_abilities` | Partial | Reconstruct copy abilities at departure when future processing needs them. Do not read them off the current printed graveyard card. Retain only proven history; unsupported look-back requirements refuse import. |
| 17 | `ltb_attachments` | Partial | Reconstruct attachments at departure with the same lifetime as the associated object version. The current battlefield cannot supply a departed attachment. |
| 18 | `ceased` | Partial | Departed tokens/abilities awaiting trigger scans are not in the view. Must be empty once the initial root's scans are established as consumed. |
| 19 | `commanders` | Partial | Designations, identities and cast counts are visible. Remap handles, reconstruct each `answered` arrival marker and sample hidden placement when its location is not known. Designation does not reveal a library position. |
| 20 | `monarch` | Known | Directly from the view. |
| 21 | `day_night` | Known | Directly from the view, including no designation. |
| 22 | `previous_turn` | Partial | Recover the previous active seat and its spell count from history; current day/night does not determine them. |
| 23 | `starting_player` | Partial | Public setup/history supplies it. Do not substitute the current active player. |
| 24 | `ability_fires` | Partial | Recover per-object, per-ability uses this turn, resetting on the relevant zone change. Engine-owned loyalty uses need the same treatment in the resume description. |
| 25 | `rng` | Sampled; independent | Replace with the `future` stream in Q2. Never reconstruct the real RNG from game history or a supplied seed. |
| 26 | `journal` | Partial | Build simulation-local entries for known past facts that rules still read, with coherent cursors, then append simulated events. No raw host journal: it includes the seed, hidden object handles and unfiltered events. Unknown past facts are not fabricated as observations. |
| 27 | `names` | Partial; derived | Intern definition names for known and sampled objects in stable simulation order. Display strings are not rules identity. Never copy the host interner, which includes hidden cards. |
| 28 | `bases` | Partial; derived | Build a fresh cache from those definitions and observed copy/token causes. Sharing immutable faces within fabricated worlds is allowed; taking the real game's cache is not. |
| 29 | `timestamp` | Partial | Reconstruct relative effect/control/arrival order from history and assign consistent local stamps. Current summoning sickness alone does not recover every ordering dependency. |
| 30 | `effects` | Partial | Rebuild statics through the ordinary engine machinery; recover temporary effects, source bindings, expiry and ordering from observations. Matching projected P/T at this instant is necessary but insufficient. |
| 31 | `replacement_rules` | Partial | Rebuild registered rules from the fabricated sources and the normal registration machinery. Preserve controller and provenance. Do not copy the host registry. |
| 32 | `characteristics_generation` | Partial; derived | Start invalid and refresh. Its numeric value need not equal the host's; equality with the rebuilt effect generation must have the usual meaning. |
| 33 | `effect_generation` | Partial; derived | Use a local generation advanced by ordinary invalidation. Never trust copied view characteristics as a valid engine cache. |

The private `projection_ids` and `token_cleanup` are initialized inside the
engine's position builder. They are empty at the supported settled root.
External struct literals cannot construct `GameState` today because these
fields are private; a blank, validated position builder is part of the seam.
Using `from_preset` as a blank builder would also create spurious setup events,
opening hands and random draws.

### The distribution, and what it does not claim

For a known deck multiset, subtract cards whose current whereabouts are
known, preserving duplicate counts and keeping generated tokens/copies
separate from physical deck cards. Jointly allocate the residual copies to
unknown hand slots, face-down objects, hidden exile and library slots,
without replacement. Condition on every still-valid observation and every
rule restricting that placement. Conditional on a multiset and those
constraints, use an exchangeable distribution over physical copies and a
uniform permutation of unconstrained library positions. Scouting's top-first
order must be reversed when placed into `Zones`, whose last element is top.

For an unknown opposing deck, use an explicitly versioned, public collection
of legal candidate deck multisets, including sideboards when relevant,
with equal prior weights, then condition
on observations and weight by their likelihood under the allocation model.
It is not the opponent's submitted deck read from setup. If the collection
has no compatible candidate, decline lookahead. Choosing the actual
collection and validating its calibration remain open. A uniform draw of
card names from the compiled pool is rejected: it ignores multiplicities,
format constraints and deck composition. Observed strategic choices are not
assigned invented likelihoods in the first model.

The seat's own submitted list can arrive through a separate ordinary
seat-owned setup input. It is not present in `PlayerView`, and a print table
does not encode its multiplicities. An own list obtained only through
scouting retains scouting's temporary lifetime.

Face-down identity is conditioned on how that object became face down and
on any prior entitled observation, never inferred from an arena slot number.
Its visible characteristics stay those prescribed by that operation
(CR 708.2); entitlement to look is separate (CR 708.5). The initial adapter
refuses face-down mechanisms it cannot reconstruct. A shuffle removes
positional knowledge over the shuffled portion (CR 701.24a), while preserving
known composition and any explicitly excluded cards.

Allocate dense synthetic `ObjectId`s and retain a reversible mapping for
currently observable handles. A real handle's numeric slot is not a hint
about the card there. Remap attachments, targets, combat, commanders, effects
and the root action consistently. Unknown cards have no actionable host
handle. Return only a root action mapped back into the actual offered ids;
future sampled actions never escape the simulation.

The known/sampled invariant is:

> Every accepted sample reproduces all current entitled observations and
> retained valid facts after handle remapping, conserves zone counts and
> known deck multiplicities, and satisfies the root's offered choices.
> Varying the simulation seed may change only unconstrained hidden facts
> and future random outcomes. Changing the host's unobserved world while
> holding the authorized input fixed changes neither samples nor action.

Test this as `determinizations_preserve_observations_and_counts`, with
duplicate cards, a known top card, a revealed hand card followed by a hidden
draw, a shuffle, and a face-down fixture. Add duration/expiry and restricted
mana cases: round-tripping today's view alone would miss those wrong worlds.
An inconsistent sample is refused within a fixed attempt budget, never
repaired by changing a known fact. Repeated refusal returns the heuristic.

## Q4 — Memory belongs to the seat, scouting to the decision

`PlayerView` is insufficient. `looking_at` disappears with the pending
question. Views cannot recover a reveal between two snapshots, a shuffle
that leaves counts unchanged, temporary-effect duration, or all per-turn
events. `GameEvent::Revealed` exists, but the current host/view path does not
deliver an observation history to the house AI. Calling `act` only when that
seat answers also misses observations during other seats' decisions.

Define `SeatKnowledge` in `baylee-ai`, with one instance owned by `Session`
per game and seat. Update it explicitly from an ordered, seat-filtered
observation stream before asking for a decision. A separate lookahead
controller borrows that store; the stateless heuristic and its temporary
scouting clone do not own or mutate it. The harness must exercise the same
adapter. Keep the existing `act` usable with no memory and no search.

The stream needs event-time identities for authorized reveals, zone/count
changes, shuffles, observed ordering choices, public casts and resolutions,
effect creation/expiry and turn boundaries. Public choices supply the
resume facts. Private choices are delivered only to their entitled seat.
Capture a reveal at its event, before its object moves again. Filtering a
raw journal later by asking where that object is now is not equivalent.
Do not send the raw `Journal` or the opponent's full `Pending` as history.

Put these wire-stable observation values beside `PlayerView` in
`baylee-view`; adding a breaking payload bumps `VIEW_VERSION` and its host
and client assertions. Keep `baylee-view` free of the engine. Extract the
existing projection into a small `baylee-observation` crate shared by
gamehost and the house AI, since AI cannot depend on gamehost without a
cycle. It depends on engine and view; view still depends only on core and
serde. Simulated response policies receive a view for the simulated deciding
seat, never the complete sampled world. The leaf evaluator likewise reads
the root seat's simulated observations. This avoids letting an opponent
policy choose from cards that even that sampled opponent cannot see.

Keep observed identity, known location and known order as separate facts.
A revealed card can remain known to be in a hand after another card is
drawn; an unobserved departure can make that association uncertain. Record
alternatives or forget the association, rather than following a concealed
stable handle. Zone changes use observed incarnations, because engine
`ObjectId`s survive moves even though rules objects change (CR 400.7).

`take_over` and `release` retain the heuristic controller today. The separate
seat store survives both and continues receiving only that seat's ordinary
observations while Driven. It never starts remembering another seat because
the controller changed. A fresh game or seat assignment clears it. Sequence
gaps mark historical facts incomplete; a reconnect snapshot is not a complete
history. Search stays disabled for affected roots until an authorized replay
of observations restores those facts, or they cease to matter. A new
StandIn starts from the ordinary knowledge available for that human seat;
it cannot backfill through scouting. `hand_back` discards transient search.

Scouting may condition a determinization only through the existing guarded
request, with its current profile limits: disclosed decks/hands become
fixed constraints and expert's disclosed prefix fixes those positions for
that decision. No request is widened to full libraries for lookahead.
The report, conditioned samples, posterior weights, scores and any search
cache influenced by it are discarded before the decision returns. Nothing
derived from scouting updates `SeatKnowledge`. Re-check current
`SeatKind::Ai` on every request; Driven and StandIn remain denied. Retaining
only a numerical posterior would still retain the privilege and is rejected.

Test `takeover_preserves_observations_but_not_scouting`: reveal an ordinary
card, obtain a privileged report, take over, observe another ordinary event,
release, and compare the next no-scout decision with a fresh controller fed
only that ordinary history. Also test a sequence gap and a new game. Retain
the existing scouting denial and human-view invariance tests.

Reject memory in `PlayerView`: a snapshot should remain a snapshot. Reject
the live journal as memory: entitlement is per event and seat. Reject
keeping belief only in the cloned heuristic: `act_with_scouting` and takeover
would make both its update lifetime and its privilege lifetime ambiguous.

## Q5 — Bound attempted work, retain a completed incumbent

Start with **256 attempted simulation transitions per sharp decision** and
**1024 per expert decision**, only at the supported roots. Other profiles
keep the current heuristic. These are additional full-engine transitions,
not the much cheaper reply nodes of today's combat search. The first policy
uses at most eight root candidates, including the heuristic incumbent and
pass, in deterministic rank order. Sharp uses eight particles and four
actions per rollout; expert uses sixteen particles and eight actions:
`8 × 8 × 4 = 256`, `8 × 16 × 8 = 1024`. Fewer candidates spend less.
Root construction and every failed/retried attempt have separate finite
quotas; a failed advance still spends a transition.

Compute the heuristic incumbent first. Evaluate candidates in complete
particle rounds, so every compared candidate has the same samples and
horizon. Promote only after a complete round; retain the last completed
round's best, breaking ties in favor of the incumbent and then stable offer
order. An interrupted round contributes no scores. If even the first round
is incomplete, return the heuristic. A rollout ending inside a cast,
resolution or unsettled state is incomplete, not a cheap favorable leaf.
Terminal results can finish early; other evaluations require a settled
decision boundary. This is a short rollout estimate, not a proof of lethal
or an optimal hidden-information strategy.

Use the existing heuristic for subsequent simulated choices, with each
seat's own view and selected-effect context, until that rollout's action
limit. This keeps the first policy small. Future samples are possibilities,
not observations fed back into the persistent belief. Do not use a real
`snapshot_hash` as a policy seed, value or transposition key.

At the measured small-state cost, even charging one clone per transition
would spend approximately 1.40 ms for sharp or 5.59 ms for expert on clones
alone. Charging another state hash every time would add about 1.62/6.46 ms.
Neither sum prices real spell resolution or supplies an upper bound. Avoid
per-node snapshot hashes initially; use them in correctness tests.

**A transition cap does not bound `Engine::apply`.** One answer can allocate
many tokens or process a large finite chain of triggers. Brent's loop watch
detects repetition, not expensive finite work. Before enabling search in
gamehost, add deterministic simulation fuel to the shared advancement path:
charge effect operations, event production, object creation, scans and layer
work, including inner loops and allocations. Charge bulk operations before
allocating their output. An exhausted simulation is discarded with
`BudgetExhausted`; it does not publish half-settled priority, declare a draw,
or change the live game's loop rules.

Initial admission limits are 512 allocated object slots, 64 battlefield
objects, and 4096 retained journal entries per fabricated root. Bound
generated objects and bytes as well as iterations. Begin calibration with
2 million work units per sharp decision and 8 million per expert decision;
the work-unit charging scheme and its worst primitive cost must be measured
before those provisional numbers enable a profile. Missing history,
unsupported mechanics, oversized worlds or sampling exhaustion return the
incumbent before expensive reconstruction. Time is measured externally,
never read by the engine or used to choose between partially scored moves.

Target p99 total decision latency below 25 ms for sharp and 100 ms for expert
on supported positions, including observation building and sampling. These
are proposed acceptance targets, not measurements. Measure token growth,
long journals, replacement chains and many continuous effects as well as
ordinary turns. CI should assert work counts, deterministic answers and
legal fallback; a controlled benchmark establishes latency. A deterministic
budget cannot guarantee wall time under arbitrary machine suspension.

The host also needs bounded pumping: at most one lookahead decision before
returning control to the engine-server, which explicitly schedules the next
pump. A smaller loop bound without that rescheduling could strand an all-AI
game. Check decision sequence and current seat ownership before applying a
deferred answer. Timeout actions and StandIn use the current heuristic at
first. The synchronous engine stays synchronous; transport yields between
decisions. Do not enable search before fuel and host scheduling land.

Reject “search for 100 ms”: the action would depend on CPU contention,
contradicting `docs/house-ai.md`'s deterministic node-budget contract. Reject
spending anything close to 600 seconds and relying on the clock to interrupt
it: the clock cannot preempt this synchronous call, and AI chairs have none.
Reject promoting a partly evaluated candidate: it rewards whichever hidden
world or response happened to be visited first.

## Q6 — The first implementation commit

The smallest useful milestone is the checked settled-main-priority
constructor, its independent RNG argument, and
`engine::simulation_tests::a_fabricated_midgame_branch_leaves_its_source_unchanged`.
The test belongs in an inline `#[cfg(test)]` engine module. No such code or
test is added by this design commit.

Build a fixture through ordinary `Engine::apply`: keep opening hands, reach
a main phase, play a land, and reach settled priority with an empty stack.
Record both the engine and state `snapshot_hash` plus RNG seed, position and
call count. Inside the test only, clone its mid-game state and supply the
explicit resume facts known from those actions. Construct a simulation with
an independent seed, select an actually offered land play or mana ability,
apply it, and assert the branch's state hash changes while every recorded
source value is unchanged. The fixture must arrange such a non-pass action.

Also assert the imported turn and pending player, the absence of a mulligan,
and the normalized legal offer. In a control continuation, apply the same
nonrandom action sequence to the original fixture and compare subsequent
offers and rules facts with the simulation, excluding its deliberately
different RNG. Exercise a second resume description with a nonzero pass
count so a constructor that resets all latches cannot pass. An unsupported
mid-resolution resume must return an error. These checks can be subcases of
the same first regression; the isolation hash alone is not evidence that
the cloned game continues correctly.

Before the constructor exists, the proposed test does not compile. After
it exists, replacing the import with preset setup or dropping the supplied
pass count must fail the behavioral assertions. That is the red/green
counter-test. Tests may see both worlds; production house-AI inputs never
receive the original fixture's equivalent.

Reject the smaller-looking generic constructor plus one hash comparison.
It would prove isolation while allowing an engine that restarts mulligans
or forgets passed priority. The first milestone proves a narrow continuation
honestly and does not yet connect lookahead to a live seat.

## Seams, ranked by what they unblock

Sizes are relative implementation/review scope: S is one local change, M
crosses a module boundary, L crosses contracts, XL is a general subsystem.
They are not elapsed-time estimates. Ranking expresses dependency value;
all enabling gates still apply regardless of rank.

| Rank | Crate(s) | Seam | Size | Documentation cost |
| ---: | --- | --- | :---: | --- |
| 1 | `baylee-engine` | Validated fabricated-position builder and constructor with explicit settled-priority resume facts; preserve the one rules driver. | M | Qualify `CLAUDE.md`'s no-Engine/no-GameState sentence and extend its engine API/diagram; qualify `GameState`'s AI-clone claim. |
| 2 | `baylee-gamehost`, `baylee-view`, proposed `baylee-observation` | Ordered seat-authorized observations, missing public rule provenance, and one projector shared with simulated decisions. | L | Extend the snapshot-only view contract alongside, not inside, `PlayerView`; bump its version if breaking. Preserve the engine-free view crate. |
| 3 | `baylee-ai`, `baylee-gamehost` | Seat-scoped belief, constrained sampling, synthetic-id mapping, reconstruction refusal and ephemeral scouting conditioning. | L | Extend `docs/house-ai.md`'s stateless/tactical policy scope; retain its ban on retained scouting. |
| 4 | `baylee-engine` | Deterministic fuel and allocation limits inside simulated advancement, with an explicit exhausted result. | L | Extend `docs/engine-internals.md`'s advance-to-next-choice contract for disposable simulations; no new live-game loop result. |
| 5 | `baylee-ai`, `baylee-gamehost` | Independent policy seed in both Session and harness, plus paired RNG/noninterference tests. | S | Replace “Randomness is keyed by the game seed” in `docs/house-ai.md`; raw game-seed plumbing must end before search starts. |
| 6 | `baylee-ai`, `baylee-gamehost`, `baylee-engine-server` | Bounded root rollouts, completed-round incumbent, externally measured latency and pump rescheduling. | L | Add full-engine budgets beside the existing combat budgets; preserve “time is never a policy input” and synchronous rules. |

## Open decisions and the evidence that settles them

- **Coverage beyond ordinary main-phase priority.** Arbitrary cast and
  resolution imports, active loop continuation, combat roots and checkpoint
  forks are not selected for the first rollout policy. A continuation
  inventory and paired continuation tests settle each extension; expect XL
  for a general import, not a larger version of the first M-sized change.
- **Unknown-deck prior.** Its finite, public, versioned shape is selected;
  its candidate lists and calibration are not. Recorded held-out games and
  support/refusal rates decide whether it represents this pool's play. No
  compatible prior means heuristic, not access to the submitted deck.
- **Playing strength and evaluation weights.** Short heuristic rollouts are
  the baseline; MCTS, adversarial search and learned opponent likelihoods are
  unchosen. Paired seeds/seats against today's heuristic, including unfinished
  games and decision latency, decide whether any successor is better.
- **Fuel constants and enabled profiles.** Transition counts above are
  initial caps, not a release claim. An implemented charging scheme,
  allocation bounds and measured tail latency settle them. Search remains
  disabled until those checks pass.
- **Observation retention and recovery encoding.** Per-seat provenance,
  sequence-gap refusal and scouting expiry are decided. The compact replay
  format and retention window need an inventory of historical readers and
  long-game memory measurements. Discarding required history may reduce
  search coverage; it may never silently change known facts.

## Findings worth separate tickets

1. **The policy already receives the real shuffle seed.**
   `session.rs::Session::new` and `harness.rs::play_report` seed `HeuristicAgent`
   from `preset.seed`, also used by `GameState::from_preset`. With authorized
   deck knowledge this is material for reproducing hidden setup order. The
   present heuristic does not do that; adding determinization without
   removing this input would make the boundary depend on restraint.
2. **Snapshot hashes are not complete continuation identities.**
   `GameState::snapshot_hash` claims to include everything affecting future
   outcomes but does not hash `lands_played_this_turn`, `per_turn`,
   `delayed`, `extra_turns` or `ability_fires`, among others.
   `Engine::snapshot_hash` does not add all driver fields either, including
   `loyalty_used_this_turn`. Equal hashes therefore do not establish equal
   futures. Keep the requested unchanged-source hash assertion, add direct
   checks, and audit hashing separately before using it for transpositions.
3. **Public reveal events have no observation-history consumer here.**
   The engine journals `Revealed`, but the house AI receives snapshots and
   pending-specific `looking_at`. A reveal completed between decisions
   cannot enter a persistent belief through the existing interface. Add
   event-time, seat-filtered delivery; do not expose the raw journal.
4. **Some prose overstates current guarantees.** The state “cloneable for
   AI” comment and per-ply clone claims in `docs/engine-internals.md` describe
   an intended primitive, not a resumable engine. `CLAUDE.md` restates
   `VIEW_VERSION` as 12 in its held-chair paragraph although it is now 21.
   The baseline bench still says CI regression budgets derive from it;
   `docs/perf-baseline.md` explicitly says comparison is manual. These are
   documentation tickets, not reasons to change production code here.

The field counts, the capability-checked mutable accessor, superseded
benchmark figures and the absence of an AI decision clock correct premises
of the brief; they are not newly discovered regressions in the repository.

## Evidence and rule citations

The four timing paths above were rerun under the cargo lock. The structural
findings come from the declarations and callers at the recorded commit;
future regression names in this document are proposed tests, not claimed
passing tests. No production or proof code was written for this design.
The two existing gamehost tests selected by `cargo test -p baylee-gamehost
--lib scouting` passed under the same cargo lock: current-seat authorization
through takeover/stand-in, and bounded scouting without changing human views.
`harness::tests::every_profile_is_blind_to_an_opponents_hidden_cards` also
passed under the lock. These are existing boundary tests, not proof of the
proposed sampler. The existing `target/debug/xtask cr-check --rules
/Users/viktor/Projects/mtg/MagicCompRules.txt` passed with this document staged:
2365 citations in 277 files. The checker verifies numbers and neighboring
rule words; reading the rules supplies their meaning.

CR numbers used here were looked up in the external local
`MagicCompRules.txt` (effective 27 February 2026) and checked against the
[Wizards rules text](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt)
linked by the [official rules page](https://magic.wizards.com/en/rules) on
19 September. That linked text is marked effective 25 September, so it is
not claimed to be the in-force edition on the measurement date. The cited
rules agree on the relevant points: zones and object changes (CR 400.2,
CR 400.7), priority settlement (CR 117.5), layered/copied characteristics
(CR 613.1, CR 707.2), face-down characteristics and viewing rights
(CR 708.2, CR 708.5), and randomized order (CR 701.24a). No external rules
text or corpus file is copied into this repository.
